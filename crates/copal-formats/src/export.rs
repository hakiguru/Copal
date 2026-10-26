//! Slices a project and writes the printer file.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use copal_core::presets::{PrintSettings, Printer};
use copal_core::time::DateTime;
use copal_core::{Document, library};
use copal_slice::{Raster, Slicer};
use glam::Vec3;
use rayon::prelude::*;

use crate::goo::{self, Header, LayerDef, LayerEncoder, Preview};
use crate::thumbnail;

/// Typical resin density, g/ml, for the weight estimate.
pub const RESIN_DENSITY: f32 = 1.1;

#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("choose a printer first (project.set_printer)")]
    NoPrinter,
    #[error("unknown printer `{0}`")]
    UnknownPrinter(String),
    #[error("Copal can not write `.{0}` files yet")]
    UnsupportedFormat(String),
    #[error("there is nothing to slice: the plate is empty")]
    Empty,
    #[error("`{0}` is outside the build plate")]
    OutsidePlate(String),
    #[error("`{0}` is taller than the printer can build")]
    TooTall(String),
    #[error("`{0}` floats above the plate without supports: add supports or drop it to the plate")]
    Floating(String),
    #[error("cancelled")]
    Cancelled,
    #[error("could not write the file: {0}")]
    Io(#[from] std::io::Error),
}

/// What was written.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportReport {
    pub path: PathBuf,
    pub layers: u32,
    pub height_mm: f32,
    pub volume_ml: f32,
    pub weight_g: f32,
    pub print_time_s: f32,
    pub file_bytes: u64,
    /// The big preview as RGB, for the print history.
    pub preview_rgb: Vec<u8>,
    pub preview_size: u32,
}

/// Every triangle of every object, in plate coordinates.
pub fn scene_triangles(doc: &Document) -> Vec<[Vec3; 3]> {
    let mut out = Vec::new();
    for o in doc.objects() {
        object_triangles(o, &mut out);
    }
    out
}

fn mesh_triangles(mesh: &copal_core::supports::geometry::SupportMesh, out: &mut Vec<[Vec3; 3]>) {
    let m = &mesh.mesh;
    out.extend(
        m.triangles
            .iter()
            .map(|t| t.map(|i| m.positions[i as usize])),
    );
}

/// Everything one object adds to the slices: the model, its hollow inside (inside out), drain
/// holes (inside out) and its supports with the raft.
pub fn object_triangles(o: &copal_core::SceneObject, out: &mut Vec<[Vec3; 3]>) {
    let m = o.transform.matrix();
    let flip = m.determinant() < 0.0;
    let mut add = |mesh: &copal_core::supports::geometry::SupportMesh, placed: bool| {
        if placed {
            mesh_triangles(mesh, out);
            return;
        }
        let p: Vec<Vec3> = mesh
            .mesh
            .positions
            .iter()
            .map(|v| m.transform_point3(*v))
            .collect();
        out.extend(mesh.mesh.triangles.iter().map(|t| {
            let (a, b, c) = (p[t[0] as usize], p[t[1] as usize], p[t[2] as usize]);
            // A mirrored object turns its triangles inside out; turn them back.
            if flip { [a, c, b] } else { [a, b, c] }
        }));
    };
    let wrap = |mesh: copal_core::supports::Mesh| copal_core::supports::geometry::SupportMesh {
        mesh,
        ranges: Vec::new(),
    };
    add(&wrap((*o.mesh).clone()), false);
    if let Some(c) = &o.prep.cavity {
        add(&wrap((**c).clone()), false);
    }
    for h in &o.prep.holes {
        let point = m.transform_point3(h.point);
        let dir = m.transform_vector3(h.dir).normalize_or(Vec3::NEG_Z);
        let hole = copal_core::supports::hollow::drain_hole(point, dir, h.diameter, h.depth);
        add(&wrap(hole), true);
    }
    if let Some(s) = &o.prep.supports {
        add(
            &copal_core::supports::build_mesh(s, o.transform.translation),
            true,
        );
    }
}

