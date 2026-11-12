//! The Prepare step: supports (auto, islands only, by hand, select and delete), island check,
//! hollowing and drain holes. Heavy work runs in the background.

use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;
use std::thread::JoinHandle;

use copal_core::prepare::{
    AutoPrepare, AutoSupports, Hollow, HollowResult, Orient, OrientResult, PrepareOptions,
    PreparedObject,
};
use copal_core::presets::SupportSettings;
use copal_core::supports::{Goal, Island, Mode, SupportSet};
use copal_core::{Batch, Command, Document, ObjectId};
use copal_mesh::Mesh;
use egui::{DragValue, RichText, Ui};
use glam::Vec3;

use crate::camera::Ray;
use crate::look::Look;
use crate::panels::tab_button;
use crate::widgets::{self, ButtonKind};
use crate::workspace::{LineKind, Workspace};

/// What a click in the 3D view does in the Prepare step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PrepTool {
    /// Pick models, orbit the view.
    #[default]
    Look,
    /// Click supports to select them (Shift adds), Delete removes them.
    Select,
    /// Click on the model to add a support there.
    AddSupport,
    /// Click on the model to add a drain hole there.
    AddHole,
}

/// Supports made for one object in the background: the set and where the object goes.
type SupportJob = (ObjectId, Mode, Result<(SupportSet, Vec3), String>);

pub(crate) enum JobResult {
    Orient(Vec<(ObjectId, Goal, Result<OrientResult, String>)>),
    Prepare(Vec<(ObjectId, PrepareOptions, Result<PreparedObject, String>)>),
    Supports(Vec<SupportJob>),
    Hollow(Vec<(ObjectId, f32, f32, Result<HollowResult, String>)>),
    Islands(Vec<(ObjectId, Result<Vec<Island>, String>)>),
}

/// Work running in the background.
pub struct PrepJob {
    pub label: String,
    pub started: std::time::Instant,
    pub(crate) handle: Option<JoinHandle<JobResult>>,
}

/// A support mesh ready to draw, cached per object.
pub(crate) struct CachedSupports {
    key: (usize, u32),
    pub mesh: Arc<Mesh>,
    pub ranges: Vec<(u32, Range<usize>)>,
    /// Mesh of the selected supports only (for highlighting).
    pub selected: Option<(Vec<u32>, Arc<Mesh>)>,
}

#[derive(Default)]
pub(crate) struct PrepState {
    pub tool: PrepTool,
    pub mode: Mode,
    /// Selected supports: (object, support id).
    pub selection: Vec<(ObjectId, u32)>,
    pub job: Option<PrepJob>,
    /// Last island check: islands without supports.
    pub islands: Option<Vec<(ObjectId, Vec<Island>)>>,
    pub cache: HashMap<ObjectId, CachedSupports>,
    pub resize_diameter: f32,
    pub resize_tip: f32,
    /// What automatic orientation aims for.
    pub goal: Goal,
    /// "Prepare for printing" leaves the orientation as it is.
    pub keep_orientation: bool,
    /// "Prepare for printing" adds no supports.
    pub skip_supports: bool,
    /// The drain hole check of the selected object, remembered until something changes:
    /// (what it was made for, the line to show, all is well).
    pub(crate) hole_status: Option<(HoleKey, String, bool)>,
}

/// What a drain hole check depends on: object, hollow, number and depth of the holes.
type HoleKey = (ObjectId, usize, usize, u32);

/// The names of the support presets, with what they are for.
const SUPPORT_PRESETS: [(&str, &str); 3] = [
    (
        "Light",
        "Thin tips that come off easily: small, light parts (Ctrl+1)",
    ),
    ("Medium", "The everyday preset (Ctrl+2)"),
    (
        "Heavy",
        "Thick, dense supports for big, heavy parts (Ctrl+3)",
    ),
];

/// Which preset the support settings are, if they are one.
fn support_preset(s: &SupportSettings) -> Option<usize> {
    [
        SupportSettings::light(),
        SupportSettings::medium(),
        SupportSettings::heavy(),
    ]
    .iter()
    .position(|p| {
        p.tip_diameter_mm == s.tip_diameter_mm
            && p.diameter_mm == s.diameter_mm
            && p.density == s.density
    })
}

