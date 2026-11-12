//! Hollowing and drain holes.
//!
//! The model is turned into voxels (test layers from the slicer), a distance transform gives each
//! inside voxel its distance to the outside, and the surface at `wall thickness` is extracted
//! with surface nets. That surface is stored inside out, so the slicer cuts it away. Drain holes
//! are inside-out cylinders too.
//!
//! Small closed pockets are filled again: they would only trap resin. When a model is too big
//! for the voxel size asked for, bigger voxels are used instead of giving up.

use copal_mesh::Mesh;
use copal_mesh::bvh::Bvh;
use copal_slice::{Raster, Slicer};
use glam::Vec3;
use rayon::prelude::*;

use crate::geometry::frustum;

const FAR: f32 = 1e20;

/// 1D squared Euclidean distance transform (Felzenszwalb & Huttenlocher).
fn edt_1d(f: &mut [f32], v: &mut Vec<usize>, z: &mut Vec<f32>, out: &mut Vec<f32>) {
    let n = f.len();
    v.clear();
    v.resize(n, 0);
    z.clear();
    z.resize(n + 1, 0.0);
    out.clear();
    out.resize(n, 0.0);
    let mut k = 0usize;
    v[0] = 0;
    z[0] = -FAR;
    z[1] = FAR;
    for q in 1..n {
        loop {
            let p = v[k];
            let s = ((f[q] + (q * q) as f32) - (f[p] + (p * p) as f32))
                / (2.0 * q as f32 - 2.0 * p as f32);
            if s <= z[k] && k > 0 {
                k -= 1;
                continue;
            }
            if s <= z[k] {
                // k == 0: replace the first parabola.
                v[0] = q;
                z[0] = -FAR;
                z[1] = FAR;
                break;
            }
            k += 1;
            v[k] = q;
            z[k] = s;
            z[k + 1] = FAR;
            break;
        }
    }
    k = 0;
    for (q, o) in out.iter_mut().enumerate() {
        while z[k + 1] < q as f32 {
            k += 1;
        }
        let p = v[k];
        let d = q as f32 - p as f32;
        *o = d * d + f[p];
    }
    f.copy_from_slice(out);
}

/// The biggest voxel grid hollowing works on (about 200 MB of memory).
pub const MAX_VOXELS: usize = 48_000_000;

/// Empty voxels around the model.
const PAD: usize = 2;

fn grid_cells(size: Vec3, v: f32) -> (usize, usize, usize) {
    (
        (size.x / v).ceil() as usize + 2 * PAD,
        (size.y / v).ceil() as usize + 2 * PAD,
        (size.z / v).ceil() as usize + 2 * PAD,
    )
}

/// The voxel size hollowing uses for a model of this size: `voxel`, or bigger when the grid
/// would not fit in memory.
pub fn fit_voxel(size: Vec3, voxel: f32) -> f32 {
    let mut v = voxel.max(0.05);
    for _ in 0..60 {
        let (x, y, z) = grid_cells(size, v);
        if x.saturating_mul(y).saturating_mul(z) <= MAX_VOXELS {
            break;
        }
        v *= 1.1;
    }
    v
}

/// Why a model could not be hollowed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HollowError {
    /// There are no triangles.
    Empty,
    /// No part of the model is thicker than two walls.
    TooThin,
}

impl std::fmt::Display for HollowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            HollowError::Empty => "the model is empty",
            HollowError::TooThin => "the model is too thin for walls this thick",
        })
    }
}

/// The result of hollowing.
#[derive(Debug, Clone, PartialEq)]
pub struct Hollowed {
    /// The inner surface, inside out, in the same space as the triangles given.
    pub mesh: Mesh,
    /// The voxel size that was used (bigger than asked for on very big models).
    pub voxel: f32,
    /// Volume of the hollow in mm³.
    pub volume: f32,
    /// Separate hollows that were kept.
    pub cavities: usize,
    /// Small closed pockets that were filled again.
    pub pockets_filled: usize,
}

/// The inner surface of a hollowed model, inside out, in the same space as `tris`.
/// `None` when the model is too thin to hollow. Pockets are kept; see [`hollow_detailed`].
pub fn hollow(tris: &[[Vec3; 3]], thickness: f32, voxel: f32) -> Option<Mesh> {
    hollow_detailed(tris, thickness, voxel, 0.0)
        .ok()
        .map(|h| h.mesh)
}

