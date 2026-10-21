//! Textures and settings every widget needs: the crystal material and the logo images.

use egui::{ColorImage, Context, TextureHandle, TextureOptions};

use crate::crystal;
use crate::theme::{self, Edition};

static LOGO_PNG: &[u8] = include_bytes!("../../../assets/logo-16.png");
static WORDMARK_PNG: &[u8] = include_bytes!("../../../assets/wordmark-1x.png");

/// Shared look of one app. Create once at startup and keep it.
pub struct Look {
    pub edition: Edition,
    pub crystal: TextureHandle,
    pub logo: TextureHandle,
    pub wordmark: TextureHandle,
}

impl Look {
    pub fn new(ctx: &Context, edition: Edition) -> Self {
        theme::apply(ctx, edition);
        Self {
            edition,
            crystal: crystal::load_texture(ctx),
            logo: ctx.load_texture("copal-logo", decode(LOGO_PNG), TextureOptions::NEAREST),
            wordmark: ctx.load_texture(
                "copal-wordmark",
                decode(WORDMARK_PNG),
                TextureOptions::NEAREST,
            ),
        }
    }

    /// Glints and other pure decoration are only drawn in the full app.
    pub fn decorations(&self) -> bool {
        self.edition == Edition::Full
    }
}

fn decode(png: &[u8]) -> ColorImage {
    let img = image::load_from_memory_with_format(png, image::ImageFormat::Png)
        .expect("bundled logo PNG is valid")
        .to_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    ColorImage::from_rgba_unmultiplied(size, img.as_raw())
}

static ICON_PNG: &[u8] = include_bytes!("../../../assets/logo-256.png");

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
