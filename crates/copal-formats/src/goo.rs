//! Elegoo `.goo` files (format V3.0).
//!
//! Written from Elegoo's public format description ("Goo Format Spec V1.2",
//! <https://github.com/elegooofficial/GOO>) and checked against files made by other slicers that
//! printed well. All numbers are big-endian.
//!
//! Layout: a fixed 195 477-byte header (with two RGB565 previews), then for every layer a 66-byte
//! definition, the size of its image data, the image data and `\r\n`, and an 11-byte ending.
//!
//! Layer image data: `0x55`, run-length coded pixels (row-major), one checksum byte.
//! Each run starts with a byte whose top two bits give the kind of run:
//! `00` black, `11` white, `01` grey (the value follows), `10` a small step from the previous
//! value. Bits 5–4 give how many extra length bytes follow (0–3); the length is those bytes
//! (big-endian) followed by the low four bits of the first byte. The checksum is the inverted
//! low byte of the sum of all run bytes.

use std::io::{self, Read, Seek, SeekFrom, Write};

use copal_slice::RowSink;

pub const MAGIC: [u8; 8] = [0x07, 0x00, 0x00, 0x00, 0x44, 0x4C, 0x50, 0x00];
pub const ENDING: [u8; 11] = [
    0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00, 0x44, 0x4C, 0x50, 0x00,
];
pub const HEADER_SIZE: u64 = 195_477;
pub const SMALL_PREVIEW: u32 = 116;
pub const BIG_PREVIEW: u32 = 290;
const LAYER_DEF_SIZE: usize = 66;

#[derive(Debug, thiserror::Error)]
pub enum GooError {
    #[error("{0}")]
    Io(#[from] io::Error),
    #[error("not a .goo file (wrong magic bytes)")]
    NotGoo,
    #[error("layer {layer}: {reason}")]
    BadLayer { layer: u32, reason: String },
}

/// A preview picture: `width × height` pixels in RGB565.
#[derive(Debug, Clone, PartialEq)]
pub struct Preview {
    pub width: u32,
    pub height: u32,
    pub rgb565: Vec<u16>,
}

impl Preview {
    pub fn blank(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            rgb565: vec![0; (width * height) as usize],
        }
    }

    /// From 8-bit RGB pixels.
    pub fn from_rgb(width: u32, height: u32, rgb: &[u8]) -> Self {
        let rgb565 = rgb
            .as_chunks::<3>()
            .0
            .iter()
            .map(|p| ((p[0] as u16 >> 3) << 11) | ((p[1] as u16 >> 2) << 5) | (p[2] as u16 >> 3))
            .collect();
        Self {
            width,
            height,
            rgb565,
        }
    }

    pub fn to_rgb(&self) -> Vec<u8> {
        self.rgb565
            .iter()
            .flat_map(|&c| {
                let r = ((c >> 11) & 0x1F) as u8;
                let g = ((c >> 5) & 0x3F) as u8;
                let b = (c & 0x1F) as u8;
                [
                    (r << 3) | (r >> 2),
                    (g << 2) | (g >> 4),
                    (b << 3) | (b >> 2),
                ]
            })
            .collect()
    }
}

/// Every value in the file header.
#[derive(Debug, Clone, PartialEq)]
pub struct Header {
    pub version: String,
    pub software_info: String,
    pub software_version: String,
    pub file_time: String,
    pub printer_name: String,
    pub printer_type: String,
    pub profile_name: String,
    pub anti_aliasing_level: u16,
    pub grey_level: u16,
    pub blur_level: u16,
    pub small_preview: Preview,
    pub big_preview: Preview,
    pub layer_count: u32,
    pub resolution: [u16; 2],
    pub mirror_x: bool,
    pub mirror_y: bool,
    /// Plate size in mm: X, Y, Z.
    pub size_mm: [f32; 3],
    pub layer_height_mm: f32,
    pub exposure_s: f32,
    /// True: the waits below are used. False: `turn_off_time_s` is used.
    pub delay_mode: bool,
    pub turn_off_time_s: f32,
    pub bottom_before_lift_s: f32,
    pub bottom_after_lift_s: f32,
    pub bottom_after_retract_s: f32,
    pub before_lift_s: f32,
    pub after_lift_s: f32,
    pub after_retract_s: f32,
    pub bottom_exposure_s: f32,
    pub bottom_layers: u32,
    /// Distances in mm and speeds in mm/min, two stages each: `[stage 1, stage 2]`.
    pub bottom_lift: [(f32, f32); 2],
    pub lift: [(f32, f32); 2],
    pub bottom_retract: [(f32, f32); 2],
    pub retract: [(f32, f32); 2],
    pub bottom_pwm: u16,
    pub pwm: u16,
    /// True: every layer uses the values in its own definition.
    pub advance_mode: bool,
    pub print_time_s: u32,
    /// Resin volume in mm³.
    pub volume_mm3: f32,
    pub weight_g: f32,
    pub price: f32,
    pub price_unit: String,
    pub grey_scale: bool,
    pub transition_layers: u16,
}

