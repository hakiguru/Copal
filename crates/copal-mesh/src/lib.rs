//! Triangle meshes for Copal.
//!
//! A [`Mesh`] is an indexed triangle list. Import from STL/OBJ/3MF arrives in 0.1;
//! repair, booleans and editing come later (see `docs/ROADMAP.md`).

pub mod bvh;
pub mod import;

use glam::{Mat4, Vec3};
use thiserror::Error;

/// An indexed triangle mesh. Units are millimeters.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Mesh {
    pub positions: Vec<Vec3>,
    pub triangles: Vec<[u32; 3]>,
}

/// Axis-aligned bounding box.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub fn center(&self) -> Vec3 {
        (self.min + self.max) / 2.0
    }

    pub fn size(&self) -> Vec3 {
        self.max - self.min
    }
}

#[derive(Debug, Error, PartialEq)]
pub enum MeshError {
    #[error("not a Copal mesh file")]
    BadMagic,
    #[error("unsupported mesh version {0}")]
    BadVersion(u32),
    #[error("mesh data is truncated")]
    Truncated,
    #[error("triangle {triangle} uses vertex {index}, but the mesh has {count} vertices")]
    BadIndex {
        triangle: usize,
        index: u32,
        count: usize,
    },
}

const MAGIC: &[u8; 8] = b"COPALMSH";
const VERSION: u32 = 1;

impl Mesh {
    /// An axis-aligned box centered on the origin in X/Y, standing on Z = 0.
    pub fn cuboid(size: Vec3) -> Self {
        let h = Vec3::new(size.x / 2.0, size.y / 2.0, 0.0);
        let p = |x: f32, y: f32, z: f32| Vec3::new(x * h.x, y * h.y, z * size.z);
        let positions = vec![
            p(-1.0, -1.0, 0.0),
            p(1.0, -1.0, 0.0),
            p(1.0, 1.0, 0.0),
            p(-1.0, 1.0, 0.0),
            p(-1.0, -1.0, 1.0),
            p(1.0, -1.0, 1.0),
            p(1.0, 1.0, 1.0),
            p(-1.0, 1.0, 1.0),
        ];
        // Counter-clockwise when seen from outside.
        let triangles = vec![
            [0, 2, 1],
            [0, 3, 2], // bottom
            [4, 5, 6],
            [4, 6, 7], // top
            [0, 1, 5],
            [0, 5, 4], // front
            [1, 2, 6],
            [1, 6, 5], // right
            [2, 3, 7],
            [2, 7, 6], // back
            [3, 0, 4],
            [3, 4, 7], // left
        ];
        Self {
            positions,
            triangles,
        }
    }

    pub fn vertex_count(&self) -> usize {
        self.positions.len()
    }

    pub fn triangle_count(&self) -> usize {
        self.triangles.len()
    }

    /// Bounding box, or `None` for an empty mesh.
    pub fn bounds(&self) -> Option<Aabb> {
        let first = *self.positions.first()?;
        let (min, max) = self
            .positions
            .iter()
            .fold((first, first), |(lo, hi), p| (lo.min(*p), hi.max(*p)));
        Some(Aabb { min, max })
    }

    /// Signed volume in mm³ (positive for a closed mesh with outward-facing triangles).
    pub fn volume(&self) -> f32 {
        self.triangles
            .iter()
            .map(|t| {
                let [a, b, c] = t.map(|i| self.positions[i as usize]);
                a.dot(b.cross(c)) / 6.0
            })
            .sum()
    }

    /// Flips all triangles if the mesh is inside-out (negative volume).
    /// Returns true if it flipped.
    pub fn fix_orientation(&mut self) -> bool {
        if self.volume() < 0.0 {
            for t in &mut self.triangles {
                t.swap(1, 2);
            }
            true
        } else {
            false
        }
    }

