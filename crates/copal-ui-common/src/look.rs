//! Everything widgets need to draw themselves: settings, colors, textures and outlines.

use copal_core::{Settings, ThemeStyle};
use egui::{
    ColorImage, Context, CornerRadius, Painter, Rect, Stroke, StrokeKind, TextureHandle,
    TextureOptions,
};

use crate::crystal::{self, Border};
use crate::palette::Palette;
use crate::theme::{self, Edition, RADIUS};

static LOGO_PNG: &[u8] = include_bytes!("../../../assets/logo-16.png");
static WORDMARK_PNG: &[u8] = include_bytes!("../../../assets/wordmark-1x.png");
static ICON_PNG: &[u8] = include_bytes!("../../../assets/logo-256.png");

/// What kind of thing gets an outline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    Button,
    Panel,
    /// The 3D view.
    Stage,
}

/// Interaction state, so outlines can react to the pointer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Idle,
    Hovered,
    Pressed,
    Selected,
}

/// The look of one app. Create once at startup, keep it, call [`Look::apply`] when settings change.
pub struct Look {
    pub edition: Edition,
    pub settings: Settings,
    pub pal: Palette,
    pub crystal: TextureHandle,
    pub logo: TextureHandle,
    pub wordmark: TextureHandle,
}

impl Look {
    pub fn new(ctx: &Context, edition: Edition, settings: &Settings) -> Self {
        let pal = Palette::for_settings(settings);
        theme::apply(ctx, edition, settings, &pal);
        Self {
            edition,
            settings: settings.clone(),
            pal,
            crystal: crystal::load_texture(ctx),
            logo: ctx.load_texture("copal-logo", decode(LOGO_PNG), TextureOptions::NEAREST),
            wordmark: ctx.load_texture(
                "copal-wordmark",
                decode(WORDMARK_PNG),
                TextureOptions::NEAREST,
            ),
        }
    }

    /// Switches theme or color mode.
    pub fn apply(&mut self, ctx: &Context, settings: &Settings) {
        self.settings = settings.clone();
        self.pal = Palette::for_settings(settings);
        theme::apply(ctx, self.edition, settings, &self.pal);
    }

    pub fn art(&self) -> bool {
        self.settings.theme == ThemeStyle::Art
    }

    /// Glints and other pure decoration: only the art theme in the full app.
    pub fn decorations(&self) -> bool {
        self.art() && self.edition == Edition::Full
    }

    /// Corner radius for filled shapes (buttons, tabs).
    pub fn radius(&self, edge: Edge) -> CornerRadius {
        match (self.art(), edge) {
            (true, _) => CornerRadius::ZERO,
            (false, Edge::Button) => CornerRadius::same(RADIUS),
            (false, _) => CornerRadius::same(6),
        }
    }

    /// Draws the outline of a button, panel or the 3D view just inside `rect`.
    ///
    /// Default theme: a simple orange line. Art theme: the amber crystal border.
    pub fn outline(&self, painter: &Painter, rect: Rect, edge: Edge, state: State) {
        if self.art() {
            let base = match edge {
                Edge::Button => Border::BUTTON,
                Edge::Panel => Border::PANEL,
                Edge::Stage => Border::WINDOW,
            };
            let brightness = match state {
                State::Idle => base.brightness,
                State::Hovered | State::Selected => 1.0,
                State::Pressed => 0.8,
            };
            crystal::paint_border(
                painter,
                self.crystal.id(),
                rect,
                base.with_brightness(brightness),
            );
        } else {
            let color = match state {
                State::Idle | State::Selected => self.pal.accent,
                State::Hovered | State::Pressed => self.pal.accent_hover,
            };
            let width = if state == State::Selected { 1.5 } else { 1.0 };
            painter.rect_stroke(
                rect,
                self.radius(edge),
                Stroke::new(width, color),
                StrokeKind::Inside,
            );
        }
    }
}

fn decode(png: &[u8]) -> ColorImage {
    let img = image::load_from_memory_with_format(png, image::ImageFormat::Png)
        .expect("bundled logo PNG is valid")
        .to_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    ColorImage::from_rgba_unmultiplied(size, img.as_raw())
}

/// The window icon (the crystal).
pub fn icon() -> egui::IconData {
    let img = image::load_from_memory_with_format(ICON_PNG, image::ImageFormat::Png)
        .expect("bundled icon PNG is valid")
        .to_rgba8();
    egui::IconData {
        width: img.width(),
        height: img.height(),
        rgba: img.into_raw(),
    }
}
