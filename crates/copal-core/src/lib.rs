//! Copal core: the project document, the command system with undo/redo, the text command
//! language and the `.copal` project file.
//!
//! Every user action goes through a [`Command`], whether it comes from a GUI button,
//! a hotkey, the CLI, the console or a script. See `docs/ARCHITECTURE.md`.

pub mod command;
pub mod commands;
pub mod document;
mod library;
pub mod ops;
pub mod prepare;
pub mod prints;
pub mod project;
pub mod script;
mod session;
pub mod settings;
pub mod time;

pub use command::{Batch, Command, CommandError, History};
pub use commands::{CommandInfo, parse_command, registry};
pub use copal_presets as presets;
pub use copal_supports as supports;
pub use document::Prepared;
pub use document::{Document, ObjectId, ProjectSettings, SceneObject, Transform};
pub use library::{library, load_user_library, reload_user_library, set_library, user_presets};
pub use ops::{Section, SetSettings};
pub use project::ProjectError;
pub use session::{RunError, Session};
pub use settings::{ColorMode, Panels, Settings, ThemeStyle};

/// Copal version, shared by every crate and app.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
