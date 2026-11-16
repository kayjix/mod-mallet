//! The shape of the land: how high the ground is in each cell, and where
//! there is water.
//!
//! The game describes the ground of every outside cell as a grid of
//! heights. Mod Mallet keeps a few samples of each cell, enough to draw the
//! landscape under the map of where mods place objects.

use super::RecordId;

/// How many height samples per cell side are kept.
pub const SAMPLES_PER_CELL: usize = 4;

/// The land of every worldspace, in the version the game uses.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TerrainFacts {
    /// One entry per worldspace that has land.
    pub worlds: Vec<TerrainWorld>,
}

/// The land of one worldspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerrainWorld {
    /// The worldspace.
    pub worldspace: RecordId,
    /// The cells that have land.
    pub cells: Vec<TerrainCell>,
}

/// The land of one cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerrainCell {
    /// The cell's column; east is larger.
    pub x: i32,
    /// The cell's row; north is larger.
    pub y: i32,
    /// Heights in game units, [`SAMPLES_PER_CELL`] rows of as many samples,
    /// from south to north and from west to east.
    pub heights: Vec<i32>,
    /// The height of the water surface, if the cell has water.
    pub water: Option<i32>,
}
