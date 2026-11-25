//! The game's list of active plugins, when the game itself rewrote it
//! (ADR 0089).
//!
//! Skyrim Special Edition and the games like it keep the list of active
//! plugins in `Plugins.txt`. The mod manager writes it before the game
//! starts. When the game finds a plugin it cannot load, such as one whose
//! master is not there, it switches the plugin off and writes the file
//! again, with a header of its own. The mod manager then finds a list it
//! did not write and says another program changed the load order.

use crate::ModRef;

/// The list of active plugins as the game last wrote it, and the plugins
/// the game switched off that the mod manager had on.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GameRewrite {
    /// The game's name as players know it, such as `Skyrim Special
    /// Edition`.
    pub game: String,
    /// The mod manager whose list the game replaced, such as `vortex`;
    /// `None` without one.
    pub manager: Option<String>,
    /// The plugins the game switched off, in the mod manager's load order
    /// where it is known.
    pub switched_off: Vec<SwitchedOffByGame>,
}

/// A plugin the game switched off when it started.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwitchedOffByGame {
    /// The plugin's file name, as the list spells it.
    pub plugin: String,
    /// The mod that brings it, where the mod manager tells.
    pub mod_ref: Option<ModRef>,
    /// Why the game could not load it.
    pub reason: OffReason,
}

/// Why the game switched a plugin off.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OffReason {
    /// Plugins it builds on are not loaded.
    MissingMasters(Vec<MissingMaster>),
    /// The file is not in the data folder.
    NotFound,
    /// The file does not start like a plugin of the game.
    Unreadable,
    /// More plugins are on than the game can load.
    TooMany,
    /// None of the above could be told.
    Unknown,
}

/// A master of a plugin that the game does not load.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingMaster {
    /// The master's file name, as the plugin names it.
    pub plugin: String,
    /// Whether its file lies in the data folder: it is there but off.
    pub present: bool,
    /// The mod that has it and that the mod manager keeps switched off,
    /// where there is one.
    pub switched_off_mod: Option<ModRef>,
}