impl Default for Header {
    fn default() -> Self {
        Self {
            version: "V3.0".into(),
            software_info: "Copal".into(),
            software_version: env!("CARGO_PKG_VERSION").into(),
            file_time: String::new(),
            printer_name: String::new(),
            printer_type: String::new(),
            profile_name: String::new(),
            anti_aliasing_level: 1,
            grey_level: 1,
            blur_level: 0,
            small_preview: Preview::blank(SMALL_PREVIEW, SMALL_PREVIEW),
            big_preview: Preview::blank(BIG_PREVIEW, BIG_PREVIEW),
            layer_count: 0,
            resolution: [0, 0],
            mirror_x: false,
            mirror_y: false,
            size_mm: [0.0; 3],
            layer_height_mm: 0.05,
            exposure_s: 2.5,
            delay_mode: true,
            turn_off_time_s: 0.0,
            bottom_before_lift_s: 0.0,
            bottom_after_lift_s: 0.0,
            bottom_after_retract_s: 0.0,
            before_lift_s: 0.0,
            after_lift_s: 0.0,
            after_retract_s: 0.0,
            bottom_exposure_s: 30.0,
            bottom_layers: 5,
            bottom_lift: [(5.0, 60.0), (0.0, 0.0)],
            lift: [(5.0, 60.0), (0.0, 0.0)],
            bottom_retract: [(5.0, 150.0), (0.0, 0.0)],
            retract: [(5.0, 150.0), (0.0, 0.0)],
            bottom_pwm: 255,
            pwm: 255,
            advance_mode: false,
            print_time_s: 0,
            volume_mm3: 0.0,
            weight_g: 0.0,
            price: 0.0,
            price_unit: String::new(),
            grey_scale: true,
            transition_layers: 0,
        }
    }
}

/// Per-layer values.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct LayerDef {
    pub pause: bool,
    pub pause_z_mm: f32,
    /// Height of the top of this layer.
    pub z_mm: f32,
    pub exposure_s: f32,
    pub turn_off_time_s: f32,
    pub before_lift_s: f32,
    pub after_lift_s: f32,
    pub after_retract_s: f32,
    /// `[(distance mm, speed mm/min); 2]`
    pub lift: [(f32, f32); 2],
    pub retract: [(f32, f32); 2],
    pub pwm: u16,
}

// --- writing helpers ---------------------------------------------------------------------------

struct Out<W: Write> {
    w: W,
}

impl<W: Write> Out<W> {
    fn bytes(&mut self, b: &[u8]) -> io::Result<()> {
        self.w.write_all(b)
    }
    fn u8(&mut self, v: u8) -> io::Result<()> {
        self.bytes(&[v])
    }
    fn u16(&mut self, v: u16) -> io::Result<()> {
        self.bytes(&v.to_be_bytes())
    }
    fn i32(&mut self, v: i32) -> io::Result<()> {
        self.bytes(&v.to_be_bytes())
    }
    fn f32(&mut self, v: f32) -> io::Result<()> {
        self.bytes(&v.to_be_bytes())
    }
    fn text(&mut self, s: &str, len: usize) -> io::Result<()> {
        // Fixed-length field, zero padded. A value may fill the whole field (the file time does).
        let mut buf = vec![0u8; len];
        let b = s.as_bytes();
        let n = b.len().min(len);
        buf[..n].copy_from_slice(&b[..n]);
        self.bytes(&buf)
    }
    fn crlf(&mut self) -> io::Result<()> {
        self.bytes(b"\r\n")
    }
    fn preview(&mut self, p: &Preview, size: u32) -> io::Result<()> {
        let mut buf = Vec::with_capacity((size * size * 2) as usize);
        for y in 0..size {
            for x in 0..size {
                let c = if x < p.width && y < p.height {
                    p.rgb565[(y * p.width + x) as usize]
                } else {
                    0
                };
                buf.extend_from_slice(&c.to_be_bytes());
            }
        }
        self.bytes(&buf)?;
        self.crlf()
    }
}

