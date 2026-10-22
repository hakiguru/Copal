//! The 3D camera: orbits around a target point, Z is up.

use egui::{Pos2, Rect, pos2};
use glam::{Mat4, Vec3, Vec4};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    /// Turn around the vertical axis, degrees. 0 looks at the front of the plate.
    pub yaw: f32,
    /// Angle above the plate, degrees.
    pub pitch: f32,
    /// Distance from the target in mm.
    pub distance: f32,
    pub target: Vec3,
    /// Vertical field of view, degrees.
    pub fov_y: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            yaw: -28.0,
            pitch: 30.0,
            distance: 380.0,
            target: Vec3::new(0.0, 0.0, 20.0),
            fov_y: 35.0,
        }
    }
}

/// A ray in world space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ray {
    pub origin: Vec3,
    pub dir: Vec3,
}

impl Ray {
    /// Where the ray crosses the horizontal plane at height `z`.
    pub fn hit_plane_z(&self, z: f32) -> Option<Vec3> {
        if self.dir.z.abs() < 1e-6 {
            return None;
        }
        let t = (z - self.origin.z) / self.dir.z;
        (t > 0.0).then(|| self.origin + self.dir * t)
    }

    /// Parameter along the line `p + a·s` of the point closest to this ray.
    pub fn closest_on_line(&self, p: Vec3, a: Vec3) -> Option<f32> {
        // Two-line closest points.
        let w0 = p - self.origin;
        let (aa, ab, bb) = (a.dot(a), a.dot(self.dir), self.dir.dot(self.dir));
        let (ad, bd) = (a.dot(w0), self.dir.dot(w0));
        let denom = aa * bb - ab * ab;
        if denom.abs() < 1e-9 {
            return None;
        }
        Some((ab * bd - bb * ad) / denom)
    }
}

impl Camera {
    pub fn eye(&self) -> Vec3 {
        let (yaw, pitch) = (self.yaw.to_radians(), self.pitch.to_radians());
        self.target
            + self.distance
                * Vec3::new(
                    yaw.sin() * pitch.cos(),
                    -yaw.cos() * pitch.cos(),
                    pitch.sin(),
                )
    }

    /// Unit vector from the eye toward the target.
    pub fn forward(&self) -> Vec3 {
        (self.target - self.eye()).normalize()
    }

    pub fn view(&self) -> Mat4 {
        // Looking straight down needs another up vector.
        let up = if self.pitch.abs() > 89.5 {
            Vec3::Y
        } else {
            Vec3::Z
        };
        glam::camera::rh::view::look_at_mat4(self.eye(), self.target, up)
    }

    pub fn projection(&self, aspect: f32) -> Mat4 {
        let near = (self.distance * 0.01).max(0.5);
        let far = self.distance * 10.0 + 2000.0;
        glam::camera::rh::proj::opengl::perspective(
            self.fov_y.to_radians(),
            aspect.max(0.01),
            near,
            far,
        )
    }

    pub fn view_projection(&self, rect: Rect) -> Mat4 {
        self.projection(rect.width() / rect.height().max(1.0)) * self.view()
    }

    /// Screen position of a world point, or `None` behind the camera.
    pub fn project(&self, rect: Rect, p: Vec3) -> Option<Pos2> {
        let clip = self.view_projection(rect) * Vec4::new(p.x, p.y, p.z, 1.0);
        if clip.w <= 1e-4 {
            return None;
        }
        let ndc = clip.truncate() / clip.w;
        Some(pos2(
            rect.left() + (ndc.x + 1.0) / 2.0 * rect.width(),
            rect.top() + (1.0 - ndc.y) / 2.0 * rect.height(),
        ))
    }

    /// The ray from the eye through a screen point.
    pub fn ray(&self, rect: Rect, pos: Pos2) -> Ray {
        let inv = self.view_projection(rect).inverse();
        let x = (pos.x - rect.left()) / rect.width() * 2.0 - 1.0;
        let y = 1.0 - (pos.y - rect.top()) / rect.height() * 2.0;
        let near = inv * Vec4::new(x, y, -1.0, 1.0);
        let far = inv * Vec4::new(x, y, 1.0, 1.0);
        let near = near.truncate() / near.w;
        let far = far.truncate() / far.w;
        Ray {
            origin: near,
            dir: (far - near).normalize(),
        }
    }

