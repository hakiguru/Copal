//! Shared look and widgets of the Copal apps.
//!
//! - [`theme`]: warm dark colors with amber accents.
//! - [`crystal`]: the amber crystal material for borders, made from the logo palette.
//! - [`widgets`]: buttons, tabs and frames with crystal borders.
//! - [`workspace`]: the state and panels both apps share (console, inspector, scene list).

pub mod crystal;
pub mod look;
pub mod palette;
pub mod theme;
pub mod viewport;
pub mod widgets;
pub mod workspace;

pub use look::Look;
pub use theme::Edition;
pub use workspace::Workspace;
