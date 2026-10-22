//! Times the export of a big sphere: cargo run --release -p copal-formats --example bench_export -- [segments] [aa]

use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use copal_core::{Document, Session};
use copal_mesh::Mesh;
use glam::Vec3;

fn sphere(r: f32, seg: u32) -> Mesh {
    let mut m = Mesh::default();
    for i in 0..=seg {
        let t = std::f32::consts::PI * i as f32 / seg as f32;
        for j in 0..seg * 2 {
            let p = std::f32::consts::PI * j as f32 / seg as f32;
            m.positions.push(Vec3::new(
                r * t.sin() * p.cos(),
                r * t.sin() * p.sin(),
                r - r * t.cos(),
            ));
        }
    }
    let w = seg * 2;
    for i in 0..seg {
        for j in 0..w {
            let a = i * w + j;
            let b = i * w + (j + 1) % w;
            let c = a + w;
            let d = b + w;
            m.triangles.push([a, d, b]);
            m.triangles.push([a, c, d]);
        }
    }
    m
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let seg: u32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(700);
    let aa: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(8);
    let mut s = Session::new(Document::new("bench"));
    s.run_script(&format!(
        "project.set_printer printer=elegoo/saturn-4-ultra-16k\nprint.set layer_height_mm=0.03 anti_aliasing={aa}"
    ))
    .unwrap();
    let mesh = sphere(30.0, seg);
    println!("triangles: {}", mesh.triangles.len());
    let path = std::env::temp_dir().join("copal-bench.stl");
    std::fs::write(&path, copal_mesh::import::write_stl(&mesh)).unwrap();
    s.run_line(&format!("object.import path={}", path.display()))
        .unwrap();
    let _ = Arc::new(());
    let t = std::time::Instant::now();
    let out = std::env::temp_dir().join("copal-bench.goo");
    let r = copal_formats::export(&s.doc, &out, &|_| {}, &AtomicBool::new(false)).unwrap();
    println!(
        "{} layers, {:.1} ml, file {:.1} MB, {:.1} s",
        r.layers,
        r.volume_ml,
        r.file_bytes as f64 / 1e6,
        t.elapsed().as_secs_f64()
    );
    println!(
        "expected volume {:.1} ml",
        4.0 / 3.0 * std::f64::consts::PI * 27.0
    );
}
