//! What a set of supports is made of.

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

    /// Height of the raft top (0 without a raft).
    pub fn raft_top(&self) -> f32 {
        if self.raft.is_empty() {
            0.0
        } else {
            self.raft_thickness
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

    /// Recomputes the raft outline around all plate feet: their convex hull, grown by the base
    /// radius and `margin`. Keeps no raft when `enabled` is false.
    pub fn rebuild_raft(&mut self, enabled: bool, margin: f32) {
        self.raft.clear();
        if !enabled {
            return;
        }
        let r = self.base_diameter / 2.0 + margin;
        let mut pts = Vec::new();
        for s in &self.supports {
            if s.foot == Foot::Plate {
                let c = s.column_top().truncate();
                for k in 0..12 {
                    let a = k as f32 / 12.0 * std::f32::consts::TAU;
                    pts.push(c + Vec2::new(a.cos(), a.sin()) * r);
                }
            }
        }
        self.raft = convex_hull(pts);
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
        let json = serde_json::to_string(&set).unwrap();
        let back: SupportSet = serde_json::from_str(&json).unwrap();
        assert_eq!(back, set);
        assert_eq!(set.remove(&[id]), 1);
    }
}