/// Writes the header. The caller then writes `layer_count` layers with [`write_layer`] and
/// finishes with [`write_ending`].
pub fn write_header<W: Write>(w: W, h: &Header) -> io::Result<()> {
    let mut o = Out { w };
    o.text(&h.version, 4)?;
    o.bytes(&MAGIC)?;
    o.text(&h.software_info, 32)?;
    o.text(&h.software_version, 24)?;
    o.text(&h.file_time, 24)?;
    o.text(&h.printer_name, 32)?;
    o.text(&h.printer_type, 32)?;
    o.text(&h.profile_name, 32)?;
    o.u16(h.anti_aliasing_level)?;
    o.u16(h.grey_level)?;
    o.u16(h.blur_level)?;
    o.preview(&h.small_preview, SMALL_PREVIEW)?;
    o.preview(&h.big_preview, BIG_PREVIEW)?;
    o.i32(h.layer_count as i32)?;
    o.u16(h.resolution[0])?;
    o.u16(h.resolution[1])?;
    o.u8(h.mirror_x as u8)?;
    o.u8(h.mirror_y as u8)?;
    for v in h.size_mm {
        o.f32(v)?;
    }
    o.f32(h.layer_height_mm)?;
    o.f32(h.exposure_s)?;
    o.u8(h.delay_mode as u8)?;
    o.f32(h.turn_off_time_s)?;
    o.f32(h.bottom_before_lift_s)?;
    o.f32(h.bottom_after_lift_s)?;
    o.f32(h.bottom_after_retract_s)?;
    o.f32(h.before_lift_s)?;
    o.f32(h.after_lift_s)?;
    o.f32(h.after_retract_s)?;
    o.f32(h.bottom_exposure_s)?;
    o.i32(h.bottom_layers as i32)?;
    for (d, s) in [
        h.bottom_lift[0],
        h.lift[0],
        h.bottom_retract[0],
        h.retract[0],
        h.bottom_lift[1],
        h.lift[1],
        h.bottom_retract[1],
        h.retract[1],
    ] {
        o.f32(d)?;
        o.f32(s)?;
    }
    o.u16(h.bottom_pwm)?;
    o.u16(h.pwm)?;
    o.u8(h.advance_mode as u8)?;
    o.i32(h.print_time_s as i32)?;
    o.f32(h.volume_mm3)?;
    o.f32(h.weight_g)?;
    o.f32(h.price)?;
    o.text(&h.price_unit, 8)?;
    o.i32(HEADER_SIZE as i32)?;
    o.u8(h.grey_scale as u8)?;
    o.u16(h.transition_layers)?;
    Ok(())
}

/// Writes one layer: its definition and its encoded image (from [`LayerEncoder::finish`]).
pub fn write_layer<W: Write>(w: W, def: &LayerDef, data: &[u8]) -> io::Result<()> {
    let mut o = Out { w };
    o.u16(def.pause as u16)?;
    o.f32(def.pause_z_mm)?;
    o.f32(def.z_mm)?;
    o.f32(def.exposure_s)?;
    o.f32(def.turn_off_time_s)?;
    o.f32(def.before_lift_s)?;
    o.f32(def.after_lift_s)?;
    o.f32(def.after_retract_s)?;
    o.f32(def.lift[0].0)?;
    o.f32(def.lift[0].1)?;
    o.f32(def.lift[1].0)?;
    o.f32(def.lift[1].1)?;
    o.f32(def.retract[0].0)?;
    o.f32(def.retract[0].1)?;
    o.f32(def.retract[1].0)?;
    o.f32(def.retract[1].1)?;
    o.u16(def.pwm)?;
    o.crlf()?;
    o.i32(data.len() as i32)?;
    o.bytes(data)?;
    o.crlf()
}

