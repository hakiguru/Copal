//! The 3D view: camera controls, picking, dragging models on the plate, gizmos, and the layer
//! preview.

use std::sync::Arc;

use copal_core::{ObjectId, Transform};
use copal_slice::{Raster, Slicer};
use egui::{
    Align2, Color32, ColorImage, FontId, Rect, RichText, Sense, Stroke, TextureHandle,
    TextureOptions, Ui, pos2, vec2,
};
use glam::{Mat4, Vec3};

use crate::camera::Ray;
use crate::gizmo::{self, Drag, Gizmo, Handle};
use crate::look::Look;
use crate::render::{ColorVertex, DrawObject, Frame3d, Renderer};
use crate::widgets;
use crate::workspace::{Step, Workspace};

/// What a mouse drag in the view is doing.
#[derive(Debug, Clone, Copy)]
pub(crate) enum ViewDrag {
    Gizmo {
        id: ObjectId,
        drag: Drag,
        keep_on_plate: bool,
    },
    /// Moving a model over the plate with the cursor; its height never changes.
    Plate {
        id: ObjectId,
        start: Transform,
        grab: Vec3,
    },
    Orbit,
    Pan,
}

/// Faces that need supports: a soft lilac that stands out from the model and the supports.
pub const OVERHANG_COLOR: [f32; 3] = [0.62, 0.56, 0.88];

/// The layer preview of the Export step.
#[derive(Default)]
pub(crate) struct LayerPreview {
    pub show: bool,
    pub layer: u32,
    /// The layer slider of the 3D view: show models and supports only up to this layer
    /// (counted from 1). `None` shows everything.
    pub cut: Option<u32>,
    slicer: Option<(u64, Arc<Slicer>)>,
    texture: Option<(u64, u32, TextureHandle)>,
}

fn rgb(c: Color32) -> [f32; 3] {
    [
        c.r() as f32 / 255.0,
        c.g() as f32 / 255.0,
        c.b() as f32 / 255.0,
    ]
}

fn rgba(c: Color32, a: f32) -> [f32; 4] {
    let [r, g, b] = rgb(c);
    [r, g, b, a]
}

/// Closest object hit by a ray.
fn pick(doc: &copal_core::Document, ray: &Ray) -> Option<(ObjectId, Vec3)> {
    let mut best: Option<(f32, ObjectId)> = None;
    for o in doc.objects() {
        let Some(b) = o.world_bounds() else { continue };
        if !ray_hits_box(ray, b.min, b.max) {
            continue;
        }
        let inv: Mat4 = o.transform.matrix().inverse();
        let origin = inv.transform_point3(ray.origin);
        let dir = inv.transform_vector3(ray.dir);
        if let Some((t, _)) = o.mesh.raycast(origin, dir)
            && best.is_none_or(|(bt, _)| t < bt)
        {
            best = Some((t, o.id));
        }
    }
    best.map(|(t, id)| (id, ray.origin + ray.dir * t))
}

/// Where a model grabbed at `grab` is dragged to: the point of the plane at the grab height
/// under the pointer. `None` when the pointer is above the horizon or the point is farther than
/// `reach` from the middle of the plate (a view almost level with the plate sends it far away).
fn plate_point(ray: &Ray, grab: Vec3, reach: f32) -> Option<Vec3> {
    let hit = ray.hit_plane_z(grab.z)?;
    (hit.truncate().length() <= reach).then_some(hit)
}

/// Is any part of the box outside the build volume (the plate and the height above it)?
fn outside_volume(min: Vec3, max: Vec3, volume: Vec3) -> bool {
    let eps = 1e-3;
    min.x < -volume.x / 2.0 - eps
        || max.x > volume.x / 2.0 + eps
        || min.y < -volume.y / 2.0 - eps
        || max.y > volume.y / 2.0 + eps
        || min.z < -eps
        || max.z > volume.z + eps
}

fn ray_hits_box(ray: &Ray, min: Vec3, max: Vec3) -> bool {
    let inv = ray.dir.recip();
    let t1 = (min - ray.origin) * inv;
    let t2 = (max - ray.origin) * inv;
    let tmin = t1.min(t2).max_element();
    let tmax = t1.max(t2).min_element();
    tmax >= tmin.max(0.0)
}

/// The camera positions of the buttons over the view (and of the keys 4 to 7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraView {
    Iso,
    Top,
    Front,
    Side,
}

impl CameraView {
    pub fn parse(s: &str) -> Option<CameraView> {
        match s {
            "iso" => Some(CameraView::Iso),
            "top" => Some(CameraView::Top),
            "front" => Some(CameraView::Front),
            "side" => Some(CameraView::Side),
            _ => None,
        }
    }
}

impl Workspace {
    /// Turns the camera to one of the fixed views and shows everything.
    pub fn set_camera_view(&mut self, view: CameraView) {
        let (yaw, pitch) = match view {
            CameraView::Iso => (-28.0, 30.0),
            CameraView::Top => (0.0, 89.9),
            CameraView::Front => (0.0, 4.0),
            CameraView::Side => (90.0, 4.0),
        };
        self.camera.set_view(yaw, pitch);
        self.framed = false;
    }

