//! Panels both apps use: step tabs, tools, settings tabs, scene list, console, status line,
//! preferences and the print history.

use copal_core::presets::Kind;
use copal_core::prints::Outcome;
use copal_core::time::format_duration;
use copal_core::{ColorMode, ObjectId, Settings, ThemeStyle, library, script::quote};
use egui::text::{CCursor, CCursorRange};
use egui::{
    Color32, DragValue, FontId, Id, Key, Modifiers, RichText, ScrollArea, Sense, TextEdit, Ui, vec2,
};

use crate::files::Purpose;
use crate::gizmo::{self, AXIS_COLORS};
use crate::look::{Edge, Look, State};
use crate::widgets::{self, ButtonKind};
use crate::workspace::{APP_COMMANDS, LineKind, SettingsTab, Step, Workspace};

/// A horizontal tab (steps in the top bar, settings tabs). Wraps to a new row when the row is
/// full, because it is laid out in a wrapping row by the caller.
pub fn tab_button(
    ui: &mut Ui,
    look: &Look,
    selected: bool,
    text: &str,
    big: bool,
) -> egui::Response {
    let font = if big {
        FontId::proportional(14.0)
    } else {
        FontId::proportional(12.5)
    };
    let text = if big {
        text.to_uppercase()
    } else {
        text.to_string()
    };
    let galley = ui
        .painter()
        .layout_no_wrap(text, font, Color32::PLACEHOLDER);
    let pad = if big {
        vec2(18.0, 9.0)
    } else {
        vec2(10.0, 5.0)
    };
    let size = galley.size() + pad * 2.0;
    let (rect, r) = ui.allocate_exact_size(size, Sense::click());
    if ui.is_rect_visible(rect) {
        let p = &look.pal;
        let painter = ui.painter();
        if selected {
            painter.rect_filled(rect, look.radius(Edge::Button), p.raised);
            look.outline(painter, rect, Edge::Button, State::Selected);
        } else if r.hovered() {
            painter.rect_filled(rect, look.radius(Edge::Button), p.raised_hover);
        }
        let color = match (selected, r.hovered()) {
            (true, _) => p.accent_text,
            (false, true) => p.text,
            (false, false) => p.text_weak,
        };
        painter.galley(rect.center() - galley.size() / 2.0, galley, color);
    }
    r
}

/// A folded tool window: a thin stripe with its name, like in IntelliJ. `vertical` turns the text
/// for the left and right edges. Returns the click response.
pub fn stripe_button(ui: &mut Ui, look: &Look, text: &str, vertical: bool) -> egui::Response {
    let font = FontId::proportional(12.5);
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_string(), font, Color32::PLACEHOLDER);
    let gs = galley.size();
    let size = if vertical {
        vec2(gs.y + 10.0, gs.x + 18.0)
    } else {
        vec2(gs.x + 18.0, gs.y + 8.0)
    };
    let (rect, r) = ui.allocate_exact_size(size, Sense::click());
    let p = &look.pal;
    if r.hovered() {
        ui.painter()
            .rect_filled(rect, look.radius(Edge::Button), p.raised_hover);
    }
    let color = if r.hovered() { p.text } else { p.text_weak };
    if vertical {
        // Reads bottom to top, like IDE side stripes.
        let pos = egui::pos2(rect.center().x - gs.y / 2.0, rect.center().y + gs.x / 2.0);
        ui.painter().add(
            egui::epaint::TextShape::new(pos, galley, color)
                .with_angle(-std::f32::consts::FRAC_PI_2),
        );
    } else {
        ui.painter().galley(rect.center() - gs / 2.0, galley, color);
    }
    r
}

/// Title row of an open tool window with a button that folds it away. Returns true when the
/// button was clicked.
pub fn window_header(ui: &mut Ui, look: &Look, title: &str, hotkey: &str) -> bool {
    let mut close = false;
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(title.to_uppercase())
                .small()
                .color(look.pal.text_faint),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let r = ui
                .add(egui::Button::new(RichText::new("—").color(look.pal.text_weak)).frame(false))
                .on_hover_text(format!("Hide ({hotkey})"));
            close = r.clicked();
        });
    });
    close
}

/// One number field that changes a project setting. Dragging previews the value; the command
/// runs when the drag ends or a typed value is confirmed, so undo gets one step.
#[allow(clippy::too_many_arguments)]
fn num(
    ws: &mut Workspace,
    ui: &mut Ui,
    section: &str,
    key: &str,
    index: Option<(usize, [f32; 2])>,
    value: f64,
    speed: f64,
    range: std::ops::RangeInclusive<f64>,
    suffix: &str,
) {
    let edit_key = format!("{section}.{key}.{}", index.map(|i| i.0).unwrap_or(0));
    let mut v = ws.field_edit.get(&edit_key).copied().unwrap_or(value);
    let whole = matches!(key, "bottom_layers" | "transition_layers" | "light_pwm");
    let r = ui.add(
        DragValue::new(&mut v)
            .speed(speed)
            .range(range)
            .max_decimals(if whole { 0 } else { 3 })
            .suffix(suffix),
    );
    if whole {
        v = v.round();
    }
    if r.dragged() {
        ws.field_edit.insert(edit_key.clone(), v);
    }
    let commit = r.drag_stopped() || (r.changed() && !r.dragged());
    if commit {
        ws.field_edit.remove(&edit_key);
        let text = match index {
            Some((i, mut pair)) => {
                pair[i] = v as f32;
                format!("{},{}", pair[0], pair[1])
            }
            None => format!("{v}"),
        };
        ws.run(&format!("{section}.set {key}={text}"));
    }
}

fn check(
    ws: &mut Workspace,
    ui: &mut Ui,
    section: &str,
    key: &str,
    value: bool,
    label: &str,
    hint: &str,
) {
    let mut v = value;
    if ui.checkbox(&mut v, label).on_hover_text(hint).changed() {
        ws.run(&format!("{section}.set {key}={v}"));
    }
}

fn row_label(ui: &mut Ui, look: &Look, text: &str, hint: &str) {
    ui.label(RichText::new(text).color(look.pal.text_weak))
        .on_hover_text(hint);
}

impl Workspace {
    // --------------------------------------------------------------------------- top bar

    /// The three workflow steps as big tabs.
    pub fn step_tabs(&mut self, ui: &mut Ui, look: &Look) {
        for (i, (step, name)) in Step::ALL.into_iter().enumerate() {
            let text = format!("{}  {name}", i + 1);
            if tab_button(ui, look, self.step == step, &text, true).clicked() {
                self.step = step;
            }
        }
    }

    pub fn printer_resin_pickers(&mut self, ui: &mut Ui, look: &Look) {
        let lib = library();
        let doc = self.doc();
        let printer = doc.printer.clone().unwrap_or_default();
        let resin = doc.resin.clone().unwrap_or_default();
        let printer_name = lib
            .printer(&printer)
            .map(|p| p.name())
            .unwrap_or_else(|| "Choose a printer".into());
        let resin_name = lib
            .resin(&resin)
            .map(|r| r.title())
            .unwrap_or_else(|| "Choose a resin".into());
        let mut new_printer = None;
        let mut new_resin = None;
        ui.label(RichText::new("Printer").color(look.pal.text_weak));
        egui::ComboBox::from_id_salt("printer")
            .selected_text(printer_name)
            .width(200.0)
            .show_ui(ui, |ui| {
                for p in &lib.printers {
                    let label = if p.status == copal_core::presets::Status::Experimental {
                        format!("{}  (experimental)", p.name())
                    } else {
                        p.name()
                    };
                    if ui.selectable_label(p.id == printer, label).clicked() {
                        new_printer = Some(p.id.clone());
                    }
                }
            });
        ui.label(RichText::new("Resin").color(look.pal.text_weak));
        egui::ComboBox::from_id_salt("resin")
            .selected_text(resin_name)
            .width(220.0)
            .show_ui(ui, |ui| {
                let (fits, other): (Vec<_>, Vec<_>) = lib
                    .resins
                    .iter()
                    .partition(|r| r.profile_for(&printer).is_some());
                for r in fits {
                    if ui.selectable_label(r.id == resin, r.title()).clicked() {
                        new_resin = Some(r.id.clone());
                    }
                }
                if !other.is_empty() {
                    ui.separator();
                    ui.label(
                        RichText::new("No settings for this printer:")
                            .small()
                            .color(look.pal.text_faint),
                    );
                    for r in other {
                        if ui.selectable_label(r.id == resin, r.title()).clicked() {
                            new_resin = Some(r.id.clone());
                        }
                    }
                }
            });
        if let Some(p) = new_printer {
            self.run(&format!("project.set_printer printer={}", quote(&p)));
            self.framed = false;
        }
        if let Some(r) = new_resin {
            self.run(&format!("project.set_resin resin={}", quote(&r)));
        }
    }

