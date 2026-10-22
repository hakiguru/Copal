//! Commands for settings, presets, importing and laying out objects.

use std::path::Path;
use std::sync::Arc;

use copal_mesh::Mesh;
use glam::Vec3;
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::command::{Batch, Command, CommandError};
use crate::commands::{AddObject, RemoveObject, SetTransform};
use crate::document::{Document, ObjectId, ProjectSettings, Transform};
use crate::script::{Args, quote};

// ---------------------------------------------------------------------------------------------
// Settings

/// A part of [`ProjectSettings`] that `<section>.set` changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Print,
    Supports,
    Hollow,
    Layout,
}

impl Section {
    pub const ALL: [Section; 4] = [
        Section::Print,
        Section::Supports,
        Section::Hollow,
        Section::Layout,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Section::Print => "print",
            Section::Supports => "supports",
            Section::Hollow => "hollow",
            Section::Layout => "layout",
        }
    }

    /// The section's settings as a flat TOML table.
    pub fn table(self, s: &ProjectSettings) -> toml::Table {
        fn t<T: Serialize>(v: &T) -> toml::Table {
            toml::Table::try_from(v).unwrap_or_default()
        }
        match self {
            Section::Print => t(&s.print),
            Section::Supports => t(&s.supports),
            Section::Hollow => t(&s.hollow),
            Section::Layout => t(&s.layout),
        }
    }

    fn store(self, s: &mut ProjectSettings, table: toml::Table) -> Result<(), String> {
        fn f<T: DeserializeOwned>(table: toml::Table) -> Result<T, String> {
            table
                .try_into()
                .map_err(|e: toml::de::Error| e.message().to_string())
        }
        match self {
            Section::Print => s.print = f(table)?,
            Section::Supports => s.supports = f(table)?,
            Section::Hollow => s.hollow = f(table)?,
            Section::Layout => s.layout = f(table)?,
        }
        Ok(())
    }

    /// Setting names of this section, sorted.
    pub fn keys(self) -> Vec<String> {
        self.table(&ProjectSettings::default())
            .keys()
            .cloned()
            .collect()
    }
}

/// Writes a TOML value the way the text language takes it: `1.5`, `true`, `2,1`.
pub fn format_value(v: &toml::Value) -> String {
    match v {
        toml::Value::Array(items) => items.iter().map(format_value).collect::<Vec<_>>().join(","),
        toml::Value::String(s) => quote(s),
        toml::Value::Float(f) => format!("{}", *f as f32),
        other => other.to_string(),
    }
}

fn parse_like(old: &toml::Value, raw: &str) -> Result<toml::Value, String> {
    let raw = raw.trim();
    Ok(match old {
        toml::Value::Boolean(_) => toml::Value::Boolean(match raw {
            "true" | "on" | "yes" | "1" => true,
            "false" | "off" | "no" | "0" => false,
            _ => return Err("expected true or false".into()),
        }),
        toml::Value::Integer(_) => toml::Value::Integer(
            raw.parse::<i64>()
                .map_err(|_| "expected a whole number".to_string())?,
        ),
        toml::Value::Float(_) => {
            let f = raw
                .parse::<f64>()
                .map_err(|_| "expected a number".to_string())?;
            if !f.is_finite() {
                return Err("expected a number".into());
            }
            toml::Value::Float(f)
        }
        toml::Value::Array(items) => {
            let parts: Vec<&str> = raw
                .trim_start_matches('[')
                .trim_end_matches(']')
                .split(',')
                .map(str::trim)
                .collect();
            if parts.len() != items.len() {
                return Err(format!(
                    "expected {} values separated by commas",
                    items.len()
                ));
            }
            let first = items.first().cloned().unwrap_or(toml::Value::Float(0.0));
            toml::Value::Array(
                parts
                    .iter()
                    .map(|p| parse_like(&first, p))
                    .collect::<Result<_, _>>()?,
            )
        }
        toml::Value::String(_) => toml::Value::String(raw.to_string()),
        _ => return Err("this setting can not be changed from text".into()),
    })
}