/// Hollows a model: keeps walls of `thickness` and returns the inner surface, inside out.
///
/// `voxel` is the size of the test voxels; it grows by itself when the model is too big for it.
/// Closed hollows smaller than `min_pocket` mm³ are filled again (0 keeps them all).
pub fn hollow_detailed(
    tris: &[[Vec3; 3]],
    thickness: f32,
    voxel: f32,
    min_pocket: f32,
) -> Result<Hollowed, HollowError> {
    let (mut lo, mut hi) = (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY));
    for t in tris {
        for p in t {
            lo = lo.min(*p);
            hi = hi.max(*p);
        }
    }
    if !lo.is_finite() {
        return Err(HollowError::Empty);
    }
    let pad = PAD;
    let size = hi - lo;
    let v = fit_voxel(size, voxel);
    let (nx, ny, nz) = grid_cells(size, v);
    let center = (lo + hi) / 2.0;
    let z0 = lo.z - pad as f32 * v;
    let shift = Vec3::new(-center.x, -center.y, -z0);
    let moved: Vec<[Vec3; 3]> = tris.iter().map(|t| t.map(|p| p + shift)).collect();
    let raster = Raster::new([nx as u32, ny as u32], [nx as f32 * v, ny as f32 * v]);
    let slicer = Slicer::new(moved, raster, v, 1);
    let plane = nx * ny;
    let idx = |c: usize, r: usize, k: usize| k * plane + r * nx + c;

    // Squared distance (in voxels) from each inside voxel to the outside.
    let mut d: Vec<f32> = vec![0.0; plane * nz];
    d.par_chunks_mut(plane).enumerate().for_each(|(k, layer)| {
        let (img, _) = slicer.layer_image(k as u32);
        for (o, px) in layer.iter_mut().zip(img) {
            *o = if px >= 128 { FAR } else { 0.0 };
        }
    });
    // Along X (rows), then Y (columns of each layer), then Z.
    d.par_chunks_mut(nx).for_each(|row| {
        let (mut a, mut b, mut c) = (Vec::new(), Vec::new(), Vec::new());
        edt_1d(row, &mut a, &mut b, &mut c);
    });
    d.par_chunks_mut(plane).for_each(|layer| {
        let (mut a, mut b, mut c) = (Vec::new(), Vec::new(), Vec::new());
        let mut col = vec![0.0; ny];
        for x in 0..nx {
            for y in 0..ny {
                col[y] = layer[y * nx + x];
            }
            edt_1d(&mut col, &mut a, &mut b, &mut c);
            for y in 0..ny {
                layer[y * nx + x] = col[y];
            }
        }
    });
    {
        // Along Z, one column at a time with shared buffers (no extra copy of the grid).
        let (mut a, mut b, mut c) = (Vec::new(), Vec::new(), Vec::new());
        let mut col = vec![0.0f32; nz];
        for i in 0..plane {
            for (k, slot) in col.iter_mut().enumerate() {
                *slot = d[k * plane + i];
            }
            edt_1d(&mut col, &mut a, &mut b, &mut c);
            for (k, val) in col.iter().enumerate() {
                d[k * plane + i] = *val;
            }
        }
    }
    // Field: > 0 deeper than the wall. Written over the distances to save memory.
    d.par_iter_mut().for_each(|s| {
        *s = if *s <= 0.0 {
            -v
        } else {
            s.sqrt() * v - 0.5 * v - thickness
        };
    });
    let mut g = d;
    if !g.par_iter().any(|x| *x > 0.0) {
        return Err(HollowError::TooThin);
    }
    // Separate hollows: fill the small ones again, they would only trap resin.
    let voxel_volume = v * v * v;
    let mut seen = vec![false; g.len()];
    let mut stack: Vec<usize> = Vec::new();
    let mut members: Vec<usize> = Vec::new();
    let (mut cavities, mut pockets_filled, mut volume) = (0usize, 0usize, 0f32);
    for seed in 0..g.len() {
        if seen[seed] || g[seed] <= 0.0 {
            continue;
        }
        seen[seed] = true;
        stack.push(seed);
        members.clear();
        while let Some(p) = stack.pop() {
            members.push(p);
            let (c, r, k) = (p % nx, (p / nx) % ny, p / plane);
            let mut visit = |q: usize| {
                if !seen[q] && g[q] > 0.0 {
                    seen[q] = true;
                    stack.push(q);
                }
            };
            if c > 0 {
                visit(p - 1);
            }
            if c + 1 < nx {
                visit(p + 1);
            }
            if r > 0 {
                visit(p - nx);
            }
            if r + 1 < ny {
                visit(p + nx);
            }
            if k > 0 {
                visit(p - plane);
            }
            if k + 1 < nz {
                visit(p + plane);
            }
        }
        let size = members.len() as f32 * voxel_volume;
        if size < min_pocket {
            pockets_filled += 1;
            for m in &members {
                g[*m] = -v;
            }
        } else {
            cavities += 1;
            volume += size;
        }
    }
    drop(seen);
    if cavities == 0 {
        return Err(HollowError::TooThin);
    }
    let world = |c: f32, r: f32, k: f32| {
        Vec3::new(
            (c + 0.5 - nx as f32 / 2.0) * v,
            (ny as f32 / 2.0 - r - 0.5) * v,
            (k + 0.5) * v,
        ) - shift
    };

    // Surface nets: one vertex per cell with a sign change.
    let cells = |n: usize| n.saturating_sub(1);
    let (cx, cy, cz) = (cells(nx), cells(ny), cells(nz));
    let cidx = |c: usize, r: usize, k: usize| (k * cy + r) * cx + c;
    let mut vert_of = vec![u32::MAX; cx * cy * cz];
    let mut mesh = Mesh::default();
    const EDGES: [(usize, usize); 12] = [
        (0, 1),
        (2, 3),
        (4, 5),
        (6, 7),
        (0, 2),
        (1, 3),
        (4, 6),
        (5, 7),
        (0, 4),
        (1, 5),
        (2, 6),
        (3, 7),
    ];
    for k in 0..cz {
        for r in 0..cy {
            for c in 0..cx {
                let corner = |i: usize| (c + (i & 1), r + ((i >> 1) & 1), k + ((i >> 2) & 1));
                let vals: [f32; 8] = std::array::from_fn(|i| {
                    let (a, b, e) = corner(i);
                    g[idx(a, b, e)]
                });
                let pos = vals.iter().filter(|x| **x > 0.0).count();
                if pos == 0 || pos == 8 {
                    continue;
                }
                let mut sum = Vec3::ZERO;
                let mut n = 0.0;
                for (a, b) in EDGES {
                    let (va, vb) = (vals[a], vals[b]);
                    if (va > 0.0) != (vb > 0.0) {
                        let t = va / (va - vb);
                        let (pa, pb) = (corner(a), corner(b));
                        let p = Vec3::new(pa.0 as f32, pa.1 as f32, pa.2 as f32)
                            + (Vec3::new(pb.0 as f32, pb.1 as f32, pb.2 as f32)
                                - Vec3::new(pa.0 as f32, pa.1 as f32, pa.2 as f32))
                                * t;
                        sum += p;
                        n += 1.0;
                    }
                }
                let p = sum / n;
                vert_of[cidx(c, r, k)] = mesh.positions.len() as u32;
                mesh.positions.push(world(p.x, p.y, p.z));
            }
        }
    }
    // One quad per voxel edge with a sign change.
    let dims = [nx, ny, nz];
    for k in 0..nz {
        for r in 0..ny {
            for c in 0..nx {
                let p = [c, r, k];
                for axis in 0..3 {
                    let mut q = p;
                    q[axis] += 1;
                    if q[axis] >= dims[axis] {
                        continue;
                    }
                    let (g0, g1) = (g[idx(p[0], p[1], p[2])], g[idx(q[0], q[1], q[2])]);
                    if (g0 > 0.0) == (g1 > 0.0) {
                        continue;
                    }
                    let (b, e) = ((axis + 1) % 3, (axis + 2) % 3);
                    let mut quad = [u32::MAX; 4];
                    let mut ok = true;
                    for (slot, (db, de)) in [(0, 0), (1, 0), (1, 1), (0, 1)].into_iter().enumerate()
                    {
                        let mut cell = p;
                        if cell[b] < db || cell[e] < de {
                            ok = false;
                            break;
                        }
                        cell[b] -= db;
                        cell[e] -= de;
                        if cell[0] >= cx || cell[1] >= cy || cell[2] >= cz {
                            ok = false;
                            break;
                        }
                        let vi = vert_of[cidx(cell[0], cell[1], cell[2])];
                        if vi == u32::MAX {
                            ok = false;
                            break;
                        }
                        quad[slot] = vi;
                    }
                    if !ok {
                        continue;
                    }
                    // Normals point into the hollow (toward g > 0).
                    let wp = world(p[0] as f32, p[1] as f32, p[2] as f32);
                    let wq = world(q[0] as f32, q[1] as f32, q[2] as f32);
                    let want = if g1 > 0.0 { wq - wp } else { wp - wq };
                    let [a, b2, c2, d2] = quad.map(|i| mesh.positions[i as usize]);
                    let n = (b2 - a).cross(c2 - a) + (c2 - a).cross(d2 - a);
                    if n.dot(want) >= 0.0 {
                        mesh.triangles.push([quad[0], quad[1], quad[2]]);
                        mesh.triangles.push([quad[0], quad[2], quad[3]]);
                    } else {
                        mesh.triangles.push([quad[0], quad[2], quad[1]]);
                        mesh.triangles.push([quad[0], quad[3], quad[2]]);
                    }
                }
            }
        }
    }
    smooth(&mut mesh, 2);
    Ok(Hollowed {
        mesh,
        voxel: v,
        volume,
        cavities,
        pockets_filled,
    })
}

