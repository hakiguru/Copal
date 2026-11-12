//! Commands of the Prepare step: supports, hollowing and drain holes.
//!
//! The heavy work (`auto_supports`, `hollow_object`) is public so an app can run it in the
//! background on a copy of the document and then apply the result as a command.

use std::sync::{Arc, Mutex};

use copal_mesh::Mesh;
use copal_mesh::bvh::Bvh;
use copal_supports::{DrainHole, Goal, Mode, OrientStats, SupportSet, Target};
use glam::{EulerRot, Mat4, Vec3};

use crate::command::{Command, CommandError};
use crate::document::{Document, ObjectId, SceneObject, Transform};
use crate::script::Args;

/// Puts back what a prepare command changed. Fields that are `None` stay as they are.
#[derive(Debug, Clone, Default)]
pub struct RestorePrep {
    pub id: ObjectId,
    pub translation: Option<Vec3>,
    /// Position, rotation and scale together (wins over `translation`).
    pub transform: Option<Transform>,
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
        if let Some(t) = self.transform {
            undo.transform = Some(o.transform);
            o.transform = t;
        } else if let Some(t) = self.translation {
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

/// The last few BVHs built, so clicking supports one by one stays fast: the mesh (kept alive,
/// so its address is not given to another mesh meanwhile), transform bits, tree. The newest
/// is last.
type CachedBvh = (Arc<Mesh>, [u32; 16], Arc<Bvh>);
static BVH_CACHE: Mutex<Vec<CachedBvh>> = Mutex::new(Vec::new());

/// The object's triangles in world space, as a BVH (cached).
pub fn object_bvh(o: &SceneObject) -> Arc<Bvh> {
    let m = o.transform.matrix();
    bvh_at(&o.mesh, m)
}

fn bvh_at(mesh: &Arc<Mesh>, m: Mat4) -> Arc<Bvh> {
    let bits = m.to_cols_array().map(f32::to_bits);
    if let Ok(mut cache) = BVH_CACHE.lock()
        && let Some(i) = cache
            .iter()
            .position(|(p, t, _)| Arc::ptr_eq(p, mesh) && *t == bits)
    {
        let hit = cache.remove(i);
        let b = hit.2.clone();
        cache.push(hit);
        return b;
    }
    let b = Arc::new(Bvh::from_mesh(mesh, &m));
    if let Ok(mut cache) = BVH_CACHE.lock() {
        if cache.len() >= 4 {
            cache.remove(0);
        }
        cache.push((mesh.clone(), bits, b.clone()));
    }
    b
}

/// An object as the support generator sees it: its outside, the wall of its hollow and the
/// mouths of its drain holes, all in world space with the object at `transform`.
struct PrepScene {
    body: Arc<Bvh>,
    cavity: Option<Arc<Bvh>>,
    avoid: Vec<(Vec3, f32)>,
    holes: Vec<[Vec3; 3]>,
}

impl PrepScene {
    fn new(o: &SceneObject, transform: Transform) -> Self {
        let m = transform.matrix();
        PrepScene {
            body: bvh_at(&o.mesh, m),
            cavity: o.prep.cavity.as_ref().map(|c| bvh_at(c, m)),
            avoid: o
                .prep
                .holes
                .iter()
                .map(|h| (m.transform_point3(h.point), h.diameter / 2.0))
                .collect(),
            holes: o
                .prep
                .holes
                .iter()
                .flat_map(|h| {
                    let hole = copal_supports::hollow::drain_hole(
                        m.transform_point3(h.point),
                        m.transform_vector3(h.dir).normalize_or(Vec3::NEG_Z),
                        h.diameter,
                        h.depth,
                    );
                    hole.triangles
                        .iter()
                        .map(|t| t.map(|i| hole.positions[i as usize]))
                        .collect::<Vec<_>>()
                })
                .collect(),
        }
    }

    fn target(&self) -> Target<'_> {
        Target {
            body: &self.body,
            cavity: self.cavity.as_deref(),
            avoid: &self.avoid,
            holes: &self.holes,
        }
    }
}

/// How far a hole that starts at `point` and goes along `dir` has to go to open the hollow
/// `cavity` over most of its width; `None` when it does not get there within `limit`. Several
/// rays, because a single one can slip between two triangles.
fn hole_reach(cavity: &Bvh, point: Vec3, dir: Vec3, radius: f32, limit: f32) -> Option<(f32, u32)> {
    let u = if dir.x.abs() < 0.9 { Vec3::X } else { Vec3::Y };
    let u = (u - dir * u.dot(dir)).normalize_or(Vec3::Y);
    let v = dir.cross(u);
    let r = radius * 0.4;
    let mut best: Option<(f32, u32)> = None;
    for off in [Vec3::ZERO, u * r, -u * r, v * r, -v * r, (u + v) * r * 0.5] {
        if let Some(hit) = cavity.raycast(point + off, dir, 0.0, limit)
            && best.is_none_or(|(t, _)| hit.t > t)
        {
            best = Some((hit.t, hit.triangle));
        }
    }
    best
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
    let scene = PrepScene::new(o, t);
    let set = copal_supports::generate_for(
        &scene.target(),
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
                let scene = PrepScene::new(o, t);
                copal_supports::add_manual_for(
                    &mut set,
                    &scene.target(),
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

/// What automatic orientation found for an object.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrientResult {
    /// Where the object goes: turned, and back on the plate at the same place.
    pub transform: Transform,
    /// The numbers of the orientation it has now.
    pub before: OrientStats,
    /// The numbers of the new orientation.
    pub after: OrientStats,
}

/// Finds the best way to turn an object for printing (see `copal_supports::orient`).
pub fn orient_object(
    doc: &Document,
    id: ObjectId,
    goal: Goal,
) -> Result<OrientResult, CommandError> {
    let o = doc.object(id).ok_or(CommandError::NoSuchObject(id))?;
    // The model at its real size, not turned yet.
    let unturned = Transform {
        translation: Vec3::ZERO,
        rotation_deg: Vec3::ZERO,
        scale: o.transform.scale,
    };
    let bvh = Bvh::from_mesh(&o.mesh, &unturned.matrix());
    let s = &doc.settings.supports;
    let analyzer =
        copal_supports::Analyzer::new(bvh.triangles(), s.overhang_angle_deg.to_radians().cos())
            .ok_or_else(|| CommandError::Invalid(format!("object {id} is empty")))?;
    // Room for the lift and the raft under the model, and for the raft around it.
    let mut volume = doc.build_volume();
    volume.z -= lift_target(doc);
    let margin = if s.raft { 2.0 * s.raft_margin_mm } else { 0.0 };
    volume.x -= margin;
    volume.y -= margin;
    let best = analyzer.best(goal, Some(volume)).ok_or_else(|| {
        CommandError::Invalid(format!(
            "object {id} does not fit the printer any way round: scale it down or cut it"
        ))
    })?;
    let r = o.transform.rotation_deg;
    let now = glam::Quat::from_euler(
        EulerRot::XYZ,
        r.x.to_radians(),
        r.y.to_radians(),
        r.z.to_radians(),
    );
    let before = analyzer.stats(now.inverse() * Vec3::NEG_Z);
    let (x, y, z) = best.rotation.to_euler(EulerRot::XYZ);
    let round = |a: f32| (a.to_degrees() * 100.0).round() / 100.0;
    let mut turned = o.clone();
    turned.transform.rotation_deg = Vec3::new(round(x), round(y), round(z));
    // Same place on the plate, standing on it.
    if let (Some(old), Some(new)) = (o.world_bounds(), turned.world_bounds()) {
        let (co, cn) = (old.center(), new.center());
        turned.transform.translation += Vec3::new(co.x - cn.x, co.y - cn.y, -new.min.z);
    }
    Ok(OrientResult {
        transform: turned.transform,
        before,
        after: best.stats,
    })
}

/// `object.orient id=<id> [goal=balanced|strong|supports|fast]`
#[derive(Debug, Clone)]
pub struct Orient {
    pub id: ObjectId,
    pub goal: Goal,
    /// A result computed in the background (see [`orient_object`]).
    pub precomputed: Option<Transform>,
}

impl Command for Orient {
    fn name(&self) -> &'static str {
        "object.orient"
    }

    fn to_text(&self) -> Option<String> {
        Some(match self.goal {
            Goal::Balanced => format!("object.orient id={}", self.id),
            g => format!("object.orient id={} goal={}", self.id, g.name()),
        })
    }

    fn apply(&self, doc: &mut Document) -> Result<Box<dyn Command>, CommandError> {
        let transform = match self.precomputed {
            Some(t) => t,
            None => orient_object(doc, self.id, self.goal)?.transform,
        };
        // Supports were made for the old orientation.
        RestorePrep {
            id: self.id,
            transform: Some(transform),
            supports: Some(None),
            ..Default::default()
        }
        .apply(doc)
    }
}

fn goal_arg(a: &Args, key: &str) -> Result<Option<Goal>, CommandError> {
    match a.opt_str(key) {
        None => Ok(None),
        Some(g) => Goal::parse(g)
            .map(Some)
            .ok_or_else(|| CommandError::BadValue {
                command: a.command().to_string(),
                arg: key.into(),
                value: g.into(),
                reason: "expected balanced, strong, supports or fast".into(),
            }),
    }
}

pub(crate) fn build_orient(a: &Args) -> Result<Box<dyn Command>, CommandError> {
    a.only(&["id", "goal"])?;
    Ok(Box::new(Orient {
        id: ObjectId(a.get("id")?),
        goal: goal_arg(a, "goal")?.unwrap_or_default(),
        precomputed: None,
    }))
}

/// How the drain holes of an object do their job.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct HoleCheck {
    /// Separate hollows in the object.
    pub cavities: usize,
    /// Hollows that no hole opens: resin stays inside and the print pulls on the film like a
    /// suction cup.
    pub closed: usize,
    /// Hollows with a single hole: they drain, but slowly, because no air gets in.
    pub single: usize,
    /// Holes (numbers start at 0) that end in the wall and open nothing.
    pub blind: Vec<usize>,
}

impl HoleCheck {
    pub fn ok(&self) -> bool {
        self.closed == 0 && self.blind.is_empty()
    }

