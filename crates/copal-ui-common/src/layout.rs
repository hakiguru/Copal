//! The side windows both apps share: tools (left), settings (right), console (bottom) and the
//! status bar. Each window folds away to a thin stripe and comes back with a click or Alt+1/2/3,
//! like tool windows in IntelliJ.

use egui::{Frame, Margin, Panel, Rect, RichText, ScrollArea, Ui};

use crate::look::Look;
use crate::panels::{stripe_button, window_header};
use crate::theme::Edition;
use crate::widgets;
use crate::workspace::Workspace;

/// Draws status bar, console, tools and settings. Returns the rectangles of the open windows
/// (for outlines). Call before the central panel.
pub fn side_windows(ui: &mut Ui, ws: &mut Workspace, look: &Look) -> Vec<Rect> {
    let p = look.pal;
    let lite = look.edition == Edition::Lite;
    let margin = if lite { 10 } else { 12 };
    let frame = |fill| Frame::new().fill(fill).inner_margin(Margin::same(margin));
    let mut panels = ws.settings.panels;
    let mut rects = Vec::new();

    Panel::bottom("status")
        .frame(
            Frame::new()
                .fill(p.base)
                .inner_margin(Margin::symmetric(8, 3)),
        )
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let color = if panels.console {
                    p.accent
                } else {
                    p.text_weak
                };
                if ui
                    .add(
                        egui::Button::new(RichText::new("Console").small().color(color))
                            .frame(false),
                    )
                    .on_hover_text("Show or hide the console (Alt+3)")
                    .clicked()
                {
                    panels.console = !panels.console;
                }
                ui.separator();
                ws.status_line(ui, look);
            });
        });

    if panels.console {
        let fill = if look.art() { p.panel } else { p.term_bg };
        let r = Panel::bottom("console")
            .resizable(true)
            .default_size(if lite { 120.0 } else { 160.0 })
            .min_size(48.0)
            .max_size(700.0)
            .frame(frame(fill))
            .show(ui, |ui| {
                if window_header(ui, look, "Console", "Alt+3") {
                    panels.console = false;
                }
                ws.console_panel(ui, look);
            });
        rects.push(r.response.rect);
    }

    if panels.tools {
        let r = Panel::left("tools")
            .resizable(true)
            .default_size(if lite { 260.0 } else { 250.0 })
            .min_size(190.0)
            .max_size(520.0)
            .frame(frame(p.panel))
            .show(ui, |ui| {
                if window_header(ui, look, "Tools", "Alt+1") {
                    panels.tools = false;
                }
                ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if lite {
                            widgets::wordmark(ui, look, 28.0);
                            ui.add_space(4.0);
                            ui.horizontal_wrapped(|ui| ws.printer_resin_pickers(ui, look));
                        }
                        ws.tool_shelf(ui, look);
                    });
            });
        rects.push(r.response.rect);
    } else {
        Panel::left("tools-stripe")
            .resizable(false)
            .exact_size(26.0)
            .frame(
                Frame::new()
                    .fill(p.base)
                    .inner_margin(Margin::symmetric(2, 8)),
            )
            .show(ui, |ui| {
                if stripe_button(ui, look, "Tools", true)
                    .on_hover_text("Alt+1")
                    .clicked()
                {
                    panels.tools = true;
                }
            });
    }

    if panels.settings {
        let r = Panel::right("settings")
            .resizable(true)
            .default_size(if lite { 300.0 } else { 330.0 })
            .min_size(220.0)
            .max_size(700.0)
            .frame(frame(p.panel))
            .show(ui, |ui| {
                if window_header(ui, look, "Settings", "Alt+2") {
                    panels.settings = false;
                }
                ws.settings_panel(ui, look);
            });
        rects.push(r.response.rect);
    } else {
        Panel::right("settings-stripe")
            .resizable(false)
            .exact_size(26.0)
            .frame(
                Frame::new()
                    .fill(p.base)
                    .inner_margin(Margin::symmetric(2, 8)),
            )
            .show(ui, |ui| {
                if stripe_button(ui, look, "Settings", true)
                    .on_hover_text("Alt+2")
                    .clicked()
                {
                    panels.settings = true;
                }
            });
    }
    ws.set_panels(panels);
    rects
}
