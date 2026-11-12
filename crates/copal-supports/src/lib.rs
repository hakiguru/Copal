//! Prepare tools: island detection, automatic and manual supports, rafts, hollowing and
//! drain holes.
//!
//! Supports are stored relative to their object's position (world orientation), so they move
//! with the object when it is dragged over the plate. Feet that stand on the plate are always at
//! the plate, whatever the object's height.

pub mod generate;
pub mod geometry;
pub mod hollow;
pub mod islands;
pub mod model;
pub mod orient;

pub use copal_mesh::Mesh;
pub use generate::{
    Contact, Mode, Overhangs, Reason, Target, add_manual, add_manual_for, find_overhangs, generate,
    generate_for, spacing_for,
};
pub use geometry::{SupportMesh, build_mesh};
pub use islands::{Island, find_islands, find_islands_in};
pub use model::{Brace, DrainHole, Foot, Support, SupportSet};
pub use orient::{Analyzer, Goal, OrientStats, Oriented};
