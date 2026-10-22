//! Settings that a project carries and a user can save as a preset.
//!
//! Every struct uses `#[serde(default)]`, so old files and partial presets still load.

use serde::{Deserialize, Serialize};

/// Exposure and layer settings. Depend on the resin (and its color, batch, temperature).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ExposureSettings {
    pub layer_height_mm: f32,
    pub exposure_s: f32,
    pub bottom_layers: u32,
    pub bottom_exposure_s: f32,
    /// Layers after the bottom ones whose exposure steps down to the normal value.
    pub transition_layers: u32,
    /// UV light power, 0–255.
    pub light_pwm: u8,
}

impl Default for ExposureSettings {
    fn default() -> Self {
        Self {
            layer_height_mm: 0.05,
            exposure_s: 2.5,
            bottom_layers: 5,
            bottom_exposure_s: 30.0,
            transition_layers: 5,
            light_pwm: 255,
        }
    }
}

impl ExposureSettings {
    /// Exposure time of layer `index` (0 = first layer on the plate).
    pub fn exposure_of(&self, index: u32) -> f32 {
        if index < self.bottom_layers {
            return self.bottom_exposure_s;
        }
        // Steps down evenly; the last transition layer reaches the normal exposure.
        // (Same steps as the files the owner printed: 29 s → 23.48 → 17.96 → … → 1.4 s.)
        let t = index - self.bottom_layers;
        if t < self.transition_layers {
            let step = (self.bottom_exposure_s - self.exposure_s) / self.transition_layers as f32;
            return self.bottom_exposure_s - step * (t + 1) as f32;
        }
        self.exposure_s
    }

    /// True for bottom layers, which use the slow "bottom" motion.
    pub fn is_bottom(&self, index: u32) -> bool {
        index < self.bottom_layers
    }
}

/// Plate motion, waits and anti-aliasing. Depend mostly on the printer.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MotionSettings {
    /// Anti-aliasing level: 1 (off), 2, 4 or 8.
    pub anti_aliasing: u8,
    /// Wait after the light turns off, before the plate lifts.
    pub bottom_rest_after_exposure_s: f32,
    /// Wait after the plate comes back, before the next exposure.
    pub bottom_rest_after_retract_s: f32,
    pub rest_after_exposure_s: f32,
    pub rest_after_retract_s: f32,
    /// Two-stage motion: `[first stage, second stage]`. Distances in mm, speeds in mm/min.
    /// A second stage of 0 mm means a single-stage move.
    pub bottom_lift_mm: [f32; 2],
    pub bottom_lift_speed: [f32; 2],
    pub bottom_retract_mm: [f32; 2],
    pub bottom_retract_speed: [f32; 2],
    pub lift_mm: [f32; 2],
    pub lift_speed: [f32; 2],
    pub retract_mm: [f32; 2],
    pub retract_speed: [f32; 2],
}

impl Default for MotionSettings {
    fn default() -> Self {
        Self {
            anti_aliasing: 4,
            bottom_rest_after_exposure_s: 0.5,
            bottom_rest_after_retract_s: 2.0,
            rest_after_exposure_s: 0.5,
            rest_after_retract_s: 1.5,
            bottom_lift_mm: [5.0, 0.0],
            bottom_lift_speed: [60.0, 60.0],
            bottom_retract_mm: [5.0, 0.0],
            bottom_retract_speed: [150.0, 150.0],
            lift_mm: [5.0, 0.0],
            lift_speed: [80.0, 80.0],
            retract_mm: [5.0, 0.0],
            retract_speed: [150.0, 150.0],
        }
    }
}

impl MotionSettings {
    /// Seconds the plate spends moving between two layers.
    pub fn move_time_s(&self, bottom: bool) -> f32 {
        let (lm, ls, rm, rs) = if bottom {
            (
                self.bottom_lift_mm,
                self.bottom_lift_speed,
                self.bottom_retract_mm,
                self.bottom_retract_speed,
            )
        } else {
            (
                self.lift_mm,
                self.lift_speed,
                self.retract_mm,
                self.retract_speed,
            )
        };
        let t = |d: [f32; 2], s: [f32; 2]| {
            (0..2)
                .map(|i| if s[i] > 0.0 { d[i] / s[i] * 60.0 } else { 0.0 })
                .sum::<f32>()
        };
        t(lm, ls) + t(rm, rs)
    }
}

/// Everything the printer needs to know besides the image of each layer.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PrintSettings {
    #[serde(flatten)]
    pub exposure: ExposureSettings,
    #[serde(flatten)]
    pub motion: MotionSettings,
}

impl PrintSettings {
    /// Rough print time in seconds for `layers` layers.
    pub fn estimate_time_s(&self, layers: u32) -> f32 {
        let e = &self.exposure;
        let m = &self.motion;
        (0..layers)
            .map(|i| {
                let bottom = e.is_bottom(i);
                let rest = if bottom {
                    m.bottom_rest_after_exposure_s + m.bottom_rest_after_retract_s
                } else {
                    m.rest_after_exposure_s + m.rest_after_retract_s
                };
                e.exposure_of(i) + rest + m.move_time_s(bottom)
            })
            .sum()
    }
}

