//! Move, rotate and scale handles drawn over the 3D view.
//!
//! Handles keep the same size on screen at any zoom; the user can make them bigger or smaller
//! (`gizmo` in the app settings).

use copal_core::Transform;
use egui::{Color32, Painter, Pos2, Rect, Shape, Stroke, vec2};
use glam::{EulerRot, Quat, Vec3};

use crate::camera::{Camera, Ray};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    Move,
    Rotate,
    Scale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Handle {
    /// Move along X, Y or Z.
    Axis(usize),
    /// Move on the plate (X and Y together).
    Plane,
    /// Rotate around X, Y or Z.
    Ring(usize),
    /// Scale along the object's own X, Y or Z.
    ScaleAxis(usize),
    /// Scale evenly.
    Uniform,
}

pub const AXIS_COLORS: [Color32; 3] = [
    Color32::from_rgb(230, 72, 62),
    Color32::from_rgb(98, 190, 70),
    Color32::from_rgb(62, 130, 235),
];

/// Where the gizmo is and how big.
#[derive(Debug, Clone, Copy)]
pub struct Gizmo {
    pub center: Vec3,
    /// World directions of the handles. World axes for move and rotate, the object's own axes
    /// for scale.
    pub axes: [Vec3; 3],
    /// Handle length in mm (depends on zoom).
    pub length: f32,
}

impl Gizmo {
    pub fn new(
        cam: &Camera,
        rect: Rect,
        center: Vec3,
        rotation_deg: Vec3,
        size: f32,
        mode: Mode,
    ) -> Self {
        let axes = if mode == Mode::Scale {
            let q = quat(rotation_deg);
            [q * Vec3::X, q * Vec3::Y, q * Vec3::Z]
        } else {
            [Vec3::X, Vec3::Y, Vec3::Z]
        };
        Self {
            center,
            axes,
            length: 90.0 * size * cam.mm_per_point(rect, center),
        }
    }

    fn ring_points(&self, axis: usize, n: usize) -> Vec<Vec3> {
        let u = self.axes[(axis + 1) % 3];
        let v = self.axes[(axis + 2) % 3];
        let r = self.length * 0.8;
        (0..=n)
            .map(|i| {
                let t = i as f32 / n as f32 * std::f32::consts::TAU;
                self.center + r * (u * t.cos() + v * t.sin())
            })
            .collect()
    }

    /// The handle under the pointer.
    pub fn hit(&self, cam: &Camera, rect: Rect, mode: Mode, pos: Pos2) -> Option<Handle> {
        let c = cam.project(rect, self.center)?;
        let near = |a: Pos2, b: Pos2, tol: f32| seg_dist(pos, a, b) < tol;
        match mode {
            Mode::Move => {
                for i in 0..3 {
                    let tip = cam.project(rect, self.center + self.axes[i] * self.length)?;
                    if (tip - c).length() > 12.0 && near(c + (tip - c) * 0.2, tip, 7.0) {
                        return Some(Handle::Axis(i));
                    }
                }
                let sq = self.plane_square(cam, rect)?;
                if point_in_quad(pos, &sq) {
                    return Some(Handle::Plane);
                }
                None
            }
            Mode::Rotate => {
                let mut best: Option<(f32, usize)> = None;
                for i in 0..3 {
                    let pts: Vec<Pos2> = self
                        .ring_points(i, 64)
                        .into_iter()
                        .filter_map(|p| cam.project(rect, p))
                        .collect();
                    let d = pts
                        .windows(2)
                        .map(|w| seg_dist(pos, w[0], w[1]))
                        .fold(f32::INFINITY, f32::min);
                    if d < 7.0 && best.is_none_or(|(bd, _)| d < bd) {
                        best = Some((d, i));
                    }
                }
                best.map(|(_, i)| Handle::Ring(i))
            }
            Mode::Scale => {
                if (pos - c).length() < 9.0 {
                    return Some(Handle::Uniform);
                }
                for i in 0..3 {
                    let tip = cam.project(rect, self.center + self.axes[i] * self.length)?;
                    if (tip - c).length() > 12.0 && near(c + (tip - c) * 0.2, tip, 7.0) {
                        return Some(Handle::ScaleAxis(i));
                    }
                }
                None
            }
        }
    }