fn check_ranges(section: Section, s: &ProjectSettings) -> Result<(), String> {
    let table = section.table(s);
    for (k, v) in &table {
        let nums: Vec<f64> = match v {
            toml::Value::Float(f) => vec![*f],
            toml::Value::Integer(i) => vec![*i as f64],
            toml::Value::Array(a) => a
                .iter()
                .filter_map(|x| x.as_float().or(x.as_integer().map(|i| i as f64)))
                .collect(),
            _ => vec![],
        };
        if nums.iter().any(|n| *n < 0.0) {
            return Err(format!("`{k}` can not be negative"));
        }
    }
    let p = &s.print;
    if section == Section::Print {
        if !(0.005..=0.5).contains(&p.exposure.layer_height_mm) {
            return Err("`layer_height_mm` must be between 0.005 and 0.5".into());
        }
        if ![1, 2, 4, 8].contains(&p.motion.anti_aliasing) {
            return Err("`anti_aliasing` must be 1 (off), 2, 4 or 8".into());
        }
    }
    if section == Section::Supports && !(0.0..=89.0).contains(&s.supports.overhang_angle_deg) {
        return Err("`overhang_angle_deg` must be between 0 and 89".into());
    }
    if section == Section::Hollow && s.hollow.voxel_mm < 0.05 {
        return Err("`voxel_mm` must be at least 0.05".into());
    }
    Ok(())
}

/// `<section>.set key=value …`
#[derive(Debug, Clone)]
pub struct SetSettings {
    pub section: Section,
    pub values: Vec<(String, String)>,
}

impl SetSettings {
    pub fn build(section: Section, a: &Args) -> Result<Box<dyn Command>, CommandError> {
        let keys = section.keys();
        let mut values = Vec::new();
        for k in a.keys() {
            if !keys.iter().any(|x| x == k) {
                return Err(CommandError::UnknownArg {
                    command: a.command().to_string(),
                    arg: k.to_string(),
                });
            }
            values.push((k.to_string(), a.str(k)?.to_string()));
        }
        if values.is_empty() {
            return Err(CommandError::Invalid(format!(
                "`{}.set` needs at least one setting, e.g. `{}`",
                section.name(),
                match section {
                    Section::Print => "print.set exposure_s=2.5",
                    Section::Supports => "supports.set overhang_angle_deg=50",
                    Section::Hollow => "hollow.set wall_thickness_mm=2",
                    Section::Layout => "layout.set keep_on_plate=false",
                }
            )));
        }
        // Check the values now, so typos fail before anything runs.
        let mut probe = Document::default();
        let cmd = SetSettings { section, values };
        cmd.apply(&mut probe)?;
        Ok(Box::new(cmd))
    }

    /// One command line that sets every value of a section, e.g. to apply a saved preset.
    pub fn line_for(section: Section, settings: &ProjectSettings) -> String {
        let mut line = format!("{}.set", section.name());
        for (k, v) in section.table(settings) {
            line.push_str(&format!(" {k}={}", format_value(&v)));
        }
        line
    }
}

impl Command for SetSettings {
    fn name(&self) -> &'static str {
        match self.section {
            Section::Print => "print.set",
            Section::Supports => "supports.set",
            Section::Hollow => "hollow.set",
            Section::Layout => "layout.set",
        }
    }

    fn to_text(&self) -> Option<String> {
        let mut line = format!("{}.set", self.section.name());
        for (k, v) in &self.values {
            line.push_str(&format!(" {k}={}", quote(v)));
        }
        Some(line)
    }

    fn apply(&self, doc: &mut Document) -> Result<Box<dyn Command>, CommandError> {
        let mut table = self.section.table(&doc.settings);
        for (k, raw) in &self.values {
            let old = table.get(k).ok_or_else(|| CommandError::UnknownArg {
                command: self.name().to_string(),
                arg: k.clone(),
            })?;
            let new = parse_like(old, raw).map_err(|reason| CommandError::BadValue {
                command: self.name().to_string(),
                arg: k.clone(),
                value: raw.clone(),
                reason,
            })?;
            table.insert(k.clone(), new);
        }
        let mut next = doc.settings;
        self.section
            .store(&mut next, table)
            .map_err(CommandError::Invalid)?;
        check_ranges(self.section, &next).map_err(CommandError::Invalid)?;
        let old = std::mem::replace(&mut doc.settings, next);
        Ok(Box::new(RestoreProject {
            printer: doc.printer.clone(),
            resin: doc.resin.clone(),
            settings: old,
        }))
    }
}