    // --------------------------------------------------------------------------- left: tools

    /// Tools of the current workflow step.
    pub fn tool_shelf(&mut self, ui: &mut Ui, look: &Look) {
        let w = Some(ui.available_width());
        match self.step {
            Step::Layout => self.layout_tools(ui, look, w),
            Step::Prepare => self.prepare_tools(ui, look, w),
            Step::Export => self.export_tools(ui, look, w),
        }
    }

    fn layout_tools(&mut self, ui: &mut Ui, look: &Look, w: Option<f32>) {
        widgets::section(ui, look, "Add");
        if widgets::button_sized(ui, look, ButtonKind::Primary, "Import models…", w)
            .on_hover_text("STL, OBJ or 3MF (Ctrl+I). You can also drop files on the window.")
            .clicked()
        {
            self.open_dialog(Purpose::ImportModel);
        }
        if widgets::button_sized(ui, look, ButtonKind::Normal, "Add test cube", w)
            .on_hover_text("A 20 mm cube, handy for a first test print")
            .clicked()
        {
            self.run("object.add_cube size=20");
        }
        let hint = format!(
            "Every model you exported, with its supports and settings, ready to open again ({})",
            self.key_text("history")
        );
        if widgets::button_sized(ui, look, ButtonKind::Normal, "Open from print history…", w)
            .on_hover_text(hint)
            .clicked()
        {
            self.show_history = true;
        }

        // The handles first, then what they can't do, in rows of equal buttons.
        widgets::section(ui, look, "Transform");
        ui.horizontal_wrapped(|ui| {
            for (mode, name, id) in [
                (gizmo::Mode::Move, "Move", "mode_move"),
                (gizmo::Mode::Rotate, "Rotate", "mode_rotate"),
                (gizmo::Mode::Scale, "Scale", "mode_scale"),
            ] {
                let key = self.key_text(id);
                if tab_button(ui, look, self.gizmo_mode == mode, name, false)
                    .on_hover_text(format!(
                        "{key} — drag the handles in the 3D view. Ctrl snaps. \
                         Change keys in Keys (F1)."
                    ))
                    .clicked()
                {
                    self.gizmo_mode = mode;
                }
            }
        });
        ui.label(
            RichText::new("Drag a model to move it on the plate. Arrow keys nudge it.")
                .small()
                .color(look.pal.text_faint),
        );
        let sel = self.selected;
        let on = sel.is_some();
        let id = sel.map(|i| i.0).unwrap_or(0);
        match widgets::button_row(
            ui,
            look,
            &[
                ("On plate", "P. Put the model down on the build plate", on),
                ("Center", "C. Move the model to the middle of the plate", on),
            ],
        ) {
            Some(0) => self.run(&format!("object.drop id={id}")),
            Some(_) => self.run(&format!("object.center id={id}")),
            None => {}
        }
        let busy = self.job_running();
        let hint = format!(
            "{}. Turn the model the best way up for printing: tilted on two axes, small layers \
             that grow slowly, few overhangs, steady on its supports. More choices in Prepare.",
            self.key_text("orient")
        );
        if widgets::button_row(
            ui,
            look,
            &[(
                "Auto orient",
                &hint,
                !busy && !self.doc().objects().is_empty(),
            )],
        )
        .is_some()
        {
            self.auto_orient(false);
        }
        match widgets::button_row(
            ui,
            look,
            &[
                (
                    "Reset rotation",
                    "Ctrl+R. Turn the model back to how it was imported",
                    on,
                ),
                ("Reset all", "Rotation and size as imported", on),
            ],
        ) {
            Some(0) => self.run(&format!("object.reset id={id}")),
            Some(_) => self.run(&format!("object.reset id={id} what=all")),
            None => {}
        }
        if let Some(i) = widgets::button_row(
            ui,
            look,
            &[
                ("Mirror X", "Alt+X", on),
                ("Mirror Y", "Alt+Y", on),
                ("Mirror Z", "Alt+Z", on),
            ],
        ) {
            let axis = ["x", "y", "z"][i];
            self.run(&format!("object.mirror id={id} axis={axis}"));
        }
        match widgets::button_row(
            ui,
            look,
            &[
                (
                    "Duplicate",
                    "Ctrl+D. A copy on a free place of the plate",
                    on,
                ),
                ("Delete", "Delete or X", on),
            ],
        ) {
            Some(0) => self.run(&format!("object.duplicate id={id}")),
            Some(_) => self.run(&format!("object.remove id={id}")),
            None => {}
        }

        widgets::section(ui, look, "Plate");
        let any = !self.doc().objects().is_empty();
        match widgets::button_row(
            ui,
            look,
            &[
                (
                    "Arrange all",
                    "Shift+A. Place every model in neat rows",
                    any,
                ),
                ("Undo", "Ctrl+Z", self.session().history.can_undo()),
                ("Redo", "Ctrl+Shift+Z", self.session().history.can_redo()),
            ],
        ) {
            Some(0) => self.run("object.arrange"),
            Some(1) => self.undo(),
            Some(_) => self.redo(),
            None => {}
        }
        let outside = self.outside_plate();
        if !outside.is_empty() {
            ui.label(
                RichText::new(format!(
                    "Outside the build volume: {}. Arrange, move or scale them before export.",
                    outside.join(", ")
                ))
                .small()
                .color(look.pal.error),
            );
        }
        widgets::section(ui, look, "Objects");
        self.scene_list(ui, look);
    }

