//! Printer and resin database, settings structs and personal presets.
//!
//! Built-in presets are the TOML files in `presets/`, compiled into the binary.
//! Personal presets live in the user config folder (`<config>/presets/<kind>/<name>.toml`)
//! and are never touched by updates.

pub mod settings;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

pub use settings::{
    ExposureSettings, HollowSettings, LayoutSettings, MotionSettings, PrintSettings,
    SupportSettings,
};

mod builtin {
    include!(concat!(env!("OUT_DIR"), "/builtin.rs"));
}

#[derive(Debug, thiserror::Error)]
pub enum PresetError {
    #[error("{0}: {1}")]
    Io(PathBuf, std::io::Error),
    #[error("{0}: {1}")]
    Parse(String, toml::de::Error),
    #[error("could not write preset: {0}")]
    Write(#[from] toml::ser::Error),
    #[error("a preset name can not be empty")]
    EmptyName,
}

/// Is a printer confirmed by a real print?
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Stable,
    #[default]
    Experimental,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Screen {
    /// Pixels, X × Y.
    pub resolution: [u32; 2],
    /// Size of the display area in mm.
    pub size_mm: [f32; 2],
    #[serde(default)]
    pub mirror_x: bool,
    #[serde(default)]
    pub mirror_y: bool,
    #[serde(default = "eight")]
    pub greyscale_bits: u8,
}

fn eight() -> u8 {
    8
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Build {
    pub height_mm: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileInfo {
    /// File extension without the dot: `goo`, `ctb`, …
    pub format: String,
    #[serde(default)]
    pub format_version: String,
    #[serde(default)]
    pub thumbnails: Vec<[u32; 2]>,
    #[serde(default = "eight")]
    pub anti_aliasing_levels: u8,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Motion {
    /// Two-stage lift and retract.
    #[serde(default)]
    pub supports_tsmc: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Printer {
    pub id: String,
    pub brand: String,
    pub model: String,
    #[serde(default)]
    pub status: Status,
    pub screen: Screen,
    pub build: Build,
    pub file: FileInfo,
    #[serde(default)]
    pub motion: Motion,
    #[serde(default)]
    pub sources: BTreeMap<String, String>,
}

impl Printer {
    pub fn name(&self) -> String {
        format!("{} {}", self.brand, self.model)
    }

    /// Size of one pixel in mm, X and Y.
    pub fn pixel_mm(&self) -> [f32; 2] {
        [
            self.screen.size_mm[0] / self.screen.resolution[0] as f32,
            self.screen.size_mm[1] / self.screen.resolution[1] as f32,
        ]
    }

    /// Build volume in mm: X, Y, Z.
    pub fn volume_mm(&self) -> [f32; 3] {
        [
            self.screen.size_mm[0],
            self.screen.size_mm[1],
            self.build.height_mm,
        ]
    }
}

/// Settings of one resin on one printer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResinProfile {
    pub printer: String,
    #[serde(flatten)]
    pub settings: PrintSettings,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Resin {
    pub id: String,
    pub brand: String,
    pub name: String,
    #[serde(default)]
    pub color: String,
    #[serde(default, rename = "type")]
    pub kind: String,
    /// For personal presets: the preset this one started from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub based_on: Option<String>,
    #[serde(default)]
    pub profiles: Vec<ResinProfile>,
}

impl Resin {
    pub fn title(&self) -> String {
        if self.color.is_empty() {
            format!("{} {}", self.brand, self.name)
        } else {
            format!("{} {} {}", self.brand, self.name, self.color)
        }
    }

    pub fn profile_for(&self, printer: &str) -> Option<&ResinProfile> {
        self.profiles.iter().find(|p| p.printer == printer)
    }
}

/// All printers and resins Copal knows about: built-in ones and the user's own.
#[derive(Debug, Clone, Default)]
pub struct Library {
    pub printers: Vec<Printer>,
    pub resins: Vec<Resin>,
}

fn parse<T: DeserializeOwned>(name: &str, text: &str) -> Result<T, PresetError> {
    toml::from_str(text).map_err(|e| PresetError::Parse(name.to_string(), e))
}

impl Library {
    /// Only the presets compiled into Copal.
    pub fn builtin() -> Self {
        let mut lib = Library::default();
        for (path, text) in builtin::BUILTIN {
            // Built-in files are checked by tests, so a parse error here is a bug in the data.
            if path.starts_with("printers/") {
                if let Ok(p) = parse::<Printer>(path, text) {
                    lib.printers.push(p);
                }
            } else if path.starts_with("resins/")
                && let Ok(r) = parse::<Resin>(path, text)
            {
                lib.resins.push(r);
            }
        }
        lib.sort();
        lib
    }

    /// Built-in presets plus personal resins from `user_dir` (see [`UserPresets`]).
    /// A personal resin with the same id as a built-in one replaces it.
    pub fn load(user: Option<&UserPresets>) -> Self {
        let mut lib = Self::builtin();
        if let Some(user) = user {
            for (_, r) in user.load_all::<Resin>(Kind::Resin) {
                lib.resins.retain(|x| x.id != r.id);
                lib.resins.push(r);
            }
        }
        lib.sort();
        lib
    }

    fn sort(&mut self) {
        self.printers.sort_by(|a, b| a.id.cmp(&b.id));
        self.resins.sort_by(|a, b| a.id.cmp(&b.id));
    }

    pub fn printer(&self, id: &str) -> Option<&Printer> {
        self.printers.iter().find(|p| p.id == id)
    }

    pub fn resin(&self, id: &str) -> Option<&Resin> {
        self.resins.iter().find(|r| r.id == id)
    }

    /// Resins that have a profile for this printer.
    pub fn resins_for<'a>(&'a self, printer: &'a str) -> impl Iterator<Item = &'a Resin> + 'a {
        self.resins
            .iter()
            .filter(move |r| r.profile_for(printer).is_some())
    }
}

/// Kinds of personal presets. Each kind is a folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    /// A whole resin with its profiles ([`Resin`]).
    Resin,
    /// Exposure and layer settings ([`ExposureSettings`]).
    Exposure,
    /// Motion, waits and anti-aliasing ([`MotionSettings`]).
    Motion,
    Supports,
    Hollow,
    Layout,
}

impl Kind {
    pub const ALL: [Kind; 6] = [
        Kind::Resin,
        Kind::Exposure,
        Kind::Motion,
        Kind::Supports,
        Kind::Hollow,
        Kind::Layout,
    ];

    pub fn folder(self) -> &'static str {
        match self {
            Kind::Resin => "resins",
            Kind::Exposure => "exposure",
            Kind::Motion => "motion",
            Kind::Supports => "supports",
            Kind::Hollow => "hollow",
            Kind::Layout => "layout",
        }
    }

    pub fn parse(s: &str) -> Option<Kind> {
        Kind::ALL
            .into_iter()
            .find(|k| k.folder() == s || k.folder().trim_end_matches('s') == s)
    }
}

/// Turns a preset name into a safe file name.
pub fn slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.trim().chars() {
        if c.is_alphanumeric() || c == '.' || c == '_' {
            out.extend(c.to_lowercase());
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').to_string()
}

/// The folder with the user's own presets.
#[derive(Debug, Clone)]
pub struct UserPresets {
    pub dir: PathBuf,
}

impl UserPresets {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    fn path(&self, kind: Kind, name: &str) -> PathBuf {
        self.dir
            .join(kind.folder())
            .join(format!("{}.toml", slug(name)))
    }

    /// Names of saved presets of one kind, sorted.
    pub fn list(&self, kind: Kind) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(self.dir.join(kind.folder()))
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| {
                let p = e.path();
                (p.extension()? == "toml").then(|| p.file_stem()?.to_str().map(String::from))?
            })
            .collect();
        names.sort();
        names
    }

    pub fn save<T: Serialize>(
        &self,
        kind: Kind,
        name: &str,
        value: &T,
    ) -> Result<PathBuf, PresetError> {
        if slug(name).is_empty() {
            return Err(PresetError::EmptyName);
        }
        let path = self.path(kind, name);
        let text = toml::to_string_pretty(value)?;
        let dir = path.parent().unwrap_or(&self.dir).to_path_buf();
        std::fs::create_dir_all(&dir).map_err(|e| PresetError::Io(dir, e))?;
        std::fs::write(&path, text).map_err(|e| PresetError::Io(path.clone(), e))?;
        Ok(path)
    }

    pub fn load<T: DeserializeOwned>(&self, kind: Kind, name: &str) -> Result<T, PresetError> {
        let path = self.path(kind, name);
        let text = std::fs::read_to_string(&path).map_err(|e| PresetError::Io(path.clone(), e))?;
        parse(&path.display().to_string(), &text)
    }

    pub fn delete(&self, kind: Kind, name: &str) -> Result<(), PresetError> {
        let path = self.path(kind, name);
        std::fs::remove_file(&path).map_err(|e| PresetError::Io(path, e))
    }

    /// Every preset of one kind that parses. Broken files are skipped.
    pub fn load_all<T: DeserializeOwned>(&self, kind: Kind) -> Vec<(String, T)> {
        self.list(kind)
            .into_iter()
            .filter_map(|n| self.load(kind, &n).ok().map(|v| (n, v)))
            .collect()
    }
}

/// Built-in presets for one kind of settings, by name (for example `light`, `medium`, `heavy`).
pub fn builtin_supports() -> [(&'static str, SupportSettings); 3] {
    [
        ("light", SupportSettings::light()),
        ("medium", SupportSettings::medium()),
        ("heavy", SupportSettings::heavy()),
    ]
}

/// The path of a file inside `presets/` as compiled in, with its text. For `copal-cli presets`.
pub fn builtin_files() -> &'static [(&'static str, &'static str)] {
    builtin::BUILTIN
}

/// Where personal presets live inside a config folder.
pub fn user_dir(config_dir: &Path) -> PathBuf {
    config_dir.join("presets")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_builtin_file_parses() {
        for (path, text) in builtin::BUILTIN {
            if path.starts_with("printers/") {
                let p: Printer = parse(path, text).unwrap();
                assert!(
                    path.ends_with(&format!("{}.toml", p.id)),
                    "{path}: id {}",
                    p.id
                );
                assert!(p.screen.resolution[0] > 0 && p.build.height_mm > 0.0);
                assert!(
                    !p.sources.is_empty(),
                    "{path}: every printer needs a source"
                );
            } else if path.starts_with("resins/") {
                let r: Resin = parse(path, text).unwrap();
                assert!(
                    path.ends_with(&format!("{}.toml", r.id)),
                    "{path}: id {}",
                    r.id
                );
                for prof in &r.profiles {
                    assert!(
                        prof.source.is_some(),
                        "{path}: every profile needs a source"
                    );
                }
            }
        }
    }

    #[test]
    fn library_has_the_test_printer_and_resin() {
        let lib = Library::builtin();
        let p = lib.printer("elegoo/saturn-4-ultra-16k").unwrap();
        assert_eq!(p.screen.resolution, [15120, 6230]);
        assert!((p.pixel_mm()[0] - 0.014).abs() < 1e-4);
        let r = lib.resin("anycubic/abs-like-pro-2-grey").unwrap();
        let prof = r.profile_for(&p.id).unwrap();
        assert_eq!(prof.settings.exposure.layer_height_mm, 0.03);
        assert_eq!(prof.settings.motion.lift_speed, [45.0, 180.0]);
        assert_eq!(lib.resins_for(&p.id).count(), 1);
    }

    #[test]
    fn user_presets_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let user = UserPresets::new(dir.path());
        let mut s = SupportSettings::heavy();
        s.overhang_angle_deg = 50.0;
        user.save(Kind::Supports, "My Heavy / big", &s).unwrap();
        assert_eq!(user.list(Kind::Supports), vec!["my-heavy-big"]);
        let back: SupportSettings = user.load(Kind::Supports, "my-heavy-big").unwrap();
        assert_eq!(back, s);
        user.delete(Kind::Supports, "my-heavy-big").unwrap();
        assert!(user.list(Kind::Supports).is_empty());
        assert!(matches!(
            user.save(Kind::Supports, " / ", &s),
            Err(PresetError::EmptyName)
        ));
    }

    #[test]
    fn user_resin_replaces_builtin() {
        let dir = tempfile::tempdir().unwrap();
        let user = UserPresets::new(dir.path());
        let mut r = Library::builtin()
            .resin("anycubic/abs-like-pro-2-grey")
            .unwrap()
            .clone();
        r.profiles[0].settings.exposure.exposure_s = 1.7;
        user.save(Kind::Resin, "abs-like-pro-2-grey", &r).unwrap();
        let lib = Library::load(Some(&user));
        let got = lib.resin("anycubic/abs-like-pro-2-grey").unwrap();
        assert_eq!(got.profiles[0].settings.exposure.exposure_s, 1.7);
        assert_eq!(lib.resins.iter().filter(|r| r.id == got.id).count(), 1);
    }

    #[test]
    fn kinds_parse() {
        assert_eq!(Kind::parse("resin"), Some(Kind::Resin));
        assert_eq!(Kind::parse("supports"), Some(Kind::Supports));
        assert_eq!(Kind::parse("support"), Some(Kind::Supports));
        assert_eq!(Kind::parse("hollow"), Some(Kind::Hollow));
        assert_eq!(slug("ABS-Like Pro 2 (grey)"), "abs-like-pro-2-grey");
    }
}
