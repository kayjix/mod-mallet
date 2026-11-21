//! Doors that mods stack on top of each other, and how to merge them into
//! one (ADR 0067).
//!
//! A spot of the kind [`crate::SpotKind::Door`] carries a [`DoorGroup`]:
//! the doors standing there, where each leads, and for each door a plan
//! to keep that one and let the others go. Hiding the others is what the
//! patch of ADR 0025 does already. What else has to change, so that
//! whatever used a hidden door uses the kept one, is said as
//! [`RecordChange`]s: data for a plugin Mod Mallet writes, not texts.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{ObjectRef, SpotLocation};

/// Doors that stand on almost the same spot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DoorGroup {
    /// The doors, the one suggested to keep first.
    pub doors: Vec<StackedDoor>,
    /// For each door, in the same order, what keeping it means.
    pub plans: Vec<DoorMerge>,
    /// Whether two of the doors stand in the same plane, so that the game
    /// shows them flickering through each other.
    pub flicker: bool,
}

/// One door of a [`DoorGroup`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "each is a fact the window shows"
)]
#[serde(rename_all = "camelCase")]
pub struct StackedDoor {
    /// The door, by the plugin that placed it first.
    pub object: ObjectRef,
    /// The plugin whose version of the door the game uses, as the load
    /// order spells it.
    pub plugin: String,
    /// The internal name of the door's base object.
    pub name: String,
    /// Where it leads; `None` for a door that only opens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub leads_to: Option<DoorDestination>,
    /// Whether the door on the other side leads back to this one.
    pub leads_back: bool,
    /// Whether characters walk through it: a path of the game names it.
    pub walkable: bool,
    /// Whether characters walk through the door on the other side.
    pub other_side_walkable: bool,
    /// How many doors lead to this one and how many paths name it.
    pub references: u32,
    /// Whether the player sees this door in front of the others.
    pub front: bool,
    /// How far it stands from the first door, in game units.
    pub offset: u32,
    /// Whether keeping this one is suggested.
    pub suggested: bool,
}

/// Where a door leads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DoorDestination {
    /// The place on the other side.
    pub location: SpotLocation,
    /// The door on the other side, if it is there.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub door: Option<ObjectRef>,
}

/// What keeping one door of a group means.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DoorMerge {
    /// The door that stays.
    pub keep: ObjectRef,
    /// The doors the patch hides.
    pub hide: Vec<ObjectRef>,
    /// What has to change in the plugins, so that whatever used a hidden
    /// door uses the kept one. Empty if hiding is all it takes.
    pub changes: Vec<RecordChange>,
    /// Places only a hidden door led to: they can no longer be entered
    /// here.
    pub lost: Vec<DoorDestination>,
}

/// A change to a record that merging doors needs. Mod Mallet writes such
/// changes as overrides in a plugin of its own (ADR 0060); the record is
/// copied from the version the game uses and only the named field changes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum RecordChange {
    /// A door leads to another door: its teleport (`XTEL`) names `to`
    /// instead of `from`.
    Teleport {
        /// The door whose teleport changes.
        door: ObjectRef,
        /// Where that door stands, in whole game units, to find it by.
        at: [i32; 3],
        /// The interior cell it stands in, by its editor ID; `None` outside.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cell: Option<String>,
        /// The plugin whose version of it the game uses.
        plugin: String,
        /// The door it led to; `None` where it led nowhere before.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        from: Option<ObjectRef>,
        /// The door it is to lead to.
        to: ObjectRef,
        /// Where the player is to arrive, in game units, if that moves.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        arrival: Option<[i32; 3]>,
        /// Which way the player is to face there, in thousandths of a
        /// radian, if that changes.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        facing: Option<[i32; 3]>,
    },
    /// A door triangle of a navigation mesh (`NVNM`) names `to` instead of
    /// `from`.
    NavmeshDoor {
        /// The navigation mesh.
        navmesh: ObjectRef,
        /// The plugin whose version of it the game uses.
        plugin: String,
        /// The triangle, by its number in the mesh.
        triangle: u32,
        /// The door it named.
        from: ObjectRef,
        /// The door it is to name.
        to: ObjectRef,
    },
}