/// Puts printer, resin and settings back exactly. The inverse of the commands in this file.
#[derive(Debug, Clone)]
struct RestoreProject {
    printer: Option<String>,
    resin: Option<String>,
    settings: ProjectSettings,
}

impl Command for RestoreProject {
    fn name(&self) -> &'static str {
        "project.restore"
    }

    fn to_text(&self) -> Option<String> {
        None
    }

    fn apply(&self, doc: &mut Document) -> Result<Box<dyn Command>, CommandError> {
        let old = RestoreProject {
            printer: std::mem::replace(&mut doc.printer, self.printer.clone()),
            resin: std::mem::replace(&mut doc.resin, self.resin.clone()),
            settings: std::mem::replace(&mut doc.settings, self.settings),
        };
        Ok(Box::new(old))
    }
}

/// `project.set_printer` and `project.set_resin`. When the resin has a profile for the printer,
/// its print settings are applied too.
#[derive(Debug, Clone)]
pub enum SetMachine {
    Printer(Option<String>),
    Resin(Option<String>),
}

impl Command for SetMachine {
    fn name(&self) -> &'static str {
        match self {
            Self::Printer(_) => "project.set_printer",
            Self::Resin(_) => "project.set_resin",
        }
    }

    fn to_text(&self) -> Option<String> {
        let opt = |v: &Option<String>| quote(v.as_deref().unwrap_or(""));
        Some(match self {
            Self::Printer(p) => format!("project.set_printer printer={}", opt(p)),
            Self::Resin(r) => format!("project.set_resin resin={}", opt(r)),
        })
    }

    fn apply(&self, doc: &mut Document) -> Result<Box<dyn Command>, CommandError> {
        let undo = RestoreProject {
            printer: doc.printer.clone(),
            resin: doc.resin.clone(),
            settings: doc.settings,
        };
        let lib = crate::library();
        match self {
            Self::Printer(p) => {
                if let Some(id) = p
                    && lib.printer(id).is_none()
                {
                    return Err(CommandError::Invalid(format!(
                        "unknown printer `{id}` (run `copal-cli presets printers` to see the list)"
                    )));
                }
                doc.printer = p.clone();
            }
            Self::Resin(r) => {
                if let Some(id) = r
                    && lib.resin(id).is_none()
                {
                    return Err(CommandError::Invalid(format!(
                        "unknown resin `{id}` (run `copal-cli presets resins` to see the list)"
                    )));
                }
                doc.resin = r.clone();
            }
        }
        if let (Some(printer), Some(resin)) = (&doc.printer, &doc.resin)
            && let Some(profile) = lib.resin(resin).and_then(|r| r.profile_for(printer))
        {
            doc.settings.print = profile.settings;
        }
        Ok(Box::new(undo))
    }
}

// ---------------------------------------------------------------------------------------------
// Objects

/// Moves a mesh so its bounding box is centered on X and Y and stands on z = 0.
pub fn normalize_mesh(mesh: &mut Mesh) {
    if let Some(b) = mesh.bounds() {
        let shift = Vec3::new(
            -(b.min.x + b.max.x) / 2.0,
            -(b.min.y + b.max.y) / 2.0,
            -b.min.z,
        );
        for p in &mut mesh.positions {
            *p += shift;
        }
    }
}

