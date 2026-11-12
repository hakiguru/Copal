//! What a set of supports is made of.

use copal_presets::BaseKind;
use glam::{Vec2, Vec3};
use serde::{Deserialize, Serialize};

/// Where a support ends at the bottom.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "on", rename_all = "lowercase")]
pub enum Foot {
    /// On the plate (or the raft).
    Plate,
    /// On a lower part of the model. `point` is relative to the object position.
    Model { point: Vec3 },
}

/// One support. Points are relative to the object position (world orientation).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Support {
    pub id: u32,
    /// Where the tip touches the model.
    pub tip: Vec3,
    /// Surface normal at the tip (points out of the model).
    pub normal: Vec3,
    /// End of the thin tip cone, where the body starts.
    pub joint: Vec3,
    /// A bend that leads the body around a part of the model below.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub knee: Option<Vec3>,
    pub foot: Foot,
    pub tip_diameter: f32,
    pub diameter: f32,
    /// Placed by hand (kept apart from automatic ones when regenerating).
    #[serde(default)]
    pub manual: bool,
}

impl Support {
    /// The top of the straight vertical part.
    pub fn column_top(&self) -> Vec3 {
        self.knee.unwrap_or(self.joint)
    }
}

/// A cross brace between the vertical parts of two supports. Heights are relative to the
/// object position.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Brace {
    pub a: u32,
    pub b: u32,
    /// Height on support `a`.
    pub za: f32,
    /// Height on support `b`.
    pub zb: f32,
    pub diameter: f32,
}

/// All supports of one object, plus its raft.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SupportSet {
    pub supports: Vec<Support>,
    #[serde(default)]
    pub braces: Vec<Brace>,
    /// Raft outline (relative XY, counter-clockwise). Empty = no raft.
    #[serde(default)]
    pub raft: Vec<Vec2>,
    /// What kind of base the supports stand on (see [`SupportSet::rebuild_base`]).
    #[serde(default)]
    pub base: BaseKind,
    /// Round pads on the plate (relative XY of their centers), for pads and grid bases.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pads: Vec<Vec2>,
    #[serde(default)]
    pub pad_diameter: f32,
    /// Bars that lie on the plate between pads, for a grid base (relative XY of their ends).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bars: Vec<[Vec2; 2]>,
    pub raft_thickness: f32,
    pub base_diameter: f32,
    /// Height of the cone foot on the plate or raft.
    pub base_height: f32,
    pub next_id: u32,
}

impl Default for SupportSet {
    fn default() -> Self {
        Self {
            supports: Vec::new(),
            braces: Vec::new(),
            raft: Vec::new(),
            base: BaseKind::Raft,
            pads: Vec::new(),
            pad_diameter: 0.0,
            bars: Vec::new(),
            raft_thickness: 1.0,
            base_diameter: 2.5,
            base_height: 0.8,
            next_id: 1,
        }
    }
}

impl SupportSet {
    pub fn is_empty(&self) -> bool {
        self.supports.is_empty()
    }

    pub fn get(&self, id: u32) -> Option<&Support> {
        self.supports.iter().find(|s| s.id == id)
    }

    /// Is there a base (raft or pads) under the supports?
    pub fn has_base(&self) -> bool {
        !self.raft.is_empty() || !self.pads.is_empty()
    }

    /// Height of the raft or pad top (0 without a base).
    pub fn raft_top(&self) -> f32 {
        if self.has_base() {
            self.raft_thickness
        } else {
            0.0
        }
    }

    /// Absolute height where a plate support's vertical body ends and its cone foot begins.
    pub fn base_top(&self) -> f32 {
        self.raft_top() + self.base_height
    }

    pub fn push(&mut self, mut s: Support) -> u32 {
        s.id = self.next_id;
        self.next_id += 1;
        self.supports.push(s);
        s.id
    }

