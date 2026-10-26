//! Automatic and manual supports.
//!
//! 1. Contact points: every island (found on test layers) and faces that point down more steeply
//!    than the overhang angle, spread out with a minimum distance. The lowest 2 mm of a model get
//!    denser and thicker supports, because the first layers pull hardest on them.
//! 2. Each point gets a tip cone that comes out of the surface and points down, then a vertical
//!    body to the plate. When something is in the way, the body bends around it (a knee), and if
//!    that does not work either, it stands on the model below.
//! 3. Neighbouring supports get cross braces, and the plate feet get a raft.

use copal_mesh::bvh::Bvh;
use copal_presets::SupportSettings;
use glam::{Vec2, Vec3};

use crate::islands::find_islands;
use crate::model::{Brace, Foot, Support, SupportSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    /// Islands and overhangs.
    #[default]
    Auto,
    /// Only islands (the minimum to print at all).
    Islands,
}

impl Mode {
    pub fn parse(s: &str) -> Option<Mode> {
        match s {
            "auto" | "all" => Some(Mode::Auto),
            "islands" | "island" => Some(Mode::Islands),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Mode::Auto => "auto",
            Mode::Islands => "islands",
        }
    }
}

/// Distance between supports at density 1.
const BASE_SPACING: f32 = 3.0;
/// The lowest part of a model that gets stronger supports.
const HEAVY_ZONE: f32 = 2.0;

/// Deterministic pseudo-random numbers in 0..1 (no global state, same result every run).
fn hash01(a: u32, b: u32) -> f32 {
    let mut x = a.wrapping_mul(0x9E37_79B9) ^ b.wrapping_mul(0x85EB_CA6B);
    x ^= x >> 16;
    x = x.wrapping_mul(0x7FEB_352D);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846C_A68B);
    x ^= x >> 16;
    (x >> 8) as f32 / (1u32 << 24) as f32
}

struct Candidate {
    point: Vec3,
    normal: Vec3,
    /// Islands must be supported; overhang points may be skipped when they can't be routed.
    must: bool,
}

/// Spatial hash for "is there an accepted point within r".
struct Grid {
    cell: f32,
    map: std::collections::HashMap<(i32, i32, i32), Vec<Vec3>>,
}

impl Grid {
    fn new(cell: f32) -> Self {
        Self {
            cell,
            map: Default::default(),
        }
    }
    fn key(&self, p: Vec3) -> (i32, i32, i32) {
        (
            (p.x / self.cell).floor() as i32,
            (p.y / self.cell).floor() as i32,
            (p.z / self.cell).floor() as i32,
        )
    }
    fn near(&self, p: Vec3, r: f32) -> bool {
        let k = self.key(p);
        let n = (r / self.cell).ceil() as i32;
        for dx in -n..=n {
            for dy in -n..=n {
                for dz in -n..=n {
                    if let Some(v) = self.map.get(&(k.0 + dx, k.1 + dy, k.2 + dz))
                        && v.iter().any(|q| q.distance_squared(p) < r * r)
                    {
                        return true;
                    }
                }
            }
        }
        false
    }
    fn add(&mut self, p: Vec3) {
        let k = self.key(p);
        self.map.entry(k).or_default().push(p);
    }
}

/// Where one support goes: tip, joint, optional knee and foot (absolute coordinates).
struct Route {
    joint: Vec3,
    knee: Option<Vec3>,
    foot: Foot,
}

struct Router<'a> {
    bvh: &'a Bvh,
    s: &'a SupportSettings,
    /// Absolute height where plate supports end (raft top + base cone).
    base_top: f32,
}