/// Mesh of a support set placed at height `z` (and X = Y = 0); the view adds X and Y.
pub(crate) fn support_mesh<'a>(
    state: &'a mut PrepState,
    id: ObjectId,
    set: &Arc<SupportSet>,
    z: f32,
) -> &'a CachedSupports {
    let key = (Arc::as_ptr(set) as usize, z.to_bits());
    let stale = state.cache.get(&id).is_none_or(|c| c.key != key);
    if stale {
        let built = copal_core::supports::build_mesh(set, Vec3::new(0.0, 0.0, z));
        state.cache.insert(
            id,
            CachedSupports {
                key,
                mesh: Arc::new(built.mesh),
                ranges: built.ranges,
                selected: None,
            },
        );
    }
    let sel: Vec<u32> = state
        .selection
        .iter()
        .filter(|(o, _)| *o == id)
        .map(|(_, s)| *s)
        .collect();
    let c = state.cache.get_mut(&id).expect("just inserted");
    let need = !sel.is_empty() && c.selected.as_ref().is_none_or(|(ids, _)| *ids != sel);
    if sel.is_empty() {
        c.selected = None;
    } else if need {
        let mut m = Mesh::default();
        for (sid, range) in &c.ranges {
            if sel.contains(sid) {
                for t in &c.mesh.triangles[range.clone()] {
                    let base = m.positions.len() as u32;
                    for i in t {
                        m.positions.push(c.mesh.positions[*i as usize]);
                    }
                    m.triangles.push([base, base + 1, base + 2]);
                }
            }
        }
        c.selected = Some((sel, Arc::new(m)));
    }
    state.cache.get(&id).expect("present")
}

/// Distance from a ray to a segment, and the ray parameter at the closest point.
fn ray_segment(ray: &Ray, a: Vec3, b: Vec3) -> (f32, f32) {
    let u = ray.dir;
    let v = b - a;
    let w = ray.origin - a;
    let (aa, bb, cc, dd, ee) = (u.dot(u), u.dot(v), v.dot(v), u.dot(w), v.dot(w));
    let den = aa * cc - bb * bb;
    let (mut sc, mut tc) = if den.abs() < 1e-9 {
        (0.0, if bb > cc { dd / bb } else { ee / cc })
    } else {
        ((bb * ee - cc * dd) / den, (aa * ee - bb * dd) / den)
    };
    tc = tc.clamp(0.0, 1.0);
    sc = sc.max(0.0);
    if den.abs() >= 1e-9 {
        // Recompute sc for the clamped tc.
        sc = ((a + v * tc - ray.origin).dot(u) / aa).max(0.0);
    }
    let p = ray.origin + u * sc;
    let q = a + v * tc;
    (p.distance(q), sc)
}

/// The support under a ray: (object, support id, distance along the ray).
pub(crate) fn pick_support(
    doc: &Document,
    ray: &Ray,
    tolerance: f32,
) -> Option<(ObjectId, u32, f32)> {
    let mut best: Option<(ObjectId, u32, f32)> = None;
    for o in doc.objects() {
        let Some(set) = &o.prep.supports else {
            continue;
        };
        let pos = o.transform.translation;
        for s in &set.supports {
            let mut pts = vec![s.tip + pos, s.joint + pos];
            if let Some(k) = s.knee {
                pts.push(k + pos);
            }
            let top = *pts.last().unwrap_or(&pos);
            let bottom = match s.foot {
                copal_core::supports::Foot::Plate => Vec3::new(top.x, top.y, set.base_top()),
                copal_core::supports::Foot::Model { point } => point + pos,
            };
            pts.push(bottom);
            for w in pts.windows(2) {
                let (d, t) = ray_segment(ray, w[0], w[1]);
                if d < s.diameter / 2.0 + tolerance && best.is_none_or(|(_, _, bt)| t < bt) {
                    best = Some((o.id, s.id, t));
                }
            }
        }
    }
    best
}

impl Workspace {
    pub(crate) fn job_running(&self) -> bool {
        self.prep.job.is_some()
    }

    fn start_job(&mut self, label: &str, f: impl FnOnce() -> JobResult + Send + 'static) {
        if self.prep.job.is_some() {
            self.log(LineKind::Error, "wait for the running task to finish");
            return;
        }
        self.log(LineKind::Info, format!("{label}…"));
        self.prep.job = Some(PrepJob {
            label: label.to_string(),
            started: std::time::Instant::now(),
            handle: Some(std::thread::spawn(f)),
        });
    }