    /// One line for people, e.g. "2 holes open the hollow".
    pub fn summary(&self, holes: usize) -> String {
        if self.cavities == 0 {
            return if holes == 0 {
                "solid, no drain holes".into()
            } else {
                format!("solid: {holes} holes do nothing until the model is hollow")
            };
        }
        let mut parts = Vec::new();
        if self.closed > 0 {
            parts.push(if self.cavities == 1 {
                "the hollow has no drain hole: resin stays inside".to_string()
            } else {
                format!(
                    "{} of {} hollows have no drain hole: resin stays inside",
                    self.closed, self.cavities
                )
            });
        }
        if !self.blind.is_empty() {
            parts.push(format!(
                "{} holes end in the wall: make them deeper or move them",
                self.blind.len()
            ));
        }
        if self.single > 0 && self.closed == 0 {
            parts.push("add a second hole so air gets in while resin runs out".into());
        }
        if parts.is_empty() {
            format!("{holes} holes open the hollow")
        } else {
            parts.join("; ")
        }
    }
}

/// Checks that every hollow of an object is opened by its drain holes.
pub fn hole_check(doc: &Document, id: ObjectId) -> Result<HoleCheck, CommandError> {
    let o = doc.object(id).ok_or(CommandError::NoSuchObject(id))?;
    let Some(cavity) = &o.prep.cavity else {
        return Ok(HoleCheck::default());
    };
    let parts = copal_supports::hollow::cavity_parts(cavity);
    let mut part_of = vec![usize::MAX; cavity.positions.len()];
    for (k, part) in parts.iter().enumerate() {
        for v in part {
            part_of[*v as usize] = k;
        }
    }
    // In the object's own coordinates, where the holes are stored (depths are real sizes, so
    // this is exact for objects that are not scaled unevenly).
    let bvh = bvh_at(cavity, Mat4::IDENTITY);
    let scale = o.transform.scale.abs().max_element().max(1e-6);
    let mut opened = vec![0usize; parts.len()];
    let mut blind = Vec::new();
    for (i, h) in o.prep.holes.iter().enumerate() {
        let dir = h.dir.normalize_or(Vec3::NEG_Z);
        let from = h.point - dir * 0.5 / scale;
        let radius = h.diameter / 2.0 / scale;
        match hole_reach(&bvh, from, dir, radius, (h.depth + 0.5) / scale) {
            Some((_, triangle)) => {
                let v = cavity.triangles[triangle as usize][0];
                opened[part_of[v as usize]] += 1;
            }
            None => blind.push(i),
        }
    }
    Ok(HoleCheck {
        cavities: parts.len(),
        closed: opened.iter().filter(|n| **n == 0).count(),
        single: opened.iter().filter(|n| **n == 1).count(),
        blind,
    })
}

/// What `object.prepare` does. Each step can be left out.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PrepareOptions {
    /// Turn the model to the best orientation for this goal.
    pub orient: Option<Goal>,
    /// Hollow the model with the project's wall thickness.
    pub hollow: bool,
    /// Add drain holes to the hollow.
    pub holes: bool,
    /// Add supports.
    pub supports: Option<Mode>,
}

