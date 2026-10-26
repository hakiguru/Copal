//! Bounding volume hierarchy for fast ray casts against big meshes (millions of triangles).

use glam::{Mat4, Vec3};

use crate::{Mesh, ray_triangle};

#[derive(Debug, Clone, Copy)]
struct Node {
    min: Vec3,
    max: Vec3,
    /// Leaf: index of the first triangle. Inner node: index of the left child (right = left + 1).
    start: u32,
    /// Number of triangles in a leaf, 0 for inner nodes.
    count: u32,
}

/// A ray hit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hit {
    /// Distance along the ray (in units of the ray direction's length).
    pub t: f32,
    pub point: Vec3,
    /// Unit normal of the hit triangle, from its winding (points out of a well-made mesh).
    pub normal: Vec3,
    /// Index of the triangle in the original order.
    pub triangle: u32,
}

#[derive(Debug, Clone, Default)]
pub struct Bvh {
    nodes: Vec<Node>,
    tris: Vec<[Vec3; 3]>,
    ids: Vec<u32>,
}

const LEAF: usize = 4;

fn tri_bounds(t: &[Vec3; 3]) -> (Vec3, Vec3) {
    (t[0].min(t[1]).min(t[2]), t[0].max(t[1]).max(t[2]))
}

impl Bvh {
    /// Builds the tree for triangles given in world space.
    pub fn new(tris: Vec<[Vec3; 3]>) -> Self {
        let n = tris.len();
        let mut ids: Vec<u32> = (0..n as u32).collect();
        let centers: Vec<Vec3> = tris.iter().map(|t| (t[0] + t[1] + t[2]) / 3.0).collect();
        let mut nodes = Vec::with_capacity(2 * n / LEAF + 1);
        nodes.push(Node {
            min: Vec3::ZERO,
            max: Vec3::ZERO,
            start: 0,
            count: n as u32,
        });
        // (node index, start, end)
        let mut stack = vec![(0usize, 0usize, n)];
        while let Some((ni, s, e)) = stack.pop() {
            let (mut lo, mut hi) = (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY));
            let (mut clo, mut chi) = (lo, hi);
            for &i in &ids[s..e] {
                let (a, b) = tri_bounds(&tris[i as usize]);
                lo = lo.min(a);
                hi = hi.max(b);
                clo = clo.min(centers[i as usize]);
                chi = chi.max(centers[i as usize]);
            }
            // A little padding, so rays that graze an edge are not lost to rounding.
            let pad = Vec3::splat(1e-4) + (hi - lo).abs() * 1e-6;
            nodes[ni].min = lo - pad;
            nodes[ni].max = hi + pad;
            if e - s <= LEAF {
                nodes[ni].start = s as u32;
                nodes[ni].count = (e - s) as u32;
                continue;
            }
            let ext = chi - clo;
            let axis = if ext.x >= ext.y && ext.x >= ext.z {
                0
            } else if ext.y >= ext.z {
                1
            } else {
                2
            };
            let mid = (s + e) / 2;
            ids[s..e].select_nth_unstable_by(mid - s, |a, b| {
                centers[*a as usize][axis].total_cmp(&centers[*b as usize][axis])
            });
            let left = nodes.len();
            nodes.push(Node {
                min: Vec3::ZERO,
                max: Vec3::ZERO,
                start: 0,
                count: 0,
            });
            nodes.push(Node {
                min: Vec3::ZERO,
                max: Vec3::ZERO,
                start: 0,
                count: 0,
            });
            nodes[ni].start = left as u32;
            nodes[ni].count = 0;
            stack.push((left, s, mid));
            stack.push((left + 1, mid, e));
        }
        let tris_sorted = ids.iter().map(|&i| tris[i as usize]).collect();
        Self {
            nodes,
            tris: tris_sorted,
            ids,
        }
    }

    /// A tree for a mesh placed with `matrix`.
    pub fn from_mesh(mesh: &Mesh, matrix: &Mat4) -> Self {
        let p: Vec<Vec3> = mesh
            .positions
            .iter()
            .map(|v| matrix.transform_point3(*v))
            .collect();
        let flip = matrix.determinant() < 0.0;
        Self::new(
            mesh.triangles
                .iter()
                .map(|t| {
                    let (a, b, c) = (p[t[0] as usize], p[t[1] as usize], p[t[2] as usize]);
                    if flip { [a, c, b] } else { [a, b, c] }
                })
                .collect(),
        )
    }

    pub fn is_empty(&self) -> bool {
        self.tris.is_empty()
    }

    pub fn triangles(&self) -> &[[Vec3; 3]] {
        &self.tris
    }

    /// Bounds of everything.
    pub fn bounds(&self) -> Option<(Vec3, Vec3)> {
        self.nodes
            .first()
            .filter(|_| !self.tris.is_empty())
            .map(|n| (n.min, n.max))
    }

    /// The nearest hit along the ray with `t` in `(min_t, max_t)`.
    pub fn raycast(&self, origin: Vec3, dir: Vec3, min_t: f32, max_t: f32) -> Option<Hit> {
        if self.tris.is_empty() {
            return None;
        }
        let inv = dir.recip();
        let mut best: Option<(f32, usize)> = None;
        let mut limit = max_t;
        let mut stack = [0usize; 64];
        let mut sp = 1;
        stack[0] = 0;
        while sp > 0 {
            sp -= 1;
            let n = self.nodes[stack[sp]];
            if !slab(origin, inv, n.min, n.max, limit) {
                continue;
            }
            if n.count > 0 {
                for i in n.start as usize..(n.start + n.count) as usize {
                    let [a, b, c] = self.tris[i];
                    if let Some(t) = ray_triangle(origin, dir, a, b, c)
                        && t > min_t
                        && t < limit
                    {
                        limit = t;
                        best = Some((t, i));
                    }
                }
            } else if sp + 2 <= stack.len() {
                stack[sp] = n.start as usize;
                stack[sp + 1] = n.start as usize + 1;
                sp += 2;
            }
        }
        best.map(|(t, i)| {
            let [a, b, c] = self.tris[i];
            Hit {
                t,
                point: origin + dir * t,
                normal: (b - a).cross(c - a).normalize_or_zero(),
                triangle: self.ids[i],
            }
        })
    }

    /// True if anything is hit with `t` in `(min_t, max_t)`.
    pub fn occluded(&self, origin: Vec3, dir: Vec3, min_t: f32, max_t: f32) -> bool {
        self.raycast(origin, dir, min_t, max_t).is_some()
    }

    /// Closest point on the mesh to `p` within `radius`, with the triangle's normal.
    pub fn closest_point(&self, p: Vec3, radius: f32) -> Option<(Vec3, Vec3)> {
        let mut best: Option<(f32, Vec3, Vec3)> = None;
        let mut r2 = radius * radius;
        let mut stack = vec![0usize];
        while let Some(ni) = stack.pop() {
            let n = self.nodes[ni];
            let d = (n.min - p).max(p - n.max).max(Vec3::ZERO);
            if d.length_squared() > r2 {
                continue;
            }
            if n.count > 0 {
                for i in n.start as usize..(n.start + n.count) as usize {
                    let [a, b, c] = self.tris[i];
                    let q = closest_on_triangle(p, a, b, c);
                    let dd = (q - p).length_squared();
                    if dd < r2 {
                        r2 = dd;
                        best = Some((dd, q, (b - a).cross(c - a).normalize_or_zero()));
                    }
                }
            } else {
                stack.push(n.start as usize);
                stack.push(n.start as usize + 1);
            }
        }
        best.map(|(_, q, nrm)| (q, nrm))
    }
}