    fn plane_square(&self, cam: &Camera, rect: Rect) -> Option<[Pos2; 4]> {
        let (a, b) = (
            self.axes[0] * self.length * 0.22,
            self.axes[1] * self.length * 0.22,
        );
        let o = self.center + (a + b) * 0.5;
        Some([
            cam.project(rect, o)?,
            cam.project(rect, o + a)?,
            cam.project(rect, o + a + b)?,
            cam.project(rect, o + b)?,
        ])
    }

    /// Draws the gizmo. `hot` is the hovered or dragged handle.
    pub fn draw(
        &self,
        painter: &Painter,
        cam: &Camera,
        rect: Rect,
        mode: Mode,
        hot: Option<Handle>,
        accent: Color32,
    ) {
        let Some(c) = cam.project(rect, self.center) else {
            return;
        };
        let width = |h: Handle| if hot == Some(h) { 3.5 } else { 2.2 };
        let color = |i: usize, h: Handle| {
            if hot == Some(h) {
                Color32::from_rgb(255, 230, 120)
            } else {
                AXIS_COLORS[i]
            }
        };
        match mode {
            Mode::Move => {
                if let Some(sq) = self.plane_square(cam, rect) {
                    let hotp = hot == Some(Handle::Plane);
                    let fill = if hotp {
                        accent.gamma_multiply(0.7)
                    } else {
                        accent.gamma_multiply(0.35)
                    };
                    painter.add(Shape::convex_polygon(
                        sq.to_vec(),
                        fill,
                        Stroke::new(1.0, accent),
                    ));
                }
                for i in 0..3 {
                    let Some(tip) = cam.project(rect, self.center + self.axes[i] * self.length)
                    else {
                        continue;
                    };
                    let d = tip - c;
                    if d.length() < 12.0 {
                        continue;
                    }
                    let h = Handle::Axis(i);
                    let col = color(i, h);
                    painter.line_segment([c + d * 0.15, tip], Stroke::new(width(h), col));
                    let dir = d.normalized();
                    let side = vec2(-dir.y, dir.x);
                    let head = 12.0 + if hot == Some(h) { 3.0 } else { 0.0 };
                    painter.add(Shape::convex_polygon(
                        vec![
                            tip + dir * head,
                            tip + side * head * 0.42,
                            tip - side * head * 0.42,
                        ],
                        col,
                        Stroke::NONE,
                    ));
                }
            }
            Mode::Rotate => {
                for i in 0..3 {
                    let pts: Vec<Pos2> = self
                        .ring_points(i, 72)
                        .into_iter()
                        .filter_map(|p| cam.project(rect, p))
                        .collect();
                    let h = Handle::Ring(i);
                    painter.add(Shape::line(pts, Stroke::new(width(h), color(i, h))));
                }
                painter.circle_filled(c, 3.0, accent);
            }
            Mode::Scale => {
                for i in 0..3 {
                    let Some(tip) = cam.project(rect, self.center + self.axes[i] * self.length)
                    else {
                        continue;
                    };
                    let d = tip - c;
                    if d.length() < 12.0 {
                        continue;
                    }
                    let h = Handle::ScaleAxis(i);
                    let col = color(i, h);
                    painter.line_segment([c + d * 0.15, tip], Stroke::new(width(h), col));
                    let s = if hot == Some(h) { 7.0 } else { 5.5 };
                    painter.rect_filled(
                        Rect::from_center_size(tip, vec2(2.0 * s, 2.0 * s)),
                        1.0,
                        col,
                    );
                }
                let s = if hot == Some(Handle::Uniform) {
                    8.0
                } else {
                    6.0
                };
                painter.rect_filled(
                    Rect::from_center_size(c, vec2(2.0 * s, 2.0 * s)),
                    1.0,
                    accent,
                );
            }
        }
    }
}

/// A drag of one handle, from the moment the button went down.
#[derive(Debug, Clone, Copy)]
pub struct Drag {
    pub handle: Handle,
    pub gizmo: Gizmo,
    pub start: Transform,
    /// Measured at the start: axis parameter, angle, plane point or screen distance.
    start_value: Vec3,
}

fn ring_angle(g: &Gizmo, axis: usize, ray: &Ray) -> Option<f32> {
    let n = g.axes[axis];
    let denom = n.dot(ray.dir);
    if denom.abs() < 1e-4 {
        return None;
    }
    let t = n.dot(g.center - ray.origin) / denom;
    let p = ray.origin + ray.dir * t - g.center;
    let u = g.axes[(axis + 1) % 3];
    let v = g.axes[(axis + 2) % 3];
    Some(p.dot(v).atan2(p.dot(u)))
}

