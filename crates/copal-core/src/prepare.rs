//! Commands of the Prepare step: supports, hollowing and drain holes.
//!
//! The heavy work (`auto_supports`, `hollow_object`) is public so an app can run it in the
//! background on a copy of the document and then apply the result as a command.

use std::sync::{Arc, Mutex};

use copal_mesh::Mesh;
use copal_mesh::bvh::Bvh;
use copal_supports::{DrainHole, Mode, SupportSet};
use glam::{Mat4, Vec3};

use crate::command::{Command, CommandError};
use crate::document::{Document, ObjectId, SceneObject};
use crate::script::Args;

/// Puts back what a prepare command changed. Fields that are `None` stay as they are.
#[derive(Debug, Clone, Default)]
pub struct RestorePrep {
    pub id: ObjectId,
    pub translation: Option<Vec3>,
    pub supports: Option<Option<Arc<SupportSet>>>,
    pub cavity: Option<Option<Arc<Mesh>>>,
    pub holes: Option<Vec<DrainHole>>,
    /// Label for the history (the command that was undone is shown instead when `None`).
    pub text: Option<String>,
}

impl Command for RestorePrep {
    fn name(&self) -> &'static str {
        "prepare.restore"
    }

    fn to_text(&self) -> Option<String> {
        self.text.clone()
    }

    fn apply(&self, doc: &mut Document) -> Result<Box<dyn Command>, CommandError> {
        let o = doc
            .object_mut(self.id)
            .ok_or(CommandError::NoSuchObject(self.id))?;
        let mut undo = RestorePrep {
            id: self.id,
            ..Default::default()
        };
        if let Some(t) = self.translation {
            undo.translation = Some(o.transform.translation);
            o.transform.translation = t;
        }
        if let Some(s) = &self.supports {
            undo.supports = Some(std::mem::replace(&mut o.prep.supports, s.clone()));
        }
        if let Some(c) = &self.cavity {
            undo.cavity = Some(std::mem::replace(&mut o.prep.cavity, c.clone()));
        }
        if let Some(h) = &self.holes {
            undo.holes = Some(std::mem::replace(&mut o.prep.holes, h.clone()));
        }
        Ok(Box::new(undo))
    }
}

/// A BVH of the last object asked for, so clicking supports one by one stays fast.
/// The last BVH built: mesh address, transform bits, tree.
type CachedBvh = (usize, [u32; 16], Arc<Bvh>);
static BVH_CACHE: Mutex<Option<CachedBvh>> = Mutex::new(None);

/// The object's triangles in world space, as a BVH (cached).
pub fn object_bvh(o: &SceneObject) -> Arc<Bvh> {
    let m = o.transform.matrix();
    bvh_at(&o.mesh, m)
}

fn bvh_at(mesh: &Arc<Mesh>, m: Mat4) -> Arc<Bvh> {
    let key = (
        Arc::as_ptr(mesh) as usize,
        m.to_cols_array().map(f32::to_bits),
    );
    if let Ok(cache) = BVH_CACHE.lock()
        && let Some((p, t, b)) = cache.as_ref()
        && *p == key.0
        && *t == key.1
    {
        return b.clone();
    }
    let b = Arc::new(Bvh::from_mesh(mesh, &m));
    if let Ok(mut cache) = BVH_CACHE.lock() {
        *cache = Some((key.0, key.1, b.clone()));
    }
    b
}

/// Height of the model bottom when it stands on supports.
fn lift_target(doc: &Document) -> f32 {
    let s = &doc.settings.supports;
    s.lift_height_mm + if s.raft { s.raft_thickness_mm } else { 0.0 }
}

/// Computes automatic supports for one object: returns the supports and the object position
/// (lifted off the plate if it was lower than the lift height). Manual supports stay.
pub fn auto_supports(
    doc: &Document,
    id: ObjectId,
    mode: Mode,
) -> Result<(SupportSet, Vec3), CommandError> {
    let o = doc.object(id).ok_or(CommandError::NoSuchObject(id))?;
    let mut position = o.transform.translation;
    let b = o
        .world_bounds()
        .ok_or_else(|| CommandError::Invalid(format!("object {id} is empty")))?;
    let target = lift_target(doc);
    if b.min.z < target - 1e-3 {
        position.z += target - b.min.z;
    }
    let mut t = o.transform;
    t.translation = position;
    let bvh = bvh_at(&o.mesh, t.matrix());
    let set = copal_supports::generate(
        &bvh,
        position,
        &doc.settings.supports,
        mode,
        o.prep.supports.as_deref(),
    );
    Ok((set, position))
}

