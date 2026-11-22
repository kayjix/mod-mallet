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

    /// Every file the manager keeps for `mod_ref`, deployed or not, by the
    /// path the game would see it under. `None` if the manager does not
    /// say. A file the manager kept back, because the file of another mod
    /// wins, is among them.
    fn staged_files(&self, mod_ref: &ModRef) -> Option<Vec<GamePath>> {
        let _ = mod_ref;
        None
    }

    /// The mods the manager keeps installed but switched off in the profile
    /// in use that bring a file at `path`, by the path the game would see it
    /// under. Empty if there are none or the manager does not say.
    fn switched_off_with(&self, path: &GamePath) -> Vec<ModRef> {
        let _ = path;
        Vec::new()
    }

    /// Every file the mods that are installed but switched off in the
    /// profile in use bring, by the path the game would see it under, with
    /// the mod. Empty if there are none or the manager does not say.
    fn switched_off_files(&self) -> Vec<(GamePath, ModRef)> {
        Vec::new()
    }
}

/// A plugin of a mod the mod manager keeps switched off (ADR 0061).
///
/// Usually the game no longer loads it, and the decisions about its
/// collisions wait until the mod is on again. The game still loads it while
/// the manager has not deployed since the mod went off, or when a copy the
/// manager did not put there is still in the data directory and active.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "camelCase")]
pub struct SwitchedOffPlugin {
    /// The plugin's file name.
    pub plugin: String,
    /// The switched-off mod that brings it.
    #[serde(rename = "mod")]
    pub mod_ref: ModRef,
    /// Whether the game loads the plugin all the same.
    pub loaded: bool,
    /// The files of the plugin's name in the data directory that no mod
    /// manager put there: the plugin and the archives named after it, left
    /// behind when the mod went (ADR 0073). Empty where the mod manager
    /// still has its own copies there, or nothing is left.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub left_behind: Vec<String>,
}

/// The [`ModSource`] of a setup that no known mod manager looks after.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NoModSource;

impl ModSource for NoModSource {
    fn provider_of(&self, _path: &GamePath) -> Option<ModRef> {
        None
    }
}
