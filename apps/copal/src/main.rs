//! Copal — the full app.
//!
//! Usage: `copal [project.copal]`

// No console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use copal_core::{Session, Settings};
use copal_ui_common::crystal::paint_sparkle;
use copal_ui_common::look::{Edge, State};
use copal_ui_common::workspace::{LineKind, Step};
use copal_ui_common::{Edition, Look, Workspace, widgets};
use eframe::egui::{self, CentralPanel, Frame, Margin, Panel, RichText, Ui, ViewportCommand, vec2};

struct CopalApp {
    look: Look,
    ws: Workspace,
    last_title: String,
}

impl CopalApp {
    fn new(cc: &eframe::CreationContext<'_>, file: Option<String>) -> Self {
        let settings = Settings::load_or_default();
        let look = Look::new(&cc.egui_ctx, Edition::Full, &settings);
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
        Self {
            look,
            ws,
            last_title: String::new(),
        }
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
        ws.hotkeys(ui);
        let title = ws.title();
        if title != self.last_title {
            ui.ctx()
                .send_viewport_cmd(ViewportCommand::Title(title.clone()));
            self.last_title = title;
        }
        let panel_frame = |fill| Frame::new().fill(fill).inner_margin(Margin::same(12));

        // Top bar: wordmark, printer, resin, settings.
        let top = Panel::top("top")
            .frame(
                Frame::new()
                    .fill(p.base)
                    .inner_margin(Margin::symmetric(14, 8)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    widgets::wordmark(ui, look, 28.0);
                    ui.add_space(18.0);
                    ws.printer_resin_pickers(ui, look);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let r = widgets::button(ui, look, widgets::ButtonKind::Normal, "Settings");
                        if r.on_hover_text("Theme and colors (Ctrl+,)").clicked() {
                            ws.show_settings = !ws.show_settings;
                        }
                    });
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

        Panel::bottom("status")
            .frame(
                Frame::new()
                    .fill(p.base)
                    .inner_margin(Margin::symmetric(12, 4)),
            )
            .show(ui, |ui| ui.horizontal(|ui| ws.status_line(ui, look)));

        let console_fill = if look.art() { p.panel } else { p.term_bg };
        let console = Panel::bottom("console")
            .exact_size(160.0)
            .frame(panel_frame(console_fill))
            .show(ui, |ui| ws.console_panel(ui, look));

        let left = Panel::left("workflow")
            .exact_size(236.0)
            .frame(panel_frame(p.panel))
            .show(ui, |ui| {
                for (i, (step, name)) in [
                    (Step::Prepare, "Prepare"),
                    (Step::Supports, "Supports"),
                    (Step::Slice, "Slice & export"),
                ]
                .into_iter()
                .enumerate()
                {
                    if widgets::tab(ui, look, ws.step == step, i + 1, name).clicked() {
                        ws.step = step;
                    }
                    ui.add_space(4.0);
                }
                ui.add_space(8.0);
                ws.tool_shelf(ui, look);
            });

        let right = Panel::right("inspector")
            .default_size(300.0)
            .resizable(true)
            .frame(panel_frame(p.panel))
            .show(ui, |ui| {
                widgets::section(ui, look, "Object");
                ws.inspector(ui, look);
                ui.add_space(10.0);
                widgets::section(ui, look, "Scene");
                ws.scene_list(ui, look);
            });

        let center = CentralPanel::default()
            .frame(Frame::new().fill(p.void).inner_margin(Margin::same(6)))
            .show(ui, |ui| ws.viewport(ui, look));

        // Outlines around the main areas.
        let painter = ui.painter();
        for rect in [
            left.response.rect,
            right.response.rect,
            console.response.rect,
        ] {
            look.outline(painter, rect.shrink(2.0), Edge::Panel, State::Idle);
        }
        let stage = center.response.rect.shrink(2.0);
        look.outline(painter, stage, Edge::Stage, State::Idle);
        if look.decorations() {
            paint_sparkle(painter, stage.left_top() + vec2(3.0, 3.0), 2.0, 2);
        }

        if ws.session.doc.objects().is_empty() {
            let msg = RichText::new(
                "Add a test cube from the Prepare step, or type  object.add_cube size=20  in the console",
            )
            .color(p.text_faint);
            ui.put(
                egui::Rect::from_center_size(
                    stage.center_bottom() - vec2(0.0, 30.0),
                    vec2(stage.width(), 20.0),
                ),
                egui::Label::new(msg),
            );
        }

        ws.settings_window(ui.ctx(), look);
    }
}

fn main() -> eframe::Result {
    let file = std::env::args().nth(1);
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Copal")
            .with_app_id("io.github.hakiguru.Copal")
            .with_inner_size([1440.0, 880.0])
            .with_min_inner_size([960.0, 600.0])
            .with_icon(copal_ui_common::look::icon()),
        ..Default::default()
    };
    eframe::run_native(
        "Copal",
        options,
        Box::new(move |cc| Ok(Box::new(CopalApp::new(cc, file)))),
    )
}
