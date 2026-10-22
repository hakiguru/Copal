//! State and panels shared by Copal and Copal Lite. The apps only decide the layout.

use std::path::PathBuf;

use copal_core::{
    ColorMode, Document, ObjectId, Session, Settings, ThemeStyle, Transform, script::quote,
};
use egui::text::{CCursor, CCursorRange};
use egui::{
    Color32, DragValue, FontId, Id, Key, KeyboardShortcut, Modifiers, RichText, ScrollArea,
    TextEdit, Ui,
};

use crate::look::Look;
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

/// Console commands that belong to the app, not to the document.
pub const APP_COMMANDS: &[&str] = &[
    "clear", "commands", "help", "new", "open", "redo", "save", "settings", "undo",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Prepare,
    Supports,
    Slice,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineKind {
    /// A command the user typed, with the prompt it was typed at.
    Input {
        prompt: String,
    },
    Info,
    Error,
}

pub struct Workspace {
    pub session: Session,
    pub settings: Settings,
    pub selected: Option<ObjectId>,
    pub camera: Camera,
    pub step: Step,
    pub show_settings: bool,
    pub console_input: String,
    pub console: Vec<(LineKind, String)>,
    settings_changed: bool,
    /// Typed commands, oldest first, for ↑ / ↓.
    typed: Vec<String>,
    typed_pos: Option<usize>,
    console_focused: bool,
    /// Transform being edited in the inspector: (object, value before editing started).
    editing: Option<(ObjectId, Transform)>,
}

impl Default for Workspace {
    fn default() -> Self {
        Self::new(Settings::default())
    }
}

impl Workspace {
    pub fn new(settings: Settings) -> Self {
        let mut doc = Document::new("Untitled");
        doc.printer = Some(PRINTERS[0].0.to_string());
        let mut ws = Self {
            session: Session::new(doc),
            settings,
            selected: None,
            camera: Camera::default(),
            step: Step::Prepare,
            show_settings: false,
            console_input: String::new(),
            console: Vec::new(),
            settings_changed: false,
            typed: Vec::new(),
            typed_pos: None,
            console_focused: false,
            editing: None,
        };
        ws.log(
            LineKind::Info,
            format!(
                "Copal {} console. Type `help`, press Tab to complete.",
                copal_core::VERSION
            ),
        );
        ws
    }

    /// True once after the settings changed; the app then re-applies its look.
    pub fn take_settings_changed(&mut self) -> bool {
        std::mem::take(&mut self.settings_changed)
    }

    pub fn log(&mut self, kind: LineKind, text: impl Into<String>) {
        self.console.push((kind, text.into()));
        if self.console.len() > 500 {
            self.console.drain(..100);
        }
    }

    /// The bash-like prompt: `copal:<project>$`.
    pub fn prompt_place(&self) -> String {
        self.session
            .path()
            .and_then(|p| p.file_stem())
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "untitled".into())
    }

    /// Runs one console line: a document command, or one of [`APP_COMMANDS`].
    pub fn run(&mut self, line: &str) {
        let line = line.trim();
        if line.is_empty() {
            return;
        }
        let prompt = self.prompt_place();
        self.log(LineKind::Input { prompt }, line);
        let (head, rest) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
        let rest = rest.trim();
        match head {
            "help" => {
                self.log(
                    LineKind::Info,
                    "App commands: undo, redo, save [path], open <path>, new, clear, commands,",
                );
                self.log(
                    LineKind::Info,
                    "              settings [theme=default|art] [mode=dark|light]",
                );
                self.log(
                    LineKind::Info,
                    "Document commands work like in copal-cli, e.g. object.add_cube size=20",
                );
                self.log(LineKind::Info, "Keys: ↑/↓ history, Tab complete.");
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
                let settings = self.settings.clone();
                let typed = std::mem::take(&mut self.typed);
                *self = Self::new(settings);
                self.typed = typed;
            }
            "save" => self.save(rest.trim_matches('"')),
            "open" => match Session::open(&PathBuf::from(rest.trim_matches('"'))) {
                Ok(s) => {
                    self.session = s;
                    self.selected = None;
                    self.log(LineKind::Info, format!("Opened {rest}"));
                }
                Err(e) => self.log(LineKind::Error, e.to_string()),
            },
            "settings" => self.settings_command(rest),
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

    fn save(&mut self, path: &str) {
        let result = if path.is_empty() {
            match self.session.save() {
                Ok(true) => Ok(()),
                Ok(false) => Err("no file yet: use `save <path>`".to_string()),
                Err(e) => Err(e.to_string()),
            }
        } else {
            self.session
                .save_as(&PathBuf::from(path))
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

    fn settings_command(&mut self, rest: &str) {
        if rest.is_empty() {
            for (key, values) in Settings::KEYS {
                let value = self.settings.get(key).unwrap_or_default();
                self.log(LineKind::Info, format!("{key} = {value}    ({values})"));
            }
            return;
        }
        let mut changed = self.settings.clone();
        for pair in rest.split_whitespace() {
            let Some((k, v)) = pair.split_once('=') else {
                self.log(LineKind::Error, format!("expected key=value, got `{pair}`"));
                return;
            };
            if let Err(e) = changed.set(k, v) {
                self.log(LineKind::Error, e.to_string());
                return;
            }
        }
        self.set_settings(changed);
        self.log(LineKind::Info, "Settings saved");
    }

    /// Applies and saves new settings.
    pub fn set_settings(&mut self, settings: Settings) {
        if settings == self.settings {
            return;
        }
        self.settings = settings;
        self.settings_changed = true;
        if let Err(e) = self.settings.save() {
            self.log(LineKind::Error, format!("could not save settings: {e}"));
        }
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
        let settings = ctx.input_mut(|i| {
            i.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::Comma))
        });
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
        if settings {
            self.show_settings = !self.show_settings;
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

    pub fn printer_resin_pickers(&mut self, ui: &mut Ui, look: &Look) {
        let doc = &self.session.doc;
        let mut printer = doc.printer.clone().unwrap_or_default();
        let mut resin = doc.resin.clone().unwrap_or_default();
        let label = |list: &[(&str, &str)], id: &str, empty: &str| {
            list.iter()
                .find(|(i, _)| *i == id)
                .map(|(_, n)| n.to_string())
                .unwrap_or_else(|| empty.to_string())
        };
        ui.label(RichText::new("Printer").color(look.pal.text_weak));
        egui::ComboBox::from_id_salt("printer")
            .selected_text(label(PRINTERS, &printer, "Choose a printer"))
            .width(210.0)
            .show_ui(ui, |ui| {
                for (id, name) in PRINTERS {
                    ui.selectable_value(&mut printer, id.to_string(), *name);
                }
            });
        ui.label(RichText::new("Resin").color(look.pal.text_weak));
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
                widgets::section(ui, look, "Add");
                if widgets::button_sized(ui, look, ButtonKind::Primary, "Add test cube", w)
                    .clicked()
                {
                    self.run("object.add_cube size=20");
                }
                ui.label(
                    RichText::new("STL / OBJ / 3MF import arrives in 0.1")
                        .small()
                        .color(look.pal.text_faint),
                );
                widgets::section(ui, look, "Arrange");
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
                widgets::section(ui, look, "History");
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
    pub fn scene_list(&mut self, ui: &mut Ui, look: &Look) {
        if self.session.doc.objects().is_empty() {
            ui.label(RichText::new("No objects yet").color(look.pal.text_faint));
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
            let color = if selected {
                look.pal.accent_text
            } else {
                look.pal.text
            };
            let text = RichText::new(format!("{name}   ·  {tris} tri")).color(color);
            if ui.selectable_label(selected, text).clicked() {
                self.selected = Some(id);
            }
        }
    }

    /// Transform fields of the selected object.
    pub fn inspector(&mut self, ui: &mut Ui, look: &Look) {
        let Some(id) = self.selected else {
            ui.label(
                RichText::new("Select an object to see its settings").color(look.pal.text_faint),
            );
            return;
        };
        let Some(obj) = self.session.doc.object(id) else {
            return;
        };
        let name = obj.name.clone();
        let mut t = obj.transform;
        let size = obj.world_bounds().map(|b| b.size()).unwrap_or_default();
        ui.label(RichText::new(name).strong().color(look.pal.text));
        ui.label(
            RichText::new(format!("{:.1} × {:.1} × {:.1} mm", size.x, size.y, size.z))
                .small()
                .color(look.pal.text_weak),
        );
        ui.add_space(4.0);

        let axis_colors = if look.pal.dark {
            [
                Color32::from_rgb(230, 110, 90),
                Color32::from_rgb(140, 200, 110),
                Color32::from_rgb(110, 160, 235),
            ]
        } else {
            [
                Color32::from_rgb(200, 60, 40),
                Color32::from_rgb(50, 140, 40),
                Color32::from_rgb(40, 100, 200),
            ]
        };
        let mut changed = false;
        let mut active = false;
        egui::Grid::new("transform")
            .num_columns(4)
            .spacing([6.0, 6.0])
            .show(ui, |ui| {
                let mut row =
                    |ui: &mut Ui, label: &str, v: &mut glam::Vec3, speed: f64, suffix: &str| {
                        ui.label(RichText::new(label).color(look.pal.text_weak));
                        for ((axis, value), color) in
                            [("X", &mut v.x), ("Y", &mut v.y), ("Z", &mut v.z)]
                                .into_iter()
                                .zip(axis_colors)
                        {
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
                if new != original {
                    let (p, r, s) = (new.translation, new.rotation_deg, new.scale);
                    self.run(&format!(
                        "object.set_transform id={eid} tx={} ty={} tz={} rx={} ry={} rz={} sx={} sy={} sz={}",
                        p.x, p.y, p.z, r.x, r.y, r.z, s.x, s.y, s.z
                    ));
                }
            }
        }
    }

    /// The console. Default theme: looks like a plain terminal. Art theme: amber log.
    ///
    /// Like a real terminal, the input line follows right after the last output line.
    pub fn console_panel(&mut self, ui: &mut Ui, look: &Look) {
        let p = look.pal;
        let font = FontId::monospace(13.0);
        // Keep scrolled-away lines inside the panel margin.
        ui.set_clip_rect(ui.max_rect());
        ui.spacing_mut().item_spacing.y = 2.0;

        // Shell keys, handled before the text field sees them.
        let id = Id::unique("copal-console-input");
        if self.console_focused {
            let (up, down, tab) = ui.input_mut(|i| {
                (
                    i.consume_key(Modifiers::NONE, Key::ArrowUp),
                    i.consume_key(Modifiers::NONE, Key::ArrowDown),
                    i.consume_key(Modifiers::NONE, Key::Tab),
                )
            });
            if up || down {
                self.browse_history(up);
                move_cursor_to_end(ui, id, &self.console_input);
            }
            if tab {
                self.complete();
                move_cursor_to_end(ui, id, &self.console_input);
            }
        }

        let place = self.prompt_place();
        let console = &self.console;
        let input = &mut self.console_input;
        let r = ScrollArea::vertical()
            .stick_to_bottom(true)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (kind, text) in console {
                    match kind {
                        LineKind::Input { prompt } => {
                            ui.horizontal_wrapped(|ui| {
                                ui.spacing_mut().item_spacing.x = 0.0;
                                prompt_labels(ui, look, prompt, &font);
                                ui.label(RichText::new(text).font(font.clone()).color(p.term_fg));
                            });
                        }
                        LineKind::Info => {
                            let color = if look.art() { p.text_weak } else { p.term_fg };
                            ui.label(RichText::new(text).font(font.clone()).color(color));
                        }
                        LineKind::Error => {
                            ui.label(RichText::new(text).font(font.clone()).color(p.term_error));
                        }
                    }
                }
                // Top-aligned row: the text field puts its text at the top, so the prompt must too.
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    prompt_labels(ui, look, &place, &font);
                    ui.add(
                        TextEdit::singleline(input)
                            .id(id)
                            .font(font.clone())
                            .text_color(p.term_fg)
                            .frame(egui::Frame::NONE)
                            // No inner margin: the text sits on the same line as the prompt.
                            .margin(egui::Margin::ZERO)
                            .lock_focus(true)
                            .hint_text(RichText::new("help").font(font.clone()).color(p.text_faint))
                            .desired_width(f32::INFINITY),
                    )
                })
                .inner
            })
            .inner;
        // Clicking anywhere in the console puts the cursor in the input line.
        if ui.rect_contains_pointer(ui.max_rect()) && ui.input(|i| i.pointer.primary_clicked()) {
            r.request_focus();
        }
        self.console_focused = r.has_focus();
        if r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
            let line = std::mem::take(&mut self.console_input);
            if !line.trim().is_empty() && self.typed.last() != Some(&line) {
                self.typed.push(line.clone());
            }
            self.typed_pos = None;
            self.run(&line);
            r.request_focus();
        }
    }

    fn browse_history(&mut self, older: bool) {
        if self.typed.is_empty() {
            return;
        }
        let last = self.typed.len() - 1;
        self.typed_pos = match (self.typed_pos, older) {
            (None, true) => Some(last),
            (None, false) => None,
            (Some(0), true) => Some(0),
            (Some(i), true) => Some(i - 1),
            (Some(i), false) if i >= last => None,
            (Some(i), false) => Some(i + 1),
        };
        self.console_input = self
            .typed_pos
            .map(|i| self.typed[i].clone())
            .unwrap_or_default();
    }

    /// Bash-style Tab: complete a unique match, otherwise list the options.
    fn complete(&mut self) {
        let input = self.console_input.clone();
        let (options, prefix_len): (Vec<String>, usize) = match input.split_once(' ') {
            None => {
                let mut names: Vec<String> = copal_core::registry()
                    .iter()
                    .map(|c| c.name.to_string())
                    .chain(APP_COMMANDS.iter().map(|s| s.to_string()))
                    .filter(|n| n.starts_with(&input))
                    .collect();
                names.sort();
                (names, 0)
            }
            Some(("settings", _)) => {
                let word_start = input.rfind(' ').map(|i| i + 1).unwrap_or(0);
                let word = &input[word_start..];
                let keys = Settings::KEYS
                    .iter()
                    .map(|(k, _)| format!("{k}="))
                    .filter(|k| k.starts_with(word))
                    .collect();
                (keys, word_start)
            }
            Some(_) => (Vec::new(), 0),
        };
        match options.len() {
            0 => {}
            1 => {
                let space = if options[0].ends_with('=') { "" } else { " " };
                self.console_input = format!("{}{}{space}", &input[..prefix_len], options[0]);
            }
            _ => {
                // Extend to the longest common prefix, then show the options.
                let common = options.iter().skip(1).fold(options[0].clone(), |acc, o| {
                    acc.chars()
                        .zip(o.chars())
                        .take_while(|(a, b)| a == b)
                        .map(|(a, _)| a)
                        .collect()
                });
                self.console_input = format!("{}{common}", &input[..prefix_len]);
                let prompt = self.prompt_place();
                self.log(LineKind::Input { prompt }, input);
                self.log(LineKind::Info, options.join("   "));
            }
        }
    }

    pub fn viewport(&mut self, ui: &mut Ui, look: &Look) {
        let out = viewport::show(
            ui,
            &self.session.doc,
            &mut self.camera,
            self.selected,
            &look.pal,
        );
        if let Some(hit) = out.clicked {
            self.selected = hit;
        }
    }

    pub fn status_line(&self, ui: &mut Ui, look: &Look) {
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
            RichText::new(format!("Copal {}", copal_core::VERSION))
                .small()
                .color(look.pal.accent_text),
        );
        ui.label(
            RichText::new(format!(
                "·  {}  ·  {file}{dirty}  ·  {} objects",
                doc.name,
                doc.objects().len()
            ))
            .small()
            .color(look.pal.text_faint),
        );
    }

    /// The settings window. Changes apply at once and are saved.
    pub fn settings_window(&mut self, ctx: &egui::Context, look: &Look) {
        let mut open = self.show_settings;
        let mut s = self.settings.clone();
        egui::Window::new("Settings")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(320.0)
            .show(ctx, |ui| {
                widgets::section(ui, look, "Theme");
                ui.radio_value(
                    &mut s.theme,
                    ThemeStyle::Default,
                    "Default — calm colors, orange outlines",
                );
                ui.radio_value(&mut s.theme, ThemeStyle::Art, "Art — amber crystal borders");
                widgets::section(ui, look, "Colors");
                ui.add_enabled_ui(s.theme == ThemeStyle::Default, |ui| {
                    ui.horizontal(|ui| {
                        ui.radio_value(&mut s.mode, ColorMode::Dark, "Dark");
                        ui.radio_value(&mut s.mode, ColorMode::Light, "Light");
                    });
                });
                if s.theme == ThemeStyle::Art {
                    ui.label(
                        RichText::new("The art theme is dark only.")
                            .small()
                            .color(look.pal.text_faint),
                    );
                }
                ui.add_space(8.0);
                let where_ = Settings::path()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default();
                ui.label(
                    RichText::new(format!("Saved to {where_}"))
                        .small()
                        .color(look.pal.text_faint),
                );
                ui.label(
                    RichText::new("Also: `settings theme=art` in the console or copal-cli.")
                        .small()
                        .color(look.pal.text_faint),
                );
            });
        self.show_settings = open;
        self.set_settings(s);
    }
}

fn prompt_labels(ui: &mut Ui, look: &Look, place: &str, font: &FontId) {
    let p = look.pal;
    if look.art() {
        ui.label(RichText::new("› ").font(font.clone()).color(p.term_user));
    } else {
        ui.label(
            RichText::new("copal")
                .font(font.clone())
                .color(p.term_user)
                .strong(),
        );
        ui.label(RichText::new(":").font(font.clone()).color(p.term_fg));
        ui.label(
            RichText::new(place)
                .font(font.clone())
                .color(p.term_path)
                .strong(),
        );
        ui.label(RichText::new("$ ").font(font.clone()).color(p.term_fg));
    }
}

fn move_cursor_to_end(ui: &Ui, id: Id, text: &str) {
    if let Some(mut state) = TextEdit::load_state(ui.ctx(), id) {
        let end = CCursor::new(text.chars().count());
        state.cursor.set_char_range(Some(CCursorRange::one(end)));
        state.store(ui.ctx(), id);
    }
}

fn planned(ui: &mut Ui, look: &Look, title: &str, text: &str) {
    widgets::section(ui, look, title);
    ui.label(RichText::new(text).color(look.pal.text_weak));
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
