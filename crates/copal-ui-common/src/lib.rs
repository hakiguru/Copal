//! Shared look, 3D view and panels of the Copal apps.
//!
//! - [`theme`], [`palette`], [`look`], [`crystal`], [`widgets`]: colors, outlines, buttons.
//! - [`camera`], [`render`], [`gizmo`], [`view3d`]: the OpenGL 3D view with handles.
//! - [`workspace`], [`panels`], [`files`]: state and panels both apps share.

pub mod camera;
pub mod crystal;
pub mod files;
pub mod gizmo;
pub mod look;
pub mod palette;
pub mod panels;
pub mod render;
pub mod theme;
pub mod view3d;
pub mod widgets;
pub mod workspace;

pub use look::Look;
pub use theme::Edition;
pub use workspace::Workspace;
