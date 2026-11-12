//! Automatic and manual supports.
//!
//! 1. Contact points, most important first:
//!    - islands, found on test layers: parts of a layer with nothing under them,
//!    - the lowest vertices of the mesh (the very first things that print),
//!    - edges: the free rims of overhangs and downward ridges, at even steps,
//!    - the lowest point of every overhang area,
//!    - a regular grid over faces that point down more steeply than the overhang angle.
//!
//!    Points are spread out with a minimum distance. The lowest 2 mm of a model get denser and
//!    thicker supports, because the first layers pull hardest on them.
//! 2. Each point gets a tip cone that comes out of the surface and points down, then a vertical
//!    body to the plate. When something is in the way, the body bends around it (a knee), and if
//!    that does not work either, it stands on the model below. Close neighbours share one trunk
//!    when parenting is on.
//! 3. Neighbouring supports get cross braces, and the plate feet get a raft.

use copal_mesh::bvh::{Bvh, Hit};
use copal_presets::SupportSettings;
use glam::{Vec2, Vec3};

use crate::islands::find_islands_in;
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

/// Distance between supports for these settings: closer at a higher density.
pub fn spacing_for(s: &SupportSettings) -> f32 {
    BASE_SPACING / s.density.max(0.05).sqrt()
}
/// The lowest part of a model that gets stronger supports.
const HEAVY_ZONE: f32 = 2.0;
/// Supports in the heavy zone stand this much closer.
const HEAVY_DENSER: f32 = 0.75;
/// At most this many supports join one trunk.
const MAX_CHILDREN: u8 = 2;

/// Why a contact point is there. Lower values are placed first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Reason {
    /// A part of a layer with nothing under it.
    Island,
    /// A vertex that is lower than everything around it.
    Minimum,
    /// The free rim of an overhang, or a ridge that points down.
    Edge,
    /// The lowest point of an overhang area.
    AreaLow,
    /// The grid over an overhang.
    Grid,
}

impl Reason {
    /// Islands and minima must be supported; the others may be skipped when they can't be routed.
    fn must(self) -> bool {
        matches!(self, Reason::Island | Reason::Minimum)
    }
}

/// A point on the model that should get a support.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Contact {
    pub point: Vec3,
    pub normal: Vec3,
    pub reason: Reason,
}

/// What the overhang analysis of a model found. Used by the generator and by `overhangs`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Overhangs {
    /// Contact points from the mesh (no islands), unsorted and not spread out yet.
    pub contacts: Vec<Contact>,
    /// Area of all faces that point down more steeply than the overhang angle, mm².
    pub area: f32,
    /// Separate overhang areas.
    pub regions: usize,
    /// Vertices lower than everything around them.
    pub minima: usize,
    /// Length of free rims and downward ridges, mm.
    pub edge_length: f32,
}

/// Vertices closer than this are one vertex.
const WELD: f32 = 1e-3;
/// Faces lower than this lie on the plate and need nothing.
const ON_PLATE: f32 = 0.05;
/// Overhang areas smaller than this get no point of their own (islands still do).
const MIN_AREA: f32 = 0.3;

#[derive(Clone, Copy)]
struct HalfEdge {
    a: u32,
    b: u32,
    face: u32,
    /// The vertex of the face that is not on the edge.
    third: u32,
}

/// Unit direction inside a face, away from the edge `a`–`b` toward the vertex `w`.
fn away_from_edge(a: Vec3, b: Vec3, w: Vec3) -> Option<Vec3> {
    let e = (b - a).try_normalize()?;
    let d = (w - a) - e * (w - a).dot(e);
    d.try_normalize()
}

