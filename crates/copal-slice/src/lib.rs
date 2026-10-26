//! Turns triangle meshes into layer images.
//!
//! How it works, per layer:
//! 1. Every triangle that crosses the middle of the layer gives one line segment. Segments are
//!    oriented from the triangle's edge that goes down to the edge that goes up, so outer
//!    contours run counter-clockwise (seen from above) and holes clockwise.
//! 2. A scanline pass fills everything with a positive winding number. That handles touching
//!    and overlapping models and supports (they add up), hollow models with an inside-out inner
//!    wall, and drain holes (inside-out cylinders), which subtract wherever they are.
//! 3. Anti-aliasing takes several sub-rows per pixel row and exact horizontal coverage, so edge
//!    pixels get grey values.
//!
//! Rows are streamed to a [`RowSink`] one by one, so a 16K layer never has to sit in memory as a
//! whole image. Layers are independent and can be rendered in parallel.

use glam::{Vec2, Vec3};

/// The printer screen: pixel grid, its size in mm and mirroring.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Raster {
    pub width: u32,
    pub height: u32,
    /// Size of one pixel in mm (X, Y).
    pub pixel_mm: Vec2,
    /// Flip the image left-right.
    pub mirror_x: bool,
    /// Flip the image top-bottom.
    pub mirror_y: bool,
}

impl Raster {
    pub fn new(resolution: [u32; 2], size_mm: [f32; 2]) -> Self {
        Self {
            width: resolution[0],
            height: resolution[1],
            pixel_mm: Vec2::new(
                size_mm[0] / resolution[0] as f32,
                size_mm[1] / resolution[1] as f32,
            ),
            mirror_x: false,
            mirror_y: false,
        }
    }

    /// The same screen at a lower resolution, for previews. Keeps the aspect of the pixels.
    pub fn scaled(&self, factor: f32) -> Self {
        let width = ((self.width as f32 * factor).round() as u32).max(1);
        let height = ((self.height as f32 * factor).round() as u32).max(1);
        Self {
            width,
            height,
            pixel_mm: Vec2::new(
                self.pixel_mm.x * self.width as f32 / width as f32,
                self.pixel_mm.y * self.height as f32 / height as f32,
            ),
            ..*self
        }
    }

    pub fn size_mm(&self) -> Vec2 {
        Vec2::new(
            self.pixel_mm.x * self.width as f32,
            self.pixel_mm.y * self.height as f32,
        )
    }

    /// Plate millimetres (origin in the middle of the plate, +Y to the back) to continuous
    /// pixel coordinates of the final image. Without mirroring the column grows with X and
    /// row 0 is the back edge of the plate.
    pub fn to_pixels(&self, p: Vec2) -> Vec2 {
        let col = p.x / self.pixel_mm.x;
        let row = -p.y / self.pixel_mm.y;
        Vec2::new(
            self.width as f32 / 2.0 + if self.mirror_x { -col } else { col },
            self.height as f32 / 2.0 + if self.mirror_y { -row } else { row },
        )
    }
}

/// Receives the pixels of one layer in order (row-major, top row first).
pub trait RowSink {
    /// `count` pixels with value 0. Whole empty rows come this way too.
    fn zeros(&mut self, count: usize);
    /// The next pixels (part of a row, a row, or more).
    fn pixels(&mut self, pixels: &[u8]);
}

/// Collects a layer into a plain image buffer (row-major, one byte per pixel).
#[derive(Debug, Default, Clone)]
pub struct ImageSink {
    pub pixels: Vec<u8>,
}

impl RowSink for ImageSink {
    fn zeros(&mut self, count: usize) {
        self.pixels.resize(self.pixels.len() + count, 0);
    }
    fn pixels(&mut self, pixels: &[u8]) {
        self.pixels.extend_from_slice(pixels);
    }
}