impl Drag {
    pub fn start(
        handle: Handle,
        gizmo: Gizmo,
        start: Transform,
        cam: &Camera,
        rect: Rect,
        pos: Pos2,
    ) -> Option<Self> {
        let ray = cam.ray(rect, pos);
        let start_value = match handle {
            Handle::Axis(i) | Handle::ScaleAxis(i) => {
                Vec3::splat(ray.closest_on_line(gizmo.center, gizmo.axes[i])?)
            }
            Handle::Plane => ray.hit_plane_z(gizmo.center.z)?,
            Handle::Ring(i) => Vec3::splat(ring_angle(&gizmo, i, &ray)?),
            Handle::Uniform => {
                let c = cam.project(rect, gizmo.center)?;
                Vec3::splat((pos - c).length().max(1.0))
            }
        };
        Some(Self {
            handle,
            gizmo,
            start,
            start_value,
        })
    }

    /// The transform for the pointer at `pos`. `snap`: Ctrl is held, round to these steps
    /// (mm, degrees, percent).
    pub fn update(
        &self,
        cam: &Camera,
        rect: Rect,
        pos: Pos2,
        snap: Option<(f32, f32, f32)>,
    ) -> Option<Transform> {
        let ray = cam.ray(rect, pos);
        let g = &self.gizmo;
        let mut t = self.start;
        let round = |v: f32, step: f32| {
            if step > 0.0 {
                (v / step).round() * step
            } else {
                v
            }
        };
        match self.handle {
            Handle::Axis(i) => {
                let mut d = ray.closest_on_line(g.center, g.axes[i])? - self.start_value.x;
                if let Some((mm, _, _)) = snap {
                    d = round(d, mm);
                }
                t.translation += g.axes[i] * d;
            }
            Handle::Plane => {
                let p = ray.hit_plane_z(g.center.z)?;
                let mut d = p - self.start_value;
                if let Some((mm, _, _)) = snap {
                    d = Vec3::new(round(d.x, mm), round(d.y, mm), 0.0);
                }
                t.translation += Vec3::new(d.x, d.y, 0.0);
            }
            Handle::Ring(i) => {
                let a = ring_angle(g, i, &ray)?;
                let mut delta = (a - self.start_value.x).to_degrees();
                // Keep it in -180..180 so crossing the seam does not jump.
                delta = (delta + 540.0).rem_euclid(360.0) - 180.0;
                if let Some((_, deg, _)) = snap {
                    delta = round(delta, deg);
                }
                let q = Quat::from_axis_angle(g.axes[i], delta.to_radians());
                let r = q * quat(self.start.rotation_deg);
                let (x, y, z) = r.to_euler(EulerRot::XYZ);
                t.rotation_deg = clean_angles(Vec3::new(x, y, z));
                t.translation = g.center + q * (self.start.translation - g.center);
            }
            Handle::ScaleAxis(i) => {
                let now = ray.closest_on_line(g.center, g.axes[i])?;
                let mut f = (now / self.start_value.x).clamp(0.01, 100.0);
                if let Some((_, _, pct)) = snap {
                    f = round(f * 100.0, pct).max(pct) / 100.0;
                }
                t.scale[i] = self.start.scale[i] * f;
            }
            Handle::Uniform => {
                let c = cam.project(rect, g.center)?;
                let mut f = ((pos - c).length().max(1.0) / self.start_value.x).clamp(0.01, 100.0);
                if let Some((_, _, pct)) = snap {
                    f = round(f * 100.0, pct).max(pct) / 100.0;
                }
                t.scale = self.start.scale * f;
            }
        }
        Some(t)
    }
}

pub fn quat(rotation_deg: Vec3) -> Quat {
    Quat::from_euler(
        EulerRot::XYZ,
        rotation_deg.x.to_radians(),
        rotation_deg.y.to_radians(),
        rotation_deg.z.to_radians(),
    )
}

/// Degrees with tiny float noise removed (89.99999 → 90).
fn clean_angles(r: Vec3) -> Vec3 {
    let c = |v: f32| {
        let d = v.to_degrees();
        let r = (d * 1000.0).round() / 1000.0;
        if r == -0.0 { 0.0 } else { r }
    };
    Vec3::new(c(r.x), c(r.y), c(r.z))
}

