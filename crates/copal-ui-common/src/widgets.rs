//! Copal widgets: buttons, tabs and frames with crystal borders.

use egui::{
    Align2, Color32, FontId, InnerResponse, Margin, Rect, Response, Sense, Ui, Vec2, WidgetText,
    pos2, vec2,
};

use crate::crystal::{Border, paint_border, paint_sparkle};
use crate::look::Look;
use crate::palette::{amber, ink, shade};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonKind {
    /// Everyday button.
    Normal,
    /// The main action of a step (Slice, Export, Auto supports).
    Primary,
}

/// A button with a crystal border. Brighter on hover, darker when pressed.
pub fn button(ui: &mut Ui, look: &Look, kind: ButtonKind, text: impl Into<WidgetText>) -> Response {
    button_sized(ui, look, kind, text, None)
}

/// Like [`button`], with an optional fixed width.
pub fn button_sized(
    ui: &mut Ui,
    look: &Look,
    kind: ButtonKind,
    text: impl Into<WidgetText>,
    width: Option<f32>,
) -> Response {
    let border = Border::BUTTON;
    let pad = ui.spacing().button_padding + Vec2::splat(border.width());
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
        let (fill, brightness, text_color) = match (
            kind,
            response.is_pointer_button_down_on(),
            response.hovered(),
        ) {
            (_, true, _) => (shade(ink::RAISED, 0.8), 0.8, amber::LIGHTER),
            (ButtonKind::Primary, false, true) => (shade(amber::DEEP, 0.42), 1.0, amber::GLINT),
            (ButtonKind::Primary, false, false) => {
                (shade(amber::DEEP, 0.32), 0.95, amber::HIGHLIGHT)
            }
            (ButtonKind::Normal, false, true) => (ink::RAISED_HOVER, 1.0, amber::HIGHLIGHT),
            (ButtonKind::Normal, false, false) => (ink::RAISED, border.brightness, ink::TEXT),
        };
        let text_color = if ui.is_enabled() {
            text_color
        } else {
            ink::TEXT_FAINT
        };
        let painter = ui.painter();
        painter.rect_filled(rect, 0.0, fill);
        paint_border(
            painter,
            look.crystal.id(),
            rect,
            border.with_brightness(brightness),
        );
        let text_pos = rect.center() - galley.size() / 2.0;
        painter.galley(text_pos, galley, text_color);
        if look.decorations() && kind == ButtonKind::Primary && response.hovered() {
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
        let painter = ui.painter();
        let hovered = response.hovered();
        if selected {
            painter.rect_filled(rect, 0.0, ink::RAISED);
            paint_border(
                painter,
                look.crystal.id(),
                rect,
                Border::BUTTON.with_brightness(0.95),
            );
            if look.decorations() {
                paint_sparkle(painter, rect.right_top() + vec2(-6.0, 6.0), 1.5, 1);
            }
        } else if hovered {
            painter.rect_filled(rect, 0.0, ink::BASE);
        }
        let color = match (selected, hovered) {
            (true, _) => amber::HIGHLIGHT,
            (false, true) => ink::TEXT,
            (false, false) => ink::TEXT_WEAK,
        };
        let num_color = if selected {
            amber::LIGHT
        } else {
            ink::TEXT_FAINT
        };
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

/// A framed group with a crystal border.
pub fn frame<R>(
    ui: &mut Ui,
    look: &Look,
    border: Border,
    fill: Color32,
    add: impl FnOnce(&mut Ui) -> R,
) -> InnerResponse<R> {
    let margin = Margin::same((border.width() + 8.0).round() as i8);
    let inner = egui::Frame::new()
        .fill(fill)
        .inner_margin(margin)
        .show(ui, add);
    paint_border(ui.painter(), look.crystal.id(), inner.response.rect, border);
    inner
}

/// A small section title: an amber tick and quiet uppercase text.
pub fn section(ui: &mut Ui, title: &str) {
    ui.add_space(6.0);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 18.0), Sense::hover());
    let tick = Rect::from_min_size(pos2(rect.left(), rect.center().y - 3.0), vec2(6.0, 6.0));
    ui.painter().rect_filled(tick, 0.0, amber::MID);
    ui.painter().text(
        pos2(rect.left() + 12.0, rect.center().y),
        Align2::LEFT_CENTER,
        title.to_uppercase(),
        FontId::proportional(11.5),
        ink::TEXT_WEAK,
    );
}

/// The wordmark image at a whole-number scale, so pixels stay sharp.
pub fn wordmark(ui: &mut Ui, look: &Look, height: f32) -> Response {
    let [w, h] = look.wordmark.size();
    let scale = (height / h as f32).round().max(1.0);
    let size = vec2(w as f32 * scale, h as f32 * scale);
    ui.add(egui::Image::new((look.wordmark.id(), size)))
}