    /// Objects to work on: the selected one, or all.
    fn prep_targets(&self, all: bool) -> Vec<ObjectId> {
        match (all, self.selected) {
            (false, Some(id)) => vec![id],
            _ => self.doc().objects().iter().map(|o| o.id).collect(),
        }
    }

    pub fn auto_supports(&mut self, all: bool) {
        let ids = self.prep_targets(all);
        if ids.is_empty() {
            return;
        }
        let doc = self.doc().clone();
        let mode = self.prep.mode;
        self.prep.islands = None;
        self.start_job("Adding supports", move || {
            JobResult::Supports(
                ids.into_iter()
                    .map(|id| {
                        let r = copal_core::prepare::auto_supports(&doc, id, mode)
                            .map_err(|e| e.to_string());
                        (id, mode, r)
                    })
                    .collect(),
            )
        });
    }

    /// Turns the selected model (or all) the best way up for printing.
    pub fn auto_orient(&mut self, all: bool) {
        let ids = self.prep_targets(all);
        if ids.is_empty() {
            return;
        }
        let doc = self.doc().clone();
        let goal = self.prep.goal;
        self.prep.islands = None;
        self.start_job("Looking for the best orientation", move || {
            JobResult::Orient(
                ids.into_iter()
                    .map(|id| {
                        let r = copal_core::prepare::orient_object(&doc, id, goal)
                            .map_err(|e| e.to_string());
                        (id, goal, r)
                    })
                    .collect(),
            )
        });
    }

    /// What "Prepare for printing" will do, from the switches under the button.
    pub fn prepare_options(&self) -> PrepareOptions {
        PrepareOptions {
            orient: (!self.prep.keep_orientation).then_some(self.prep.goal),
            hollow: self.doc().settings.hollow.enabled,
            holes: true,
            supports: (!self.prep.skip_supports).then_some(self.prep.mode),
        }
    }

    /// Orientation, hollow, drain holes and supports for the selected model (or all), in one go.
    pub fn prepare_for_printing(&mut self, all: bool) {
        let ids = self.prep_targets(all);
        if ids.is_empty() {
            return;
        }
        let doc = self.doc().clone();
        let opts = self.prepare_options();
        self.prep.islands = None;
        self.start_job("Preparing for printing", move || {
            JobResult::Prepare(
                ids.into_iter()
                    .map(|id| {
                        let r = copal_core::prepare::prepare_object(&doc, id, opts)
                            .map_err(|e| e.to_string());
                        (id, opts, r)
                    })
                    .collect(),
            )
        });
    }

    pub fn hollow_selected(&mut self, all: bool) {
        let ids = self.prep_targets(all);
        if ids.is_empty() {
            return;
        }
        let doc = self.doc().clone();
        let h = doc.settings.hollow;
        self.start_job("Hollowing", move || {
            JobResult::Hollow(
                ids.into_iter()
                    .map(|id| {
                        let r = copal_core::prepare::hollow_object_detailed(
                            &doc,
                            id,
                            h.wall_thickness_mm,
                            h.voxel_mm,
                        )
                        .map_err(|e| e.to_string());
                        (id, h.wall_thickness_mm, h.voxel_mm, r)
                    })
                    .collect(),
            )
        });
    }

    pub fn check_islands(&mut self) {
        let doc = self.doc().clone();
        let ids: Vec<ObjectId> = doc.objects().iter().map(|o| o.id).collect();
        self.start_job("Looking for islands", move || {
            JobResult::Islands(
                ids.into_iter()
                    .map(|id| {
                        (
                            id,
                            copal_core::prepare::unsupported_islands(&doc, id)
                                .map_err(|e| e.to_string()),
                        )
                    })
                    .collect(),
            )
        });
    }

