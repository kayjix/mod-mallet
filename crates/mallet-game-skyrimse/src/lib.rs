//! Mod Mallet's adapter for The Elder Scrolls V: Skyrim Special Edition.
//!
//! The adapter knows where this game keeps its files and what they mean. It
//! turns them into the game-neutral facts of `mallet-core`.
//!
//! Pure parts, which take text or parsed data and can be tested without a
//! disk:
//!
//! * [`skse_log`] reads the log of SKSE, the game's script extender.
//! * [`archives`] decides which archives the game loads, and in which order.
//! * [`npc`] extracts what determines a character's face from a plugin
//!   record.
//! * [`meshes::texture_path`] turns a texture path from a mesh into the
//!   path the game looks under.
//!
//! Parts that touch the system:
//!
//! * [`locate`] finds the game and its files on this machine.
//! * [`ingest`] reads the load order, the data directory, the archives,
//!   the plugins, and the meshes that exist both loose and in an archive.

pub mod archives;
pub mod files;
pub mod ingest;
pub mod locate;
pub mod lod;
pub mod markers;
pub mod meshes;
pub mod npc;
pub mod objects;
pub mod skse_log;
pub mod swap;
pub mod terrain;
pub mod tools;

/// Identifier of this adapter, used as [`mallet_core::GameInfo::id`].
pub const GAME_ID: &str = "skyrimse";

/// The game's name as shown to people.
pub const GAME_NAME: &str = "Skyrim Special Edition";

/// The directory that holds plugins, archives and loose files, relative to
/// the game directory.
pub const DATA_DIR: &str = "data";

/// The directory SKSE loads plugins from, relative to the game directory.
pub const EXTENDER_PLUGIN_DIR: &str = "data/skse/plugins";