/// `supports.auto id=<id> [mode=auto|islands]`
#[derive(Debug, Clone)]
pub struct AutoSupports {
    pub id: ObjectId,
    pub mode: Mode,
    /// A result computed in the background (see [`auto_supports`]).
    pub precomputed: Option<(Arc<SupportSet>, Vec3)>,
}

impl Command for AutoSupports {
    fn name(&self) -> &'static str {
        "supports.auto"
    }

    fn to_text(&self) -> Option<String> {
        Some(match self.mode {
            Mode::Auto => format!("supports.auto id={}", self.id),
            m => format!("supports.auto id={} mode={}", self.id, m.name()),
        })
    }

    fn apply(&self, doc: &mut Document) -> Result<Box<dyn Command>, CommandError> {
        let (set, position) = match &self.precomputed {
            Some((s, p)) => (s.clone(), *p),
            None => {
                let (s, p) = auto_supports(doc, self.id, self.mode)?;
                (Arc::new(s), p)
            }
        };
        RestorePrep {
            id: self.id,
            translation: Some(position),
            supports: Some(Some(set)),
            ..Default::default()
        }
        .apply(doc)
    }
}

/// Changes the supports of an object with `f`, as one undo step.
#[derive(Debug, Clone)]
pub struct EditSupports {
    pub id: ObjectId,
    pub text: String,
    pub op: SupportOp,
}

#[derive(Debug, Clone)]
pub enum SupportOp {
    /// A surface point (world space).
    Add {
        point: Vec3,
    },
    Remove(Vec<u32>),
    Clear,
    Resize {
        ids: Vec<u32>,
        diameter: Option<f32>,
        tip: Option<f32>,
    },
}

impl Command for EditSupports {
    fn name(&self) -> &'static str {
        match self.op {
            SupportOp::Add { .. } => "supports.add",
            SupportOp::Remove(_) => "supports.remove",
            SupportOp::Clear => "supports.clear",
            SupportOp::Resize { .. } => "supports.resize",
        }
    }

    fn to_text(&self) -> Option<String> {
        Some(self.text.clone())
    }

    fn apply(&self, doc: &mut Document) -> Result<Box<dyn Command>, CommandError> {
        let settings = doc.settings.supports;
        let target = lift_target(doc);
        let o = doc
            .object(self.id)
            .ok_or(CommandError::NoSuchObject(self.id))?;
        let mut position = o.transform.translation;
        let mut set = o.prep.supports.as_deref().cloned().unwrap_or_default();
        match &self.op {
            SupportOp::Add { point } => {
                let bvh = object_bvh(o);
                let (p, n) = bvh.closest_point(*point, 2.0).ok_or_else(|| {
                    CommandError::Invalid("that point is not on the model".into())
                })?;
                let mut p = p;
                // The first support lifts a model that stands on the plate.
                if set.is_empty()
                    && let Some(b) = o.world_bounds()
                    && b.min.z < target - 1e-3
                {
                    let dz = target - b.min.z;
                    position.z += dz;
                    p.z += dz;
                }
                let mut t = o.transform;
                t.translation = position;
                let bvh = bvh_at(&o.mesh, t.matrix());
                copal_supports::add_manual(
                    &mut set,
                    &bvh,
                    position,
                    p,
                    n,
                    &settings,
                    settings.manual_bracing,
                )
                .ok_or_else(|| {
                    CommandError::Invalid(
                        "no room for a support there (it would go through the model)".into(),
                    )
                })?;
            }
            SupportOp::Remove(ids) => {
                if set.remove(ids) == 0 {
                    return Err(CommandError::Invalid("no such supports".into()));
                }
                set.rebuild_raft(
                    settings.raft && !set.raft.is_empty(),
                    settings.raft_margin_mm,
                );
            }
            SupportOp::Clear => set = SupportSet::default(),
            SupportOp::Resize { ids, diameter, tip } => {
                let mut n = 0;
                for s in set.supports.iter_mut().filter(|s| ids.contains(&s.id)) {
                    if let Some(d) = diameter {
                        s.diameter = *d;
                    }
                    if let Some(t) = tip {
                        s.tip_diameter = *t;
                    }
                    n += 1;
                }
                if n == 0 {
                    return Err(CommandError::Invalid("no such supports".into()));
                }
            }
        }
        let supports = (!set.is_empty()).then(|| Arc::new(set));
        RestorePrep {
            id: self.id,
            translation: Some(position),
            supports: Some(supports),
            ..Default::default()
        }
        .apply(doc)
    }
}