/// Finds overhangs in triangles given in world space.
///
/// `cos_limit` is the cosine of the overhang angle, `spacing` the distance between supports at
/// this density, `zmin` the lowest point of the model.
pub fn find_overhangs(tris: &[[Vec3; 3]], cos_limit: f32, spacing: f32, zmin: f32) -> Overhangs {
    let mut out = Overhangs::default();
    // Weld the triangle soup, so faces know their neighbours.
    let mut index: std::collections::HashMap<[i32; 3], u32> =
        std::collections::HashMap::with_capacity(tris.len());
    let mut verts: Vec<Vec3> = Vec::new();
    let mut faces: Vec<[u32; 3]> = Vec::with_capacity(tris.len());
    for t in tris {
        let mut f = [0u32; 3];
        for (slot, p) in f.iter_mut().zip(t.iter()) {
            let key = [
                (p.x / WELD).round() as i32,
                (p.y / WELD).round() as i32,
                (p.z / WELD).round() as i32,
            ];
            *slot = *index.entry(key).or_insert_with(|| {
                verts.push(*p);
                (verts.len() - 1) as u32
            });
        }
        faces.push(f);
    }
    drop(index);
    let n = faces.len();
    let point = |i: u32| verts[i as usize];

    // Normals, areas and which faces hang over.
    let mut normal = vec![Vec3::ZERO; n];
    let mut area = vec![0f32; n];
    let mut over = vec![false; n];
    for (i, f) in faces.iter().enumerate() {
        if f[0] == f[1] || f[1] == f[2] || f[0] == f[2] {
            continue;
        }
        let (a, b, c) = (point(f[0]), point(f[1]), point(f[2]));
        let e = (b - a).cross(c - a);
        let len = e.length();
        if len < 1e-9 {
            continue;
        }
        normal[i] = e / len;
        area[i] = len / 2.0;
        over[i] = -normal[i].z > cos_limit && a.z.max(b.z).max(c.z) > ON_PLATE;
        if over[i] {
            out.area += area[i];
        }
    }
    let pitch_at = |z: f32| {
        spacing
            * if z - zmin < HEAVY_ZONE {
                HEAVY_DENSER
            } else {
                1.0
            }
    };

    // Every edge with the faces on it.
    let mut half: Vec<HalfEdge> = Vec::with_capacity(n * 3);
    for (i, f) in faces.iter().enumerate() {
        if area[i] == 0.0 {
            continue;
        }
        for k in 0..3 {
            let (a, b) = (f[k], f[(k + 1) % 3]);
            half.push(HalfEdge {
                a: a.min(b),
                b: a.max(b),
                face: i as u32,
                third: f[(k + 2) % 3],
            });
        }
    }
    half.sort_unstable_by_key(|h| (h.a, h.b));

    let mut not_lowest = vec![false; verts.len()];
    let mut used = vec![false; verts.len()];
    // Overhang areas: faces joined over shared edges (union-find).
    let mut parent: Vec<u32> = (0..n as u32).collect();
    fn find(parent: &mut [u32], x: u32) -> u32 {
        let mut r = x;
        while parent[r as usize] != r {
            parent[r as usize] = parent[parent[r as usize] as usize];
            r = parent[r as usize];
        }
        r
    }
    let sin_limit = (1.0 - cos_limit * cos_limit).max(0.0).sqrt();
    let mut start = 0;
    while start < half.len() {
        let mut end = start + 1;
        while end < half.len() && half[end].a == half[start].a && half[end].b == half[start].b {
            end += 1;
        }
        let group = &half[start..end];
        start = end;
        let (ia, ib) = (group[0].a, group[0].b);
        let (a, b) = (point(ia), point(ib));
        used[ia as usize] = true;
        used[ib as usize] = true;
        if b.z <= a.z + 1e-4 {
            not_lowest[ia as usize] = true;
        }
        if a.z <= b.z + 1e-4 {
            not_lowest[ib as usize] = true;
        }
        // Edges of open or broken meshes (one face, or more than two) are left alone.
        if group.len() != 2 {
            continue;
        }
        let (h0, h1) = (group[0], group[1]);
        let (f0, f1) = (h0.face as usize, h1.face as usize);
        if over[f0] && over[f1] {
            let (ra, rb) = (find(&mut parent, h0.face), find(&mut parent, h1.face));
            if ra != rb {
                parent[ra as usize] = rb;
            }
            continue;
        }
        let length = a.distance(b);
        if length < 1e-6 {
            continue;
        }
        let (Some(d0), Some(d1)) = (
            away_from_edge(a, b, point(h0.third)),
            away_from_edge(a, b, point(h1.third)),
        ) else {
            continue;
        };
        let pitch = pitch_at(a.z.min(b.z));
        let count = (length / pitch).ceil().max(1.0) as u32;
        if over[f0] != over[f1] {
            // The rim of an overhang. It is free unless the face next to it goes down from the
            // edge (a wall below holds it) or the surface just bends on (a smooth transition).
            let (fo, inward, other, w_other) = if over[f0] {
                (f0, d0, f1, point(h1.third))
            } else {
                (f1, d1, f0, point(h0.third))
            };
            if w_other.z < a.z.min(b.z) - 1e-3 || normal[fo].dot(normal[other]) > 0.7 {
                continue;
            }
            out.edge_length += length;
            let inset = inward * (0.25 * pitch).min(0.35);
            for k in 0..count {
                let t = (k as f32 + 0.5) / count as f32;
                out.contacts.push(Contact {
                    point: a + (b - a) * t + inset,
                    normal: normal[fo],
                    reason: Reason::Edge,
                });
            }
        } else {
            // A ridge that points down: both faces rise from the edge, the edge itself is too
            // flat to print on its own, and the faces look down.
            let nn = (normal[f0] + normal[f1]).normalize_or_zero();
            if d0.z > 0.1 && d1.z > 0.1 && (b.z - a.z).abs() / length < sin_limit && nn.z < -0.05 {
                out.edge_length += length;
                for k in 0..count {
                    let t = (k as f32 + 0.5) / count as f32;
                    out.contacts.push(Contact {
                        point: a + (b - a) * t,
                        normal: nn,
                        reason: Reason::Edge,
                    });
                }
            }
        }
    }
    drop(half);

    // The lowest vertices.
    let mut vertex_normal = vec![Vec3::ZERO; verts.len()];
    for (i, f) in faces.iter().enumerate() {
        for v in f {
            vertex_normal[*v as usize] += normal[i] * area[i];
        }
    }
    for (v, p) in verts.iter().enumerate() {
        if !used[v] || not_lowest[v] || p.z <= ON_PLATE {
            continue;
        }
        let nn = vertex_normal[v].normalize_or(Vec3::NEG_Z);
        if nn.z < 0.0 {
            out.minima += 1;
            out.contacts.push(Contact {
                point: *p,
                normal: nn,
                reason: Reason::Minimum,
            });
        }
    }

    // The lowest point of every overhang area.
    let mut regions: std::collections::HashMap<u32, (f32, Vec3, Vec3)> = Default::default();
    for (i, f) in faces.iter().enumerate() {
        if !over[i] {
            continue;
        }
        let root = find(&mut parent, i as u32);
        let entry = regions
            .entry(root)
            .or_insert((0.0, Vec3::splat(f32::INFINITY), Vec3::NEG_Z));
        entry.0 += area[i];
        for v in f {
            let p = point(*v);
            if p.z < entry.1.z {
                entry.1 = p;
                entry.2 = normal[i];
            }
        }
    }
    out.regions = regions.len();
    let mut lows: Vec<(f32, Vec3, Vec3)> = regions.into_values().collect();
    // A fixed order, so the same model always gets the same supports.
    lows.sort_by(|p, q| {
        p.1.z
            .total_cmp(&q.1.z)
            .then(p.1.x.total_cmp(&q.1.x))
            .then(p.1.y.total_cmp(&q.1.y))
    });
    for (region_area, p, nn) in lows {
        if region_area >= MIN_AREA {
            out.contacts.push(Contact {
                point: p,
                normal: nn,
                reason: Reason::AreaLow,
            });
        }
    }

    // A regular grid over the overhangs: rows shifted by half a step, like a honeycomb.
    for (i, f) in faces.iter().enumerate() {
        if !over[i] {
            continue;
        }
        let (a, b, c) = (point(f[0]), point(f[1]), point(f[2]));
        let det = (b.y - c.y) * (a.x - c.x) + (c.x - b.x) * (a.y - c.y);
        if det.abs() < 1e-9 {
            continue;
        }
        let pitch = pitch_at(a.z.min(b.z).min(c.z));
        let row = pitch * 0.866_025_4;
        let lo = a.min(b).min(c);
        let hi = a.max(b).max(c);
        let (j0, j1) = ((lo.y / row).ceil() as i64, (hi.y / row).floor() as i64);
        for j in j0..=j1 {
            let y = j as f32 * row;
            let shift = if j & 1 == 1 { 0.5 * pitch } else { 0.0 };
            let i0 = ((lo.x - shift) / pitch).ceil() as i64;
            let i1 = ((hi.x - shift) / pitch).floor() as i64;
            for ii in i0..=i1 {
                let x = ii as f32 * pitch + shift;
                let u = ((b.y - c.y) * (x - c.x) + (c.x - b.x) * (y - c.y)) / det;
                let v = ((c.y - a.y) * (x - c.x) + (a.x - c.x) * (y - c.y)) / det;
                let w = 1.0 - u - v;
                if u < 0.0 || v < 0.0 || w < 0.0 {
                    continue;
                }
                out.contacts.push(Contact {
                    point: a * u + b * v + c * w,
                    normal: normal[i],
                    reason: Reason::Grid,
                });
            }
        }
    }
    out
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
    /// The wall of the hollow inside the model, if it has one.
    inner: Option<&'a Bvh>,
    s: &'a SupportSettings,
    /// Absolute height where plate supports end (raft top + base cone).
    base_top: f32,
}