pub fn write_ending<W: Write>(mut w: W) -> io::Result<()> {
    w.write_all(&ENDING)
}

// --- layer image encoding -------------------------------------------------------------------

/// Run-length encodes one layer image. Feed it rows (it is a [`RowSink`]), then call
/// [`finish`](Self::finish).
#[derive(Debug, Clone)]
pub struct LayerEncoder {
    out: Vec<u8>,
    value: u8,
    run: u64,
}

impl Default for LayerEncoder {
    fn default() -> Self {
        Self::new()
    }
}

/// Longest run one chunk can hold: 3 extra bytes + 4 bits.
const MAX_RUN: u64 = (1 << 28) - 1;

impl LayerEncoder {
    pub fn new() -> Self {
        Self {
            out: vec![0x55],
            value: 0,
            run: 0,
        }
    }

    fn flush(&mut self) {
        while self.run > 0 {
            let n = self.run.min(MAX_RUN);
            self.run -= n;
            let kind: u8 = match self.value {
                0x00 => 0b00,
                0xFF => 0b11,
                _ => 0b01,
            };
            let extra: u8 = match n {
                0..=0xF => 0,
                0x10..=0xFFF => 1,
                0x1000..=0xF_FFFF => 2,
                _ => 3,
            };
            self.out.push((kind << 6) | (extra << 4) | (n & 0xF) as u8);
            if kind == 0b01 {
                self.out.push(self.value);
            }
            let hi = n >> 4;
            for i in (0..extra).rev() {
                self.out.push((hi >> (8 * i)) as u8);
            }
        }
    }

    fn push(&mut self, v: u8, count: u64) {
        if count == 0 {
            return;
        }
        if v != self.value {
            self.flush();
            self.value = v;
        }
        self.run += count;
    }

    /// The encoded data: `0x55`, the runs and the checksum.
    pub fn finish(mut self) -> Vec<u8> {
        self.flush();
        let sum = self.out[1..]
            .iter()
            .fold(0u8, |acc, b| acc.wrapping_add(*b));
        self.out.push(!sum);
        self.out
    }
}

impl RowSink for LayerEncoder {
    fn zeros(&mut self, count: usize) {
        self.push(0, count as u64);
    }

    fn pixels(&mut self, pixels: &[u8]) {
        let mut i = 0;
        while i < pixels.len() {
            let v = pixels[i];
            let start = i;
            while i < pixels.len() && pixels[i] == v {
                i += 1;
            }
            self.push(v, (i - start) as u64);
        }
    }
}

/// Decodes a layer image (`0x55`, runs, checksum) into `pixels` grey values.
pub fn decode_layer(data: &[u8], pixels: usize) -> Result<Vec<u8>, String> {
    let mut out = Vec::with_capacity(pixels);
    let total = decode_runs(data, |v, n| out.extend(std::iter::repeat_n(v, n)))?;
    if total != pixels {
        return Err(format!("decodes to {total} pixels, expected {pixels}"));
    }
    Ok(out)
}

/// Walks the runs of a layer image without building the image: `run(value, count)` for each.
/// Checks the start byte and the checksum. Returns the number of pixels.
pub fn decode_runs(data: &[u8], mut run: impl FnMut(u8, usize)) -> Result<usize, String> {
    if data.len() < 2 || data[0] != 0x55 {
        return Err("layer data does not start with 0x55".into());
    }
    let body = &data[1..data.len() - 1];
    let sum = body.iter().fold(0u8, |a, b| a.wrapping_add(*b));
    if !sum != data[data.len() - 1] {
        return Err("checksum does not match".into());
    }
    let mut total = 0usize;
    let mut prev = 0u8;
    let mut i = 0;
    let take = |i: &mut usize| -> Result<u8, String> {
        let b = *body
            .get(*i)
            .ok_or("layer data ends in the middle of a run")?;
        *i += 1;
        Ok(b)
    };
    while i < body.len() {
        let b0 = take(&mut i)?;
        let kind = b0 >> 6;
        let l = (b0 >> 4) & 3;
        if kind == 0b10 {
            let diff = b0 & 0xF;
            let n = if l & 1 == 1 {
                take(&mut i)? as usize
            } else {
                1
            };
            let v = if l < 2 {
                prev.wrapping_add(diff)
            } else {
                prev.wrapping_sub(diff)
            };
            run(v, n);
            total += n;
            prev = v;
            continue;
        }
        let v = match kind {
            0b00 => 0,
            0b11 => 255,
            _ => take(&mut i)?,
        };
        let mut hi: u64 = 0;
        for _ in 0..l {
            hi = (hi << 8) | take(&mut i)? as u64;
        }
        let n = ((hi << 4) | (b0 & 0xF) as u64) as usize;
        run(v, n);
        total += n;
        prev = v;
    }
    Ok(total)
}