    /// Removes supports and their braces. Returns how many were removed.
    pub fn remove(&mut self, ids: &[u32]) -> usize {
        let before = self.supports.len();
        self.supports.retain(|s| !ids.contains(&s.id));
        self.braces
            .retain(|b| !ids.contains(&b.a) && !ids.contains(&b.b));
        before - self.supports.len()
    }

    /// Recomputes the base under all plate feet. Keeps no base when `enabled` is false. See
    /// [`SupportSet::rebuild_base`].
    pub fn rebuild_raft(&mut self, enabled: bool, margin: f32) {
        let kind = self.base;
        self.rebuild_base(enabled.then_some(kind), margin);
    }

    /// Recomputes the base under all plate feet:
    ///
    /// - a raft: the convex hull of the feet, grown by the base radius and `margin`;
    /// - pads: a round pad under each foot, half again as wide as the foot and `margin` more;
    /// - a grid: pads tied by bars to their nearest neighbours, so the base is one piece.
    pub fn rebuild_base(&mut self, kind: Option<BaseKind>, margin: f32) {
        self.raft.clear();
        self.pads.clear();
        self.bars.clear();
        let Some(kind) = kind else {
            return;
        };
        self.base = kind;
        let feet: Vec<Vec2> = self
            .plate_feet()
            .map(|s| s.column_top().truncate())
            .collect();
        match kind {
            BaseKind::Raft => {
                let r = self.base_diameter / 2.0 + margin;
                let mut pts = Vec::new();
                for c in &feet {
                    for k in 0..12 {
                        let a = k as f32 / 12.0 * std::f32::consts::TAU;
                        pts.push(*c + Vec2::new(a.cos(), a.sin()) * r);
                    }
                }
                self.raft = convex_hull(pts);
            }
            BaseKind::Pads | BaseKind::Grid => {
                self.pad_diameter = self.base_diameter * 1.5 + margin.min(2.0);
                self.pads = feet.clone();
                if kind == BaseKind::Grid {
                    self.bars = grid_bars(&feet, self.base_diameter * 6.0 + 6.0);
                }
            }
        }
    }

    /// Width of the bars of a grid base.
    pub fn bar_width(&self) -> f32 {
        (self.base_diameter * 0.5).max(1.0)
    }

    /// Lowest point of the supports relative to the object, for checks.
    pub fn plate_feet(&self) -> impl Iterator<Item = &Support> {
        self.supports.iter().filter(|s| s.foot == Foot::Plate)
    }
}

/// A drain hole in a hollow model. Point and direction are in the object's own coordinates
/// (before its transform), so the hole moves and turns with the object.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DrainHole {
    /// Where the hole starts on the surface.
    pub point: Vec3,
    /// Into the model.
    pub dir: Vec3,
    pub diameter: f32,
    pub depth: f32,
}

/// Bars for a grid base: every foot to its three nearest neighbours within `reach`, plus the
/// shortest bars that join all groups into one piece (a spanning tree), whatever the distance.
pub fn grid_bars(feet: &[Vec2], reach: f32) -> Vec<[Vec2; 2]> {
    let n = feet.len();
    if n < 2 {
        return Vec::new();
    }
    let mut pairs: Vec<(f32, usize, usize)> = Vec::new();
    for i in 0..n {
        for j in i + 1..n {
            pairs.push((feet[i].distance(feet[j]), i, j));
        }
    }
    pairs.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut parent: Vec<usize> = (0..n).collect();
    fn find(p: &mut [usize], mut x: usize) -> usize {
        while p[x] != x {
            p[x] = p[p[x]];
            x = p[x];
        }
        x
    }
    let mut degree = vec![0u8; n];
    let mut chosen = std::collections::BTreeSet::new();
    // The spanning tree first: the base must be one piece.
    for &(_, i, j) in &pairs {
        let (a, b) = (find(&mut parent, i), find(&mut parent, j));
        if a != b {
            parent[a] = b;
            chosen.insert((i, j));
            degree[i] += 1;
            degree[j] += 1;
        }
    }
    // Then short bars that close the net, so it does not bend.
    for &(d, i, j) in &pairs {
        if d > reach {
            break;
        }
        if (degree[i] < 3 || degree[j] < 3) && chosen.insert((i, j)) {
            degree[i] += 1;
            degree[j] += 1;
        }
    }
    chosen
        .into_iter()
        .map(|(i, j)| [feet[i], feet[j]])
        .collect()
}

