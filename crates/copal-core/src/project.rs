//! The `.copal` project file: a zip archive with `project.json` and one binary file per mesh.

use std::collections::BTreeSet;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Seek, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use copal_mesh::{Mesh, MeshError};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::document::{Document, ObjectId, ProjectSettings, SceneObject, Transform};

/// Version of the `project.json` layout. Bump it and add a migration when the layout changes.
///
/// - 1: name, printer, resin, objects.
/// - 2: adds `settings` (print, supports, hollow, layout). Version 1 files get the defaults.
/// - 3: objects may have `supports`, `holes` and a `cavity` mesh (`meshes/<id>.cavity.bin`).
pub const FORMAT_VERSION: u32 = 3;
const FORMAT_NAME: &str = "copal-project";
const PROJECT_JSON: &str = "project.json";

#[derive(Debug, Error)]
pub enum ProjectError {
    #[error("cannot read or write the file: {0}")]
    Io(#[from] std::io::Error),
    #[error("the file is not a valid Copal project archive: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("project.json is broken: {0}")]
    Json(#[from] serde_json::Error),
    #[error("this is not a Copal project")]
    NotAProject,
    #[error(
        "this project was made by a newer Copal (format {found}, this version reads up to {FORMAT_VERSION}). Please update Copal."
    )]
    TooNew { found: u32 },
    #[error("mesh `{path}` is broken: {source}")]
    Mesh { path: String, source: MeshError },
    #[error("the project is inconsistent: {0}")]
    Invalid(String),
}

#[derive(Serialize, Deserialize)]
struct ProjectJson {
    format: String,
    version: u32,
    /// Copal version that wrote the file, for bug reports.
    written_by: String,
    name: String,
    printer: Option<String>,
    resin: Option<String>,
    #[serde(default)]
    settings: ProjectSettings,
    next_id: u64,
    objects: Vec<ObjectJson>,
}

#[derive(Serialize, Deserialize)]
struct ObjectJson {
    id: ObjectId,
    name: String,
    transform: Transform,
    mesh: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    supports: Option<copal_supports::SupportSet>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    holes: Vec<copal_supports::DrainHole>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cavity: Option<String>,
}

fn mesh_path(id: ObjectId) -> String {
    format!("meshes/{id}.bin")
}

fn cavity_path(id: ObjectId) -> String {
    format!("meshes/{id}.cavity.bin")
}

fn read_mesh<R: Read + Seek>(zip: &mut ZipArchive<R>, path: &str) -> Result<Mesh, ProjectError> {
    let mut bytes = Vec::new();
    zip.by_name(path)
        .map_err(|_| ProjectError::Invalid(format!("mesh `{path}` is missing")))?
        .read_to_end(&mut bytes)?;
    Mesh::from_bytes(&bytes).map_err(|source| ProjectError::Mesh {
        path: path.to_string(),
        source,
    })
}

/// Writes the project to any seekable writer.
pub fn write_to<W: Write + Seek>(doc: &Document, writer: W) -> Result<(), ProjectError> {
    let mut zip = ZipWriter::new(writer);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let json = ProjectJson {
        format: FORMAT_NAME.into(),
        version: FORMAT_VERSION,
        written_by: format!("Copal {}", env!("CARGO_PKG_VERSION")),
        name: doc.name.clone(),
        printer: doc.printer.clone(),
        resin: doc.resin.clone(),
        settings: doc.settings,
        next_id: doc.next_id,
        objects: doc
            .objects()
            .iter()
            .map(|o| ObjectJson {
                id: o.id,
                name: o.name.clone(),
                transform: o.transform,
                mesh: mesh_path(o.id),
                supports: o.prep.supports.as_deref().cloned(),
                holes: o.prep.holes.clone(),
                cavity: o.prep.cavity.as_ref().map(|_| cavity_path(o.id)),
            })
            .collect(),
    };
    zip.start_file(PROJECT_JSON, options)?;
    serde_json::to_writer_pretty(&mut zip, &json)?;
    for o in doc.objects() {
        zip.start_file(mesh_path(o.id), options)?;
        zip.write_all(&o.mesh.to_bytes())?;
        if let Some(c) = &o.prep.cavity {
            zip.start_file(cavity_path(o.id), options)?;
            zip.write_all(&c.to_bytes())?;
        }
    }
    zip.finish()?;
    Ok(())
}

/// Reads a project from any seekable reader.
pub fn read_from<R: Read + Seek>(reader: R) -> Result<Document, ProjectError> {
    let mut zip = ZipArchive::new(reader)?;
    let json: ProjectJson = {
        let file = zip
            .by_name(PROJECT_JSON)
            .map_err(|_| ProjectError::NotAProject)?;
        serde_json::from_reader(file)?
    };
    if json.format != FORMAT_NAME {
        return Err(ProjectError::NotAProject);
    }
    if json.version > FORMAT_VERSION {
        return Err(ProjectError::TooNew {
            found: json.version,
        });
    }

    let mut seen = BTreeSet::new();
    let mut objects = Vec::with_capacity(json.objects.len());
    for o in json.objects {
        if !seen.insert(o.id) {
            return Err(ProjectError::Invalid(format!(
                "object id {} is used twice",
                o.id
            )));
        }
        if o.id.0 >= json.next_id {
            return Err(ProjectError::Invalid(format!(
                "object id {} is not below next_id",
                o.id
            )));
        }
        let mesh = read_mesh(&mut zip, &o.mesh)?;
        let cavity = match &o.cavity {
            Some(p) => Some(Arc::new(read_mesh(&mut zip, p)?)),
            None => None,
        };
        let mut obj = SceneObject::new(o.id, o.name, o.transform, Arc::new(mesh));
        obj.prep.supports = o.supports.map(Arc::new);
        obj.prep.holes = o.holes;
        obj.prep.cavity = cavity;
        objects.push(obj);
    }

    Ok(Document {
        name: json.name,
        printer: json.printer,
        resin: json.resin,
        settings: json.settings,
        objects,
        next_id: json.next_id,
    })
}

/// Saves to `path` safely: writes a temporary file next to it, then renames it over the target.
pub fn save(doc: &Document, path: &Path) -> Result<(), ProjectError> {
    let tmp = temp_path(path);
    let result = (|| {
        let mut writer = BufWriter::new(File::create(&tmp)?);
        write_to(doc, &mut writer)?;
        writer
            .into_inner()
            .map_err(|e| e.into_error())?
            .sync_all()?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

pub fn load(path: &Path) -> Result<Document, ProjectError> {
    read_from(BufReader::new(File::open(path)?))
}

fn temp_path(path: &Path) -> PathBuf {
    let mut name = path
        .file_name()
        .map(|n| n.to_os_string())
        .unwrap_or_default();
    name.push(".tmp");
    path.with_file_name(name)
}