// --- reading --------------------------------------------------------------------------------

struct In<R: Read> {
    r: R,
}

impl<R: Read> In<R> {
    fn bytes(&mut self, n: usize) -> io::Result<Vec<u8>> {
        let mut b = vec![0; n];
        self.r.read_exact(&mut b)?;
        Ok(b)
    }
    fn arr<const N: usize>(&mut self) -> io::Result<[u8; N]> {
        let mut b = [0; N];
        self.r.read_exact(&mut b)?;
        Ok(b)
    }
    fn u8(&mut self) -> io::Result<u8> {
        Ok(self.arr::<1>()?[0])
    }
    fn u16(&mut self) -> io::Result<u16> {
        Ok(u16::from_be_bytes(self.arr()?))
    }
    fn i32(&mut self) -> io::Result<i32> {
        Ok(i32::from_be_bytes(self.arr()?))
    }
    fn f32(&mut self) -> io::Result<f32> {
        Ok(f32::from_be_bytes(self.arr()?))
    }
    fn text(&mut self, len: usize) -> io::Result<String> {
        let b = self.bytes(len)?;
        let end = b.iter().position(|c| *c == 0).unwrap_or(len);
        Ok(String::from_utf8_lossy(&b[..end]).into_owned())
    }
    fn preview(&mut self, size: u32) -> io::Result<Preview> {
        let b = self.bytes((size * size * 2) as usize)?;
        self.bytes(2)?;
        Ok(Preview {
            width: size,
            height: size,
            rgb565: b
                .as_chunks::<2>()
                .0
                .iter()
                .map(|c| u16::from_be_bytes(*c))
                .collect(),
        })
    }
    fn pair(&mut self) -> io::Result<(f32, f32)> {
        Ok((self.f32()?, self.f32()?))
    }
}

pub fn read_header<R: Read>(r: R) -> Result<Header, GooError> {
    let mut i = In { r };
    let version = i.text(4)?;
    if i.arr::<8>()? != MAGIC {
        return Err(GooError::NotGoo);
    }
    let mut h = Header {
        version,
        software_info: i.text(32)?,
        software_version: i.text(24)?,
        file_time: i.text(24)?,
        printer_name: i.text(32)?,
        printer_type: i.text(32)?,
        profile_name: i.text(32)?,
        anti_aliasing_level: i.u16()?,
        grey_level: i.u16()?,
        blur_level: i.u16()?,
        small_preview: i.preview(SMALL_PREVIEW)?,
        big_preview: i.preview(BIG_PREVIEW)?,
        layer_count: i.i32()? as u32,
        ..Header::default()
    };
    h.resolution = [i.u16()?, i.u16()?];
    h.mirror_x = i.u8()? != 0;
    h.mirror_y = i.u8()? != 0;
    h.size_mm = [i.f32()?, i.f32()?, i.f32()?];
    h.layer_height_mm = i.f32()?;
    h.exposure_s = i.f32()?;
    h.delay_mode = i.u8()? != 0;
    h.turn_off_time_s = i.f32()?;
    h.bottom_before_lift_s = i.f32()?;
    h.bottom_after_lift_s = i.f32()?;
    h.bottom_after_retract_s = i.f32()?;
    h.before_lift_s = i.f32()?;
    h.after_lift_s = i.f32()?;
    h.after_retract_s = i.f32()?;
    h.bottom_exposure_s = i.f32()?;
    h.bottom_layers = i.i32()? as u32;
    h.bottom_lift[0] = i.pair()?;
    h.lift[0] = i.pair()?;
    h.bottom_retract[0] = i.pair()?;
    h.retract[0] = i.pair()?;
    h.bottom_lift[1] = i.pair()?;
    h.lift[1] = i.pair()?;
    h.bottom_retract[1] = i.pair()?;
    h.retract[1] = i.pair()?;
    h.bottom_pwm = i.u16()?;
    h.pwm = i.u16()?;
    h.advance_mode = i.u8()? != 0;
    h.print_time_s = i.i32()? as u32;
    h.volume_mm3 = i.f32()?;
    h.weight_g = i.f32()?;
    h.price = i.f32()?;
    h.price_unit = i.text(8)?;
    let _offset = i.i32()?;
    h.grey_scale = i.u8()? != 0;
    h.transition_layers = i.u16()?;
    Ok(h)
}

