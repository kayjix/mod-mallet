//! Reads what the mod manager knows beyond the deployment: the names of the
//! mods, and their installers with the options that were chosen.
//!
//! An installer description lies inside the archive a mod was installed
//! from. Getting it out means starting an unpacker for each archive, which
//! takes seconds for large ones. Two things keep that bearable:
//!
//! * The archives are looked into in parallel.
//! * The description is kept in the cache under the archive's size and
//!   modification time. An archive is looked into again only when it
//!   changed (ADR 0006).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use mallet_core::facts::InstallerFacts;
use mallet_core::{LocationOrigin, LocationRole, ModInfo, ModRef, ScanLocation};
use mallet_format_fomod::MODULE_CONFIG_PATH;
use mallet_source_vortex::state::{live_state_path, state_backup_path};
use mallet_source_vortex::{SOURCE_ID, VortexDeployment, VortexMod, VortexState, VortexTool};
use mallet_store::Store;
use mallet_unpack::Unpacker;

use crate::{EngineError, ScanOptions, read_if_exists};

/// Under which name installer descriptions are kept in the cache.
const CACHE_NAMESPACE: &str = "fomod";

/// How many unpackers run at once. More than a handful only makes them
/// compete for the disk.
const MAX_WORKERS: usize = 6;

/// What was learned from the mod manager's state.
#[derive(Default)]
pub(crate) struct ManagerFacts {
    /// The name the manager shows for each mod, by staging folder.
    pub(crate) names: HashMap<String, String>,
    /// The installers that could be read.
    pub(crate) installers: Vec<InstallerFacts>,
    /// What the manager knows about each installed mod.
    pub(crate) mods: Vec<ModInfo>,
}

/// Reads the manager's state and the installers of the deployed mods.
pub(crate) fn read(
    options: &ScanOptions,
    deployment: &VortexDeployment,
    store: Option<&Store>,
    locations: &mut Vec<ScanLocation>,
) -> Result<ManagerFacts, EngineError> {
    let Some(sources) = state_sources(options) else {
        return Ok(ManagerFacts::default());
    };
    let (state_path, origin) = &sources.backup;
    let state = if let Some(state) = read_live_state(&sources, options.game_id(), locations) {
        state
    } else {
        let state_json = read_if_exists(state_path)?;
        locations.push(location(
            LocationRole::ManagerState,
            state_path,
            *origin,
            state_json.is_some(),
        ));
        let Some(state_json) = state_json else {
            return Ok(ManagerFacts::default());
        };
        VortexState::parse(&state_json, options.game_id())?
    };
    let mut facts = ManagerFacts {
        names: state.names(),
        installers: Vec::new(),
        mods: state
            .mods
            .iter()
            .map(|vortex_mod| ModInfo {
                mod_ref: ModRef {
                    source: SOURCE_ID.to_owned(),
                    id: vortex_mod.folder.clone(),
                    name: vortex_mod.name.clone(),
                },
                enabled: vortex_mod.enabled,
                version: vortex_mod.version.clone(),
                newer_version: vortex_mod.newer_version.clone(),
                author: vortex_mod.author.clone(),
                page: vortex_mod.page.clone(),
            })
            .collect(),
    };

    // Only mods that are in the game and went through an installer.
    let candidates: Vec<(&VortexMod, &str)> = state
        .mods
        .iter()
        .filter(|vortex_mod| vortex_mod.enabled && !vortex_mod.chosen.is_empty())
        .filter(|vortex_mod| deployment.has_mod(&vortex_mod.folder))
        .filter_map(|vortex_mod| Some((vortex_mod, vortex_mod.archive.as_deref()?)))
        .collect();
    if candidates.is_empty() {
        return Ok(facts);
    }

    let (downloads_dir, origin) = if let Some(given) = &options.downloads_dir {
        (given.clone(), LocationOrigin::Given)
    } else {
        // The backup lies three levels below Vortex's data directory.
        let data_dir = state_path.ancestors().nth(3).unwrap_or(Path::new(""));
        (
            state.downloads_dir(data_dir, options.game_id()),
            LocationOrigin::Standard,
        )
    };
    locations.push(location(
        LocationRole::Downloads,
        &downloads_dir,
        origin,
        downloads_dir.is_dir(),
    ));

    let (unpacker, origin) = match &options.unpacker {
        Some(given) => (Some(Unpacker::at(given.clone())), LocationOrigin::Given),
        None => (Unpacker::locate(), LocationOrigin::Standard),
    };
    locations.push(match &unpacker {
        Some(unpacker) => location(LocationRole::Unpacker, unpacker.program(), origin, true),
        None => location(LocationRole::Unpacker, Path::new("7-Zip"), origin, false),
    });

    let descriptions = descriptions(&candidates, &downloads_dir, unpacker.as_ref(), store);
    for ((vortex_mod, _), description) in candidates.iter().zip(descriptions) {
        let Some(description) = description else {
            continue;
        };
        match mallet_format_fomod::parse(&description) {
            Ok(steps) if !steps.is_empty() => facts.installers.push(InstallerFacts {
                mod_ref: ModRef {
                    source: SOURCE_ID.to_owned(),
                    id: vortex_mod.folder.clone(),
                    name: vortex_mod.name.clone(),
                },
                steps,
                chosen: vortex_mod.chosen.clone(),
            }),
            Ok(_) => {}
            Err(error) => {
                tracing::warn!(mod_name = %vortex_mod.name, %error, "installer not readable");
            }
        }
    }
    tracing::info!(installers = facts.installers.len(), "read mod installers");
    Ok(facts)
}

