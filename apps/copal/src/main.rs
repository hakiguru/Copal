//! Copal — the full app.
//!
//! Usage: `copal [project.copal | model.stl …]`

// No console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;

use copal_core::Settings;
use copal_ui_common::crystal::paint_sparkle;
use copal_ui_common::files::Purpose;
use copal_ui_common::look::{Edge, State};
use copal_ui_common::{Edition, Look, Workspace, widgets};
use eframe::egui::{self, CentralPanel, Frame, Margin, Panel, Ui, ViewportCommand, vec2};

struct CopalApp {
    look: Look,
    ws: Workspace,
    last_title: String,
}

impl CopalApp {
    fn new(cc: &eframe::CreationContext<'_>, files: Vec<PathBuf>) -> Self {
        let settings = Settings::load_or_default();
        let look = Look::new(&cc.egui_ctx, Edition::Full, &settings);
        let mut ws = Workspace::new(settings);
        open_args(&mut ws, files);
        Self {
            look,
            ws,
            last_title: String::new(),
        }
    }
}

/// Opens a project or imports models given on the command line.
pub fn open_args(ws: &mut Workspace, files: Vec<PathBuf>) {
    let (projects, models): (Vec<_>, Vec<_>) = files.into_iter().partition(|p| {
        p.extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("copal"))
    });
    if let Some(p) = projects.first() {
        ws.open(p);
    }
    if !models.is_empty() {
        ws.import(&models);
    }
}

impl eframe::App for CopalApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        if self.ws.take_settings_changed() {
            self.look.apply(ui.ctx(), &self.ws.settings);
        }
        let look = &self.look;
        let p = look.pal;
        let ws = &mut self.ws;
        ws.poll_export(ui.ctx());
        ws.poll_prep_job(ui.ctx());
        ws.poll_presets(ui.ctx());
        ws.dropped_files(ui);
        ws.hotkeys(ui);
        let title = ws.title();
        if title != self.last_title {
            ui.ctx()
                .send_viewport_cmd(ViewportCommand::Title(title.clone()));
            self.last_title = title;
        }

        // Top bar: wordmark, steps, printer and resin, file buttons.
        let top = Panel::top("top")
            .frame(
                Frame::new()
                    .fill(p.base)
                    .inner_margin(Margin::symmetric(14, 8)),
            )
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    widgets::wordmark(ui, look, 28.0);
                    ui.add_space(14.0);
                    ws.step_tabs(ui, look);
                    ui.add_space(14.0);
                    ws.printer_resin_pickers(ui, look);
                    ui.add_space(8.0);
                    let b = |ui: &mut Ui, t: &str| {
                        widgets::button(ui, look, widgets::ButtonKind::Normal, t)
                    };
                    if b(ui, "Open")
                        .on_hover_text("Open a project (Ctrl+O)")
                        .clicked()
                    {
                        ws.open_dialog(Purpose::OpenProject);
                    }
                    if b(ui, "Save")
                        .on_hover_text("Save the project (Ctrl+S, Ctrl+Shift+S: save as)")
                        .clicked()
                    {
                        if ws.session().path().is_some() {
                            ws.run("save");
                        } else {
                            ws.open_dialog(Purpose::SaveProject);
                        }
                    }
                    if b(ui, "History")
                        .on_hover_text("Every exported file, with ratings and project snapshots")
                        .clicked()
                    {
                        ws.show_history = !ws.show_history;
                    }
                    if b(ui, "Preferences")
                        .on_hover_text("Theme, colors, gizmo size (Ctrl+,)")
                        .clicked()
                    {
                        ws.show_settings = !ws.show_settings;
                    }
                });
            });
        let top_rect = top.response.rect;
        let line = if look.art() {
            copal_ui_common::palette::amber::OUTLINE
        } else {
            p.line
        };
        ui.painter().rect_filled(
            egui::Rect::from_min_max(
                top_rect.left_bottom() - vec2(0.0, 1.0),
                top_rect.right_bottom(),
            ),
            0.0,
            line,
        );

        let windows = copal_ui_common::layout::side_windows(ui, ws, look);

        let center = CentralPanel::default()
            .frame(Frame::new().fill(p.void).inner_margin(Margin::same(4)))
            .show(ui, |ui| ws.viewport(ui, look));

        // Outlines around the main areas.
        let painter = ui.painter();
        for rect in windows {
            look.outline(painter, rect.shrink(2.0), Edge::Panel, State::Idle);
        }
        let stage = center.response.rect.shrink(2.0);
        look.outline(painter, stage, Edge::Stage, State::Idle);
        if look.decorations() {
            paint_sparkle(painter, stage.left_top() + vec2(3.0, 3.0), 2.0, 2);
        }

        ws.dialogs(ui.ctx(), look);
        ws.history_window(ui.ctx(), look);
        ws.settings_window(ui.ctx(), look);
    }
}

fn main() -> eframe::Result {
    let files: Vec<PathBuf> = std::env::args().skip(1).map(PathBuf::from).collect();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Copal")
            .with_app_id("io.github.hakiguru.Copal")
            .with_inner_size([1440.0, 880.0])
            .with_min_inner_size([960.0, 600.0])
            .with_icon(copal_ui_common::look::icon())
            .with_drag_and_drop(true),
        depth_buffer: 24,
        // No multisampling: some drivers have no such config and the app would not start.
        multisampling: 0,
        ..Default::default()
    };
    eframe::run_native(
        "Copal",
        options,
        Box::new(move |cc| Ok(Box::new(CopalApp::new(cc, files)))),
    )
}
