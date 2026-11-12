//! Automatic orientation: which way up a model prints best.
//!
//! A resin print fails when a layer pulls harder on the film than the supports and the plate can
//! hold. So a good orientation keeps every layer small, lets layers grow slowly (no big flat
//! face that appears at once), and leaves little that hangs over. We try many directions for
//! "down" and score each one with numbers that are cheap to get:
//!
//! - the model as a coarse voxel grid gives the area of every layer for any direction,
//! - the faces, grouped by direction, give the area that hangs over.

use copal_slice::{Raster, Slicer};
use glam::{Quat, Vec2, Vec3};
use rayon::prelude::*;

/// What matters most when choosing an orientation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Goal {
    /// Small layers, slow growth, few supports, not too tall.
    #[default]
    Balanced,
    /// The smallest layers: the least pull on the film and the plate.
    Strong,
    /// The least area that hangs over.
    FewSupports,
    /// The lowest print.
    Fast,
}

impl Goal {
    pub const ALL: [Goal; 4] = [Goal::Balanced, Goal::Strong, Goal::FewSupports, Goal::Fast];

    pub fn parse(s: &str) -> Option<Goal> {
        match s {
            "balanced" | "auto" => Some(Goal::Balanced),
            "strong" | "safe" => Some(Goal::Strong),
            "supports" | "few-supports" => Some(Goal::FewSupports),
            "fast" | "low" => Some(Goal::Fast),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Goal::Balanced => "balanced",
            Goal::Strong => "strong",
            Goal::FewSupports => "supports",
            Goal::Fast => "fast",
        }
    }

    /// Weights of (biggest layer, growth, overhang, height).
    fn weights(self) -> [f32; 4] {
        match self {
            Goal::Balanced => [1.0, 1.0, 6.0, 0.4],
            Goal::Strong => [1.5, 1.5, 1.0, 0.0],
            Goal::FewSupports => [0.3, 0.5, 9.0, 0.2],
            Goal::Fast => [0.4, 0.5, 2.0, 3.0],
        }
    }
}

/// Numbers that describe one orientation.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct OrientStats {
    /// Height of the print, mm.
    pub height: f32,
    /// Area of the biggest layer, mm².
    pub peak_area: f32,
    /// The most a layer grows over one millimetre of height, mm². A flat face that looks down
    /// shows up here.
    pub growth: f32,
    /// Area that hangs over more than the overhang angle, mm².
    pub overhang_area: f32,
    /// The overhang area with flatter faces counted more: they need the most supports and show
    /// the most marks. Level faces count in full, faces at the overhang angle a quarter.
    pub support_load: f32,
}

/// A way to turn the model.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Oriented {
    /// Turns the model as given into the print orientation.
    pub rotation: Quat,
    pub stats: OrientStats,
    /// Lower is better. Only comparable for the same model and goal.
    pub score: f32,
}

/// A model prepared for trying orientations.
pub struct Analyzer {
    /// Centers of the voxels inside the model.
    voxels: Vec<Vec3>,
    voxel: f32,
    /// Faces grouped by direction: (unit normal, area).
    normals: Vec<(Vec3, f32)>,
    surface: f32,
    cos_limit: f32,
}

/// Voxels along the longest side of the model.
const GRID: f32 = 44.0;

