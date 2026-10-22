//! Reading STL (binary and text), OBJ and 3MF files.

use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

use glam::{Mat4, Vec3};
use thiserror::Error;

use crate::Mesh;

#[derive(Debug, Error)]
pub enum ImportError {
    #[error("cannot read the file: {0}")]
    Io(#[from] std::io::Error),
    #[error("unsupported file type `{0}` (use .stl, .obj or .3mf)")]
    UnknownFormat(String),
    #[error("the file is broken: {0}")]
    Broken(String),
    #[error("the file has no triangles")]
    Empty,
}

/// Loads a model file. The format is picked by the extension.
pub fn load(path: &Path) -> Result<Mesh, ImportError> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let bytes = std::fs::read(path)?;
    let mesh = match ext.as_str() {
        "stl" => read_stl(&bytes)?,
        "obj" => read_obj(&String::from_utf8_lossy(&bytes))?,
        "3mf" => read_3mf(&bytes)?,
        other => return Err(ImportError::UnknownFormat(other.to_string())),
    };
    if mesh.triangles.is_empty() {
        return Err(ImportError::Empty);
    }
    Ok(mesh)
}

/// Joins vertices with exactly the same position, so the mesh is connected.
struct Welder {
    map: HashMap<[u32; 3], u32>,
    mesh: Mesh,
}

impl Welder {
    fn new() -> Self {
        Self {
            map: HashMap::new(),
            mesh: Mesh::default(),
        }
    }

    fn vertex(&mut self, p: Vec3) -> u32 {
        // -0.0 and 0.0 are the same point.
        let key = [p.x + 0.0, p.y + 0.0, p.z + 0.0].map(f32::to_bits);
        let next = self.mesh.positions.len() as u32;
        *self.map.entry(key).or_insert_with(|| {
            self.mesh.positions.push(p);
            next
        })
    }

    fn triangle(&mut self, a: Vec3, b: Vec3, c: Vec3) {
        if !(a.is_finite() && b.is_finite() && c.is_finite()) {
            return;
        }
        let t = [self.vertex(a), self.vertex(b), self.vertex(c)];
        // Skip triangles that collapsed to a line or a point.
        if t[0] != t[1] && t[1] != t[2] && t[0] != t[2] {
            self.mesh.triangles.push(t);
        }
    }

    fn finish(self) -> Mesh {
        self.mesh
    }
}

pub fn read_stl(bytes: &[u8]) -> Result<Mesh, ImportError> {
    let is_binary = bytes.len() >= 84 && {
        let n = u32::from_le_bytes(bytes[80..84].try_into().unwrap()) as usize;
        84usize.checked_add(n.saturating_mul(50)) == Some(bytes.len())
    };
    if is_binary {
        let n = u32::from_le_bytes(bytes[80..84].try_into().unwrap()) as usize;
        let mut w = Welder::new();
        let f = |o: usize| f32::from_le_bytes(bytes[o..o + 4].try_into().unwrap());
        for i in 0..n {
            let o = 84 + i * 50 + 12; // skip the stored normal
            let v = |k: usize| Vec3::new(f(o + k * 12), f(o + k * 12 + 4), f(o + k * 12 + 8));
            w.triangle(v(0), v(1), v(2));
        }
        return Ok(w.finish());
    }
    let text = String::from_utf8_lossy(bytes);
    if !text.trim_start().starts_with("solid") {
        return Err(ImportError::Broken("not a binary or text STL".into()));
    }
    let mut w = Welder::new();
    let mut corner = Vec::with_capacity(3);
    for line in text.lines() {
        let mut parts = line.split_whitespace();
        if parts.next() == Some("vertex") {
            let v: Vec<f32> = parts.filter_map(|p| p.parse().ok()).collect();
            if v.len() != 3 {
                return Err(ImportError::Broken(format!("bad vertex line `{line}`")));
            }
            corner.push(Vec3::new(v[0], v[1], v[2]));
            if corner.len() == 3 {
                w.triangle(corner[0], corner[1], corner[2]);
                corner.clear();
            }
        }
    }
    Ok(w.finish())
}

pub fn read_obj(text: &str) -> Result<Mesh, ImportError> {
    let mut positions: Vec<Vec3> = Vec::new();
    let mut w = Welder::new();
    for (n, line) in text.lines().enumerate() {
        let mut parts = line.split_whitespace();
        match parts.next() {
            Some("v") => {
                let v: Vec<f32> = parts.take(3).filter_map(|p| p.parse().ok()).collect();
                if v.len() != 3 {
                    return Err(ImportError::Broken(format!("line {}: bad vertex", n + 1)));
                }
                positions.push(Vec3::new(v[0], v[1], v[2]));
            }
            Some("f") => {
                let mut idx = Vec::new();
                for p in parts {
                    let first = p.split('/').next().unwrap_or("");
                    let i: i64 = first
                        .parse()
                        .map_err(|_| ImportError::Broken(format!("line {}: bad face", n + 1)))?;
                    let i = if i < 0 {
                        positions.len() as i64 + i
                    } else {
                        i - 1
                    };
                    let v = positions
                        .get(usize::try_from(i).unwrap_or(usize::MAX))
                        .copied()
                        .ok_or_else(|| {
                            ImportError::Broken(format!(
                                "line {}: face uses a missing vertex",
                                n + 1
                            ))
                        })?;
                    idx.push(v);
                }
                for k in 1..idx.len().saturating_sub(1) {
                    w.triangle(idx[0], idx[k], idx[k + 1]);
                }
            }
            _ => {}
        }
    }
    Ok(w.finish())
}

