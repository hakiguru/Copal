//! Triangle meshes for Copal.
//!
//! A [`Mesh`] is an indexed triangle list. Import from STL/OBJ/3MF arrives in 0.1;
//! repair, booleans and editing come later (see `docs/ROADMAP.md`).

use glam::Vec3;
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
    fn empty_mesh_has_no_bounds() {
        assert!(Mesh::default().bounds().is_none());
    }
}
