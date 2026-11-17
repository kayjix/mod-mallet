//! Mod Mallet's adapter for the games of the Bethesda family: Skyrim
//! Special Edition, Skyrim VR, Skyrim, Fallout 4, Fallout 4 VR, Fallout:
//! New Vegas, Fallout 3 and Oblivion.
//!
//! The adapter knows where these games keep their files and what they
//! mean. It turns them into the game-neutral facts of `mallet-core`. The
//! games share their formats and most of what their records mean; what
//! differs between them is in [`game`] (ADR 0036).
//!
//! Pure parts, which take text or parsed data and can be tested without a
//! disk:
//!
//! * [`game`] says what differs between the games.
//! * [`skse_log`] reads the log of the game's script extender.
//! * [`archives`] decides which archives the game loads, and in which order.
//! * [`npc`] extracts what determines a character's face from a plugin
//!   record.
//! * [`meshes::texture_path`] turns a texture path from a mesh into the
//!   path the game looks under.
//!
//! Parts that touch the system:
//!
//! * [`locate`] finds the games and their files on this machine.
//! * [`ingest`] reads the load order, the data directory, the archives,
//!   the plugins, and the meshes that exist both loose and in an archive.

mod archive;
pub mod archives;
pub mod files;
pub mod game;
pub mod ingest;
pub mod locate;
pub mod lod;
pub mod lod_library;
pub mod markers;
pub mod meshes;
pub mod navmeshes;
pub mod npc;
pub mod objects;
pub mod skse_log;
pub mod swap;
pub mod terrain;
pub mod tools;

pub use game::Game;

/// The directory that holds plugins, archives and loose files, relative to
/// the game directory.
pub const DATA_DIR: &str = "data";
