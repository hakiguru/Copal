//! Turns supports into closed, outward-facing triangle meshes for drawing and slicing.

use std::ops::Range;

use copal_mesh::Mesh;
use glam::{Vec2, Vec3};

use crate::model::{Foot, SupportSet};

/// Mesh of all supports of one object, with the triangles of each support.
#[derive(Debug, Clone, Default)]
pub struct SupportMesh {
    pub mesh: Mesh,
    /// (support id, its triangles). Braces and the raft are not in any range.
    pub ranges: Vec<(u32, Range<usize>)>,
}

/// How far tips and feet reach into the model, so the joint is solid.
pub const EMBED: f32 = 0.3;

fn basis(axis: Vec3) -> (Vec3, Vec3) {
    let u = if axis.x.abs() < 0.9 { Vec3::X } else { Vec3::Y };
    let u = (u - axis * u.dot(axis)).normalize();
    (u, axis.cross(u))
}

/// A closed truncated cone from `a` (radius `ra`) to `b` (radius `rb`).
pub fn frustum(m: &mut Mesh, a: Vec3, ra: f32, b: Vec3, rb: f32, sides: u32) {
    let len = (b - a).length();
    if len < 1e-4 {
        return;
    }
    let axis = (b - a) / len;
    let (u, v) = basis(axis);
    let (ra, rb) = (ra.max(0.01), rb.max(0.01));
    let base = m.positions.len() as u32;
    m.positions.push(a);
    m.positions.push(b);
    for k in 0..sides {
        let t = k as f32 / sides as f32 * std::f32::consts::TAU;
        let dir = u * t.cos() + v * t.sin();
        m.positions.push(a + dir * ra);
        m.positions.push(b + dir * rb);
    }
    let ring_a = |k: u32| base + 2 + 2 * (k % sides);
    let ring_b = |k: u32| base + 3 + 2 * (k % sides);
    for k in 0..sides {
        let (a0, a1, b0, b1) = (ring_a(k), ring_a(k + 1), ring_b(k), ring_b(k + 1));
        m.triangles.push([a0, a1, b1]);
        m.triangles.push([a0, b1, b0]);
        m.triangles.push([base, a1, a0]);
        m.triangles.push([base + 1, b0, b1]);
    }
}

/// A closed sphere.
pub fn sphere(m: &mut Mesh, c: Vec3, r: f32, rings: u32, segments: u32) {
    let base = m.positions.len() as u32;
    m.positions.push(c + Vec3::Z * r);
    for i in 1..rings {
        let phi = std::f32::consts::PI * i as f32 / rings as f32;
        for j in 0..segments {
            let th = std::f32::consts::TAU * j as f32 / segments as f32;
            m.positions
                .push(c + r * Vec3::new(phi.sin() * th.cos(), phi.sin() * th.sin(), phi.cos()));
        }
    }
    let bottom = m.positions.len() as u32;
    m.positions.push(c - Vec3::Z * r);
    let at = |i: u32, j: u32| base + 1 + (i - 1) * segments + (j % segments);
    for j in 0..segments {
        m.triangles.push([base, at(1, j), at(1, j + 1)]);
        m.triangles
            .push([bottom, at(rings - 1, j + 1), at(rings - 1, j)]);
    }
    for i in 1..rings - 1 {
        for j in 0..segments {
            let (a, b, c2, d) = (at(i, j), at(i, j + 1), at(i + 1, j), at(i + 1, j + 1));
            m.triangles.push([a, c2, d]);
            m.triangles.push([a, d, b]);
        }
    }
}

/// A flat slab under the outline with a chamfered top edge.
pub fn raft(m: &mut Mesh, outline: &[Vec2], thickness: f32) {
    if outline.len() < 3 {
        return;
    }
    let n = outline.len() as u32;
    let centroid = outline.iter().copied().sum::<Vec2>() / outline.len() as f32;
    let chamfer = thickness.min(1.5);
    let base = m.positions.len() as u32;
    for p in outline {
        m.positions.push(p.extend(0.0));
    }
    for p in outline {
        let d = *p - centroid;
        let len = d.length().max(1e-3);
        let top = centroid + d * ((len - chamfer).max(len * 0.5) / len);
        m.positions.push(top.extend(thickness));
    }
    m.positions.push(centroid.extend(0.0));
    m.positions.push(centroid.extend(thickness));
    let (cb, ct) = (base + 2 * n, base + 2 * n + 1);
    for k in 0..n {
        let (b0, b1) = (base + k, base + (k + 1) % n);
        let (t0, t1) = (base + n + k, base + n + (k + 1) % n);
        // The outline is counter-clockwise seen from above.
        m.triangles.push([cb, b1, b0]);
        m.triangles.push([ct, t0, t1]);
        m.triangles.push([b0, b1, t1]);
        m.triangles.push([b0, t1, t0]);
    }
}

