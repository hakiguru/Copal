//! A simple 3D view of the build plate, drawn with egui shapes.
//!
//! This is the preview renderer: orthographic camera, flat lighting, painter's algorithm.
//! It is fast enough for test objects; the real GPU renderer comes in 0.2 (Lite) and 0.4 (full).

use copal_core::{Document, ObjectId};
use egui::{Color32, Mesh, Painter, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, pos2};
use glam::Vec3;

use crate::palette::{Palette, shade};

/// Known build volumes (mm). The preset database replaces this in 0.1.
pub fn build_volume(printer: Option<&str>) -> Vec3 {
    match printer {
        Some("elegoo/saturn-4-ultra-16k") => Vec3::new(211.68, 118.37, 220.0),
        _ => Vec3::new(200.0, 125.0, 200.0),
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Camera {
    /// Rotation around the vertical axis, degrees.
    pub yaw: f32,
    /// Angle above the plate, degrees.
    pub pitch: f32,
    /// Screen points per millimeter. 0 = fit to the view on the next frame.
    pub zoom: f32,
    pub target: Vec3,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            yaw: -35.0,
            pitch: 32.0,
            zoom: 0.0,
            target: Vec3::ZERO,
        }
    }
}

struct View {
    right: Vec3,
    up: Vec3,
    forward: Vec3,
    target: Vec3,
    zoom: f32,
    center: Pos2,
}

impl View {
    fn new(cam: &Camera, rect: Rect) -> Self {
        let (yaw, pitch) = (cam.yaw.to_radians(), cam.pitch.to_radians());
        // Direction from the camera towards the target.
        let forward = -Vec3::new(
            yaw.sin() * pitch.cos(),
            -yaw.cos() * pitch.cos(),
            pitch.sin(),
        );
        let right = forward.cross(Vec3::Z).normalize();
        let up = right.cross(forward);
        Self {
            right,
            up,
            forward,
            target: cam.target,
            zoom: cam.zoom,
            center: rect.center(),
        }
    }

    fn project(&self, p: Vec3) -> (Pos2, f32) {
        let d = p - self.target;
        let screen = pos2(
            self.center.x + d.dot(self.right) * self.zoom,
            self.center.y - d.dot(self.up) * self.zoom,
        );
        (screen, d.dot(self.forward))
    }
}

/// What happened in the viewport this frame.
pub struct ViewportOutput {
    pub response: Response,
    /// Set when the user clicked: `Some(id)` on an object, `None` on empty space.
    pub clicked: Option<Option<ObjectId>>,
}

pub fn show(
    ui: &mut Ui,
    doc: &Document,
    cam: &mut Camera,
    selected: Option<ObjectId>,
    pal: &Palette,
) -> ViewportOutput {
    let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
    let rect = response.rect;
    painter.rect_filled(rect, 0.0, pal.void);

    let volume = build_volume(doc.printer.as_deref());
    if cam.zoom <= 0.0 {
        cam.zoom = (rect.width() / volume.x.max(volume.y))
            .min(rect.height() / volume.z.max(volume.y))
            * 0.62;
        cam.target = Vec3::new(0.0, 0.0, volume.z * 0.25);
    }
    // Orbit by dragging, zoom with the wheel.
    if response.dragged() {
        let d = response.drag_delta();
        cam.yaw -= d.x * 0.4;
        cam.pitch = (cam.pitch + d.y * 0.4).clamp(-5.0, 89.0);
    }
    if response.hovered() {
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll != 0.0 {
            cam.zoom = (cam.zoom * (1.0 + scroll * 0.0015)).clamp(0.2, 40.0);
        }
    }

    let view = View::new(cam, rect);
    let painter = painter.with_clip_rect(rect);
    draw_plate(&painter, &view, volume, pal);
    let hits = draw_objects(&painter, &view, doc, selected, pal);

    let clicked = if response.clicked() {
        let pointer = response.interact_pointer_pos();
        Some(pointer.and_then(|p| {
            hits.iter()
                .rev()
                .find(|(tri, _)| inside(p, tri))
                .map(|(_, id)| *id)
        }))
    } else {
        None
    };
    ViewportOutput { response, clicked }
}