impl Router<'_> {
    /// Is anything of the model on the ray between `min_t` and `max_t`?
    fn occluded(&self, origin: Vec3, dir: Vec3, min_t: f32, max_t: f32) -> bool {
        self.bvh.occluded(origin, dir, min_t, max_t)
            || self
                .inner
                .is_some_and(|b| b.occluded(origin, dir, min_t, max_t))
    }

    /// The first thing the ray hits: the model or the wall of its hollow.
    fn raycast(&self, origin: Vec3, dir: Vec3, min_t: f32, max_t: f32) -> Option<Hit> {
        let a = self.bvh.raycast(origin, dir, min_t, max_t);
        let b = self
            .inner
            .and_then(|b| b.raycast(origin, dir, min_t, max_t));
        match (a, b) {
            (Some(a), Some(b)) => Some(if b.t < a.t { b } else { a }),
            (a, b) => a.or(b),
        }
    }

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
            .all(|o| !self.occluded(a + *o * clearance, dir, 1e-4, len))
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

    /// Finds a way from the contact point `p` (surface normal `n`) to the plate or the model
    /// below, for a body of this `diameter` and a tip cone `tip_length` long.
    fn route(&self, p: Vec3, n: Vec3, diameter: f32, tip_length: f32) -> Option<Route> {
        let len = tip_length.max(0.5);
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
            if let Some(hit) = self.raycast(joint, Vec3::NEG_Z, 0.05, joint.z)
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

/// A straight support that stands on the plate. Others may lean over and join it.
struct Trunk {
    xy: Vec2,
    /// Height of its top (absolute).
    top: f32,
    /// Which support it is: an index for the caller.
    index: usize,
    /// Supports that joined it.
    children: u8,
}

impl Router<'_> {
    /// Looks for a trunk the body under `joint` can join: the closest one in reach that is tall
    /// enough, with a free way to it. Returns the trunk and where the branch meets it.
    /// A trunk takes at most `room` supports.
    fn join(
        &self,
        trunks: &[Trunk],
        joint: Vec3,
        dia: f32,
        reach: f32,
        room: u8,
    ) -> Option<(usize, Vec3)> {
        let r = dia / 2.0;
        let here = joint.truncate();
        let mut near: Vec<(f32, usize)> = trunks
            .iter()
            .enumerate()
            .filter_map(|(k, t)| {
                let d = t.xy.distance(here);
                // A little steeper than 45°, so the branch prints cleanly.
                let knee_z = joint.z - d * 1.2;
                (t.children < room
                    && (0.5..=reach).contains(&d)
                    && (self.base_top + 1.0..=t.top).contains(&knee_z))
                .then_some((d, k))
            })
            .collect();
        near.sort_by(|a, b| a.0.total_cmp(&b.0));
        for (d, k) in near.into_iter().take(3) {
            let knee = trunks[k].xy.extend(joint.z - d * 1.2);
            if self.clear(joint, knee, r + self.s.safe_distance_mm * 0.5)
                && self.column_clear(knee, r)
            {
                return Some((k, knee));
            }
        }
        None
    }
}

