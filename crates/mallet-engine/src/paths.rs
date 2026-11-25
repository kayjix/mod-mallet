//! The plugin that carries the repaired paths (ADR 0058).
//!
//! It is written into Mod Mallet's own folder, next to the patches and the
//! moves, as an archive the mod manager installs like a downloaded mod.
//! Nothing is written into the game or the mod manager's data; installing
//! it is an action the user starts.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use mallet_core::walkway_repair::{
    CreatedPaths, PATHS_NAME, PathRefusal, PathRepairs, RefusedPath,
};
use mallet_game_bethesda::catalogue::string_table;
use mallet_game_bethesda::lod_library::LodLibrary;
use mallet_game_bethesda::navmesh_records::NavmeshRecord;
use mallet_source_vortex::link::Link;

use crate::area::reader;
use crate::{EngineError, ManagerOptions, ScanOptions};

/// The language whose string tables names are taken from first.
const NAMES_LANGUAGE: &str = "english";

/// Where the archive of the paths is written: next to the patches.
pub fn paths_file(options: &ManagerOptions) -> Option<PathBuf> {
    Some(
        Link::new(options.link_dir()?)
            .patches_dir()
            .join(format!("{PATHS_NAME}.zip")),
    )
}

/// Builds the plugin, named `own`, that carries the repaired paths of
/// `repairs` into the setup `scan` describes, from the versions of the
/// meshes other than Mod Mallet's own repairs. Each place is repaired with
/// every object hidden there; the meshes chosen for it are kept, or, for a
/// place repaired by itself (ADR 0070), every mesh whose path the game uses
/// runs over a hidden object. Also returns the meshes that cannot be repaired, and why.
///
/// # Errors
///
/// If the setup cannot be found or the plugin cannot be made.
pub(crate) fn paths_plugin(
    scan: &ScanOptions,
    repairs: &PathRepairs,
    own: &str,
) -> Result<
    (
        mallet_game_bethesda::paths_plugin::PathsPlugin,
        Vec<RefusedPath>,
    ),
    EngineError,
> {
    let game = scan.game()?;
    let (game_dir, _) = scan.located_game_dir().ok_or(EngineError::GameNotFound)?;

    // The records of every place, each mesh once.
    let mut records: BTreeMap<String, NavmeshRecord> = BTreeMap::new();
    let mut refused: Vec<RefusedPath> = Vec::new();
    for place in &repairs.places {
        let centre = place.centre.map(|[x, y]| [as_f32(x), as_f32(y)]);
        // A place repaired by itself takes every mesh that needs it there.
        let chosen: std::collections::BTreeSet<String> = place.navmeshes.iter().cloned().collect();
        let found = crate::walkways::repaired_navmeshes(
            scan,
            &place.place,
            centre,
            &repairs.hidden,
            (!place.every).then_some(&chosen),
        )
        .unwrap_or_default();
        for record in found {
            records.entry(record.navmesh.to_string()).or_insert(record);
        }
    }
    for place in &repairs.places {
        for navmesh in &place.navmeshes {
            if !records.contains_key(navmesh)
                && !refused.iter().any(|known| &known.navmesh == navmesh)
            {
                refused.push(RefusedPath {
                    navmesh: navmesh.clone(),
                    reason: PathRefusal::NothingToRepair,
                });
            }
        }
    }

    let plugins = reader(scan)
        .and_then(|reader| Some(reader.lock().ok()?.without_paths()))
        .ok_or(EngineError::GameNotFound)?;
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
    let records: Vec<NavmeshRecord> = records.into_values().collect();
    let built = mallet_game_bethesda::paths_plugin::build(game, &plugins, &records, &names, own)
        .map_err(|error| EngineError::PathsNotWritten(error.to_string()))?;
    refused.extend(built.refused.iter().map(|(navmesh, reason)| RefusedPath {
        navmesh: navmesh.to_string(),
        reason: reason.clone(),
    }));
    Ok((built, refused))
}

/// Writes the plugin that carries the repaired paths of `repairs` into the
/// setup `scan` describes, in an archive in Mod Mallet's own folder,
/// replacing the one written before. Each place is repaired with every
/// object the patch hides there, and the meshes chosen for it are kept.
/// Meshes that cannot be repaired are listed in the result; if none can,
/// nothing is written and the result's path is empty. Nothing is
/// installed: that is [`mallet_core::Action::RepairPaths`].
///
/// # Errors
///
/// If the setup or Mod Mallet's folder cannot be found, the plugin cannot
/// be made, or the archive cannot be written.
pub fn create_paths(
    scan: &ScanOptions,
    repairs: &PathRepairs,
    options: &ManagerOptions,
) -> Result<CreatedPaths, EngineError> {
    let started = std::time::Instant::now();
    let own = mallet_game_bethesda::paths_plugin::file_name();
    let (built, refused) = paths_plugin(scan, repairs, &own)?;
    let repaired: Vec<String> = built.repaired.iter().map(ToString::to_string).collect();
    let Some(bytes) = built.bytes else {
        tracing::info!(
            refused = refused.len(),
            "no path could be repaired; nothing written"
        );
        return Ok(CreatedPaths {
            path: String::new(),
            navmeshes: repaired,
            masters: built.masters,
            light: built.light,
            refused,
        });
    };
    let archive = paths_file(options).ok_or(EngineError::ManagerNotFound)?;
    crate::patch::write_zip(&archive, &[(own, bytes)])
        .map_err(EngineError::not_written(&archive))?;
    tracing::info!(
        path = %archive.display(),
        repaired = repaired.len(),
        refused = refused.len(),
        masters = built.masters.len(),
        light = built.light,
        elapsed_ms = started.elapsed().as_millis(),
        "repaired paths written"
    );
    Ok(CreatedPaths {
        path: archive.display().to_string(),
        navmeshes: repaired,
        masters: built.masters,
        light: built.light,
        refused,
    })
}

/// A whole number of game units as a float.
#[allow(clippy::cast_precision_loss)]
fn as_f32(value: i32) -> f32 {
    // Coordinates of the games are far below 2^24.
    value as f32
}