impl PrepareOptions {
    /// Orientation, drain holes and supports; hollowing when the project asks for it.
    pub fn for_project(doc: &Document) -> Self {
        Self {
            orient: Some(Goal::Balanced),
            hollow: doc.settings.hollow.enabled,
            holes: true,
            supports: Some(Mode::Auto),
        }
    }

    fn text(&self) -> String {
        format!(
            "orient={} hollow={} holes={} supports={}",
            self.orient.map_or("off", Goal::name),
            self.hollow,
            self.holes,
            self.supports.map_or("off", Mode::name),
        )
    }
}

/// An object made ready to print, as computed by [`prepare_object`].
#[derive(Debug, Clone)]
pub struct PreparedObject {
    pub transform: Transform,
    pub prep: crate::document::Prepared,
    /// What was done, one line per step, for the console.
    pub notes: Vec<String>,
    /// Problems that are left, e.g. islands without support.
    pub warnings: Vec<String>,
}

/// Makes one object ready to print: orientation, hollow, drain holes, supports, and a check at
/// the end. Nothing is changed; apply the result with [`AutoPrepare`].
pub fn prepare_object(
    doc: &Document,
    id: ObjectId,
    opts: PrepareOptions,
) -> Result<PreparedObject, CommandError> {
    let mut work = doc.clone();
    let (mut notes, mut warnings) = (Vec::new(), Vec::new());
    let mut turned = false;
    if let Some(goal) = opts.orient {
        let found = orient_object(&work, id, goal)?;
        let now = work.object(id).map(|o| o.transform.rotation_deg);
        turned = now != Some(found.transform.rotation_deg);
        Orient {
            id,
            goal,
            precomputed: Some(found.transform),
        }
        .apply(&mut work)?;
        let r = found.transform.rotation_deg;
        notes.push(format!(
"turned to x={:.0}° y={:.0}° z={:.0}°: biggest layer {:.0} → {:.0} mm², overhangs {:.0} → {:.0} mm², height {:.0} → {:.0} mm",
            r.x,
            r.y,
            r.z,
            found.before.peak_area,
            found.after.peak_area,
            found.before.overhang_area,
            found.after.overhang_area,
            found.before.height,
            found.after.height,
        ));
    }
    let h = work.settings.hollow;
    if opts.hollow && work.object(id).is_some_and(|o| o.prep.cavity.is_none()) {
        match hollow_object_detailed(&work, id, h.wall_thickness_mm, h.voxel_mm) {
            Ok(done) => {
                notes.push(format!(
                    "hollow with {} mm walls: {:.1} ml of resin saved",
                    h.wall_thickness_mm, done.volume_ml
                ));
                Hollow {
                    id,
                    thickness: h.wall_thickness_mm,
                    voxel: h.voxel_mm,
                    precomputed: Some(Arc::new(done.cavity)),
                }
                .apply(&mut work)?;
            }
            // A model too thin to hollow prints fine solid.
            Err(e) => notes.push(format!("left solid: {e}")),
        }
    }
    let hollow = work.object(id).is_some_and(|o| o.prep.cavity.is_some());
    if opts.holes && hollow {
        // Holes made for another orientation are no longer at the lowest points.
        let stale = turned || work.object(id).is_some_and(|o| o.prep.holes.is_empty());
        if stale {
            let edit = |op: HoleOp| EditHoles {
                id,
                op,
                text: String::new(),
            };
            edit(HoleOp::Clear).apply(&mut work)?;
            // Standing on supports, so the holes come out where they will print.
            if let Some(o) = work.object_mut(id)
                && let Some(b) = o.world_bounds()
            {
                o.transform.translation.z += lift_target(doc) - b.min.z;
            }
            match edit(HoleOp::Auto {
                count: 2,
                diameter: None,
            })
            .apply(&mut work)
            {
                Ok(_) => notes.push(format!(
                    "{} drain holes at the lowest points of the hollow",
                    work.object(id).map_or(0, |o| o.prep.holes.len())
                )),
                Err(e) => warnings.push(e.to_string()),
            }
        }
    }
    if hollow {
        let check = hole_check(&work, id)?;
        if !check.ok() {
            let holes = work.object(id).map_or(0, |o| o.prep.holes.len());
            warnings.push(check.summary(holes));
        }
    }
    if let Some(mode) = opts.supports {
        let (set, position) = auto_supports(&work, id, mode)?;
        notes.push(format!(
            "{} supports, {} braces",
            set.supports.len(),
            set.braces.len()
        ));
        AutoSupports {
            id,
            mode,
            precomputed: Some((Arc::new(set), position)),
        }
        .apply(&mut work)?;
        let left = unsupported_islands(&work, id)?;
        if left.is_empty() {
            notes.push("every island is supported".into());
        } else {
            warnings.push(format!(
                "{} islands have no support: add supports by hand there (Check islands shows them)",
                left.len()
            ));
        }
    }
    let o = work.object(id).ok_or(CommandError::NoSuchObject(id))?;
    Ok(PreparedObject {
        transform: o.transform,
        prep: o.prep.clone(),
        notes,
        warnings,
    })
}