fn slab(o: Vec3, inv: Vec3, min: Vec3, max: Vec3, max_t: f32) -> bool {
    let t1 = (min - o) * inv;
    let t2 = (max - o) * inv;
    let tmin = t1.min(t2);
    let tmax = t1.max(t2);
    // NaN (0 * inf) compares false; treat such axes as "inside".
    let lo = [tmin.x, tmin.y, tmin.z]
        .iter()
        .filter(|v| !v.is_nan())
        .fold(f32::NEG_INFINITY, |a, b| a.max(*b));
    let hi = [tmax.x, tmax.y, tmax.z]
        .iter()
        .filter(|v| !v.is_nan())
        .fold(f32::INFINITY, |a, b| a.min(*b));
    hi >= lo.max(0.0) && lo <= max_t
}

/// Closest point on triangle abc to p (Ericson, Real-Time Collision Detection).
pub fn closest_on_triangle(p: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Vec3 {
    let ab = b - a;
    let ac = c - a;
    let ap = p - a;
    let d1 = ab.dot(ap);
    let d2 = ac.dot(ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return a;
    }
    let bp = p - b;
    let d3 = ab.dot(bp);
    let d4 = ac.dot(bp);
    if d3 >= 0.0 && d4 <= d3 {
        return b;
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        return a + ab * (d1 / (d1 - d3));
    }
    let cp = p - c;
    let d5 = ab.dot(cp);
    let d6 = ac.dot(cp);
    if d6 >= 0.0 && d5 <= d6 {
        return c;
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        return a + ac * (d2 / (d2 - d6));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        return b + (c - b) * ((d4 - d3) / ((d4 - d3) + (d5 - d6)));
    }
    let denom = 1.0 / (va + vb + vc);
    a + ab * (vb * denom) + ac * (vc * denom)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_brute_force() {
        // A grid of many small cubes.
        let mut m = Mesh::default();
        for i in 0..10 {
            for j in 0..10 {
                let mut c = Mesh::cuboid(Vec3::splat(1.0));
                for p in &mut c.positions {
                    *p += Vec3::new(i as f32 * 2.0, j as f32 * 2.0, (i + j) as f32 * 0.3);
                }
                m.append(&c);
            }
        }
        let bvh = Bvh::from_mesh(&m, &Mat4::IDENTITY);
        for k in 0..200 {
            let o = Vec3::new(
                (k % 23) as f32 * 0.9 - 1.0,
                (k % 17) as f32 * 1.2 - 1.0,
                20.0,
            );
            let d = Vec3::new(0.05 * (k % 5) as f32, -0.03 * (k % 7) as f32, -1.0).normalize();
            let brute = m.raycast(o, d).map(|(t, _)| t);
            let fast = bvh.raycast(o, d, 0.0, f32::INFINITY).map(|h| h.t);
            match (brute, fast) {
                (Some(a), Some(b)) => assert!((a - b).abs() < 1e-4, "{k}: {a} vs {b}"),
                (None, None) => {}
                other => panic!("{k}: {other:?}"),
            }
        }
        let (q, n) = bvh.closest_point(Vec3::new(0.0, 0.0, -2.0), 5.0).unwrap();
        assert!((q.z - 0.0).abs() < 1e-5 && n.z < -0.9, "{q} {n}");
    }
}
