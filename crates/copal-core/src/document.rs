//! The open project: settings and the objects on the build plate.

use std::sync::Arc;

use copal_mesh::{Aabb, Mesh};
use copal_presets::{HollowSettings, LayoutSettings, PrintSettings, SupportSettings};
use glam::{EulerRot, Mat4, Quat, Vec3};
use serde::{Deserialize, Serialize};

/// Stable id of an object inside one project. Never reused.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
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

/// What the Prepare step adds to an object: supports, hollowing and drain holes.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Prepared {
    /// Supports, relative to the object's position (see `copal_supports`).
    pub supports: Option<Arc<copal_supports::SupportSet>>,
    /// Inner wall of a hollowed object, inside out, in the object's own coordinates.
    pub cavity: Option<Arc<Mesh>>,
    pub holes: Vec<copal_supports::DrainHole>,
}

impl Prepared {
    pub fn has_supports(&self) -> bool {
        self.supports.as_ref().is_some_and(|s| !s.is_empty())
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
    pub prep: Prepared,
}

impl SceneObject {
    pub fn new(
        id: ObjectId,
        name: impl Into<String>,
        transform: Transform,
        mesh: Arc<Mesh>,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            transform,
            mesh,
            prep: Prepared::default(),
        }
    }

    /// Bounding box of the mesh after the object's transform.
    pub fn world_bounds(&self) -> Option<Aabb> {
        let m = self.transform.matrix();
        let mut points = self.mesh.positions.iter().map(|p| m.transform_point3(*p));
        let first = points.next()?;
        let (min, max) = points.fold((first, first), |(lo, hi), p| (lo.min(p), hi.max(p)));
        Some(Aabb { min, max })
    }
}

/// Settings stored in the project. Each part can be saved and loaded as a preset.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ProjectSettings {
    pub print: PrintSettings,
    pub supports: SupportSettings,
    pub hollow: HollowSettings,
    pub layout: LayoutSettings,
}

/// The whole project.
#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    pub name: String,
    /// Printer preset id, e.g. `elegoo/saturn-4-ultra-16k`.
    pub printer: Option<String>,
    /// Resin preset id, e.g. `anycubic/abs-like-pro-2-grey`.
    pub resin: Option<String>,
    pub settings: ProjectSettings,
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
            settings: ProjectSettings::default(),
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

    /// Size of the build plate in mm (X, Y) and the build height, from the printer preset.
    /// Falls back to a typical 8K plate when no printer is chosen.
    pub fn build_volume(&self) -> Vec3 {
        self.printer
            .as_deref()
            .and_then(|id| {
                crate::library()
                    .printer(id)
                    .map(|p| Vec3::from(p.volume_mm()))
            })
            .unwrap_or(Vec3::new(218.88, 122.88, 220.0))
    }

    /// Bounding box of all objects together.
    pub fn bounds(&self) -> Option<Aabb> {
        self.objects
            .iter()
            .filter_map(|o| o.world_bounds())
            .reduce(|a, b| Aabb {
                min: a.min.min(b.min),
                max: a.max.max(b.max),
            })
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