impl Analyzer {
    /// `tris` is the model at its real size. `cos_limit` is the cosine of the overhang angle.
    pub fn new(tris: &[[Vec3; 3]], cos_limit: f32) -> Option<Analyzer> {
        let (mut lo, mut hi) = (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY));
        for t in tris {
            for p in t {
                lo = lo.min(*p);
                hi = hi.max(*p);
            }
        }
        if !lo.is_finite() {
            return None;
        }
        let size = hi - lo;
        let v = (size.max_element() / GRID).max(1e-3);
        let center = (lo + hi) / 2.0;
        let shift = Vec3::new(-center.x, -center.y, -lo.z);
        let moved: Vec<[Vec3; 3]> = tris.iter().map(|t| t.map(|p| p + shift)).collect();
        let nx = (size.x / v).ceil() as u32 + 2;
        let ny = (size.y / v).ceil() as u32 + 2;
        let raster = Raster::new([nx, ny], [nx as f32 * v, ny as f32 * v]);
        let slicer = Slicer::new(moved, raster, v, 1);
        let voxels: Vec<Vec3> = (0..slicer.layer_count())
            .into_par_iter()
            .flat_map_iter(|k| {
                let (img, _) = slicer.layer_image(k);
                let z = (k as f32 + 0.5) * v;
                img.into_iter()
                    .enumerate()
                    .filter(|(_, px)| *px >= 128)
                    .map(move |(i, _)| {
                        let (c, r) = (i as u32 % nx, i as u32 / nx);
                        Vec3::new(
                            (c as f32 + 0.5 - nx as f32 / 2.0) * v,
                            (ny as f32 / 2.0 - r as f32 - 0.5) * v,
                            z,
                        ) - shift
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
        if voxels.is_empty() {
            return None;
        }
        // Faces by direction, so a model with millions of triangles costs the same per try.
        let mut bins: std::collections::HashMap<[i8; 3], (Vec3, f32)> = Default::default();
        let mut surface = 0.0;
        for t in tris {
            let e = (t[1] - t[0]).cross(t[2] - t[0]);
            let len = e.length();
            if len < 1e-9 {
                continue;
            }
            let n = e / len;
            let key = [
                (n.x * 20.0).round() as i8,
                (n.y * 20.0).round() as i8,
                (n.z * 20.0).round() as i8,
            ];
            let slot = bins.entry(key).or_insert((Vec3::ZERO, 0.0));
            slot.0 += e / 2.0;
            slot.1 += len / 2.0;
            surface += len / 2.0;
        }
        let mut normals: Vec<(Vec3, f32)> = bins
            .into_values()
            .filter_map(|(sum, area)| sum.try_normalize().map(|n| (n, area)))
            .collect();
        // A fixed order, so the same model always gets the same answer.
        normals.sort_by(|a, b| {
            a.0.x
                .total_cmp(&b.0.x)
                .then(a.0.y.total_cmp(&b.0.y))
                .then(a.0.z.total_cmp(&b.0.z))
        });
        Some(Analyzer {
            voxels,
            voxel: v,
            normals,
            surface,
            cos_limit,
        })
    }

    /// The numbers for printing with the model direction `down` toward the plate.
    pub fn stats(&self, down: Vec3) -> OrientStats {
        let down = down.normalize_or(Vec3::NEG_Z);
        let v = self.voxel;
        let (mut lo, mut hi) = (f32::INFINITY, f32::NEG_INFINITY);
        for p in &self.voxels {
            let h = -down.dot(*p);
            lo = lo.min(h);
            hi = hi.max(h);
        }
        let bins = ((hi - lo) / v).floor() as usize + 1;
        let mut count = vec![0f32; bins + 2];
        for p in &self.voxels {
            let h = (-down.dot(*p) - lo) / v;
            // Shared between two neighbouring layers, so tilted cuts come out smooth.
            let k = h.floor();
            let f = h - k;
            count[k as usize] += 1.0 - f;
            count[k as usize + 1] += f;
        }
        let area: Vec<f32> = count.iter().map(|c| c * v * v).collect();
        let peak_area = area.iter().copied().fold(0.0, f32::max);
        // Growth over one millimetre (at least two layers of voxels).
        let step = ((1.0 / v).ceil() as usize).max(2);
        let mut growth = 0f32;
        for k in 0..area.len() {
            let before = if k >= step { area[k - step] } else { 0.0 };
            growth = growth.max((area[k] - before) * (1.0 / (step as f32 * v)).min(1.0));
        }
        let (mut overhang_area, mut support_load) = (0.0, 0.0);
        // Faces a little steeper than the overhang angle (up to 15°) still count a bit: they
        // print without supports only just, and a model balanced on them is not held well.
        let soft = (self.cos_limit.clamp(-1.0, 1.0).acos() + 15f32.to_radians())
            .min(std::f32::consts::FRAC_PI_2)
            .cos();
        for (n, a) in &self.normals {
            let c = n.dot(down);
            if c > self.cos_limit {
                overhang_area += a;
                let flat = (c - self.cos_limit) / (1.0 - self.cos_limit).max(1e-6);
                support_load += a * (0.25 + 0.75 * flat * flat);
            } else if c > soft {
                support_load += a * 0.25 * (c - soft) / (self.cos_limit - soft).max(1e-6);
            }
        }
        OrientStats {
            height: hi - lo + v,
            peak_area,
            growth,
            overhang_area,
            support_load,
        }
    }

    /// The best way to turn the model for `goal`.
    ///
    /// With `volume` (the printer's build volume), orientations that do not fit are left out and
    /// the model is turned so its long side lies along the long side of the plate.
    pub fn best(&self, goal: Goal, volume: Option<Vec3>) -> Option<Oriented> {
        // Directions spread evenly over the sphere, plus the six axes.
        const N: usize = 700;
        let mut dirs: Vec<Vec3> = (0..N)
            .map(|i| {
                let z = 1.0 - 2.0 * (i as f32 + 0.5) / N as f32;
                let r = (1.0 - z * z).max(0.0).sqrt();
                let a = i as f32 * 2.399_963_2;
                Vec3::new(r * a.cos(), r * a.sin(), z)
            })
            .collect();
        dirs.extend([
            Vec3::X,
            Vec3::Y,
            Vec3::Z,
            Vec3::NEG_X,
            Vec3::NEG_Y,
            Vec3::NEG_Z,
        ]);
        let all: Vec<(Vec3, OrientStats)> = dirs.par_iter().map(|d| (*d, self.stats(*d))).collect();
        let min_peak = all
            .iter()
            .map(|(_, s)| s.peak_area)
            .fold(f32::INFINITY, f32::min)
            .max(1e-6);
        let max_height = all
            .iter()
            .map(|(_, s)| s.height)
            .fold(0.0, f32::max)
            .max(1e-6);
        let w = goal.weights();
        let score = |s: &OrientStats| {
            w[0] * s.peak_area / min_peak
                + w[1] * s.growth / min_peak
                + w[2] * s.support_load / self.surface.max(1e-6)
                + w[3] * s.height / max_height
        };
        let mut ranked: Vec<(f32, Vec3, OrientStats)> =
            all.into_iter().map(|(d, s)| (score(&s), d, s)).collect();
        ranked.sort_by(|a, b| a.0.total_cmp(&b.0));
        for (score, down, stats) in ranked {
            let tilt = Quat::from_rotation_arc(down, Vec3::NEG_Z);
            if let Some(v) = volume
                && stats.height > v.z
            {
                continue;
            }
            let Some(yaw) = self.yaw_for(tilt, volume) else {
                continue;
            };
            return Some(Oriented {
                rotation: Quat::from_rotation_z(yaw) * tilt,
                stats,
                score,
            });
        }
        None
    }

    /// Turn around the vertical axis so the long side lies along X; `None` when the model does
    /// not fit on the plate any way round.
    fn yaw_for(&self, tilt: Quat, volume: Option<Vec3>) -> Option<f32> {
        let pts: Vec<Vec2> = self
            .voxels
            .iter()
            .step_by((self.voxels.len() / 4000).max(1))
            .map(|p| (tilt * *p).truncate())
            .collect();
        let mut best: Option<(f32, f32)> = None;
        for k in 0..36 {
            let yaw = (k as f32 * 5.0).to_radians();
            let rot = Vec2::from_angle(yaw);
            let (mut lo, mut hi) = (Vec2::splat(f32::INFINITY), Vec2::splat(f32::NEG_INFINITY));
            for p in &pts {
                let q = rot.rotate(*p);
                lo = lo.min(q);
                hi = hi.max(q);
            }
            let size = hi - lo + Vec2::splat(self.voxel);
            if let Some(v) = volume
                && (size.x > v.x || size.y > v.y)
            {
                continue;
            }
            // The narrowest way across Y; among equals, the smallest turn.
            if best.is_none_or(|(y, _)| size.y < y - 1e-3) {
                best = Some((size.y, yaw));
            }
        }
        best.map(|(_, yaw)| yaw)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use copal_mesh::Mesh;
    use copal_mesh::bvh::Bvh;
    use glam::Mat4;

    fn slab() -> Vec<[Vec3; 3]> {
        // A flat brick, 60 × 40 × 10 mm.
        let m = Mesh::cuboid(Vec3::new(60.0, 40.0, 10.0));
        Bvh::from_mesh(&m, &Mat4::IDENTITY).triangles().to_vec()
    }

    #[test]
    fn numbers_of_a_brick_lying_flat_and_standing() {
        let a = Analyzer::new(&slab(), 45f32.to_radians().cos()).unwrap();
        let flat = a.stats(Vec3::NEG_Z);
        assert!((flat.height - 10.0).abs() < 2.0, "{flat:?}");
        assert!((flat.peak_area - 2400.0).abs() < 250.0, "{flat:?}");
        assert!((flat.overhang_area - 2400.0).abs() < 1.0, "{flat:?}");
        // The whole face appears at once.
        assert!(flat.growth > 700.0, "{flat:?}");
        let up = a.stats(Vec3::NEG_X);
        assert!((up.height - 60.0).abs() < 3.0, "{up:?}");
        assert!((up.peak_area - 400.0).abs() < 60.0, "{up:?}");
        assert!((up.overhang_area - 400.0).abs() < 1.0, "{up:?}");
    }

    #[test]
    fn the_best_orientation_is_tilted_with_small_layers() {
        let a = Analyzer::new(&slab(), 45f32.to_radians().cos()).unwrap();
        let flat = a.stats(Vec3::NEG_Z);
        let best = a.best(Goal::Balanced, None).unwrap();
        assert!(best.stats.peak_area < flat.peak_area * 0.5, "{best:?}");
        assert!(best.stats.growth < flat.growth * 0.3, "{best:?}");
        // No face of the brick is level with the plate.
        for axis in [Vec3::X, Vec3::Y, Vec3::Z] {
            let n = best.rotation * axis;
            assert!(n.z.abs() < 0.97, "{axis} is level: {n}");
        }
        // The same model, the same answer.
        assert_eq!(a.best(Goal::Balanced, None), Some(best));
        // The fast goal keeps the print low.
        let fast = a.best(Goal::Fast, None).unwrap();
        assert!(fast.stats.height < best.stats.height, "{fast:?} {best:?}");
    }

    #[test]
    fn orientations_that_do_not_fit_are_left_out() {
        let a = Analyzer::new(&slab(), 45f32.to_radians().cos()).unwrap();
        let v = Vec3::new(70.0, 45.0, 30.0);
        let best = a.best(Goal::Strong, Some(v)).unwrap();
        assert!(best.stats.height <= 30.0, "{best:?}");
        assert!(a.best(Goal::Strong, Some(Vec3::splat(20.0))).is_none());
    }
}
