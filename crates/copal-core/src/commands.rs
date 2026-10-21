//! Built-in commands and the registry that maps text names to them.

use std::sync::Arc;

use copal_mesh::Mesh;
use glam::Vec3;

use crate::command::{Command, CommandError};
use crate::document::{Document, ObjectId, SceneObject, Transform};
use crate::script::{Args, parse_line, quote};

/// Describes one command for help screens, autocomplete and docs.
pub struct CommandInfo {
    pub name: &'static str,
    pub summary: &'static str,
    pub usage: &'static str,
    build: fn(&Args) -> Result<Box<dyn Command>, CommandError>,
}

impl std::fmt::Debug for CommandInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommandInfo")
            .field("name", &self.name)
            .finish()
    }
}

/// All commands, sorted by name.
pub fn registry() -> &'static [CommandInfo] {
    REGISTRY
}

/// Builds a command from parsed arguments.
pub fn build(args: &Args) -> Result<Box<dyn Command>, CommandError> {
    let info = REGISTRY
        .iter()
        .find(|c| c.name == args.command())
        .ok_or_else(|| CommandError::UnknownCommand(args.command().to_string()))?;
    (info.build)(args)
}

/// Parses and builds one line of the text language. `Ok(None)` for blank lines and comments.
pub fn parse_command(line: &str) -> Result<Option<Box<dyn Command>>, CommandError> {
    match parse_line(line)? {
        Some(args) => build(&args).map(Some),
        None => Ok(None),
    }
}

static REGISTRY: &[CommandInfo] = &[
    CommandInfo {
        name: "object.add_cube",
        summary: "Add a box. Handy for tests and calibration.",
        usage: "object.add_cube size=<mm> | x=<mm> y=<mm> z=<mm> [name=<text>]",
        build: build_add_cube,
    },
    CommandInfo {
        name: "object.drop",
        summary: "Move an object down so it stands on the build plate.",
        usage: "object.drop id=<id>",
        build: |a| {
            a.only(&["id"])?;
            Ok(Box::new(DropToPlate {
                id: ObjectId(a.get("id")?),
            }))
        },
    },
    CommandInfo {
        name: "object.move",
        summary: "Move an object by an offset in mm.",
        usage: "object.move id=<id> [x=<mm>] [y=<mm>] [z=<mm>]",
        build: |a| {
            a.only(&["id", "x", "y", "z"])?;
            let delta = vec3_args(a, 0.0)?;
            Ok(Box::new(EditTransform {
                id: ObjectId(a.get("id")?),
                edit: Edit::Move(delta),
            }))
        },
    },
    CommandInfo {
        name: "object.remove",
        summary: "Delete an object.",
        usage: "object.remove id=<id>",
        build: |a| {
            a.only(&["id"])?;
            Ok(Box::new(RemoveObject {
                id: ObjectId(a.get("id")?),
            }))
        },
    },
    CommandInfo {
        name: "object.rename",
        summary: "Rename an object.",
        usage: "object.rename id=<id> name=<text>",
        build: |a| {
            a.only(&["id", "name"])?;
            Ok(Box::new(RenameObject {
                id: ObjectId(a.get("id")?),
                name: a.str("name")?.to_string(),
            }))
        },
    },
    CommandInfo {
        name: "object.rotate",
        summary: "Rotate an object by angles in degrees.",
        usage: "object.rotate id=<id> [x=<deg>] [y=<deg>] [z=<deg>]",
        build: |a| {
            a.only(&["id", "x", "y", "z"])?;
            let delta = vec3_args(a, 0.0)?;
            Ok(Box::new(EditTransform {
                id: ObjectId(a.get("id")?),
                edit: Edit::Rotate(delta),
            }))
        },
    },
    CommandInfo {
        name: "object.scale",
        summary: "Scale an object. `factor` scales all axes; x/y/z scale one axis.",
        usage: "object.scale id=<id> factor=<n> | [x=<n>] [y=<n>] [z=<n>]",
        build: |a| {
            a.only(&["id", "factor", "x", "y", "z"])?;
            let factor = match a.opt::<f32>("factor")? {
                Some(f) => Vec3::splat(f),
                None => vec3_args(a, 1.0)?,
            };
            if factor.cmple(Vec3::ZERO).any() || !factor.is_finite() {
                return Err(CommandError::Invalid(
                    "scale factors must be positive numbers".into(),
                ));
            }
            Ok(Box::new(EditTransform {
                id: ObjectId(a.get("id")?),
                edit: Edit::Scale(factor),
            }))
        },
    },
    CommandInfo {
        name: "object.set_transform",
        summary: "Set position (mm), rotation (degrees) and scale of an object exactly.",
        usage: "object.set_transform id=<id> [tx ty tz rx ry rz sx sy sz]=<n>",
        build: |a| {
            a.only(&["id", "tx", "ty", "tz", "rx", "ry", "rz", "sx", "sy", "sz"])?;
            let id = ObjectId(a.get("id")?);
            let t = Transform {
                translation: Vec3::new(a.get("tx")?, a.get("ty")?, a.get("tz")?),
                rotation_deg: Vec3::new(a.get("rx")?, a.get("ry")?, a.get("rz")?),
                scale: Vec3::new(a.get("sx")?, a.get("sy")?, a.get("sz")?),
            };
            Ok(Box::new(SetTransform { id, transform: t }))
        },
    },
    CommandInfo {
        name: "project.rename",
        summary: "Rename the project.",
        usage: "project.rename name=<text>",
        build: |a| {
            a.only(&["name"])?;
            Ok(Box::new(SetProjectField::Name(a.str("name")?.to_string())))
        },
    },
    CommandInfo {
        name: "project.set_printer",
        summary: "Choose the printer preset. An empty value clears it.",
        usage: "project.set_printer printer=<brand/model>",
        build: |a| {
            a.only(&["printer"])?;
            Ok(Box::new(SetProjectField::Printer(non_empty(
                a.str("printer")?,
            ))))
        },
    },
    CommandInfo {
        name: "project.set_resin",
        summary: "Choose the resin preset. An empty value clears it.",
        usage: "project.set_resin resin=<brand/resin>",
        build: |a| {
            a.only(&["resin"])?;
            Ok(Box::new(SetProjectField::Resin(non_empty(a.str("resin")?))))
        },
    },
];

