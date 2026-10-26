//! App settings shared by Copal, Copal Lite and the CLI.
//!
//! Stored as `settings.json` in the user config folder:
//! `~/.config/copal` on Linux (or `$XDG_CONFIG_HOME/copal`), `%APPDATA%\Copal` on Windows.
//! `COPAL_CONFIG_DIR` overrides the folder (used by tests).

use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// How the interface looks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeStyle {
    /// Calm neutral colors with simple orange outlines.
    #[default]
    Default,
    /// Warm amber colors with crystal borders and glints.
    Art,
}

/// Dark or light colors. Both themes have both.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ColorMode {
    #[default]
    Dark,
    Light,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub theme: ThemeStyle,
    pub mode: ColorMode,
    /// Size of the move / rotate / scale handles in the 3D view, 1 = normal.
    pub gizmo_size: f32,
    /// Which side windows of the apps are open (they fold away like tool windows in an IDE).
    pub panels: Panels,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Panels {
    pub tools: bool,
    pub settings: bool,
    pub console: bool,
}

impl Default for Panels {
    fn default() -> Self {
        Self {
            tools: true,
            settings: true,
            console: true,
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: ThemeStyle::Default,
            mode: ColorMode::Dark,
            gizmo_size: 1.0,
            panels: Panels::default(),
        }
    }
}

/// Smallest and biggest gizmo size.
pub const GIZMO_SIZES: (f32, f32) = (0.4, 3.0);

#[derive(Debug, Error)]
pub enum SettingsError {
    #[error("unknown setting `{0}` (known: theme, mode, gizmo)")]
    UnknownKey(String),
    #[error("`{key}` can be {allowed}, not `{value}`")]
    BadValue {
        key: String,
        value: String,
        allowed: &'static str,
    },
    #[error("cannot find the config folder (no HOME or APPDATA)")]
    NoConfigDir,
    #[error("cannot read or write settings: {0}")]
    Io(#[from] std::io::Error),
    #[error("settings file is broken: {0}")]
    Json(#[from] serde_json::Error),
}

impl Settings {
    /// Every setting with the values it accepts, for help screens.
    pub const KEYS: &[(&str, &str)] = &[
        ("theme", "default | art"),
        ("mode", "dark | light"),
        ("gizmo", "0.4 – 3, size of the move/rotate/scale handles"),
    ];

    pub fn get(&self, key: &str) -> Result<String, SettingsError> {
        match key {
            "theme" => Ok(self.theme.to_string()),
            "mode" => Ok(self.mode.to_string()),
            "gizmo" => Ok(format!("{}", self.gizmo_size)),
            _ => Err(SettingsError::UnknownKey(key.to_string())),
        }
    }

    pub fn set(&mut self, key: &str, value: &str) -> Result<(), SettingsError> {
        let bad = |allowed| SettingsError::BadValue {
            key: key.to_string(),
            value: value.to_string(),
            allowed,
        };
        match key {
            "theme" => self.theme = value.parse().map_err(|_| bad("default or art"))?,
            "mode" => self.mode = value.parse().map_err(|_| bad("dark or light"))?,
            "gizmo" => {
                let v: f32 = value.parse().map_err(|_| bad("a number from 0.4 to 3"))?;
                if !(GIZMO_SIZES.0..=GIZMO_SIZES.1).contains(&v) {
                    return Err(bad("a number from 0.4 to 3"));
                }
                self.gizmo_size = v;
            }
            _ => return Err(SettingsError::UnknownKey(key.to_string())),
        }
        Ok(())
    }

    /// The color mode in use.
    pub fn effective_mode(&self) -> ColorMode {
        self.mode
    }

    /// Where settings are stored.
    pub fn path() -> Result<PathBuf, SettingsError> {
        config_dir()
            .map(|d| d.join("settings.json"))
            .ok_or(SettingsError::NoConfigDir)
    }

    /// Loads settings. A missing file gives the defaults; a broken file is an error.
    pub fn load_from(path: &Path) -> Result<Self, SettingsError> {
        match std::fs::read_to_string(path) {
            Ok(text) => Ok(serde_json::from_str(&text)?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e.into()),
        }
    }

    pub fn save_to(&self, path: &Path) -> Result<(), SettingsError> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, serde_json::to_string_pretty(self)? + "\n")?;
        Ok(())
    }

