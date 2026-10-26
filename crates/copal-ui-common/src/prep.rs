//! The Prepare step: supports (auto, islands only, by hand, select and delete), island check,
//! hollowing and drain holes. Heavy work runs in the background.

use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;
use std::thread::JoinHandle;

use copal_core::prepare::{AutoSupports, Hollow};
use copal_core::supports::{Island, Mode, SupportSet};
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
    Supports(Vec<SupportJob>),
    Hollow(Vec<(ObjectId, f32, f32, Result<Mesh, String>)>),
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
                        let r = copal_core::prepare::hollow_object(
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
        match result {
            JobResult::Supports(list) => {
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
                        Ok(mesh) => {
                            let c = Hollow {
                                id,
                                thickness,
                                voxel,
                                precomputed: Some(Arc::new(mesh)),
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

    pub(crate) fn prepare_tools(&mut self, ui: &mut Ui, look: &Look, w: Option<f32>) {
        let faint = |ui: &mut Ui, t: &str| {
            ui.label(RichText::new(t).small().color(look.pal.text_faint));
        };
        let busy = self.job_running();
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

        widgets::section(ui, look, "Supports");
        ui.horizontal_wrapped(|ui| {
            for name in ["light", "medium", "heavy"] {
                let label = format!("{}{}", name[..1].to_uppercase(), &name[1..]);
                if widgets::button(ui, look, ButtonKind::Normal, &label)
                    .on_hover_text(format!(
                        "Load the {name} preset: tip, thickness and density for new supports"
                    ))
                    .clicked()
                {
                    self.run(&format!("presets load supports {name}"));
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Mode").color(look.pal.text_weak));
            if tab_button(ui, look, self.prep.mode == Mode::Auto, "Auto", false)
                .on_hover_text("Islands and overhangs: a full set of supports")
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
            .on_hover_text("Only the points that would print in mid-air")
            .clicked()
            {
                self.prep.mode = Mode::Islands;
            }
        });
        ui.add_enabled_ui(!busy && !self.doc().objects().is_empty(), |ui| {
            let label = if self.selected.is_some() { "Auto supports (selected)" } else { "Auto supports (all)" };
            if widgets::button_sized(ui, look, ButtonKind::Primary, label, w)
                .on_hover_text("Lifts the model, finds islands and overhangs, adds braces and a raft. Supports you placed by hand stay.")
                .clicked()
            {
                self.auto_supports(false);
            }
            if self.selected.is_some()
                && self.doc().objects().len() > 1
                && widgets::button_sized(ui, look, ButtonKind::Normal, "Auto supports for all models", w).clicked()
            {
                self.auto_supports(true);
            }
        });
        ui.horizontal_wrapped(|ui| {
            for (tool, name, hint) in [
                (
                    PrepTool::Look,
                    "View",
                    "Click models to select them, drag to turn the view",
                ),
                (
                    PrepTool::Select,
                    "Select",
                    "Click supports to select them, Shift+click adds, Delete removes",
                ),
                (
                    PrepTool::AddSupport,
                    "Add",
                    "Click on the model to add a support there",
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
                .checkbox(&mut s.manual_bracing, "Connect new supports with braces")
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
            ui.horizontal_wrapped(|ui| {
                if widgets::button(ui, look, ButtonKind::Normal, "Apply size").clicked() {
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
                if widgets::button(ui, look, ButtonKind::Normal, "Delete selected").clicked() {
                    self.delete_selected_supports();
                }
                if widgets::button(ui, look, ButtonKind::Normal, "Deselect").clicked() {
                    self.prep.selection.clear();
                }
            });
        }
        ui.add_enabled_ui(!busy, |ui| {
            ui.horizontal_wrapped(|ui| {
                if widgets::button(ui, look, ButtonKind::Normal, "Check islands")
                    .on_hover_text("Finds parts that would print in mid-air without a support")
                    .clicked()
                {
                    self.check_islands();
                }
                if widgets::button(ui, look, ButtonKind::Normal, "Remove supports").clicked() {
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
            });
        });
        if let Some(found) = &self.prep.islands {
            let total: usize = found.iter().map(|(_, v)| v.len()).sum();
            let (text, color) = if total == 0 {
                ("All islands are supported".to_string(), look.pal.text_weak)
            } else {
                (
                    format!("{total} islands without support (red dots)"),
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
                o.prep
                    .supports
                    .as_ref()
                    .map(|s| format!("{}: {}", o.name, s.supports.len()))
            })
            .collect();
        if !counts.is_empty() {
            faint(ui, &format!("Supports — {}", counts.join(", ")));
        }

        widgets::section(ui, look, "Hollow");
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
        ui.add_enabled_ui(!busy && !self.doc().objects().is_empty(), |ui| {
            ui.horizontal_wrapped(|ui| {
                let label = if self.selected.is_some() { "Hollow (selected)" } else { "Hollow (all)" };
                if widgets::button(ui, look, ButtonKind::Normal, label)
                    .on_hover_text("Keeps walls of this thickness and removes the inside. Add drain holes so resin can flow out.")
                    .clicked()
                {
                    self.hollow_selected(false);
                }
                if let Some(id) = self.selected
                    && self.doc().object(id).is_some_and(|o| o.prep.cavity.is_some())
                    && widgets::button(ui, look, ButtonKind::Normal, "Make solid").clicked()
                {
                    self.run(&format!("object.unhollow id={id}"));
                }
            });
        });

        widgets::section(ui, look, "Drain holes");
        ui.horizontal_wrapped(|ui| {
            if tab_button(ui, look, self.prep.tool == PrepTool::AddHole, "Add holes", false)
                .on_hover_text("Click on the model to add a hole there. Put them near the plate, at the lowest points of the hollow.")
                .clicked()
            {
                self.prep.tool = if self.prep.tool == PrepTool::AddHole { PrepTool::Look } else { PrepTool::AddHole };
            }
            if let Some(id) = self.selected
                && self.doc().object(id).is_some_and(|o| !o.prep.holes.is_empty())
                && widgets::button(ui, look, ButtonKind::Normal, "Remove holes").clicked()
            {
                self.run(&format!("holes.clear id={id}"));
            }
        });
        ui.horizontal(|ui| {
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
            ui.label(RichText::new("depth").color(look.pal.text_weak));
            let mut dp = h.drain_hole_depth_mm;
            let r = ui.add(
                DragValue::new(&mut dp)
                    .speed(0.05)
                    .range(0.5..=30.0)
                    .suffix(" mm"),
            );
            if r.drag_stopped() || (r.changed() && !r.dragged()) {
                self.run(&format!("hollow.set drain_hole_depth_mm={dp}"));
            }
        });
        if let Some(o) = self.selected.and_then(|id| self.doc().object(id)) {
            let hollow = if o.prep.cavity.is_some() {
                "hollow"
            } else {
                "solid"
            };
            faint(
                ui,
                &format!("{}: {hollow}, {} drain holes", o.name, o.prep.holes.len()),
            );
        }

        widgets::section(ui, look, "Objects");
        self.scene_list(ui, look);
    }
}