/// Finds a spot on the plate for an object of this footprint that does not overlap others.
fn free_spot(doc: &Document, size: Vec3, spacing: f32) -> Vec3 {
    let Some(all) = doc.bounds() else {
        return Vec3::ZERO;
    };
    let overlaps = |c: Vec3| {
        doc.objects()
            .iter()
            .filter_map(|o| o.world_bounds())
            .any(|b| {
                c.x - size.x / 2.0 < b.max.x + spacing
                    && c.x + size.x / 2.0 > b.min.x - spacing
                    && c.y - size.y / 2.0 < b.max.y + spacing
                    && c.y + size.y / 2.0 > b.min.y - spacing
            })
    };
    if !overlaps(Vec3::ZERO) {
        return Vec3::ZERO;
    }
    let plate = doc.build_volume();
    // Try spots in rings around the center, nearest first.
    let step = (size.x.max(size.y) + spacing).max(1.0) / 2.0;
    for ring in 1..200 {
        let r = ring as f32 * step;
        let n = (ring * 8).max(8);
        for i in 0..n {
            let a = i as f32 / n as f32 * std::f32::consts::TAU;
            let c = Vec3::new(r * a.cos(), r * a.sin(), 0.0);
            let inside = c.x.abs() + size.x / 2.0 <= plate.x / 2.0
                && c.y.abs() + size.y / 2.0 <= plate.y / 2.0;
            if inside && !overlaps(c) {
                return c;
            }
        }
    }
    Vec3::new(all.max.x + spacing + size.x / 2.0, 0.0, 0.0)
}

/// `object.import path=<file>`: reads STL, OBJ or 3MF and puts the model on the plate.
#[derive(Debug, Clone)]
pub struct Import {
    pub path: String,
    pub name: String,
    pub mesh: Arc<Mesh>,
    name_given: bool,
}

impl Import {
    pub fn build(a: &Args) -> Result<Box<dyn Command>, CommandError> {
        a.only(&["path", "name"])?;
        let path = a.str("path")?.to_string();
        let mut mesh = copal_mesh::import::load(Path::new(&path))
            .map_err(|e| CommandError::Invalid(format!("{path}: {e}")))?;
        normalize_mesh(&mut mesh);
        let name = match a.opt_str("name") {
            Some(n) => n.to_string(),
            None => Path::new(&path)
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "Model".into()),
        };
        Ok(Box::new(Import {
            path,
            name,
            mesh: Arc::new(mesh),
            name_given: a.opt_str("name").is_some(),
        }))
    }
}

impl Command for Import {
    fn name(&self) -> &'static str {
        "object.import"
    }

    fn to_text(&self) -> Option<String> {
        let mut t = format!("object.import path={}", quote(&self.path));
        if self.name_given {
            t.push_str(&format!(" name={}", quote(&self.name)));
        }
        Some(t)
    }

    fn apply(&self, doc: &mut Document) -> Result<Box<dyn Command>, CommandError> {
        let size = self.mesh.bounds().map(|b| b.size()).unwrap_or(Vec3::ZERO);
        let spot = free_spot(doc, size, doc.settings.layout.arrange_spacing_mm);
        AddObject {
            id: None,
            index: None,
            name: self.name.clone(),
            transform: Transform {
                translation: spot,
                ..Default::default()
            },
            mesh: self.mesh.clone(),
            text: None,
        }
        .apply(doc)
    }
}

/// `object.duplicate id=<id> [count=<n>]`
#[derive(Debug, Clone)]
pub struct Duplicate {
    pub id: ObjectId,
    pub count: u32,
}

