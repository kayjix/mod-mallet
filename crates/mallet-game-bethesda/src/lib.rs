//! Mod Mallet's adapter for the games of the Bethesda family: Skyrim
//! Special Edition, Skyrim VR, Skyrim, Fallout 4, Fallout 4 VR, Fallout:
//! New Vegas, Fallout 3, Oblivion and Oblivion Remastered.
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
//! * [`anchors`] finds what ties placed objects to the game (ADR 0046).
//! * [`ingest`] reads the load order, the data directory, the archives,
//!   the plugins, and the meshes that exist both loose and in an archive.

pub mod anchors;
mod archive;
mod archive_meshes;
pub mod archives;
pub mod area;
pub mod catalogue;
pub mod crash_logs;
pub mod files;
pub mod game;
pub mod ingest;
pub mod land_textures;
pub mod locate;
pub mod lod;
pub mod lod_library;
pub mod loose_meshes;
pub mod markers;
pub mod meshes;
pub mod models;
pub mod navmeshes;
pub mod npc;
pub mod objects;
pub mod place_kinds;
pub mod skse_log;
pub mod swap;
pub mod terrain;
pub mod tools;
pub mod unreal;
pub mod walkways;

pub use game::Game;

/// The directory that holds plugins, archives and loose files, relative to
/// the game directory.
pub const DATA_DIR: &str = "data";
