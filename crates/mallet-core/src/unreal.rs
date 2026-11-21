//! Mods for the Unreal Engine side of a game: Oblivion Remastered runs the
//! old game's logic on its plugins, and draws with Unreal Engine, which
//! loads mods of its own packed as `.pak` files (ADR 0065).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ModRef;

/// The folder below the game's `Content/Paks` an Unreal mod lies in.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum PakFolder {
    /// `~mods`: packages that replace the game's own assets, such as
    /// models, textures or menus.
    Mods,
    /// `LogicMods`: packages with game logic of their own, loaded by
    /// UE4SS's mod loader.
    LogicMods,
}

/// One Unreal package a mod brought: the `.pak` file and the files that
/// belong to it (`.utoc`, `.ucas`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UnrealPak {
    /// The package, below the directory the game is installed in, with
    /// `/` between folders and as spelled on disk; without the extension.
    pub path: String,
    /// The folder it lies in.
    pub folder: PakFolder,
    /// The extensions of its files, in lower case, such as `pak`, `ucas`,
    /// `utoc`.
    pub extensions: Vec<String>,
    /// The size of its files together, in bytes.
    pub bytes: u64,
    /// The mod its `.pak` file came from, if the mod manager knows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<ModRef>,
}
