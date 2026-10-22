//! Print history: every exported file, with a snapshot of the project as it was sliced.
//!
//! Each export gets its own folder in `<data dir>/history/`:
//!
//! ```text
//! 2026-10-22_09-05-42_torso/
//!     record.json     what was printed, with which settings, and how it went
//!     project.copal   the project exactly as it was sliced (open it to print again)
//!     preview.png     a picture of the plate
//! ```

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::document::Document;
use crate::project::{self, ProjectError};
use crate::time::DateTime;

#[derive(Debug, Error)]
pub enum HistoryError {
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Project(#[from] ProjectError),
    #[error("no print with id `{0}` in the history")]
    NotFound(String),
    #[error("this system has no folder for app data")]
    NoDataDir,
}

/// How a print went.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Outcome {
    Good,
    Failed,
}

impl Outcome {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "good" | "ok" | "success" => Some(Self::Good),
            "failed" | "fail" | "bad" => Some(Self::Failed),
            _ => None,
        }
    }
}

/// One exported file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrintRecord {
    /// Folder name, also the id used by `copal-cli history`.
    #[serde(default)]
    pub id: String,
    /// UTC, ISO 8601.
    pub date: String,
    pub project_name: String,
    /// Where the printer file was written.
    pub output: PathBuf,
    pub printer: Option<String>,
    pub resin: Option<String>,
    pub layers: u32,
    pub height_mm: f32,
    pub volume_ml: f32,
    pub print_time_s: f32,
    pub layer_height_mm: f32,
    pub exposure_s: f32,
    pub bottom_exposure_s: f32,
    pub bottom_layers: u32,
    /// 1 to 5 stars, set by the user after the print.
    #[serde(default)]
    pub rating: Option<u8>,
    #[serde(default)]
    pub outcome: Option<Outcome>,
    #[serde(default)]
    pub note: String,
}

impl PrintRecord {
    /// A record for a project that was just exported.
    pub fn new(doc: &Document, output: &Path) -> Self {
        let e = &doc.settings.print.exposure;
        Self {
            id: String::new(),
            date: DateTime::now().iso(),
            project_name: doc.name.clone(),
            output: output.to_path_buf(),
            printer: doc.printer.clone(),
            resin: doc.resin.clone(),
            layers: 0,
            height_mm: 0.0,
            volume_ml: 0.0,
            print_time_s: 0.0,
            layer_height_mm: e.layer_height_mm,
            exposure_s: e.exposure_s,
            bottom_exposure_s: e.bottom_exposure_s,
            bottom_layers: e.bottom_layers,
            rating: None,
            outcome: None,
            note: String::new(),
        }
    }
}

/// The history folder.
#[derive(Debug, Clone)]
pub struct PrintHistory {
    pub dir: PathBuf,
}

const RECORD: &str = "record.json";
const SNAPSHOT: &str = "project.copal";
const PREVIEW: &str = "preview.png";

fn slug(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
        .take(40)
        .collect();
    let s = s.trim_matches('_').to_string();
    if s.is_empty() { "project".into() } else { s }
}

impl PrintHistory {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    /// The history in the user's data folder.
    pub fn open_default() -> Result<Self, HistoryError> {
        crate::settings::data_dir()
            .map(|d| Self::new(d.join("history")))
            .ok_or(HistoryError::NoDataDir)
    }

    /// Adds an export: saves the record, a snapshot of the project and the preview (PNG bytes).
    pub fn add(
        &self,
        doc: &Document,
        mut record: PrintRecord,
        preview_png: Option<&[u8]>,
    ) -> Result<PrintRecord, HistoryError> {
        let stamp = DateTime::now().file_stamp();
        let base = format!("{stamp}_{}", slug(&doc.name));
        let mut id = base.clone();
        let mut n = 2;
        while self.dir.join(&id).exists() {
            id = format!("{base}_{n}");
            n += 1;
        }
        let folder = self.dir.join(&id);
        std::fs::create_dir_all(&folder)?;
        record.id = id;
        project::save(doc, &folder.join(SNAPSHOT))?;
        if let Some(png) = preview_png {
            std::fs::write(folder.join(PREVIEW), png)?;
        }
        self.write(&record)?;
        Ok(record)
    }

