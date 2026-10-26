//! Supports a model and reports what happened (for tuning).
//!
//! cargo run --release -p copal-supports --example support_report -- model.obj [scale] [rx] [ry]

use copal_mesh::bvh::Bvh;
use copal_presets::SupportSettings;
use copal_supports::{Mode, find_islands, generate};
use glam::{EulerRot, Mat4, Quat, Vec3};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mesh = copal_mesh::import::load(std::path::Path::new(&args[1])).unwrap();
    let scale: f32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(1.0);
    let rx: f32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let ry: f32 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let rot = Quat::from_euler(EulerRot::XYZ, rx.to_radians(), ry.to_radians(), 0.0);
    let m = Mat4::from_scale_rotation_translation(Vec3::splat(scale), rot, Vec3::ZERO);
    let b = Bvh::from_mesh(&mesh, &m);
    let (lo, hi) = b.bounds().unwrap();
    let s = SupportSettings::medium();
    let pos = Vec3::new(-(lo.x + hi.x) / 2.0, -(lo.y + hi.y) / 2.0, 6.0 - lo.z);
    let bvh = Bvh::from_mesh(&mesh, &(Mat4::from_translation(pos) * m));
    println!("size {:?}, {} triangles", hi - lo, mesh.triangles.len());
    let t = std::time::Instant::now();
    let islands = find_islands(&bvh, 0.05, 0.1, 2.4);
    println!(
        "{} islands in {:.2} s",
        islands.len(),
        t.elapsed().as_secs_f32()
    );
    let t = std::time::Instant::now();
    let set = generate(&bvh, pos, &s, Mode::Auto, None);
    println!(
        "{} supports ({} knees, {} on the model), {} braces in {:.2} s",
        set.supports.len(),
        set.supports.iter().filter(|x| x.knee.is_some()).count(),
        set.supports
            .iter()
            .filter(|x| matches!(x.foot, copal_supports::Foot::Model { .. }))
            .count(),
        set.braces.len(),
        t.elapsed().as_secs_f32()
    );
    let tips: Vec<Vec3> = set.supports.iter().map(|x| x.tip + pos).collect();
    let mut missing = 0;
    for i in &islands {
        let held = tips.iter().any(|t| {
            t.x >= i.min.x - 0.6
                && t.x <= i.max.x + 0.6
                && t.y >= i.min.y - 0.6
                && t.y <= i.max.y + 0.6
                && t.z >= i.z - 0.4
                && t.z <= i.z + 1.5
        });
        if !held {
            missing += 1;
            let nearest = tips
                .iter()
                .map(|t| t.distance(i.contacts[0].0))
                .fold(f32::INFINITY, f32::min);
            println!(
                "  unsupported island z={:.2} area={:.2} contact {} normal {} nearest tip {:.2} mm",
                i.z, i.area, i.contacts[0].0, i.contacts[0].1, nearest
            );
        }
    }
    println!("{missing} of {} islands without a support", islands.len());
}