/// Bounds of what an object prints, supports and raft included.
pub fn object_print_bounds(o: &copal_core::SceneObject) -> Option<(Vec3, Vec3)> {
    let mut b = o.world_bounds().map(|b| (b.min, b.max))?;
    if let Some(s) = &o.prep.supports {
        let mesh = copal_core::supports::build_mesh(s, o.transform.translation);
        if let Some(sb) = mesh.mesh.bounds() {
            b = (b.0.min(sb.min), b.1.max(sb.max));
        }
    }
    Some(b)
}

/// The printer of the project, or an error saying what is missing.
pub fn project_printer(doc: &Document) -> Result<Printer, ExportError> {
    let id = doc.printer.as_deref().ok_or(ExportError::NoPrinter)?;
    library()
        .printer(id)
        .cloned()
        .ok_or_else(|| ExportError::UnknownPrinter(id.to_string()))
}

/// Checks that every object fits on the plate.
pub fn check_fits(doc: &Document, printer: &Printer) -> Result<(), ExportError> {
    if doc.objects().is_empty() {
        return Err(ExportError::Empty);
    }
    let [w, d, h] = printer.volume_mm();
    let eps = 1e-3;
    for o in doc.objects() {
        let Some((min, max)) = object_print_bounds(o) else {
            continue;
        };
        if min.x < -w / 2.0 - eps
            || max.x > w / 2.0 + eps
            || min.y < -d / 2.0 - eps
            || max.y > d / 2.0 + eps
        {
            return Err(ExportError::OutsidePlate(o.name.clone()));
        }
        if !o.prep.has_supports() && min.z > 0.05 {
            return Err(ExportError::Floating(o.name.clone()));
        }
        if max.z > h + eps {
            return Err(ExportError::TooTall(o.name.clone()));
        }
    }
    Ok(())
}

/// The slicer for a project, ready to render layers (also used by the layer preview).
pub fn slicer_for(doc: &Document, printer: &Printer, settings: &PrintSettings) -> Slicer {
    let mut raster = Raster::new(printer.screen.resolution, printer.screen.size_mm);
    raster.mirror_x = printer.screen.mirror_x;
    raster.mirror_y = printer.screen.mirror_y;
    Slicer::new(
        scene_triangles(doc),
        raster,
        settings.exposure.layer_height_mm,
        settings.motion.anti_aliasing as u32,
    )
}

fn layer_def(s: &PrintSettings, i: u32) -> LayerDef {
    let e = &s.exposure;
    let m = &s.motion;
    let bottom = e.is_bottom(i);
    let pair = |d: [f32; 2], v: [f32; 2]| [(d[0], v[0]), (d[1], v[1])];
    LayerDef {
        pause: false,
        pause_z_mm: 0.0,
        z_mm: (i + 1) as f32 * e.layer_height_mm,
        exposure_s: e.exposure_of(i),
        turn_off_time_s: 0.0,
        before_lift_s: if bottom {
            m.bottom_rest_after_exposure_s
        } else {
            m.rest_after_exposure_s
        },
        after_lift_s: 0.0,
        after_retract_s: if bottom {
            m.bottom_rest_after_retract_s
        } else {
            m.rest_after_retract_s
        },
        lift: if bottom {
            pair(m.bottom_lift_mm, m.bottom_lift_speed)
        } else {
            pair(m.lift_mm, m.lift_speed)
        },
        retract: if bottom {
            pair(m.bottom_retract_mm, m.bottom_retract_speed)
        } else {
            pair(m.retract_mm, m.retract_speed)
        },
        pwm: e.light_pwm as u16,
    }
}