pub fn read_3mf(bytes: &[u8]) -> Result<Mesh, ImportError> {
    use quick_xml::events::Event;

    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes))
        .map_err(|e| ImportError::Broken(format!("not a 3MF archive: {e}")))?;
    let mut xml = String::new();
    let name = (0..zip.len())
        .filter_map(|i| {
            zip.by_index(i)
                .ok()
                .and_then(|f| f.name().ok().map(|n| n.to_string()))
        })
        .find(|n| n.to_ascii_lowercase().ends_with(".model"))
        .ok_or_else(|| ImportError::Broken("no 3D model inside the 3MF".into()))?;
    zip.by_name(&name)
        .map_err(|e| ImportError::Broken(e.to_string()))?
        .read_to_string(&mut xml)?;

    // Objects by id: their own triangles and their components.
    struct Obj {
        positions: Vec<Vec3>,
        triangles: Vec<[u32; 3]>,
        components: Vec<(String, Mat4)>,
    }
    let mut objects: HashMap<String, Obj> = HashMap::new();
    let mut build: Vec<(String, Mat4)> = Vec::new();
    let mut scale = 1.0f32;
    let mut current: Option<String> = None;

    let mut reader = quick_xml::Reader::from_str(&xml);
    let attr = |e: &quick_xml::events::BytesStart, key: &str| -> Option<String> {
        e.attributes()
            .flatten()
            .find(|a| a.key.local_name().as_ref() == key)
            .map(|a| a.value.to_string())
    };
    loop {
        let ev = reader
            .read_event()
            .map_err(|e| ImportError::Broken(format!("3MF XML: {e}")))?;
        match ev {
            Event::Eof => break,
            Event::Start(e) | Event::Empty(e) => match e.local_name().as_ref() {
                "model" => {
                    scale = match attr(&e, "unit").as_deref() {
                        Some("micron") => 0.001,
                        Some("centimeter") => 10.0,
                        Some("inch") => 25.4,
                        Some("foot") => 304.8,
                        Some("meter") => 1000.0,
                        _ => 1.0,
                    }
                }
                "object" => {
                    let id = attr(&e, "id").unwrap_or_default();
                    objects.insert(
                        id.clone(),
                        Obj {
                            positions: Vec::new(),
                            triangles: Vec::new(),
                            components: Vec::new(),
                        },
                    );
                    current = Some(id);
                }
                "vertex" => {
                    if let Some(o) = current.as_ref().and_then(|c| objects.get_mut(c)) {
                        let f = |k| {
                            attr(&e, k)
                                .and_then(|v| v.parse::<f32>().ok())
                                .unwrap_or(0.0)
                        };
                        o.positions.push(Vec3::new(f("x"), f("y"), f("z")) * scale);
                    }
                }
                "triangle" => {
                    if let Some(o) = current.as_ref().and_then(|c| objects.get_mut(c)) {
                        let f = |k| attr(&e, k).and_then(|v| v.parse::<u32>().ok()).unwrap_or(0);
                        o.triangles.push([f("v1"), f("v2"), f("v3")]);
                    }
                }
                "component" => {
                    if let Some(o) = current.as_ref().and_then(|c| objects.get_mut(c)) {
                        let id = attr(&e, "objectid").unwrap_or_default();
                        o.components
                            .push((id, parse_transform(attr(&e, "transform"), scale)));
                    }
                }
                "item" => {
                    let id = attr(&e, "objectid").unwrap_or_default();
                    build.push((id, parse_transform(attr(&e, "transform"), scale)));
                }
                _ => {}
            },
            Event::End(e) if e.local_name().as_ref() == "object" => current = None,
            _ => {}
        }
    }

    fn emit(objects: &HashMap<String, Obj>, id: &str, m: Mat4, w: &mut Welder, depth: u32) {
        let Some(o) = objects.get(id) else { return };
        if depth > 16 {
            return;
        }
        for t in &o.triangles {
            let p = |i: u32| o.positions.get(i as usize).map(|v| m.transform_point3(*v));
            if let (Some(a), Some(b), Some(c)) = (p(t[0]), p(t[1]), p(t[2])) {
                w.triangle(a, b, c);
            }
        }
        for (cid, cm) in &o.components {
            emit(objects, cid, m * *cm, w, depth + 1);
        }
    }
    let mut w = Welder::new();
    if build.is_empty() {
        for id in objects.keys() {
            emit(&objects, id, Mat4::IDENTITY, &mut w, 0);
        }
    }
    for (id, m) in &build {
        emit(&objects, id, *m, &mut w, 0);
    }
    Ok(w.finish())
}