/// A few rounds of Laplacian smoothing.
fn smooth(mesh: &mut Mesh, rounds: usize) {
    let n = mesh.positions.len();
    let mut neigh: Vec<Vec<u32>> = vec![Vec::new(); n];
    for t in &mesh.triangles {
        for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            if !neigh[a as usize].contains(&b) {
                neigh[a as usize].push(b);
            }
            if !neigh[b as usize].contains(&a) {
                neigh[b as usize].push(a);
            }
        }
    }
    for _ in 0..rounds {
        let old = mesh.positions.clone();
        for (i, p) in mesh.positions.iter_mut().enumerate() {
            let nb = &neigh[i];
            if nb.is_empty() {
                continue;
            }
            let avg = nb.iter().map(|j| old[*j as usize]).sum::<Vec3>() / nb.len() as f32;
            *p = old[i] * 0.5 + avg * 0.5;
        }
    }
}

/// A drain hole: an inside-out cylinder from just outside the surface `depth` mm into the model.
/// `dir` points into the model.
pub fn drain_hole(point: Vec3, dir: Vec3, diameter: f32, depth: f32) -> Mesh {
    let mut m = Mesh::default();
    let d = dir.normalize_or(Vec3::NEG_Z);
    frustum(
        &mut m,
        point - d * 1.0,
        diameter / 2.0,
        point + d * depth,
        diameter / 2.0,
        20,
    );
    for t in &mut m.triangles {
        t.swap(1, 2);
    }
    m
}

