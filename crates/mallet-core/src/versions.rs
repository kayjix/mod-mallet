//! What a scan remembers about versions, and what changed between two
//! scans (ADR 0079): the game, its script extender, the extender's plugins
//! and the mods.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ModRef;

/// The versions one scan saw, kept with its report so that the next scan
/// can say what changed.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct VersionSnapshot {
    /// The game's version as its program told it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game: Option<String>,
    /// The game's version the script extender logged at the last start of
    /// the game, which is older than [`VersionSnapshot::game`] when the
    /// game was updated and not started since.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_started: Option<String>,
    /// The script extender's version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extender: Option<String>,
    /// The script extender's plugins with their file versions, by file
    /// name.
    #[serde(default)]
    pub plugins: Vec<PluginVersion>,
}

/// A script extender plugin and its version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PluginVersion {
    /// Its file name.
    pub file: String,
    /// Its file version, if it has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// One thing that went from one version to another.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct VersionStep {
    /// The version before.
    pub from: String,
    /// The version now.
    pub to: String,
}

/// A script extender plugin that went from one version to another.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PluginStep {
    /// Its file name.
    pub file: String,
    /// The version before.
    pub from: String,
    /// The version now.
    pub to: String,
}

/// A mod that went from one version to another.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ModStep {
    /// The mod.
    #[serde(rename = "mod")]
    pub mod_ref: ModRef,
    /// The version before.
    pub from: String,
    /// The version now.
    pub to: String,
}

/// What changed in the versions of a setup since an earlier scan, and what
/// of it no longer fits (ADR 0079).
///
/// A change is kept from scan to scan while something it broke is still
/// broken, and for [`VersionChanges::KEPT_FOR`] seconds after it was first
/// noticed, so that scanning again does not hide it at once.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct VersionChanges {
    /// When the scan ran that saw the versions before, in seconds since
    /// 1970-01-01 UTC; `0` when the earlier versions come from the script
    /// extender's log instead of a scan.
    pub since: u64,
    /// When a scan first noticed the change.
    pub noticed: u64,
    /// The game's update, if it was updated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game: Option<VersionStep>,
    /// The script extender's update, if it was updated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extender: Option<VersionStep>,
    /// The script extender plugins whose files changed version.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub plugins: Vec<PluginStep>,
    /// The mods the mod manager updated.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mods: Vec<ModStep>,
    /// The script extender plugins that do not fit the setup now, by file
    /// name: filled in from the findings of this scan.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unfit: Vec<String>,
    /// Whether the script extender, or the Address Library most of its
    /// plugins need, does not support the game version now, so that more
    /// than the plugins named in [`VersionChanges::unfit`] stops working.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub foundation_unfit: bool,
}

impl VersionChanges {
    /// How long a change is kept after it was first noticed once nothing it
    /// broke is broken any more: a week.
    pub const KEPT_FOR: u64 = 7 * 24 * 60 * 60;

    /// Whether anything changed at all.
    pub fn is_empty(&self) -> bool {
        self.game.is_none()
            && self.extender.is_none()
            && self.plugins.is_empty()
            && self.mods.is_empty()
    }
}
