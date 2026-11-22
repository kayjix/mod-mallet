//! Patches of Mod Mallet: what they hide, in parts, and who made them.
//!
//! A patch carries no files of anybody's mod, only instructions, so anybody
//! may make one and pass it on. It is made of parts, each about the plugin
//! whose objects it changes, so that a setup takes only the parts that
//! apply to it (ADR 0053).

use std::collections::BTreeSet;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ObjectRef;

/// The name a patch has until the user gives it another.
pub const DEFAULT_PATCH_NAME: &str = "Mod Mallet Patch";

/// A patch as it is made and passed on: what it hides, in parts, and,
/// since patches are mods of their own (ADR 0070), what it moves and which
/// paths it repairs, with the manifest that lets its decisions be read
/// back.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Patch {
    /// Its name. Patches with different names can be installed side by
    /// side; one with the same name replaces the one before.
    pub name: String,
    /// Who made it; `None` for the user's own. Plain text: nothing proves
    /// it yet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// Its version, as its maker numbers it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Its parts.
    pub parts: Vec<PatchPart>,
    /// What it moves, and the doors it merges (ADR 0060, 0067), written
    /// as its own plugin of moves.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub moves: Option<crate::moves::Moves>,
    /// The paths it repairs (ADR 0058), written as its own plugin of paths.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repairs: Option<crate::walkway_repair::PathRepairs>,
    /// Its decisions, for reading it back into Mod Mallet (ADR 0070).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest: Option<PatchManifest>,
}

/// The version of the manifest's format that this Mod Mallet writes and
/// reads. A manifest of another format is refused, not guessed at.
pub const PATCH_MANIFEST_FORMAT: u32 = 1;

/// The file name of the manifest at the top of a patch's archive. The
/// installer does not copy it into the game.
pub const PATCH_MANIFEST_FILE: &str = "mod-mallet-patch.json";

/// Everything needed to rebuild a patch's decisions in Mod Mallet: carried
/// in its archive, so that a patch can be loaded, changed and installed
/// again, by its maker or anybody else (ADR 0070).
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PatchManifest {
    /// The format of the manifest, [`PATCH_MANIFEST_FORMAT`] when written.
    pub format: u32,
    /// The patch's name.
    pub name: String,
    /// Who made it; `None` for a patch its maker kept for themselves.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// Its version, as its maker numbers it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// The decisions, by the keys the desktop app makes for them: a
    /// collision finding, a spot, an object, a move, a repaired navigation
    /// mesh, a duel, a version of a thing. The keys do not depend on the
    /// load order (ADR 0007).
    pub decisions: std::collections::BTreeMap<String, Option<String>>,
    /// The plugins the decisions are about, by file name.
    #[serde(default)]
    pub plugins: Vec<String>,
    /// The mods the decisions are about, by the names the maker's mod
    /// manager showed for them.
    #[serde(default)]
    pub mods: Vec<String>,
}

/// The user's patches for one game, as the window keeps them in the
/// settings (ADR 0070). The keys of decisions are the window's; the engine
/// keeps them as they are.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PatchBook {
    /// The patches, in the order in which they count: on the same decision
    /// the later one wins.
    pub patches: Vec<BookPatch>,
    /// The identity of the patch new decisions go to, one of the user's
    /// own.
    pub active: String,
    /// The spots pinned to solve later, by their keys among the decisions
    /// (`pin:…`). A pin is no decision and belongs to no patch (ADR 0050).
    #[serde(default)]
    pub pins: Vec<String>,
}

/// One patch of the [`PatchBook`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct BookPatch {
    /// Its identity, which stays when it is renamed.
    pub id: String,
    /// Its name, which names its archive and its mod.
    pub name: String,
    /// Who made it, if they said.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// Its version, as its maker numbers it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// The archive it was loaded from; `None` for a patch made here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loaded_from: Option<String>,
    /// Whether it is somebody else's: loaded, and not made one's own. The
    /// window never changes such a patch's decisions.
    #[serde(default)]
    pub foreign: bool,
    /// Whether the user switched it off: its decisions are kept but do not
    /// count.
    #[serde(default)]
    pub off: bool,
    /// Its decisions, by key, as in [`PatchManifest::decisions`].
    pub decisions: std::collections::BTreeMap<String, Option<String>>,
    /// The keys of its decisions that are unticked: kept, but not counted
    /// and not written.
    #[serde(default)]
    pub unticked: Vec<String>,
}

/// A patch read from an archive: its manifest if it has one, and what its
/// swap files hide, for an archive without one.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LoadedPatch {
    /// The archive's file name, as the user picked it.
    pub archive: String,
    /// The manifest; `None` if the archive has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest: Option<PatchManifest>,
    /// The patch its swap files describe: name, author, version and parts.
    pub patch: Patch,
}

impl Patch {
    /// Every object the patch hides, each once, in a fixed order.
    pub fn objects(&self) -> Vec<ObjectRef> {
        self.parts
            .iter()
            .flat_map(|part| part.objects.iter().cloned())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }
}

/// One part of a patch: what it hides of one plugin, and the plugin that
/// was the reason, if any.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PatchPart {
    /// The plugin whose objects the part hides, by its file name.
    pub plugin: String,
    /// The plugin the part was decided against, as in a collision; `None`
    /// for objects hidden for their own sake. A part with one applies only
    /// where both plugins are active.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub against: Option<String>,
    /// The objects it hides.
    pub objects: Vec<ObjectRef>,
}

/// A patch of Mod Mallet that is in the game, with the parts of it that
/// were installed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PatchInGame {
    /// The patch, with the parts that are installed. A patch written before
    /// patches had parts has one part per plugin, against none.
    pub patch: Patch,
    /// The mod its files belong to, if the mod manager knows it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<crate::ModRef>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn objects_are_listed_once() {
        let object = |local_id| ObjectRef {
            plugin: "A.esp".to_owned(),
            local_id,
        };
        let patch = Patch {
            name: DEFAULT_PATCH_NAME.to_owned(),
            author: None,
            version: None,
            parts: vec![
                PatchPart {
                    plugin: "A.esp".to_owned(),
                    against: Some("B.esp".to_owned()),
                    objects: vec![object(2), object(1)],
                },
                PatchPart {
                    plugin: "A.esp".to_owned(),
                    against: None,
                    objects: vec![object(2)],
                },
            ],
            ..Patch::default()
        };
        assert_eq!(patch.objects(), [object(1), object(2)]);
    }
}
