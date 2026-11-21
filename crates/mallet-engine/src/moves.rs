//! The plugin that moves placed objects a little (ADR 0060).
//!
//! It is written into Mod Mallet's own folder, next to the patches, as an
//! archive the mod manager installs like a downloaded mod. Nothing is
//! written into the game or the mod manager's data; installing it is an
//! action the user starts.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;

use mallet_core::moves::{CreatedMoves, MOVES_NAME, Moves};
use mallet_game_bethesda::catalogue::string_table;
use mallet_game_bethesda::lod_library::LodLibrary;
use mallet_source_vortex::link::Link;

use crate::{EngineError, ManagerOptions, ScanOptions};

/// The language whose string tables names are taken from first, where a
/// localized plugin keeps them there; else any the setup has.
const NAMES_LANGUAGE: &str = "english";

/// Where the archive of the moves is written: next to the patches.
pub fn moves_file(options: &ManagerOptions) -> Option<PathBuf> {
    Some(
        Link::new(options.link_dir()?)
            .patches_dir()
            .join(format!("{MOVES_NAME}.zip")),
    )
}

/// Writes the plugin that applies `moves` to the setup `scan` describes,
/// in an archive in Mod Mallet's own folder, replacing the one written
/// before. Objects that cannot be moved are listed in the result; if none
/// can, nothing is written and the result's path is empty. Nothing is
/// installed: that is [`mallet_core::Action::MoveObjects`].
///
/// # Errors
///
/// If the setup or Mod Mallet's folder cannot be found, the plugin cannot
/// be made, or the archive cannot be written.
pub fn create_moves(
    scan: &ScanOptions,
    moves: &Moves,
    options: &ManagerOptions,
) -> Result<CreatedMoves, EngineError> {
    let started = std::time::Instant::now();
    let game = scan.game()?;
    let (game_dir, _) = scan.located_game_dir().ok_or(EngineError::GameNotFound)?;
    let own = format!("{MOVES_NAME}.esp");
    // The moves written before are not read: the new plugin replaces them
    // and must not copy from itself.
    let plugins = mallet_game_bethesda::moves::active_plugins(
        game,
        &game_dir,
        scan.local_dir.as_deref(),
        &own,
    );
    let tables = LodLibrary::with_prefixes(
        game,
        &game_dir,
        scan.local_dir.as_deref(),
        &["strings/".to_owned()],
    );
    let read: RefCell<HashMap<String, HashMap<u32, String>>> = RefCell::new(HashMap::new());
    let names = |plugin: &str, id: u32| -> Option<String> {
        let key = plugin.to_ascii_lowercase();
        let mut read = read.borrow_mut();
        read.entry(key)
            .or_insert_with(|| string_table(&tables, plugin, NAMES_LANGUAGE))
            .get(&id)
            .cloned()
    };
    let built = mallet_game_bethesda::moves::build(game, &plugins, &moves.moves, &names)
        .map_err(|error| EngineError::MovesNotWritten(error.to_string()))?;
    let count = u32::try_from(built.moved.len()).unwrap_or(u32::MAX);
    let Some(bytes) = built.bytes else {
        tracing::info!(
            refused = built.refused.len(),
            "no object could be moved; nothing written"
        );
        return Ok(CreatedMoves {
            path: String::new(),
            moved: count,
            masters: built.masters,
            light: built.light,
            refused: built.refused,
        });
    };
    let archive = moves_file(options).ok_or(EngineError::ManagerNotFound)?;
    crate::patch::write_zip(&archive, &[(own, bytes)]).map_err(|source| EngineError::Write {
        path: archive.clone(),
        source,
    })?;
    tracing::info!(
        path = %archive.display(),
        moved = count,
        refused = built.refused.len(),
        masters = built.masters.len(),
        light = built.light,
        elapsed_ms = started.elapsed().as_millis(),
        "moves written"
    );
    for refused in &built.refused {
        tracing::info!(
            plugin = refused.object.plugin,
            local_id = format!("{:06X}", refused.object.local_id),
            reason = ?refused.reason,
            "object not moved"
        );
    }
    Ok(CreatedMoves {
        path: archive.display().to_string(),
        moved: count,
        masters: built.masters,
        light: built.light,
        refused: built.refused,
    })
}