/// Statistics of one rendered layer.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct LayerStats {
    /// Sum of all pixel values / 255: the lit area in pixels.
    pub lit_pixels: f64,
    /// Bounding box of lit pixels in the final (mirrored) image: min col, min row, max col, max row.
    pub bounds: Option<[u32; 4]>,
}

#[derive(Debug, Clone, Copy)]
struct Seg {
    /// Start and end in pixel coordinates (row axis = y).
    a: Vec2,
    b: Vec2,
}

/// Prepared geometry, ready to render any layer.
#[derive(Debug)]
pub struct Slicer {
    tris: Vec<[Vec3; 3]>,
    /// For each bucket of `BUCKET_MM` height: triangles that reach into it.
    buckets: Vec<Vec<u32>>,
    pub raster: Raster,
    pub layer_height: f32,
    /// Sub-samples per pixel row: 1 (anti-aliasing off), 2, 4 or 8.
    pub anti_aliasing: u32,
    layers: u32,
}

const BUCKET_MM: f32 = 0.5;

impl Slicer {
    /// `triangles` are in plate coordinates (mm, origin in the middle of the plate, z up).
    pub fn new(
        triangles: Vec<[Vec3; 3]>,
        raster: Raster,
        layer_height: f32,
        anti_aliasing: u32,
    ) -> Self {
        let top = triangles
            .iter()
            .flat_map(|t| t.iter().map(|v| v.z))
            .fold(0.0f32, f32::max);
        let layers = if triangles.is_empty() {
            0
        } else {
            (top / layer_height).ceil().max(1.0) as u32
        };
        let nb = (top / BUCKET_MM).floor() as usize + 1;
        let mut buckets = vec![Vec::new(); nb];
        for (i, t) in triangles.iter().enumerate() {
            let lo = t[0].z.min(t[1].z).min(t[2].z).max(0.0);
            let hi = t[0].z.max(t[1].z).max(t[2].z);
            if hi < 0.0 {
                continue;
            }
            let (b0, b1) = (
                (lo / BUCKET_MM) as usize,
                ((hi / BUCKET_MM) as usize).min(nb - 1),
            );
            for b in &mut buckets[b0.min(nb - 1)..=b1] {
                b.push(i as u32);
            }
        }
        Self {
            tris: triangles,
            buckets,
            raster,
            layer_height,
            anti_aliasing: anti_aliasing.clamp(1, 16),
            layers,
        }
    }

    pub fn layer_count(&self) -> u32 {
        self.layers
    }

    /// Height of the cut for layer `i`: the middle of the layer.
    pub fn layer_z(&self, i: u32) -> f32 {
        (i as f32 + 0.5) * self.layer_height
    }

    fn segments(&self, z: f32) -> Vec<Seg> {
        let mut segs = Vec::new();
        let Some(bucket) = self.buckets.get((z / BUCKET_MM) as usize) else {
            return segs;
        };
        for &ti in bucket {
            let t = &self.tris[ti as usize];
            let above = [t[0].z > z, t[1].z > z, t[2].z > z];
            let n_above = above.iter().filter(|a| **a).count();
            if n_above == 0 || n_above == 3 {
                continue;
            }
            let mut up = None; // point on the edge that goes from below to above
            let mut down = None; // point on the edge that goes from above to below
            for e in 0..3 {
                let (p, q) = (t[e], t[(e + 1) % 3]);
                let (pa, qa) = (above[e], above[(e + 1) % 3]);
                if pa == qa {
                    continue;
                }
                let k = (z - p.z) / (q.z - p.z);
                let x = p + (q - p) * k;
                let x = self.raster.to_pixels(Vec2::new(x.x, x.y));
                if qa {
                    up = Some(x);
                } else {
                    down = Some(x);
                }
            }
            if let (Some(a), Some(b)) = (down, up) {
                segs.push(Seg { a, b });
            }
        }
        segs
    }