impl Router<'_> {
    /// Is the straight segment from `a` to `b` free, with `clearance` around it?
    fn clear(&self, a: Vec3, b: Vec3, clearance: f32) -> bool {
        let d = b - a;
        let len = d.length();
        if len < 1e-4 {
            return true;
        }
        let dir = d / len;
        let (u, v) = {
            let u = if dir.x.abs() < 0.9 { Vec3::X } else { Vec3::Y };
            let u = (u - dir * u.dot(dir)).normalize();
            (u, dir.cross(u))
        };
        let offsets = [Vec3::ZERO, u, -u, v, -v];
        offsets
            .iter()
            .all(|o| !self.bvh.occluded(a + *o * clearance, dir, 1e-4, len))
    }

    /// Can a vertical body stand from `top` down to the plate?
    fn column_clear(&self, top: Vec3, radius: f32) -> bool {
        top.z > self.base_top + 0.3
            && self.clear(
                top,
                Vec3::new(top.x, top.y, self.base_top),
                radius + self.s.safe_distance_mm,
            )
    }

    fn route(&self, p: Vec3, n: Vec3, diameter: f32) -> Option<Route> {
        let len = self.s.tip_length_mm.max(0.5);
        let r = diameter / 2.0;
        // Tip directions: along the normal but pointing down, then some alternatives.
        let mut base = (n + Vec3::NEG_Z).normalize_or(Vec3::NEG_Z);
        if base.z > -0.5 {
            let h = base.truncate().normalize_or_zero() * (1.0f32 - 0.25).sqrt();
            base = h.extend(-0.5);
        }
        let mut dirs = vec![base, Vec3::NEG_Z];
        let hz = base.truncate();
        for angle in [35f32, -35.0, 70.0, -70.0] {
            let rot = Vec2::from_angle(angle.to_radians()).rotate(hz);
            dirs.push(rot.extend(base.z).normalize());
        }
        for d in dirs {
            let joint = p + d * len;
            if joint.z < self.base_top + 0.5 {
                continue;
            }
            // The tip must not run through the model.
            let start = p + d * 0.25;
            if !self.clear(start, joint, 0.0) {
                continue;
            }
            if self.column_clear(joint, r) {
                return Some(Route {
                    joint,
                    knee: None,
                    foot: Foot::Plate,
                });
            }
            // Bend around what is below: 45° away, up to 12 mm.
            let away = d.truncate().normalize_or(Vec2::X);
            for radius in [1.5f32, 3.0, 4.5, 6.0, 8.0, 10.0, 12.0] {
                for k in 0..12 {
                    let a = away.rotate(Vec2::from_angle(k as f32 / 12.0 * std::f32::consts::TAU));
                    let knee = joint + (a * radius).extend(-radius);
                    if knee.z < self.base_top + 0.5 {
                        continue;
                    }
                    if self.clear(joint, knee, r + self.s.safe_distance_mm * 0.5)
                        && self.column_clear(knee, r)
                    {
                        return Some(Route {
                            joint,
                            knee: Some(knee),
                            foot: Foot::Plate,
                        });
                    }
                }
            }
            // Stand on the model below.
            if let Some(hit) = self.bvh.raycast(joint, Vec3::NEG_Z, 0.05, joint.z)
                && hit.normal.z > 0.3
                && joint.z - hit.point.z > 1.5
            {
                return Some(Route {
                    joint,
                    knee: None,
                    foot: Foot::Model { point: hit.point },
                });
            }
        }
        None
    }
}

