//! The places of the world map: towns, caves, forts and the like.
//!
//! Every place the game's map shows is a *map marker*, a reference to the
//! game's marker object with a name, a kind and a position. Mods add
//! markers for the places they add, and change or remove the game's.

use super::RecordId;
use crate::report::PlaceKind;

/// The map markers of a setup.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MarkerFacts {
    /// Every marker that some active plugin places, sorted by identity.
    pub markers: Vec<MapMarkerFact>,
}

/// One map marker, as the plugin that placed it wrote it and as the game
/// will use it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapMarkerFact {
    /// The reference. Its plugin is the one that placed it first.
    pub id: RecordId,
    /// The worldspace it lies in.
    pub worldspace: RecordId,
    /// The version of the plugin that placed it.
    pub first: MarkerVersion,
    /// The version the game uses: the one of the last plugin in the load
    /// order that has it.
    pub last: MarkerVersion,
}

/// One plugin's version of a map marker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkerVersion {
    /// The plugin, in the spelling of the load order.
    pub plugin: String,
    /// Whether the plugin is one of the game's own.
    pub official: bool,
    /// The kind of place. Each game numbers its kinds in its own way; the
    /// game adapter translates them.
    pub kind: PlaceKind,
    /// The name the map shows, if it could be read.
    pub name: Option<String>,
    /// The position, in game units.
    pub position: [i32; 3],
    /// Whether this version deletes the marker or switches it off.
    pub removed: bool,
}