/// The text of each candidate's installer description, from the cache
/// where possible. `None` where there is none or it could not be read.
fn descriptions(
    candidates: &[(&VortexMod, &str)],
    downloads_dir: &Path,
    unpacker: Option<&Unpacker>,
    store: Option<&Store>,
) -> Vec<Option<String>> {
    let mut descriptions: Vec<Option<String>> = vec![None; candidates.len()];
    // Archives that have to be looked into: position, path, fingerprint.
    let mut pending: Vec<(usize, PathBuf, String)> = Vec::new();

    for (index, (_, archive_name)) in candidates.iter().enumerate() {
        let archive = downloads_dir.join(archive_name);
        let Some(fingerprint) = fingerprint(&archive) else {
            continue;
        };
        let cached = store.and_then(|store| {
            store
                .cached_text(CACHE_NAMESPACE, archive_name, &fingerprint)
                .ok()
                .flatten()
        });
        match (cached, descriptions.get_mut(index)) {
            // An empty entry records that the archive has no installer.
            (Some(text), Some(slot)) => *slot = Some(text).filter(|text| !text.is_empty()),
            _ => pending.push((index, archive, fingerprint)),
        }
    }

    let Some(unpacker) = unpacker else {
        return descriptions;
    };
    let archives: Vec<&Path> = pending.iter().map(|(_, path, _)| path.as_path()).collect();
    for ((index, archive, fingerprint), text) in pending.iter().zip(unpack_all(unpacker, &archives))
    {
        let Some(text) = text else {
            continue;
        };
        if let (Some(store), Some(name)) =
            (store, archive.file_name().and_then(|name| name.to_str()))
            && let Err(error) = store.cache_text(CACHE_NAMESPACE, name, fingerprint, &text)
        {
            tracing::warn!(%error, "installer description could not be cached");
        }
        if let Some(slot) = descriptions.get_mut(*index) {
            *slot = Some(text).filter(|text| !text.is_empty());
        }
    }
    descriptions
}

/// Looks into every archive, several at a time. Returns the installer
/// description as text, an empty text if the archive has none, and `None`
/// if the archive could not be read.
fn unpack_all(unpacker: &Unpacker, archives: &[&Path]) -> Vec<Option<String>> {
    let workers = std::thread::available_parallelism()
        .map_or(1, std::num::NonZero::get)
        .min(MAX_WORKERS)
        .min(archives.len());
    let next = AtomicUsize::new(0);

    let unpack_some = || {
        let mut done = Vec::new();
        loop {
            let index = next.fetch_add(1, Ordering::Relaxed);
            let Some(archive) = archives.get(index) else {
                break done;
            };
            let text = match unpacker.read_by_suffix(archive, MODULE_CONFIG_PATH) {
                Ok(Some(bytes)) => Some(mallet_format_fomod::decode(&bytes)),
                Ok(None) => Some(String::new()),
                Err(error) => {
                    tracing::warn!(%error, "archive not readable");
                    None
                }
            };
            done.push((index, text));
        }
    };

    let mut results: Vec<Option<String>> = vec![None; archives.len()];
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..workers).map(|_| scope.spawn(unpack_some)).collect();
        for handle in handles {
            // A worker that panicked simply contributes nothing.
            for (index, text) in handle.join().unwrap_or_default() {
                if let Some(slot) = results.get_mut(index) {
                    *slot = text;
                }
            }
        }
    });
    results
}