/// Makes supports for a model given in world space (`bvh`) at `position`.
///
/// Manual supports in `keep` stay. The result is relative to `position`.
pub fn generate(
    bvh: &Bvh,
    position: Vec3,
    s: &SupportSettings,
    mode: Mode,
    keep: Option<&SupportSet>,
) -> SupportSet {
    let mut set = SupportSet {
        raft_thickness: s.raft_thickness_mm,
        base_diameter: s.base_diameter_mm,
        ..SupportSet::default()
    };
    if let Some(k) = keep {
        for m in k.supports.iter().filter(|m| m.manual) {
            set.push(*m);
        }
    }
    if s.raft {
        // A placeholder outline so base_top() counts the raft while routing.
        set.raft = vec![Vec2::ZERO; 3];
    }
    let router = Router {
        bvh,
        s,
        base_top: set.base_top(),
    };
    let Some((lo, _)) = bvh.bounds() else {
        return set;
    };
    let zmin = lo.z;
    let spacing = BASE_SPACING / s.density.max(0.05).sqrt();

    // 1. Contact points.
    let mut cands: Vec<Candidate> = Vec::new();
    if s.support_islands || mode == Mode::Islands {
        for island in find_islands(bvh, 0.05, 0.1, spacing * 0.8) {
            for (p, n) in island.contacts {
                cands.push(Candidate {
                    point: p,
                    normal: n,
                    must: true,
                });
            }
        }
    }
    if mode == Mode::Auto {
        let cos_limit = s.overhang_angle_deg.to_radians().cos();
        for (i, t) in bvh.triangles().iter().enumerate() {
            let e = (t[1] - t[0]).cross(t[2] - t[0]);
            let area = e.length() / 2.0;
            if area < 1e-9 {
                continue;
            }
            let n = e / (2.0 * area);
            if -n.z <= cos_limit {
                continue;
            }
            let local = spacing
                * if t[0].z - zmin < HEAVY_ZONE {
                    0.75
                } else {
                    1.0
                };
            let expected = area / (local * local) * 4.0;
            let count = expected.floor() as u32 + u32::from(hash01(i as u32, 7) < expected.fract());
            for k in 0..count {
                let (mut a, mut b) = (hash01(i as u32, 2 * k + 11), hash01(i as u32, 2 * k + 12));
                if a + b > 1.0 {
                    (a, b) = (1.0 - a, 1.0 - b);
                }
                let p = t[0] + (t[1] - t[0]) * a + (t[2] - t[0]) * b;
                cands.push(Candidate {
                    point: p,
                    normal: n,
                    must: false,
                });
            }
        }
    }
    // Islands first, then from the bottom up: low points matter most.
    cands.sort_by(|a, b| b.must.cmp(&a.must).then(a.point.z.total_cmp(&b.point.z)));

    // 2. Spread them out and route them.
    let mut grid = Grid::new(spacing.max(0.5));
    for m in &set.supports {
        grid.add(m.tip + position);
    }
    let mut placed: Vec<Support> = Vec::new();
    for c in cands {
        let heavy = c.point.z - zmin < HEAVY_ZONE;
        // Faces just past the angle need fewer supports than flat ones.
        let flat = (-c.normal.z).clamp(0.0, 1.0);
        let local = spacing * if heavy { 0.75 } else { 1.0 } * (1.0 + 0.5 * (1.0 - flat));
        // Separate islands need their own supports, however close they are.
        let min_dist = if c.must { 0.35 } else { local };
        if grid.near(c.point, min_dist) {
            continue;
        }
        let (tip_d, dia) = if heavy {
            (s.tip_diameter_mm * 1.15, s.diameter_mm * 1.25)
        } else {
            (s.tip_diameter_mm, s.diameter_mm)
        };
        let Some(route) = router.route(c.point, c.normal, dia) else {
            continue;
        };
        grid.add(c.point);
        placed.push(Support {
            id: 0,
            tip: c.point - position,
            normal: c.normal,
            joint: route.joint - position,
            knee: route.knee.map(|k| k - position),
            foot: match route.foot {
                Foot::Plate => Foot::Plate,
                Foot::Model { point } => Foot::Model {
                    point: point - position,
                },
            },
            tip_diameter: tip_d,
            diameter: dia,
            manual: false,
        });
    }
    for p in placed {
        set.push(p);
    }

    // 3. Braces and raft.
    if s.auto_bracing {
        brace_all(&mut set, bvh, position, s, spacing);
    }
    set.rebuild_raft(s.raft, s.raft_margin_mm);
    set
}

/// Vertical span of a plate support's body, absolute heights.
fn column_span(set: &SupportSet, s: &Support, position: Vec3) -> Option<(f32, f32)> {
    (s.foot == Foot::Plate).then(|| {
        (
            set.base_top() + 0.6,
            s.column_top().z + position.z - s.diameter * 0.6,
        )
    })
}

/// Adds X braces between `a` and `b` if they are close and the braces miss the model.
fn brace_pair(
    set: &mut SupportSet,
    bvh: &Bvh,
    position: Vec3,
    a: u32,
    b: u32,
    spacing: f32,
) -> bool {
    let (Some(sa), Some(sb)) = (set.get(a).copied(), set.get(b).copied()) else {
        return false;
    };
    let (Some((a0, a1)), Some((b0, b1))) = (
        column_span(set, &sa, position),
        column_span(set, &sb, position),
    ) else {
        return false;
    };
    let pa = sa.column_top().truncate() + position.truncate();
    let pb = sb.column_top().truncate() + position.truncate();
    let dist = pa.distance(pb);
    let (lo, hi) = (a0.max(b0), a1.min(b1));
    let step = spacing.max(dist * 0.9);
    if hi - lo < step * 0.6 || dist < 0.8 {
        return false;
    }
    let dia = 0.6 * sa.diameter.min(sb.diameter);
    let mut z = lo;
    let mut added = false;
    let mut forward = true;
    // A zigzag: one diagonal per step, alternating direction.
    while z < hi - 0.1 {
        let z2 = (z + step).min(hi);
        let (from, to) = if forward { (pa, pb) } else { (pb, pa) };
        let p0 = from.extend(z);
        let p1 = to.extend(z2);
        let d = p1 - p0;
        let len = d.length();
        // The ends sit inside the bodies, so only the middle has to be clear.
        if !bvh.occluded(p0, d / len, 0.5, len - 0.5) {
            let (ia, ib) = if forward { (a, b) } else { (b, a) };
            set.braces.push(Brace {
                a: ia,
                b: ib,
                za: z - position.z,
                zb: z2 - position.z,
                diameter: dia,
            });
            added = true;
        }
        forward = !forward;
        if z2 >= hi {
            break;
        }
        z = z2;
    }
    added
}

