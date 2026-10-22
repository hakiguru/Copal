//! Copal widgets: buttons, tabs, section titles. They follow the current theme.

use egui::{Align2, FontId, Rect, Response, Sense, Ui, Vec2, WidgetText, pos2, vec2};

use crate::crystal::paint_sparkle;
use crate::look::{Edge, Look, State};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonKind {
    /// Everyday button.
    Normal,
    /// The main action of a step (Slice, Export, Auto supports).
    Primary,
}

fn state_of(response: &Response) -> State {
    if response.is_pointer_button_down_on() {
        State::Pressed
    } else if response.hovered() {
        State::Hovered
    } else {
        State::Idle
    }
}

/// A button with the theme's outline. Brighter on hover, darker when pressed.
pub fn button(ui: &mut Ui, look: &Look, kind: ButtonKind, text: impl Into<WidgetText>) -> Response {
    button_sized(ui, look, kind, text, None)
}

/// Like [`button`], with an optional minimum width.
pub fn button_sized(
    ui: &mut Ui,
    look: &Look,
    kind: ButtonKind,
    text: impl Into<WidgetText>,
    width: Option<f32>,
) -> Response {
    let pad = ui.spacing().button_padding + Vec2::splat(if look.art() { 2.0 } else { 1.0 });
    let galley = text.into().into_galley(
        ui,
        Some(egui::TextWrapMode::Extend),
        f32::INFINITY,
        egui::TextStyle::Button,
    );
    let mut size = galley.size() + 2.0 * pad;
    size.y = size.y.max(ui.spacing().interact_size.y + 2.0);
    if let Some(w) = width {
        size.x = size.x.max(w);
    }
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    if ui.is_rect_visible(rect) {
        let p = &look.pal;
        let state = state_of(&response);
        let fill = match (kind, state) {
            (_, State::Pressed) => p.pressed,
            (ButtonKind::Primary, State::Hovered) => p.primary_fill_hover,
            (ButtonKind::Primary, _) => p.primary_fill,
            (ButtonKind::Normal, State::Hovered) => p.raised_hover,
            (ButtonKind::Normal, _) => p.raised,
        };
        let text_color = if !ui.is_enabled() {
            p.text_faint
        } else if kind == ButtonKind::Primary {
            p.accent_text
        } else {
            p.text
        };
        let painter = ui.painter();
        painter.rect_filled(rect, look.radius(Edge::Button), fill);
        look.outline(painter, rect, Edge::Button, state);
        painter.galley(rect.center() - galley.size() / 2.0, galley, text_color);
        if look.decorations() && kind == ButtonKind::Primary && state == State::Hovered {
            paint_sparkle(painter, rect.left_top() + vec2(2.0, 2.0), 1.5, 1);
        }
    }
    response
}

/// A big workflow tab (Prepare / Supports / Slice & Export).
pub fn tab(ui: &mut Ui, look: &Look, selected: bool, number: usize, text: &str) -> Response {
    let size = vec2(ui.available_width(), 40.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    if ui.is_rect_visible(rect) {
        let p = &look.pal;
        let painter = ui.painter();
        let hovered = response.hovered();
        if selected {
            painter.rect_filled(rect, look.radius(Edge::Button), p.raised);
            look.outline(painter, rect, Edge::Button, State::Selected);
            if look.decorations() {
                paint_sparkle(painter, rect.right_top() + vec2(-6.0, 6.0), 1.5, 1);
            }
        } else if hovered {
            painter.rect_filled(rect, look.radius(Edge::Button), p.raised_hover);
        }
        let color = match (selected, hovered) {
            (true, _) => p.accent_text,
            (false, true) => p.text,
            (false, false) => p.text_weak,
        };
        let num_color = if selected { p.accent } else { p.text_faint };
        painter.text(
            pos2(rect.left() + 14.0, rect.center().y),
            Align2::LEFT_CENTER,
            format!("{number}"),
            FontId::monospace(13.0),
            num_color,
        );
        painter.text(
            pos2(rect.left() + 32.0, rect.center().y),
            Align2::LEFT_CENTER,
            text.to_uppercase(),
            FontId::proportional(13.5),
            color,
        );
    }
    response
}

/// A small section title: an accent tick and quiet uppercase text.
pub fn section(ui: &mut Ui, look: &Look, title: &str) {
    ui.add_space(6.0);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 18.0), Sense::hover());
    let tick = Rect::from_min_size(pos2(rect.left(), rect.center().y - 3.0), vec2(6.0, 6.0));
    let radius = if look.art() { 0.0 } else { 3.0 };
    ui.painter().rect_filled(tick, radius, look.pal.accent);
    ui.painter().text(
        pos2(rect.left() + 12.0, rect.center().y),
        Align2::LEFT_CENTER,
        title.to_uppercase(),
        FontId::proportional(11.5),
        look.pal.text_weak,
    );
}

/// The wordmark image at a whole-number scale, so pixels stay sharp.
pub fn wordmark(ui: &mut Ui, look: &Look, height: f32) -> Response {
    let [w, h] = look.wordmark.size();
    let scale = (height / h as f32).round().max(1.0);
    let size = vec2(w as f32 * scale, h as f32 * scale);
    ui.add(egui::Image::new((look.wordmark.id(), size)))
}
