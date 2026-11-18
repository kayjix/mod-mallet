//! The programs and files a setup needs besides the mods, and how they
//! stand: the script extender, its libraries, the mod manager, the tools
//! Mod Mallet itself uses.
//!
//! Like a finding, a tool carries no prose. Its `id` names it; the words
//! for it come from `mallet-messages`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// One program or file the setup needs or can use.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Tool {
    /// Fixed identifier, for example `skse` or `address-library`.
    pub id: String,
    /// What kind of tool it is, for showing the mod manager apart.
    #[serde(default)]
    pub group: ToolGroup,
    /// How much the setup depends on it.
    pub need: ToolNeed,
    /// How it stands right now.
    pub state: ToolState,
    /// Its version, where known.
    pub version: Option<String>,
    /// Where it was found, where known.
    pub path: Option<String>,
    /// Where to get it.
    pub page: Option<String>,
    /// The tool that brings this one along, when it is not installed on its
    /// own: `vortex` for the LOOT that Vortex has built in.
    #[serde(default)]
    pub provided_by: Option<String>,
}

/// What kind of tool a tool is.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ToolGroup {
    /// A mod manager, such as Vortex.
    ModManager,
    /// Mod Mallet's link into a mod manager.
    ManagerLink,
    /// Something the game needs, such as its script extender.
    #[default]
    Game,
    /// A program Mod Mallet or the user works with, such as 7-Zip.
    Helper,
}

/// How much a setup depends on a tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ToolNeed {
    /// Without it, mods that rely on it do not work.
    Required,
    /// Most setups use it; some of Mod Mallet's checks or fixes need it.
    Recommended,
    /// Useful for some tasks, not needed to play.
    Optional,
}

/// How a tool stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ToolState {
    /// Present and fitting.
    Ready,
    /// Not found.
    Missing,
    /// Present, but not in the version the game or Mod Mallet needs.
    Outdated,
    /// Installed, but not running right now.
    Stopped,
}

impl Tool {
    /// A tool with nothing known about it but its state.
    pub fn new(id: &str, need: ToolNeed, state: ToolState) -> Self {
        Self {
            id: id.to_owned(),
            group: ToolGroup::Game,
            need,
            state,
            version: None,
            path: None,
            page: None,
            provided_by: None,
        }
    }

    /// The same tool in another group.
    #[must_use]
    pub fn in_group(mut self, group: ToolGroup) -> Self {
        self.group = group;
        self
    }

    /// The same tool with its version.
    #[must_use]
    pub fn with_version(mut self, version: Option<String>) -> Self {
        self.version = version;
        self
    }

    /// The same tool with the place it was found.
    #[must_use]
    pub fn with_path(mut self, path: Option<String>) -> Self {
        self.path = path;
        self
    }

    /// The same tool, brought along by the tool `provider`.
    #[must_use]
    pub fn provided_by(mut self, provider: &str) -> Self {
        self.provided_by = Some(provider.to_owned());
        self
    }

    /// The same tool with the page to get it from.
    #[must_use]
    pub fn with_page(mut self, page: &str) -> Self {
        self.page = Some(page.to_owned());
        self
    }

    /// Whether the user should do something about it: a required or
    /// recommended tool that is not ready.
    pub fn wants_attention(&self) -> bool {
        self.state != ToolState::Ready && self.need != ToolNeed::Optional
    }
}
