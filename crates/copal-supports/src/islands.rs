//! Islands: parts of a layer that are not connected to anything in the layer below. Each one
//! would print in mid-air and needs a support.

use copal_mesh::bvh::Bvh;
use copal_slice::{Raster, Slicer};
use glam::{Vec2, Vec3};
use rayon::prelude::*;

/// Where an island starts, with the points that should get supports.
#[derive(Debug, Clone, PartialEq)]
pub struct Island {
    /// Height of the layer where the island appears.
    pub z: f32,
    /// Area in mm² in its first layer.
    pub area: f32,
    /// Outline box in XY (min, max).
    pub min: Vec2,
    pub max: Vec2,
    /// Contact points on the model with their surface normals (at least one).
    pub contacts: Vec<(Vec3, Vec3)>,
}

/// Finds islands in triangles given in world space.
///
/// `layer` is the step between test layers, `pixel` the size of test pixels, `spacing` the
/// distance between contact points on big islands.
pub fn find_islands(bvh: &Bvh, layer: f32, pixel: f32, spacing: f32) -> Vec<Island> {
    let Some((lo, hi)) = bvh.bounds() else {
        return Vec::new();
    };
    let center = (lo + hi) / 2.0;
    let shift = Vec3::new(-center.x, -center.y, -lo.z);
    let tris: Vec<[Vec3; 3]> = bvh
        .triangles()
        .iter()
        .map(|t| t.map(|v| v + shift))
        .collect();
    let size = hi - lo;
    let w = (size.x / pixel).ceil() as u32 + 6;
    let h = (size.y / pixel).ceil() as u32 + 6;
    let raster = Raster::new([w, h], [w as f32 * pixel, h as f32 * pixel]);
    let slicer = Slicer::new(tris, raster, layer, 1);
    let layers = slicer.layer_count();
    let (wu, hu) = (w as usize, h as usize);
    // Pixel (col, row) back to world XY.
    let to_world = |c: usize, r: usize| {
        Vec2::new(
            (c as f32 + 0.5 - w as f32 / 2.0) * pixel - shift.x,
            (h as f32 / 2.0 - r as f32 - 0.5) * pixel - shift.y,
        )
    };
    let step = ((spacing / pixel).round() as usize).max(2);

    let mut out = Vec::new();
    let mut prev: Vec<bool> = vec![false; wu * hu];
    let batch = (rayon::current_num_threads() * 4).max(8) as u32;
    let mut labels = vec![0u32; wu * hu];
    let mut start = 0;
    while start < layers {
        let end = (start + batch).min(layers);
        let images: Vec<Vec<bool>> = (start..end)
            .into_par_iter()
            .map(|i| slicer.layer_image(i).0.iter().map(|v| *v >= 128).collect())
            .collect();
        for (k, cur) in images.into_iter().enumerate() {
            let i = start + k as u32;
            let z = lo.z + i as f32 * layer;
            labels.fill(0);
            let mut next_label = 0u32;
            let mut queue = Vec::new();
            for seed in 0..cur.len() {
                if !cur[seed] || labels[seed] != 0 {
                    continue;
                }
                next_label += 1;
                labels[seed] = next_label;
                queue.clear();
                queue.push(seed);
                let mut pixels = Vec::new();
                let mut supported = false;
                while let Some(p) = queue.pop() {
                    pixels.push(p);
                    supported |= prev[p];
                    let (c, r) = (p % wu, p / wu);
                    let mut visit = |q: usize| {
                        if cur[q] && labels[q] == 0 {
                            labels[q] = next_label;
                            queue.push(q);
                        }
                    };
                    if c > 0 {
                        visit(p - 1);
                    }
                    if c + 1 < wu {
                        visit(p + 1);
                    }
                    if r > 0 {
                        visit(p - wu);
                    }
                    if r + 1 < hu {
                        visit(p + wu);
                    }
                }
                if supported {
                    continue;
                }
                out.push(island_of(
                    bvh, &pixels, wu, z, layer, pixel, step, &to_world,
                ));
            }
            prev = cur;
        }
        start = end;
    }
    out.retain(|i| !i.contacts.is_empty());
    out
}

