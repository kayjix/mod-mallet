//! Duels: everything two mods disagree about, as one decision.
//!
//! A large setup has hundreds of findings, but most of them are the same
//! few pairs of mods getting in each other's way in several places: the
//! faces, the textures, the objects in a town. Deciding pair by pair, and
//! for a pair as a whole, keeps each mod in one piece; deciding thing by
//! thing would tear mods apart without anyone meaning to.
//!
//! Like findings, duels carry no prose.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{Action, FindingId, ModRef};

/// What two mods disagree about.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum Topic {
    /// The faces of characters.
    Faces,
    /// Textures, from loose files or archives.
    Textures,
    /// Meshes and the textures they use.
    Meshes,
    /// Objects placed in the world.
    Objects,
    /// One mod's plugin contained in the other.
    Contained,
}

/// One of the two mods of a duel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DuelSide {
    /// The mod, if a mod manager knows it.
    #[serde(rename = "mod")]
    pub mod_ref: Option<ModRef>,
    /// What to call this side: the mod's name, or the file's.
    pub name: String,
    /// The plugins of this side that the duel's findings name.
    #[serde(default)]
    pub plugins: Vec<String>,
}

/// How much two mods disagree about one topic, and who has it now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DuelTopic {
    /// The topic.
    pub topic: Topic,
    /// How many things of it are disputed.
    pub count: u32,
    /// Which side the game takes them from now, 0 or 1; `None` where
    /// neither wins, as with objects standing in each other.
    pub winning: Option<u8>,
}

/// Why one side is suggested.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum SuggestionReason {
    /// One side's plugin is built on the other's: made to change it.
    BuiltOn,
    /// Most of what one side disagrees about with any mod is this topic;
    /// for the other side it is a small part. The specialist wins.
    Specialist,
}

/// The side Mod Mallet would pick, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DuelSuggestion {
    /// The side, 0 or 1.
    pub side: u8,
    /// Why.
    pub reason: SuggestionReason,
    /// The topic the reason is about, where it is about one.
    pub topic: Option<Topic>,
}

/// One file the two mods disagree about, to look at.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DuelFile {
    /// The file, below the data directory (`meshes/...`, `textures/...`).
    pub path: String,
    /// What the disagreement about it is.
    pub topic: Topic,
    /// The name of an object that uses it, if one is known: the name
    /// players see, or else the internal one.
    pub object: Option<String>,
    /// What kind of object that is, such as `static` or `container`.
    pub object_kind: Option<String>,
}

/// What makes one side win one finding of the duel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DuelResolution {
    /// The side it makes win, 0 or 1.
    pub side: u8,
    /// The finding it settles.
    pub finding: FindingId,
    /// The action to hand to the mod manager.
    pub action: Action,
}

/// Everything two mods disagree about.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ModDuel {
    /// Stable identity: the two sides, in an order that does not depend on
    /// the load order.
    pub id: String,
    /// The two mods, sorted by name.
    pub sides: [DuelSide; 2],
    /// What they disagree about, most first.
    pub topics: Vec<DuelTopic>,
    /// The findings it gathers.
    pub findings: Vec<FindingId>,
    /// The files the findings name, to look at side by side.
    #[serde(default)]
    pub files: Vec<DuelFile>,
    /// The side Mod Mallet would pick, if it has a reason to.
    pub suggestion: Option<DuelSuggestion>,
    /// The actions that make one side or the other win. Objects are hidden
    /// through the patch, which the window handles.
    pub resolutions: Vec<DuelResolution>,
}