impl Router<'_> {
    /// The last resort for a point that must be held and has no way down: a short strut to the
    /// nearest part of the model below it, whatever its slope. Tiny islands in tight corners
    /// and inside hollows are held this way.
    fn strut(&self, p: Vec3, n: Vec3, tip_length: f32) -> Option<Route> {
        let len = tip_length.clamp(0.5, 1.2);
        let mut dirs = vec![Vec3::NEG_Z];
        let away = (n.truncate()).normalize_or(Vec2::X);
        for down in [0.5f32, 0.8] {
            let flat = (1.0 - down * down).sqrt();
            for k in 0..12 {
                let a = away.rotate(Vec2::from_angle(k as f32 / 12.0 * std::f32::consts::TAU));
                dirs.push((a * flat).extend(-down));
            }
        }
        let mut best: Option<(f32, Route)> = None;
        for d in dirs {
            // The strut leaves the surface, so it must not start into the model.
            if d.dot(n) < 0.05 {
                continue;
            }
            let joint = p + d * len;
            if !self.clear(p + d * 0.25, joint, 0.0) {
                continue;
            }
            if let Some(hit) = self.raycast(joint, d, 0.05, 25.0)
                && hit.t > 0.6
                && best.as_ref().is_none_or(|(t, _)| hit.t < *t)
            {
                best = Some((
                    hit.t,
                    Route {
                        joint,
                        knee: None,
                        foot: Foot::Model { point: hit.point },
                    },
                ));
            }
        }
        best.map(|(_, r)| r)
    }
}

/// The model supports are made for, in world space.
#[derive(Clone, Copy)]
pub struct Target<'a> {
    /// The outside of the model.
    pub body: &'a Bvh,
    /// The wall of its hollow (inside out), if it is hollow. Islands inside the hollow get
    /// supports that stand on the floor of the hollow.
    pub cavity: Option<&'a Bvh>,
    /// Places to keep free, with their radius: the mouths of drain holes.
    pub avoid: &'a [(Vec3, f32)],
    /// The drain holes as meshes (inside out). What they cut away is gone, so the rim of a hole
    /// at the lowest point of a model is where that model starts to print.
    pub holes: &'a [[Vec3; 3]],
}

impl<'a> Target<'a> {
    pub fn solid(body: &'a Bvh) -> Self {
        Target {
            body,
            cavity: None,
            avoid: &[],
            holes: &[],
        }
    }

    /// Everything that is cut out of the body: the hollow and the drain holes.
    pub fn cuts(&self) -> Vec<[Vec3; 3]> {
        let mut cuts = self
            .cavity
            .map(|c| c.triangles().to_vec())
            .unwrap_or_default();
        cuts.extend_from_slice(self.holes);
        cuts
    }

    /// The first thing a ray hits: the model or the wall of its hollow.
    pub fn raycast(&self, origin: Vec3, dir: Vec3, min_t: f32, max_t: f32) -> Option<Hit> {
        let a = self.body.raycast(origin, dir, min_t, max_t);
        let b = self
            .cavity
            .and_then(|b| b.raycast(origin, dir, min_t, max_t));
        match (a, b) {
            (Some(a), Some(b)) => Some(if b.t < a.t { b } else { a }),
            (a, b) => a.or(b),
        }
    }

