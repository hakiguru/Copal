//! Small shaded pictures of the scene for file previews and the print history.

use glam::{Mat3, Vec3};

/// Colors of a preview, as RGB.
#[derive(Debug, Clone, Copy)]
pub struct PreviewStyle {
    pub background: [u8; 3],
    pub model: [u8; 3],
}

impl Default for PreviewStyle {
    fn default() -> Self {
        Self {
            background: [24, 26, 30],
            model: [236, 150, 40],
        }
    }
}

/// Renders triangles (plate coordinates, mm) into a `width × height` RGB image, seen from the
/// front, a bit from above, and zoomed to fit.
pub fn render(tris: &[[Vec3; 3]], width: u32, height: u32, style: PreviewStyle) -> Vec<u8> {
    let (w, h) = (width as usize, height as usize);
    let mut rgb: Vec<u8> = style.background.repeat(w * h);
    if tris.is_empty() || w == 0 || h == 0 {
        return rgb;
    }
    // Camera: turn 30° around Z, then tilt 35° so we look down at the model.
    let view =
        Mat3::from_rotation_x(-55f32.to_radians()) * Mat3::from_rotation_z(-30f32.to_radians());
    let (mut lo, mut hi) = (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY));
    for t in tris {
        for v in t {
            let p = view * *v;
            lo = lo.min(p);
            hi = hi.max(p);
        }
    }
    let size = (hi - lo).max(Vec3::splat(1e-3));
    let scale = 0.88 * (w as f32 / size.x).min(h as f32 / size.y);
    let center = (lo + hi) / 2.0;
    let to_screen = |v: Vec3| {
        let p = view * v - center;
        Vec3::new(
            w as f32 / 2.0 + p.x * scale,
            h as f32 / 2.0 - p.y * scale,
            p.z,
        )
    };
    let light = Vec3::new(0.35, -0.5, 0.8).normalize();
    let mut depth = vec![f32::NEG_INFINITY; w * h];
    for t in tris {
        let n = (t[1] - t[0]).cross(t[2] - t[0]);
        if n.length_squared() < 1e-12 {
            continue;
        }
        let n = n.normalize();
        // Two-sided light so open meshes still look solid.
        let shade = 0.25 + 0.75 * n.dot(light).abs();
        let color = style.model.map(|c| (c as f32 * shade).min(255.0) as u8);
        let [a, b, c] = t.map(to_screen);
        let min_x = a.x.min(b.x).min(c.x).floor().max(0.0) as usize;
        let max_x = (a.x.max(b.x).max(c.x).ceil() as usize).min(w - 1);
        let min_y = a.y.min(b.y).min(c.y).floor().max(0.0) as usize;
        let max_y = (a.y.max(b.y).max(c.y).ceil() as usize).min(h - 1);
        let area = (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
        if area.abs() < 1e-9 || min_x > max_x || min_y > max_y {
            continue;
        }
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
                let w0 = ((b.x - px) * (c.y - py) - (b.y - py) * (c.x - px)) / area;
                let w1 = ((c.x - px) * (a.y - py) - (c.y - py) * (a.x - px)) / area;
                let w2 = 1.0 - w0 - w1;
                if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                    continue;
                }
                let z = w0 * a.z + w1 * b.z + w2 * c.z;
                let i = y * w + x;
                if z > depth[i] {
                    depth[i] = z;
                    rgb[i * 3..i * 3 + 3].copy_from_slice(&color);
                }
            }
        }
    }
    rgb
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draws_something_in_the_middle() {
        let tris = vec![[
            Vec3::new(-5.0, 0.0, 0.0),
            Vec3::new(5.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 10.0),
        ]];
        let style = PreviewStyle::default();
        let img = render(&tris, 50, 40, style);
        assert_eq!(img.len(), 50 * 40 * 3);
        let mid = (20 * 50 + 25) * 3;
        assert_ne!(&img[mid..mid + 3], &style.background);
        assert_eq!(&img[0..3], &style.background);
    }
}
