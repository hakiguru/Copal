//! Copal Lite — the same features in a simple, fast interface for weak computers.
//!
//! Usage: `copal-lite [project.copal | model.stl …]`

// No console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;

use copal_core::Settings;
use copal_ui_common::files::Purpose;
use copal_ui_common::look::{Edge, State};
use copal_ui_common::workspace::Step;
use copal_ui_common::{Edition, Look, Workspace};
use eframe::egui::{self, CentralPanel, Frame, Margin, Panel, Ui, ViewportCommand};

struct LiteApp {
    look: Look,
    ws: Workspace,
    last_title: String,
}

impl eframe::App for LiteApp {
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
        let title = ws.title().replace("— Copal", "— Copal Lite");
        if title != self.last_title {
            ui.ctx()
                .send_viewport_cmd(ViewportCommand::Title(title.clone()));
            self.last_title = title;
        }

        Panel::top("menu")
            .frame(
                Frame::new()
                    .fill(p.base)
                    .inner_margin(Margin::symmetric(8, 4)),
            )
            .show(ui, |ui| {
                egui::MenuBar::new().ui(ui, |ui| {
                    ui.menu_button("File", |ui| {
                        if ui.button("New").clicked() {
                            ws.run("new");
                        }
                        if ui.button("Open…   Ctrl+O").clicked() {
                            ws.open_dialog(Purpose::OpenProject);
                        }
                        if ui.button("Save   Ctrl+S").clicked() {
                            if ws.session().path().is_some() {
                                ws.run("save");
                            } else {
                                ws.open_dialog(Purpose::SaveProject);
                            }
                        }
                        if ui.button("Save as…   Ctrl+Shift+S").clicked() {
                            ws.open_dialog(Purpose::SaveProject);
                        }
                        ui.separator();
                        if ui.button("Import models…   Ctrl+I").clicked() {
                            ws.open_dialog(Purpose::ImportModel);
                        }
                        if ui.button("Slice & export…   Ctrl+E").clicked() {
                            ws.open_dialog(Purpose::Export);
                        }
                        if ui.button("Print history").clicked() {
                            ws.show_history = true;
                        }
                        ui.separator();
                        if ui.button("Quit").clicked() {
                            ui.ctx().send_viewport_cmd(ViewportCommand::Close);
                        }
                    });
                    ui.menu_button("Edit", |ui| {
                        if ui.button("Undo   Ctrl+Z").clicked() {
                            ws.undo();
                        }
                        if ui.button("Redo   Ctrl+Shift+Z").clicked() {
                            ws.redo();
                        }
                        ui.separator();
                        if ui.button("Arrange all").clicked() {
                            ws.run("object.arrange");
                        }
                        if ui.button("Preferences…   Ctrl+,").clicked() {
                            ws.show_settings = true;
                        }
                    });
                    ui.menu_button("View", |ui| {
                        let mut panels = ws.settings.panels;
                        ui.checkbox(&mut panels.tools, "Tools   Alt+1");
                        ui.checkbox(&mut panels.settings, "Settings   Alt+2");
                        ui.checkbox(&mut panels.console, "Console   Alt+3");
                        ws.set_panels(panels);
                    });
                    ui.menu_button("Help", |ui| {
                        if ui.button("Console commands").clicked() {
                            ws.run("help");
                        }
                    });
                    ui.add_space(16.0);
                    for (step, name) in Step::ALL {
                        if ui.selectable_label(ws.step == step, name).clicked() {
                            ws.step = step;
                        }
                    }
                });
            });

        let windows = copal_ui_common::layout::side_windows(ui, ws, look);

        CentralPanel::default()
            .frame(Frame::new().fill(p.void))
            .show(ui, |ui| ws.viewport(ui, look));

        let painter = ui.painter();
        for rect in windows {
            look.outline(painter, rect.shrink(1.0), Edge::Panel, State::Idle);
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
            .with_title("Copal Lite")
            .with_app_id("io.github.hakiguru.Copal.Lite")
            .with_inner_size([1180.0, 760.0])
            .with_min_inner_size([800.0, 520.0])
            .with_icon(copal_ui_common::look::icon())
            .with_drag_and_drop(true),
        // Lite: no multisampling, to stay light on old GPUs. The depth buffer is needed for 3D.
        multisampling: 0,
        depth_buffer: 24,
        ..Default::default()
    };
    eframe::run_native(
        "Copal Lite",
        options,
        Box::new(move |cc| {
            let settings = Settings::load_or_default();
            let look = Look::new(&cc.egui_ctx, Edition::Lite, &settings);
            let mut ws = Workspace::new(settings);
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
            Ok(Box::new(LiteApp {
                look,
                ws,
                last_title: String::new(),
            }))
        }),
    )
}
