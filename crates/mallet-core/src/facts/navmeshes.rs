//! Where characters can walk, and which plugins change it.
//!
//! The game keeps, for every cell, a net of triangles on the ground that
//! characters find their way on: its navigation meshes. A plugin that
//! changes a navigation mesh of another plugin replaces it as a whole. When
//! two plugins change the same one, the later one wins and the changes of
//! the earlier one are gone.

use super::RecordId;

/// The navigation meshes that mods change.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NavmeshFacts {
    /// What each plugin that is not the game's own changes, in load order.
    /// Plugins that change no navigation mesh of another plugin are left
    /// out.
    pub changes: Vec<NavmeshChanges>,
}

/// The navigation meshes of other plugins that one plugin changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavmeshChanges {
    /// The plugin, in the spelling of the load order.
    pub plugin: String,
    /// The navigation meshes it changes.
    pub navmeshes: Vec<NavmeshChange>,
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
}
