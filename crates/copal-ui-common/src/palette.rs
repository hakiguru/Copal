//! Colors. The amber tones are the same as in the logo (`assets/make_logo.py`).
//!
//! [`Palette`] is the set of colors one theme uses; widgets never hard-code colors.

use copal_core::{ColorMode, Settings, ThemeStyle};
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

/// Warm browns of the art theme.
pub mod ink {
    use egui::Color32;
    pub const VOID: Color32 = Color32::from_rgb(16, 13, 11);
    pub const BASE: Color32 = Color32::from_rgb(24, 20, 17);
    pub const PANEL: Color32 = Color32::from_rgb(31, 26, 22);
    pub const RAISED: Color32 = Color32::from_rgb(42, 35, 29);
    pub const RAISED_HOVER: Color32 = Color32::from_rgb(54, 44, 35);
    pub const LINE: Color32 = Color32::from_rgb(58, 47, 38);
    pub const TEXT: Color32 = Color32::from_rgb(239, 228, 214);
    pub const TEXT_WEAK: Color32 = Color32::from_rgb(168, 153, 138);
    pub const TEXT_FAINT: Color32 = Color32::from_rgb(112, 100, 89);
    pub const ERROR: Color32 = Color32::from_rgb(240, 96, 80);
}

/// Scales a color's brightness (0 = black, 1 = unchanged). Alpha is kept.
pub fn shade(c: Color32, factor: f32) -> Color32 {
    let f = |v: u8| (v as f32 * factor).round().clamp(0.0, 255.0) as u8;
    Color32::from_rgba_premultiplied(f(c.r()), f(c.g()), f(c.b()), c.a())
}

const fn rgb(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

/// All colors of one theme.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palette {
    pub dark: bool,
    /// 3D view background.
    pub void: Color32,
    /// Top bar and status bar.
    pub base: Color32,
    pub panel: Color32,
    /// Buttons and fields.
    pub raised: Color32,
    pub raised_hover: Color32,
    pub pressed: Color32,
    /// Separators.
    pub line: Color32,
    pub text: Color32,
    pub text_weak: Color32,
    pub text_faint: Color32,
    /// Outlines of windows and buttons.
    pub accent: Color32,
    pub accent_hover: Color32,
    /// Text drawn in the accent color (selected tab, primary button).
    pub accent_text: Color32,
    pub primary_fill: Color32,
    pub primary_fill_hover: Color32,
    /// Background of selected list rows and text selection.
    pub selection: Color32,
    pub error: Color32,
    // 3D view
    pub plate: Color32,
    pub plate_edge: Color32,
    pub grid: Color32,
    pub volume: Color32,
    pub object: Color32,
    pub object_selected: Color32,
    // Terminal
    pub term_bg: Color32,
    pub term_fg: Color32,
    pub term_user: Color32,
    pub term_path: Color32,
    pub term_error: Color32,
}

/// The logo orange, used for outlines in the default theme.
const ORANGE: Color32 = amber::MID;

impl Palette {
    pub fn for_settings(s: &Settings) -> Self {
        match (s.theme, s.effective_mode()) {
            (ThemeStyle::Art, ColorMode::Dark) => Self::ART,
            (ThemeStyle::Art, ColorMode::Light) => Self::ART_LIGHT,
            (ThemeStyle::Default, ColorMode::Dark) => Self::DARK,
            (ThemeStyle::Default, ColorMode::Light) => Self::LIGHT,
        }
    }

    /// Neutral dark, like the dark themes of IDEs and chat apps.
    pub const DARK: Self = Self {
        dark: true,
        void: rgb(30, 31, 34),
        base: rgb(35, 36, 39),
        panel: rgb(43, 45, 48),
        raised: rgb(54, 56, 60),
        raised_hover: rgb(66, 68, 73),
        pressed: rgb(47, 49, 52),
        line: rgb(60, 62, 66),
        text: rgb(223, 225, 229),
        text_weak: rgb(157, 160, 168),
        text_faint: rgb(111, 115, 122),
        accent: ORANGE,
        accent_hover: amber::LIGHT,
        accent_text: rgb(247, 150, 62),
        primary_fill: rgb(54, 56, 60),
        primary_fill_hover: rgb(66, 68, 73),
        selection: Color32::from_rgba_premultiplied(60, 31, 5, 64),
        error: rgb(240, 96, 80),
        plate: rgb(40, 42, 45),
        plate_edge: rgb(200, 100, 18),
        grid: rgb(56, 58, 62),
        volume: Color32::from_rgba_premultiplied(36, 37, 40, 64),
        object: rgb(172, 176, 183),
        object_selected: ORANGE,
        term_bg: rgb(24, 25, 27),
        term_fg: rgb(210, 212, 216),
        term_user: rgb(115, 201, 90),
        term_path: rgb(114, 159, 207),
        term_error: rgb(240, 96, 80),
    };