    /// Applies a finished background task. Call every frame.
    pub fn poll_prep_job(&mut self, ctx: &egui::Context) {
        let Some(job) = &mut self.prep.job else {
            return;
        };
        if !job.handle.as_ref().is_some_and(|h| h.is_finished()) {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
            return;
        }
        let secs = job.started.elapsed().as_secs_f32();
        let label = job.label.clone();
        let result = job.handle.take().map(|h| h.join());
        self.prep.job = None;
        let Some(Ok(result)) = result else {
            self.log(LineKind::Error, format!("{label} stopped unexpectedly"));
            return;
        };
        let mut cmds: Vec<Box<dyn Command>> = Vec::new();
        let mut texts = Vec::new();
        // Supports are checked for islands right after they are made.
        let mut check_after = false;
        match result {
            JobResult::Orient(list) => {
                for (id, goal, r) in list {
                    match r {
                        Ok(found) => {
                            let (b, a) = (found.before, found.after);
                            self.log(
                                LineKind::Info,
                                format!(
                                    "[{id}] biggest layer {:.0} → {:.0} mm², overhangs {:.0} → {:.0} mm², height {:.0} → {:.0} mm",
                                    b.peak_area,
                                    a.peak_area,
                                    b.overhang_area,
                                    a.overhang_area,
                                    b.height,
                                    a.height
                                ),
                            );
                            let c = Orient {
                                id,
                                goal,
                                precomputed: Some(found.transform),
                            };
                            texts.push(c.to_text().unwrap_or_default());
                            cmds.push(Box::new(c));
                        }
                        Err(e) => self.log(LineKind::Error, format!("[{id}] {e}")),
                    }
                }
            }
            JobResult::Prepare(list) => {
                for (id, opts, r) in list {
                    match r {
                        Ok(done) => {
                            for n in &done.notes {
                                self.log(LineKind::Info, format!("[{id}] {n}"));
                            }
                            for w in &done.warnings {
                                self.log(LineKind::Error, format!("[{id}] check: {w}"));
                            }
                            let c = AutoPrepare {
                                id,
                                opts,
                                precomputed: Some(Arc::new(done)),
                            };
                            texts.push(c.to_text().unwrap_or_default());
                            cmds.push(Box::new(c));
                        }
                        Err(e) => self.log(LineKind::Error, format!("[{id}] {e}")),
                    }
                }
            }
            JobResult::Supports(list) => {
                check_after = true;
                for (id, mode, r) in list {
                    match r {
                        Ok((set, pos)) => {
                            self.log(
                                LineKind::Info,
                                format!(
                                    "[{id}] {} supports, {} braces",
                                    set.supports.len(),
                                    set.braces.len()
                                ),
                            );
                            let c = AutoSupports {
                                id,
                                mode,
                                precomputed: Some((Arc::new(set), pos)),
                            };
                            texts.push(c.to_text().unwrap_or_default());
                            cmds.push(Box::new(c));
                        }
                        Err(e) => self.log(LineKind::Error, format!("[{id}] {e}")),
                    }
                }
            }
            JobResult::Hollow(list) => {
                for (id, thickness, voxel, r) in list {
                    match r {
                        Ok(done) => {
                            let mut note =
                                format!("[{id}] hollow: {:.1} ml of resin saved", done.volume_ml);
                            if done.cavities > 1 {
                                note.push_str(&format!(", {} separate hollows", done.cavities));
                            }
                            if done.pockets_filled > 0 {
                                note.push_str(&format!(
                                    ", {} small pockets filled",
                                    done.pockets_filled
                                ));
                            }
                            if done.voxel > voxel + 1e-4 {
                                note.push_str(&format!(
                                    ", {:.2} mm voxels (big model)",
                                    done.voxel
                                ));
                            }
                            self.log(LineKind::Info, note);
                            let c = Hollow {
                                id,
                                thickness,
                                voxel,
                                precomputed: Some(Arc::new(done.cavity)),
                            };
                            texts.push(c.to_text().unwrap_or_default());
                            cmds.push(Box::new(c));
                        }
                        Err(e) => self.log(LineKind::Error, e),
                    }
                }
            }
            JobResult::Islands(list) => {
                let mut total = 0;
                let mut found = Vec::new();
                for (id, r) in list {
                    match r {
                        Ok(v) => {
                            total += v.len();
                            found.push((id, v));
                        }
                        Err(e) => self.log(LineKind::Error, e),
                    }
                }
                if total == 0 {
                    self.log(
                        LineKind::Info,
                        format!("Every island is supported ({secs:.1} s)"),
                    );
                } else {
                    self.log(
                        LineKind::Error,
                        format!("{total} islands without support — red dots in the 3D view"),
                    );
                }
                self.prep.islands = Some(found);
                return;
            }
        }
        if cmds.is_empty() {
            return;
        }
        let prompt = self.prompt_place();
        for t in &texts {
            self.log(
                LineKind::Input {
                    prompt: prompt.clone(),
                },
                t.clone(),
            );
        }
        let cmd: Box<dyn Command> = if cmds.len() == 1 {
            cmds.pop().expect("one")
        } else {
            Box::new(Batch::new(texts.join(" ; "), cmds))
        };
        if let Err(e) = self.shell.session.execute(cmd) {
            self.log(LineKind::Error, e.to_string());
        } else {
            self.log(LineKind::Info, format!("{label} took {secs:.1} s"));
        }
        self.prep.selection.retain(|(o, s)| {
            self.shell
                .session
                .doc
                .object(*o)
                .and_then(|x| x.prep.supports.as_ref())
                .is_some_and(|set| set.get(*s).is_some())
        });
        if check_after {
            self.check_islands();
        }
    }