    fn gizmo_for(&self, rect: Rect) -> Option<(ObjectId, Gizmo)> {
        if self.step != Step::Layout {
            return None;
        }
        let id = self.selected?;
        let o = self.doc().object(id)?;
        let b = o.world_bounds()?;
        Some((
            id,
            Gizmo::new(
                &self.camera,
                rect,
                b.center(),
                o.transform.rotation_deg,
                self.settings.gizmo_size,
                self.gizmo_mode,
            ),
        ))
    }

    /// Moves a previewed transform so the model's lowest point is on the plate.
    fn on_plate(&self, id: ObjectId, mut t: Transform) -> Transform {
        if let Some(low) = self.lowest_point(id, t) {
            t.translation.z -= low;
        }
        t
    }

    /// Height of the lowest point of an object if it had the transform `t`.
    fn lowest_point(&self, id: ObjectId, t: Transform) -> Option<f32> {
        let mut probe = self.doc().object(id)?.clone();
        probe.transform = t;
        probe.world_bounds().map(|b| b.min.z)
    }

    /// Names of the models that are not fully inside the build volume.
    pub(crate) fn outside_plate(&self) -> Vec<String> {
        let volume = self.doc().build_volume();
        self.doc()
            .objects()
            .iter()
            .filter(|o| {
                o.world_bounds()
                    .is_some_and(|b| outside_volume(b.min, b.max, volume))
            })
            .map(|o| o.name.clone())
            .collect()
    }

    pub fn viewport(&mut self, ui: &mut Ui, look: &Look) {
        let rect = ui.available_rect_before_wrap();
        if self.step == Step::Export && self.preview.show {
            self.layer_view(ui, look, rect);
            return;
        }
        let response = ui.allocate_rect(rect, Sense::click_and_drag());
        let volume = self.doc().build_volume();
        if !self.framed {
            self.framed = true;
            let (min, max) = match self.doc().bounds() {
                Some(b) if !self.doc().objects().is_empty() => (
                    b.min.min(Vec3::new(-volume.x / 2.0, -volume.y / 2.0, 0.0)),
                    b.max.max(Vec3::new(volume.x / 2.0, volume.y / 2.0, 20.0)),
                ),
                _ => (
                    Vec3::new(-volume.x / 2.0, -volume.y / 2.0, 0.0),
                    Vec3::new(volume.x / 2.0, volume.y / 2.0, volume.z * 0.3),
                ),
            };
            self.camera.frame(min, max);
        }

        let pointer = response.hover_pos().or(response.interact_pointer_pos());
        // Where the button went down. A drag is only noticed a few pixels later, and by then
        // the pointer may have left a thin handle: handles and models are picked from here.
        if !response.is_pointer_button_down_on() {
            self.press_pos = None;
        } else if self.press_pos.is_none() {
            self.press_pos = response.interact_pointer_pos();
        }
        let gizmo = self.gizmo_for(rect);
        let snap_on = ui.input(|i| i.modifiers.command);
        let layout = self.doc().settings.layout;
        let snap = snap_on.then_some((
            layout.move_snap_mm,
            layout.rotate_snap_deg,
            layout.scale_snap_percent,
        ));

        // Hover highlight of gizmo handles.
        if self.drag.is_none() {
            self.hot_handle = match (gizmo, pointer) {
                (Some((_, g)), Some(p)) => g.hit(&self.camera, rect, self.gizmo_mode, p),
                _ => None,
            };
        }

        // Start a drag.
        if response.drag_started()
            && let Some(now) = response.interact_pointer_pos()
        {
            let p = self.press_pos.unwrap_or(now);
            let (primary, shift) = ui.input(|i| (i.pointer.primary_down(), i.modifiers.shift));
            self.drag = if !primary || shift {
                Some(ViewDrag::Pan)
            } else if let Some((id, g)) = gizmo
                && let Some(h) = g.hit(&self.camera, rect, self.gizmo_mode, p)
                && let Some(o) = self.doc().object(id)
                && let Some(drag) = Drag::start(h, g, o.transform, &self.camera, rect, p)
            {
                let on_plate = o.world_bounds().is_some_and(|b| b.min.z.abs() < 0.01);
                // Rotating or scaling removes the supports, and with them the reason to hang
                // in the air: such a model goes back to the plate too.
                let loses_supports = o.prep.has_supports()
                    && matches!(h, Handle::Ring(_) | Handle::ScaleAxis(_) | Handle::Uniform);
                Some(ViewDrag::Gizmo {
                    id,
                    drag,
                    keep_on_plate: layout.keep_on_plate
                        && (on_plate || loses_supports)
                        && !matches!(h, Handle::Axis(2)),
                })
            } else if self.step == Step::Layout
                && let Some((id, hit)) = pick(self.doc(), &self.camera.ray(rect, p))
            {
                self.selected = Some(id);
                let start = self
                    .doc()
                    .object(id)
                    .map(|o| o.transform)
                    .unwrap_or_default();
                Some(ViewDrag::Plate {
                    id,
                    start,
                    grab: hit,
                })
            } else {
                Some(ViewDrag::Orbit)
            };
        }

        // Continue it.
        if response.dragged()
            && let Some(drag) = self.drag
        {
            let delta = response.drag_delta();
            let pos = response.interact_pointer_pos();
            match drag {
                ViewDrag::Orbit => self.camera.orbit(delta.x, delta.y),
                ViewDrag::Pan => self.camera.pan(rect, delta.x, delta.y),
                ViewDrag::Gizmo {
                    id,
                    drag,
                    keep_on_plate,
                } => {
                    if let Some(p) = pos
                        && let Some(mut t) = drag.update(&self.camera, rect, p, snap)
                    {
                        if keep_on_plate {
                            t = self.on_plate(id, t);
                        } else if layout.keep_on_plate
                            && matches!(drag.handle, Handle::Axis(2))
                            && let Some(low) = self.lowest_point(id, t)
                            && low < 0.0
                        {
                            // The up arrow lifts a model, but never pushes it into the plate.
                            t.translation.z -= low;
                        }
                        self.preview_transform(id, t);
                    }
                }
                ViewDrag::Plate { id, start, grab } => {
                    // Far enough to park a model next to the plate, not far enough to lose it.
                    let reach = volume.x.max(volume.y) * 1.5;
                    if let Some(p) = pos
                        && let Some(hit) = plate_point(&self.camera.ray(rect, p), grab, reach)
                    {
                        let mut d = hit - grab;
                        if let Some((mm, _, _)) = snap
                            && mm > 0.0
                        {
                            d = (d / mm).round() * mm;
                        }
                        let mut t = start;
                        t.translation.x += d.x;
                        t.translation.y += d.y;
                        self.preview_transform(id, t);
                    }
                }
            }
        }

        // Finish it: one undo step.
        if response.drag_stopped() {
            match self.drag.take() {
                Some(ViewDrag::Gizmo { id, drag, .. }) => self.commit_transform(id, drag.start),
                Some(ViewDrag::Plate { id, start, .. }) => self.commit_transform(id, start),
                _ => {}
            }
        }

        // Click: select, or use the Prepare tool. A click on a handle of the gizmo is not a
        // click on what lies behind it (it used to deselect the model).
        if response.clicked()
            && let Some(p) = response.interact_pointer_pos()
        {
            let on_handle =
                gizmo.is_some_and(|(_, g)| g.hit(&self.camera, rect, self.gizmo_mode, p).is_some());
            if !on_handle {
                let ray = self.camera.ray(rect, p);
                let shift = ui.input(|i| i.modifiers.shift);
                self.click(&ray, rect, p, shift);
            }
        }
        if response.double_clicked() {
            self.framed = false;
        }

        // Zoom.
        if response.hovered() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0.0
                && let Some(p) = pointer
            {
                self.camera.zoom(rect, p, scroll);
            }
        }