/// How automatic and manual supports are built.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SupportSettings {
    /// Diameter of the contact point on the model.
    pub tip_diameter_mm: f32,
    /// Length of the thin cone at the top.
    pub tip_length_mm: f32,
    /// Diameter of the support body.
    pub diameter_mm: f32,
    /// Diameter of the foot on the raft.
    pub base_diameter_mm: f32,
    /// Surfaces steeper than this (from the vertical) get supports.
    pub overhang_angle_deg: f32,
    /// Support points per square millimetre of overhang, roughly.
    pub density: f32,
    /// Keep support bodies at least this far from the model.
    pub safe_distance_mm: f32,
    /// Lift the model this high when supports are added to a model on the plate.
    pub lift_height_mm: f32,
    /// Connect neighbouring supports with cross braces.
    pub auto_bracing: bool,
    /// New manual supports connect to the nearest ones with braces.
    pub manual_bracing: bool,
    /// Distance between braces along a support.
    pub bracing_spacing_mm: f32,
    /// Let short supports start on a nearby support instead of the raft.
    pub auto_parent: bool,
    /// Add supports under islands: parts of a layer not connected to the layer below.
    pub support_islands: bool,
    /// Raft under the supports.
    pub raft: bool,
    pub raft_thickness_mm: f32,
    /// How far the raft goes past the supports.
    pub raft_margin_mm: f32,
}

impl Default for SupportSettings {
    fn default() -> Self {
        Self::medium()
    }
}

impl SupportSettings {
    pub fn light() -> Self {
        Self {
            tip_diameter_mm: 0.25,
            tip_length_mm: 1.6,
            diameter_mm: 0.8,
            base_diameter_mm: 2.0,
            density: 0.6,
            ..Self::medium()
        }
    }

    pub fn medium() -> Self {
        Self {
            tip_diameter_mm: 0.35,
            tip_length_mm: 2.0,
            diameter_mm: 1.0,
            base_diameter_mm: 2.5,
            overhang_angle_deg: 45.0,
            density: 1.0,
            safe_distance_mm: 0.5,
            lift_height_mm: 5.0,
            auto_bracing: true,
            manual_bracing: true,
            bracing_spacing_mm: 6.0,
            auto_parent: true,
            support_islands: true,
            raft: true,
            raft_thickness_mm: 1.0,
            raft_margin_mm: 2.0,
        }
    }

    pub fn heavy() -> Self {
        Self {
            tip_diameter_mm: 0.5,
            tip_length_mm: 2.2,
            diameter_mm: 1.4,
            base_diameter_mm: 3.0,
            density: 1.6,
            ..Self::medium()
        }
    }
}

/// Hollowing and drain holes.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct HollowSettings {
    pub enabled: bool,
    pub wall_thickness_mm: f32,
    /// Size of the voxels used to build the inner wall. Smaller is slower and more exact.
    pub voxel_mm: f32,
    pub drain_hole_diameter_mm: f32,
    /// How deep a drain hole goes into the wall.
    pub drain_hole_depth_mm: f32,
}

impl Default for HollowSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            wall_thickness_mm: 2.5,
            voxel_mm: 0.5,
            drain_hole_diameter_mm: 3.0,
            drain_hole_depth_mm: 4.0,
        }
    }
}

/// How models move on the plate.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LayoutSettings {
    /// Keep the lowest point of a model on the plate after every move, rotation or scale.
    pub keep_on_plate: bool,
    /// Gap between models when they are arranged.
    pub arrange_spacing_mm: f32,
    /// Snap step for moving with Ctrl held. 0 = no snapping.
    pub move_snap_mm: f32,
    /// Snap step for rotating with Ctrl held.
    pub rotate_snap_deg: f32,
    /// Snap step for scaling with Ctrl held, in percent.
    pub scale_snap_percent: f32,
}

impl Default for LayoutSettings {
    fn default() -> Self {
        Self {
            keep_on_plate: true,
            arrange_spacing_mm: 5.0,
            move_snap_mm: 1.0,
            rotate_snap_deg: 15.0,
            scale_snap_percent: 10.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transition_steps_down() {
        let e = ExposureSettings {
            bottom_layers: 2,
            bottom_exposure_s: 30.0,
            exposure_s: 2.0,
            transition_layers: 3,
            ..Default::default()
        };
        let times: Vec<f32> = (0..7).map(|i| e.exposure_of(i)).collect();
        assert_eq!(times[0], 30.0);
        assert_eq!(times[1], 30.0);
        assert!(times[2] < 30.0 && times[2] > times[3] && times[3] > times[4]);
        assert!((times[4] - 2.0).abs() < 1e-5);
        assert_eq!(times[5], 2.0);
        let lychee = ExposureSettings {
            bottom_layers: 5,
            bottom_exposure_s: 29.0,
            exposure_s: 1.4,
            transition_layers: 5,
            ..Default::default()
        };
        assert!((lychee.exposure_of(5) - 23.48).abs() < 1e-4);
        assert!((lychee.exposure_of(6) - 17.96).abs() < 1e-4);
    }

    #[test]
    fn flat_toml_round_trip() {
        let s = PrintSettings::default();
        let text = toml::to_string(&s).unwrap();
        assert!(text.contains("exposure_s"));
        assert!(text.contains("lift_speed"));
        let back: PrintSettings = toml::from_str(&text).unwrap();
        assert_eq!(back, s);
        let partial: PrintSettings = toml::from_str("exposure_s = 1.4").unwrap();
        assert_eq!(partial.exposure.exposure_s, 1.4);
        assert_eq!(partial.motion, MotionSettings::default());
    }
}
