//! Built-in commands and the registry that maps text names to them.

use std::sync::Arc;

use copal_mesh::Mesh;
use glam::Vec3;

use crate::command::{Command, CommandError};
use crate::document::{Document, ObjectId, SceneObject, Transform};
use crate::ops::{Arrange, Duplicate, Import, Mirror, RemoveAll, Section, SetMachine, SetSettings};
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
        name: "holes.add",
        summary: "Add a drain hole at a point on the model surface (goes into the model along its surface normal).",
        usage: "holes.add id=<id> x=<mm> y=<mm> z=<mm> [diameter=<mm>] [depth=<mm>]",
        build: crate::prepare::build_holes_add,
    },
    CommandInfo {
        name: "holes.clear",
        summary: "Remove all drain holes of an object.",
        usage: "holes.clear id=<id>",
        build: crate::prepare::build_holes_clear,
    },
    CommandInfo {
        name: "holes.remove",
        summary: "Remove one drain hole (numbers start at 0).",
        usage: "holes.remove id=<id> index=<n>",
        build: crate::prepare::build_holes_remove,
    },
    CommandInfo {
        name: "hollow.set",
        summary: "Change hollowing settings (wall thickness, drain holes).",
        usage: "hollow.set <setting>=<value> … (run `settings hollow` to see them)",
        build: |a| SetSettings::build(Section::Hollow, a),
    },
    CommandInfo {
        name: "layout.set",
        summary: "Change how models move on the plate (keep on plate, snapping, spacing).",
        usage: "layout.set <setting>=<value> … (run `settings layout` to see them)",
        build: |a| SetSettings::build(Section::Layout, a),
    },
    CommandInfo {
        name: "object.add_cube",
        summary: "Add a box. Handy for tests and calibration.",
        usage: "object.add_cube size=<mm> | x=<mm> y=<mm> z=<mm> [name=<text>]",
        build: build_add_cube,
    },
    CommandInfo {
        name: "object.arrange",
        summary: "Place all objects on the plate in neat rows.",
        usage: "object.arrange [spacing=<mm>]",
        build: |a| {
            a.only(&["spacing"])?;
            Ok(Box::new(Arrange {
                spacing: a.opt("spacing")?,
            }))
        },
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
        name: "object.duplicate",
        summary: "Copy an object to free places on the plate.",
        usage: "object.duplicate id=<id> [count=<n>]",
        build: |a| {
            a.only(&["id", "count"])?;
            let count: u32 = a.opt("count")?.unwrap_or(1);
            if !(1..=500).contains(&count) {
                return Err(CommandError::Invalid("count must be 1 to 500".into()));
            }
            Ok(Box::new(Duplicate {
                id: ObjectId(a.get("id")?),
                count,
            }))
        },
    },
    CommandInfo {
        name: "object.hollow",
        summary: "Hollow an object: keep walls of the given thickness. Add drain holes so resin can flow out.",
        usage: "object.hollow id=<id> [thickness=<mm>] [voxel=<mm>]",
        build: crate::prepare::build_hollow,
    },
    CommandInfo {
        name: "object.import",
        summary: "Load a model from an STL, OBJ or 3MF file onto the plate.",
        usage: "object.import path=<file> [name=<text>]",
        build: Import::build,
    },
    CommandInfo {
        name: "object.mirror",
        summary: "Mirror an object along the X, Y or Z axis.",
        usage: "object.mirror id=<id> axis=x|y|z",
        build: |a| {
            a.only(&["id", "axis"])?;
            let axis = match a.str("axis")? {
                "x" | "X" => 0,
                "y" | "Y" => 1,
                "z" | "Z" => 2,
                other => {
                    return Err(CommandError::BadValue {
                        command: "object.mirror".into(),
                        arg: "axis".into(),
                        value: other.into(),
                        reason: "expected x, y or z".into(),
                    });
                }
            };
            Ok(Box::new(Mirror {
                id: ObjectId(a.get("id")?),
                axis,
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
        name: "object.remove_all",
        summary: "Delete every object.",
        usage: "object.remove_all",
        build: |a| {
            a.only(&[])?;
            Ok(Box::new(RemoveAll))
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
            Ok(Box::new(SetTransform {
                id,
                transform: t,
                supports: None,
            }))
        },
    },
    CommandInfo {
        name: "object.unhollow",
        summary: "Make a hollowed object solid again.",
        usage: "object.unhollow id=<id>",
        build: crate::prepare::build_unhollow,
    },
    CommandInfo {
        name: "print.set",
        summary: "Change print settings: layers, exposure, lift and retract, anti-aliasing.",
        usage: "print.set <setting>=<value> … (run `settings print` to see them)",
        build: |a| SetSettings::build(Section::Print, a),
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
        summary: "Choose the printer. Applies the resin's settings for it, if it has them.",
        usage: "project.set_printer printer=<brand/model>",
        build: |a| {
            a.only(&["printer"])?;
            Ok(Box::new(SetMachine::Printer(non_empty(a.str("printer")?))))
        },
    },
    CommandInfo {
        name: "project.set_resin",
        summary: "Choose the resin. Applies its print settings for the chosen printer.",
        usage: "project.set_resin resin=<brand/resin>",
        build: |a| {
            a.only(&["resin"])?;
            Ok(Box::new(SetMachine::Resin(non_empty(a.str("resin")?))))
        },
    },
    CommandInfo {
        name: "supports.add",
        summary: "Add one support at a point on the model surface. Braces itself to neighbours when manual bracing is on.",
        usage: "supports.add id=<id> x=<mm> y=<mm> z=<mm>",
        build: crate::prepare::build_supports_add,
    },
    CommandInfo {
        name: "supports.auto",
        summary: "Add supports automatically: islands and overhangs (auto) or only islands. Lifts the model off the plate. Manual supports stay.",
        usage: "supports.auto id=<id> [mode=auto|islands]",
        build: crate::prepare::build_supports_auto,
    },
    CommandInfo {
        name: "supports.clear",
        summary: "Remove all supports of an object.",
        usage: "supports.clear id=<id>",
        build: crate::prepare::build_supports_clear,
    },
    CommandInfo {
        name: "supports.remove",
        summary: "Remove supports by their numbers.",
        usage: "supports.remove id=<id> sid=<n>[,<n>…]",
        build: crate::prepare::build_supports_remove,
    },
    CommandInfo {
        name: "supports.resize",
        summary: "Change the thickness of some supports.",
        usage: "supports.resize id=<id> sid=<n>[,<n>…] [diameter=<mm>] [tip=<mm>]",
        build: crate::prepare::build_supports_resize,
    },
    CommandInfo {
        name: "supports.set",
        summary: "Change support settings (tips, thickness, overhang angle, bracing, raft).",
        usage: "supports.set <setting>=<value> … (run `settings supports` to see them)",
        build: |a| SetSettings::build(Section::Supports, a),
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
        prep: Default::default(),
        text: Some(text),
    }))
}

// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone)]
enum SetProjectField {
    Name(String),
}

impl Command for SetProjectField {
    fn name(&self) -> &'static str {
        "project.rename"
    }

    fn to_text(&self) -> Option<String> {
        let Self::Name(n) = self;
        Some(format!("project.rename name={}", quote(n)))
    }

    fn apply(&self, doc: &mut Document) -> Result<Box<dyn Command>, CommandError> {
        let Self::Name(n) = self;
        Ok(Box::new(Self::Name(std::mem::replace(
            &mut doc.name,
            n.clone(),
        ))))
    }
}

