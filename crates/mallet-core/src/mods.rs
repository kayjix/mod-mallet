//! The seam between Mod Mallet and a mod manager.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::GamePath;

/// A mod as a mod manager knows it.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "camelCase")]
pub struct ModRef {
    /// The manager that knows this mod, for example `vortex`.
    pub source: String,
    /// The manager's own identifier of the mod.
    pub id: String,
    /// The name to show to people.
    pub name: String,
}

/// What Mod Mallet needs to know from a mod manager.
///
/// Analyses only ever see this trait, never a concrete manager (ADR 0009).
/// It grows one question at a time, as analyses need answers.
pub trait ModSource {
    /// The mod that put the file at `path` into the game, if the manager
    /// deployed it.
    fn provider_of(&self, path: &GamePath) -> Option<ModRef>;
}

/// The [`ModSource`] of a setup that no known mod manager looks after.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NoModSource;

impl ModSource for NoModSource {
    fn provider_of(&self, _path: &GamePath) -> Option<ModRef> {
        None
    }
}