fn non_empty(s: &str) -> Option<String> {
    (!s.is_empty()).then(|| s.to_string())
}

fn vec3_args(a: &Args, default: f32) -> Result<Vec3, CommandError> {
    Ok(Vec3::new(
        a.opt("x")?.unwrap_or(default),
        a.opt("y")?.unwrap_or(default),
        a.opt("z")?.unwrap_or(default),
    ))
}

fn fmt_vec(prefix: &str, v: Vec3, skip: f32) -> String {
    [("x", v.x), ("y", v.y), ("z", v.z)]
        .iter()
        .filter(|(_, n)| *n != skip)
        .map(|(k, n)| format!(" {prefix}{k}={n}"))
        .collect()
}

fn build_add_cube(a: &Args) -> Result<Box<dyn Command>, CommandError> {
    a.only(&["size", "x", "y", "z", "name"])?;
    let size = match a.opt::<f32>("size")? {
        Some(s) => Vec3::splat(s),
        None => Vec3::new(a.get("x")?, a.get("y")?, a.get("z")?),
    };
    if size.cmple(Vec3::ZERO).any() || !size.is_finite() {
        return Err(CommandError::Invalid("cube size must be positive".into()));
    }
    let name = a.opt_str("name").unwrap_or("Cube").to_string();
    let text = if size.x == size.y && size.y == size.z {
        format!("object.add_cube size={}", size.x)
    } else {
        format!("object.add_cube x={} y={} z={}", size.x, size.y, size.z)
    };
    let text = if a.opt_str("name").is_some() {
        format!("{text} name={}", quote(&name))
    } else {
        text
    };
    Ok(Box::new(AddObject {
        id: None,
        index: None,
        name,
        transform: Transform::default(),
        mesh: Arc::new(Mesh::cuboid(size)),
        text: Some(text),
    }))
}

// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone)]
enum SetProjectField {
    Name(String),
    Printer(Option<String>),
    Resin(Option<String>),
}

impl Command for SetProjectField {
    fn name(&self) -> &'static str {
        match self {
            Self::Name(_) => "project.rename",
            Self::Printer(_) => "project.set_printer",
            Self::Resin(_) => "project.set_resin",
        }
    }

    fn to_text(&self) -> Option<String> {
        let opt = |v: &Option<String>| quote(v.as_deref().unwrap_or(""));
        Some(match self {
            Self::Name(n) => format!("project.rename name={}", quote(n)),
            Self::Printer(p) => format!("project.set_printer printer={}", opt(p)),
            Self::Resin(r) => format!("project.set_resin resin={}", opt(r)),
        })
    }

    fn apply(&self, doc: &mut Document) -> Result<Box<dyn Command>, CommandError> {
        let old = match self {
            Self::Name(n) => Self::Name(std::mem::replace(&mut doc.name, n.clone())),
            Self::Printer(p) => Self::Printer(std::mem::replace(&mut doc.printer, p.clone())),
            Self::Resin(r) => Self::Resin(std::mem::replace(&mut doc.resin, r.clone())),
        };
        Ok(Box::new(old))
    }
}

#[derive(Debug, Clone)]
struct AddObject {
    /// `None` = allocate a new id. Set when re-adding a removed object (undo).
    id: Option<ObjectId>,
    /// `None` = append. Set when re-adding a removed object (undo).
    index: Option<usize>,
    name: String,
    transform: Transform,
    mesh: Arc<Mesh>,
    text: Option<String>,
}

impl Command for AddObject {
    fn name(&self) -> &'static str {
        "object.add"
    }

    fn to_text(&self) -> Option<String> {
        self.text.clone()
    }

    fn apply(&self, doc: &mut Document) -> Result<Box<dyn Command>, CommandError> {
        let id = match self.id {
            Some(id) if doc.object(id).is_some() => {
                return Err(CommandError::Invalid(format!(
                    "object id {id} is already used"
                )));
            }
            Some(id) => id,
            None => doc.allocate_id(),
        };
        let object = SceneObject {
            id,
            name: self.name.clone(),
            transform: self.transform,
            mesh: self.mesh.clone(),
        };
        let index = self
            .index
            .unwrap_or(doc.objects.len())
            .min(doc.objects.len());
        doc.objects.insert(index, object);
        Ok(Box::new(RemoveObject { id }))
    }
}

#[derive(Debug, Clone)]
struct RemoveObject {
    id: ObjectId,
}

impl Command for RemoveObject {
    fn name(&self) -> &'static str {
        "object.remove"
    }

    fn to_text(&self) -> Option<String> {
        Some(format!("object.remove id={}", self.id))
    }

    fn apply(&self, doc: &mut Document) -> Result<Box<dyn Command>, CommandError> {
        let index = doc
            .index_of(self.id)
            .ok_or(CommandError::NoSuchObject(self.id))?;
        let o = doc.objects.remove(index);
        Ok(Box::new(AddObject {
            id: Some(o.id),
            index: Some(index),
            name: o.name,
            transform: o.transform,
            mesh: o.mesh,
            text: None,
        }))
    }
}

#[derive(Debug, Clone)]
struct RenameObject {
    id: ObjectId,
    name: String,
}