fn draw_plate(painter: &Painter, view: &View, volume: Vec3, pal: &Palette) {
    let (hx, hy) = (volume.x / 2.0, volume.y / 2.0);
    let corners = [
        Vec3::new(-hx, -hy, 0.0),
        Vec3::new(hx, -hy, 0.0),
        Vec3::new(hx, hy, 0.0),
        Vec3::new(-hx, hy, 0.0),
    ];
    let plate: Vec<Pos2> = corners.iter().map(|c| view.project(*c).0).collect();
    painter.add(Shape::convex_polygon(
        plate.clone(),
        pal.plate,
        Stroke::NONE,
    ));

    let grid = Stroke::new(1.0, pal.grid);
    let mut x = -hx + (hx % 10.0);
    while x <= hx {
        let a = view.project(Vec3::new(x, -hy, 0.0)).0;
        let b = view.project(Vec3::new(x, hy, 0.0)).0;
        painter.line_segment([a, b], grid);
        x += 10.0;
    }
    let mut y = -hy + (hy % 10.0);
    while y <= hy {
        let a = view.project(Vec3::new(-hx, y, 0.0)).0;
        let b = view.project(Vec3::new(hx, y, 0.0)).0;
        painter.line_segment([a, b], grid);
        y += 10.0;
    }
    let edge = Stroke::new(1.5, pal.plate_edge);
    for i in 0..4 {
        painter.line_segment([plate[i], plate[(i + 1) % 4]], edge);
    }
    // Build volume, very faint.
    let faint = Stroke::new(1.0, pal.volume);
    for i in 0..4 {
        let (a, b) = (corners[i], corners[(i + 1) % 4]);
        let up = Vec3::Z * volume.z;
        painter.line_segment([view.project(a).0, view.project(a + up).0], faint);
        painter.line_segment([view.project(a + up).0, view.project(b + up).0], faint);
    }
    // Front marker, so it is clear which side faces you.
    let front = view.project(Vec3::new(0.0, -hy - 6.0, 0.0)).0;
    painter.text(
        front,
        egui::Align2::CENTER_CENTER,
        "FRONT",
        egui::FontId::proportional(10.0),
        pal.text_faint,
    );
}

type Tri = [Pos2; 3];

fn draw_objects(
    painter: &Painter,
    view: &View,
    doc: &Document,
    selected: Option<ObjectId>,
    pal: &Palette,
) -> Vec<(Tri, ObjectId)> {
    let light = Vec3::new(-0.4, -0.6, 0.85).normalize();
    let mut tris: Vec<(f32, Tri, Color32, ObjectId)> = Vec::new();
    for o in doc.objects() {
        let m = o.transform.matrix();
        let world: Vec<Vec3> = o
            .mesh
            .positions
            .iter()
            .map(|p| m.transform_point3(*p))
            .collect();
        let base = if selected == Some(o.id) {
            pal.object_selected
        } else {
            pal.object
        };
        for t in &o.mesh.triangles {
            let [a, b, c] = t.map(|i| world[i as usize]);
            let n = (b - a).cross(c - a).normalize_or_zero();
            if n.dot(view.forward) > 0.0 {
                continue; // back face
            }
            let (pa, da) = view.project(a);
            let (pb, db) = view.project(b);
            let (pc, dc) = view.project(c);
            let lit = 0.5 + 0.5 * n.dot(light).max(0.0);
            tris.push(((da + db + dc) / 3.0, [pa, pb, pc], shade(base, lit), o.id));
        }
    }
    // Far to near.
    tris.sort_by(|x, y| y.0.total_cmp(&x.0));
    let mut mesh = Mesh::default();
    for (_, [a, b, c], color, _) in &tris {
        let i = mesh.vertices.len() as u32;
        mesh.colored_vertex(*a, *color);
        mesh.colored_vertex(*b, *color);
        mesh.colored_vertex(*c, *color);
        mesh.add_triangle(i, i + 1, i + 2);
    }
    painter.add(Shape::mesh(mesh));
    tris.into_iter().map(|(_, t, _, id)| (t, id)).collect()
}

fn inside(p: Pos2, [a, b, c]: &Tri) -> bool {
    let s = |p1: Pos2, p2: Pos2, p3: Pos2| {
        (p1.x - p3.x) * (p2.y - p3.y) - (p2.x - p3.x) * (p1.y - p3.y)
    };
    let (d1, d2, d3) = (s(p, *a, *b), s(p, *b, *c), s(p, *c, *a));
    let neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
    let pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
    !(neg && pos)
}