/// Name of the settings in the file header (32 bytes at most), e.g. `Anycubic ABS-Like Pro 2 30u`.
pub fn profile_name(doc: &Document) -> String {
    let microns = (doc.settings.print.exposure.layer_height_mm * 1000.0).round() as u32;
    let lib = library();
    let names: Vec<String> = match doc.resin.as_deref().and_then(|id| lib.resin(id)) {
        Some(r) => vec![
            r.title(),
            format!("{} {}", r.brand, r.name),
            format!("{} {}", r.name, r.color),
            r.name.clone(),
        ],
        None => vec!["Custom".into()],
    };
    names
        .iter()
        .map(|n| format!("{} {microns}u", n.trim()))
        .find(|n| n.len() <= 32)
        .unwrap_or_else(|| format!("{microns}u"))
}

/// Slices `doc` and writes a `.goo` file to `path`.
///
/// `progress` gets values from 0 to 1. Setting `cancel` stops the export and removes the
/// half-written file.
pub fn export(
    doc: &Document,
    path: &Path,
    progress: &(dyn Fn(f32) + Sync),
    cancel: &AtomicBool,
) -> Result<ExportReport, ExportError> {
    let printer = project_printer(doc)?;
    if printer.file.format != "goo" {
        return Err(ExportError::UnsupportedFormat(printer.file.format.clone()));
    }
    check_fits(doc, &printer)?;
    let settings = doc.settings.print;
    let slicer = slicer_for(doc, &printer, &settings);
    let layers = slicer.layer_count();
    let tris = scene_triangles(doc);
    let style = thumbnail::PreviewStyle::default();
    let big = thumbnail::render(&tris, goo::BIG_PREVIEW, goo::BIG_PREVIEW, style);
    let small = thumbnail::render(&tris, goo::SMALL_PREVIEW, goo::SMALL_PREVIEW, style);

    let tmp = path.with_extension("goo.part");
    let result = (|| -> Result<(f64, u64), ExportError> {
        let mut file = BufWriter::with_capacity(1 << 20, File::create(&tmp)?);
        let mut header = make_header(doc, &printer, &settings, layers);
        header.small_preview = Preview::from_rgb(goo::SMALL_PREVIEW, goo::SMALL_PREVIEW, &small);
        header.big_preview = Preview::from_rgb(goo::BIG_PREVIEW, goo::BIG_PREVIEW, &big);
        // Volume and time are known only after slicing: write the header, patch it at the end.
        goo::write_header(&mut file, &header)?;
        let mut lit = 0f64;
        let batch = (rayon::current_num_threads() * 2).max(4) as u32;
        let mut start = 0;
        while start < layers {
            if cancel.load(Ordering::Relaxed) {
                return Err(ExportError::Cancelled);
            }
            let end = (start + batch).min(layers);
            let encoded: Vec<(Vec<u8>, f64)> = (start..end)
                .into_par_iter()
                .map(|i| {
                    let mut enc = LayerEncoder::new();
                    let stats = slicer.render_layer(i, &mut enc);
                    (enc.finish(), stats.lit_pixels)
                })
                .collect();
            for (k, (data, px)) in encoded.into_iter().enumerate() {
                lit += px;
                goo::write_layer(&mut file, &layer_def(&settings, start + k as u32), &data)?;
            }
            start = end;
            progress(start as f32 / layers.max(1) as f32);
        }
        goo::write_ending(&mut file)?;
        let mut file = file.into_inner().map_err(|e| e.into_error())?;
        // Patch the totals into the header.
        let px = printer.pixel_mm();
        let volume_mm3 = lit * (px[0] * px[1] * settings.exposure.layer_height_mm) as f64;
        header.volume_mm3 = volume_mm3 as f32;
        header.weight_g = (volume_mm3 / 1000.0) as f32 * RESIN_DENSITY;
        header.print_time_s = settings.estimate_time_s(layers) as u32;
        use std::io::Seek;
        file.seek(std::io::SeekFrom::Start(0))?;
        let mut w = BufWriter::new(&mut file);
        goo::write_header(&mut w, &header)?;
        w.flush()?;
        drop(w);
        file.sync_all()?;
        let bytes = file.metadata()?.len();
        Ok((volume_mm3, bytes))
    })();
    match result {
        Ok((volume_mm3, file_bytes)) => {
            std::fs::rename(&tmp, path)?;
            let volume_ml = (volume_mm3 / 1000.0) as f32;
            Ok(ExportReport {
                path: path.to_path_buf(),
                layers,
                height_mm: layers as f32 * settings.exposure.layer_height_mm,
                volume_ml,
                weight_g: volume_ml * RESIN_DENSITY,
                print_time_s: settings.estimate_time_s(layers),
                file_bytes,
                preview_rgb: big,
                preview_size: goo::BIG_PREVIEW,
            })
        }
        Err(e) => {
            let _ = std::fs::remove_file(&tmp);
            Err(e)
        }
    }
}