/// The separate hollows of a hollowed model: for each one the numbers of its vertices, the
/// biggest hollow first. Each hollow needs its own drain holes.
pub fn cavity_parts(mesh: &Mesh) -> Vec<Vec<u32>> {
    let n = mesh.positions.len();
    let mut parent: Vec<u32> = (0..n as u32).collect();
    fn find(p: &mut [u32], x: u32) -> u32 {
        let mut r = x;
        while p[r as usize] != r {
            p[r as usize] = p[p[r as usize] as usize];
            r = p[r as usize];
        }
        r
    }
    let mut used = vec![false; n];
    for t in &mesh.triangles {
        for v in t {
            used[*v as usize] = true;
        }
        let a = find(&mut parent, t[0]);
        for v in &t[1..] {
            let b = find(&mut parent, *v);
            parent[b as usize] = a;
        }
    }
    let mut groups: std::collections::BTreeMap<u32, Vec<u32>> = Default::default();
    for v in 0..n as u32 {
        if used[v as usize] {
            groups.entry(find(&mut parent, v)).or_default().push(v);
        }
    }
    let mut parts: Vec<Vec<u32>> = groups.into_values().collect();
    parts.sort_by(|a, b| b.len().cmp(&a.len()).then(a[0].cmp(&b[0])));
    parts
}

