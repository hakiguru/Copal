//! Turns a [`Palette`] into egui styles.

use copal_core::{Settings, ThemeStyle};
use egui::{Color32, Context, CornerRadius, Margin, Shadow, Stroke, Style, Theme, Visuals, vec2};

use crate::palette::{Palette, shade};

/// Which app is using the theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edition {
    /// Full app: decorations allowed, a little more air.
    Full,
    /// Lite: same colors, no decoration that costs anything.
    Lite,
}

/// Corner radius of buttons and fields in the default theme. The art theme is square.
pub const RADIUS: u8 = 4;

pub fn apply(ctx: &Context, edition: Edition, settings: &Settings, pal: &Palette) {
    let theme = if pal.dark { Theme::Dark } else { Theme::Light };
    ctx.set_theme(theme);
    let art = settings.theme == ThemeStyle::Art;
    ctx.style_mut_of(theme, |style| configure(style, edition, art, pal));
}

fn configure(style: &mut Style, edition: Edition, art: bool, pal: &Palette) {
    let radius = CornerRadius::same(if art { 0 } else { RADIUS });
    let v: &mut Visuals = &mut style.visuals;
    *v = if pal.dark {
        Visuals::dark()
    } else {
        Visuals::light()
    };
    v.override_text_color = None;
    v.panel_fill = pal.panel;
    v.window_fill = pal.panel;
    v.window_stroke = Stroke::new(1.0, pal.accent);
    v.window_corner_radius = CornerRadius::same(if art { 0 } else { 6 });
    v.menu_corner_radius = radius;
    let shadow_alpha = if pal.dark { 120 } else { 40 };
    v.window_shadow = Shadow {
        offset: [0, 6],
        blur: 18,
        spread: 0,
        color: Color32::from_black_alpha(shadow_alpha),
    };
    v.popup_shadow = Shadow {
        offset: [0, 4],
        blur: 10,
        spread: 0,
        color: Color32::from_black_alpha(shadow_alpha - 20),
    };
    v.extreme_bg_color = pal.void;
    v.faint_bg_color = pal.base;
    v.code_bg_color = pal.base;
    v.text_edit_bg_color = Some(pal.base);
    v.hyperlink_color = pal.accent_text;
    v.warn_fg_color = pal.accent_text;
    v.error_fg_color = pal.error;
    v.selection.bg_fill = pal.selection;
    v.selection.stroke = Stroke::new(1.0, pal.accent_text);
    v.window_highlight_topmost = false;

    let w = &mut v.widgets;
    w.noninteractive.bg_fill = pal.panel;
    w.noninteractive.weak_bg_fill = pal.panel;
    w.noninteractive.bg_stroke = Stroke::new(1.0, pal.line);
    w.noninteractive.fg_stroke = Stroke::new(1.0, pal.text);
    w.noninteractive.corner_radius = radius;

    // Fields and combo boxes get the same outline as buttons.
    let idle_outline = if art {
        shade(pal.accent, 0.55)
    } else {
        pal.accent
    };
    w.inactive.bg_fill = pal.raised;
    w.inactive.weak_bg_fill = pal.raised;
    w.inactive.bg_stroke = Stroke::new(1.0, idle_outline);
    w.inactive.fg_stroke = Stroke::new(1.0, pal.text);
    w.inactive.corner_radius = radius;

    w.hovered.bg_fill = pal.raised_hover;
    w.hovered.weak_bg_fill = pal.raised_hover;
    w.hovered.bg_stroke = Stroke::new(1.0, pal.accent_hover);
    w.hovered.fg_stroke = Stroke::new(1.5, pal.text);
    w.hovered.corner_radius = radius;
    w.hovered.expansion = 0.0;

    w.active.bg_fill = pal.pressed;
    w.active.weak_bg_fill = pal.pressed;
    w.active.bg_stroke = Stroke::new(1.0, pal.accent_hover);
    w.active.fg_stroke = Stroke::new(1.5, pal.text);
    w.active.corner_radius = radius;
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
