//! What community-maintained rules say about the plugins of a setup.
//!
//! The rules come from the LOOT masterlist (ADR 0008): which files a
//! plugin needs, which it cannot be used with, and which versions of a
//! plugin contain records that should be cleaned out. The adapter that
//! reads them has already checked each rule's condition against the setup,
//! so only rules that apply to this setup are listed here.

use crate::GamePath;

/// The rules that apply to the active plugins of a setup.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KnowledgeFacts {
    /// One entry per active plugin the rules say something about, in load
    /// order.
    pub plugins: Vec<PluginKnowledge>,
}

/// What the rules say about one active plugin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginKnowledge {
    /// The plugin's file name in the spelling of the load order.
    pub plugin: String,
    /// Where the plugin lies.
    pub path: GamePath,
    /// Whether the plugin comes with the game, so that the game's updates
    /// replace it.
    pub shipped_with_game: bool,
    /// Files the plugin needs.
    pub requirements: Vec<KnownFile>,
    /// Files the plugin cannot be used together with.
    pub incompatibilities: Vec<KnownFile>,
    /// What a cleaning tool found in exactly this version of the plugin.
    /// Empty unless the plugin's checksum matches a version the rules know
    /// to need cleaning.
    pub dirty: Vec<Cleaning>,
    /// What LOOT tells about the plugin in this setup, in its own words.
    pub messages: Vec<KnownMessage>,
}

/// A message LOOT shows for a plugin, such as "a compatibility patch is
/// available". Messages are LOOT's prose; Mod Mallet quotes them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnownMessage {
    /// How much LOOT says it matters.
    pub level: MessageLevel,
    /// The message in English, as plain text. Links are written as their
    /// text followed by the address in brackets.
    pub text: String,
}

/// How much a message of LOOT matters, in LOOT's own three steps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MessageLevel {
    /// Worth knowing.
    Note,
    /// Something to look at.
    Warning,
    /// Something to fix.
    Error,
}

/// A file a rule names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnownFile {
    /// The file, relative to the game's data directory, as the rule names
    /// it. A path that starts with `../` lies in the game's directory.
    pub name: String,
    /// What the rule calls the file for people, usually the mod it comes
    /// with, where the rule names one.
    pub display: Option<String>,
    /// The page the rule links to for the file, where it names one.
    pub page: Option<String>,
    /// Whether the file is in the game.
    pub present: bool,
}

impl KnownFile {
    /// What to call the file in a message: the rule's name for it, or else
    /// its file name.
    pub fn label(&self) -> &str {
        self.display.as_deref().unwrap_or(&self.name)
    }

    /// Whether the file is a plugin, which counts only while it is active.
    pub fn is_plugin(&self) -> bool {
        let lower = self.name.to_lowercase();
        [".esp", ".esm", ".esl"]
            .iter()
            .any(|extension| lower.ends_with(extension))
    }
}

/// What a cleaning tool found in one version of a plugin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cleaning {
    /// Records that are identical to the version of a master and so only
    /// undo other mods' changes.
    pub identical_records: u32,
    /// Placed objects of a master that the plugin deletes instead of
    /// switching them off.
    pub deleted_references: u32,
    /// Navigation meshes of a master that the plugin deletes.
    pub deleted_navmeshes: u32,
    /// The tool the counts come from, as the rule names it.
    pub utility: String,
}