/// `object.prepare id=<id> [orient=…] [hollow=…] [holes=…] [supports=…]`: everything a model
/// needs before printing, in one step.
#[derive(Debug, Clone)]
pub struct AutoPrepare {
    pub id: ObjectId,
    pub opts: PrepareOptions,
    /// A result computed in the background (see [`prepare_object`]).
    pub precomputed: Option<Arc<PreparedObject>>,
}

impl Command for AutoPrepare {
    fn name(&self) -> &'static str {
        "object.prepare"
    }

    fn to_text(&self) -> Option<String> {
        Some(format!(
            "object.prepare id={} {}",
            self.id,
            self.opts.text()
        ))
    }

    fn apply(&self, doc: &mut Document) -> Result<Box<dyn Command>, CommandError> {
        let done = match &self.precomputed {
            Some(p) => p.clone(),
            None => Arc::new(prepare_object(doc, self.id, self.opts)?),
        };
        RestorePrep {
            id: self.id,
            transform: Some(done.transform),
            supports: Some(done.prep.supports.clone()),
            cavity: Some(done.prep.cavity.clone()),
            holes: Some(done.prep.holes.clone()),
            ..Default::default()
        }
        .apply(doc)
    }
}

pub(crate) fn build_prepare(a: &Args) -> Result<Box<dyn Command>, CommandError> {
    a.only(&["id", "orient", "hollow", "holes", "supports"])?;
    let orient = match a.opt_str("orient") {
        None => Some(Goal::Balanced),
        Some("off" | "false" | "no") => None,
        Some(_) => goal_arg(a, "orient")?,
    };
    let supports = match a.opt_str("supports") {
        None | Some("true" | "on") => Some(Mode::Auto),
        Some("off" | "false" | "no") => None,
        Some(m) => Some(Mode::parse(m).ok_or_else(|| CommandError::BadValue {
            command: "object.prepare".into(),
            arg: "supports".into(),
            value: m.into(),
            reason: "expected auto, islands or off".into(),
        })?),
    };
    Ok(Box::new(AutoPrepare {
        id: ObjectId(a.get("id")?),
        opts: PrepareOptions {
            orient,
            hollow: a.opt("hollow")?.unwrap_or(false),
            holes: a.opt("holes")?.unwrap_or(true),
            supports,
        },
        precomputed: None,
    }))
}