    fn write(&self, record: &PrintRecord) -> Result<(), HistoryError> {
        let path = self.dir.join(&record.id).join(RECORD);
        std::fs::write(path, serde_json::to_string_pretty(record)?)?;
        Ok(())
    }

    /// All records, newest first. Folders that can not be read are skipped.
    pub fn list(&self) -> Vec<PrintRecord> {
        let mut out: Vec<PrintRecord> = std::fs::read_dir(&self.dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| {
                let id = e.file_name().to_string_lossy().into_owned();
                let text = std::fs::read_to_string(e.path().join(RECORD)).ok()?;
                let mut r: PrintRecord = serde_json::from_str(&text).ok()?;
                r.id = id;
                Some(r)
            })
            .collect();
        out.sort_by(|a, b| b.id.cmp(&a.id));
        out
    }

    pub fn get(&self, id: &str) -> Result<PrintRecord, HistoryError> {
        if id.contains(['/', '\\']) || id.starts_with('.') {
            return Err(HistoryError::NotFound(id.into()));
        }
        let text = std::fs::read_to_string(self.dir.join(id).join(RECORD))
            .map_err(|_| HistoryError::NotFound(id.into()))?;
        let mut r: PrintRecord = serde_json::from_str(&text)?;
        r.id = id.to_string();
        Ok(r)
    }

    /// Changes the rating, outcome or note of a record.
    pub fn update(&self, record: &PrintRecord) -> Result<(), HistoryError> {
        self.get(&record.id)?;
        self.write(record)
    }

    pub fn snapshot_path(&self, id: &str) -> PathBuf {
        self.dir.join(id).join(SNAPSHOT)
    }

    pub fn preview_path(&self, id: &str) -> Option<PathBuf> {
        let p = self.dir.join(id).join(PREVIEW);
        p.exists().then_some(p)
    }

    /// The project as it was sliced.
    pub fn load_snapshot(&self, id: &str) -> Result<Document, HistoryError> {
        self.get(id)?;
        Ok(project::load(&self.snapshot_path(id))?)
    }

    pub fn remove(&self, id: &str) -> Result<(), HistoryError> {
        self.get(id)?;
        std::fs::remove_dir_all(self.dir.join(id))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Session;

    #[test]
    fn add_list_rate_reopen_remove() {
        let dir = tempfile::tempdir().unwrap();
        let h = PrintHistory::new(dir.path());
        let mut s = Session::new(Document::new("Torso / v2"));
        s.run_line("object.add_cube size=12").unwrap();
        let mut rec = PrintRecord::new(&s.doc, Path::new("/tmp/torso.goo"));
        rec.layers = 240;
        let a = h.add(&s.doc, rec.clone(), Some(b"png")).unwrap();
        let b = h.add(&s.doc, rec, None).unwrap();
        assert!(a.id.ends_with("_Torso___v2"), "{}", a.id);
        assert_ne!(a.id, b.id);
        let list = h.list();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].id, b.id, "newest first");

        let mut got = h.get(&a.id).unwrap();
        got.rating = Some(5);
        got.outcome = Some(Outcome::Good);
        got.note = "perfect".into();
        h.update(&got).unwrap();
        assert_eq!(h.get(&a.id).unwrap().rating, Some(5));

        // Changing the open project later does not change the snapshot.
        s.run_line("object.move id=1 x=40").unwrap();
        let snap = h.load_snapshot(&a.id).unwrap();
        assert_eq!(snap.objects()[0].transform.translation.x, 0.0);
        assert!(h.preview_path(&a.id).is_some());
        assert!(h.preview_path(&b.id).is_none());

        h.remove(&a.id).unwrap();
        assert_eq!(h.list().len(), 1);
        assert!(matches!(h.get("../x"), Err(HistoryError::NotFound(_))));
    }
}
