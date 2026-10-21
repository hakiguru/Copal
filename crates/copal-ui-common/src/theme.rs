//! The Copal look: warm dark neutrals, amber only as an accent.

use egui::{Color32, Context, CornerRadius, Margin, Shadow, Stroke, Style, Theme, Visuals, vec2};

use crate::palette::{amber, ink, shade};

/// Which app is using the theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edition {
    /// Full app: glints, a little more air.
    Full,
    /// Lite: same colors, no decoration that costs anything.
    Lite,
}

pub fn apply(ctx: &Context, edition: Edition) {
    ctx.set_theme(Theme::Dark);
    ctx.style_mut_of(Theme::Dark, |style| configure(style, edition));
}

fn configure(style: &mut Style, edition: Edition) {
    let v: &mut Visuals = &mut style.visuals;
    *v = Visuals::dark();
    v.override_text_color = None;
    v.panel_fill = ink::PANEL;
    v.window_fill = ink::PANEL;
    v.window_stroke = Stroke::new(1.0, amber::OUTLINE);
    v.window_corner_radius = CornerRadius::ZERO;
    v.menu_corner_radius = CornerRadius::ZERO;
    v.window_shadow = Shadow {
        offset: [0, 6],
        blur: 18,
        spread: 0,
        color: Color32::from_black_alpha(120),
    };
    v.popup_shadow = Shadow {
        offset: [0, 4],
        blur: 10,
        spread: 0,
        color: Color32::from_black_alpha(100),
    };
    v.extreme_bg_color = ink::VOID;
    v.faint_bg_color = ink::BASE;
    v.code_bg_color = ink::BASE;
    v.text_edit_bg_color = Some(ink::BASE);
    v.hyperlink_color = amber::LIGHTER;
    v.warn_fg_color = amber::LIGHT;
    v.error_fg_color = ink::ERROR;
    v.selection.bg_fill = shade(amber::DEEP, 0.75);
    v.selection.stroke = Stroke::new(1.0, amber::HIGHLIGHT);
    v.window_highlight_topmost = false;

    let w = &mut v.widgets;
    let square = CornerRadius::ZERO;
    w.noninteractive.bg_fill = ink::PANEL;
    w.noninteractive.weak_bg_fill = ink::PANEL;
    w.noninteractive.bg_stroke = Stroke::new(1.0, ink::LINE);
    w.noninteractive.fg_stroke = Stroke::new(1.0, ink::TEXT);
    w.noninteractive.corner_radius = square;

    w.inactive.bg_fill = ink::RAISED;
    w.inactive.weak_bg_fill = ink::RAISED;
    w.inactive.bg_stroke = Stroke::new(1.0, shade(amber::DEEP, 0.7));
    w.inactive.fg_stroke = Stroke::new(1.0, ink::TEXT);
    w.inactive.corner_radius = square;

    w.hovered.bg_fill = ink::RAISED_HOVER;
    w.hovered.weak_bg_fill = ink::RAISED_HOVER;
    w.hovered.bg_stroke = Stroke::new(1.0, amber::MID);
    w.hovered.fg_stroke = Stroke::new(1.5, amber::HIGHLIGHT);
    w.hovered.corner_radius = square;
    w.hovered.expansion = 0.0;

    w.active.bg_fill = shade(ink::RAISED, 0.8);
    w.active.weak_bg_fill = shade(ink::RAISED, 0.8);
    w.active.bg_stroke = Stroke::new(1.0, amber::LIGHT);
    w.active.fg_stroke = Stroke::new(1.5, amber::GLINT);
    w.active.corner_radius = square;
    w.active.expansion = 0.0;

    w.open = w.active;

    let s = &mut style.spacing;
    s.item_spacing = vec2(8.0, 6.0);
    s.button_padding = vec2(10.0, 5.0);
    s.window_margin = Margin::same(12);
    s.menu_margin = Margin::same(6);
    s.interact_size.y = 24.0;
    if edition == Edition::Lite {
        s.item_spacing = vec2(6.0, 4.0);
        s.button_padding = vec2(8.0, 3.0);
        v.window_shadow = Shadow::NONE;
        v.popup_shadow = Shadow::NONE;
        style.animation_time = 0.0;
    }
}