    /// A copy with every vertex transformed.
    pub fn transformed(&self, m: &Mat4) -> Mesh {
        let flip = m.determinant() < 0.0;
        Mesh {
            positions: self
                .positions
                .iter()
                .map(|p| m.transform_point3(*p))
                .collect(),
            triangles: if flip {
                self.triangles.iter().map(|t| [t[0], t[2], t[1]]).collect()
            } else {
                self.triangles.clone()
            },
        }
    }

    /// Adds all triangles of `other` to this mesh.
    pub fn append(&mut self, other: &Mesh) {
        let base = self.positions.len() as u32;
        self.positions.extend_from_slice(&other.positions);
        self.triangles.extend(
            other
                .triangles
                .iter()
                .map(|t| [t[0] + base, t[1] + base, t[2] + base]),
        );
    }

    /// Distance along the ray to the nearest triangle, if any. `dir` must be normalized.
    pub fn raycast(&self, origin: Vec3, dir: Vec3) -> Option<(f32, usize)> {
        let mut best: Option<(f32, usize)> = None;
        for (i, t) in self.triangles.iter().enumerate() {
            let [a, b, c] = t.map(|k| self.positions[k as usize]);
            if let Some(d) = ray_triangle(origin, dir, a, b, c)
                && best.is_none_or(|(bd, _)| d < bd)
            {
                best = Some((d, i));
            }
        }
        best
    }

    /// Unit normal of a triangle (zero for a degenerate one).
    pub fn triangle_normal(&self, i: usize) -> Vec3 {
        let [a, b, c] = self.triangles[i].map(|k| self.positions[k as usize]);
        (b - a).cross(c - a).normalize_or_zero()
    }

    /// Area of a triangle in mm².
    pub fn triangle_area(&self, i: usize) -> f32 {
        let [a, b, c] = self.triangles[i].map(|k| self.positions[k as usize]);
        (b - a).cross(c - a).length() * 0.5
    }

    /// Checks that every triangle points at an existing vertex.
    pub fn validate(&self) -> Result<(), MeshError> {
        let count = self.positions.len();
        for (triangle, t) in self.triangles.iter().enumerate() {
            if let Some(&index) = t.iter().find(|&&i| i as usize >= count) {
                return Err(MeshError::BadIndex {
                    triangle,
                    index,
                    count,
                });
            }
        }
        Ok(())
    }

    /// Encodes the mesh in Copal's compact binary form (little-endian).
    ///
    /// Layout: `COPALMSH`, u32 version, u32 vertex count, u32 triangle count,
    /// then `3 × f32` per vertex and `3 × u32` per triangle.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out =
            Vec::with_capacity(20 + self.positions.len() * 12 + self.triangles.len() * 12);
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&VERSION.to_le_bytes());
        out.extend_from_slice(&(self.positions.len() as u32).to_le_bytes());
        out.extend_from_slice(&(self.triangles.len() as u32).to_le_bytes());
        for p in &self.positions {
            for v in p.to_array() {
                out.extend_from_slice(&v.to_le_bytes());
            }
        }
        for t in &self.triangles {
            for i in t {
                out.extend_from_slice(&i.to_le_bytes());
            }
        }
        out
    }

    /// Decodes a mesh written by [`Mesh::to_bytes`].
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, MeshError> {
        let mut r = Reader { bytes, pos: 0 };
        if r.take(8)? != MAGIC {
            return Err(MeshError::BadMagic);
        }
        let version = r.u32()?;
        if version != VERSION {
            return Err(MeshError::BadVersion(version));
        }
        let vertex_count = r.u32()? as usize;
        let triangle_count = r.u32()? as usize;
        let needed = vertex_count
            .checked_mul(12)
            .and_then(|v| {
                triangle_count
                    .checked_mul(12)
                    .and_then(|t| v.checked_add(t))
            })
            .ok_or(MeshError::Truncated)?;
        if bytes.len() - r.pos < needed {
            return Err(MeshError::Truncated);
        }
        let mut positions = Vec::with_capacity(vertex_count);
        for _ in 0..vertex_count {
            positions.push(Vec3::new(r.f32()?, r.f32()?, r.f32()?));
        }
        let mut triangles = Vec::with_capacity(triangle_count);
        for _ in 0..triangle_count {
            triangles.push([r.u32()?, r.u32()?, r.u32()?]);
        }
        let mesh = Self {
            positions,
            triangles,
        };
        mesh.validate()?;
        Ok(mesh)
    }
}