/// Builds the mesh of a support set for an object at `position`.
pub fn build_mesh(set: &SupportSet, position: Vec3) -> SupportMesh {
    let mut out = SupportMesh::default();
    let m = &mut out.mesh;
    let base_top = set.base_top();
    let raft_top = set.raft_top();
    for s in &set.supports {
        let start = m.triangles.len();
        let p = |v: Vec3| v + position;
        let tip = p(s.tip);
        let joint = p(s.joint);
        let d = (joint - tip).normalize_or_zero();
        let (r, rt) = (s.diameter / 2.0, s.tip_diameter / 2.0);
        let sides = if s.diameter < 0.9 { 8 } else { 10 };
        frustum(m, tip - d * EMBED, rt, joint, r, sides);
        sphere(m, joint, r, 4, sides);
        let mut top = joint;
        if let Some(k) = s.knee {
            let k = p(k);
            frustum(m, joint, r, k, r, sides);
            sphere(m, k, r, 4, sides);
            top = k;
        }
        match s.foot {
            Foot::Plate => {
                let bt = Vec3::new(top.x, top.y, base_top.min(top.z - 0.05));
                frustum(m, top, r, bt, r, sides);
                let bottom = Vec3::new(top.x, top.y, (raft_top - 0.2).max(0.0));
                frustum(m, bt, r, bottom, set.base_diameter / 2.0, 12);
            }
            Foot::Model { point } => {
                let h = p(point);
                let gap = top.z - h.z;
                let cone = (gap * 0.45).clamp(0.2, 2.0);
                let neck = h + Vec3::Z * cone;
                if top.z - neck.z > 0.05 {
                    frustum(m, top, r, neck, r, sides);
                }
                frustum(m, neck, r, h - Vec3::Z * EMBED, rt, sides);
            }
        }
        out.ranges.push((s.id, start..m.triangles.len()));
    }
    for b in &set.braces {
        let (Some(sa), Some(sb)) = (set.get(b.a), set.get(b.b)) else {
            continue;
        };
        let a = sa.column_top().truncate().extend(b.za) + position;
        let c = sb.column_top().truncate().extend(b.zb) + position;
        frustum(m, a, b.diameter / 2.0, c, b.diameter / 2.0, 6);
    }
    if !set.raft.is_empty() {
        let outline: Vec<Vec2> = set.raft.iter().map(|v| *v + position.truncate()).collect();
        raft(m, &outline, set.raft_thickness);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primitives_are_closed_and_outward() {
        let mut m = Mesh::default();
        frustum(&mut m, Vec3::ZERO, 1.0, Vec3::new(0.3, 0.2, 5.0), 0.5, 16);
        let v = m.volume();
        // Truncated cone volume π h/3 (R² + Rr + r²) ≈ 9.2 for h≈5.01.
        assert!(v > 8.0 && v < 9.5, "{v}");
        let mut s = Mesh::default();
        sphere(&mut s, Vec3::new(1.0, 2.0, 3.0), 2.0, 16, 24);
        let vs = s.volume();
        assert!(
            (vs - 4.0 / 3.0 * std::f32::consts::PI * 8.0).abs() < 2.0,
            "{vs}"
        );
        let mut r = Mesh::default();
        raft(
            &mut r,
            &[
                Vec2::new(0.0, 0.0),
                Vec2::new(10.0, 0.0),
                Vec2::new(10.0, 10.0),
                Vec2::new(0.0, 10.0),
            ],
            1.0,
        );
        let vr = r.volume();
        assert!(vr > 70.0 && vr < 100.0, "{vr}");
    }
}