impl Command for Duplicate {
    fn name(&self) -> &'static str {
        "object.duplicate"
    }

    fn to_text(&self) -> Option<String> {
        Some(if self.count == 1 {
            format!("object.duplicate id={}", self.id)
        } else {
            format!("object.duplicate id={} count={}", self.id, self.count)
        })
    }

    fn apply(&self, doc: &mut Document) -> Result<Box<dyn Command>, CommandError> {
        let src = doc
            .object(self.id)
            .cloned()
            .ok_or(CommandError::NoSuchObject(self.id))?;
        let b = src.world_bounds().unwrap_or(copal_mesh::Aabb {
            min: Vec3::ZERO,
            max: Vec3::ZERO,
        });
        let center = (b.min + b.max) / 2.0;
        let offset = src.transform.translation - center;
        let mut inverses: Vec<Box<dyn Command>> = Vec::new();
        for _ in 0..self.count {
            let mut spot = free_spot(doc, b.size(), doc.settings.layout.arrange_spacing_mm);
            spot.z = center.z;
            let inv = AddObject {
                id: None,
                index: None,
                name: format!("{} copy", src.name),
                transform: Transform {
                    translation: spot + offset,
                    ..src.transform
                },
                mesh: src.mesh.clone(),
                text: None,
            }
            .apply(doc)?;
            inverses.push(inv);
        }
        inverses.reverse();
        Ok(Box::new(Batch::inverse(inverses)))
    }
}

/// Mirrors an object along a world axis, around its own origin.
pub fn mirrored(t: Transform, axis: usize) -> Transform {
    let mut out = t;
    // Reflecting a rotation keeps the angle about the mirror axis and flips the other two.
    for i in 0..3 {
        if i != axis {
            out.rotation_deg[i] = -out.rotation_deg[i];
        }
    }
    out.scale[axis] = -out.scale[axis];
    out
}

/// `object.mirror id=<id> axis=x|y|z`
#[derive(Debug, Clone)]
pub struct Mirror {
    pub id: ObjectId,
    pub axis: usize,
}

impl Command for Mirror {
    fn name(&self) -> &'static str {
        "object.mirror"
    }

    fn to_text(&self) -> Option<String> {
        Some(format!(
            "object.mirror id={} axis={}",
            self.id,
            ["x", "y", "z"][self.axis]
        ))
    }

    fn apply(&self, doc: &mut Document) -> Result<Box<dyn Command>, CommandError> {
        let o = doc
            .object_mut(self.id)
            .ok_or(CommandError::NoSuchObject(self.id))?;
        let old = o.transform;
        o.transform = mirrored(old, self.axis);
        if self.axis == 2 {
            // Mirroring in Z turns the model upside down: put it back on the plate.
            if let Some(b) = o.world_bounds() {
                o.transform.translation.z -= b.min.z;
            }
        }
        Ok(Box::new(SetTransform {
            id: self.id,
            transform: old,
        }))
    }
}

/// `object.arrange [spacing=<mm>]`: packs every object on the plate in rows.
#[derive(Debug, Clone)]
pub struct Arrange {
    pub spacing: Option<f32>,
}