/// Möller–Trumbore ray / triangle test. Hits from both sides count.
pub fn ray_triangle(origin: Vec3, dir: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Option<f32> {
    let e1 = b - a;
    let e2 = c - a;
    let p = dir.cross(e2);
    let det = e1.dot(p);
    if det.abs() < 1e-12 {
        return None;
    }
    let inv = 1.0 / det;
    let s = origin - a;
    let u = s.dot(p) * inv;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = s.cross(e1);
    let v = dir.dot(q) * inv;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let t = e2.dot(q) * inv;
    (t > 1e-6).then_some(t)
}

struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl Reader<'_> {
    fn take(&mut self, n: usize) -> Result<&[u8], MeshError> {
        let end = self.pos.checked_add(n).ok_or(MeshError::Truncated)?;
        let slice = self.bytes.get(self.pos..end).ok_or(MeshError::Truncated)?;
        self.pos = end;
        Ok(slice)
    }

    fn u32(&mut self) -> Result<u32, MeshError> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }

    fn f32(&mut self) -> Result<f32, MeshError> {
        Ok(f32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cuboid_has_expected_shape() {
        let m = Mesh::cuboid(Vec3::new(10.0, 20.0, 30.0));
        assert_eq!(m.vertex_count(), 8);
        assert_eq!(m.triangle_count(), 12);
        let b = m.bounds().unwrap();
        assert_eq!(b.size(), Vec3::new(10.0, 20.0, 30.0));
        assert_eq!(b.min.z, 0.0);
        assert!((m.volume() - 6000.0).abs() < 1e-2, "volume {}", m.volume());
    }

    #[test]
    fn binary_round_trip() {
        let m = Mesh::cuboid(Vec3::splat(5.0));
        let back = Mesh::from_bytes(&m.to_bytes()).unwrap();
        assert_eq!(m, back);
    }

    #[test]
    fn rejects_garbage() {
        assert_eq!(Mesh::from_bytes(b"hello"), Err(MeshError::Truncated));
        assert_eq!(
            Mesh::from_bytes(b"NOTAMESH0000000000000"),
            Err(MeshError::BadMagic)
        );
        let mut bytes = Mesh::cuboid(Vec3::ONE).to_bytes();
        bytes.truncate(bytes.len() - 1);
        assert_eq!(Mesh::from_bytes(&bytes), Err(MeshError::Truncated));
    }

    #[test]
    fn rejects_bad_index() {
        let mut m = Mesh::cuboid(Vec3::ONE);
        m.triangles[3] = [0, 1, 99];
        assert!(matches!(
            Mesh::from_bytes(&m.to_bytes()),
            Err(MeshError::BadIndex {
                triangle: 3,
                index: 99,
                ..
            })
        ));
    }

    #[test]
    fn orientation_fix_and_transform() {
        let mut m = Mesh::cuboid(Vec3::ONE);
        for t in &mut m.triangles {
            t.swap(1, 2);
        }
        assert!(m.volume() < 0.0);
        assert!(m.fix_orientation());
        assert!(m.volume() > 0.0);
        let mirrored = m.transformed(&Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0)));
        assert!(
            mirrored.volume() > 0.0,
            "mirroring keeps triangles facing out"
        );
    }

    #[test]
    fn raycast_hits_the_near_face() {
        let m = Mesh::cuboid(Vec3::splat(10.0));
        let (d, _) = m.raycast(Vec3::new(0.0, 0.0, 50.0), Vec3::NEG_Z).unwrap();
        assert!((d - 40.0).abs() < 1e-4);
        assert!(m.raycast(Vec3::new(20.0, 0.0, 50.0), Vec3::NEG_Z).is_none());
    }

    #[test]
    fn empty_mesh_has_no_bounds() {
        assert!(Mesh::default().bounds().is_none());
    }
}