fn make_header(doc: &Document, printer: &Printer, s: &PrintSettings, layers: u32) -> Header {
    let e = &s.exposure;
    let m = &s.motion;
    let aa = m.anti_aliasing.max(1) as u16;
    let name = printer.name();
    Header {
        file_time: DateTime::now().iso(),
        printer_name: name.clone(),
        printer_type: name,
        profile_name: profile_name(doc),
        anti_aliasing_level: aa,
        grey_level: (aa > 1) as u16,
        blur_level: (aa > 1) as u16,
        layer_count: layers,
        resolution: [
            printer.screen.resolution[0] as u16,
            printer.screen.resolution[1] as u16,
        ],
        mirror_x: printer.screen.mirror_x,
        mirror_y: printer.screen.mirror_y,
        size_mm: printer.volume_mm(),
        layer_height_mm: e.layer_height_mm,
        exposure_s: e.exposure_s,
        delay_mode: true,
        turn_off_time_s: 0.0,
        bottom_before_lift_s: m.bottom_rest_after_exposure_s,
        bottom_after_lift_s: 0.0,
        bottom_after_retract_s: m.bottom_rest_after_retract_s,
        before_lift_s: m.rest_after_exposure_s,
        after_lift_s: 0.0,
        after_retract_s: m.rest_after_retract_s,
        bottom_exposure_s: e.bottom_exposure_s,
        bottom_layers: e.bottom_layers,
        bottom_lift: [
            (m.bottom_lift_mm[0], m.bottom_lift_speed[0]),
            (m.bottom_lift_mm[1], m.bottom_lift_speed[1]),
        ],
        lift: [
            (m.lift_mm[0], m.lift_speed[0]),
            (m.lift_mm[1], m.lift_speed[1]),
        ],
        bottom_retract: [
            (m.bottom_retract_mm[0], m.bottom_retract_speed[0]),
            (m.bottom_retract_mm[1], m.bottom_retract_speed[1]),
        ],
        retract: [
            (m.retract_mm[0], m.retract_speed[0]),
            (m.retract_mm[1], m.retract_speed[1]),
        ],
        bottom_pwm: e.light_pwm as u16,
        pwm: e.light_pwm as u16,
        advance_mode: false,
        grey_scale: aa > 1,
        transition_layers: e.transition_layers as u16,
        ..Header::default()
    }
}

/// A file name for the export of a project: `<project>.<ext>` with unsafe characters replaced.
pub fn default_file_name(doc: &Document, printer: &Printer) -> String {
    let stem: String = doc
        .name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || "-_ .".contains(c) {
                c
            } else {
                '_'
            }
        })
        .collect();
    let stem = stem.trim();
    format!(
        "{}.{}",
        if stem.is_empty() { "copal" } else { stem },
        printer.file.format
    )
}

/// Encodes an RGB image as PNG.
pub fn png(rgb: &[u8], width: u32, height: u32) -> Vec<u8> {
    let mut out = Vec::new();
    let enc = image::codecs::png::PngEncoder::new(&mut out);
    use image::ImageEncoder;
    if enc
        .write_image(rgb, width, height, image::ExtendedColorType::Rgb8)
        .is_err()
    {
        out.clear();
    }
    out
}