    /// Neutral light: white and cool greys, no brown.
    pub const LIGHT: Self = Self {
        dark: false,
        void: rgb(236, 238, 241),
        base: rgb(247, 248, 250),
        panel: rgb(255, 255, 255),
        raised: rgb(255, 255, 255),
        raised_hover: rgb(243, 244, 246),
        pressed: rgb(232, 234, 237),
        line: rgb(222, 224, 228),
        text: rgb(30, 31, 34),
        text_weak: rgb(95, 99, 110),
        text_faint: rgb(150, 155, 165),
        accent: rgb(232, 112, 10),
        accent_hover: rgb(245, 135, 25),
        accent_text: rgb(196, 90, 0),
        primary_fill: rgb(255, 255, 255),
        primary_fill_hover: rgb(255, 245, 235),
        selection: Color32::from_rgba_premultiplied(42, 21, 2, 44),
        error: rgb(200, 40, 30),
        plate: rgb(252, 252, 253),
        plate_edge: rgb(232, 112, 10),
        grid: rgb(222, 225, 230),
        volume: Color32::from_rgba_premultiplied(70, 74, 82, 70),
        object: rgb(160, 165, 173),
        object_selected: ORANGE,
        term_bg: rgb(255, 255, 255),
        term_fg: rgb(30, 31, 34),
        term_user: rgb(38, 130, 30),
        term_path: rgb(30, 90, 180),
        term_error: rgb(200, 40, 30),
    };

    /// Warm amber on parchment, with the same crystal borders.
    pub const ART_LIGHT: Self = Self {
        dark: false,
        void: rgb(236, 228, 216),
        base: rgb(244, 237, 227),
        panel: rgb(250, 245, 238),
        raised: rgb(255, 251, 245),
        raised_hover: rgb(250, 238, 222),
        pressed: rgb(238, 226, 210),
        line: rgb(222, 206, 186),
        text: rgb(46, 33, 23),
        text_weak: rgb(112, 90, 70),
        text_faint: rgb(160, 139, 118),
        accent: amber::SHADOW,
        accent_hover: amber::MID,
        accent_text: rgb(168, 66, 4),
        primary_fill: rgb(255, 234, 204),
        primary_fill_hover: rgb(255, 222, 180),
        selection: Color32::from_rgba_premultiplied(120, 60, 8, 60),
        error: rgb(190, 42, 28),
        plate: rgb(246, 238, 226),
        plate_edge: amber::SHADOW,
        grid: rgb(222, 207, 188),
        volume: Color32::from_rgba_premultiplied(90, 64, 40, 70),
        object: rgb(150, 138, 126),
        object_selected: amber::MID,
        term_bg: rgb(250, 245, 238),
        term_fg: rgb(88, 66, 50),
        term_user: amber::DEEP,
        term_path: amber::SHADOW,
        term_error: rgb(190, 42, 28),
    };

    /// Warm amber with crystal borders.
    pub const ART: Self = Self {
        dark: true,
        void: ink::VOID,
        base: ink::BASE,
        panel: ink::PANEL,
        raised: ink::RAISED,
        raised_hover: ink::RAISED_HOVER,
        pressed: rgb(34, 28, 23),
        line: ink::LINE,
        text: ink::TEXT,
        text_weak: ink::TEXT_WEAK,
        text_faint: ink::TEXT_FAINT,
        accent: amber::MID,
        accent_hover: amber::LIGHT,
        accent_text: amber::HIGHLIGHT,
        primary_fill: rgb(56, 19, 2),
        primary_fill_hover: rgb(74, 24, 3),
        selection: rgb(132, 44, 4),
        error: ink::ERROR,
        plate: ink::BASE,
        plate_edge: rgb(141, 46, 5),
        grid: ink::LINE,
        volume: Color32::from_rgba_premultiplied(19, 16, 13, 40),
        object: rgb(196, 184, 170),
        object_selected: amber::LIGHT,
        term_bg: ink::PANEL,
        term_fg: ink::TEXT_WEAK,
        term_user: amber::LIGHTER,
        term_path: amber::LIGHT,
        term_error: ink::ERROR,
    };
}