#[allow(clippy::too_many_arguments)]
fn island_of(
    bvh: &Bvh,
    pixels: &[usize],
    wu: usize,
    z: f32,
    layer: f32,
    pixel: f32,
    step: usize,
    to_world: &dyn Fn(usize, usize) -> Vec2,
) -> Island {
    let pts: Vec<(usize, usize)> = pixels.iter().map(|p| (p % wu, p / wu)).collect();
    let n = pts.len() as f32;
    let (sc, sr) = pts
        .iter()
        .fold((0.0, 0.0), |(a, b), (c, r)| (a + *c as f32, b + *r as f32));
    let (cc, cr) = (sc / n, sr / n);
    let mut by_center = pts.clone();
    by_center.sort_by(|a, b| {
        let da = (a.0 as f32 - cc).powi(2) + (a.1 as f32 - cr).powi(2);
        let db = (b.0 as f32 - cc).powi(2) + (b.1 as f32 - cr).powi(2);
        da.total_cmp(&db)
    });
    let mut wanted: Vec<(usize, usize)> = Vec::new();
    // Big islands get a grid of points, every island gets its middle.
    if n * pixel * pixel > (step as f32 * pixel).powi(2) * 1.5 {
        wanted.extend(pts.iter().filter(|(c, r)| c % step == 0 && r % step == 0));
    }
    let mut contacts = Vec::new();
    let probe = |c: usize, r: usize| -> Option<(Vec3, Vec3)> {
        let xy = to_world(c, r);
        let origin = xy.extend(z - layer * 2.0 - 0.01);
        let hit = bvh.raycast(origin, Vec3::Z, 0.0, layer * 8.0 + 0.2)?;
        (hit.normal.z < 0.05).then_some((hit.point, hit.normal))
    };
    // The middle: the first pixel near the centre where the ray finds the bottom surface.
    if let Some(c) = by_center.iter().take(200).find_map(|&(c, r)| probe(c, r)) {
        contacts.push(c);
    }
    for (c, r) in wanted {
        if let Some(p) = probe(c, r)
            && contacts
                .iter()
                .all(|(q, _)| q.distance(p.0) > pixel * step as f32 * 0.6)
        {
            contacts.push(p);
        }
    }
    let (mut min, mut max) = (Vec2::splat(f32::INFINITY), Vec2::splat(f32::NEG_INFINITY));
    for &(c, r) in &pts {
        let p = to_world(c, r);
        min = min.min(p);
        max = max.max(p);
    }
    Island {
        z,
        area: n * pixel * pixel,
        min,
        max,
        contacts,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use copal_mesh::Mesh;
    use glam::Mat4;

    #[test]
    fn finds_the_bottom_of_two_floating_boxes() {
        // Box A from z=5, box B from z=8 next to it.
        let mut m = Mesh::default();
        let mut add = |min: Vec3, max: Vec3| {
            let mut c = Mesh::cuboid(max - min);
            let off = Vec3::new((min.x + max.x) / 2.0, (min.y + max.y) / 2.0, min.z);
            for p in &mut c.positions {
                *p += off;
            }
            m.append(&c);
        };
        add(Vec3::new(0.0, 0.0, 5.0), Vec3::new(4.0, 4.0, 10.0));
        add(Vec3::new(10.0, 0.0, 8.0), Vec3::new(12.0, 2.0, 10.0));
        let bvh = Bvh::from_mesh(&m, &Mat4::IDENTITY);
        let islands = find_islands(&bvh, 0.1, 0.1, 2.0);
        assert_eq!(islands.len(), 2, "{islands:?}");
        assert!((islands[0].z - 5.0).abs() < 0.11);
        assert!((islands[1].z - 8.0).abs() < 0.11);
        for i in &islands {
            for (p, n) in &i.contacts {
                assert!((p.z - i.z).abs() < 0.2, "{p}");
                assert!(n.z < -0.9);
            }
        }
        // The big bottom gets several points.
        assert!(islands[0].contacts.len() >= 4, "{:?}", islands[0].contacts);
    }
}
