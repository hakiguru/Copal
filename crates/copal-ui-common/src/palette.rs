//! Colors. The amber tones are the same as in the logo (`assets/make_logo.py`).

use egui::Color32;

/// Amber tones from dark to bright, exactly as in the logo.
pub mod amber {
    use egui::Color32;
    pub const OUTLINE: Color32 = Color32::from_rgb(112, 32, 4);
    pub const DEEP: Color32 = Color32::from_rgb(176, 58, 6);
    pub const SHADOW: Color32 = Color32::from_rgb(214, 88, 10);
    pub const MID: Color32 = Color32::from_rgb(240, 122, 18);
    pub const LIGHT: Color32 = Color32::from_rgb(252, 158, 38);
    pub const LIGHTER: Color32 = Color32::from_rgb(255, 190, 72);
    pub const HIGHLIGHT: Color32 = Color32::from_rgb(255, 222, 128);
    pub const GLINT: Color32 = Color32::from_rgb(255, 251, 228);

    /// Tones used by the crystal texture, dark → bright.
    pub const RAMP: [Color32; 7] = [DEEP, SHADOW, MID, LIGHT, LIGHTER, HIGHLIGHT, GLINT];
}

/// Warm, quiet neutrals. Most of the interface uses these; amber is only an accent.
pub mod ink {
    use egui::Color32;
    /// Behind everything (viewport background).
    pub const VOID: Color32 = Color32::from_rgb(16, 13, 11);
    /// Main background.
    pub const BASE: Color32 = Color32::from_rgb(24, 20, 17);
    /// Panels.
    pub const PANEL: Color32 = Color32::from_rgb(31, 26, 22);
    /// Buttons and fields.
    pub const RAISED: Color32 = Color32::from_rgb(42, 35, 29);
    /// Hovered buttons.
    pub const RAISED_HOVER: Color32 = Color32::from_rgb(54, 44, 35);
    /// Thin separators.
    pub const LINE: Color32 = Color32::from_rgb(58, 47, 38);
    pub const TEXT: Color32 = Color32::from_rgb(239, 228, 214);
    pub const TEXT_WEAK: Color32 = Color32::from_rgb(168, 153, 138);
    pub const TEXT_FAINT: Color32 = Color32::from_rgb(112, 100, 89);
    pub const ERROR: Color32 = Color32::from_rgb(240, 96, 80);
    pub const OK: Color32 = Color32::from_rgb(150, 200, 120);
}

/// Scales a color's brightness (0 = black, 1 = unchanged). Alpha is kept.
pub fn shade(c: Color32, factor: f32) -> Color32 {
    let f = |v: u8| (v as f32 * factor).round().clamp(0.0, 255.0) as u8;
    Color32::from_rgba_premultiplied(f(c.r()), f(c.g()), f(c.b()), c.a())
}
