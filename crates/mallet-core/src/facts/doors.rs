//! Doors, where each leads, and which paths go through them (ADR 0067).
//!
//! A door that leads somewhere else, into a house or out of a cave, is a
//! placed object with a teleport: the door on the other side, and the
//! point where the player arrives there. The door on the other side has a
//! teleport of its own that leads back. Characters walk through a door
//! where a navigation mesh has a door triangle naming it.
//!
//! Unlike [`super::ObjectFacts`], which keeps only what mods place, the
//! doors of the game are here as well: a mod that adds a door at the spot
//! of one of the game's doors is the case these facts are for.

use super::RecordId;
use super::objects::Place;

/// Every door of a setup, in the version the game uses, and the door
/// triangles of the navigation meshes the game uses.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DoorFacts {
    /// The placed doors that are shown when the game starts. Deleted doors
    /// and doors switched off at the start are left out.
    pub doors: Vec<PlacedDoor>,
    /// The door triangles of the navigation meshes, in the version the
    /// game uses.
    pub links: Vec<DoorLink>,
    /// The internal names of the cells and worldspaces the doors are in.
    pub place_names: Vec<super::objects::PlaceName>,
}

/// A placed door.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacedDoor {
    /// The reference, by the plugin that placed it first.
    pub id: RecordId,
    /// The plugin whose version the game uses, in lower case.
    pub plugin: String,
    /// Whether one of the game's own plugins placed it.
    pub official: bool,
    /// The other plugins that change this door, in lower case, in load
    /// order: the plugin that placed it first included, if it is not the
    /// one whose version is used.
    pub changed_by: Vec<String>,
    /// The base object: which door.
    pub base: RecordId,
    /// The base object's internal name, if known.
    pub base_name: Option<String>,
    /// Whether the base object has a model: a door without one is not
    /// seen.
    pub visible: bool,
    /// The box around the base object's model, before turning and scaling,
    /// as two corners; `None` if the base object does not say.
    pub bounds: Option<[[i32; 3]; 2]>,
    /// Where it stands.
    pub place: Place,
    /// The position, in game units.
    pub position: [i32; 3],
    /// The rotation about the x, y and z axes, in thousandths of a radian.
    pub rotation: [i32; 3],
    /// Whether quests and scripts may refer to it wherever the player is.
    pub persistent: bool,
    /// The placed object that switches it on and off, if any.
    pub enable_parent: Option<EnableParent>,
    /// Where it leads, if it is a door to somewhere else.
    pub teleport: Option<Teleport>,
}

/// The placed object a door is switched on and off with, as a quest
/// switches one door for another.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnableParent {
    /// The object.
    pub parent: RecordId,
    /// Whether the door shows while the object does not.
    pub opposite: bool,
}

/// Where a door leads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Teleport {
    /// The door on the other side.
    pub door: RecordId,
    /// Where the player arrives there, in game units.
    pub position: [i32; 3],
    /// Which way the player faces there, in thousandths of a radian.
    pub rotation: [i32; 3],
    /// The teleport's flags as the plugin keeps them.
    pub flags: u32,
}

/// A triangle of a navigation mesh where characters step through a door.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoorLink {
    /// The navigation mesh, by the plugin that first defined it.
    pub navmesh: RecordId,
    /// The plugin whose version of the mesh the game uses, in lower case.
    pub plugin: String,
    /// The triangle, by its number in the mesh.
    pub triangle: u32,
    /// The door.
    pub door: RecordId,
}
