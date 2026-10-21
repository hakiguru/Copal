//! The open project: settings and the objects on the build plate.

use std::sync::Arc;

use copal_mesh::{Aabb, Mesh};
use glam::{EulerRot, Mat4, Quat, Vec3};
use serde::{Deserialize, Serialize};

/// Stable id of an object inside one project. Never reused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ObjectId(pub u64);

impl std::fmt::Display for ObjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Position, rotation (degrees, XYZ order) and scale of an object.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation_deg: Vec3,
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation_deg: Vec3::ZERO,
            scale: Vec3::ONE,
        }
    }
}

impl Transform {
    pub fn matrix(&self) -> Mat4 {
        let r = self.rotation_deg;
        let rotation = Quat::from_euler(
            EulerRot::XYZ,
            r.x.to_radians(),
            r.y.to_radians(),
            r.z.to_radians(),
        );
        Mat4::from_scale_rotation_translation(self.scale, rotation, self.translation)
    }
}

/// One model on the build plate.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneObject {
    pub id: ObjectId,
    pub name: String,
    pub transform: Transform,
    /// Shared so that undo history can keep a copy cheaply.
    pub mesh: Arc<Mesh>,
}

impl SceneObject {
    /// Bounding box of the mesh after the object's transform.
    pub fn world_bounds(&self) -> Option<Aabb> {
        let m = self.transform.matrix();
        let mut points = self.mesh.positions.iter().map(|p| m.transform_point3(*p));
        let first = points.next()?;
        let (min, max) = points.fold((first, first), |(lo, hi), p| (lo.min(p), hi.max(p)));
        Some(Aabb { min, max })
    }
}

/// The whole project.
#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    pub name: String,
    /// Printer preset id, e.g. `elegoo/saturn-4-ultra-16k`.
    pub printer: Option<String>,
    /// Resin preset id, e.g. `anycubic/abs-like-pro-2-grey`.
    pub resin: Option<String>,
    pub(crate) objects: Vec<SceneObject>,
    pub(crate) next_id: u64,
}

impl Default for Document {
    fn default() -> Self {
        Self::new("Untitled")
    }
}

impl Document {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            printer: None,
            resin: None,
            objects: Vec::new(),
            next_id: 1,
        }
    }

    pub fn objects(&self) -> &[SceneObject] {
        &self.objects
    }

    pub fn object(&self, id: ObjectId) -> Option<&SceneObject> {
        self.objects.iter().find(|o| o.id == id)
    }

    pub fn object_mut(&mut self, id: ObjectId) -> Option<&mut SceneObject> {
        self.objects.iter_mut().find(|o| o.id == id)
    }

    pub(crate) fn index_of(&self, id: ObjectId) -> Option<usize> {
        self.objects.iter().position(|o| o.id == id)
    }

    pub(crate) fn allocate_id(&mut self) -> ObjectId {
        let id = ObjectId(self.next_id);
        self.next_id += 1;
        id
    }
}
