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

/// The layer preview of the Export step.
#[derive(Default)]
pub(crate) struct LayerPreview {
    pub show: bool,
    pub layer: u32,
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

fn ray_hits_box(ray: &Ray, min: Vec3, max: Vec3) -> bool {
    let inv = ray.dir.recip();
    let t1 = (min - ray.origin) * inv;
    let t2 = (max - ray.origin) * inv;
    let tmin = t1.min(t2).max_element();
    let tmax = t1.max(t2).min_element();
    tmax >= tmin.max(0.0)
}

impl Workspace {
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
        if let Some(o) = self.doc().object(id) {
            let mut probe = o.clone();
            probe.transform = t;
            if let Some(b) = probe.world_bounds() {
                t.translation.z -= b.min.z;
            }
        }
        t
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
            && let Some(p) = response.interact_pointer_pos()
        {
            let (primary, shift) = ui.input(|i| (i.pointer.primary_down(), i.modifiers.shift));
            self.drag = if !primary || shift {
                Some(ViewDrag::Pan)
            } else if let Some((id, g)) = gizmo
                && let Some(h) = g.hit(&self.camera, rect, self.gizmo_mode, p)
                && let Some(o) = self.doc().object(id)
                && let Some(drag) = Drag::start(h, g, o.transform, &self.camera, rect, p)
            {
                let on_plate = o.world_bounds().is_some_and(|b| b.min.z.abs() < 0.01);
                Some(ViewDrag::Gizmo {
                    id,
                    drag,
                    keep_on_plate: layout.keep_on_plate
                        && on_plate
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
                        }
                        self.preview_transform(id, t);
                    }
                }
                ViewDrag::Plate { id, start, grab } => {
                    if let Some(p) = pos
                        && let Some(hit) = self.camera.ray(rect, p).hit_plane_z(grab.z)
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

        // Click to select.
        if response.clicked()
            && let Some(p) = response.interact_pointer_pos()
        {
            self.selected = pick(self.doc(), &self.camera.ray(rect, p)).map(|(id, _)| id);
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
        self.view_overlay(ui, look, rect);
    }

    fn build_frame(&self, look: &Look, rect: Rect, volume: Vec3) -> Frame3d {
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
        let objects = self
            .doc()
            .objects()
            .iter()
            .map(|o| DrawObject {
                mesh: o.mesh.clone(),
                model: o.transform.matrix(),
                color: rgb(if Some(o.id) == selected {
                    p.object_selected
                } else {
                    p.object
                }),
            })
            .collect();
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
            overhang_color: [0.86, 0.26, 0.22],
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
        if small(ui, "Iso")
            .on_hover_text("Front-left view from above")
            .clicked()
        {
            self.camera.set_view(-28.0, 30.0);
            self.framed = false;
        }
        if small(ui, "Top").clicked() {
            self.camera.set_view(0.0, 89.9);
            self.framed = false;
        }
        if small(ui, "Front").clicked() {
            self.camera.set_view(0.0, 4.0);
            self.framed = false;
        }
        if small(ui, "Side").clicked() {
            self.camera.set_view(90.0, 4.0);
            self.framed = false;
        }
        ui.add_space(10.0);
        let label = if self.show_overhangs || self.step == Step::Prepare {
            "Overhangs: on"
        } else {
            "Overhangs"
        };
        if small(ui, label)
            .on_hover_text("Color the faces that need supports (always on in Prepare)")
            .clicked()
        {
            self.show_overhangs = !self.show_overhangs;
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
                .on_hover_text("Show the sliced layers")
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
        let slicer = Arc::new(Slicer::new(
            copal_formats::export::scene_triangles(doc),
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
        if widgets::button(&mut child, look, widgets::ButtonKind::Normal, "3D view").clicked() {
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
}