    /// Renders layer `i` into `sink` and returns its statistics.
    pub fn render_layer(&self, i: u32, sink: &mut impl RowSink) -> LayerStats {
        let r = &self.raster;
        let (w, h) = (r.width as usize, r.height as usize);
        let mut segs = self.segments(self.layer_z(i));
        let mut stats = LayerStats::default();
        // Row range that has anything at all.
        let (mut lo, mut hi) = (f32::INFINITY, f32::NEG_INFINITY);
        for s in &segs {
            lo = lo.min(s.a.y.min(s.b.y));
            hi = hi.max(s.a.y.max(s.b.y));
        }
        if segs.is_empty() || hi < 0.0 || lo >= h as f32 {
            sink.zeros(w * h);
            return stats;
        }
        let first = (lo.floor().max(0.0)) as usize;
        let last = (hi.ceil() as usize).min(h); // exclusive
        segs.sort_by(|p, q| p.a.y.min(p.b.y).total_cmp(&q.a.y.min(q.b.y)));

        let aa = self.anti_aliasing as usize;
        let mut direct = vec![0f32; w + 2];
        let mut delta = vec![0f32; w + 2];
        let mut out = vec![0u8; w];
        let mut active: Vec<usize> = Vec::new();
        let mut next = 0;
        let mut xs: Vec<(f32, i32)> = Vec::new();

        let mut emitted = 0usize; // pixels already sent to the sink

        let total = w * h;
        // Inside a well-made mesh the winding is +1 in pixel space, or -1 when the image is
        // mirrored once.
        let sign = if r.mirror_x != r.mirror_y { -1 } else { 1 };
        for row in first..last {
            // Columns this row touched, inclusive. Only they are read and cleared, so a small
            // model on a 16K screen costs little.
            let mut touched: Option<(usize, usize)> = None;
            for k in 0..aa {
                let y = row as f32 + (k as f32 + 0.5) / aa as f32;
                while next < segs.len() && segs[next].a.y.min(segs[next].b.y) <= y {
                    active.push(next);
                    next += 1;
                }
                active.retain(|&s| segs[s].a.y.max(segs[s].b.y) > y);
                xs.clear();
                for &si in &active {
                    let s = &segs[si];
                    let (ylo, yhi) = (s.a.y.min(s.b.y), s.a.y.max(s.b.y));
                    // Half-open in y, so a vertex shared by two segments counts once.
                    if y < ylo || y >= yhi {
                        continue;
                    }
                    let t = (y - s.a.y) / (s.b.y - s.a.y);
                    let x = s.a.x + (s.b.x - s.a.x) * t;
                    xs.push((x, if s.b.y > s.a.y { 1 } else { -1 }));
                }
                if xs.len() < 2 {
                    continue;
                }
                xs.sort_unstable_by(|p, q| p.0.total_cmp(&q.0));
                let mut wind = 0;
                let mut start = 0f32;
                for &(x, d) in &xs {
                    let before = wind > 0;
                    wind += d * sign;
                    let after = wind > 0;
                    if !before && after {
                        start = x;
                    } else if before
                        && !after
                        && let Some((c0, c1)) =
                            add_span(&mut direct, &mut delta, start, x, w, aa == 1)
                    {
                        touched = Some(match touched {
                            None => (c0, c1),
                            Some((a, b)) => (a.min(c0), b.max(c1)),
                        });
                    }
                }
            }
            let Some((c0, c1)) = touched else { continue };
            let c1 = c1.min(w - 1);
            let scale = 255.0 / aa as f32;
            let mut run = 0f32;
            let mut lit = 0f64;
            let (mut cmin, mut cmax) = (usize::MAX, 0usize);
            for c in c0..=c1 {
                run += delta[c];
                let v = ((direct[c] + run) * scale).round().clamp(0.0, 255.0) as u8;
                out[c] = v;
                if v > 0 {
                    lit += v as f64;
                    cmin = cmin.min(c);
                    cmax = c;
                }
            }
            let clear_end = (c1 + 2).min(w + 2);
            direct[c0..clear_end].fill(0.0);
            delta[c0..clear_end].fill(0.0);
            if cmin == usize::MAX {
                continue;
            }
            stats.lit_pixels += lit / 255.0;
            let r32 = row as u32;
            stats.bounds = Some(match stats.bounds {
                None => [cmin as u32, r32, cmax as u32, r32],
                Some(b) => [
                    b[0].min(cmin as u32),
                    b[1].min(r32),
                    b[2].max(cmax as u32),
                    b[3].max(r32),
                ],
            });
            let pos = row * w + cmin;
            sink.zeros(pos - emitted);
            sink.pixels(&out[cmin..=cmax]);
            emitted = row * w + cmax + 1;
        }
        sink.zeros(total - emitted);
        stats
    }