#[derive(Debug, Clone)]
pub(crate) struct AddObject {
    /// `None` = allocate a new id. Set when re-adding a removed object (undo).
    pub id: Option<ObjectId>,
    /// `None` = append. Set when re-adding a removed object (undo).
    pub index: Option<usize>,
    pub name: String,
    pub transform: Transform,
    pub mesh: Arc<Mesh>,
    pub prep: crate::document::Prepared,
    pub text: Option<String>,
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
            prep: self.prep.clone(),
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
pub(crate) struct RemoveObject {
    pub id: ObjectId,
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
            prep: o.prep,
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
        let keep_on_plate = doc.settings.layout.keep_on_plate;
        let o = doc
            .object_mut(self.id)
            .ok_or(CommandError::NoSuchObject(self.id))?;
        let old = o.transform;
        let old_supports = o.prep.supports.clone();
        let was_on_plate = o.world_bounds().is_some_and(|b| b.min.z.abs() < 1e-3);
        match self.edit {
            Edit::Move(d) => o.transform.translation += d,
            Edit::Rotate(d) => o.transform.rotation_deg += d,
            Edit::Scale(f) => o.transform.scale *= f,
        }
        let lost_supports = drop_stale_supports(o, &old);
        // A model that stood on the plate stays on it after rotating or scaling. One that had
        // supports goes back to the plate when they are removed.
        if keep_on_plate
            && (was_on_plate || lost_supports)
            && !matches!(self.edit, Edit::Move(_))
            && let Some(b) = o.world_bounds()
        {
            o.transform.translation.z -= b.min.z;
        }
        // Restore the exact old value instead of subtracting, so undo never drifts.
        Ok(Box::new(SetTransform {
            id: self.id,
            transform: old,
            supports: Some(old_supports),
        }))
    }
}

/// Supports are made for one orientation and size. When either changes they no longer fit and
/// are removed. Returns true if supports were removed.
pub(crate) fn drop_stale_supports(o: &mut crate::document::SceneObject, old: &Transform) -> bool {
    let changed = o.transform.rotation_deg != old.rotation_deg || o.transform.scale != old.scale;
    if changed && o.prep.supports.is_some() {
        o.prep.supports = None;
        return true;
    }
    false
}

#[derive(Debug, Clone)]
pub(crate) struct SetTransform {
    pub id: ObjectId,
    pub transform: Transform,
    /// `Some`: set the supports to exactly this (used by undo). `None`: keep them if they still
    /// fit (only the position changed), otherwise remove them.
    pub supports: Option<Option<Arc<copal_supports::SupportSet>>>,
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
        let old_supports = o.prep.supports.clone();
        match &self.supports {
            Some(s) => o.prep.supports = s.clone(),
            None => {
                drop_stale_supports(o, &old);
            }
        }
        Ok(Box::new(SetTransform {
            id: self.id,
            transform: old,
            supports: Some(old_supports),
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
        if o.prep.has_supports() {
            return Err(CommandError::Invalid(format!(
                "object {} stands on supports; remove them first (supports.clear id={})",
                self.id, self.id
            )));
        }
        let old = o.transform;
        if let Some(b) = o.world_bounds() {
            o.transform.translation.z -= b.min.z;
        }
        Ok(Box::new(SetTransform {
            id: self.id,
            transform: old,
            supports: None,
        }))
    }
}