    /// Millimetres per screen point at a world point (for gizmos of constant screen size).
    pub fn mm_per_point(&self, rect: Rect, p: Vec3) -> f32 {
        let depth = (p - self.eye()).dot(self.forward()).max(1.0);
        2.0 * depth * (self.fov_y.to_radians() / 2.0).tan() / rect.height().max(1.0)
    }

    pub fn orbit(&mut self, dx: f32, dy: f32) {
        self.yaw -= dx * 0.4;
        self.pitch = (self.pitch + dy * 0.4).clamp(-89.9, 89.9);
    }

    /// Moves the target with the mouse, in the camera's screen plane.
    pub fn pan(&mut self, rect: Rect, dx: f32, dy: f32) {
        let k = self.mm_per_point(rect, self.target);
        let f = self.forward();
        let right = f.cross(Vec3::Z).try_normalize().unwrap_or(Vec3::X);
        let up = right.cross(f);
        self.target += (-dx * right + dy * up) * k;
    }

    /// Zooms toward the point under the cursor.
    pub fn zoom(&mut self, rect: Rect, pos: Pos2, steps: f32) {
        let factor = (-steps * 0.0015).exp();
        let new_distance = (self.distance * factor).clamp(5.0, 5000.0);
        let ray = self.ray(rect, pos);
        let toward = ray.hit_plane_z(self.target.z).unwrap_or(self.target);
        let k = 1.0 - new_distance / self.distance;
        self.target += (toward - self.target) * k;
        self.distance = new_distance;
    }

    /// Frames a box (plate or objects) so it fills the view.
    pub fn frame(&mut self, min: Vec3, max: Vec3) {
        let center = (min + max) / 2.0;
        let radius = ((max - min).length() / 2.0).max(5.0);
        self.target = center;
        self.distance = radius / (self.fov_y.to_radians() / 2.0).sin() * 1.05;
    }

    pub fn set_view(&mut self, yaw: f32, pitch: f32) {
        self.yaw = yaw;
        self.pitch = pitch;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect() -> Rect {
        Rect::from_min_size(pos2(10.0, 20.0), egui::vec2(800.0, 600.0))
    }

    #[test]
    fn target_projects_to_the_center_and_rays_round_trip() {
        let cam = Camera::default();
        let c = cam.project(rect(), cam.target).unwrap();
        assert!((c.x - 410.0).abs() < 0.5 && (c.y - 320.0).abs() < 0.5);
        let p = Vec3::new(30.0, -10.0, 5.0);
        let s = cam.project(rect(), p).unwrap();
        let ray = cam.ray(rect(), s);
        let t = ray.closest_on_line(p, Vec3::X).unwrap();
        assert!(t.abs() < 0.05, "{t}");
        let hit = ray.hit_plane_z(5.0).unwrap();
        assert!((hit - p).length() < 0.05, "{hit}");
    }

    #[test]
    fn mm_per_point_matches_projection() {
        let cam = Camera::default();
        let k = cam.mm_per_point(rect(), cam.target);
        let a = cam.project(rect(), cam.target).unwrap();
        let f = cam.forward();
        let right = f.cross(Vec3::Z).normalize();
        let b = cam.project(rect(), cam.target + right * 100.0 * k).unwrap();
        assert!(
            ((b - a).length() - 100.0).abs() < 1.0,
            "{}",
            (b - a).length()
        );
    }

    #[test]
    fn top_view_works() {
        let mut cam = Camera::default();
        cam.set_view(0.0, 90.0);
        cam.pitch = cam.pitch.clamp(-89.9, 89.9);
        let a = cam.project(rect(), Vec3::new(0.0, 50.0, 0.0)).unwrap();
        let b = cam.project(rect(), Vec3::new(0.0, -50.0, 0.0)).unwrap();
        assert!(a.y < b.y, "back of the plate is up on the screen");
    }
}
