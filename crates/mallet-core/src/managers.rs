//! The mod managers Mod Mallet can read, and which one the user chose
//! (ADR 0009, ADR 0064).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The identifier of Vortex.
pub const VORTEX: &str = "vortex";

/// The identifier of Mod Organizer 2.
pub const MOD_ORGANIZER: &str = "mo2";

/// The mod managers Mod Mallet knows, which of them are on this machine,
/// and which one looks after the game the user looks at.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ManagerChoice {
    /// The manager in use, by its identifier: the one the user chose, or
    /// else the one suggested.
    pub active: String,
    /// Whether the user chose it. If not, it is a suggestion.
    pub chosen: bool,
    /// Every manager Mod Mallet can read.
    pub managers: Vec<KnownManager>,
}

/// A mod manager Mod Mallet can read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct KnownManager {
    /// Its identifier: `vortex` or `mo2`.
    pub id: String,
    /// Its name, as it calls itself.
    pub name: String,
    /// Whether it was found on this machine, without opening any of its
    /// files.
    pub found: bool,
    /// Whether Mod Mallet can hand it actions through a link. Without one,
    /// actions are described for the user to make by hand.
    pub link: bool,
    /// Its instances that were found, for a manager that has them (Mod
    /// Organizer 2). Portable instances are only known if the user named
    /// one.
    pub instances: Vec<KnownInstance>,
}

/// An instance of a mod manager: one setup it manages, in a folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct KnownInstance {
    /// Its name: the folder's name.
    pub name: String,
    /// The folder.
    pub path: String,
    /// Whether it is portable: kept anywhere, beside the program.
    pub portable: bool,
}