/// A drain hole placed automatically, in the space of the model given.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AutoHole {
    /// On the outside surface.
    pub point: Vec3,
    /// Into the model.
    pub dir: Vec3,
    /// Deep enough to reach the hollow.
    pub depth: f32,
}

/// Places `count` drain holes at the lowest points of a hollow, where they let air in while the
/// model prints and resin out afterwards.
///
/// `outer` is the model, `cavity` the vertices of its hollow, both in world space. Holes keep at
/// least `gap` mm from each other. A hole that would open onto the plate goes through the side
/// instead.
pub fn auto_drain_holes(
    outer: &Bvh,
    cavity: &[Vec3],
    count: usize,
    diameter: f32,
    gap: f32,
) -> Vec<AutoHole> {
    let mut order: Vec<usize> = (0..cavity.len()).collect();
    order.sort_by(|a, b| {
        let (p, q) = (cavity[*a], cavity[*b]);
        p.z.total_cmp(&q.z)
            .then(p.x.total_cmp(&q.x))
            .then(p.y.total_cmp(&q.y))
    });
    let mut out: Vec<AutoHole> = Vec::new();
    let mut taken: Vec<Vec3> = Vec::new();
    for i in order {
        if out.len() >= count {
            break;
        }
        let inside = cavity[i];
        if taken
            .iter()
            .any(|t| t.truncate().distance(inside.truncate()) < gap)
        {
            continue;
        }
        let Some(hole) = hole_from(outer, inside, diameter) else {
            continue;
        };
        taken.push(inside);
        out.push(hole);
    }
    out
}

