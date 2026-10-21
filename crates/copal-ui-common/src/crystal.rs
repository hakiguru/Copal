//! The amber crystal material used for borders: a small pixel-art texture with facets and
//! glints, generated in code so it always matches the logo palette.

use egui::{
    Color32, ColorImage, Context, Mesh, Painter, Pos2, Rect, Shape, TextureHandle, TextureId,
    TextureOptions, TextureWrapMode, Vec2, pos2, vec2,
};

use crate::palette::{amber, shade};

/// Length of the repeating texture, in texels.
pub const TILE: usize = 64;
/// Rows across the band: 0 = outer edge (lit), last = inner edge (in shadow).
pub const ROWS: usize = 4;

/// Builds the crystal "rod" texture: `TILE` texels along the border, `ROWS` across it.
///
/// Across the band it is shaded like a bevel (bright outer ridge → dark inner side).
/// Along the band it is cut into facets of different lengths with slanted edges,
/// a bright ridge at every cut and a few glints, like the logo crystal.
pub fn crystal_image() -> ColorImage {
    // Facet lengths along the rod; they add up to TILE so the texture repeats seamlessly.
    const FACETS: [usize; 7] = [9, 13, 6, 11, 7, 12, 6];
    // Tone offset of each facet, so neighbouring facets catch the light differently.
    const FACET_TONE: [i32; 7] = [0, 1, -1, 0, 1, -1, 1];
    // Base ramp index per row (outer → inner).
    const ROW_TONE: [i32; ROWS] = [3, 2, 1, 0];
    const GLINTS: [usize; 3] = [4, 30, 49];

    let mut cuts = Vec::new();
    let mut acc = 0;
    for len in FACETS {
        cuts.push(acc);
        acc += len;
    }
    debug_assert_eq!(acc, TILE);

    let mut pixels = Vec::with_capacity(TILE * ROWS);
    for (row, &row_tone) in ROW_TONE.iter().enumerate() {
        for x in 0..TILE {
            // Slanted facet edges: each row's cut is shifted by one texel.
            let shifted = (x + TILE - row) % TILE;
            let facet = cuts.iter().rposition(|&c| shifted >= c).unwrap_or(0);
            let mut tone = row_tone + FACET_TONE[facet];
            if shifted == cuts[facet] {
                tone += 2; // bright ridge on the cut
            } else if shifted + 1 == cuts.get(facet + 1).copied().unwrap_or(TILE) {
                tone -= 1; // shadow just before the next cut
            }
            if row == 0 && GLINTS.contains(&x) {
                tone = 5;
            }
            pixels.push(amber::RAMP[tone.clamp(0, 6) as usize]);
        }
    }
    ColorImage::new([TILE, ROWS], pixels)
}

/// Loads the crystal texture into egui. Keep the handle alive (store it in your app).
pub fn load_texture(ctx: &Context) -> TextureHandle {
    let options = TextureOptions {
        wrap_mode: TextureWrapMode::Repeat,
        ..TextureOptions::NEAREST
    };
    ctx.load_texture("copal-crystal", crystal_image(), options)
}

/// How a crystal border looks.
#[derive(Debug, Clone, Copy)]
pub struct Border {
    /// Width of the band in texels.
    pub texels: u32,
    /// Size of one texel in points (1 for buttons, 2 for windows).
    pub texel_size: f32,
    /// Overall brightness, 0..=1. Hover = brighter.
    pub brightness: f32,
    /// Cut the outer corner texel, like the corners of the logo crystal.
    pub chamfer: bool,
}

impl Border {
    pub const BUTTON: Self = Self {
        texels: 2,
        texel_size: 1.0,
        brightness: 0.9,
        chamfer: true,
    };
    pub const PANEL: Self = Self {
        texels: 2,
        texel_size: 1.5,
        brightness: 0.86,
        chamfer: true,
    };
    pub const WINDOW: Self = Self {
        texels: 4,
        texel_size: 1.5,
        brightness: 1.0,
        chamfer: true,
    };

    pub fn width(&self) -> f32 {
        self.texels as f32 * self.texel_size
    }

    pub fn with_brightness(mut self, b: f32) -> Self {
        self.brightness = b;
        self
    }
}

/// Light comes from the top left, like on the logo: top edge brightest, bottom darkest.
const SIDE_LIGHT: [f32; 4] = [1.0, 0.8, 0.94, 0.86]; // top, bottom, left, right

/// Adds a quad whose texture runs along `along` and across from `outer` to the inside.
fn add_band(mesh: &mut Mesh, quad: [Pos2; 4], u0: f32, u1: f32, color: Color32) {
    // quad: outer-start, outer-end, inner-end, inner-start
    let uvs = [pos2(u0, 0.0), pos2(u1, 0.0), pos2(u1, 1.0), pos2(u0, 1.0)];
    let i = mesh.vertices.len() as u32;
    for (p, uv) in quad.into_iter().zip(uvs) {
        mesh.vertices
            .push(egui::epaint::Vertex { pos: p, uv, color });
    }
    mesh.add_triangle(i, i + 1, i + 2);
    mesh.add_triangle(i, i + 2, i + 3);
}