/// Reads the definition and the raw (still encoded) image data of the next layer.
pub fn read_layer<R: Read>(r: R) -> Result<(LayerDef, Vec<u8>), GooError> {
    let mut i = In { r };
    let mut d = LayerDef {
        pause: i.u16()? != 0,
        pause_z_mm: i.f32()?,
        z_mm: i.f32()?,
        exposure_s: i.f32()?,
        turn_off_time_s: i.f32()?,
        before_lift_s: i.f32()?,
        after_lift_s: i.f32()?,
        after_retract_s: i.f32()?,
        ..Default::default()
    };
    d.lift = [i.pair()?, i.pair()?];
    d.retract = [i.pair()?, i.pair()?];
    d.pwm = i.u16()?;
    i.bytes(2)?;
    let size = i.i32()?;
    if size < 2 {
        return Err(GooError::BadLayer {
            layer: 0,
            reason: format!("data size {size}"),
        });
    }
    let data = i.bytes(size as usize)?;
    i.bytes(2)?;
    Ok((d, data))
}

/// A `.goo` file opened for reading: the header and where every layer starts.
#[derive(Debug)]
pub struct GooFile<R> {
    pub header: Header,
    offsets: Vec<u64>,
    reader: R,
}

impl<R: Read + Seek> GooFile<R> {
    pub fn open(mut reader: R) -> Result<Self, GooError> {
        let header = read_header(&mut reader)?;
        let mut offsets = Vec::with_capacity(header.layer_count as usize);
        let mut pos = HEADER_SIZE;
        for layer in 0..header.layer_count {
            offsets.push(pos);
            reader.seek(SeekFrom::Start(pos + (LAYER_DEF_SIZE as u64)))?;
            let mut b = [0u8; 4];
            reader.read_exact(&mut b)?;
            let size = i32::from_be_bytes(b);
            if size < 2 {
                return Err(GooError::BadLayer {
                    layer,
                    reason: format!("data size {size}"),
                });
            }
            pos += LAYER_DEF_SIZE as u64 + 4 + size as u64 + 2;
        }
        Ok(Self {
            header,
            offsets,
            reader,
        })
    }

    pub fn layer_count(&self) -> u32 {
        self.offsets.len() as u32
    }

    /// Definition and still-encoded image data of layer `i`.
    pub fn raw_layer(&mut self, i: u32) -> Result<(LayerDef, Vec<u8>), GooError> {
        let off = *self.offsets.get(i as usize).ok_or(GooError::BadLayer {
            layer: i,
            reason: "no such layer".into(),
        })?;
        self.reader.seek(SeekFrom::Start(off))?;
        read_layer(&mut self.reader)
    }

    /// Lit area of layer `i` in pixels (sum of values / 255), without building the image.
    /// Also checks that the layer decodes to the right number of pixels.
    pub fn layer_lit(&mut self, i: u32) -> Result<f64, GooError> {
        let (_, data) = self.raw_layer(i)?;
        let mut lit = 0u64;
        let n = decode_runs(&data, |v, n| lit += v as u64 * n as u64)
            .map_err(|reason| GooError::BadLayer { layer: i, reason })?;
        let px = self.header.resolution[0] as usize * self.header.resolution[1] as usize;
        if n != px {
            return Err(GooError::BadLayer {
                layer: i,
                reason: format!("decodes to {n} pixels, expected {px}"),
            });
        }
        Ok(lit as f64 / 255.0)
    }