/// 3MF transforms are 12 numbers: a 3×4 matrix, column by column of the rotation, then translation.
fn parse_transform(text: Option<String>, scale: f32) -> Mat4 {
    let Some(text) = text else {
        return Mat4::IDENTITY;
    };
    let v: Vec<f32> = text
        .split_whitespace()
        .filter_map(|p| p.parse().ok())
        .collect();
    if v.len() != 12 {
        return Mat4::IDENTITY;
    }
    Mat4::from_cols_array(&[
        v[0],
        v[1],
        v[2],
        0.0, //
        v[3],
        v[4],
        v[5],
        0.0, //
        v[6],
        v[7],
        v[8],
        0.0, //
        v[9] * scale,
        v[10] * scale,
        v[11] * scale,
        1.0,
    ])
}

/// Writes a binary STL (used for "export with supports" and tests).
pub fn write_stl(mesh: &Mesh) -> Vec<u8> {
    let mut out = Vec::with_capacity(84 + mesh.triangles.len() * 50);
    let mut header = [0u8; 80];
    header[..19].copy_from_slice(b"Copal binary STL   ");
    out.extend_from_slice(&header);
    out.extend_from_slice(&(mesh.triangles.len() as u32).to_le_bytes());
    for t in &mesh.triangles {
        let [a, b, c] = t.map(|i| mesh.positions[i as usize]);
        let n = (b - a).cross(c - a).normalize_or_zero();
        for v in [n, a, b, c] {
            for x in v.to_array() {
                out.extend_from_slice(&x.to_le_bytes());
            }
        }
        out.extend_from_slice(&[0, 0]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_stl_round_trip_welds_vertices() {
        let cube = Mesh::cuboid(Vec3::new(10.0, 20.0, 30.0));
        let back = read_stl(&write_stl(&cube)).unwrap();
        assert_eq!(back.triangle_count(), 12);
        assert_eq!(back.vertex_count(), 8);
        assert!((back.volume() - 6000.0).abs() < 0.01);
    }

    #[test]
    fn text_stl() {
        let text = "solid t\nfacet normal 0 0 1\nouter loop\nvertex 0 0 0\nvertex 1 0 0\nvertex 0 1 0\nendloop\nendfacet\nendsolid t\n";
        let m = read_stl(text.as_bytes()).unwrap();
        assert_eq!(m.triangle_count(), 1);
    }

    #[test]
    fn obj_with_quads_and_slashes() {
        let text =
            "v 0 0 0\nv 1 0 0\nv 1 1 0\nv 0 1 0\nvn 0 0 1\nf 1//1 2//1 3//1 4//1\nf -4 -3 -2\n";
        let m = read_obj(text).unwrap();
        assert_eq!(m.triangle_count(), 3);
        assert_eq!(m.vertex_count(), 4);
        assert!(read_obj("v 0 0 0\nf 1 2 3\n").is_err());
    }

    #[test]
    fn threemf_with_build_transform_and_units() {
        let model = r#"<?xml version="1.0" encoding="UTF-8"?>
<model unit="centimeter" xmlns="http://schemas.microsoft.com/3dmanufacturing/core/2015/02">
 <resources><object id="1" type="model"><mesh>
  <vertices><vertex x="0" y="0" z="0"/><vertex x="1" y="0" z="0"/><vertex x="0" y="1" z="0"/><vertex x="0" y="0" z="1"/></vertices>
  <triangles><triangle v1="0" v2="2" v3="1"/><triangle v1="0" v2="1" v3="3"/><triangle v1="1" v2="2" v3="3"/><triangle v1="0" v2="3" v3="2"/></triangles>
 </mesh></object></resources>
 <build><item objectid="1" transform="1 0 0 0 1 0 0 0 1 5 0 0"/></build>
</model>"#;
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        zip.start_file("3D/3dmodel.model", zip::write::SimpleFileOptions::default())
            .unwrap();
        std::io::Write::write_all(&mut zip, model.as_bytes()).unwrap();
        let bytes = zip.finish().unwrap().into_inner();
        let m = read_3mf(&bytes).unwrap();
        assert_eq!(m.triangle_count(), 4);
        let b = m.bounds().unwrap();
        assert_eq!(b.max.y, 10.0, "centimeters become millimeters");
        assert_eq!(b.min.x, 50.0, "the build transform moves the object");
    }

    #[test]
    fn unknown_extension_is_an_error() {
        let dir = std::env::temp_dir().join("copal-import-test.xyz");
        std::fs::write(&dir, b"hello").unwrap();
        assert!(matches!(load(&dir), Err(ImportError::UnknownFormat(_))));
    }
}