/// Computes the inner wall of a hollowed object, in the object's own coordinates.
pub fn hollow_object(
    doc: &Document,
    id: ObjectId,
    thickness: f32,
    voxel: f32,
) -> Result<Mesh, CommandError> {
    let o = doc.object(id).ok_or(CommandError::NoSuchObject(id))?;
    // Work at real size (rotation and scale applied), then go back to the object's coordinates.
    let mut t = o.transform;
    t.translation = Vec3::ZERO;
    let m = t.matrix();
    let bvh = Bvh::from_mesh(&o.mesh, &m);
    let cavity =
        copal_supports::hollow::hollow(bvh.triangles(), thickness, voxel).ok_or_else(|| {
            CommandError::Invalid(format!(
                "object {id} is too thin to hollow with {thickness} mm walls"
            ))
        })?;
    let mut local = cavity.transformed(&m.inverse());
    // `transformed` flips triangles for mirrored matrices; the cavity must stay inside out.
    if m.determinant() < 0.0 {
        for tri in &mut local.triangles {
            tri.swap(1, 2);
        }
    }
    Ok(local)
}

/// `object.hollow id=<id> [thickness=<mm>] [voxel=<mm>]`
#[derive(Debug, Clone)]
pub struct Hollow {
    pub id: ObjectId,
    pub thickness: f32,
    pub voxel: f32,
    pub precomputed: Option<Arc<Mesh>>,
}

impl Command for Hollow {
    fn name(&self) -> &'static str {
        "object.hollow"
    }

    fn to_text(&self) -> Option<String> {
        Some(format!(
            "object.hollow id={} thickness={} voxel={}",
            self.id, self.thickness, self.voxel
        ))
    }

    fn apply(&self, doc: &mut Document) -> Result<Box<dyn Command>, CommandError> {
        let cavity = match &self.precomputed {
            Some(c) => c.clone(),
            None => Arc::new(hollow_object(doc, self.id, self.thickness, self.voxel)?),
        };
        RestorePrep {
            id: self.id,
            cavity: Some(Some(cavity)),
            ..Default::default()
        }
        .apply(doc)
    }
}

/// `holes.add`, `holes.remove`, `holes.clear`, `object.unhollow`.
#[derive(Debug, Clone)]
pub enum HoleOp {
    Add {
        point: Vec3,
        diameter: f32,
        depth: f32,
    },
    Remove(usize),
    Clear,
    Unhollow,
}

#[derive(Debug, Clone)]
pub struct EditHoles {
    pub id: ObjectId,
    pub op: HoleOp,
    pub text: String,
}