/// A value that changes when the archive does: its size and the time it
/// was last written. `None` if the archive is not there.
fn fingerprint(archive: &Path) -> Option<String> {
    let metadata = std::fs::metadata(archive).ok()?;
    let modified = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |elapsed| elapsed.as_secs());
    Some(format!("{}:{modified}", metadata.len()))
}

/// The two files Vortex's state can be read from.
struct StateSources {
    /// The backup Vortex writes itself, about once an hour.
    backup: (PathBuf, LocationOrigin),
    /// The copy the Mod Mallet link writes whenever something changes.
    live: Option<(PathBuf, LocationOrigin)>,
}

fn state_sources(options: &ScanOptions) -> Option<StateSources> {
    let backup = match &options.manager_state {
        Some(given) => (given.clone(), LocationOrigin::Given),
        None => (
            state_backup_path(&dirs::config_dir()?.join("Vortex")),
            LocationOrigin::Standard,
        ),
    };
    let live = match (&options.manager_live_state, &options.manager_state) {
        (Some(given), _) => Some((given.clone(), LocationOrigin::Given)),
        (None, None) => crate::actions::default_link_dir().map(|dir| {
            (
                live_state_path(&dir, options.game_id()),
                LocationOrigin::Standard,
            )
        }),
        // Whoever names the backup wants that file read, not one from the
        // machine's standard places.
        (None, Some(_)) => None,
    };
    Some(StateSources { backup, live })
}

/// The state from the link's copy, if that is the newer of the two files
/// and can be read. Otherwise `None`: the backup is to be read.
///
/// The link's copy is written within a second of a change, the backup once
/// an hour. The link's copy is older only if the link has not been running,
/// and then the backup knows more.
fn read_live_state(
    sources: &StateSources,
    game: &str,
    locations: &mut Vec<ScanLocation>,
) -> Option<VortexState> {
    let (path, origin) = sources.newer_live()?;
    let state_json = read_if_exists(path).ok().flatten()?;
    match VortexState::parse(&state_json, game) {
        Ok(state) => {
            locations.push(location(LocationRole::ManagerState, path, *origin, true));
            Some(state)
        }
        Err(error) => {
            tracing::warn!(path = %path.display(), %error, "the link's state is not readable");
            None
        }
    }
}

/// The tools the mod manager lists for the game on its dashboard: from
/// the link's copy of the state if that has them and is newer than the
/// backup, else from the backup. Empty when neither can be read. Only reads.
pub(crate) fn manager_tools(options: &ScanOptions) -> Vec<VortexTool> {
    let Some(sources) = state_sources(options) else {
        return Vec::new();
    };
    let game = options.game_id();
    let read = |path: &Path| {
        let text = read_if_exists(path).ok().flatten()?;
        VortexState::parse(&text, game).ok()?.tools
    };
    sources
        .newer_live()
        .and_then(|(path, _)| read(path))
        .or_else(|| read(&sources.backup.0))
        .unwrap_or_default()
}

impl StateSources {
    /// The link's copy, if it is there and newer than the backup.
    fn newer_live(&self) -> Option<&(PathBuf, LocationOrigin)> {
        let live = self.live.as_ref()?;
        let written = modified(&live.0)?;
        let backup_is_as_new = modified(&self.backup.0).is_some_and(|backup| backup >= written);
        (!backup_is_as_new).then_some(live)
    }
}

/// The file a scan would read the mod manager's state from, without
/// reading it.
pub(crate) fn planned_state(options: &ScanOptions) -> Option<(PathBuf, LocationOrigin)> {
    let sources = state_sources(options)?;
    Some(match sources.newer_live() {
        Some(live) => live.clone(),
        None => sources.backup,
    })
}

fn modified(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).ok()?.modified().ok()
}

fn location(role: LocationRole, path: &Path, origin: LocationOrigin, found: bool) -> ScanLocation {
    ScanLocation {
        role,
        path: path.display().to_string(),
        origin,
        found,
    }
}