    /// Removes the supports of the selected model, or of all models when none is selected.
    pub fn remove_supports(&mut self) {
        for id in self.prep_targets(self.selected.is_none()) {
            if self
                .doc()
                .object(id)
                .is_some_and(|o| o.prep.supports.is_some())
            {
                self.run(&format!("supports.clear id={id}"));
            }
        }
        self.prep.selection.clear();
    }

    /// Drain holes at the lowest points of the selected hollow model (or of every hollow one).
    pub fn auto_holes(&mut self) {
        let ids: Vec<ObjectId> = self
            .prep_targets(self.selected.is_none())
            .into_iter()
            .filter(|id| {
                self.doc()
                    .object(*id)
                    .is_some_and(|o| o.prep.cavity.is_some())
            })
            .collect();
        if ids.is_empty() {
            self.log(
                LineKind::Error,
                "drain holes need a hollow model: hollow it first",
            );
            return;
        }
        for id in ids {
            self.run(&format!("holes.auto id={id}"));
        }
    }

    pub fn delete_selected_supports(&mut self) {
        let mut by_object: HashMap<ObjectId, Vec<u32>> = HashMap::new();
        for (o, s) in self.prep.selection.drain(..) {
            by_object.entry(o).or_default().push(s);
        }
        for (o, ids) in by_object {
            let list: Vec<String> = ids.iter().map(|i| i.to_string()).collect();
            self.run(&format!("supports.remove id={o} sid={}", list.join(",")));
        }
    }

    /// The drain hole check of an object as one line, and whether all is well. Remembered
    /// until the hollow or the holes change.
    fn hole_status(&mut self, id: ObjectId) -> Option<(String, bool)> {
        let o = self.doc().object(id)?;
        let holes = o.prep.holes.len();
        let depth: f32 = o.prep.holes.iter().map(|h| h.depth + h.diameter).sum();
        let key: HoleKey = (
            id,
            o.prep
                .cavity
                .as_ref()
                .map_or(0, |c| Arc::as_ptr(c) as usize),
            holes,
            depth.to_bits(),
        );
        if let Some((k, text, ok)) = &self.prep.hole_status
            && *k == key
        {
            return Some((text.clone(), *ok));
        }
        let check = copal_core::prepare::hole_check(self.doc(), id).ok()?;
        let (text, ok) = (check.summary(holes), check.ok());
        self.prep.hole_status = Some((key, text.clone(), ok));
        Some((text, ok))
    }