/// Connects plate supports into a light net: a minimum spanning tree over nearby supports plus
/// each support's nearest neighbour, so every support is braced without a forest of braces.
fn brace_all(set: &mut SupportSet, bvh: &Bvh, position: Vec3, s: &SupportSettings, spacing: f32) {
    let max_d = (spacing * 2.5).clamp(4.0, 10.0);
    let step = s.bracing_spacing_mm.max(1.0);
    let nodes: Vec<(u32, Vec2)> = set
        .plate_feet()
        .map(|p| (p.id, p.column_top().truncate()))
        .collect();
    let n = nodes.len();
    // Candidate edges between near supports, shortest first (Kruskal).
    let mut edges: Vec<(f32, usize, usize)> = Vec::new();
    for i in 0..n {
        for j in i + 1..n {
            let d = nodes[i].1.distance(nodes[j].1);
            if d <= max_d {
                edges.push((d, i, j));
            }
        }
    }
    edges.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut parent: Vec<usize> = (0..n).collect();
    fn find(p: &mut [usize], x: usize) -> usize {
        let mut r = x;
        while p[r] != r {
            p[r] = p[p[r]];
            r = p[r];
        }
        r
    }
    let mut chosen = std::collections::HashSet::new();
    for &(_, i, j) in &edges {
        let (ri, rj) = (find(&mut parent, i), find(&mut parent, j));
        if ri != rj {
            parent[ri] = rj;
            chosen.insert((i, j));
        }
    }
    // Plus the nearest neighbour of each support, for some triangles in the net.
    for i in 0..n {
        if let Some(&(_, a, b)) = edges.iter().find(|(_, a, b)| *a == i || *b == i) {
            chosen.insert((a, b));
        }
    }
    let mut chosen: Vec<(usize, usize)> = chosen.into_iter().collect();
    chosen.sort();
    for (i, j) in chosen {
        brace_pair(set, bvh, position, nodes[i].0, nodes[j].0, step);
    }
}