    fn router(&self, s: &'a SupportSettings, base_top: f32) -> Router<'a> {
        Router {
            bvh: self.body,
            inner: self.cavity,
            s,
            base_top,
        }
    }

    fn avoids(&self, p: Vec3) -> bool {
        // Only what the hole removes: its rim is solid and often needs a support.
        self.avoid.iter().any(|(c, r)| c.distance(p) < r * 0.9)
    }
}

/// Makes supports for a solid model given in world space (`bvh`) at `position`.
///
/// Manual supports in `keep` stay. The result is relative to `position`.
pub fn generate(
    bvh: &Bvh,
    position: Vec3,
    s: &SupportSettings,
    mode: Mode,
    keep: Option<&SupportSet>,
) -> SupportSet {
    generate_for(&Target::solid(bvh), position, s, mode, keep)
}

/// Makes supports for a model that may be hollow and have drain holes. See [`generate`].
pub fn generate_for(
    target: &Target,
    position: Vec3,
    s: &SupportSettings,
    mode: Mode,
    keep: Option<&SupportSet>,
) -> SupportSet {
    let bvh = target.body;
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
    let router = target.router(s, set.base_top());
    let Some((lo, hi)) = bvh.bounds() else {
        return set;
    };
    let zmin = lo.z;
    let spacing = spacing_for(s);

    // 1. Contact points.
    let mut cands: Vec<Contact> = Vec::new();
    if s.support_islands || mode == Mode::Islands {
        // Big models are tested on a coarser grid, so the search stays quick.
        let size = hi - lo;
        let pixel = (size.x.max(size.y) / 1500.0).max(0.1);
        let layer = (size.z / 3000.0).max(0.05);
        let cuts = target.cuts();
        let probe = |o: Vec3, d: Vec3, t0: f32, t1: f32| router.raycast(o, d, t0, t1);
        for island in find_islands_in(bvh, &cuts, &probe, layer, pixel, spacing * 0.8) {
            for (p, n) in island.contacts {
                cands.push(Contact {
                    point: p,
                    normal: n,
                    reason: Reason::Island,
                });
            }
        }
    }
    if mode == Mode::Auto {
        let cos_limit = s.overhang_angle_deg.to_radians().cos();
        cands.extend(find_overhangs(bvh.triangles(), cos_limit, spacing, zmin).contacts);
    }
    // The most important first, then from the bottom up: low points matter most.
    cands.sort_by(|a, b| {
        a.reason
            .cmp(&b.reason)
            .then(a.point.z.total_cmp(&b.point.z))
            .then(a.point.x.total_cmp(&b.point.x))
            .then(a.point.y.total_cmp(&b.point.y))
    });

    // 2. Spread them out and route them.
    let mut grid = Grid::new(spacing.max(0.5));
    for m in &set.supports {
        grid.add(m.tip + position);
    }
    let mut placed: Vec<Support> = Vec::new();
    let mut trunks: Vec<Trunk> = Vec::new();
    let join_reach = spacing * 0.8;
    for c in cands {
        let heavy = c.point.z - zmin < HEAVY_ZONE;
        let local = spacing * if heavy { HEAVY_DENSER } else { 1.0 };
        let min_dist = match c.reason {
            // Separate islands need their own supports, however close they are.
            Reason::Island => 0.35,
            // Rough surfaces have many tiny low points; one support holds a group of them.
            Reason::Minimum => 0.5 * local,
            Reason::Edge | Reason::AreaLow => 0.7 * local,
            Reason::Grid => 0.6 * local,
        };
        if grid.near(c.point, min_dist) || target.avoids(c.point) {
            continue;
        }
        let (tip_d, dia) = if heavy {
            (s.tip_diameter_mm * 1.15, s.diameter_mm * 1.25)
        } else {
            (s.tip_diameter_mm, s.diameter_mm)
        };
        let mut found = router
            .route(c.point, c.normal, dia, s.tip_length_mm)
            .map(|r| (r, tip_d, dia));
        if found.is_none() && c.reason.must() {
            // A tight spot: try a short, thin support before giving up on an island.
            found = router
                .route(c.point, c.normal, dia * 0.7, s.tip_length_mm * 0.5)
                .map(|r| (r, tip_d * 0.85, dia * 0.7));
        }
        if found.is_none() && c.reason.must() {
            found = router
                .strut(c.point, c.normal, s.tip_length_mm * 0.5)
                .map(|r| (r, tip_d * 0.85, dia * 0.7));
        }
        let Some((mut route, tip_d, mut dia)) = found else {
            continue;
        };
        // Tall supports sway; they get a thicker body, up to half as much again at 13 cm.
        let length = route.knee.unwrap_or(route.joint).z - router.base_top;
        if route.foot == Foot::Plate {
            dia *= 1.0 + ((length - 30.0) / 200.0).clamp(0.0, 0.5);
        }
        // Parenting: a support next to a trunk leans over and joins it instead of going down on
        // its own. Fewer feet, a smaller raft, and trunks that hold each other.
        let own_column = route.knee.is_none() && route.foot == Foot::Plate;
        let mut joined = false;
        if s.auto_parent
            && own_column
            && let Some((k, knee)) =
                router.join(&trunks, route.joint, dia, join_reach, MAX_CHILDREN)
        {
            route.knee = Some(knee);
            trunks[k].children += 1;
            // A trunk that carries more gets thicker.
            let trunk = &mut placed[trunks[k].index];
            trunk.diameter = trunk.diameter.max(dia) * 1.15;
            joined = true;
        }
        if own_column && !joined {
            trunks.push(Trunk {
                xy: route.joint.truncate(),
                top: route.joint.z,
                index: placed.len(),
                children: 0,
            });
        }
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
        brace_all(&mut set, &router, position);
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

/// Adds diagonal braces between the trunks `a` and `b`, where both stand next to each other and
/// the braces miss the model. Returns how many were added.
fn brace_pair(set: &mut SupportSet, router: &Router, position: Vec3, a: u32, b: u32) -> usize {
    let (Some(sa), Some(sb)) = (set.get(a).copied(), set.get(b).copied()) else {
        return 0;
    };
    let (Some((a0, a1)), Some((b0, b1))) = (
        column_span(set, &sa, position),
        column_span(set, &sb, position),
    ) else {
        return 0;
    };
    let pa = sa.column_top().truncate() + position.truncate();
    let pb = sb.column_top().truncate() + position.truncate();
    let dist = pa.distance(pb);
    let (lo, hi) = (a0.max(b0), a1.min(b1));
    if dist < 0.8 || hi - lo < 1.0 {
        return 0;
    }
    // Each brace climbs at 45°; flatter ones (down to about 25°) only where the trunks are short.
    let rise = dist.min(hi - lo);
    if rise < dist * 0.45 {
        return 0;
    }
    // Close trunks get a brace every `bracing_spacing_mm`, far ones an unbroken zigzag.
    let period = router.s.bracing_spacing_mm.max(1.0).max(rise);
    let dia = (0.6 * sa.diameter.min(sb.diameter)).max(0.4);
    let mut added = 0;
    let mut forward = true;
    let mut z = lo;
    while z + rise <= hi + 1e-3 {
        let (from, to) = if forward { (pa, pb) } else { (pb, pa) };
        let p0 = from.extend(z);
        let d = to.extend(z + rise) - p0;
        let len = d.length();
        // The ends sit inside the bodies, so only the middle has to be clear.
        if len > 1.0 && !router.occluded(p0, d / len, 0.5, len - 0.5) {
            let (ia, ib) = if forward { (a, b) } else { (b, a) };
            set.braces.push(Brace {
                a: ia,
                b: ib,
                za: z - position.z,
                zb: z + rise - position.z,
                diameter: dia,
            });
            added += 1;
        }
        forward = !forward;
        z += period;
    }
    added
}

/// The trunks of a set: supports with their own straight body down to the plate, one per place
/// (supports that joined a trunk share its place). Returns (id, place), sorted by id.
fn trunk_nodes(set: &SupportSet) -> Vec<(u32, Vec2)> {
    let mut best: std::collections::HashMap<(i32, i32), (f32, u32, Vec2)> = Default::default();
    for p in set.plate_feet() {
        let c = p.column_top();
        let key = ((c.x * 20.0).round() as i32, (c.y * 20.0).round() as i32);
        // The tallest body at a place stands for it.
        if best.get(&key).is_none_or(|(z, _, _)| c.z > *z) {
            best.insert(key, (c.z, p.id, c.truncate()));
        }
    }
    let mut nodes: Vec<(u32, Vec2)> = best.into_values().map(|(_, id, xy)| (id, xy)).collect();
    nodes.sort_by_key(|n| n.0);
    nodes
}

/// How far apart two trunks may stand and still get braces.
fn brace_reach(s: &SupportSettings) -> f32 {
    (spacing_for(s) * 2.5).clamp(4.0, 10.0)
}

/// Every trunk is tied to this many neighbours, if it has them.
const BRACE_NEIGHBOURS: usize = 3;

/// Connects the trunks into a lattice: every trunk is tied to its nearest neighbours, and a
/// spanning tree over all of them makes sure no group stands alone.
fn brace_all(set: &mut SupportSet, router: &Router, position: Vec3) {
    let max_d = brace_reach(router.s);
    let nodes = trunk_nodes(set);
    let n = nodes.len();
    // Near pairs, found through a grid so big sets stay quick.
    let mut cells: std::collections::HashMap<(i32, i32), Vec<usize>> = Default::default();
    let cell_of = |p: Vec2| ((p.x / max_d).floor() as i32, (p.y / max_d).floor() as i32);
    for (i, (_, p)) in nodes.iter().enumerate() {
        cells.entry(cell_of(*p)).or_default().push(i);
    }
    let mut edges: Vec<(f32, usize, usize)> = Vec::new();
    for (i, (_, p)) in nodes.iter().enumerate() {
        let (cx, cy) = cell_of(*p);
        for dx in -1..=1 {
            for dy in -1..=1 {
                for &j in cells.get(&(cx + dx, cy + dy)).into_iter().flatten() {
                    let d = p.distance(nodes[j].1);
                    if j > i && d <= max_d {
                        edges.push((d, i, j));
                    }
                }
            }
        }
    }
    // Shortest first; a fixed order, so the same model always gets the same braces.
    edges.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
    let mut parent: Vec<usize> = (0..n).collect();
    fn find(p: &mut [usize], x: usize) -> usize {
        let mut r = x;
        while p[r] != r {
            p[r] = p[p[r]];
            r = p[r];
        }
        r
    }
    let mut degree = vec![0usize; n];
    for &(_, i, j) in &edges {
        let (ri, rj) = (find(&mut parent, i), find(&mut parent, j));
        let wanted = degree[i] < BRACE_NEIGHBOURS || degree[j] < BRACE_NEIGHBOURS;
        // Too many ties to one trunk only make the supports hard to remove.
        let room = degree[i] < 2 * BRACE_NEIGHBOURS && degree[j] < 2 * BRACE_NEIGHBOURS;
        if (ri != rj || (wanted && room))
            && brace_pair(set, router, position, nodes[i].0, nodes[j].0) > 0
        {
            parent[ri] = rj;
            degree[i] += 1;
            degree[j] += 1;
        }
    }
}

/// Adds one support by hand at `point` (on the model surface, world space) with the surface
/// `normal`. It becomes part of what is there already: next to a trunk it leans over and joins
/// it (when parenting is on), otherwise it gets its own body and, with `brace`, braces to the
/// trunks around it. Returns its id.
pub fn add_manual(
    set: &mut SupportSet,
    bvh: &Bvh,
    position: Vec3,
    point: Vec3,
    normal: Vec3,
    s: &SupportSettings,
    brace: bool,
) -> Option<u32> {
    add_manual_for(set, &Target::solid(bvh), position, point, normal, s, brace)
}

/// Like [`add_manual`], for a model that may be hollow.
pub fn add_manual_for(
    set: &mut SupportSet,
    target: &Target,
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
    let router = target.router(s, set.base_top());
    let mut route = router.route(point, normal, s.diameter_mm, s.tip_length_mm)?;
    // Join a trunk that stands close by.
    let own_column = route.knee.is_none() && route.foot == Foot::Plate;
    if s.auto_parent && own_column {
        let nodes = trunk_nodes(set);
        let trunks: Vec<Trunk> = nodes
            .iter()
            .enumerate()
            .filter_map(|(index, (id, xy))| {
                let t = set.get(*id)?;
                let c = t.column_top();
                let children = set
                    .plate_feet()
                    .filter(|o| {
                        o.id != *id && o.column_top().truncate().distance(c.truncate()) < 0.05
                    })
                    .count();
                Some(Trunk {
                    xy: *xy + position.truncate(),
                    top: c.z + position.z,
                    index,
                    children: children.min(255) as u8,
                })
            })
            .collect();
        // A support asked for by hand reaches further and may join a trunk that is full for
        // the automatic ones: the user wants it there.
        if let Some((k, knee)) = router.join(
            &trunks,
            route.joint,
            s.diameter_mm,
            spacing_for(s) * 1.6,
            2 * MAX_CHILDREN,
        ) {
            route.knee = Some(knee);
            let id = nodes[trunks[k].index].0;
            if let Some(t) = set.supports.iter_mut().find(|t| t.id == id) {
                t.diameter = t.diameter.max(s.diameter_mm) * 1.15;
            }
        }
    }
    let joined = own_column && route.knee.is_some();
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
    if brace && !joined {
        let me = set
            .get(id)
            .map(|x| x.column_top().truncate())
            .unwrap_or_default();
        let reach = brace_reach(s).max(12.0);
        let mut near: Vec<(f32, u32)> = trunk_nodes(set)
            .into_iter()
            .filter(|(o, _)| *o != id)
            .map(|(o, xy)| (xy.distance(me), o))
            .filter(|(d, _)| *d <= reach)
            .collect();
        near.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut tied = 0;
        for (_, o) in near {
            if tied >= BRACE_NEIGHBOURS {
                break;
            }
            if brace_pair(set, &router, position, id.min(o), id.max(o)) > 0 {
                tied += 1;
            }
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
    fn overhang_analysis_of_the_tilted_cube() {
        let (bvh, _) = tilted_cube(6.0);
        let zmin = bvh.bounds().unwrap().0.z;
        let o = find_overhangs(bvh.triangles(), 45f32.to_radians().cos(), 3.0, zmin);
        // Only the bottom face hangs over: one 20 × 20 mm area with four free rims.
        assert!((o.area - 400.0).abs() < 1.0, "{}", o.area);
        assert_eq!(o.regions, 1);
        assert_eq!(o.minima, 1, "the lowest corner");
        assert!((o.edge_length - 80.0).abs() < 0.5, "{}", o.edge_length);
        let count = |r: Reason| o.contacts.iter().filter(|c| c.reason == r).count();
        assert_eq!(count(Reason::Minimum), 1);
        assert_eq!(count(Reason::AreaLow), 1);
        let (edge, grid) = (count(Reason::Edge), count(Reason::Grid));
        assert!((28..=36).contains(&edge), "{edge} edge points");
        assert!((60..=85).contains(&grid), "{grid} grid points");
        // Every point lies on the bottom face and looks down.
        for c in &o.contacts {
            assert!(c.normal.z < 0.0, "{c:?}");
            let (q, _) = bvh.closest_point(c.point, 1.0).expect("on the model");
            assert!(q.distance(c.point) < 1e-2, "{c:?}");
        }
    }

    #[test]
    fn a_smooth_ball_gets_no_rim_and_a_wall_under_a_shelf_counts() {
        // A ball: the overhang ends where the surface gets steep, with no edge there.
        let mut ball = Mesh::default();
        crate::geometry::sphere(&mut ball, Vec3::new(0.0, 0.0, 16.0), 10.0, 24, 32);
        let bvh = Bvh::from_mesh(&ball, &Mat4::IDENTITY);
        let o = find_overhangs(bvh.triangles(), 45f32.to_radians().cos(), 3.0, 6.0);
        assert_eq!(o.minima, 1);
        assert_eq!(o.regions, 1);
        assert!(o.edge_length < 1.0, "{}", o.edge_length);
        assert!(o.contacts.iter().any(|c| c.reason == Reason::Grid));
    }

    #[test]
    fn close_supports_share_a_trunk_when_parenting_is_on() {
        let mut s = SupportSettings::medium();
        let (bvh, pos) = tilted_cube(8.0 + s.raft_thickness_mm);
        s.auto_parent = false;
        let alone = generate(&bvh, pos, &s, Mode::Auto, None);
        assert!(alone.supports.iter().all(|x| x.knee.is_none()));
        s.auto_parent = true;
        let joined = generate(&bvh, pos, &s, Mode::Auto, None);
        assert_eq!(joined.supports.len(), alone.supports.len());
        let feet = |set: &SupportSet| {
            let mut places: Vec<(i32, i32)> = set
                .plate_feet()
                .map(|x| {
                    let c = x.column_top();
                    ((c.x * 100.0).round() as i32, (c.y * 100.0).round() as i32)
                })
                .collect();
            places.sort();
            places.dedup();
            places.len()
        };
        assert!(
            feet(&joined) < feet(&alone),
            "{} feet with parenting, {} without",
            feet(&joined),
            feet(&alone)
        );
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

    /// Trunks of a set as (id, place), and for each trunk the trunks it is braced to.
    fn lattice(set: &SupportSet) -> (Vec<(u32, Vec2)>, Vec<Vec<usize>>) {
        let nodes = trunk_nodes(set);
        let index = |id: u32| nodes.iter().position(|n| n.0 == id);
        let mut ties = vec![Vec::new(); nodes.len()];
        for b in &set.braces {
            if let (Some(i), Some(j)) = (index(b.a), index(b.b))
                && !ties[i].contains(&j)
            {
                ties[i].push(j);
                ties[j].push(i);
            }
        }
        (nodes, ties)
    }

    #[test]
    fn supports_are_tied_into_one_lattice() {
        let s = SupportSettings::medium();
        // High enough that every trunk has room for braces.
        let (bvh, pos) = tilted_cube(20.0);
        let set = generate(&bvh, pos, &s, Mode::Auto, None);
        let (nodes, ties) = lattice(&set);
        assert!(nodes.len() >= 20, "{} trunks", nodes.len());
        // Every trunk is tied to a neighbour, most of them to several.
        let alone = ties.iter().filter(|t| t.is_empty()).count();
        assert_eq!(alone, 0, "{alone} trunks stand alone");
        let well_tied = ties.iter().filter(|t| t.len() >= 2).count();
        assert!(
            well_tied * 10 >= nodes.len() * 8,
            "only {well_tied} of {} trunks have two ties",
            nodes.len()
        );
        // And all of them hang together: one group, reachable from the first trunk.
        let mut seen = vec![false; nodes.len()];
        let mut stack = vec![0];
        seen[0] = true;
        while let Some(i) = stack.pop() {
            for &j in &ties[i] {
                if !seen[j] {
                    seen[j] = true;
                    stack.push(j);
                }
            }
        }
        assert!(seen.iter().all(|x| *x), "the lattice is in several pieces");
        // No brace goes through the cube, and none is flatter than about 25°.
        for b in &set.braces {
            let (sa, sb) = (set.get(b.a).unwrap(), set.get(b.b).unwrap());
            let p0 = sa.column_top().truncate().extend(b.za) + pos;
            let p1 = sb.column_top().truncate().extend(b.zb) + pos;
            let d = p1 - p0;
            assert!(!bvh.occluded(p0, d.normalize(), 0.5, d.length() - 0.5));
            assert!(d.z.abs() >= d.truncate().length() * 0.44, "{b:?}");
        }
        // The same model, the same lattice.
        assert_eq!(generate(&bvh, pos, &s, Mode::Auto, None), set);
    }

    #[test]
    fn a_manual_support_joins_what_is_there_and_is_kept_on_regenerate() {
        let mut s = SupportSettings::medium();
        let (bvh, pos) = tilted_cube(20.0);
        let mut set = generate(&bvh, pos, &s, Mode::Auto, None);
        // A point on the underside right next to a trunk: the new support becomes its branch.
        let middle = Vec2::new(-5.0, -3.0);
        let trunk = *set
            .plate_feet()
            .filter(|t| t.knee.is_none())
            .min_by(|a, b| {
                let d = |t: &Support| (t.joint + pos).truncate().distance(middle);
                d(a).total_cmp(&d(b))
            })
            .unwrap();
        let beside = trunk.joint + pos + Vec3::new(1.2, 0.8, 0.0);
        let hit = bvh
            .raycast(beside.truncate().extend(0.0), Vec3::Z, 0.0, 100.0)
            .unwrap();
        let before = set.plate_feet().count();
        let id = add_manual(&mut set, &bvh, pos, hit.point, hit.normal, &s, true).unwrap();
        let new = *set.get(id).unwrap();
        assert!(new.manual);
        let knee = new.knee.expect("it leans over to a trunk");
        let joined = set
            .plate_feet()
            .filter(|t| t.id != id && t.knee.is_none())
            .any(|t| t.joint.truncate().distance(knee.truncate()) < 1e-3 && t.joint.z >= knee.z);
        assert!(joined, "the branch ends on a trunk");
        assert_eq!(set.plate_feet().count(), before + 1);

        // With parenting off it gets its own body, tied to the trunks around it with braces.
        s.auto_parent = false;
        let mut set = generate(&bvh, pos, &s, Mode::Auto, None);
        let braces = set.braces.len();
        let hit = bvh
            .raycast(Vec3::new(-5.3, -3.1, 0.0), Vec3::Z, 0.0, 100.0)
            .unwrap();
        let id = add_manual(&mut set, &bvh, pos, hit.point, hit.normal, &s, true).unwrap();
        assert!(set.get(id).unwrap().knee.is_none());
        let mine = set.braces[braces..]
            .iter()
            .filter(|b| b.a == id || b.b == id)
            .count();
        assert!(mine >= 2, "{mine} braces to the new support");
        let tied: std::collections::HashSet<u32> = set.braces[braces..]
            .iter()
            .map(|b| if b.a == id { b.b } else { b.a })
            .collect();
        assert!(tied.len() >= 2, "tied to {} neighbours", tied.len());

        // Alone on a model without supports it still stands, and it stays when the automatic
        // supports are made again.
        let mut lone = SupportSet::default();
        let id = add_manual(&mut lone, &bvh, pos, hit.point, hit.normal, &s, true).unwrap();
        let again = generate(&bvh, pos, &s, Mode::Auto, Some(&lone));
        assert!(
            again
                .supports
                .iter()
                .any(|x| x.manual && x.tip == lone.get(id).unwrap().tip)
        );
    }

    #[test]
    fn an_island_inside_a_hollow_stands_on_the_floor_of_the_hollow() {
        let cube = |lo: Vec3, hi: Vec3| {
            let mut m = Mesh::cuboid(hi - lo);
            for p in &mut m.positions {
                *p += Vec3::new((lo.x + hi.x) / 2.0, (lo.y + hi.y) / 2.0, lo.z);
            }
            m
        };
        let s = SupportSettings::medium();
        let lift = 5.0 + s.raft_thickness_mm;
        let pos = Vec3::new(0.0, 0.0, lift);
        let at = Mat4::from_translation(pos);
        // A 30 mm box with a 24 mm hollow, and a post that hangs from the ceiling of the hollow.
        let body = Bvh::from_mesh(
            &cube(Vec3::new(-15.0, -15.0, 0.0), Vec3::new(15.0, 15.0, 30.0)),
            &at,
        );
        let mut cavity = cube(Vec3::new(-12.0, -12.0, 3.0), Vec3::new(12.0, 12.0, 27.0));
        for t in &mut cavity.triangles {
            t.swap(1, 2);
        }
        cavity.append(&cube(
            Vec3::new(-2.0, -2.0, 15.0),
            Vec3::new(2.0, 2.0, 28.0),
        ));
        let cavity = Bvh::from_mesh(&cavity, &at);
        let target = Target {
            body: &body,
            cavity: Some(&cavity),
            avoid: &[],
            holes: &[],
        };
        let set = generate_for(&target, pos, &s, Mode::Islands, None);
        let inside: Vec<&Support> = set
            .supports
            .iter()
            .filter(|x| (x.tip.z - 15.0).abs() < 0.3 && x.tip.truncate().length() < 3.0)
            .collect();
        assert!(
            !inside.is_empty(),
            "the post got no support: {:?}",
            set.supports
        );
        for x in inside {
            let Foot::Model { point } = x.foot else {
                panic!("a support inside the hollow went to the plate: {x:?}");
            };
            assert!((point.z - 3.0).abs() < 0.05, "{point}");
        }
        // The same box seen as solid has only its bottom to hold.
        let solid = generate(&body, pos, &s, Mode::Islands, None);
        assert!(solid.supports.iter().all(|x| x.tip.z < 0.5));
    }

    #[test]
    fn supports_keep_away_from_drain_holes() {
        let s = SupportSettings::medium();
        let (bvh, pos) = tilted_cube(8.0);
        let free = generate(&bvh, pos, &s, Mode::Auto, None);
        // A hole right where a support stands.
        let spot = free.supports[free.supports.len() / 2].tip + pos;
        let avoid = [(spot, 1.5)];
        let target = Target {
            body: &bvh,
            cavity: None,
            avoid: &avoid,
            holes: &[],
        };
        let set = generate_for(&target, pos, &s, Mode::Auto, None);
        assert!(
            set.supports
                .iter()
                .all(|x| (x.tip + pos).distance(spot) >= 1.35)
        );
        assert!(set.supports.len() + 6 >= free.supports.len());
    }
}