/// What hollowing an object made.
#[derive(Debug, Clone)]
pub struct HollowResult {
    /// The inner wall, inside out, in the object's own coordinates.
    pub cavity: Mesh,
    /// The voxel size that was used (bigger than asked for on very big models).
    pub voxel: f32,
    /// Resin saved, in millilitres.
    pub volume_ml: f32,
    /// Separate hollows.
    pub cavities: usize,
    /// Small closed pockets that were filled again.
    pub pockets_filled: usize,
}

/// Computes the inner wall of a hollowed object, in the object's own coordinates.
pub fn hollow_object(
    doc: &Document,
    id: ObjectId,
    thickness: f32,
    voxel: f32,
) -> Result<Mesh, CommandError> {
    hollow_object_detailed(doc, id, thickness, voxel).map(|r| r.cavity)
}

/// Like [`hollow_object`], and also says what was done. Pockets smaller than the project's
/// `min_pocket_mm3` are filled again.
pub fn hollow_object_detailed(
    doc: &Document,
    id: ObjectId,
    thickness: f32,
    voxel: f32,
) -> Result<HollowResult, CommandError> {
    let o = doc.object(id).ok_or(CommandError::NoSuchObject(id))?;
    // Work at real size (rotation and scale applied), then go back to the object's coordinates.
    let mut t = o.transform;
    t.translation = Vec3::ZERO;
    let m = t.matrix();
    let bvh = Bvh::from_mesh(&o.mesh, &m);
    let done = copal_supports::hollow::hollow_detailed(
        bvh.triangles(),
        thickness,
        voxel,
        doc.settings.hollow.min_pocket_mm3,
    )
    .map_err(|e| {
        CommandError::Invalid(match e {
            copal_supports::hollow::HollowError::Empty => format!("object {id} is empty"),
            copal_supports::hollow::HollowError::TooThin => {
                format!("object {id} is too thin to hollow with {thickness} mm walls")
            }
        })
    })?;
    let mut local = done.mesh.transformed(&m.inverse());
    // `transformed` flips triangles for mirrored matrices; the cavity must stay inside out.
    if m.determinant() < 0.0 {
        for tri in &mut local.triangles {
            tri.swap(1, 2);
        }
    }
    Ok(HollowResult {
        cavity: local,
        voxel: done.voxel,
        volume_ml: done.volume / 1000.0,
        cavities: done.cavities,
        pockets_filled: done.pockets_filled,
    })
}

