//! Shared look, 3D view and panels of the Copal apps.
//!
//! - [`theme`], [`palette`], [`look`], [`crystal`], [`widgets`]: colors, outlines, buttons.
//! - [`camera`], [`render`], [`gizmo`], [`view3d`]: the OpenGL 3D view with handles.
//! - [`workspace`], [`panels`], [`files`]: state and panels both apps share.
//! - [`hotkeys`]: keyboard shortcuts the user can change, and their window.
//! - [`desktop`]: the app's name and icon in the taskbar.

pub mod camera;
pub mod crystal;
pub mod desktop;
pub mod files;
pub mod gizmo;
pub mod hotkeys;
pub mod layout;
pub mod look;
pub mod palette;
pub mod panels;
pub mod prep;
pub mod render;
pub mod resins;
pub mod theme;
pub mod view3d;
pub mod widgets;
pub mod workspace;

pub use look::Look;
pub use theme::Edition;
pub use workspace::Workspace;