        // Draw.
        let frame = self.build_frame(look, rect, volume);
        // What the drag is doing, in numbers.
        if let Some(text) = self.drag_readout() {
            ui.painter().text(
                rect.center_bottom() - vec2(0.0, 22.0),
                Align2::CENTER_CENTER,
                text,
                FontId::monospace(13.0),
                look.pal.accent_text,
            );
        }
        // A model that will not print where it stands.
        let outside = self.outside_plate();
        if !outside.is_empty() {
            let text = if outside.len() == 1 {
                format!("{} is outside the build volume", outside[0])
            } else {
                format!("{} models are outside the build volume", outside.len())
            };
            ui.painter().text(
                rect.center_bottom() - vec2(0.0, 42.0),
                Align2::CENTER_CENTER,
                text,
                FontId::proportional(13.0),
                look.pal.error,
            );
        }
        // Help line for the active Prepare tool.
        if self.step == Step::Prepare {
            let hint = match self.prep.tool {
                crate::prep::PrepTool::Look => None,
                crate::prep::PrepTool::Select => {
                    Some("Click supports to select · Shift+click adds · Delete removes · Esc stops")
                }
                crate::prep::PrepTool::AddSupport => {
                    Some("Click on the model to add a support · Esc stops")
                }
                crate::prep::PrepTool::AddHole => {
                    Some("Click on the model to add a drain hole · Esc stops")
                }
            };
            if let Some(h) = hint {
                ui.painter().text(
                    rect.center_bottom() - vec2(0.0, 22.0),
                    Align2::CENTER_CENTER,
                    h,
                    FontId::proportional(13.0),
                    look.pal.accent_text,
                );
            }
        }
        let renderer = self.renderer.clone();
        let callback = egui_glow::CallbackFn::new(move |_info, painter| {
            let Ok(mut slot) = renderer.lock() else {
                return;
            };
            if slot.is_none() {
                *slot = Some(Renderer::new(painter.gl()));
            }
            if let Some(Ok(r)) = slot.as_mut() {
                r.paint(painter.gl(), &frame);
            }
        });
        ui.painter().add(egui::PaintCallback {
            rect,
            callback: Arc::new(callback),
        });
        if let Ok(slot) = self.renderer.lock()
            && let Some(Err(e)) = slot.as_ref()
        {
            ui.painter().text(
                rect.center(),
                Align2::CENTER_CENTER,
                format!("3D view not available: {e}"),
                FontId::proportional(14.0),
                look.pal.error,
            );
        }