/// Adds one support by hand at `point` (on the model surface, world space) with the surface
/// `normal`. With `brace`, it gets braces to its two nearest neighbours. Returns its id.
pub fn add_manual(
    set: &mut SupportSet,
    bvh: &Bvh,
    position: Vec3,
    point: Vec3,
    normal: Vec3,
    s: &SupportSettings,
    brace: bool,
) -> Option<u32> {
    if set.is_empty() {
        set.raft_thickness = s.raft_thickness_mm;
        set.base_diameter = s.base_diameter_mm;
    }
    let raft_wanted = s.raft;
    if raft_wanted && set.raft.is_empty() {
        set.raft = vec![Vec2::ZERO; 3];
    }
    let router = Router {
        bvh,
        s,
        base_top: set.base_top(),
    };
    let route = router.route(point, normal, s.diameter_mm)?;
    let id = set.push(Support {
        id: 0,
        tip: point - position,
        normal,
        joint: route.joint - position,
        knee: route.knee.map(|k| k - position),
        foot: match route.foot {
            Foot::Plate => Foot::Plate,
            Foot::Model { point } => Foot::Model {
                point: point - position,
            },
        },
        tip_diameter: s.tip_diameter_mm,
        diameter: s.diameter_mm,
        manual: true,
    });
    if brace {
        let me = set
            .get(id)
            .map(|x| x.column_top().truncate())
            .unwrap_or_default();
        let mut near: Vec<(f32, u32)> = set
            .plate_feet()
            .filter(|o| o.id != id)
            .map(|o| (o.column_top().truncate().distance(me), o.id))
            .filter(|(d, _)| *d <= 12.0)
            .collect();
        near.sort_by(|a, b| a.0.total_cmp(&b.0));
        for (_, o) in near.into_iter().take(2) {
            brace_pair(
                set,
                bvh,
                position,
                id.min(o),
                id.max(o),
                s.bracing_spacing_mm.max(1.0),
            );
        }
    }
    set.rebuild_raft(raft_wanted, s.raft_margin_mm);
    Some(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::build_mesh;
    use copal_mesh::Mesh;
    use glam::{EulerRot, Mat4, Quat};

    fn tilted_cube(lift: f32) -> (Bvh, Vec3) {
        let cube = Mesh::cuboid(Vec3::splat(20.0));
        let rot = Quat::from_euler(EulerRot::XYZ, 33f32.to_radians(), -16f32.to_radians(), 0.0);
        let m = Mat4::from_quat(rot);
        let b = Bvh::from_mesh(&cube, &m);
        let lo = b.bounds().unwrap().0;
        let position = Vec3::new(0.0, 0.0, lift - lo.z);
        (
            Bvh::from_mesh(&cube, &(Mat4::from_translation(position) * m)),
            position,
        )
    }

    #[test]
    fn tilted_cube_is_held_from_below() {
        let s = SupportSettings::medium();
        let (bvh, pos) = tilted_cube(5.0 + s.raft_thickness_mm);
        let set = generate(&bvh, pos, &s, Mode::Auto, None);
        eprintln!(
            "cube: {} supports, {} with knees, {} on the model, {} braces",
            set.supports.len(),
            set.supports.iter().filter(|x| x.knee.is_some()).count(),
            set.supports
                .iter()
                .filter(|x| matches!(x.foot, Foot::Model { .. }))
                .count(),
            set.braces.len()
        );
        assert!(
            set.supports.len() >= 25,
            "only {} supports",
            set.supports.len()
        );
        // The lowest corner is supported.
        let lowest = set
            .supports
            .iter()
            .map(|x| x.tip.z + pos.z)
            .fold(f32::INFINITY, f32::min);
        assert!(
            lowest < 5.0 + s.raft_thickness_mm + 0.3,
            "lowest tip at {lowest}"
        );
        // Every plate support stands on the raft and no body goes through the cube.
        for sp in set.plate_feet() {
            let top = sp.column_top() + pos;
            assert!(
                !bvh.occluded(top, Vec3::NEG_Z, 0.01, top.z - set.base_top()),
                "support {} hits the model",
                sp.id
            );
        }
        assert!(!set.braces.is_empty());
        assert!(set.raft.len() >= 3);
        let mesh = build_mesh(&set, pos);
        assert!(mesh.mesh.volume() > 0.0);
        assert_eq!(mesh.ranges.len(), set.supports.len());
    }

    #[test]
    fn islands_mode_supports_only_islands() {
        let s = SupportSettings::medium();
        let (bvh, pos) = tilted_cube(6.0);
        let auto = generate(&bvh, pos, &s, Mode::Auto, None);
        let islands = generate(&bvh, pos, &s, Mode::Islands, None);
        assert!(!islands.supports.is_empty());
        assert!(islands.supports.len() < auto.supports.len());
    }

    #[test]
    fn manual_support_braces_to_neighbours_and_kept_on_regenerate() {
        let s = SupportSettings::medium();
        let (bvh, pos) = tilted_cube(15.0);
        let mut set = generate(&bvh, pos, &s, Mode::Islands, None);
        let braces = set.braces.len();
        // A point on the underside: cast up from below the middle.
        let hit = bvh
            .raycast(Vec3::new(-5.0, -3.0, 0.0), Vec3::Z, 0.0, 100.0)
            .unwrap();
        let id = add_manual(&mut set, &bvh, pos, hit.point, hit.normal, &s, true).unwrap();
        assert!(set.get(id).unwrap().manual);
        assert!(set.braces.len() > braces, "the new support got braces");
        let again = generate(&bvh, pos, &s, Mode::Auto, Some(&set));
        assert!(
            again
                .supports
                .iter()
                .any(|x| x.manual && x.tip == set.get(id).unwrap().tip)
        );
    }
}