/// Paints a crystal border just inside `rect`.
///
/// The pattern is anchored to the screen, so it does not crawl when a panel is resized.
pub fn paint_border(painter: &Painter, texture: TextureId, rect: Rect, border: Border) {
    let t = border.texel_size;
    let w = border.width();
    if rect.width() < 2.0 * w + 2.0 || rect.height() < 2.0 * w + 2.0 {
        return;
    }
    let tint = |side: usize| {
        Color32::from_gray((255.0 * border.brightness * SIDE_LIGHT[side]).round() as u8)
    };
    let u = |coord: f32| coord / (t * TILE as f32);
    let (l, r, top, b) = (rect.left(), rect.right(), rect.top(), rect.bottom());

    let mut mesh = Mesh::with_texture(texture);
    // Top: outer edge is the top line.
    add_band(
        &mut mesh,
        [
            pos2(l + w, top),
            pos2(r - w, top),
            pos2(r - w, top + w),
            pos2(l + w, top + w),
        ],
        u(l + w),
        u(r - w),
        tint(0),
    );
    // Bottom: outer edge is the bottom line.
    add_band(
        &mut mesh,
        [
            pos2(l + w, b),
            pos2(r - w, b),
            pos2(r - w, b - w),
            pos2(l + w, b - w),
        ],
        u(l + w),
        u(r - w),
        tint(1),
    );
    // Left: runs down, outer edge is the left line.
    add_band(
        &mut mesh,
        [
            pos2(l, top + w),
            pos2(l, b - w),
            pos2(l + w, b - w),
            pos2(l + w, top + w),
        ],
        u(top + w),
        u(b - w),
        tint(2),
    );
    // Right.
    add_band(
        &mut mesh,
        [
            pos2(r, top + w),
            pos2(r, b - w),
            pos2(r - w, b - w),
            pos2(r - w, top + w),
        ],
        u(top + w),
        u(b - w),
        tint(3),
    );
    painter.add(Shape::mesh(mesh));

    // Corners: solid bevel tones, texel by texel, with the outer texel cut off.
    let n = border.texels as i32;
    let corners = [
        (l, top, 1.0, 1.0, 0),
        (r, top, -1.0, 1.0, 3),
        (l, b, 1.0, -1.0, 2),
        (r, b, -1.0, -1.0, 1),
    ];
    for (cx, cy, dx, dy, side) in corners {
        for i in 0..n {
            for j in 0..n {
                if border.chamfer && i + j == 0 {
                    continue;
                }
                let depth = i.min(j) as usize * ROWS / n as usize;
                let tone = [3, 2, 1, 0][depth.min(3)];
                let c = shade(amber::RAMP[tone], border.brightness * SIDE_LIGHT[side]);
                let x0 = cx + dx * i as f32 * t;
                let y0 = cy + dy * j as f32 * t;
                painter.rect_filled(
                    Rect::from_two_pos(pos2(x0, y0), pos2(x0 + dx * t, y0 + dy * t)),
                    0.0,
                    c,
                );
            }
        }
    }

    // Crisp dark outline outside the band, following the cut corners.
    let o = amber::OUTLINE;
    let c = if border.chamfer { t } else { 0.0 };
    for line in [
        Rect::from_two_pos(pos2(l + c, top - 1.0), pos2(r - c, top)),
        Rect::from_two_pos(pos2(l + c, b), pos2(r - c, b + 1.0)),
        Rect::from_two_pos(pos2(l - 1.0, top + c), pos2(l, b - c)),
        Rect::from_two_pos(pos2(r, top + c), pos2(r + 1.0, b - c)),
    ] {
        painter.rect_filled(line, 0.0, o);
    }
    if border.chamfer {
        for (cx, cy, dx, dy) in [
            (l, top, 1.0, 1.0),
            (r, top, -1.0, 1.0),
            (l, b, 1.0, -1.0),
            (r, b, -1.0, -1.0),
        ] {
            painter.rect_filled(
                Rect::from_two_pos(pos2(cx, cy), pos2(cx + dx * t, cy + dy * t)),
                0.0,
                o,
            );
        }
    }
    // Thin dark line on the inside, so the band separates cleanly from the content.
    painter.rect_stroke(
        rect.shrink(w),
        0.0,
        (1.0, shade(o, 0.7)),
        egui::StrokeKind::Outside,
    );
}

/// A four-point glint like the ones around the logo. `size` = arm length in texels (1 or 2).
pub fn paint_sparkle(painter: &Painter, center: Pos2, texel: f32, size: u32) {
    let sq = |p: Vec2, c: Color32| {
        let min = center + p * texel - vec2(texel, texel) * 0.5;
        painter.rect_filled(Rect::from_min_size(min, vec2(texel, texel)), 0.0, c);
    };
    sq(Vec2::ZERO, amber::GLINT);
    for d in [
        vec2(1.0, 0.0),
        vec2(-1.0, 0.0),
        vec2(0.0, 1.0),
        vec2(0.0, -1.0),
    ] {
        if size >= 2 {
            sq(d, amber::HIGHLIGHT);
            sq(d * 2.0, amber::LIGHT);
        } else {
            sq(d, amber::LIGHT);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn texture_uses_only_logo_colors() {
        let img = crystal_image();
        assert_eq!(img.size, [TILE, ROWS]);
        assert!(img.pixels.iter().all(|p| amber::RAMP.contains(p)));
    }

    #[test]
    fn outer_row_is_brighter_than_inner_row() {
        let img = crystal_image();
        let lum = |row: usize| -> u32 {
            (0..TILE)
                .map(|x| img.pixels[row * TILE + x].g() as u32)
                .sum()
        };
        assert!(lum(0) > lum(ROWS - 1));
    }
}
