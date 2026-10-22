//! The preset library the commands look printers and resins up in.
//!
//! Starts as the built-in presets. Apps call [`set_library`] at start-up to add the user's own.

use std::sync::{Arc, RwLock};

use copal_presets::Library;

static LIBRARY: RwLock<Option<Arc<Library>>> = RwLock::new(None);

/// The current library (built-in presets until an app sets another one).
pub fn library() -> Arc<Library> {
    if let Some(lib) = LIBRARY.read().ok().and_then(|l| l.clone()) {
        return lib;
    }
    let lib = Arc::new(Library::builtin());
    if let Ok(mut slot) = LIBRARY.write() {
        slot.get_or_insert_with(|| lib.clone());
    }
    lib
}

/// Replaces the library, for example after the user saved a resin preset.
pub fn set_library(lib: Library) {
    if let Ok(mut slot) = LIBRARY.write() {
        *slot = Some(Arc::new(lib));
    }
}

/// Built-in presets plus the user's own from the config folder.
pub fn load_user_library() -> Library {
    Library::load(user_presets().as_ref())
}

/// The user's personal preset folder, if the system has a config folder.
pub fn user_presets() -> Option<copal_presets::UserPresets> {
    crate::settings::config_dir()
        .map(|d| copal_presets::UserPresets::new(copal_presets::user_dir(&d)))
}
