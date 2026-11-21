//! Where characters can walk, and which plugins change it.
//!
//! The game keeps, for every cell, a net of triangles on the ground that
//! characters find their way on: its navigation meshes. A plugin that
//! changes a navigation mesh of another plugin replaces it as a whole. When
//! two plugins change the same one, the later one wins and the changes of
//! the earlier one are gone.

use super::RecordId;
use super::objects::Place;
use crate::reach::DestinationKind;

/// The navigation meshes that mods change.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NavmeshFacts {
    /// What each plugin that is not the game's own changes, in load order.
    /// Plugins that neither change a navigation mesh of another plugin nor
    /// connect a door in one of their own are left out.
    pub changes: Vec<NavmeshChanges>,
    /// The navigation meshes under the objects Mod Mallet's patch hides.
    pub near_hidden: NearHidden,
    /// The places characters walk to in the cells of [`Self::near_hidden`]
    /// (ADR 0059).
    pub destinations: Vec<PlacedDestination>,
}

/// The navigation meshes of other plugins that one plugin changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavmeshChanges {
    /// The plugin, in the spelling of the load order.
    pub plugin: String,
    /// The navigation meshes it changes.
    pub navmeshes: Vec<NavmeshChange>,
    /// The doors its own navigation meshes let characters walk through.
    pub connected: Vec<RecordId>,
}

/// One navigation mesh a plugin changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavmeshChange {
    /// The navigation mesh, by the plugin that first defined it.
    pub id: RecordId,
    /// Where it lies, for people: the internal name of its cell, or the
    /// worldspace and the cell's grid position outside. `None` if the
    /// plugin does not say.
    pub place: Option<String>,
    /// The placed doors the plugin's version lets characters walk through.
    /// `None` where its geometry could not be read, as in games other than
    /// Skyrim.
    pub doors: Option<Vec<RecordId>>,
}

/// The navigation meshes around the objects that Mod Mallet's patch hides,
/// and the ground there (ADR 0051). Hiding an object takes away what
/// characters stand on, but not the paths they follow over it. Empty when
/// no patch is installed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NearHidden {
    /// Every plugin's version of every navigation mesh in the cells of the
    /// hidden objects, in load order.
    pub navmeshes: Vec<NavmeshShape>,
    /// The ground of the outside cells of the hidden objects, in the
    /// version the game uses. A worldspace that stands on the land of
    /// another, as cities do, has that land here under its own name.
    pub ground: Vec<GroundCell>,
}

/// One plugin's version of a navigation mesh, with its triangles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavmeshShape {
    /// The navigation mesh, by the plugin that first defined it.
    pub id: RecordId,
    /// The plugin of this version, in the spelling of the load order.
    pub plugin: String,
    /// Whether this is the version the game uses: the last one.
    pub used: bool,
    /// The cell it lies in inside, or its worldspace outside.
    pub place: Place,
    /// Where it lies, for people, as in [`NavmeshChange::place`].
    pub place_name: Option<String>,
    /// The corners of its triangles, in whole game units, in the
    /// coordinates of the worldspace or the cell.
    pub vertices: Vec<[i32; 3]>,
    /// The triangles characters walk on, three corner numbers each.
    pub triangles: Vec<[u32; 3]>,
    /// For each triangle, what lies beyond its three edges: where
    /// characters can step on from it (ADR 0052).
    pub edges: Vec<[Beyond; 3]>,
    /// The triangles where characters step through a door, with the door.
    pub doors: Vec<(u32, RecordId)>,
}

/// What lies beyond one edge of a triangle of a navigation mesh.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Beyond {
    /// Nothing characters can step onto: a wall or a drop.
    Nothing,
    /// Another triangle of the same mesh, by its number.
    Triangle(u32),
    /// A triangle of a neighbouring mesh.
    Link {
        /// The neighbouring mesh, by the plugin that first defined it.
        navmesh: RecordId,
        /// The triangle, by its number there.
        triangle: u32,
    },
}

/// The ground of one outside cell, in the version the game uses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroundCell {
    /// The worldspace.
    pub worldspace: RecordId,
    /// The cell's column.
    pub x: i32,
    /// The cell's row.
    pub y: i32,
    /// The heights of 33 rows of 33 points, 128 game units apart, in whole
    /// game units; the southern row first, each from west to east.
    pub heights: Vec<i32>,
}

/// A placed object characters walk to, in the cells of navigation meshes
/// that were read (ADR 0059): a door, furniture, an idle marker or a map
/// marker, in the version the game uses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacedDestination {
    /// The placed object, by the plugin that placed it first.
    pub id: RecordId,
    /// The plugin whose version the game uses, in the spelling of the load
    /// order.
    pub plugin: String,
    /// What kind of place it is.
    pub kind: DestinationKind,
    /// Its name: the one players see where the plugin spells it out, else
    /// the internal name of what is placed. `None` if neither is known.
    pub name: Option<String>,
    /// The cell it lies in inside, or its worldspace outside.
    pub place: Place,
    /// Where it stands, in whole game units.
    pub position: [i32; 3],
    /// For a door that leads elsewhere, the door on the other side.
    pub leads_to: Option<RecordId>,
}