    fn export_tools(&mut self, ui: &mut Ui, look: &Look, w: Option<f32>) {
        let doc = self.doc();
        let lib = library();
        let printer = doc.printer.as_deref().and_then(|p| lib.printer(p)).cloned();
        widgets::section(ui, look, "Slice");
        let e = doc.settings.print.exposure;
        let (layers, height) = doc
            .bounds()
            .map(|b| {
                (
                    (b.max.z.max(0.0) / e.layer_height_mm).ceil() as u32,
                    b.max.z,
                )
            })
            .unwrap_or((0, 0.0));
        let time = doc.settings.print.estimate_time_s(layers);
        let info = |ui: &mut Ui, k: &str, v: String| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(k).color(look.pal.text_weak));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(v).color(look.pal.text));
                });
            });
        };
        info(
            ui,
            "Printer",
            printer.as_ref().map(|p| p.name()).unwrap_or("—".into()),
        );
        info(
            ui,
            "File",
            printer
                .as_ref()
                .map(|p| format!(".{}", p.file.format))
                .unwrap_or("—".into()),
        );
        info(ui, "Layers", format!("{layers} × {} mm", e.layer_height_mm));
        info(ui, "Height", format!("{height:.1} mm"));
        info(
            ui,
            "Print time",
            format!("≈ {}", format_duration(time as f64)),
        );
        ui.add_space(6.0);
        if let Some(job) = &self.export_job {
            let f = job.fraction();
            ui.add(egui::ProgressBar::new(f).show_percentage().animate(true));
            if widgets::button_sized(ui, look, ButtonKind::Normal, "Cancel (Esc)", w).clicked() {
                job.cancel.store(true, std::sync::atomic::Ordering::Relaxed);
            }
        } else {
            let can = !doc.objects().is_empty() && printer.is_some();
            ui.add_enabled_ui(can, |ui| {
                if widgets::button_sized(ui, look, ButtonKind::Primary, "Slice & export…", w)
                    .on_hover_text("Ctrl+E")
                    .clicked()
                {
                    self.open_dialog(Purpose::Export);
                }
            });
        }
        match &self.last_export {
            Some(Ok(r)) => {
                ui.label(
                    RichText::new(format!(
                        "Saved {} — {} layers, {:.1} ml, ≈ {}",
                        r.path
                            .file_name()
                            .map(|n| n.to_string_lossy())
                            .unwrap_or_default(),
                        r.layers,
                        r.volume_ml,
                        format_duration(r.print_time_s as f64)
                    ))
                    .small()
                    .color(look.pal.text_weak),
                );
            }
            Some(Err(e)) => {
                ui.label(RichText::new(e).small().color(look.pal.error));
            }
            None => {}
        }
        widgets::section(ui, look, "Preview");
        if widgets::button_sized(
            ui,
            look,
            ButtonKind::Normal,
            if self.preview.show {
                "Show 3D view"
            } else {
                "Show layers"
            },
            w,
        )
        .clicked()
        {
            self.preview.show = !self.preview.show;
        }
        widgets::section(ui, look, "Print history");
        let list = self
            .shell
            .history
            .as_ref()
            .map(|h| h.list())
            .unwrap_or_default();
        if list.is_empty() {
            ui.label(
                RichText::new("Every export is saved here with a copy of the project.")
                    .small()
                    .color(look.pal.text_faint),
            );
        }
        for r in list.iter().take(6) {
            let stars = r.rating.map(|n| format!("· {n}/5")).unwrap_or_default();
            let label = format!(
                "{}  {}",
                r.output
                    .file_name()
                    .map(|n| n.to_string_lossy())
                    .unwrap_or_default(),
                stars
            );
            if ui
                .selectable_label(false, label)
                .on_hover_text(format!(
                    "{} · {} layers · {}",
                    r.date,
                    r.layers,
                    format_duration(r.print_time_s as f64)
                ))
                .clicked()
            {
                self.show_history = true;
            }
        }
        if widgets::button_sized(ui, look, ButtonKind::Normal, "Open history…", w).clicked() {
            self.show_history = true;
        }
    }

    /// Object list.
    pub fn scene_list(&mut self, ui: &mut Ui, look: &Look) {
        if self.doc().objects().is_empty() {
            ui.label(RichText::new("No objects yet").color(look.pal.text_faint));
            return;
        }
        let items: Vec<(ObjectId, String, usize)> = self
            .doc()
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
            let tris = if tris >= 1000 {
                format!("{}k", tris / 1000)
            } else {
                tris.to_string()
            };
            let text = RichText::new(format!("{name}   ·  {tris} tri")).color(color);
            if ui.selectable_label(selected, text).clicked() {
                self.selected = Some(id);
            }
        }
    }

    // --------------------------------------------------------------------------- right: settings

    /// The settings tabs. They wrap to more rows when the panel is narrow.
    pub fn settings_panel(&mut self, ui: &mut Ui, look: &Look) {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
            for (tab, name) in SettingsTab::ALL {
                if tab_button(ui, look, self.tab == tab, name, false).clicked() {
                    self.tab = tab;
                }
            }
        });
        ui.add_space(4.0);
        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| match self.tab {
                SettingsTab::Object => self.inspector(ui, look),
                SettingsTab::Resin => self.resin_tab(ui, look),
                SettingsTab::Print => self.print_tab(ui, look),
                SettingsTab::Layout => self.layout_tab(ui, look),
                SettingsTab::Supports => self.supports_tab(ui, look),
                SettingsTab::Hollow => self.hollow_tab(ui, look),
            });
    }

    /// Load / save / delete personal presets of one kind.
    fn preset_row(
        &mut self,
        ui: &mut Ui,
        look: &Look,
        kind: Kind,
        kind_name: &str,
        builtin: &[&str],
    ) {
        let title = format!(
            "{}{} presets",
            kind_name[..1].to_uppercase(),
            &kind_name[1..]
        );
        widgets::section(ui, look, &title);
        let saved = copal_core::user_presets()
            .map(|u| u.list(kind))
            .unwrap_or_default();
        let mut load = None;
        let mut delete = None;
        ui.horizontal_wrapped(|ui| {
            egui::ComboBox::from_id_salt(("preset-load", kind_name))
                .selected_text("Load…")
                .width(130.0)
                .show_ui(ui, |ui| {
                    for b in builtin {
                        if ui.selectable_label(false, *b).clicked() {
                            load = Some(b.to_string());
                        }
                    }
                    if !builtin.is_empty() && !saved.is_empty() {
                        ui.separator();
                    }
                    for s in &saved {
                        if ui.selectable_label(false, s).clicked() {
                            load = Some(s.clone());
                        }
                    }
                    if builtin.is_empty() && saved.is_empty() {
                        ui.label(RichText::new("Nothing saved yet").color(look.pal.text_faint));
                    }
                });
            if !saved.is_empty() {
                egui::ComboBox::from_id_salt(("preset-delete", kind_name))
                    .selected_text("Delete…")
                    .width(100.0)
                    .show_ui(ui, |ui| {
                        for s in &saved {
                            if ui.selectable_label(false, s).clicked() {
                                delete = Some(s.clone());
                            }
                        }
                    });
            }
        });
        ui.horizontal(|ui| {
            ui.add(
                TextEdit::singleline(&mut self.preset_name)
                    .hint_text("name")
                    .desired_width(ui.available_width() - 70.0),
            );
            let ok = !self.preset_name.trim().is_empty();
            ui.add_enabled_ui(ok, |ui| {
                if widgets::button(ui, look, ButtonKind::Normal, "Save").clicked() {
                    let name = std::mem::take(&mut self.preset_name);
                    self.run(&format!("presets save {kind_name} {}", quote(name.trim())));
                }
            });
        });
        if let Some(n) = load {
            self.run(&format!("presets load {kind_name} {}", quote(&n)));
        }
        if let Some(n) = delete {
            self.run(&format!("presets delete {kind_name} {}", quote(&n)));
        }
    }

    fn resin_tab(&mut self, ui: &mut Ui, look: &Look) {
        let e = self.doc().settings.print.exposure;
        widgets::section(ui, look, "Layers and exposure");
        egui::Grid::new("exposure")
            .num_columns(2)
            .spacing([8.0, 6.0])
            .show(ui, |ui| {
                row_label(ui, look, "Layer height", "Thickness of one layer");
                num(
                    self,
                    ui,
                    "print",
                    "layer_height_mm",
                    None,
                    e.layer_height_mm as f64,
                    0.001,
                    0.01..=0.2,
                    " mm",
                );
                ui.end_row();
                row_label(ui, look, "Exposure", "UV time for normal layers");
                num(
                    self,
                    ui,
                    "print",
                    "exposure_s",
                    None,
                    e.exposure_s as f64,
                    0.05,
                    0.1..=60.0,
                    " s",
                );
                ui.end_row();
                row_label(
                    ui,
                    look,
                    "Bottom layers",
                    "First layers, exposed longer so the print sticks to the plate",
                );
                num(
                    self,
                    ui,
                    "print",
                    "bottom_layers",
                    None,
                    e.bottom_layers as f64,
                    0.2,
                    0.0..=100.0,
                    "",
                );
                ui.end_row();
                row_label(ui, look, "Bottom exposure", "UV time for bottom layers");
                num(
                    self,
                    ui,
                    "print",
                    "bottom_exposure_s",
                    None,
                    e.bottom_exposure_s as f64,
                    0.5,
                    1.0..=200.0,
                    " s",
                );
                ui.end_row();
                row_label(
                    ui,
                    look,
                    "Transition layers",
                    "Layers between bottom and normal, exposure steps down",
                );
                num(
                    self,
                    ui,
                    "print",
                    "transition_layers",
                    None,
                    e.transition_layers as f64,
                    0.2,
                    0.0..=50.0,
                    "",
                );
                ui.end_row();
                row_label(ui, look, "Light power", "UV PWM, 0–255");
                num(
                    self,
                    ui,
                    "print",
                    "light_pwm",
                    None,
                    e.light_pwm as f64,
                    1.0,
                    1.0..=255.0,
                    "",
                );
                ui.end_row();
            });
        ui.label(
            RichText::new("Exposure depends on your bottle, its color and the room temperature. Save what works as your own resin.")
                .small()
                .color(look.pal.text_faint),
        );
        self.preset_row(ui, look, Kind::Exposure, "exposure", &[]);
        widgets::section(ui, look, "Price");
        self.resin_price_editor(ui, look);
        self.my_resin_section(ui, look);
    }

    /// Price of the resin: amount, per kilogram or litre, currency, density. Saved with
    /// "Save as my resin", so the resin file carries it.
    pub(crate) fn resin_price_editor(&mut self, ui: &mut Ui, look: &Look) {
        let c = self.doc().settings.print.cost;
        egui::Grid::new("resin-price")
            .num_columns(2)
            .spacing([8.0, 6.0])
            .show(ui, |ui| {
                row_label(
                    ui,
                    look,
                    "Price",
                    "What a bottle costs per kilogram or per litre. 0 = not known.",
                );
                ui.horizontal(|ui| {
                    num(
                        self,
                        ui,
                        "print",
                        "resin_price",
                        None,
                        c.resin_price as f64,
                        0.1,
                        0.0..=1_000_000.0,
                        "",
                    );
                    let key = "print.currency";
                    let mut cur = self
                        .text_edit
                        .get(key)
                        .cloned()
                        .unwrap_or_else(|| c.currency.to_string());
                    let r = ui.add(
                        TextEdit::singleline(&mut cur)
                            .desired_width(44.0)
                            .hint_text("$"),
                    );
                    if r.changed() {
                        self.text_edit.insert(key.into(), cur.clone());
                    }
                    if r.lost_focus() {
                        self.text_edit.remove(key);
                        if cur.trim() != c.currency.as_str() {
                            self.run(&format!(
                                "print.set currency={}",
                                copal_core::script::quote(cur.trim())
                            ));
                        }
                    }
                    ui.label(RichText::new("per").color(look.pal.text_weak));
                    for (unit, name, hint) in [
                        (
                            copal_core::presets::PriceUnit::Kg,
                            "kg",
                            "Price of a kilogram",
                        ),
                        (
                            copal_core::presets::PriceUnit::Liter,
                            "litre",
                            "Price of a litre",
                        ),
                    ] {
                        if tab_button(ui, look, c.price_per == unit, name, false)
                            .on_hover_text(hint)
                            .clicked()
                            && c.price_per != unit
                        {
                            let v = if unit == copal_core::presets::PriceUnit::Kg {
                                "kg"
                            } else {
                                "l"
                            };
                            self.run(&format!("print.set price_per={v}"));
                        }
                    }
                });
                ui.end_row();
                row_label(
                    ui,
                    look,
                    "Density",
                    "Grams per millilitre, for the weight. Most resins are 1.05 – 1.25.",
                );
                num(
                    self,
                    ui,
                    "print",
                    "resin_density_g_ml",
                    None,
                    c.resin_density_g_ml as f64,
                    0.01,
                    0.5..=3.0,
                    " g/ml",
                );
                ui.end_row();
            });
    }

    fn print_tab(&mut self, ui: &mut Ui, look: &Look) {
        let m = self.doc().settings.print.motion;
        widgets::section(ui, look, "Anti-aliasing");
        let mut aa = m.anti_aliasing;
        egui::ComboBox::from_id_salt("aa")
            .selected_text(if aa <= 1 {
                "Off".to_string()
            } else {
                format!("{aa}×")
            })
            .show_ui(ui, |ui| {
                for v in [1u8, 2, 4, 8] {
                    ui.selectable_value(
                        &mut aa,
                        v,
                        if v == 1 {
                            "Off".to_string()
                        } else {
                            format!("{v}×")
                        },
                    );
                }
            });
        if aa != m.anti_aliasing {
            self.run(&format!("print.set anti_aliasing={aa}"));
        }
        for (title, bottom) in [
            ("Bottom layers: lift and retract", true),
            ("Normal layers: lift and retract", false),
        ] {
            widgets::section(ui, look, title);
            let p = if bottom { "bottom_" } else { "" };
            let (lift, lift_v, ret, ret_v, rest_e, rest_r) = if bottom {
                (
                    m.bottom_lift_mm,
                    m.bottom_lift_speed,
                    m.bottom_retract_mm,
                    m.bottom_retract_speed,
                    m.bottom_rest_after_exposure_s,
                    m.bottom_rest_after_retract_s,
                )
            } else {
                (
                    m.lift_mm,
                    m.lift_speed,
                    m.retract_mm,
                    m.retract_speed,
                    m.rest_after_exposure_s,
                    m.rest_after_retract_s,
                )
            };
            egui::Grid::new(("motion", bottom)).num_columns(3).spacing([6.0, 6.0]).show(ui, |ui| {
                ui.label("");
                ui.label(RichText::new("stage 1").small().color(look.pal.text_faint));
                ui.label(RichText::new("stage 2").small().color(look.pal.text_faint));
                ui.end_row();
                for (label, key, pair, speed, suffix) in [
                    ("Lift", "lift_mm", lift, 0.1, " mm"),
                    ("Lift speed", "lift_speed", lift_v, 1.0, " mm/min"),
                    ("Retract", "retract_mm", ret, 0.1, " mm"),
                    ("Retract speed", "retract_speed", ret_v, 1.0, " mm/min"),
                ] {
                    row_label(ui, look, label, "Two-stage moves: the first stage is slow to peel gently. Stage 2 = 0 turns it off.");
                    let k = format!("{p}{key}");
                    for i in 0..2 {
                        num(self, ui, "print", &k, Some((i, pair)), pair[i] as f64, speed, 0.0..=1000.0, suffix);
                    }
                    ui.end_row();
                }
            });
            egui::Grid::new(("rest", bottom))
                .num_columns(2)
                .spacing([8.0, 6.0])
                .show(ui, |ui| {
                    row_label(
                        ui,
                        look,
                        "Wait after exposure",
                        "Pause before the plate lifts",
                    );
                    num(
                        self,
                        ui,
                        "print",
                        &format!("{p}rest_after_exposure_s"),
                        None,
                        rest_e as f64,
                        0.05,
                        0.0..=60.0,
                        " s",
                    );
                    ui.end_row();
                    row_label(
                        ui,
                        look,
                        "Wait after retract",
                        "Pause before the next exposure, lets the resin settle",
                    );
                    num(
                        self,
                        ui,
                        "print",
                        &format!("{p}rest_after_retract_s"),
                        None,
                        rest_r as f64,
                        0.05,
                        0.0..=60.0,
                        " s",
                    );
                    ui.end_row();
                });
        }
        self.preset_row(ui, look, Kind::Motion, "motion", &[]);
        widgets::section(ui, look, "Printer");
        if let Some(p) = self
            .doc()
            .printer
            .as_deref()
            .and_then(|id| library().printer(id).cloned())
        {
            let px = p.pixel_mm();
            for line in [
                format!(
                    "{} × {} px, pixel {:.1} × {:.1} µm",
                    p.screen.resolution[0],
                    p.screen.resolution[1],
                    px[0] * 1000.0,
                    px[1] * 1000.0
                ),
                format!(
                    "Build volume {:.1} × {:.1} × {:.0} mm",
                    p.screen.size_mm[0], p.screen.size_mm[1], p.build.height_mm
                ),
                format!("File .{} {}", p.file.format, p.file.format_version),
            ] {
                ui.label(RichText::new(line).small().color(look.pal.text_weak));
            }
        }
        self.my_printer_section(ui, look);
    }

    fn layout_tab(&mut self, ui: &mut Ui, look: &Look) {
        let l = self.doc().settings.layout;
        widgets::section(ui, look, "Moving models");
        check(
            self,
            ui,
            "layout",
            "keep_on_plate",
            l.keep_on_plate,
            "Keep models on the plate",
            "After rotating or scaling, a model that stood on the plate stays on it",
        );
        egui::Grid::new("layout")
            .num_columns(2)
            .spacing([8.0, 6.0])
            .show(ui, |ui| {
                row_label(
                    ui,
                    look,
                    "Spacing",
                    "Gap between models for Arrange and new imports",
                );
                num(
                    self,
                    ui,
                    "layout",
                    "arrange_spacing_mm",
                    None,
                    l.arrange_spacing_mm as f64,
                    0.1,
                    0.0..=50.0,
                    " mm",
                );
                ui.end_row();
                row_label(ui, look, "Move snap", "Step when Ctrl is held while moving");
                num(
                    self,
                    ui,
                    "layout",
                    "move_snap_mm",
                    None,
                    l.move_snap_mm as f64,
                    0.1,
                    0.0..=50.0,
                    " mm",
                );
                ui.end_row();
                row_label(
                    ui,
                    look,
                    "Rotate snap",
                    "Step when Ctrl is held while rotating",
                );
                num(
                    self,
                    ui,
                    "layout",
                    "rotate_snap_deg",
                    None,
                    l.rotate_snap_deg as f64,
                    0.5,
                    0.0..=90.0,
                    "°",
                );
                ui.end_row();
                row_label(
                    ui,
                    look,
                    "Scale snap",
                    "Step when Ctrl is held while scaling",
                );
                num(
                    self,
                    ui,
                    "layout",
                    "scale_snap_percent",
                    None,
                    l.scale_snap_percent as f64,
                    0.5,
                    0.0..=100.0,
                    " %",
                );
                ui.end_row();
            });
        widgets::section(ui, look, "Handles");
        let mut s = self.settings.clone();
        ui.horizontal(|ui| {
            ui.label(RichText::new("Gizmo size").color(look.pal.text_weak));
            let (lo, hi) = copal_core::settings::GIZMO_SIZES;
            ui.add(egui::Slider::new(&mut s.gizmo_size, lo..=hi).step_by(0.1));
        });
        self.set_settings(s);
        self.preset_row(ui, look, Kind::Layout, "layout", &[]);
    }

    fn supports_tab(&mut self, ui: &mut Ui, look: &Look) {
        let s = self.doc().settings.supports;
        ui.label(
            RichText::new("Used by automatic and manual supports. The overhang angle also colors the faces that need them.")
                .small()
                .color(look.pal.text_faint),
        );
        widgets::section(ui, look, "Shape");
        egui::Grid::new("supports")
            .num_columns(2)
            .spacing([8.0, 6.0])
            .show(ui, |ui| {
                for (label, key, v, hint) in [
                    (
                        "Tip diameter",
                        "tip_diameter_mm",
                        s.tip_diameter_mm,
                        "Where the support touches the model",
                    ),
                    (
                        "Tip length",
                        "tip_length_mm",
                        s.tip_length_mm,
                        "Length of the thin cone at the top",
                    ),
                    (
                        "Diameter",
                        "diameter_mm",
                        s.diameter_mm,
                        "Thickness of the support body",
                    ),
                    (
                        "Base diameter",
                        "base_diameter_mm",
                        s.base_diameter_mm,
                        "Foot on the raft",
                    ),
                ] {
                    row_label(ui, look, label, hint);
                    num(
                        self,
                        ui,
                        "supports",
                        key,
                        None,
                        v as f64,
                        0.01,
                        0.05..=10.0,
                        " mm",
                    );
                    ui.end_row();
                }
            });
        widgets::section(ui, look, "Placement");
        egui::Grid::new("supports2")
            .num_columns(2)
            .spacing([8.0, 6.0])
            .show(ui, |ui| {
                row_label(
                    ui,
                    look,
                    "Overhang angle",
                    "Downward faces flatter than this (from the plate) need supports",
                );
                num(
                    self,
                    ui,
                    "supports",
                    "overhang_angle_deg",
                    None,
                    s.overhang_angle_deg as f64,
                    0.5,
                    0.0..=89.0,
                    "°",
                );
                ui.end_row();
                row_label(
                    ui,
                    look,
                    "Density",
                    "More or fewer supports for the same area",
                );
                num(
                    self,
                    ui,
                    "supports",
                    "density",
                    None,
                    s.density as f64,
                    0.02,
                    0.1..=5.0,
                    "×",
                );
                ui.end_row();
                row_label(
                    ui,
                    look,
                    "Safe distance",
                    "Keep support bodies this far from the model",
                );
                num(
                    self,
                    ui,
                    "supports",
                    "safe_distance_mm",
                    None,
                    s.safe_distance_mm as f64,
                    0.05,
                    0.0..=10.0,
                    " mm",
                );
                ui.end_row();
                row_label(
                    ui,
                    look,
                    "Lift height",
                    "Raise the model this much when supports are added",
                );
                num(
                    self,
                    ui,
                    "supports",
                    "lift_height_mm",
                    None,
                    s.lift_height_mm as f64,
                    0.1,
                    0.0..=50.0,
                    " mm",
                );
                ui.end_row();
                row_label(ui, look, "Brace spacing", "Distance between cross braces");
                num(
                    self,
                    ui,
                    "supports",
                    "bracing_spacing_mm",
                    None,
                    s.bracing_spacing_mm as f64,
                    0.1,
                    1.0..=50.0,
                    " mm",
                );
                ui.end_row();
            });
        check(
            self,
            ui,
            "supports",
            "auto_bracing",
            s.auto_bracing,
            "Brace automatic supports",
            "Connect neighbouring supports with cross braces",
        );
        check(
            self,
            ui,
            "supports",
            "manual_bracing",
            s.manual_bracing,
            "Brace supports added by hand",
            "A new manual support connects to its neighbours",
        );
        check(
            self,
            ui,
            "supports",
            "auto_parent",
            s.auto_parent,
            "Let close supports share a trunk",
            "A support next to another leans over and joins it: fewer feet, a smaller raft",
        );
        check(
            self,
            ui,
            "supports",
            "support_islands",
            s.support_islands,
            "Support islands",
            "Parts of a layer with nothing under them",
        );
        widgets::section(ui, look, "Base");
        let base = self.base_choice();
        if let Some(i) = widgets::choice_row(ui, look, &crate::prep::BASES, Some(base))
            && i != base
        {
            self.set_base(i);
        }
        egui::Grid::new("raft")
            .num_columns(2)
            .spacing([8.0, 6.0])
            .show(ui, |ui| {
                row_label(ui, look, "Thickness", "");
                num(
                    self,
                    ui,
                    "supports",
                    "raft_thickness_mm",
                    None,
                    s.raft_thickness_mm as f64,
                    0.05,
                    0.1..=10.0,
                    " mm",
                );
                ui.end_row();
                row_label(
                    ui,
                    look,
                    "Margin",
                    "How far the raft reaches past the supports",
                );
                num(
                    self,
                    ui,
                    "supports",
                    "raft_margin_mm",
                    None,
                    s.raft_margin_mm as f64,
                    0.1,
                    0.0..=20.0,
                    " mm",
                );
                ui.end_row();
            });
        self.preset_row(
            ui,
            look,
            Kind::Supports,
            "supports",
            &["light", "medium", "heavy"],
        );
    }

    fn hollow_tab(&mut self, ui: &mut Ui, look: &Look) {
        let h = self.doc().settings.hollow;
        ui.label(
            RichText::new(
                "Used by Hollow and the drain holes in the Prepare step. Kept with the project.",
            )
            .small()
            .color(look.pal.text_faint),
        );
        widgets::section(ui, look, "Walls");
        egui::Grid::new("hollow")
            .num_columns(2)
            .spacing([8.0, 6.0])
            .show(ui, |ui| {
                row_label(ui, look, "Wall thickness", "");
                num(
                    self,
                    ui,
                    "hollow",
                    "wall_thickness_mm",
                    None,
                    h.wall_thickness_mm as f64,
                    0.05,
                    0.5..=20.0,
                    " mm",
                );
                ui.end_row();
                row_label(
                    ui,
                    look,
                    "Precision",
                    "Voxel size for the inner wall. Smaller is slower and finer.",
                );
                num(
                    self,
                    ui,
                    "hollow",
                    "voxel_mm",
                    None,
                    h.voxel_mm as f64,
                    0.01,
                    0.05..=5.0,
                    " mm",
                );
                ui.end_row();
                row_label(
                    ui,
                    look,
                    "Fill pockets under",
                    "Closed hollows smaller than this are filled again: they would only trap resin. 0 keeps them all.",
                );
                num(
                    self,
                    ui,
                    "hollow",
                    "min_pocket_mm3",
                    None,
                    h.min_pocket_mm3 as f64,
                    1.0,
                    0.0..=5000.0,
                    " mm³",
                );
                ui.end_row();
            });
        widgets::section(ui, look, "Drain holes");
        egui::Grid::new("holes")
            .num_columns(2)
            .spacing([8.0, 6.0])
            .show(ui, |ui| {
                row_label(ui, look, "Diameter", "");
                num(
                    self,
                    ui,
                    "hollow",
                    "drain_hole_diameter_mm",
                    None,
                    h.drain_hole_diameter_mm as f64,
                    0.05,
                    0.5..=20.0,
                    " mm",
                );
                ui.end_row();
                row_label(ui, look, "Depth", "How deep a hole goes into the wall");
                num(
                    self,
                    ui,
                    "hollow",
                    "drain_hole_depth_mm",
                    None,
                    h.drain_hole_depth_mm as f64,
                    0.05,
                    0.5..=30.0,
                    " mm",
                );
                ui.end_row();
            });
        self.preset_row(ui, look, Kind::Hollow, "hollow", &[]);
    }

    /// Transform fields of the selected object.
    pub fn inspector(&mut self, ui: &mut Ui, look: &Look) {
        let Some(id) = self.selected else {
            ui.label(
                RichText::new("Select an object to see its settings").color(look.pal.text_faint),
            );
            widgets::section(ui, look, "Objects");
            self.scene_list(ui, look);
            return;
        };
        let Some(obj) = self.doc().object(id).cloned() else {
            return;
        };
        let mut name = obj.name.clone();
        let mut t = obj.transform;
        let local = obj.mesh.bounds().map(|b| b.size()).unwrap_or_default();
        let size = obj.world_bounds().map(|b| b.size()).unwrap_or_default();
        let tris = obj.mesh.triangle_count();
        let r = ui.add(TextEdit::singleline(&mut name).font(egui::TextStyle::Heading));
        if r.lost_focus() && name.trim() != obj.name && !name.trim().is_empty() {
            self.run(&format!(
                "object.rename id={id} name={}",
                quote(name.trim())
            ));
        }
        ui.label(
            RichText::new(format!(
                "{:.2} × {:.2} × {:.2} mm   ·   {tris} triangles",
                size.x, size.y, size.z
            ))
            .small()
            .color(look.pal.text_weak),
        );
        ui.add_space(4.0);
        let mut changed = false;
        let mut active = false;
        let mut dims = local * t.scale.abs();
        let mut dims_changed = None;
        egui::Grid::new("transform")
            .num_columns(4)
            .spacing([6.0, 6.0])
            .show(ui, |ui| {
                let mut row = |ui: &mut Ui,
                               label: &str,
                               v: &mut glam::Vec3,
                               speed: f64,
                               suffix: &str,
                               which: u8| {
                    ui.label(RichText::new(label).color(look.pal.text_weak));
                    for (i, (axis, value)) in [("X", &mut v.x), ("Y", &mut v.y), ("Z", &mut v.z)]
                        .into_iter()
                        .enumerate()
                    {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 3.0;
                            ui.label(RichText::new(axis).small().color(AXIS_COLORS[i]));
                            let r = ui.add(
                                DragValue::new(value)
                                    .speed(speed)
                                    .max_decimals(2)
                                    .suffix(suffix),
                            );
                            if r.changed() {
                                changed = true;
                                if which == 3 {
                                    dims_changed = Some(i);
                                }
                            }
                            active |= r.dragged() || r.has_focus();
                        });
                    }
                    ui.end_row();
                };
                row(ui, "Position", &mut t.translation, 0.5, " mm", 0);
                row(ui, "Rotation", &mut t.rotation_deg, 1.0, "°", 1);
                row(ui, "Scale", &mut t.scale, 0.01, "", 2);
                row(ui, "Size", &mut dims, 0.5, " mm", 3);
            });
        if let Some(i) = dims_changed
            && local[i] > 1e-6
        {
            let sign = if t.scale[i] < 0.0 { -1.0 } else { 1.0 };
            t.scale[i] = sign * (dims[i].max(0.01) / local[i]);
        }
        let mut uniform = ui.memory(|m| {
            m.data
                .get_temp::<bool>(Id::unique("uniform-size"))
                .unwrap_or(true)
        });
        if ui.checkbox(&mut uniform, "Keep proportions").changed() {
            ui.memory_mut(|m| m.data.insert_temp(Id::unique("uniform-size"), uniform));
        }
        if uniform && let Some(i) = dims_changed {
            let f = t.scale[i].abs() / obj.transform.scale[i].abs().max(1e-6);
            for k in 0..3 {
                if k != i {
                    t.scale[k] = obj.transform.scale[k] * f;
                }
            }
        }
        if changed {
            if self.editing.map(|(e, _)| e) != Some(id) {
                let original = self
                    .doc()
                    .object(id)
                    .map(|o| o.transform)
                    .unwrap_or_default();
                self.editing = Some((id, original));
            }
            for k in 0..3 {
                if t.scale[k].abs() < 0.001 {
                    t.scale[k] = 0.001f32.copysign(t.scale[k]);
                }
            }
            self.preview_transform(id, t);
        }
        if !active && let Some((eid, original)) = self.editing.take() {
            self.commit_transform(eid, original);
        }
        widgets::section(ui, look, "Objects");
        self.scene_list(ui, look);
    }

    // --------------------------------------------------------------------------- bottom

    pub fn status_line(&self, ui: &mut Ui, look: &Look) {
        let doc = self.doc();
        let lib = library();
        let e = doc.settings.print.exposure;
        let printer = doc
            .printer
            .as_deref()
            .and_then(|p| lib.printer(p))
            .map(|p| p.model.clone())
            .unwrap_or("no printer".into());
        let resin = doc
            .resin
            .as_deref()
            .and_then(|r| lib.resin(r))
            .map(|r| r.name.clone())
            .unwrap_or("no resin".into());
        let file = self
            .session()
            .path()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "not saved".into());
        let dirty = if self.session().is_dirty() {
            " •"
        } else {
            ""
        };
        let layers = doc
            .bounds()
            .map(|b| (b.max.z.max(0.0) / e.layer_height_mm).ceil() as u32)
            .unwrap_or(0);
        ui.label(
            RichText::new(format!("Copal {}", copal_core::VERSION))
                .small()
                .color(look.pal.accent_text),
        );
        ui.label(
            RichText::new(format!(
                "·  {printer}  ·  {resin}  ·  {} mm, {} s  ·  {layers} layers ≈ {}  ·  {file}{dirty}",
                e.layer_height_mm,
                e.exposure_s,
                format_duration(doc.settings.print.estimate_time_s(layers) as f64)
            ))
            .small()
            .color(look.pal.text_faint),
        );
    }

    /// The console: a terminal-like log with the input line right after the last output line.
    pub fn console_panel(&mut self, ui: &mut Ui, look: &Look) {
        let p = look.pal;
        let font = FontId::monospace(13.0);
        ui.set_clip_rect(ui.max_rect());
        ui.spacing_mut().item_spacing.y = 2.0;
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
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    prompt_labels(ui, look, &place, &font);
                    ui.add(
                        TextEdit::singleline(input)
                            .id(id)
                            .font(font.clone())
                            .text_color(p.term_fg)
                            .frame(egui::Frame::NONE)
                            .margin(egui::Margin::ZERO)
                            .lock_focus(true)
                            .hint_text(RichText::new("help").font(font.clone()).color(p.text_faint))
                            .desired_width(f32::INFINITY),
                    )
                })
                .inner
            })
            .inner;
        if ui.rect_contains_pointer(ui.max_rect()) && ui.input(|i| i.pointer.primary_clicked()) {
            r.request_focus();
        }
        if std::mem::take(&mut self.focus_console) {
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
        let word_start = input.rfind(' ').map(|i| i + 1).unwrap_or(0);
        let mut options = self.shell.complete(&input);
        if !input.contains(' ') {
            options.extend(
                APP_COMMANDS
                    .iter()
                    .filter(|c| c.starts_with(&input))
                    .map(|c| c.to_string()),
            );
            options.sort();
        } else {
            options.extend(crate::workspace::app_arguments(&input));
        }
        match options.len() {
            0 => {}
            1 => {
                let space = if options[0].ends_with('=') { "" } else { " " };
                self.console_input = format!("{}{}{space}", &input[..word_start], options[0]);
            }
            _ => {
                let common = options.iter().skip(1).fold(options[0].clone(), |acc, o| {
                    acc.chars()
                        .zip(o.chars())
                        .take_while(|(a, b)| a == b)
                        .map(|(a, _)| a)
                        .collect()
                });
                self.console_input = format!("{}{common}", &input[..word_start]);
                let prompt = self.prompt_place();
                self.log(LineKind::Input { prompt }, input);
                self.log(LineKind::Info, options.join("   "));
            }
        }
    }

    // --------------------------------------------------------------------------- windows

    /// The preferences window. Changes apply at once and are saved.
    pub fn settings_window(&mut self, ctx: &egui::Context, look: &Look) {
        let mut open = self.show_settings;
        let mut s: Settings = self.settings.clone();
        egui::Window::new("Preferences")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(340.0)
            .show(ctx, |ui| {
                widgets::section(ui, look, "Theme");
                ui.radio_value(&mut s.theme, ThemeStyle::Default, "Default — calm colors, orange outlines");
                ui.radio_value(&mut s.theme, ThemeStyle::Art, "Art — amber crystal borders");
                widgets::section(ui, look, "Colors");
                ui.horizontal(|ui| {
                    ui.radio_value(&mut s.mode, ColorMode::Dark, "Dark");
                    ui.radio_value(&mut s.mode, ColorMode::Light, "Light");
                });
                widgets::section(ui, look, "3D view");
                ui.horizontal(|ui| {
                    ui.label("Gizmo size");
                    let (lo, hi) = copal_core::settings::GIZMO_SIZES;
                    ui.add(egui::Slider::new(&mut s.gizmo_size, lo..=hi).step_by(0.1));
                });
                widgets::section(ui, look, "Keyboard");
                if widgets::button(ui, look, ButtonKind::Normal, "Show shortcuts")
                    .on_hover_text("Every key Copal listens to (F1)")
                    .clicked()
                {
                    self.show_hotkeys = true;
                }
                ui.add_space(8.0);
                let where_ = Settings::path().map(|p| p.display().to_string()).unwrap_or_default();
                ui.label(RichText::new(format!("Saved to {where_}")).small().color(look.pal.text_faint));
                ui.label(
                    RichText::new("Also: `settings` in copal-cli, e.g. copal-cli settings theme=art mode=light")
                        .small()
                        .color(look.pal.text_faint),
                );
            });
        self.show_settings = open;
        self.set_settings(s);
    }

    /// What the last export will take: shown when slicing ends.
    pub fn summary_window(&mut self, ctx: &egui::Context, look: &Look) {
        if !self.show_summary {
            return;
        }
        let Some(Ok(r)) = self.last_export.clone() else {
            self.show_summary = false;
            return;
        };
        let cost = self.doc().settings.print.cost;
        let mut open = true;
        let mut close = false;
        let mut history = false;
        egui::Window::new("Ready to print")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, vec2(0.0, 0.0))
            .show(ctx, |ui| {
                let name = r
                    .path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                ui.label(RichText::new(name).strong().color(look.pal.text));
                ui.label(
                    RichText::new(
                        r.path
                            .parent()
                            .map(|p| p.display().to_string())
                            .unwrap_or_default(),
                    )
                    .small()
                    .color(look.pal.text_faint),
                );
                ui.add_space(6.0);
                let big = |t: String| RichText::new(t).size(17.0).color(look.pal.accent_text);
                egui::Grid::new("summary")
                    .num_columns(2)
                    .spacing([24.0, 8.0])
                    .show(ui, |ui| {
                        row_label(
                            ui,
                            look,
                            "Print time",
                            "Estimated from exposure, waits and lift speeds",
                        );
                        ui.label(big(format_duration(r.print_time_s as f64)));
                        ui.end_row();
                        row_label(ui, look, "Layers", "");
                        ui.label(big(format!("{} · {:.1} mm high", r.layers, r.height_mm)));
                        ui.end_row();
                        row_label(
                            ui,
                            look,
                            "Resin",
                            "Everything that prints: models, supports and base",
                        );
                        ui.label(big(format!("{:.1} ml", r.volume_ml)));
                        ui.end_row();
                        row_label(ui, look, "Weight", "Resin volume × density (Resin tab)");
                        ui.label(big(format!("{:.0} g", cost.weight_g(r.volume_ml))));
                        ui.end_row();
                        row_label(ui, look, "Price", "Resin volume × resin price (Resin tab)");
                        match cost.price(r.volume_ml) {
                            Some(p) => ui.label(big(format!(
                                "≈ {p:.2} {}",
                                widgets::currency_text(ui.ctx(), cost.currency.as_str())
                            ))),
                            None => ui.label(
                                RichText::new("type the resin price below")
                                    .color(look.pal.text_faint),
                            ),
                        };
                        ui.end_row();
                    });
                ui.add_space(4.0);
                egui::CollapsingHeader::new(RichText::new("Resin price").color(look.pal.text_weak))
                    .default_open(cost.resin_price <= 0.0)
                    .show(ui, |ui| self.resin_price_editor(ui, look));
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if widgets::button(ui, look, ButtonKind::Primary, "OK").clicked() {
                        close = true;
                    }
                    if widgets::button(ui, look, ButtonKind::Normal, "Print history")
                        .on_hover_text(
                            "Every export with a copy of the project, supports and settings",
                        )
                        .clicked()
                    {
                        history = true;
                        close = true;
                    }
                });
            });
        if history {
            self.show_history = true;
        }
        self.show_summary = open && !close;
    }

    /// The print history: every export with its settings, a picture, a rating and the project
    /// as it was sliced.
    pub fn history_window(&mut self, ctx: &egui::Context, look: &Look) {
        if !self.show_history {
            return;
        }
        let Some(h) = self.shell.history.clone() else {
            self.show_history = false;
            return;
        };
        let mut open = true;
        let list = h.list();
        let mut actions: Vec<(
            String,
            &'static str,
            Option<copal_core::prints::PrintRecord>,
        )> = Vec::new();
        egui::Window::new("Print history")
            .open(&mut open)
            .default_size([620.0, 520.0])
            .show(ctx, |ui| {
                ui.label(
                    RichText::new(format!("{} exports · {}", list.len(), h.dir.display()))
                        .small()
                        .color(look.pal.text_faint),
                );
                ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    if list.is_empty() {
                        ui.label(RichText::new("Nothing yet. Every export lands here with a copy of the project.").color(look.pal.text_faint));
                    }
                    for r in &list {
                        ui.separator();
                        ui.horizontal(|ui| {
                            let tex = self.history_textures.get(&r.id).cloned().or_else(|| {
                                let path = h.preview_path(&r.id)?;
                                let bytes = std::fs::read(path).ok()?;
                                let img = image::load_from_memory(&bytes).ok()?.to_rgba8();
                                let size = [img.width() as usize, img.height() as usize];
                                let ci = egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw());
                                let t = ctx.load_texture(format!("hist-{}", r.id), ci, egui::TextureOptions::LINEAR);
                                self.history_textures.insert(r.id.clone(), t.clone());
                                Some(t)
                            });
                            match tex {
                                Some(t) => {
                                    ui.add(egui::Image::new((t.id(), vec2(96.0, 96.0))));
                                }
                                None => {
                                    ui.allocate_exact_size(vec2(96.0, 96.0), Sense::hover());
                                }
                            }
                            ui.vertical(|ui| {
                                let name = r.output.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                                ui.label(RichText::new(&name).strong().color(look.pal.text));
                                ui.label(
                                    RichText::new(format!(
                                        "{} · {} · {}",
                                        r.date.get(..16).unwrap_or(&r.date).replace('T', " "),
                                        r.printer.as_deref().unwrap_or("-"),
                                        r.resin.as_deref().unwrap_or("no resin")
                                    ))
                                    .small()
                                    .color(look.pal.text_weak),
                                );
                                ui.label(
                                    RichText::new(format!(
                                        "{} layers × {} mm · {:.1} mm · {:.1} ml{} · ≈ {} · exposure {} s, bottom {} × {} s",
                                        r.layers,
                                        r.layer_height_mm,
                                        r.height_mm,
                                        r.volume_ml,
                                        match r.price {
                                            Some(p) => format!(" · {:.0} g · {p:.2} {}", r.weight_g, widgets::currency_text(ui.ctx(), &r.currency)),
                                            None if r.weight_g > 0.0 => format!(" · {:.0} g", r.weight_g),
                                            None => String::new(),
                                        },
                                        format_duration(r.print_time_s as f64),
                                        r.exposure_s,
                                        r.bottom_layers,
                                        r.bottom_exposure_s
                                    ))
                                    .small()
                                    .color(look.pal.text_weak),
                                );
                                ui.horizontal(|ui| {
                                    let mut rec = r.clone();
                                    let mut changed = false;
                                    for n in 1..=5u8 {
                                        let on = r.rating.is_some_and(|x| x >= n);
                                        if star(ui, look, on).on_hover_text(format!("{n} of 5")).clicked() {
                                            rec.rating = if r.rating == Some(n) { None } else { Some(n) };
                                            changed = true;
                                        }
                                    }
                                    ui.add_space(8.0);
                                    for (o, label) in [(Outcome::Good, "Printed well"), (Outcome::Failed, "Failed")] {
                                        if ui.selectable_label(r.outcome == Some(o), label).clicked() {
                                            rec.outcome = if r.outcome == Some(o) { None } else { Some(o) };
                                            changed = true;
                                        }
                                    }
                                    if changed {
                                        actions.push((r.id.clone(), "update", Some(rec)));
                                    }
                                });
                                let note = self.history_note.entry(r.id.clone()).or_insert_with(|| r.note.clone());
                                let resp = ui.add(TextEdit::singleline(note).hint_text("note: what went right or wrong").desired_width(f32::INFINITY));
                                if resp.lost_focus() && *note != r.note {
                                    let mut rec = r.clone();
                                    rec.note = note.clone();
                                    actions.push((r.id.clone(), "update", Some(rec)));
                                }
                                ui.horizontal(|ui| {
                                    if widgets::button(ui, look, ButtonKind::Normal, "Open project")
                                        .on_hover_text("The project exactly as it was sliced, as a new unsaved copy")
                                        .clicked()
                                    {
                                        actions.push((r.id.clone(), "open", None));
                                    }
                                    if widgets::button(ui, look, ButtonKind::Normal, "Show folder").clicked() {
                                        actions.push((r.id.clone(), "folder", None));
                                    }
                                    if widgets::button(ui, look, ButtonKind::Normal, "Remove").clicked() {
                                        actions.push((r.id.clone(), "remove", None));
                                    }
                                });
                            });
                        });
                    }
                });
            });
        for (id, action, rec) in actions {
            match action {
                "update" => {
                    if let Some(rec) = rec
                        && let Err(e) = h.update(&rec)
                    {
                        self.log(LineKind::Error, e.to_string());
                    }
                }
                "open" => {
                    self.run(&format!("history open {id}"));
                    self.step = Step::Layout;
                }
                "folder" => {
                    let dir = h.dir.join(&id);
                    open_folder(&dir);
                }
                "remove" => {
                    self.run(&format!("history remove {id}"));
                    self.history_textures.remove(&id);
                }
                _ => {}
            }
        }
        self.show_history = open;
    }
}

