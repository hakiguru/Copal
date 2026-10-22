//! Reads a .goo file, decodes every layer, re-encodes it and checks the result is identical.
//! Writes the previews and one layer as PNG next to the given output folder.
//!
//! cargo run --release -p copal-formats --example goo_check -- file.goo out_dir [layer]

use std::fs::File;
use std::io::BufReader;

use copal_formats::goo::{GooFile, LayerEncoder, decode_layer};
use copal_slice::RowSink;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path = &args[1];
    let out = std::path::Path::new(&args[2]);
    let show: u32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(0);
    std::fs::create_dir_all(out).unwrap();
    let mut f = GooFile::open(BufReader::new(File::open(path).unwrap())).unwrap();
    let h = f.header.clone();
    println!("{:#?}", Header(&h));
    for (name, p) in [("small", &h.small_preview), ("big", &h.big_preview)] {
        image::save_buffer(
            out.join(format!("preview_{name}.png")),
            &p.to_rgb(),
            p.width,
            p.height,
            image::ColorType::Rgb8,
        )
        .unwrap();
    }
    let (w, ht) = (h.resolution[0] as usize, h.resolution[1] as usize);
    let mut lit_total = 0u64;
    let mut grey_total = 0u64;
    for i in 0..f.layer_count() {
        let (def, img) = f.layer(i).unwrap();
        lit_total += img.iter().map(|p| *p as u64).sum::<u64>();
        grey_total += img.iter().filter(|p| **p != 0 && **p != 255).count() as u64;
        if i < 7 || i == show {
            println!("layer {i}: {def:?}");
        }
        let mut enc = LayerEncoder::new();
        for row in img.chunks(w) {
            enc.pixels(row);
        }
        let again = decode_layer(&enc.finish(), w * ht).unwrap();
        assert!(again == img, "layer {i} re-encodes differently");
        if i == show {
            // Downscale 8× for viewing.
            let (sw, sh) = (w / 8, ht / 8);
            let mut small = vec![0u8; sw * sh];
            for y in 0..sh {
                for x in 0..sw {
                    small[y * sw + x] = img[(y * 8) * w + x * 8];
                }
            }
            image::save_buffer(
                out.join(format!("layer_{i}.png")),
                &small,
                sw as u32,
                sh as u32,
                image::ColorType::L8,
            )
            .unwrap();
            let grey: Vec<u8> = img
                .iter()
                .copied()
                .filter(|p| *p != 0 && *p != 255)
                .take(20)
                .collect();
            println!("some grey values: {grey:?}");
        }
    }
    let px = h.size_mm[0] as f64 / w as f64 * h.size_mm[1] as f64 / ht as f64;
    println!("grey (anti-aliased) pixels in all layers: {grey_total}");
    println!(
        "all layers decode and re-encode. volume from pixels: {:.1} mm3 (header says {})",
        lit_total as f64 / 255.0 * px * h.layer_height_mm as f64,
        h.volume_mm3
    );
}

struct Header<'a>(&'a copal_formats::goo::Header);
impl std::fmt::Debug for Header<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let h = self.0;
        f.debug_struct("Header")
            .field(
                "software",
                &(&h.software_info, &h.software_version, &h.file_time),
            )
            .field("printer", &(&h.printer_name, &h.profile_name))
            .field(
                "aa/grey/blur",
                &(h.anti_aliasing_level, h.grey_level, h.blur_level),
            )
            .field("layers", &h.layer_count)
            .field("res", &h.resolution)
            .field("mirror", &(h.mirror_x, h.mirror_y))
            .field("size", &h.size_mm)
            .field(
                "lift",
                &(h.bottom_lift, h.lift, h.bottom_retract, h.retract),
            )
            .field(
                "time/vol/weight",
                &(h.print_time_s, h.volume_mm3, h.weight_g),
            )
            .finish()
    }
}