impl Command for RenameObject {
    fn name(&self) -> &'static str {
        "object.rename"
    }

    fn to_text(&self) -> Option<String> {
        Some(format!(
            "object.rename id={} name={}",
            self.id,
            quote(&self.name)
        ))
    }

    fn apply(&self, doc: &mut Document) -> Result<Box<dyn Command>, CommandError> {
        let o = doc
            .object_mut(self.id)
            .ok_or(CommandError::NoSuchObject(self.id))?;
        let old = std::mem::replace(&mut o.name, self.name.clone());
        Ok(Box::new(RenameObject {
            id: self.id,
            name: old,
        }))
    }
}

#[derive(Debug, Clone, Copy)]
enum Edit {
    Move(Vec3),
    Rotate(Vec3),
    Scale(Vec3),
}

#[derive(Debug, Clone)]
struct EditTransform {
    id: ObjectId,
    edit: Edit,
}

impl Command for EditTransform {
    fn name(&self) -> &'static str {
        match self.edit {
            Edit::Move(_) => "object.move",
            Edit::Rotate(_) => "object.rotate",
            Edit::Scale(_) => "object.scale",
        }
    }

    fn to_text(&self) -> Option<String> {
        let (name, args) = match self.edit {
            Edit::Move(v) => ("object.move", fmt_vec("", v, 0.0)),
            Edit::Rotate(v) => ("object.rotate", fmt_vec("", v, 0.0)),
            Edit::Scale(v) if v.x == v.y && v.y == v.z => {
                ("object.scale", format!(" factor={}", v.x))
            }
            Edit::Scale(v) => ("object.scale", fmt_vec("", v, 1.0)),
        };
        Some(format!("{name} id={}{args}", self.id))
    }

    fn apply(&self, doc: &mut Document) -> Result<Box<dyn Command>, CommandError> {
        let o = doc
            .object_mut(self.id)
            .ok_or(CommandError::NoSuchObject(self.id))?;
        let old = o.transform;
        match self.edit {
            Edit::Move(d) => o.transform.translation += d,
            Edit::Rotate(d) => o.transform.rotation_deg += d,
            Edit::Scale(f) => o.transform.scale *= f,
        }
        // Restore the exact old value instead of subtracting, so undo never drifts.
        Ok(Box::new(SetTransform {
            id: self.id,
            transform: old,
        }))
    }
}

#[derive(Debug, Clone)]
struct SetTransform {
    id: ObjectId,
    transform: Transform,
}

impl Command for SetTransform {
    fn name(&self) -> &'static str {
        "object.set_transform"
    }

    fn to_text(&self) -> Option<String> {
        let t = &self.transform;
        Some(format!(
            "object.set_transform id={} tx={} ty={} tz={} rx={} ry={} rz={} sx={} sy={} sz={}",
            self.id,
            t.translation.x,
            t.translation.y,
            t.translation.z,
            t.rotation_deg.x,
            t.rotation_deg.y,
            t.rotation_deg.z,
            t.scale.x,
            t.scale.y,
            t.scale.z,
        ))
    }

    fn apply(&self, doc: &mut Document) -> Result<Box<dyn Command>, CommandError> {
        let o = doc
            .object_mut(self.id)
            .ok_or(CommandError::NoSuchObject(self.id))?;
        let old = std::mem::replace(&mut o.transform, self.transform);
        Ok(Box::new(SetTransform {
            id: self.id,
            transform: old,
        }))
    }
}

#[derive(Debug, Clone)]
struct DropToPlate {
    id: ObjectId,
}

impl Command for DropToPlate {
    fn name(&self) -> &'static str {
        "object.drop"
    }

    fn to_text(&self) -> Option<String> {
        Some(format!("object.drop id={}", self.id))
    }

    fn apply(&self, doc: &mut Document) -> Result<Box<dyn Command>, CommandError> {
        let o = doc
            .object_mut(self.id)
            .ok_or(CommandError::NoSuchObject(self.id))?;
        let old = o.transform;
        if let Some(b) = o.world_bounds() {
            o.transform.translation.z -= b.min.z;
        }
        Ok(Box::new(SetTransform {
            id: self.id,
            transform: old,
        }))
    }
}