/// Exports and, when `history` is given, saves a record with a snapshot of the project.
pub fn export_and_record(
    doc: &Document,
    path: &Path,
    progress: &(dyn Fn(f32) + Sync),
    cancel: &AtomicBool,
    history: Option<&copal_core::prints::PrintHistory>,
) -> Result<(ExportReport, Option<copal_core::prints::PrintRecord>), ExportError> {
    let report = export(doc, path, progress, cancel)?;
    let record = match history {
        None => None,
        Some(h) => {
            let mut r = copal_core::prints::PrintRecord::new(doc, &report.path);
            r.layers = report.layers;
            r.height_mm = report.height_mm;
            r.volume_ml = report.volume_ml;
            r.print_time_s = report.print_time_s;
            let preview = png(
                &report.preview_rgb,
                report.preview_size,
                report.preview_size,
            );
            // The file is written already; a history problem must not fail the export.
            h.add(doc, r, Some(&preview)).ok()
        }
    };
    Ok((report, record))
}

#[cfg(test)]
mod tests {
    use super::*;
    use copal_core::Session;
    use std::io::BufReader;

    #[test]
    fn exports_a_cube_that_reads_back() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = Session::new(Document::new("cube test"));
        s.run_script(
            "project.set_printer printer=elegoo/saturn-4-ultra-16k\n\
             project.set_resin resin=anycubic/abs-like-pro-2-grey\n\
             print.set layer_height_mm=0.05 anti_aliasing=1\n\
             object.add_cube size=2",
        )
        .unwrap();
        let path = dir.path().join("cube.goo");
        let history = copal_core::prints::PrintHistory::new(dir.path().join("history"));
        let (report, record) = export_and_record(
            &s.doc,
            &path,
            &|_| {},
            &AtomicBool::new(false),
            Some(&history),
        )
        .unwrap();
        assert_eq!(report.layers, 40);
        assert!(
            (report.volume_ml - 0.008).abs() < 0.0005,
            "{}",
            report.volume_ml
        );
        assert!(!dir.path().join("cube.goo.part").exists());
        let record = record.unwrap();
        assert_eq!(history.list().len(), 1);
        assert_eq!(history.load_snapshot(&record.id).unwrap(), s.doc);

        let mut f = crate::goo::GooFile::open(BufReader::new(File::open(&path).unwrap())).unwrap();
        assert_eq!(f.header.layer_count, 40);
        assert_eq!(f.header.printer_name, "Elegoo Saturn 4 Ultra 16K");
        assert_eq!(f.header.profile_name, "ABS-Like Resin Pro 2 Grey 50u");
        assert!((f.header.volume_mm3 - 8.0).abs() < 0.5);
        let (def, img) = f.layer(10).unwrap();
        assert!((def.z_mm - 0.55).abs() < 1e-5);
        assert_eq!(def.exposure_s, 1.4);
        // 2 mm at 14 µm ≈ 143 pixels wide, and about 2 / (118.37/6230) ≈ 105 rows high.
        let lit = img.iter().filter(|p| **p == 255).count();
        assert!((lit as f32 - 142.86 * 105.26).abs() < 400.0, "{lit}");
        let (def0, _) = f.layer(0).unwrap();
        assert_eq!(def0.exposure_s, 29.0);
        assert_eq!(def0.lift[0], (3.0, 35.0));
    }

    #[test]
    fn refuses_models_off_the_plate_and_cancels_cleanly() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = Session::new(Document::new("x"));
        s.run_script("project.set_printer printer=elegoo/saturn-4-ultra-16k\nobject.add_cube size=5\nobject.move id=1 x=200")
            .unwrap();
        let path = dir.path().join("x.goo");
        let err = export(&s.doc, &path, &|_| {}, &AtomicBool::new(false)).unwrap_err();
        assert!(matches!(err, ExportError::OutsidePlate(_)));
        s.run_line("object.move id=1 x=-200").unwrap();
        let err = export(&s.doc, &path, &|_| {}, &AtomicBool::new(true)).unwrap_err();
        assert!(matches!(err, ExportError::Cancelled));
        assert!(!path.exists() && !dir.path().join("x.goo.part").exists());
    }
}
