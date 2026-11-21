//! Two mods' versions of files that are neither textures nor meshes, side
//! by side (ADR 0066): scripts, menus, settings, sounds, animations. What
//! can be shown of them is their text, their size and date, whether the
//! bytes are the same, and for scripts the functions they contain.
//!
//! Carries no prose: what a kind of file is for is the window's to say.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::ModRef;

/// Every version of one file the setup has, compared.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FileComparison {
    /// The file, below the data directory.
    pub path: String,
    /// Its versions: the copies in archives in the order the game loads
    /// them, then the loose file the game uses, then the loose copies a
    /// mod manager keeps back.
    pub versions: Vec<ComparedVersion>,
}

/// One version of a file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ComparedVersion {
    /// The archive it is in, or `None` for a loose file.
    pub archive: Option<String>,
    /// Whether it is a loose copy that lies only in the mod manager's own
    /// folder, because another mod's copy is in the game instead.
    pub staged: bool,
    /// The mod it comes from, if the mod manager knows.
    pub provider: Option<ModRef>,
    /// Its size in bytes.
    pub size: u64,
    /// When the file, or the archive it is in, was last written, in
    /// seconds since 1970; `None` where it cannot be told.
    pub modified: Option<u64>,
    /// The position of the first earlier version with exactly the same
    /// bytes, if there is one.
    pub same_as: Option<u32>,
    /// The content, for files that are text, up to a limit.
    pub text: Option<String>,
    /// Whether the text was cut at that limit.
    #[serde(default)]
    pub text_cut: bool,
    /// The picture, a PNG or JPEG as a `data:` address, for pictures other
    /// than textures.
    pub image: Option<String>,
    /// What a compiled script says about itself.
    pub script: Option<ScriptSummary>,
}

/// The name and functions of a compiled script.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ScriptSummary {
    /// The script's name.
    pub name: String,
    /// Whether the script carries the names of its functions; without
    /// them, `functions` is empty.
    pub has_debug_info: bool,
    /// Its functions.
    pub functions: Vec<ScriptFunction>,
}

/// One function of a compiled script.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ScriptFunction {
    /// The state it belongs to; empty for the default state.
    pub state: String,
    /// Its name; for a property's getter or setter, the property's.
    pub name: String,
    /// What kind of function it is.
    pub kind: ScriptFunctionKind,
    /// Stands for the shape of its compiled code: two versions whose
    /// fingerprints differ do something different.
    pub fingerprint: String,
}

/// What kind of function a script function is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ScriptFunctionKind {
    /// An ordinary function or event.
    Plain,
    /// What reads a property.
    Getter,
    /// What sets a property.
    Setter,
}