impl Command for EditHoles {
    fn name(&self) -> &'static str {
        match self.op {
            HoleOp::Add { .. } => "holes.add",
            HoleOp::Remove(_) => "holes.remove",
            HoleOp::Clear => "holes.clear",
            HoleOp::Unhollow => "object.unhollow",
        }
    }

    fn to_text(&self) -> Option<String> {
        Some(self.text.clone())
    }

    fn apply(&self, doc: &mut Document) -> Result<Box<dyn Command>, CommandError> {
        let o = doc
            .object(self.id)
            .ok_or(CommandError::NoSuchObject(self.id))?;
        let mut holes = o.prep.holes.clone();
        let mut cavity = None;
        match &self.op {
            HoleOp::Add {
                point,
                diameter,
                depth,
            } => {
                let bvh = object_bvh(o);
                let (p, n) = bvh.closest_point(*point, 2.0).ok_or_else(|| {
                    CommandError::Invalid("that point is not on the model".into())
                })?;
                let inv = o.transform.matrix().inverse();
                let lp = inv.transform_point3(p);
                let ld = inv.transform_vector3(-n).normalize_or(Vec3::NEG_Z);
                holes.push(DrainHole {
                    point: lp,
                    dir: ld,
                    diameter: *diameter,
                    depth: *depth,
                });
            }
            HoleOp::Remove(i) => {
                if *i >= holes.len() {
                    return Err(CommandError::Invalid(format!("there is no hole {i}")));
                }
                holes.remove(*i);
            }
            HoleOp::Clear => holes.clear(),
            HoleOp::Unhollow => {
                if o.prep.cavity.is_none() {
                    return Err(CommandError::Invalid(format!(
                        "object {} is not hollow",
                        self.id
                    )));
                }
                cavity = Some(None);
            }
        }
        RestorePrep {
            id: self.id,
            holes: Some(holes),
            cavity,
            ..Default::default()
        }
        .apply(doc)
    }
}

fn ids_arg(a: &Args, key: &str) -> Result<Vec<u32>, CommandError> {
    let raw = a.str(key)?;
    raw.split(',')
        .map(|p| {
            p.trim().parse::<u32>().map_err(|_| CommandError::BadValue {
                command: a.command().to_string(),
                arg: key.to_string(),
                value: raw.to_string(),
                reason: "expected numbers separated by commas".into(),
            })
        })
        .collect()
}

fn point_arg(a: &Args) -> Result<Vec3, CommandError> {
    Ok(Vec3::new(a.get("x")?, a.get("y")?, a.get("z")?))
}

pub(crate) fn build_supports_auto(a: &Args) -> Result<Box<dyn Command>, CommandError> {
    a.only(&["id", "mode"])?;
    let mode = match a.opt_str("mode") {
        None => Mode::Auto,
        Some(m) => Mode::parse(m).ok_or_else(|| CommandError::BadValue {
            command: "supports.auto".into(),
            arg: "mode".into(),
            value: m.into(),
            reason: "expected auto or islands".into(),
        })?,
    };
    Ok(Box::new(AutoSupports {
        id: ObjectId(a.get("id")?),
        mode,
        precomputed: None,
    }))
}

fn line(a: &Args) -> String {
    let mut s = a.command().to_string();
    for k in a.keys() {
        s.push_str(&format!(
            " {k}={}",
            crate::script::quote(a.opt_str(k).unwrap_or(""))
        ));
    }
    s
}

pub(crate) fn build_supports_add(a: &Args) -> Result<Box<dyn Command>, CommandError> {
    a.only(&["id", "x", "y", "z"])?;
    Ok(Box::new(EditSupports {
        id: ObjectId(a.get("id")?),
        text: line(a),
        op: SupportOp::Add {
            point: point_arg(a)?,
        },
    }))
}

pub(crate) fn build_supports_remove(a: &Args) -> Result<Box<dyn Command>, CommandError> {
    a.only(&["id", "sid"])?;
    Ok(Box::new(EditSupports {
        id: ObjectId(a.get("id")?),
        text: line(a),
        op: SupportOp::Remove(ids_arg(a, "sid")?),
    }))
}

pub(crate) fn build_supports_clear(a: &Args) -> Result<Box<dyn Command>, CommandError> {
    a.only(&["id"])?;
    Ok(Box::new(EditSupports {
        id: ObjectId(a.get("id")?),
        text: line(a),
        op: SupportOp::Clear,
    }))
}

