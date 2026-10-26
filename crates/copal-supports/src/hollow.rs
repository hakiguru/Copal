//! Hollowing and drain holes.
//!
//! The model is turned into voxels (test layers from the slicer), a distance transform gives each
//! inside voxel its distance to the outside, and the surface at `wall thickness` is extracted
//! with surface nets. That surface is stored inside out, so the slicer cuts it away. Drain holes
//! are inside-out cylinders too.

use copal_mesh::Mesh;
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

/// The inner surface of a hollowed model, inside out, in the same space as `tris`.
/// `None` when the model is too thin to hollow.
pub fn hollow(tris: &[[Vec3; 3]], thickness: f32, voxel: f32) -> Option<Mesh> {
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
    let v = voxel.max(0.05);
    let pad = 2usize;
    let size = hi - lo;
    let nx = (size.x / v).ceil() as usize + 2 * pad;
    let ny = (size.y / v).ceil() as usize + 2 * pad;
    let nz = (size.z / v).ceil() as usize + 2 * pad;
    if nx * ny * nz > 120_000_000 {
        return None;
    }
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
        let columns: Vec<Vec<f32>> = (0..plane)
            .into_par_iter()
            .map(|i| {
                let (mut a, mut b, mut c) = (Vec::new(), Vec::new(), Vec::new());
                let mut col: Vec<f32> = (0..nz).map(|k| d[k * plane + i]).collect();
                edt_1d(&mut col, &mut a, &mut b, &mut c);
                col
            })
            .collect();
        for (i, col) in columns.into_iter().enumerate() {
            for (k, val) in col.into_iter().enumerate() {
                d[k * plane + i] = val;
            }
        }
    }
    // Field: > 0 deeper than the wall.
    let g: Vec<f32> = d
        .par_iter()
        .map(|&s| {
            if s <= 0.0 {
                -v
            } else {
                s.sqrt() * v - 0.5 * v - thickness
            }
        })
        .collect();
    if !g.iter().any(|x| *x > 0.0) {
        return None;
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
    Some(mesh)
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
}
