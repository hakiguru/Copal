//! Copal Lite — the same features in a simple, fast interface for weak computers.
//!
//! Usage: `copal-lite [project.copal]`

// No console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use copal_core::{Session, Settings};
use copal_ui_common::look::{Edge, State};
use copal_ui_common::workspace::{LineKind, Step};
use copal_ui_common::{Edition, Look, Workspace, widgets};
use eframe::egui::{self, CentralPanel, Frame, Margin, Panel, ScrollArea, Ui, ViewportCommand};

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
                        if ui.button("Save   Ctrl+S").clicked() {
                            ws.run("save");
                        }
                        if ui.button("Settings…   Ctrl+,").clicked() {
                            ws.show_settings = true;
                        }
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
                    });
                    ui.menu_button("Help", |ui| {
                        if ui.button("Console commands").clicked() {
                            ws.run("help");
                        }
                    });
                });
            });

        Panel::bottom("status")
            .frame(
                Frame::new()
                    .fill(p.base)
                    .inner_margin(Margin::symmetric(8, 3)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| ws.status_line(ui, look));
            });

        let console_fill = if look.art() { p.panel } else { p.term_bg };
        let console = Panel::bottom("console")
            .exact_size(120.0)
            .frame(
                Frame::new()
                    .fill(console_fill)
                    .inner_margin(Margin::same(8)),
            )
            .show(ui, |ui| ws.console_panel(ui, look));

        let side = Panel::left("side")
            .exact_size(280.0)
            .frame(Frame::new().fill(p.panel).inner_margin(Margin::same(10)))
            .show(ui, |ui| {
                ScrollArea::vertical().show(ui, |ui| {
                    widgets::wordmark(ui, look, 32.0);
                    ui.add_space(4.0);
                    ws.printer_resin_pickers(ui, look);
                    widgets::section(ui, look, "Objects");
                    ws.scene_list(ui, look);
                    widgets::section(ui, look, "Transform");
                    ws.inspector(ui, look);
                    let current = ws.step;
                    for step in [Step::Prepare, Step::Supports, Step::Slice] {
                        ws.step = step;
                        ws.tool_shelf(ui, look);
                    }
                    ws.step = current;
                });
            });

        CentralPanel::default()
            .frame(Frame::new().fill(p.void))
            .show(ui, |ui| ws.viewport(ui, look));

        let painter = ui.painter();
        for rect in [side.response.rect, console.response.rect] {
            look.outline(painter, rect.shrink(1.0), Edge::Panel, State::Idle);
        }

        ws.settings_window(ui.ctx(), look);
    }
}

fn main() -> eframe::Result {
    let file = std::env::args().nth(1);
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Copal Lite")
            .with_app_id("io.github.hakiguru.Copal.Lite")
            .with_inner_size([1180.0, 760.0])
            .with_min_inner_size([800.0, 520.0])
            .with_icon(copal_ui_common::look::icon()),
        // Lite: no multisampling and no depth buffer, to stay light on old GPUs.
        multisampling: 0,
        depth_buffer: 0,
        ..Default::default()
    };
    eframe::run_native(
        "Copal Lite",
        options,
        Box::new(move |cc| {
            let settings = Settings::load_or_default();
            let look = Look::new(&cc.egui_ctx, Edition::Lite, &settings);
            let mut ws = Workspace::new(settings);
            if let Some(path) = file {
                match Session::open(path.as_ref()) {
                    Ok(s) => {
                        ws.session = s;
                        ws.log(LineKind::Info, format!("Opened {path}"));
                    }
                    Err(e) => ws.log(LineKind::Error, format!("{path}: {e}")),
                }
            }
            Ok(Box::new(LiteApp {
                look,
                ws,
                last_title: String::new(),
            }))
        }),
    )
}