    /// Renders layer `i` into a new image buffer.
    pub fn layer_image(&self, i: u32) -> (Vec<u8>, LayerStats) {
        let mut sink = ImageSink {
            pixels: Vec::with_capacity(self.raster.width as usize * self.raster.height as usize),
        };
        let stats = self.render_layer(i, &mut sink);
        (sink.pixels, stats)
    }
}

/// Adds coverage of the span `[a, b)` (pixel units) to one sub-row. Returns the columns it
/// touched (inclusive), if any. With `binary`, a pixel counts only when its center is inside.
fn add_span(
    direct: &mut [f32],
    delta: &mut [f32],
    a: f32,
    b: f32,
    w: usize,
    binary: bool,
) -> Option<(usize, usize)> {
    if binary {
        let c0 = (a - 0.5).ceil().max(0.0) as usize;
        let c1 = ((b - 0.5).ceil().max(0.0) as usize).min(w); // exclusive
        if c0 >= c1 {
            return None;
        }
        delta[c0] += 1.0;
        delta[c1] -= 1.0;
        return Some((c0, c1));
    }
    let a = a.clamp(0.0, w as f32);
    let b = b.clamp(0.0, w as f32);
    if b <= a {
        return None;
    }
    let ia = a.floor() as usize;
    let ib = b.floor() as usize;
    if ia == ib {
        let c = ia.min(w - 1);
        direct[c] += b - a;
        return Some((c, c));
    }
    direct[ia] += ia as f32 + 1.0 - a;
    if ib > ia + 1 {
        delta[ia + 1] += 1.0;
        delta[ib] -= 1.0;
    }
    if ib < w {
        direct[ib] += b - ib as f32;
    }
    Some((ia, ib))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cube(min: Vec3, max: Vec3) -> Vec<[Vec3; 3]> {
        let c = |i: usize| {
            Vec3::new(
                if i & 1 != 0 { max.x } else { min.x },
                if i & 2 != 0 { max.y } else { min.y },
                if i & 4 != 0 { max.z } else { min.z },
            )
        };
        // Outward-facing, counter-clockwise from outside.
        let quads = [
            [0, 2, 3, 1], // bottom (-z)
            [4, 5, 7, 6], // top (+z)
            [0, 1, 5, 4], // -y
            [2, 6, 7, 3], // +y
            [0, 4, 6, 2], // -x
            [1, 3, 7, 5], // +x
        ];
        quads
            .iter()
            .flat_map(|q| [[c(q[0]), c(q[1]), c(q[2])], [c(q[0]), c(q[2]), c(q[3])]])
            .collect()
    }

    fn raster() -> Raster {
        // 100 × 50 pixels of 0.1 mm: a 10 × 5 mm screen.
        Raster::new([100, 50], [10.0, 5.0])
    }

    #[test]
    fn box_fills_exactly_its_pixels() {
        let s = Slicer::new(
            cube(Vec3::new(-2.0, -1.0, 0.0), Vec3::new(2.0, 1.0, 1.0)),
            raster(),
            0.05,
            1,
        );
        assert_eq!(s.layer_count(), 20);
        let (img, stats) = s.layer_image(3);
        assert_eq!(img.len(), 100 * 50);
        assert_eq!(stats.lit_pixels.round() as u32, 40 * 20);
        // x from -2 to 2 mm = columns 30..70; y from -1 to 1 mm = rows 15..35.
        assert_eq!(stats.bounds, Some([30, 15, 69, 34]));
        assert_eq!(img[25 * 100 + 50], 255);
        assert_eq!(img[25 * 100 + 29], 0);
    }

    #[test]
    fn anti_aliasing_gives_grey_edges_and_keeps_area() {
        let s = Slicer::new(
            cube(Vec3::new(-2.03, -1.0, 0.0), Vec3::new(2.03, 1.0, 1.0)),
            raster(),
            0.05,
            8,
        );
        let (img, stats) = s.layer_image(0);
        assert!(
            (stats.lit_pixels - 40.6 * 20.0).abs() < 1.0,
            "{}",
            stats.lit_pixels
        );
        let edge = img[25 * 100 + 29];
        assert!(edge > 50 && edge < 110, "edge pixel {edge}");
    }

    #[test]
    fn inner_inverted_wall_makes_a_hole_and_overlaps_merge() {
        let mut tris = cube(Vec3::new(-3.0, -2.0, 0.0), Vec3::new(3.0, 2.0, 1.0));
        // Inner box with reversed faces = hollow.
        tris.extend(
            cube(Vec3::new(-1.0, -1.0, 0.0), Vec3::new(1.0, 1.0, 1.0))
                .into_iter()
                .map(|[a, b, c]| [a, c, b]),
        );
        let s = Slicer::new(tris, raster(), 0.1, 1);
        let (img, stats) = s.layer_image(2);
        assert_eq!(img[25 * 100 + 50], 0, "hole in the middle");
        assert_eq!(img[25 * 100 + 25], 255, "wall");
        assert_eq!(stats.lit_pixels.round() as u32, 60 * 40 - 20 * 20);

        // Two overlapping boxes: the union, no hole where they overlap.
        let mut tris = cube(Vec3::new(-3.0, -1.0, 0.0), Vec3::new(1.0, 1.0, 1.0));
        tris.extend(cube(Vec3::new(-1.0, -1.0, 0.0), Vec3::new(3.0, 1.0, 1.0)));
        let s = Slicer::new(tris, raster(), 0.1, 1);
        let (img, stats) = s.layer_image(2);
        assert_eq!(img[25 * 100 + 50], 255);
        assert_eq!(stats.lit_pixels.round() as u32, 60 * 20);
    }

    #[test]
    fn mirroring_flips_the_image() {
        // An L-shape only in the back-left quarter (−X, +Y).
        let tris = cube(Vec3::new(-4.0, 1.0, 0.0), Vec3::new(-2.0, 2.0, 1.0));
        let mut r = raster();
        let plain = Slicer::new(tris.clone(), r, 0.1, 1).layer_image(0);
        assert_eq!(
            plain.1.bounds,
            Some([10, 5, 29, 14]),
            "back-left = top-left"
        );
        r.mirror_x = true;
        let mx = Slicer::new(tris.clone(), r, 0.1, 1).layer_image(0);
        assert_eq!(mx.1.bounds, Some([70, 5, 89, 14]));
        r.mirror_x = false;
        r.mirror_y = true;
        let my = Slicer::new(tris, r, 0.1, 1).layer_image(0);
        assert_eq!(my.1.bounds, Some([10, 35, 29, 44]));
        assert_eq!(my.0.len(), 100 * 50);
        assert_eq!(my.0[40 * 100 + 20], 255);
        assert_eq!(plain.0[10 * 100 + 20], 255);
    }

    #[test]
    fn nothing_above_the_model() {
        let s = Slicer::new(cube(Vec3::ZERO, Vec3::ONE), raster(), 0.25, 4);
        assert_eq!(s.layer_count(), 4);
        let (img, stats) = s.layer_image(10);
        assert_eq!(img.len(), 5000);
        assert_eq!(stats.bounds, None);
    }
}
