//! The shape of the land: how high the ground is in each cell, and where
//! there is water.
//!
//! The game describes the ground of every outside cell as a grid of
//! heights. Mod Mallet keeps a few samples of each cell, enough to draw the
//! landscape under the map of where mods place objects.

use super::RecordId;

/// How many height samples per cell side are kept.
pub const SAMPLES_PER_CELL: usize = 8;

/// The land of every worldspace, in the version the game uses.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TerrainFacts {
    /// One entry per worldspace that has land.
    pub worlds: Vec<TerrainWorld>,
    /// What the record of every worldspace says, whether or not it has
    /// land.
    pub worldspaces: Vec<WorldspaceFacts>,
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

/// What a worldspace's own record says about it, in the version the game
/// uses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldspaceFacts {
    /// The worldspace.
    pub id: RecordId,
    /// Its internal name, if it has one.
    pub editor_id: Option<String>,
    /// The name the game shows, if it has one.
    pub title: Option<String>,
    /// The worldspace it belongs to, such as Skyrim for a city.
    pub parent: Option<RecordId>,
    /// Whether it uses its parent's land.
    pub uses_parent_land: bool,
    /// Whether the game calls it a small world.
    pub small: bool,
    /// Whether the game draws no water in its distant view.
    pub no_lod_water: bool,
    /// The cells it spans, west, south, east, north, from its record.
    pub bounds: Option<[i32; 4]>,
    /// The height of its water, where a cell does not set its own.
    pub water: Option<i32>,
    /// Whether it comes with the game, the Creation Club or a mod.
    pub origin: crate::WorldOrigin,
}
