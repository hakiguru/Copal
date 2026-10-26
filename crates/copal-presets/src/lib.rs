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
    #[error("{0}: only Copal resin presets (.toml) can be imported")]
    NotToml(PathBuf),
    #[error("{0}: the resin has no settings (add a [[profiles]] section with a printer)")]
    NoProfiles(String),
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
    /// `brand/name`. Personal resins without an id get `my/<file name>`.
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub brand: String,
    /// Personal resins without a name get their file name.
    #[serde(default)]
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
        Self::load_checked(user).0
    }

    /// Like [`Library::load`], plus a message for every personal resin file that could not be
    /// read, so the user learns why a file they dropped in the folder does not show up.
    pub fn load_checked(user: Option<&UserPresets>) -> (Self, Vec<String>) {
        let mut lib = Self::builtin();
        let mut problems = Vec::new();
        if let Some(user) = user {
            let (resins, bad) = user.load_resins();
            problems = bad;
            for r in resins {
                lib.resins.retain(|x| x.id != r.id);
                lib.resins.push(r);
            }
        }
        lib.sort();
        (lib, problems)
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

    /// The folder of one kind of presets (it may not exist yet).
    pub fn folder(&self, kind: Kind) -> PathBuf {
        self.dir.join(kind.folder())
    }

    /// Creates the folder of one kind and, for resins, a short README with the file format, so
    /// someone who opens the folder knows what to put there. Returns the folder.
    pub fn ensure_folder(&self, kind: Kind) -> Result<PathBuf, PresetError> {
        let dir = self.folder(kind);
        std::fs::create_dir_all(&dir).map_err(|e| PresetError::Io(dir.clone(), e))?;
        if kind == Kind::Resin {
            let readme = dir.join("README.txt");
            if !readme.exists() {
                std::fs::write(&readme, RESIN_README).map_err(|e| PresetError::Io(readme, e))?;
            }
        }
        Ok(dir)
    }

    /// Every `.toml` file in the resin folder, as it is named on disk, with what it holds.
    pub fn resin_files(&self) -> Vec<(PathBuf, Result<Resin, String>)> {
        let mut files: Vec<PathBuf> = std::fs::read_dir(self.folder(Kind::Resin))
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                p.extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("toml"))
            })
            .collect();
        files.sort();
        files
            .into_iter()
            .map(|path| {
                let stem = path
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let r = std::fs::read_to_string(&path)
                    .map_err(|e| e.to_string())
                    .and_then(|text| parse_resin(&stem, &text).map_err(|e| e.to_string()));
                (path, r)
            })
            .collect()
    }

    /// All personal resins, and a message for each file that could not be read.
    /// Missing ids, brands and names are filled in from the file name.
    pub fn load_resins(&self) -> (Vec<Resin>, Vec<String>) {
        let mut resins = Vec::new();
        let mut problems = Vec::new();
        for (path, r) in self.resin_files() {
            match r {
                Ok(r) => resins.push(r),
                Err(e) => {
                    let file = path
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    // Parse errors already name the file.
                    problems.push(if e.starts_with(&file) || e.contains(".toml") {
                        e
                    } else {
                        format!("{file}: {e}")
                    });
                }
            }
        }
        (resins, problems)
    }

    /// Deletes the file of a personal resin, found by its id. Returns the deleted file.
    pub fn delete_resin(&self, id: &str) -> Result<PathBuf, PresetError> {
        let path = self
            .resin_files()
            .into_iter()
            .find(|(_, r)| r.as_ref().is_ok_and(|r| r.id == id))
            .map(|(p, _)| p)
            .ok_or_else(|| {
                PresetError::Io(
                    self.folder(Kind::Resin).join(id),
                    std::io::ErrorKind::NotFound.into(),
                )
            })?;
        std::fs::remove_file(&path).map_err(|e| PresetError::Io(path.clone(), e))?;
        Ok(path)
    }

    /// Copies a resin preset file into the resin folder after checking that it reads. The file
    /// keeps its name (and its comments). Returns the resin and where it went.
    pub fn import_resin(&self, file: &Path) -> Result<(Resin, PathBuf), PresetError> {
        if !file
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("toml"))
        {
            return Err(PresetError::NotToml(file.to_path_buf()));
        }
        let text =
            std::fs::read_to_string(file).map_err(|e| PresetError::Io(file.to_path_buf(), e))?;
        let stem = file
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let resin = parse_resin(&stem, &text)?;
        if slug(&stem).is_empty() {
            return Err(PresetError::EmptyName);
        }
        let dir = self.ensure_folder(Kind::Resin)?;
        let dest = dir.join(format!("{}.toml", slug(&stem)));
        if dest != file {
            std::fs::write(&dest, &text).map_err(|e| PresetError::Io(dest.clone(), e))?;
        }
        Ok((resin, dest))
    }

    /// Changes whenever a file in the folder of `kind` is added, removed or edited.
    /// Cheap enough to call every second or two.
    pub fn signature(&self, kind: Kind) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        let mut entries: Vec<(String, u64, Option<std::time::SystemTime>)> =
            std::fs::read_dir(self.folder(kind))
                .into_iter()
                .flatten()
                .flatten()
                .filter_map(|e| {
                    let m = e.metadata().ok()?;
                    Some((
                        e.file_name().to_string_lossy().into_owned(),
                        m.len(),
                        m.modified().ok(),
                    ))
                })
                .collect();
        entries.sort();
        entries.hash(&mut h);
        h.finish()
    }

    /// Every preset of one kind that parses. Broken files are skipped.
    pub fn load_all<T: DeserializeOwned>(&self, kind: Kind) -> Vec<(String, T)> {
        self.list(kind)
            .into_iter()
            .filter_map(|n| self.load(kind, &n).ok().map(|v| (n, v)))
            .collect()
    }
}