/// Overhangs of an object as it stands now: faces that point down more steeply than the overhang
/// angle, the lowest vertices and the edges that need holding. What the red faces in the 3D view
/// show, as numbers.
pub fn overhang_report(
    doc: &Document,
    id: ObjectId,
) -> Result<copal_supports::Overhangs, CommandError> {
    let o = doc.object(id).ok_or(CommandError::NoSuchObject(id))?;
    let bvh = object_bvh(o);
    let (lo, _) = bvh
        .bounds()
        .ok_or_else(|| CommandError::Invalid(format!("object {id} is empty")))?;
    let s = &doc.settings.supports;
    Ok(copal_supports::find_overhangs(
        bvh.triangles(),
        s.overhang_angle_deg.to_radians().cos(),
        copal_supports::spacing_for(s),
        lo.z,
    ))
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

/// `holes.add`, `holes.auto`, `holes.remove`, `holes.clear`, `object.unhollow`.
#[derive(Debug, Clone)]
pub enum HoleOp {
    /// `depth: None` makes the hole just deep enough to reach the hollow (or takes the depth
    /// from the settings when the model is solid).
    Add {
        point: Vec3,
        diameter: f32,
        depth: Option<f32>,
    },
    /// Holes at the lowest points of every separate hollow (`count` for each). `None` takes the
    /// diameter from the settings.
    Auto {
        count: usize,
        diameter: Option<f32>,
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
            HoleOp::Auto { .. } => "holes.auto",
            HoleOp::Remove(_) => "holes.remove",
            HoleOp::Clear => "holes.clear",
            HoleOp::Unhollow => "object.unhollow",
        }
    }

    fn to_text(&self) -> Option<String> {
        Some(self.text.clone())
    }

    fn apply(&self, doc: &mut Document) -> Result<Box<dyn Command>, CommandError> {
        let default_diameter = doc.settings.hollow.drain_hole_diameter_mm;
        let default_depth = doc.settings.hollow.drain_hole_depth_mm;
        let wall = doc.settings.hollow.wall_thickness_mm;
        let o = doc
            .object(self.id)
            .ok_or(CommandError::NoSuchObject(self.id))?;
        let mut holes = o.prep.holes.clone();
        let mut cavity = None;
        match &self.op {
            HoleOp::Auto { count, diameter } => {
                let hollow = o.prep.cavity.as_ref().ok_or_else(|| {
                    CommandError::Invalid(format!(
                        "object {} is not hollow: hollow it first (object.hollow id={})",
                        self.id, self.id
                    ))
                })?;
                let diameter = diameter.unwrap_or(default_diameter);
                let m = o.transform.matrix();
                let bvh = object_bvh(o);
                // Far enough apart that air comes in through one while resin leaves the other.
                let gap = (diameter * 4.0).max(8.0);
                let inv = m.inverse();
                let before = holes.len();
                for part in copal_supports::hollow::cavity_parts(hollow) {
                    let inside: Vec<Vec3> = part
                        .iter()
                        .map(|v| m.transform_point3(hollow.positions[*v as usize]))
                        .collect();
                    for h in copal_supports::hollow::auto_drain_holes(
                        &bvh, &inside, *count, diameter, gap,
                    ) {
                        holes.push(DrainHole {
                            point: inv.transform_point3(h.point),
                            dir: inv.transform_vector3(h.dir).normalize_or(Vec3::NEG_Z),
                            diameter,
                            depth: h.depth,
                        });
                    }
                }
                if holes.len() == before {
                    return Err(CommandError::Invalid(format!(
                        "found no place for a drain hole on object {}",
                        self.id
                    )));
                }
            }
            HoleOp::Add {
                point,
                diameter,
                depth,
            } => {
                let bvh = object_bvh(o);
                let (p, n) = bvh.closest_point(*point, 2.0).ok_or_else(|| {
                    CommandError::Invalid("that point is not on the model".into())
                })?;
                // Deep enough to open into the hollow, wherever the wall is thicker or the
                // hole goes in at an angle.
                let depth = depth.unwrap_or_else(|| {
                    o.prep
                        .cavity
                        .as_ref()
                        .and_then(|c| {
                            let limit = (wall * 4.0).max(default_depth * 2.0);
                            let cavity = bvh_at(c, o.transform.matrix());
                            hole_reach(&cavity, p, -n, *diameter / 2.0, limit)
                        })
                        .map(|(t, _)| t + (*diameter * 0.5).max(1.0))
                        .unwrap_or(default_depth)
                });
                let inv = o.transform.matrix().inverse();
                let lp = inv.transform_point3(p);
                let ld = inv.transform_vector3(-n).normalize_or(Vec3::NEG_Z);
                holes.push(DrainHole {
                    point: lp,
                    dir: ld,
                    diameter: *diameter,
                    depth,
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
            depth: a.opt("depth")?,
        },
        text: line(a),
    }))
}

pub(crate) fn build_holes_auto(a: &Args) -> Result<Box<dyn Command>, CommandError> {
    a.only(&["id", "count", "diameter"])?;
    let count: usize = a.opt("count")?.unwrap_or(2);
    let diameter: Option<f32> = a.opt("diameter")?;
    if !(1..=12).contains(&count) || diameter.is_some_and(|d| !(0.5..=20.0).contains(&d)) {
        return Err(CommandError::Invalid(
            "count must be 1 to 12 and diameter 0.5–20 mm".into(),
        ));
    }
    Ok(Box::new(EditHoles {
        id: ObjectId(a.get("id")?),
        op: HoleOp::Auto { count, diameter },
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
    let scene = PrepScene::new(o, o.transform);
    let target = scene.target();
    let cuts = target.cuts();
    let probe = |from: Vec3, dir: Vec3, t0: f32, t1: f32| target.raycast(from, dir, t0, t1);
    // Big models are tested on a coarser grid, so the check stays quick.
    let size = o.world_bounds().map(|b| b.size()).unwrap_or(Vec3::ONE);
    let pixel = (size.x.max(size.y) / 1500.0).max(0.1);
    let layer = (size.z / 3000.0).max(0.05);
    let islands = copal_supports::find_islands_in(target.body, &cuts, &probe, layer, pixel, 3.0);
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