/// A clickable rating star, drawn (the UI font has no star glyph).
fn star(ui: &mut Ui, look: &Look, on: bool) -> egui::Response {
    let (rect, r) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::click());
    let c = rect.center();
    let (outer, inner) = (8.0, 3.4);
    let pts: Vec<egui::Pos2> = (0..10)
        .map(|i| {
            let a = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::PI / 5.0;
            let rad = if i % 2 == 0 { outer } else { inner };
            c + vec2(a.cos(), a.sin()) * rad
        })
        .collect();
    let color = if on || r.hovered() {
        look.pal.accent
    } else {
        look.pal.text_faint
    };
    let painter = ui.painter();
    if on {
        // Fill as a pentagon plus five tips, all convex.
        let inner_pts: Vec<egui::Pos2> = pts.iter().skip(1).step_by(2).copied().collect();
        painter.add(egui::Shape::convex_polygon(
            inner_pts,
            color,
            egui::Stroke::NONE,
        ));
        for i in 0..5 {
            let tip = pts[2 * i];
            let a = pts[(2 * i + 9) % 10];
            let b = pts[2 * i + 1];
            painter.add(egui::Shape::convex_polygon(
                vec![tip, b, a],
                color,
                egui::Stroke::NONE,
            ));
        }
    } else {
        let mut closed = pts.clone();
        closed.push(pts[0]);
        painter.add(egui::Shape::line(closed, egui::Stroke::new(1.2, color)));
    }
    r
}

/// Opens a folder in the system file manager.
pub fn open_folder(dir: &std::path::Path) {
    let cmd = if cfg!(windows) {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    let _ = std::process::Command::new(cmd).arg(dir).spawn();
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