    pub(crate) fn prepare_tools(&mut self, ui: &mut Ui, look: &Look, w: Option<f32>) {
        let faint = |ui: &mut Ui, t: &str| {
            ui.label(RichText::new(t).small().color(look.pal.text_faint));
        };
        let busy = self.job_running();
        let any = !self.doc().objects().is_empty();
        let target = if self.selected.is_some() {
            "selected"
        } else {
            "all"
        };
        let scope = self.prep_targets(self.selected.is_none());
        let is_hollow = scope.iter().any(|id| {
            self.doc()
                .object(*id)
                .is_some_and(|o| o.prep.cavity.is_some())
        });
        let has_holes = scope.iter().any(|id| {
            self.doc()
                .object(*id)
                .is_some_and(|o| !o.prep.holes.is_empty())
        });
        let supported = scope.iter().any(|id| {
            self.doc()
                .object(*id)
                .is_some_and(|o| o.prep.supports.is_some())
        });

        // ---- Everything in one click. The steps below do the same one by one.
        widgets::section(ui, look, "Ready in one click");
        ui.add_enabled_ui(!busy && any, |ui| {
            if widgets::button_sized(
                ui,
                look,
                ButtonKind::Primary,
                format!("Prepare for printing ({target})"),
                w,
            )
            .on_hover_text(
                "Ctrl+P. Turns the model the best way up, hollows it if you want, adds drain \
                 holes and supports, and checks the result. One undo step.",
            )
            .clicked()
            {
                self.prepare_for_printing(false);
            }
        });
        ui.horizontal_wrapped(|ui| {
            let mut turn = !self.prep.keep_orientation;
            if ui
                .checkbox(&mut turn, "Turn")
                .on_hover_text("Find the best orientation first (section 1)")
                .changed()
            {
                self.prep.keep_orientation = !turn;
            }
            let mut hollow = self.doc().settings.hollow.enabled;
            if ui
                .checkbox(&mut hollow, "Hollow")
                .on_hover_text(
                    "Hollow the model and add drain holes (sections 2 and 3). Saves resin and \
                     makes big parts pull less on the film.",
                )
                .changed()
            {
                self.run(&format!("hollow.set enabled={hollow}"));
            }
            let mut supports = !self.prep.skip_supports;
            if ui
                .checkbox(&mut supports, "Supports")
                .on_hover_text("Add supports with the preset below (section 4)")
                .changed()
            {
                self.prep.skip_supports = !supports;
            }
        });
        if let Some(job) = &self.prep.job {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(format!(
                    "{} {:.0} s",
                    job.label,
                    job.started.elapsed().as_secs_f32()
                ));
            });
        }

        // ---- 1. Orientation.
        widgets::section(ui, look, "1 · Orientation");
        const GOALS: [(&str, &str); 4] = [
            (
                "Balanced",
                "Small layers that grow slowly, few overhangs, not too tall. The safe choice.",
            ),
            (
                "Strong",
                "The smallest layers: the least pull on the film and the plate. For big, heavy parts.",
            ),
            (
                "Clean",
                "The least area that hangs over: fewer supports and fewer marks.",
            ),
            ("Low", "The lowest print: the shortest print time."),
        ];
        let current = Goal::ALL.iter().position(|g| *g == self.prep.goal);
        if let Some(i) = widgets::choice_row(ui, look, &GOALS, current) {
            self.prep.goal = Goal::ALL[i];
        }
        ui.add_enabled_ui(!busy && any, |ui| {
            if widgets::button_sized(
                ui,
                look,
                ButtonKind::Normal,
                format!("Auto orient ({target})"),
                w,
            )
            .on_hover_text(
                "Ctrl+T. Turns the model so it prints safely. Supports are removed, because \
                 they were made for the old orientation.",
            )
            .clicked()
            {
                self.auto_orient(false);
            }
        });

        // ---- 2. Hollow: wall, then hollow / make solid.
        widgets::section(ui, look, "2 · Hollow");
        let h = self.doc().settings.hollow;
        ui.horizontal(|ui| {
            ui.label(RichText::new("Wall").color(look.pal.text_weak));
            let mut t = h.wall_thickness_mm;
            let r = ui.add(
                DragValue::new(&mut t)
                    .speed(0.05)
                    .range(0.5..=20.0)
                    .suffix(" mm"),
            );
            if r.drag_stopped() || (r.changed() && !r.dragged()) {
                self.run(&format!("hollow.set wall_thickness_mm={t}"));
            }
        });
        match widgets::button_row(
            ui,
            look,
            &[
                (
                    "Hollow",
                    "Ctrl+H. Keeps walls of this thickness and removes the inside. Small closed \
                     pockets are filled again. Add drain holes so resin can flow out.",
                    !busy && any,
                ),
                (
                    "Make solid",
                    "Ctrl+Shift+H. Fills the model again",
                    !busy && is_hollow,
                ),
            ],
        ) {
            Some(0) => self.hollow_selected(false),
            Some(_) => {
                for id in scope.clone() {
                    if self
                        .doc()
                        .object(id)
                        .is_some_and(|o| o.prep.cavity.is_some())
                    {
                        self.run(&format!("object.unhollow id={id}"));
                    }
                }
            }
            None => {}
        }

        // ---- 3. Drain holes: automatic, by hand, remove.
        widgets::section(ui, look, "3 · Drain holes");
        match widgets::button_row(
            ui,
            look,
            &[
                (
                    "Auto holes",
                    "Ctrl+J. Two holes at the lowest points of every hollow: one lets air in, \
                     the other lets resin out.",
                    is_hollow,
                ),
                (
                    "Remove",
                    "Removes the drain holes of the selected model, or of all",
                    has_holes,
                ),
            ],
        ) {
            Some(0) => self.auto_holes(),
            Some(_) => {
                for id in scope.clone() {
                    if self
                        .doc()
                        .object(id)
                        .is_some_and(|o| !o.prep.holes.is_empty())
                    {
                        self.run(&format!("holes.clear id={id}"));
                    }
                }
            }
            None => {}
        }
        ui.horizontal_wrapped(|ui| {
            if tab_button(
                ui,
                look,
                self.prep.tool == PrepTool::AddHole,
                "Add by click",
                false,
            )
            .on_hover_text(
                "H. Click on the model to add a hole there, as deep as the wall. Put holes \
                 near the plate, at the lowest points of the hollow.",
            )
            .clicked()
            {
                self.prep.tool = if self.prep.tool == PrepTool::AddHole {
                    PrepTool::Look
                } else {
                    PrepTool::AddHole
                };
            }
            ui.label(RichText::new("Ø").color(look.pal.text_weak));
            let mut d = h.drain_hole_diameter_mm;
            let r = ui.add(
                DragValue::new(&mut d)
                    .speed(0.05)
                    .range(0.5..=20.0)
                    .suffix(" mm"),
            );
            if r.drag_stopped() || (r.changed() && !r.dragged()) {
                self.run(&format!("hollow.set drain_hole_diameter_mm={d}"));
            }
        });
        if let Some(id) = self.selected.or_else(|| scope.first().copied())
            && let Some((text, ok)) = self.hole_status(id)
        {
            let name = self
                .doc()
                .object(id)
                .map(|o| o.name.clone())
                .unwrap_or_default();
            let color = if ok {
                look.pal.text_faint
            } else {
                look.pal.error
            };
            ui.label(
                RichText::new(format!("{name}: {text}"))
                    .small()
                    .color(color),
            );
        }

        // ---- 4. Supports: preset and mode, the big button, then the tools for single supports.
        widgets::section(ui, look, "4 · Supports");
        let preset = support_preset(&self.doc().settings.supports);
        if let Some(i) = widgets::choice_row(ui, look, &SUPPORT_PRESETS, preset) {
            let name = ["light", "medium", "heavy"][i];
            self.run(&format!("presets load supports {name}"));
        }
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Where").color(look.pal.text_weak));
            if tab_button(ui, look, self.prep.mode == Mode::Auto, "Everywhere", false)
                .on_hover_text(
                    "Islands, lowest points, edges and overhangs: a full set of supports",
                )
                .clicked()
            {
                self.prep.mode = Mode::Auto;
            }
            if tab_button(
                ui,
                look,
                self.prep.mode == Mode::Islands,
                "Islands only",
                false,
            )
            .on_hover_text("Only the points that would print in mid-air: the least you need")
            .clicked()
            {
                self.prep.mode = Mode::Islands;
            }
        });
        ui.add_enabled_ui(!busy && any, |ui| {
            if widgets::button_sized(
                ui,
                look,
                ButtonKind::Normal,
                format!("Auto supports ({target})"),
                w,
            )
            .on_hover_text(
                "Ctrl+U. Lifts the model, finds islands, edges and overhangs, ties the supports \
                 together with braces and adds a raft. Supports you placed by hand stay.",
            )
            .clicked()
            {
                self.auto_supports(false);
            }
            if self.selected.is_some()
                && self.doc().objects().len() > 1
                && widgets::button_sized(
                    ui,
                    look,
                    ButtonKind::Normal,
                    "Auto supports for all models",
                    w,
                )
                .clicked()
            {
                self.auto_supports(true);
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Click").color(look.pal.text_weak));
            for (tool, name, hint) in [
                (
                    PrepTool::Look,
                    "View",
                    "V. Click models to select them, drag to turn the view",
                ),
                (
                    PrepTool::Select,
                    "Select",
                    "B. Click supports to select them, Shift+click adds, Delete removes",
                ),
                (
                    PrepTool::AddSupport,
                    "Add",
                    "A. Click on the model to add a support there. It joins the supports \
                     around it: a branch of the nearest one, or its own body tied with braces.",
                ),
            ] {
                if tab_button(ui, look, self.prep.tool == tool, name, false)
                    .on_hover_text(hint)
                    .clicked()
                {
                    self.prep.tool = tool;
                }
            }
        });
        if self.prep.tool == PrepTool::AddSupport {
            let mut s = self.doc().settings.supports;
            if ui
                .checkbox(&mut s.auto_parent, "Join the nearest support")
                .on_hover_text("A new support next to another leans over and becomes its branch")
                .changed()
            {
                self.run(&format!("supports.set auto_parent={}", s.auto_parent));
            }
            if ui
                .checkbox(&mut s.manual_bracing, "Tie to neighbours with braces")
                .on_hover_text("A new support with its own body gets braces to the ones around it")
                .changed()
            {
                self.run(&format!("supports.set manual_bracing={}", s.manual_bracing));
            }
        }
        let n = self.prep.selection.len();
        if n > 0 {
            ui.label(format!("{n} supports selected"));
            if self.prep.resize_diameter <= 0.0 {
                self.prep.resize_diameter = self.doc().settings.supports.diameter_mm;
                self.prep.resize_tip = self.doc().settings.supports.tip_diameter_mm;
            }
            ui.horizontal(|ui| {
                ui.label("Body");
                ui.add(
                    DragValue::new(&mut self.prep.resize_diameter)
                        .speed(0.02)
                        .range(0.3..=5.0)
                        .suffix(" mm"),
                );
                ui.label("Tip");
                ui.add(
                    DragValue::new(&mut self.prep.resize_tip)
                        .speed(0.01)
                        .range(0.1..=2.0)
                        .suffix(" mm"),
                );
            });
            match widgets::button_row(
                ui,
                look,
                &[
                    ("Apply size", "Give the selected supports this size", true),
                    ("Delete", "Delete the selected supports (Delete)", true),
                    ("Deselect", "Esc", true),
                ],
            ) {
                Some(0) => {
                    let mut by_object: HashMap<ObjectId, Vec<String>> = HashMap::new();
                    for (o, s) in &self.prep.selection {
                        by_object.entry(*o).or_default().push(s.to_string());
                    }
                    let (d, t) = (self.prep.resize_diameter, self.prep.resize_tip);
                    for (o, ids) in by_object {
                        self.run(&format!(
                            "supports.resize id={o} sid={} diameter={d} tip={t}",
                            ids.join(",")
                        ));
                    }
                }
                Some(1) => self.delete_selected_supports(),
                Some(_) => self.prep.selection.clear(),
                None => {}
            }
        }
        match widgets::button_row(
            ui,
            look,
            &[
                (
                    "Check islands",
                    "I. Finds parts that would print in mid-air without a support",
                    !busy && any,
                ),
                (
                    "Overhangs",
                    "How much hangs over, as numbers in the console",
                    any,
                ),
                (
                    "Remove",
                    "Ctrl+Shift+U. Removes the supports of the selected model, or of all",
                    !busy && supported,
                ),
            ],
        ) {
            Some(0) => self.check_islands(),
            Some(1) => {
                let who = self
                    .selected
                    .map(|id| id.to_string())
                    .unwrap_or_else(|| "all".into());
                self.run(&format!("overhangs {who}"));
            }
            Some(_) => self.remove_supports(),
            None => {}
        }
        if let Some(found) = &self.prep.islands {
            let total: usize = found.iter().map(|(_, v)| v.len()).sum();
            let (text, color) = if total == 0 {
                ("All islands are supported".to_string(), look.pal.text_weak)
            } else {
                (
                    format!("{total} islands without support (red dots): add supports there"),
                    look.pal.error,
                )
            };
            ui.label(RichText::new(text).color(color));
        }
        let counts: Vec<String> = self
            .doc()
            .objects()
            .iter()
            .filter_map(|o| {
                o.prep.supports.as_ref().map(|s| {
                    format!(
                        "{}: {} supports, {} braces",
                        o.name,
                        s.supports.len(),
                        s.braces.len()
                    )
                })
            })
            .collect();
        if !counts.is_empty() {
            faint(ui, &counts.join(" · "));
        }

        widgets::section(ui, look, "Objects");
        self.scene_list(ui, look);
    }
}
