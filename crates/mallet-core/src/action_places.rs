//! The places where characters do something (ADR 0069): furniture to sit,
//! lean, sleep or work at, and idle markers where they stand and do a
//! thing, such as chopping wood or sweeping. Mods such as those that give
//! a city's people new routines add many of them.
//!
//! They can be hidden through the patch like any placed object, and moved
//! through the plugin of moves (ADR 0060). Whether characters can reach
//! one is told by the paths they walk on (ADR 0059).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ObjectRef;

/// What characters do at a place.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum ActionKind {
    /// Sit: a chair, a bench, a stool, a spot on the ground.
    Sit,
    /// Lean against a wall, a railing or a counter.
    Lean,
    /// Sleep: a bed or a bedroll.
    Sleep,
    /// Chop wood at a chopping block.
    ChopWood,
    /// Mine ore with a pickaxe.
    Mine,
    /// Work at a forge, an anvil, a smelter, a tanning rack, a grindstone
    /// or a workbench.
    Smith,
    /// Mix potions or enchant at a table.
    Craft,
    /// Cook at a pot, a spit or an oven, or churn butter.
    Cook,
    /// Pray or kneel.
    Pray,
    /// Other work: sweep, rake, farm, hammer, wash.
    Work,
    /// Stand and do something not told apart: a spot of an idle marker.
    Stand,
    /// Other furniture.
    Other,
}

/// One place where characters do something.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ActionPlace {
    /// The placed object: the plugin that placed it first and its number
    /// there.
    pub object: ObjectRef,
    /// The plugin whose version the game uses.
    pub plugin: String,
    /// What characters do there.
    pub kind: ActionKind,
    /// Whether it is an idle marker rather than furniture.
    pub idle_marker: bool,
    /// Whether the game draws it: a chair or a forge is seen, a marker to
    /// lean or sit on the ground is not.
    pub drawn: bool,
    /// The name players see, where there is one.
    pub name: Option<String>,
    /// The internal name of what is placed, such as `WallLeanMarker`.
    pub base: Option<String>,
    /// Where, in game units.
    pub position: [f32; 3],
    /// Turned about the x, y and z axes, in radians; the turn about z is
    /// the way characters face there.
    pub rotation: [f32; 3],
}

/// Whether characters can reach places where they do something, with the
/// paths as they are or with objects hidden (ADR 0069).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SpotsReach {
    /// The worldspace or interior cell asked about.
    pub place: String,
    /// Each spot asked about, in the order asked.
    pub spots: Vec<SpotReach>,
}

/// A spot asked about: an object, and where it stands or would stand.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SpotAsked {
    /// The object, such as a marker to lean on.
    pub object: ObjectRef,
    /// Where it stands, or would stand after a move, in whole game units.
    pub at: [i32; 3],
}

/// Whether characters can reach one spot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SpotReach {
    /// The object asked about.
    pub object: ObjectRef,
    /// Where, in whole game units.
    pub at: [i32; 3],
    /// Whether they can.
    pub state: SpotState,
    /// How far the spot stands from the nearest path characters can
    /// reach, in whole metres; `None` if none is near.
    pub metres_off: Option<u32>,
    /// The nearest point on paths characters can reach, within about 30
    /// metres, in whole game units: where the spot could be moved to.
    pub ground: Option<[i32; 3]>,
}

/// Whether characters can reach a spot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum SpotState {
    /// A path characters reach leads to it.
    Reached,
    /// It stands on paths, but no way leads there from the rest of the
    /// world.
    CutOff,
    /// No path is near enough to use it from.
    OffPaths,
    /// The paths there could not be read.
    Unknown,
}
