//! State and panels shared by Copal and Copal Lite. The apps only decide the layout.

use std::path::PathBuf;

use copal_core::{Document, ObjectId, Session, Transform, script::quote};
use egui::{
    Color32, DragValue, Key, KeyboardShortcut, Modifiers, RichText, ScrollArea, TextEdit, Ui,
};

use crate::look::Look;
use crate::palette::{amber, ink};
use crate::viewport::{self, Camera};
use crate::widgets::{self, ButtonKind};

/// Printers and resins offered in the pickers until the preset database lands in 0.1.
pub const PRINTERS: &[(&str, &str)] = &[("elegoo/saturn-4-ultra-16k", "Elegoo Saturn 4 Ultra 16K")];
pub const RESINS: &[(&str, &str)] = &[
    (
        "anycubic/abs-like-pro-2-grey",
        "Anycubic ABS-Like Pro 2 · Grey",
    ),
    (
        "anycubic/abs-like-pro-2-clear",
        "Anycubic ABS-Like Pro 2 · Clear",
    ),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Prepare,
    Supports,
    Slice,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    Input,
    Info,
    Error,
}

pub struct Workspace {
    pub session: Session,
    pub selected: Option<ObjectId>,
    pub camera: Camera,
    pub step: Step,
    pub console_input: String,
    pub console: Vec<(LineKind, String)>,
    /// Transform being edited in the inspector: (object, value before editing started).
    editing: Option<(ObjectId, Transform)>,
}

impl Default for Workspace {
    fn default() -> Self {
        let mut doc = Document::new("Untitled");
        doc.printer = Some(PRINTERS[0].0.to_string());
        let mut ws = Self {
            session: Session::new(doc),
            selected: None,
            camera: Camera::default(),
            step: Step::Prepare,
            console_input: String::new(),
            console: Vec::new(),
            editing: None,
        };
        ws.log(
            LineKind::Info,
            "Welcome to Copal. Type `help` for console commands.",
        );
        ws
    }
}

impl Workspace {
    pub fn log(&mut self, kind: LineKind, text: impl Into<String>) {
        self.console.push((kind, text.into()));
        if self.console.len() > 500 {
            self.console.drain(..100);
        }
    }

    /// Runs one console line: a document command, or one of the app commands
    /// (`undo`, `redo`, `save`, `open`, `new`, `help`, `clear`).
    pub fn run(&mut self, line: &str) {
        let line = line.trim();
        if line.is_empty() {
            return;
        }
        self.log(LineKind::Input, line);
        let (head, rest) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
        let rest = rest.trim().trim_matches('"');
        match head {
            "help" => {
                self.log(
                    LineKind::Info,
                    "App: undo · redo · save [path] · open <path> · new · clear · commands",
                );
                self.log(LineKind::Info, "Document commands work exactly like in copal-cli, e.g. object.add_cube size=20");
            }
            "commands" => {
                for c in copal_core::registry() {
                    self.log(LineKind::Info, c.usage);
                }
            }
            "clear" => self.console.clear(),
            "undo" => self.undo(),
            "redo" => self.redo(),
            "new" => {
                *self = Self::default();
            }
            "save" => {
                let result = if rest.is_empty() {
                    match self.session.save() {
                        Ok(true) => Ok(()),
                        Ok(false) => Err("no file yet: use `save <path>`".to_string()),
                        Err(e) => Err(e.to_string()),
                    }
                } else {
                    self.session
                        .save_as(&PathBuf::from(rest))
                        .map_err(|e| e.to_string())
                };
                match result {
                    Ok(()) => {
                        let p = self
                            .session
                            .path()
                            .map(|p| p.display().to_string())
                            .unwrap_or_default();
                        self.log(LineKind::Info, format!("Saved {p}"));
                    }
                    Err(e) => self.log(LineKind::Error, e),
                }
            }
            "open" => match Session::open(&PathBuf::from(rest)) {
                Ok(s) => {
                    self.session = s;
                    self.selected = None;
                    self.log(LineKind::Info, format!("Opened {rest}"));
                }
                Err(e) => self.log(LineKind::Error, e.to_string()),
            },
            _ => {
                let before: Vec<ObjectId> =
                    self.session.doc.objects().iter().map(|o| o.id).collect();
                if let Err(e) = self.session.run_line(line) {
                    self.log(LineKind::Error, e.to_string());
                }
                // Select a newly added object.
                if let Some(new) = self
                    .session
                    .doc
                    .objects()
                    .iter()
                    .map(|o| o.id)
                    .find(|id| !before.contains(id))
                {
                    self.selected = Some(new);
                }
            }
        }
        self.fix_selection();
    }