/// Reads a personal resin file. `stem` is its file name without `.toml`.
fn parse_resin(stem: &str, text: &str) -> Result<Resin, PresetError> {
    let mut r: Resin = parse(&format!("{stem}.toml"), text)?;
    if r.name.trim().is_empty() {
        r.name = stem.to_string();
    }
    if r.brand.trim().is_empty() {
        r.brand = "My".into();
    }
    if r.id.trim().is_empty() {
        r.id = format!("my/{}", slug(stem));
    }
    if r.profiles.is_empty() {
        return Err(PresetError::NoProfiles(r.title()));
    }
    Ok(r)
}

/// What the README in the personal resin folder says.
pub const RESIN_README: &str = include_str!("resin-readme.txt");

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
    fn dropped_and_imported_resin_files() {
        let dir = tempfile::tempdir().unwrap();
        let user = UserPresets::new(dir.path().join("presets"));
        let folder = user.ensure_folder(Kind::Resin).unwrap();
        assert!(folder.join("README.txt").exists());
        // A minimal hand-written file dropped into the folder.
        std::fs::write(
            folder.join("Grey 30um.toml"),
            "[[profiles]]\nprinter = \"elegoo/saturn-4-ultra-16k\"\nexposure_s = 1.6\n",
        )
        .unwrap();
        std::fs::write(folder.join("broken.toml"), "brand = \n").unwrap();
        std::fs::write(folder.join("empty.toml"), "name = \"Nothing\"\n").unwrap();
        let (lib, problems) = Library::load_checked(Some(&user));
        let r = lib.resin("my/grey-30um").expect("dropped file is loaded");
        assert_eq!(r.brand, "My");
        assert_eq!(r.name, "Grey 30um");
        let p = r.profile_for("elegoo/saturn-4-ultra-16k").unwrap();
        assert_eq!(p.settings.exposure.exposure_s, 1.6);
        assert_eq!(
            p.settings.exposure.layer_height_mm,
            ExposureSettings::default().layer_height_mm
        );
        assert_eq!(problems.len(), 2, "{problems:?}");
        assert!(problems.iter().any(|m| m.starts_with("broken.toml")));
        assert!(problems.iter().any(|m| m.contains("no settings")));

        // Import copies a file from elsewhere, keeps its text, refuses other formats.
        let src = dir.path().join("Siraya Fast.toml");
        std::fs::write(
            &src,
            "# shared by a friend\nid = \"siraya/fast-grey\"\nbrand = \"Siraya Tech\"\nname = \"Fast\"\n[[profiles]]\nprinter = \"elegoo/saturn-4-ultra-16k\"\n",
        )
        .unwrap();
        let before = user.signature(Kind::Resin);
        let (r, dest) = user.import_resin(&src).unwrap();
        assert_eq!(r.id, "siraya/fast-grey");
        assert_eq!(dest, folder.join("siraya-fast.toml"));
        assert!(
            std::fs::read_to_string(&dest)
                .unwrap()
                .starts_with("# shared by a friend")
        );
        assert_ne!(user.signature(Kind::Resin), before);
        assert!(
            Library::load(Some(&user))
                .resin("siraya/fast-grey")
                .is_some()
        );
        assert_eq!(
            user.delete_resin("my/grey-30um").unwrap(),
            folder.join("Grey 30um.toml")
        );
        assert!(Library::load(Some(&user)).resin("my/grey-30um").is_none());
        let lyr = dir.path().join("x.lyr");
        std::fs::write(&lyr, "x").unwrap();
        assert!(matches!(
            user.import_resin(&lyr),
            Err(PresetError::NotToml(_))
        ));
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
