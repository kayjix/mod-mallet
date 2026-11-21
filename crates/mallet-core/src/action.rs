//! Actions: what can be done about a finding.
//!
//! Mod Mallet does not change a setup itself (ADR 0005). An action is a
//! request to the tool that owns the thing to be changed, started by the
//! user. A finding lists the actions that would address it; whoever shows
//! the finding offers them.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ModRef;

/// Something that can be done about a finding.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Action {
    /// Run a mod's installer again in the mod manager, so that the user can
    /// choose options that were not offered or not chosen before.
    ReinstallMod {
        /// The mod to reinstall.
        #[serde(rename = "mod")]
        mod_ref: ModRef,
        /// The options to tick in the installer, in the order the installer
        /// shows them. Empty if the reinstall is not about particular
        /// options.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        tick: Vec<OptionToTick>,
    },
    /// Hide placed objects in the game, through a patch that the mod
    /// manager installs: each object is swapped for an invisible marker
    /// while the game runs. Removing the patch brings them back. Its parts
    /// can be left out in its installer (ADR 0053).
    HideObjects {
        /// The patch, with the objects to hide in its parts.
        patch: crate::Patch,
    },
    /// Move placed objects a little, through a plugin of overrides that
    /// the mod manager installs (ADR 0060). Removing it puts them back.
    MoveObjects {
        /// The moves.
        moves: crate::moves::Moves,
    },
    /// Load a plugin after another one, through a rule in the mod
    /// manager's plugin list, so that its version of what both change wins.
    /// The rule can be removed there again.
    LoadAfter {
        /// The plugin that is to win.
        plugin: String,
        /// The plugin it is to load after.
        after: String,
    },
    /// Switch a mod off in the mod manager's profile in use. Switching it on
    /// again there undoes it.
    DisableMod {
        /// The mod to switch off.
        #[serde(rename = "mod")]
        mod_ref: ModRef,
    },
    /// Switch a mod on that is installed in the mod manager but switched
    /// off in the profile in use. Switching it off again there undoes it.
    EnableMod {
        /// The mod to switch on.
        #[serde(rename = "mod")]
        mod_ref: ModRef,
    },
    /// Decide whose copies of some loose files reach the game, through the
    /// mod manager's file overrides (ADR 0054): the mods in `leave_out` no
    /// longer deploy the files, those in `provide` deploy them again. With
    /// every mod in `leave_out` but one, that one's copies win; with every
    /// mod in `leave_out`, the copies in the archives show. Removing the
    /// overrides in the mod manager undoes it.
    FileOverrides {
        /// The files, below the data directory (`meshes/...`).
        files: Vec<String>,
        /// The mods that are to leave the files out.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        leave_out: Vec<ModRef>,
        /// The mods that are to deploy the files again.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        provide: Vec<ModRef>,
    },
}

/// A placed object, by the plugin that placed it first and its number in
/// that plugin. Unlike a form ID as the game shows it, this does not
/// depend on the load order.
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "camelCase")]
pub struct ObjectRef {
    /// The plugin's file name.
    pub plugin: String,
    /// The object's number in that plugin.
    pub local_id: u32,
}

/// An option of a mod's installer, with where to find it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct OptionToTick {
    /// The page of the installer the option is on, counting from 1, among
    /// the pages the installer shows with the choices made so far.
    pub page: u32,
    /// How many pages the installer shows.
    pub pages: u32,
    /// The title of the page.
    pub step: String,
    /// The title of the group of options on the page.
    pub group: String,
    /// The title of the option.
    pub option: String,
    /// The position of the option in its group, counting from 0, in the
    /// order the installer shows them.
    #[serde(default)]
    pub index: u32,
    /// What the installer says about the option, in the mod author's words.
    pub description: String,
}

/// What became of an action.
///
/// An action that was *started* is in the hands of the mod manager from
/// then on: the user may still have to go through a dialog there, and may
/// cancel it. Mod Mallet learns the result from the next scan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ActionStatus {
    /// The mod manager took the request and has begun.
    Started,
    /// The mod manager is installing the patch Mod Mallet wrote.
    Installing,
    /// The mod manager has made the change; it may still ask to deploy.
    Applied,
    /// The link between Mod Mallet and the mod manager is not installed.
    LinkMissing,
    /// The mod manager is not running, or the link in it is not answering.
    ManagerNotRunning,
    /// The mod manager took too long to answer.
    NoAnswer,
    /// The mod manager is managing a different game at the moment.
    WrongGame,
    /// The mod manager does not know the mod.
    UnknownMod,
    /// The archive the mod was installed from is no longer there.
    ArchiveMissing,
    /// The mod manager's link is of a version that does not know this
    /// request.
    Unsupported,
    /// Something else went wrong; see [`ActionOutcome::detail`].
    Failed,
}

/// The result of asking for an action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ActionOutcome {
    /// What became of the action.
    pub status: ActionStatus,
    /// Technical detail for a failure, in the words of whatever failed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl ActionOutcome {
    /// An outcome without further detail.
    pub fn new(status: ActionStatus) -> Self {
        Self {
            status,
            detail: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_action_serializes_with_its_type_and_mod() {
        let action = Action::ReinstallMod {
            mod_ref: ModRef {
                source: "vortex".to_owned(),
                id: "Patch Hub 1.0".to_owned(),
                name: "Patch Hub".to_owned(),
            },
            tick: Vec::new(),
        };
        let json = serde_json::to_value(&action).unwrap();
        assert_eq!(json["type"], "reinstall-mod");
        // Nothing to tick is not written, and reads back as nothing.
        assert!(json.get("tick").is_none());
        assert_eq!(json["mod"]["id"], "Patch Hub 1.0");
        assert_eq!(serde_json::from_value::<Action>(json).unwrap(), action);
    }

    #[test]
    fn an_outcome_leaves_out_missing_detail() {
        let json =
            serde_json::to_value(ActionOutcome::new(ActionStatus::ManagerNotRunning)).unwrap();
        assert_eq!(json["status"], "manager-not-running");
        assert!(json.get("detail").is_none());
    }
}