    /// Loads from the standard place. Apps use this at startup: any problem means defaults.
    pub fn load_or_default() -> Self {
        Self::path()
            .and_then(|p| Self::load_from(&p))
            .unwrap_or_default()
    }

    pub fn save(&self) -> Result<(), SettingsError> {
        self.save_to(&Self::path()?)
    }
}

/// The user config folder for Copal.
pub fn config_dir() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("COPAL_CONFIG_DIR") {
        return Some(PathBuf::from(dir));
    }
    if cfg!(windows) {
        return std::env::var_os("APPDATA").map(|d| PathBuf::from(d).join("Copal"));
    }
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("copal"))
}

/// Folder for data Copal keeps for the user, like the print history.
/// `COPAL_DATA_DIR` overrides it (tests and portable installs use that).
pub fn data_dir() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("COPAL_DATA_DIR") {
        return Some(PathBuf::from(dir));
    }
    if cfg!(windows) {
        return std::env::var_os("APPDATA").map(|d| PathBuf::from(d).join("Copal"));
    }
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))?;
    Some(base.join("copal"))
}

impl fmt::Display for ThemeStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Default => "default",
            Self::Art => "art",
        })
    }
}

impl FromStr for ThemeStyle {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, ()> {
        match s.to_ascii_lowercase().as_str() {
            "default" => Ok(Self::Default),
            "art" => Ok(Self::Art),
            _ => Err(()),
        }
    }
}

impl fmt::Display for ColorMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Dark => "dark",
            Self::Light => "light",
        })
    }
}

impl FromStr for ColorMode {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, ()> {
        match s.to_ascii_lowercase().as_str() {
            "dark" => Ok(Self::Dark),
            "light" => Ok(Self::Light),
            _ => Err(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_default_theme_and_dark() {
        let s = Settings::default();
        assert_eq!(s.theme, ThemeStyle::Default);
        assert_eq!(s.mode, ColorMode::Dark);
    }

    #[test]
    fn set_and_get() {
        let mut s = Settings::default();
        s.set("theme", "Art").unwrap();
        s.set("mode", "light").unwrap();
        assert_eq!(s.get("theme").unwrap(), "art");
        assert_eq!(s.get("mode").unwrap(), "light");
        assert_eq!(
            s.effective_mode(),
            ColorMode::Light,
            "art has a light version too"
        );
        s.set("gizmo", "1.5").unwrap();
        assert_eq!(s.gizmo_size, 1.5);
        assert!(s.set("gizmo", "9").is_err());
        assert!(matches!(
            s.set("theme", "neon"),
            Err(SettingsError::BadValue { .. })
        ));
        assert!(matches!(
            s.set("font", "big"),
            Err(SettingsError::UnknownKey(_))
        ));
    }

    #[test]
    fn file_round_trip_and_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested/settings.json");
        assert_eq!(Settings::load_from(&path).unwrap(), Settings::default());
        let s = Settings {
            theme: ThemeStyle::Art,
            mode: ColorMode::Light,
            gizmo_size: 1.25,
            panels: Panels {
                console: false,
                ..Panels::default()
            },
        };
        s.save_to(&path).unwrap();
        assert_eq!(Settings::load_from(&path).unwrap(), s);
    }

    #[test]
    fn unknown_fields_are_ignored_and_broken_files_fail() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        std::fs::write(&path, r#"{"theme":"art","future_option":42}"#).unwrap();
        assert_eq!(Settings::load_from(&path).unwrap().theme, ThemeStyle::Art);
        std::fs::write(&path, "{ not json").unwrap();
        assert!(matches!(
            Settings::load_from(&path),
            Err(SettingsError::Json(_))
        ));
    }
}