pub(crate) fn build_supports_resize(a: &Args) -> Result<Box<dyn Command>, CommandError> {
    a.only(&["id", "sid", "diameter", "tip"])?;
    let diameter: Option<f32> = a.opt("diameter")?;
    let tip: Option<f32> = a.opt("tip")?;
    if diameter.is_none() && tip.is_none() {
        return Err(CommandError::Invalid(
            "give diameter=<mm> and/or tip=<mm>".into(),
        ));
    }
    if diameter.is_some_and(|d| !(0.2..=10.0).contains(&d))
        || tip.is_some_and(|d| !(0.05..=5.0).contains(&d))
    {
        return Err(CommandError::Invalid(
            "diameter must be 0.2–10 mm, tip 0.05–5 mm".into(),
        ));
    }
    Ok(Box::new(EditSupports {
        id: ObjectId(a.get("id")?),
        text: line(a),
        op: SupportOp::Resize {
            ids: ids_arg(a, "sid")?,
            diameter,
            tip,
        },
    }))
}

pub(crate) fn build_hollow(a: &Args) -> Result<Box<dyn Command>, CommandError> {
    a.only(&["id", "thickness", "voxel"])?;
    let thickness: f32 = a.opt("thickness")?.unwrap_or(2.5);
    let voxel: f32 = a.opt("voxel")?.unwrap_or(0.5);
    if !(0.5..=20.0).contains(&thickness) || !(0.05..=5.0).contains(&voxel) {
        return Err(CommandError::Invalid(
            "thickness must be 0.5–20 mm and voxel 0.05–5 mm".into(),
        ));
    }
    Ok(Box::new(Hollow {
        id: ObjectId(a.get("id")?),
        thickness,
        voxel,
        precomputed: None,
    }))
}

pub(crate) fn build_unhollow(a: &Args) -> Result<Box<dyn Command>, CommandError> {
    a.only(&["id"])?;
    Ok(Box::new(EditHoles {
        id: ObjectId(a.get("id")?),
        op: HoleOp::Unhollow,
        text: line(a),
    }))
}

pub(crate) fn build_holes_add(a: &Args) -> Result<Box<dyn Command>, CommandError> {
    a.only(&["id", "x", "y", "z", "diameter", "depth"])?;
    Ok(Box::new(EditHoles {
        id: ObjectId(a.get("id")?),
        op: HoleOp::Add {
            point: point_arg(a)?,
            diameter: a.opt("diameter")?.unwrap_or(3.0),
            depth: a.opt("depth")?.unwrap_or(4.0),
        },
        text: line(a),
    }))
}

pub(crate) fn build_holes_remove(a: &Args) -> Result<Box<dyn Command>, CommandError> {
    a.only(&["id", "index"])?;
    Ok(Box::new(EditHoles {
        id: ObjectId(a.get("id")?),
        op: HoleOp::Remove(a.get("index")?),
        text: line(a),
    }))
}

pub(crate) fn build_holes_clear(a: &Args) -> Result<Box<dyn Command>, CommandError> {
    a.only(&["id"])?;
    Ok(Box::new(EditHoles {
        id: ObjectId(a.get("id")?),
        op: HoleOp::Clear,
        text: line(a),
    }))
}

/// Islands of an object that no support touches. Each needs a support or the print fails there.
pub fn unsupported_islands(
    doc: &Document,
    id: ObjectId,
) -> Result<Vec<copal_supports::Island>, CommandError> {
    let o = doc.object(id).ok_or(CommandError::NoSuchObject(id))?;
    let bvh = object_bvh(o);
    let islands = copal_supports::find_islands(&bvh, 0.05, 0.1, 3.0);
    let tips: Vec<Vec3> = o
        .prep
        .supports
        .iter()
        .flat_map(|s| s.supports.iter().map(|x| x.tip + o.transform.translation))
        .collect();
    let on_plate = o.world_bounds().is_some_and(|b| b.min.z < 0.05);
    Ok(islands
        .into_iter()
        .filter(|i| {
            // The first layer of a model that stands on the plate is held by the plate.
            if on_plate && i.z < 0.05 {
                return false;
            }
            !tips.iter().any(|t| {
                t.x >= i.min.x - 0.6
                    && t.x <= i.max.x + 0.6
                    && t.y >= i.min.y - 0.6
                    && t.y <= i.max.y + 0.6
                    && t.z >= i.z - 0.4
                    && t.z <= i.z + 1.5
            })
        })
        .collect())
}