    pub fn undo(&mut self) {
        match self.session.history.undo(&mut self.session.doc) {
            Ok(Some(label)) => self.log(LineKind::Info, format!("Undo: {label}")),
            Ok(None) => self.log(LineKind::Info, "Nothing to undo"),
            Err(e) => self.log(LineKind::Error, e.to_string()),
        }
        self.fix_selection();
    }

    pub fn redo(&mut self) {
        match self.session.history.redo(&mut self.session.doc) {
            Ok(Some(label)) => self.log(LineKind::Info, format!("Redo: {label}")),
            Ok(None) => self.log(LineKind::Info, "Nothing to redo"),
            Err(e) => self.log(LineKind::Error, e.to_string()),
        }
        self.fix_selection();
    }

    fn fix_selection(&mut self) {
        if self
            .selected
            .is_some_and(|id| self.session.doc.object(id).is_none())
        {
            self.selected = None;
        }
    }

    /// Global hotkeys. Call once per frame before drawing panels.
    pub fn hotkeys(&mut self, ui: &Ui) {
        let ctx = ui.ctx();
        let redo = ctx.input_mut(|i| {
            i.consume_shortcut(&KeyboardShortcut::new(
                Modifiers::COMMAND | Modifiers::SHIFT,
                Key::Z,
            )) || i.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::Y))
        });
        let undo = ctx
            .input_mut(|i| i.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::Z)));
        let save = ctx
            .input_mut(|i| i.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::S)));
        let typing = ctx.memory(|m| m.focused().is_some());
        let delete = !typing && ctx.input(|i| i.key_pressed(Key::Delete) || i.key_pressed(Key::X));
        if redo {
            self.redo();
        } else if undo {
            self.undo();
        }
        if save {
            self.run("save");
        }
        if delete && let Some(id) = self.selected {
            self.run(&format!("object.remove id={id}"));
        }
    }

    pub fn title(&self) -> String {
        let dirty = if self.session.is_dirty() { " •" } else { "" };
        format!("{}{dirty} — Copal", self.session.doc.name)
    }

    // ------------------------------------------------------------------------------- panels

    pub fn printer_resin_pickers(&mut self, ui: &mut Ui) {
        let doc = &self.session.doc;
        let mut printer = doc.printer.clone().unwrap_or_default();
        let mut resin = doc.resin.clone().unwrap_or_default();
        let label = |list: &[(&str, &str)], id: &str, empty: &str| {
            list.iter()
                .find(|(i, _)| *i == id)
                .map(|(_, n)| n.to_string())
                .unwrap_or_else(|| empty.to_string())
        };
        ui.label(RichText::new("Printer").color(ink::TEXT_WEAK));
        egui::ComboBox::from_id_salt("printer")
            .selected_text(label(PRINTERS, &printer, "Choose a printer"))
            .width(210.0)
            .show_ui(ui, |ui| {
                for (id, name) in PRINTERS {
                    ui.selectable_value(&mut printer, id.to_string(), *name);
                }
            });
        ui.label(RichText::new("Resin").color(ink::TEXT_WEAK));
        egui::ComboBox::from_id_salt("resin")
            .selected_text(label(RESINS, &resin, "Choose a resin"))
            .width(230.0)
            .show_ui(ui, |ui| {
                for (id, name) in RESINS {
                    ui.selectable_value(&mut resin, id.to_string(), *name);
                }
            });
        if Some(&printer) != self.session.doc.printer.as_ref() && !printer.is_empty() {
            self.run(&format!("project.set_printer printer={}", quote(&printer)));
        }
        if Some(&resin) != self.session.doc.resin.as_ref() && !resin.is_empty() {
            self.run(&format!("project.set_resin resin={}", quote(&resin)));
        }
    }

    /// Tools of the current workflow step.
    pub fn tool_shelf(&mut self, ui: &mut Ui, look: &Look) {
        let w = Some(ui.available_width());
        match self.step {
            Step::Prepare => {
                widgets::section(ui, "Add");
                if widgets::button_sized(ui, look, ButtonKind::Primary, "Add test cube", w)
                    .clicked()
                {
                    self.run("object.add_cube size=20");
                }
                ui.label(
                    RichText::new("STL / OBJ / 3MF import arrives in 0.1")
                        .small()
                        .color(ink::TEXT_FAINT),
                );
                widgets::section(ui, "Arrange");
                ui.add_enabled_ui(self.selected.is_some(), |ui| {
                    let id = self.selected.map(|i| i.0).unwrap_or(0);
                    if widgets::button_sized(ui, look, ButtonKind::Normal, "Drop to plate", w)
                        .clicked()
                    {
                        self.run(&format!("object.drop id={id}"));
                    }
                    if widgets::button_sized(ui, look, ButtonKind::Normal, "Rotate 45° (X)", w)
                        .clicked()
                    {
                        self.run(&format!("object.rotate id={id} x=45"));
                    }
                    if widgets::button_sized(ui, look, ButtonKind::Normal, "Delete", w).clicked() {
                        self.run(&format!("object.remove id={id}"));
                    }
                });
                widgets::section(ui, "History");
                ui.horizontal(|ui| {
                    let half = (ui.available_width() - ui.spacing().item_spacing.x) / 2.0;
                    ui.add_enabled_ui(self.session.history.can_undo(), |ui| {
                        if widgets::button_sized(ui, look, ButtonKind::Normal, "Undo", Some(half))
                            .clicked()
                        {
                            self.undo();
                        }
                    });
                    ui.add_enabled_ui(self.session.history.can_redo(), |ui| {
                        if widgets::button_sized(ui, look, ButtonKind::Normal, "Redo", Some(half))
                            .clicked()
                        {
                            self.redo();
                        }
                    });
                });
            }
            Step::Supports => planned(
                ui,
                look,
                "Supports",
                "Auto supports, painting, rafts and manual editing arrive in Copal 0.3.",
            ),
            Step::Slice => planned(
                ui,
                look,
                "Slice & export",
                "Slicing to .goo and .ctb arrives in Copal 0.1, the layer preview in 0.2.",
            ),
        }
    }

    /// Object list.
    pub fn scene_list(&mut self, ui: &mut Ui) {
        if self.session.doc.objects().is_empty() {
            ui.label(RichText::new("No objects yet").color(ink::TEXT_FAINT));
            return;
        }
        let items: Vec<(ObjectId, String, usize)> = self
            .session
            .doc
            .objects()
            .iter()
            .map(|o| (o.id, o.name.clone(), o.mesh.triangle_count()))
            .collect();
        for (id, name, tris) in items {
            let selected = self.selected == Some(id);
            let text = RichText::new(format!("{name}   ·  {tris} tri")).color(if selected {
                amber::HIGHLIGHT
            } else {
                ink::TEXT
            });
            if ui.selectable_label(selected, text).clicked() {
                self.selected = Some(id);
            }
        }
    }

    /// Transform fields of the selected object.
    pub fn inspector(&mut self, ui: &mut Ui) {
        let Some(id) = self.selected else {
            ui.label(RichText::new("Select an object to see its settings").color(ink::TEXT_FAINT));
            return;
        };
        let Some(obj) = self.session.doc.object(id) else {
            return;
        };
        let name = obj.name.clone();
        let mut t = obj.transform;
        let size = obj.world_bounds().map(|b| b.size()).unwrap_or_default();
        ui.label(RichText::new(name).strong().color(ink::TEXT));
        ui.label(
            RichText::new(format!("{:.1} × {:.1} × {:.1} mm", size.x, size.y, size.z))
                .small()
                .color(ink::TEXT_WEAK),
        );
        ui.add_space(4.0);

        let mut changed = false;
        let mut active = false;
        egui::Grid::new("transform")
            .num_columns(4)
            .spacing([6.0, 6.0])
            .show(ui, |ui| {
                let mut row =
                    |ui: &mut Ui, label: &str, v: &mut glam::Vec3, speed: f64, suffix: &str| {
                        ui.label(RichText::new(label).color(ink::TEXT_WEAK));
                        for (axis, value, color) in [
                            ("X", &mut v.x, Color32::from_rgb(230, 110, 90)),
                            ("Y", &mut v.y, Color32::from_rgb(140, 200, 110)),
                            ("Z", &mut v.z, Color32::from_rgb(110, 160, 235)),
                        ] {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 3.0;
                                ui.label(RichText::new(axis).small().color(color));
                                let r = ui.add(
                                    DragValue::new(value)
                                        .speed(speed)
                                        .max_decimals(2)
                                        .suffix(suffix),
                                );
                                changed |= r.changed();
                                active |= r.dragged() || r.has_focus();
                            });
                        }
                        ui.end_row();
                    };
                row(ui, "Position", &mut t.translation, 0.5, " mm");
                row(ui, "Rotation", &mut t.rotation_deg, 1.0, "°");
                row(ui, "Scale", &mut t.scale, 0.01, "");
            });

        if changed {
            if self.editing.map(|(e, _)| e) != Some(id) {
                let original = self
                    .session
                    .doc
                    .object(id)
                    .map(|o| o.transform)
                    .unwrap_or_default();
                self.editing = Some((id, original));
            }
            t.scale = t.scale.max(glam::Vec3::splat(0.001));
            // Live preview without filling the history with every tiny step.
            if let Some(o) = self.session.doc.object_mut(id) {
                o.transform = t;
            }
        }
        if !active && let Some((eid, original)) = self.editing.take() {
            // Editing finished: record one history step from the original value.
            if let Some(o) = self.session.doc.object_mut(eid) {
                let new = std::mem::replace(&mut o.transform, original);
                let line = format!(
                    "object.set_transform id={eid} tx={} ty={} tz={} rx={} ry={} rz={} sx={} sy={} sz={}",
                    new.translation.x,
                    new.translation.y,
                    new.translation.z,
                    new.rotation_deg.x,
                    new.rotation_deg.y,
                    new.rotation_deg.z,
                    new.scale.x,
                    new.scale.y,
                    new.scale.z
                );
                if new != original {
                    self.run(&line);
                }
            }
        }
    }

    pub fn console_panel(&mut self, ui: &mut Ui) {
        // Keep scrolled-away lines inside the panel margin, off the crystal border.
        ui.set_clip_rect(ui.max_rect());
        // Leave exactly enough room for the input line, so the panel never grows by itself.
        let input_h = ui.spacing().interact_size.y + ui.spacing().item_spacing.y + 6.0;
        let height = (ui.available_height() - input_h).max(20.0);
        ScrollArea::vertical()
            .max_height(height)
            .stick_to_bottom(true)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (kind, text) in &self.console {
                    let (prefix, color) = match kind {
                        LineKind::Input => ("› ", amber::LIGHTER),
                        LineKind::Info => ("  ", ink::TEXT_WEAK),
                        LineKind::Error => ("! ", ink::ERROR),
                    };
                    ui.label(
                        RichText::new(format!("{prefix}{text}"))
                            .monospace()
                            .color(color),
                    );
                }
            });
        let r = ui.add(
            TextEdit::singleline(&mut self.console_input)
                .font(egui::TextStyle::Monospace)
                .hint_text("type a command, e.g. object.add_cube size=15   (help)")
                .desired_width(f32::INFINITY),
        );
        if r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
            let line = std::mem::take(&mut self.console_input);
            self.run(&line);
            r.request_focus();
        }
    }

    pub fn viewport(&mut self, ui: &mut Ui, flat: bool) {
        let out = viewport::show(ui, &self.session.doc, &mut self.camera, self.selected, flat);
        if let Some(hit) = out.clicked {
            self.selected = hit;
        }
    }

    pub fn status_line(&self, ui: &mut Ui) {
        let doc = &self.session.doc;
        let file = self
            .session
            .path()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "not saved".into());
        let dirty = if self.session.is_dirty() {
            " · unsaved changes"
        } else {
            ""
        };
        ui.label(
            RichText::new(format!("Copal {} preview", copal_core::VERSION))
                .small()
                .color(amber::LIGHT),
        );
        ui.label(
            RichText::new(format!(
                "·  {}  ·  {file}{dirty}  ·  {} objects",
                doc.name,
                doc.objects().len()
            ))
            .small()
            .color(ink::TEXT_FAINT),
        );
    }
}

fn planned(ui: &mut Ui, look: &Look, title: &str, text: &str) {
    widgets::section(ui, title);
    ui.label(RichText::new(text).color(ink::TEXT_WEAK));
    ui.add_space(6.0);
    ui.add_enabled_ui(false, |ui| {
        widgets::button_sized(
            ui,
            look,
            ButtonKind::Primary,
            title,
            Some(ui.available_width()),
        );
    });
}
