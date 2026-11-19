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

/// A patch as it is made and passed on.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
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
        };
        assert_eq!(patch.objects(), [object(1), object(2)]);
    }
}