fn seg_dist(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_sq().max(1e-6)).clamp(0.0, 1.0);
    (p - (a + ab * t)).length()
}

fn point_in_quad(p: Pos2, q: &[Pos2; 4]) -> bool {
    let sign = |a: Pos2, b: Pos2| (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x);
    let s: Vec<f32> = (0..4).map(|i| sign(q[i], q[(i + 1) % 4])).collect();
    s.iter().all(|v| *v >= 0.0) || s.iter().all(|v| *v <= 0.0)
}

/// A small triangle on the plate in front of it, pointing at the viewer: marks the front.
pub fn front_arrow(depth: f32) -> [Vec3; 3] {
    let y = -depth / 2.0 - 4.0;
    [
        Vec3::new(-7.0, y, 0.0),
        Vec3::new(7.0, y, 0.0),
        Vec3::new(0.0, y - 9.0, 0.0),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::pos2;

    fn setup() -> (Camera, Rect) {
        (
            Camera::default(),
            Rect::from_min_size(pos2(0.0, 0.0), vec2(1000.0, 700.0)),
        )
    }

    #[test]
    fn dragging_the_x_arrow_moves_along_x_only() {
        let (cam, rect) = setup();
        let start = Transform::default();
        let g = Gizmo::new(
            &cam,
            rect,
            Vec3::new(0.0, 0.0, 10.0),
            Vec3::ZERO,
            1.0,
            Mode::Move,
        );
        let tip = cam
            .project(rect, g.center + Vec3::X * g.length * 0.8)
            .unwrap();
        assert_eq!(g.hit(&cam, rect, Mode::Move, tip), Some(Handle::Axis(0)));
        let drag = Drag::start(Handle::Axis(0), g, start, &cam, rect, tip).unwrap();
        let target = cam
            .project(rect, g.center + Vec3::X * (g.length * 0.8 + 25.0))
            .unwrap();
        let t = drag.update(&cam, rect, target, None).unwrap();
        assert!((t.translation.x - 25.0).abs() < 0.2, "{:?}", t.translation);
        assert!(t.translation.y.abs() < 1e-3 && t.translation.z.abs() < 1e-3);
        let snapped = drag
            .update(&cam, rect, target, Some((10.0, 15.0, 10.0)))
            .unwrap();
        assert_eq!(snapped.translation.x, 30.0);
    }

    #[test]
    fn rotating_a_quarter_turn_around_z() {
        let (cam, rect) = setup();
        let start = Transform::default();
        let g = Gizmo::new(&cam, rect, Vec3::ZERO, Vec3::ZERO, 1.0, Mode::Rotate);
        let r = g.length * 0.8;
        let k = std::f32::consts::FRAC_1_SQRT_2 * r;
        // A point on the Z ring only (the X and Y rings cross it at the axes).
        let a = cam.project(rect, Vec3::new(k, -k, 0.0)).unwrap();
        assert_eq!(g.hit(&cam, rect, Mode::Rotate, a), Some(Handle::Ring(2)));
        let drag = Drag::start(Handle::Ring(2), g, start, &cam, rect, a).unwrap();
        let b = cam.project(rect, Vec3::new(k, k, 0.0)).unwrap();
        let t = drag.update(&cam, rect, b, None).unwrap();
        assert!(
            (t.rotation_deg.z - 90.0).abs() < 0.5,
            "{:?}",
            t.rotation_deg
        );
    }

    #[test]
    fn plane_handle_keeps_height() {
        let (cam, rect) = setup();
        let start = Transform {
            translation: Vec3::new(0.0, 0.0, 3.0),
            ..Default::default()
        };
        let g = Gizmo::new(
            &cam,
            rect,
            Vec3::new(0.0, 0.0, 8.0),
            Vec3::ZERO,
            1.0,
            Mode::Move,
        );
        let sq = g.plane_square(&cam, rect).unwrap();
        let mid = pos2((sq[0].x + sq[2].x) / 2.0, (sq[0].y + sq[2].y) / 2.0);
        assert_eq!(g.hit(&cam, rect, Mode::Move, mid), Some(Handle::Plane));
        let drag = Drag::start(Handle::Plane, g, start, &cam, rect, mid).unwrap();
        let t = drag
            .update(&cam, rect, mid + vec2(80.0, -40.0), None)
            .unwrap();
        assert_eq!(t.translation.z, 3.0);
        assert!(t.translation.truncate().length() > 1.0);
    }
}