/// Convex hull, counter-clockwise (Andrew's monotone chain).
pub fn convex_hull(mut pts: Vec<Vec2>) -> Vec<Vec2> {
    pts.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
    pts.dedup();
    if pts.len() < 3 {
        return pts;
    }
    let cross = |o: Vec2, a: Vec2, b: Vec2| (a - o).perp_dot(b - o);
    let mut lower: Vec<Vec2> = Vec::new();
    for &p in &pts {
        while lower.len() >= 2 && cross(lower[lower.len() - 2], lower[lower.len() - 1], p) <= 0.0 {
            lower.pop();
        }
        lower.push(p);
    }
    let mut upper: Vec<Vec2> = Vec::new();
    for &p in pts.iter().rev() {
        while upper.len() >= 2 && cross(upper[upper.len() - 2], upper[upper.len() - 1], p) <= 0.0 {
            upper.pop();
        }
        upper.push(p);
    }
    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_grid_base_is_one_piece() {
        // Two groups of feet far apart and a stray one.
        let mut feet = Vec::new();
        for i in 0..4 {
            for j in 0..3 {
                feet.push(Vec2::new(i as f32 * 3.0, j as f32 * 3.0));
                feet.push(Vec2::new(60.0 + i as f32 * 3.0, j as f32 * 3.0));
            }
        }
        feet.push(Vec2::new(30.0, 40.0));
        let bars = grid_bars(&feet, 21.0);
        // Joined: walk the bars from the first foot and reach every foot.
        let mut seen = vec![false; feet.len()];
        let mut stack = vec![0];
        while let Some(i) = stack.pop() {
            if std::mem::replace(&mut seen[i], true) {
                continue;
            }
            for [a, b] in &bars {
                for (x, y) in [(a, b), (b, a)] {
                    if *x == feet[i] {
                        stack.push(feet.iter().position(|f| f == y).unwrap());
                    }
                }
            }
        }
        assert!(seen.iter().all(|s| *s));
        // A net, not just a chain: more bars than a tree.
        assert!(bars.len() > feet.len());
    }

    #[test]
    fn hull_is_ccw_and_round_trips_as_json() {
        let h = convex_hull(vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(2.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(2.0, 2.0),
            Vec2::new(0.0, 2.0),
        ]);
        assert_eq!(h.len(), 4);
        let area: f32 = (0..h.len())
            .map(|i| h[i].perp_dot(h[(i + 1) % h.len()]))
            .sum::<f32>()
            / 2.0;
        assert!((area - 4.0).abs() < 1e-5);
        let mut set = SupportSet::default();
        let id = set.push(Support {
            id: 0,
            tip: Vec3::new(0.0, 0.0, 5.0),
            normal: -Vec3::Z,
            joint: Vec3::new(0.0, 0.0, 3.0),
            knee: None,
            foot: Foot::Plate,
            tip_diameter: 0.3,
            diameter: 1.0,
            manual: false,
        });
        set.rebuild_raft(true, 2.0);
        assert!(set.raft.len() >= 8);
        set.rebuild_base(Some(BaseKind::Pads), 2.0);
        assert!(set.raft.is_empty() && set.pads.len() == 1 && set.has_base());
        set.rebuild_raft(true, 2.0);
        assert_eq!(set.pads.len(), 1, "the kind of base stays");
        let json = serde_json::to_string(&set).unwrap();
        let back: SupportSet = serde_json::from_str(&json).unwrap();
        assert_eq!(back, set);
        assert_eq!(set.remove(&[id]), 1);
    }
}
