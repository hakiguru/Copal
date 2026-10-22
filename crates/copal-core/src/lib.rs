//! Copal core: the project document, the command system with undo/redo, the text command
//! language and the `.copal` project file.
//!
//! Every user action goes through a [`Command`], whether it comes from a GUI button,
//! a hotkey, the CLI, the console or a script. See `docs/ARCHITECTURE.md`.

pub mod command;
pub mod commands;
pub mod document;
pub mod project;
pub mod script;
mod session;
pub mod settings;

pub use command::{Command, CommandError, History};
pub use commands::{CommandInfo, parse_command, registry};
pub use document::{Document, ObjectId, SceneObject, Transform};
pub use project::ProjectError;
pub use session::{RunError, Session};
pub use settings::{ColorMode, Settings, ThemeStyle};

/// Copal version, shared by every crate and app.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