    /// Definition and decoded image (one grey byte per pixel) of layer `i`.
    pub fn layer(&mut self, i: u32) -> Result<(LayerDef, Vec<u8>), GooError> {
        let off = *self.offsets.get(i as usize).ok_or(GooError::BadLayer {
            layer: i,
            reason: "no such layer".into(),
        })?;
        self.reader.seek(SeekFrom::Start(off))?;
        let (def, data) = read_layer(&mut self.reader)?;
        let px = self.header.resolution[0] as usize * self.header.resolution[1] as usize;
        let image =
            decode_layer(&data, px).map_err(|reason| GooError::BadLayer { layer: i, reason })?;
        Ok((def, image))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn header_is_the_right_size_and_round_trips() {
        let mut h = Header {
            printer_name: "Elegoo Saturn 4 Ultra 16K".into(),
            layer_count: 0,
            resolution: [15120, 6230],
            mirror_y: true,
            size_mm: [211.68, 118.37, 220.0],
            bottom_lift: [(3.0, 35.0), (2.0, 120.0)],
            price_unit: "USD".into(),
            transition_layers: 5,
            ..Header::default()
        };
        h.small_preview.rgb565[5] = 0xF800;
        let mut buf = Vec::new();
        write_header(&mut buf, &h).unwrap();
        assert_eq!(buf.len() as u64, HEADER_SIZE);
        let back = read_header(Cursor::new(&buf)).unwrap();
        assert_eq!(back, h);
    }

    #[test]
    fn runs_encode_and_decode() {
        let w = 1000;
        let mut rows: Vec<Vec<u8>> = Vec::new();
        for r in 0..30 {
            let mut row = vec![0u8; w];
            for (c, p) in row.iter_mut().enumerate() {
                *p = match (r + c) % 97 {
                    0..=40 => 0,
                    41..=43 => (r * 7 + c) as u8,
                    _ => 255,
                };
            }
            rows.push(row);
        }
        let mut enc = LayerEncoder::new();
        enc.zeros(w * 5);
        for row in &rows {
            enc.pixels(row);
        }
        enc.zeros(w * 1000);
        let data = enc.finish();
        let decoded = decode_layer(&data, w * (5 + 30 + 1000)).unwrap();
        assert!(decoded[..w * 5].iter().all(|p| *p == 0));
        for (r, row) in rows.iter().enumerate() {
            assert_eq!(&decoded[w * (5 + r)..w * (6 + r)], row.as_slice());
        }
        let mut bad = data.clone();
        bad[3] ^= 1;
        assert!(decode_layer(&bad, decoded.len()).is_err());
    }

    #[test]
    fn very_long_runs_split() {
        let mut enc = LayerEncoder::new();
        enc.zeros((MAX_RUN * 2 + 5) as usize);
        let data = enc.finish();
        // 0x55, two full chunks of 4 bytes, one short chunk, checksum.
        assert_eq!(data.len(), 1 + 4 + 4 + 1 + 1);
        assert_eq!(&data[1..5], &[0x3F, 0xFF, 0xFF, 0xFF]);
    }

    #[test]
    fn whole_file_round_trip() {
        let h = Header {
            layer_count: 2,
            resolution: [8, 4],
            ..Header::default()
        };
        let mut buf = Vec::new();
        write_header(&mut buf, &h).unwrap();
        for (i, v) in [0u8, 200].into_iter().enumerate() {
            let mut enc = LayerEncoder::new();
            enc.zeros(8);
            enc.pixels(&[v; 8]);
            enc.zeros(16);
            let def = LayerDef {
                z_mm: 0.05 * (i + 1) as f32,
                exposure_s: 2.0,
                pwm: 255,
                ..Default::default()
            };
            write_layer(&mut buf, &def, &enc.finish()).unwrap();
        }
        write_ending(&mut buf).unwrap();
        let mut f = GooFile::open(Cursor::new(buf)).unwrap();
        assert_eq!(f.layer_count(), 2);
        let (def, img) = f.layer(1).unwrap();
        assert_eq!(def.z_mm, 0.1);
        assert_eq!(&img[8..16], &[200; 8]);
        assert_eq!(img.len(), 32);
    }

    #[test]
    fn preview_colors_round_trip_closely() {
        let p = Preview::from_rgb(2, 1, &[255, 128, 0, 10, 20, 30]);
        let rgb = p.to_rgb();
        for (a, b) in rgb.iter().zip([255u8, 128, 0, 10, 20, 30]) {
            assert!((*a as i32 - b as i32).abs() <= 8, "{rgb:?}");
        }
    }
}