        let painter = ui.painter_at(rect);
        if let Some((_, g)) = self.gizmo_for(rect) {
            let hot = match self.drag {
                Some(ViewDrag::Gizmo { drag, .. }) => Some(drag.handle),
                _ => self.hot_handle,
            };
            g.draw(
                &painter,
                &self.camera,
                rect,
                self.gizmo_mode,
                hot,
                look.pal.accent,
            );
        }
        // Islands without supports from the last check.
        if let Some(found) = &self.prep.islands {
            for (_, list) in found {
                for i in list {
                    let c = i
                        .contacts
                        .first()
                        .map(|c| c.0)
                        .unwrap_or(((i.min + i.max) / 2.0).extend(i.z));
                    if let Some(sp) = self.camera.project(rect, c) {
                        painter.circle(
                            sp,
                            5.0,
                            look.pal.error.gamma_multiply(0.6),
                            Stroke::new(1.5, look.pal.error),
                        );
                    }
                }
            }
        }
        self.view_overlay(ui, look, rect);
        self.layer_slider(ui, look, rect);
    }

    /// Position, angles or size of the model being dragged, for the line under the view.
    fn drag_readout(&self) -> Option<String> {
        let (id, handle) = match self.drag? {
            ViewDrag::Gizmo { id, drag, .. } => (id, Some(drag.handle)),
            ViewDrag::Plate { id, .. } => (id, None),
            _ => return None,
        };
        let t = self.doc().object(id)?.transform;
        Some(match handle {
            Some(Handle::Ring(_)) => format!(
                "rotation  X {:.1}°   Y {:.1}°   Z {:.1}°",
                t.rotation_deg.x, t.rotation_deg.y, t.rotation_deg.z
            ),
            Some(Handle::ScaleAxis(_) | Handle::Uniform) => format!(
                "scale  X {:.0} %   Y {:.0} %   Z {:.0} %",
                t.scale.x.abs() * 100.0,
                t.scale.y.abs() * 100.0,
                t.scale.z.abs() * 100.0
            ),
            _ => format!(
                "position  X {:.2}   Y {:.2}   Z {:.2} mm",
                t.translation.x, t.translation.y, t.translation.z
            ),
        })
    }

    /// A click in the 3D view (no drag).
    fn click(&mut self, ray: &Ray, rect: Rect, _pos: egui::Pos2, shift: bool) {
        use crate::prep::PrepTool;
        let tool = if self.step == Step::Prepare {
            self.prep.tool
        } else {
            PrepTool::Look
        };
        let model_hit = pick(self.doc(), ray);
        match tool {
            PrepTool::Look => {
                self.selected = model_hit.map(|(id, _)| id);
            }
            PrepTool::Select => {
                let tol = self.camera.mm_per_point(rect, self.camera.target) * 4.0;
                let hit = crate::prep::pick_support(self.doc(), ray, tol);
                let model_t = model_hit.map(|(_, p)| (p - ray.origin).length());
                match hit {
                    Some((o, s, t)) if model_t.is_none_or(|mt| t < mt + 0.5) => {
                        let key = (o, s);
                        if shift {
                            if let Some(i) = self.prep.selection.iter().position(|k| *k == key) {
                                self.prep.selection.remove(i);
                            } else {
                                self.prep.selection.push(key);
                            }
                        } else {
                            self.prep.selection = vec![key];
                        }
                        self.selected = Some(o);
                    }
                    _ => {
                        if !shift {
                            self.prep.selection.clear();
                        }
                    }
                }
            }
            PrepTool::AddSupport => {
                if let Some((id, p)) = model_hit {
                    self.selected = Some(id);
                    self.run(&format!(
                        "supports.add id={id} x={} y={} z={}",
                        p.x, p.y, p.z
                    ));
                }
            }
            PrepTool::AddHole => {
                if let Some((id, p)) = model_hit {
                    self.selected = Some(id);
                    let h = self.doc().settings.hollow;
                    // No depth: the hole goes just through the wall of the hollow.
                    self.run(&format!(
                        "holes.add id={id} x={} y={} z={} diameter={}",
                        p.x, p.y, p.z, h.drain_hole_diameter_mm
                    ));
                }
            }
        }
    }

    fn build_frame(&mut self, look: &Look, rect: Rect, volume: Vec3) -> Frame3d {
        let p = &look.pal;
        let (w, d, h) = (volume.x / 2.0, volume.y / 2.0, volume.z);
        let mut tris = Vec::new();
        let plate = rgba(p.plate, 1.0);
        let v = |x: f32, y: f32, z: f32, color: [f32; 4]| ColorVertex {
            pos: [x, y, z],
            color,
        };
        for q in [[(-w, -d), (w, -d), (w, d)], [(-w, -d), (w, d), (-w, d)]] {
            for (x, y) in q {
                tris.push(v(x, y, 0.0, plate));
            }
        }
        let arrow = rgba(p.accent, 1.0);
        for a in gizmo::front_arrow(volume.y) {
            tris.push(v(a.x, a.y, 0.0, arrow));
        }
        let mut lines = Vec::new();
        let grid = rgba(p.grid, 1.0);
        let mut x = 0.0;
        while x <= w {
            for s in [-1.0, 1.0] {
                lines.push(v(s * x, -d, 0.02, grid));
                lines.push(v(s * x, d, 0.02, grid));
            }
            x += 10.0;
        }
        let mut y = 0.0;
        while y <= d {
            for s in [-1.0, 1.0] {
                lines.push(v(-w, s * y, 0.02, grid));
                lines.push(v(w, s * y, 0.02, grid));
            }
            y += 10.0;
        }
        let edge = rgba(p.plate_edge, 1.0);
        let vol = rgba(p.text_faint, 0.35);
        let corners = [(-w, -d), (w, -d), (w, d), (-w, d)];
        for i in 0..4 {
            let (a, b) = (corners[i], corners[(i + 1) % 4]);
            lines.push(v(a.0, a.1, 0.03, edge));
            lines.push(v(b.0, b.1, 0.03, edge));
            lines.push(v(a.0, a.1, h, vol));
            lines.push(v(b.0, b.1, h, vol));
            lines.push(v(a.0, a.1, 0.0, vol));
            lines.push(v(a.0, a.1, h, vol));
        }

        let selected = self.selected;
        // In Prepare a fully tinted model hides its supports: only a light tint there.
        let selected_color = if self.step == Step::Prepare {
            let (a, b) = (rgb(p.object), rgb(p.object_selected));
            [0, 1, 2].map(|i| a[i] * 0.7 + b[i] * 0.3)
        } else {
            rgb(p.object_selected)
        };
        // A model that sticks out of the build volume is tinted red: it would not print.
        let warn = |c: [f32; 3]| {
            let e = rgb(p.error);
            [0, 1, 2].map(|i| c[i] * 0.45 + e[i] * 0.55)
        };
        let mut objects: Vec<DrawObject> = self
            .doc()
            .objects()
            .iter()
            .map(|o| {
                let color = if Some(o.id) == selected {
                    selected_color
                } else {
                    rgb(p.object)
                };
                let outside = o
                    .world_bounds()
                    .is_some_and(|b| outside_volume(b.min, b.max, volume));
                DrawObject {
                    mesh: o.mesh.clone(),
                    model: o.transform.matrix(),
                    color: if outside { warn(color) } else { color },
                    overhang: true,
                }
            })
            .collect();
        // Supports, the selected ones highlighted, and drain hole markers.
        let support_color = if p.dark {
            [0.55, 0.62, 0.70]
        } else {
            [0.45, 0.52, 0.62]
        };
        let prepared: Vec<_> = self
            .doc()
            .objects()
            .iter()
            .map(|o| {
                (
                    o.id,
                    o.transform.translation,
                    o.prep.supports.clone(),
                    o.transform.matrix(),
                    o.prep.holes.clone(),
                )
            })
            .collect();
        for (id, pos, set, m, holes) in prepared {
            if let Some(set) = set {
                let c = crate::prep::support_mesh(&mut self.prep, id, &set, pos.z);
                let model = Mat4::from_translation(Vec3::new(pos.x, pos.y, 0.0));
                objects.push(DrawObject {
                    mesh: c.mesh.clone(),
                    model,
                    color: support_color,
                    overhang: false,
                });
                if let Some((_, sel)) = &c.selected {
                    objects.push(DrawObject {
                        mesh: sel.clone(),
                        model,
                        color: rgb(p.accent),
                        overhang: false,
                    });
                }
            }
            if !holes.is_empty() {
                let mut hm = copal_mesh::Mesh::default();
                for h in &holes {
                    let point = m.transform_point3(h.point);
                    let dir = m.transform_vector3(h.dir).normalize_or(Vec3::NEG_Z);
                    copal_core::supports::geometry::frustum(
                        &mut hm,
                        point - dir * 0.15,
                        h.diameter / 2.0,
                        point + dir * 0.5,
                        h.diameter / 2.0,
                        20,
                    );
                }
                objects.push(DrawObject {
                    mesh: Arc::new(hm),
                    model: Mat4::IDENTITY,
                    color: [0.12, 0.13, 0.16],
                    overhang: false,
                });
            }
        }
        // Outline (bounding box) of the selected object.
        if let Some(b) = selected
            .and_then(|id| self.doc().object(id))
            .and_then(|o| o.world_bounds())
        {
            let c = rgba(p.accent, 0.7);
            let (lo, hi) = (b.min, b.max);
            let pts = [
                Vec3::new(lo.x, lo.y, lo.z),
                Vec3::new(hi.x, lo.y, lo.z),
                Vec3::new(hi.x, hi.y, lo.z),
                Vec3::new(lo.x, hi.y, lo.z),
            ];
            for i in 0..4 {
                let (a, b2) = (pts[i], pts[(i + 1) % 4]);
                lines.push(v(a.x, a.y, 0.05, c));
                lines.push(v(b2.x, b2.y, 0.05, c));
            }
        }
        let clip_z = self.cut_height();
        if let Some(z) = clip_z {
            // Where the layer slider cuts: an outline around the plate at that height.
            let c = rgba(p.accent, 0.8);
            for i in 0..4 {
                let (a, b) = (corners[i], corners[(i + 1) % 4]);
                lines.push(v(a.0, a.1, z, c));
                lines.push(v(b.0, b.1, z, c));
            }
        }
        let eye_dir = (self.camera.eye() - self.camera.target).normalize();
        let light = (eye_dir + Vec3::new(0.3, 0.2, 0.9)).normalize();
        let overhangs = self.show_overhangs || self.step == Step::Prepare;
        Frame3d {
            view_proj: self.camera.view_projection(rect),
            light,
            objects,
            triangles: tris,
            lines,
            overhang_cos: overhangs.then(|| {
                self.doc()
                    .settings
                    .supports
                    .overhang_angle_deg
                    .to_radians()
                    .cos()
            }),
            overhang_color: OVERHANG_COLOR,
            clip_z,
        }
    }

    /// Layers of the whole plate, models and supports: (layer height, number of layers).
    fn plate_layers(&self) -> (f32, u32) {
        let doc = self.doc();
        let lh = doc.settings.print.exposure.layer_height_mm.max(0.001);
        let top = doc
            .objects()
            .iter()
            .filter_map(|o| o.world_bounds())
            .map(|b| b.max.z)
            .fold(0.0f32, f32::max);
        (lh, (top / lh).ceil() as u32)
    }

    /// Height where the layer slider cuts the view, if it does.
    fn cut_height(&self) -> Option<f32> {
        let (lh, count) = self.plate_layers();
        self.preview
            .cut
            .filter(|l| *l < count)
            .map(|l| l as f32 * lh + lh * 0.01)
    }

    /// Moves the layer slider by `steps` layers (up is positive).
    pub fn step_cut(&mut self, steps: i64) {
        let (_, count) = self.plate_layers();
        if count == 0 {
            return;
        }
        let now = self.preview.cut.unwrap_or(count) as i64;
        let next = (now + steps).clamp(1, count as i64) as u32;
        self.preview.cut = (next < count).then_some(next);
    }

    /// Shows every layer again.
    pub fn clear_cut(&mut self) {
        self.preview.cut = None;
    }

    /// The layer slider on the right of the 3D view: which layer to look at.
    fn layer_slider(&mut self, ui: &mut Ui, look: &Look, rect: Rect) {
        let (lh, count) = self.plate_layers();
        if count < 2 || rect.height() < 200.0 {
            return;
        }
        let area = Rect::from_min_max(
            pos2(rect.right() - 74.0, rect.top() + 56.0),
            pos2(rect.right() - 8.0, rect.bottom() - 16.0),
        );
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(area)
                .layout(egui::Layout::top_down(egui::Align::Center)),
        );
        let ui = &mut child;
        let mut l = self.preview.cut.unwrap_or(count).min(count);
        let label = if l >= count {
            "All".to_string()
        } else {
            format!("{l}")
        };
        ui.label(RichText::new(label).small().color(look.pal.accent_text))
            .on_hover_text("Layer shown in the 3D view");
        ui.label(
            RichText::new(format!("{:.2} mm", l as f32 * lh))
                .small()
                .color(look.pal.text_weak),
        );
        ui.spacing_mut().slider_width = (area.height() - 90.0).max(60.0);
        let r = ui
            .add(
                egui::Slider::new(&mut l, 1..=count)
                    .vertical()
                    .show_value(false),
            )
            .on_hover_text(format!(
                "Look at the model up to this layer. {} / {} move it, double-click shows \
                     every layer.",
                self.key_text("layer_up"),
                self.key_text("layer_down")
            ));
        if r.double_clicked() {
            l = count;
        }
        if r.changed() || r.double_clicked() {
            self.preview.cut = (l < count).then_some(l);
        }
        if self.preview.cut.is_some()
            && widgets::button(ui, look, widgets::ButtonKind::Normal, "All")
                .on_hover_text("Show every layer")
                .clicked()
        {
            self.preview.cut = None;
        }
    }

    /// Buttons on top of the view: camera presets and gizmo size.
    fn view_overlay(&mut self, ui: &mut Ui, look: &Look, rect: Rect) {
        let area = Rect::from_min_size(
            rect.left_top() + vec2(10.0, 10.0),
            vec2(rect.width() - 20.0, 30.0),
        );
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(area)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        let ui = &mut child;
        ui.spacing_mut().item_spacing.x = 4.0;
        let small =
            |ui: &mut Ui, text: &str| widgets::button(ui, look, widgets::ButtonKind::Normal, text);
        for (view, name, hint) in [
            (CameraView::Iso, "Iso", "Front-left view from above (4)"),
            (CameraView::Top, "Top", "From above (5)"),
            (CameraView::Front, "Front", "From the front (6)"),
            (CameraView::Side, "Side", "From the right side (7)"),
        ] {
            if small(ui, name).on_hover_text(hint).clicked() {
                self.set_camera_view(view);
            }
        }
        if small(ui, "Fit")
            .on_hover_text("Show the whole plate and every model (F or double-click)")
            .clicked()
        {
            self.framed = false;
        }
        ui.add_space(10.0);
        let label = if self.show_overhangs || self.step == Step::Prepare {
            "Overhangs: on"
        } else {
            "Overhangs"
        };
        if small(ui, label)
            .on_hover_text("Color the faces that need supports (O). Always on in Prepare.")
            .clicked()
        {
            self.show_overhangs = !self.show_overhangs;
        }
        if self.step == Step::Layout {
            // The three handles, right where the hand already is.
            ui.add_space(10.0);
            for (mode, name, id) in [
                (gizmo::Mode::Move, "Move", "mode_move"),
                (gizmo::Mode::Rotate, "Rotate", "mode_rotate"),
                (gizmo::Mode::Scale, "Scale", "mode_scale"),
            ] {
                let key = self.key_text(id);
                if crate::panels::tab_button(ui, look, self.gizmo_mode == mode, name, false)
                    .on_hover_text(format!("{name} handles ({key}). Ctrl snaps."))
                    .clicked()
                {
                    self.gizmo_mode = mode;
                }
            }
        }
        if self.step == Step::Layout && self.selected.is_some() {
            ui.add_space(10.0);
            ui.label(RichText::new("Handles").small().color(look.pal.text_weak));
            let mut s = self.settings.clone();
            if small(ui, "−").on_hover_text("Smaller gizmo").clicked() {
                s.gizmo_size = (s.gizmo_size - 0.2).max(copal_core::settings::GIZMO_SIZES.0);
            }
            if small(ui, "+").on_hover_text("Bigger gizmo").clicked() {
                s.gizmo_size = (s.gizmo_size + 0.2).min(copal_core::settings::GIZMO_SIZES.1);
            }
            s.gizmo_size = (s.gizmo_size * 10.0).round() / 10.0;
            self.set_settings(s);
        }
        if self.step == Step::Export {
            ui.add_space(10.0);
            if small(ui, "Layers")
                .on_hover_text("Show the sliced layers (L)")
                .clicked()
            {
                self.preview.show = true;
            }
        }
        if self.doc().objects().is_empty() {
            ui.painter().text(
                rect.center_bottom() - vec2(0.0, 30.0),
                Align2::CENTER_CENTER,
                "Drop STL, OBJ or 3MF files here, or use Import (Ctrl+I)",
                FontId::proportional(14.0),
                look.pal.text_faint,
            );
        }
    }

    fn preview_slicer(&mut self) -> Result<Arc<Slicer>, String> {
        let rev = self.session().history.revision();
        if let Some((r, s)) = &self.preview.slicer
            && *r == rev
        {
            return Ok(s.clone());
        }
        let doc = self.doc();
        let printer = copal_formats::export::project_printer(doc).map_err(|e| e.to_string())?;
        let full = Raster::new(printer.screen.resolution, printer.screen.size_mm);
        let factor = (2000.0 / full.width as f32).min(1.0);
        let raster = full.scaled(factor);
        let s = &doc.settings.print;
        let t = copal_formats::export::scene_triangles(doc);
        let slicer = Arc::new(Slicer::with_cuts(
            t.body,
            t.cuts,
            t.keep,
            raster,
            s.exposure.layer_height_mm,
            4,
        ));
        self.preview.slicer = Some((rev, slicer.clone()));
        self.preview.texture = None;
        Ok(slicer)
    }

    fn layer_view(&mut self, ui: &mut Ui, look: &Look, rect: Rect) {
        let _ = ui.allocate_rect(rect, Sense::hover());
        let slicer = match self.preview_slicer() {
            Ok(s) => s,
            Err(e) => {
                ui.painter().text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    e,
                    FontId::proportional(14.0),
                    look.pal.error,
                );
                self.layer_controls(ui, look, rect, 0);
                return;
            }
        };
        let count = slicer.layer_count();
        if count == 0 {
            ui.painter().text(
                rect.center(),
                Align2::CENTER_CENTER,
                "Nothing to slice",
                FontId::proportional(14.0),
                look.pal.text_faint,
            );
            self.layer_controls(ui, look, rect, 0);
            return;
        }
        self.preview.layer = self.preview.layer.min(count - 1);
        if ui.rect_contains_pointer(rect) {
            let (up, down, scroll) = ui.input(|i| {
                (
                    i.key_pressed(egui::Key::ArrowUp),
                    i.key_pressed(egui::Key::ArrowDown),
                    i.smooth_scroll_delta.y,
                )
            });
            let mut l = self.preview.layer as i64;
            if up || scroll > 0.0 {
                l += 1;
            }
            if down || scroll < 0.0 {
                l -= 1;
            }
            self.preview.layer = l.clamp(0, count as i64 - 1) as u32;
        }
        let rev = self.session().history.revision();
        let layer = self.preview.layer;
        let need = !matches!(&self.preview.texture, Some((r, l, _)) if *r == rev && *l == layer);
        if need {
            let (img, _) = slicer.layer_image(layer);
            let r = slicer.raster;
            let lit = look.pal.accent;
            let pixels: Vec<Color32> = img
                .iter()
                .map(|v| {
                    if *v == 0 {
                        Color32::from_gray(if look.pal.dark { 14 } else { 30 })
                    } else {
                        let k = *v as f32 / 255.0;
                        Color32::from_rgb(
                            (lit.r() as f32 * k + 20.0 * (1.0 - k)) as u8,
                            (lit.g() as f32 * k + 20.0 * (1.0 - k)) as u8,
                            (lit.b() as f32 * k + 20.0 * (1.0 - k)) as u8,
                        )
                    }
                })
                .collect();
            let image = ColorImage::new([r.width as usize, r.height as usize], pixels);
            let tex = ui
                .ctx()
                .load_texture("copal-layer", image, TextureOptions::LINEAR);
            self.preview.texture = Some((rev, layer, tex));
        }
        if let Some((_, _, tex)) = &self.preview.texture {
            let size = tex.size_vec2();
            let avail = rect.shrink(40.0);
            let scale = (avail.width() / size.x).min(avail.height() / size.y);
            let img_rect = Rect::from_center_size(avail.center(), size * scale);
            ui.painter().image(
                tex.id(),
                img_rect,
                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                Color32::WHITE,
            );
            ui.painter().rect_stroke(
                img_rect,
                0.0,
                Stroke::new(1.0, look.pal.plate_edge),
                egui::StrokeKind::Outside,
            );
            ui.painter().text(
                img_rect.center_bottom() + vec2(0.0, 14.0),
                Align2::CENTER_CENTER,
                "front",
                FontId::proportional(11.0),
                look.pal.text_faint,
            );
        }
        let e = &self.doc().settings.print.exposure;
        let info = format!(
            "Layer {} / {}   ·   z {:.3} mm   ·   exposure {:.2} s",
            layer + 1,
            count,
            (layer + 1) as f32 * e.layer_height_mm,
            e.exposure_of(layer)
        );
        ui.painter().text(
            rect.center_top() + vec2(0.0, 56.0),
            Align2::CENTER_CENTER,
            info,
            FontId::proportional(13.0),
            look.pal.text,
        );
        self.layer_controls(ui, look, rect, count);
    }

    fn layer_controls(&mut self, ui: &mut Ui, look: &Look, rect: Rect, count: u32) {
        let bar = Rect::from_min_size(
            rect.left_top() + vec2(10.0, 10.0),
            vec2(rect.width() - 20.0, 30.0),
        );
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(bar)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        if widgets::button(&mut child, look, widgets::ButtonKind::Normal, "3D view")
            .on_hover_text("Back to the 3D view (L)")
            .clicked()
        {
            self.preview.show = false;
        }
        if count > 1 {
            child.add_space(12.0);
            child.spacing_mut().slider_width = (bar.width() - 200.0).max(100.0);
            let mut l = self.preview.layer + 1;
            child.add(egui::Slider::new(&mut l, 1..=count).text("layer"));
            self.preview.layer = l - 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::Camera;
    use copal_core::{Document, Session};

    #[test]
    fn picking_finds_the_object_under_the_cursor() {
        let mut s = Session::new(Document::new("t"));
        s.run_script("object.add_cube size=20\nobject.add_cube size=10\nobject.move id=2 x=40")
            .unwrap();
        let rect = Rect::from_min_size(egui::pos2(250.0, 50.0), egui::vec2(850.0, 630.0));
        let mut cam = Camera::default();
        cam.frame(Vec3::new(-105.0, -60.0, 0.0), Vec3::new(105.0, 60.0, 20.0));
        for (id, center) in [
            (1, Vec3::new(0.0, 0.0, 10.0)),
            (2, Vec3::new(40.0, 0.0, 5.0)),
        ] {
            let p = cam.project(rect, center).unwrap();
            let hit = pick(&s.doc, &cam.ray(rect, p));
            assert_eq!(hit.map(|h| h.0.0), Some(id), "object {id} at {p:?}");
        }
        let empty = cam.project(rect, Vec3::new(-80.0, 40.0, 0.0)).unwrap();
        assert_eq!(pick(&s.doc, &cam.ray(rect, empty)), None);
    }

    #[test]
    fn dragging_on_the_plate_follows_the_cursor_but_never_runs_away() {
        let rect = Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1000.0, 700.0));
        let cam = Camera::default();
        let grab = Vec3::new(10.0, -5.0, 8.0);
        // The pointer over another point at the grab height: the model goes exactly there.
        let to = Vec3::new(40.0, 20.0, 8.0);
        let p = cam.project(rect, to).unwrap();
        let hit = plate_point(&cam.ray(rect, p), grab, 300.0).unwrap();
        assert!((hit - to).length() < 0.05, "{hit}");
        // A view almost level with the plate: a few pixels up means hundreds of metres away.
        let mut flat = Camera::default();
        flat.set_view(0.0, 0.5);
        let horizon = flat.project(rect, Vec3::new(0.0, 5000.0, 8.0)).unwrap();
        assert_eq!(plate_point(&flat.ray(rect, horizon), grab, 300.0), None);
        // Above the horizon there is no point on the plate at all.
        let sky = egui::pos2(500.0, 5.0);
        assert_eq!(plate_point(&flat.ray(rect, sky), grab, 300.0), None);
    }

    #[test]
    fn models_outside_the_build_volume_are_found() {
        let volume = Vec3::new(200.0, 100.0, 150.0);
        let inside = (Vec3::new(-10.0, -10.0, 0.0), Vec3::new(10.0, 10.0, 20.0));
        assert!(!outside_volume(inside.0, inside.1, volume));
        let shift = |d: Vec3| outside_volume(inside.0 + d, inside.1 + d, volume);
        assert!(shift(Vec3::new(95.0, 0.0, 0.0)), "over the right edge");
        assert!(shift(Vec3::new(0.0, -45.0, 0.0)), "over the front edge");
        assert!(shift(Vec3::new(0.0, 0.0, -1.0)), "under the plate");
        assert!(shift(Vec3::new(0.0, 0.0, 131.0)), "too tall");
        assert!(!shift(Vec3::new(90.0, 40.0, 130.0)), "just fits");
    }
}