/// The way out of the hollow from `inside`: straight down (a vertical hole prints best), or
/// sideways when the floor under it lies on the plate.
fn hole_from(outer: &Bvh, inside: Vec3, diameter: f32) -> Option<AutoHole> {
    if let Some(hit) = outer.raycast(inside, Vec3::NEG_Z, 0.0, 1.0e4)
        && hit.point.z > 0.5
    {
        // Past the lowest point, until the hole is open over its whole width.
        return Some(AutoHole {
            point: hit.point,
            dir: Vec3::Z,
            depth: hit.t + (diameter * 1.5).max(2.0),
        });
    }
    // On the plate: go out through the nearest side wall, a little above the floor.
    let from = inside + Vec3::Z * (diameter / 2.0 + 0.5);
    let mut best: Option<(f32, Vec3, Vec3)> = None;
    for k in 0..24 {
        let a = k as f32 / 24.0 * std::f32::consts::TAU;
        let ray = Vec3::new(a.cos(), a.sin(), 0.0);
        if let Some(hit) = outer.raycast(from, ray, 0.0, 1.0e4)
            && best.is_none_or(|(t, _, _)| hit.t < t)
        {
            best = Some((hit.t, hit.point, -ray));
        }
    }
    best.map(|(t, point, dir)| AutoHole {
        point,
        dir,
        depth: t + 1.0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tris(m: &Mesh) -> Vec<[Vec3; 3]> {
        m.triangles
            .iter()
            .map(|t| t.map(|i| m.positions[i as usize]))
            .collect()
    }

    #[test]
    fn hollow_cube_has_walls_and_a_drain_hole_opens_it() {
        let mut cube = Mesh::cuboid(Vec3::splat(20.0));
        for p in &mut cube.positions {
            p.z += 1.0;
        }
        let cavity = hollow(&tris(&cube), 2.0, 0.5).unwrap();
        // Inside out: negative volume of about 16³ (smoothing rounds the corners a bit).
        let vol = -cavity.volume();
        assert!(
            vol > 16f32.powi(3) * 0.85 && vol < 16f32.powi(3) * 1.05,
            "{vol}"
        );

        let mut all = tris(&cube);
        all.extend(tris(&cavity));
        let r = Raster::new([300, 300], [30.0, 30.0]);
        let s = Slicer::new(all.clone(), r, 0.1, 1);
        let (img, _) = s.layer_image(110); // z = 11.05, middle of the cube
        let px = |x: f32, y: f32| {
            let p = r.to_pixels(glam::Vec2::new(x, y));
            img[p.y as usize * 300 + p.x as usize]
        };
        assert_eq!(px(0.0, 0.0), 0, "hollow inside");
        assert_eq!(px(9.0, 0.0), 255, "wall");
        assert_eq!(px(12.0, 0.0), 0, "outside");

        // A hole through the side wall at x = +10.
        all.extend(tris(&drain_hole(
            Vec3::new(10.0, 0.0, 11.0),
            Vec3::NEG_X,
            3.0,
            4.0,
        )));
        let s = Slicer::new(all, r, 0.1, 1);
        let (img2, _) = s.layer_image(110);
        let p = r.to_pixels(glam::Vec2::new(9.0, 0.0));
        assert_eq!(
            img2[p.y as usize * 300 + p.x as usize],
            0,
            "hole through the wall"
        );
        let p = r.to_pixels(glam::Vec2::new(0.0, 0.0));
        assert_eq!(
            img2[p.y as usize * 300 + p.x as usize],
            0,
            "still hollow, not filled"
        );
    }

    #[test]
    fn small_pockets_are_filled_and_big_models_get_bigger_voxels() {
        // A big cube and a small one next to it: the small hollow is a pocket.
        let mut both = Mesh::cuboid(Vec3::splat(20.0));
        let mut small = Mesh::cuboid(Vec3::splat(7.0));
        for p in &mut small.positions {
            p.x += 30.0;
        }
        both.append(&small);
        for p in &mut both.positions {
            p.z += 1.0;
        }
        let all = hollow_detailed(&tris(&both), 2.0, 0.5, 0.0).unwrap();
        assert_eq!((all.cavities, all.pockets_filled), (2, 0));
        let big_only = hollow_detailed(&tris(&both), 2.0, 0.5, 60.0).unwrap();
        assert_eq!((big_only.cavities, big_only.pockets_filled), (1, 1));
        assert!(
            big_only.volume > 16f32.powi(3) * 0.8 && big_only.volume < 16f32.powi(3) * 1.1,
            "{}",
            big_only.volume
        );
        assert!(-big_only.mesh.volume() < -all.mesh.volume());
        assert_eq!(all.voxel, 0.5);
        // Walls thicker than the model: nothing to hollow.
        assert_eq!(
            hollow_detailed(&tris(&both), 12.0, 0.5, 0.0),
            Err(HollowError::TooThin)
        );
        assert_eq!(hollow_detailed(&[], 2.0, 0.5, 0.0), Err(HollowError::Empty));
        // A voxel far too small for the model is made bigger instead of failing.
        assert_eq!(fit_voxel(Vec3::splat(20.0), 0.5), 0.5);
        let big = Vec3::splat(200.0);
        let v = fit_voxel(big, 0.1);
        let (x, y, z) = grid_cells(big, v);
        assert!(v > 0.1 && v < 1.0, "{v}");
        assert!(x * y * z <= MAX_VOXELS);
    }

    #[test]
    fn automatic_drain_holes_sit_low_and_reach_the_hollow() {
        // A cube lifted off the plate: holes go straight down through the floor.
        let mut cube = Mesh::cuboid(Vec3::splat(20.0));
        for p in &mut cube.positions {
            p.z += 6.0;
        }
        let cavity = hollow(&tris(&cube), 2.0, 0.5).unwrap();
        let outer = Bvh::from_mesh(&cube, &glam::Mat4::IDENTITY);
        let holes = auto_drain_holes(&outer, &cavity.positions, 2, 3.0, 8.0);
        assert_eq!(holes.len(), 2);
        for h in &holes {
            assert!((h.point.z - 6.0).abs() < 1e-3, "{h:?}");
            assert!(h.dir.z > 0.9, "{h:?}");
            assert!(h.depth > 4.0 && h.depth < 8.0, "{h:?}");
        }
        assert!(
            holes[0]
                .point
                .truncate()
                .distance(holes[1].point.truncate())
                >= 7.0
        );
        // The same cube on the plate: the floor is closed by the plate, so holes go sideways.
        let cube = Mesh::cuboid(Vec3::splat(20.0));
        let cavity = hollow(&tris(&cube), 2.0, 0.5).unwrap();
        let outer = Bvh::from_mesh(&cube, &glam::Mat4::IDENTITY);
        let holes = auto_drain_holes(&outer, &cavity.positions, 1, 3.0, 8.0);
        assert_eq!(holes.len(), 1);
        assert!(holes[0].dir.z.abs() < 1e-3, "{:?}", holes[0]);
        assert!(holes[0].point.z > 1.0 && holes[0].point.z < 6.0);
    }
}