/// New positions for every object: shelf packing, biggest first, centered on the plate.
pub fn arrange_positions(doc: &Document, spacing: f32) -> Vec<(ObjectId, Vec3)> {
    let plate = doc.build_volume();
    let mut items: Vec<(ObjectId, copal_mesh::Aabb, Vec3)> = doc
        .objects()
        .iter()
        .filter_map(|o| o.world_bounds().map(|b| (o.id, b, o.transform.translation)))
        .collect();
    items.sort_by(|a, b| {
        let (sa, sb) = (a.1.size(), b.1.size());
        (sb.y, sb.x)
            .partial_cmp(&(sa.y, sa.x))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    // Rows from the back of the plate (+Y) to the front.
    let mut rows: Vec<(Vec<usize>, f32, f32)> = Vec::new(); // (items, width, depth)
    for (i, (_, b, _)) in items.iter().enumerate() {
        let s = b.size();
        match rows.last_mut() {
            Some(row) if row.1 + spacing + s.x <= plate.x => {
                row.0.push(i);
                row.1 += spacing + s.x;
                row.2 = row.2.max(s.y);
            }
            _ => rows.push((vec![i], s.x, s.y)),
        }
    }
    let total_depth: f32 =
        rows.iter().map(|r| r.2).sum::<f32>() + spacing * rows.len().saturating_sub(1) as f32;
    let mut out = Vec::new();
    let mut y = total_depth / 2.0;
    for (row, width, depth) in rows {
        let mut x = -width / 2.0;
        for i in row {
            let (id, b, t) = items[i];
            let s = b.size();
            let center = (b.min + b.max) / 2.0;
            let target = Vec3::new(x + s.x / 2.0, y - depth / 2.0, center.z);
            out.push((id, t + (target - center)));
            x += s.x + spacing;
        }
        y -= depth + spacing;
    }
    out
}

impl Command for Arrange {
    fn name(&self) -> &'static str {
        "object.arrange"
    }

    fn to_text(&self) -> Option<String> {
        Some(match self.spacing {
            Some(s) => format!("object.arrange spacing={s}"),
            None => "object.arrange".into(),
        })
    }

    fn apply(&self, doc: &mut Document) -> Result<Box<dyn Command>, CommandError> {
        let spacing = self
            .spacing
            .unwrap_or(doc.settings.layout.arrange_spacing_mm);
        let moves: Vec<Box<dyn Command>> = arrange_positions(doc, spacing)
            .into_iter()
            .map(|(id, pos)| {
                let mut t = doc.object(id).map(|o| o.transform).unwrap_or_default();
                t.translation = pos;
                Box::new(SetTransform { id, transform: t }) as Box<dyn Command>
            })
            .collect();
        Batch::inverse(moves).apply(doc)
    }
}

/// `object.remove_all`
#[derive(Debug, Clone)]
pub struct RemoveAll;

impl Command for RemoveAll {
    fn name(&self) -> &'static str {
        "object.remove_all"
    }

    fn to_text(&self) -> Option<String> {
        Some("object.remove_all".into())
    }

    fn apply(&self, doc: &mut Document) -> Result<Box<dyn Command>, CommandError> {
        let ids: Vec<ObjectId> = doc.objects().iter().rev().map(|o| o.id).collect();
        Batch::inverse(
            ids.into_iter()
                .map(|id| Box::new(RemoveObject { id }) as Box<dyn Command>)
                .collect(),
        )
        .apply(doc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::{EulerRot, Mat4, Quat};

    #[test]
    fn mirror_matches_a_world_reflection() {
        let t = Transform {
            translation: Vec3::new(3.0, -2.0, 5.0),
            rotation_deg: Vec3::new(20.0, -35.0, 70.0),
            scale: Vec3::new(1.0, 2.0, 0.5),
        };
        for axis in 0..3 {
            let mut f = Vec3::ONE;
            f[axis] = -1.0;
            let expected = Mat4::from_translation(t.translation)
                * Mat4::from_scale(f)
                * Mat4::from_quat(Quat::from_euler(
                    EulerRot::XYZ,
                    t.rotation_deg.x.to_radians(),
                    t.rotation_deg.y.to_radians(),
                    t.rotation_deg.z.to_radians(),
                ))
                * Mat4::from_scale(t.scale);
            let got = mirrored(t, axis).matrix();
            assert!(got.abs_diff_eq(expected, 1e-4), "axis {axis}");
            assert_eq!(mirrored(mirrored(t, axis), axis), t);
        }
    }

    #[test]
    fn values_parse_like_the_old_ones() {
        let f = toml::Value::Float(1.0);
        assert_eq!(parse_like(&f, "2").unwrap(), toml::Value::Float(2.0));
        assert!(parse_like(&f, "x").is_err());
        let arr = toml::Value::Array(vec![f.clone(), f.clone()]);
        assert_eq!(
            parse_like(&arr, "[2, 1.5]").unwrap(),
            toml::Value::Array(vec![toml::Value::Float(2.0), toml::Value::Float(1.5)])
        );
        assert!(parse_like(&arr, "2").is_err());
        assert_eq!(
            parse_like(&toml::Value::Boolean(true), "off").unwrap(),
            toml::Value::Boolean(false)
        );
    }
}
