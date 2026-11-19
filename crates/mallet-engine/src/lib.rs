//! Runs a Mod Mallet scan.
//!
//! The engine is the one place where the stages meet (ADR 0004):
//!
//! 1. **Ingest.** Adapters read the game's and the mod manager's files and
//!    turn them into [`Facts`].
//! 2. **Analyse.** The analyses derive findings from the facts.
//! 3. **Report.** The findings become a [`ScanReport`], which is cached.
//!
//! The command line and the desktop app are thin shells around the functions
//! here, so both always behave the same.
//!
//! A scan only reads. Nothing in the game, the mods or the mod manager is
//! changed (ADR 0005). Changes are requested from the mod manager with
//! [`run_action`], and only when the user asks for them.

mod actions;
mod anchors;
mod area;
mod catalogue;
mod installers;
mod knowledge;
mod lod;
mod logging;
mod mesh;
mod patch;
mod png;
mod preview;
mod settings;
mod staging;
mod toolbox;
mod versions;
mod walkways;

use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use mallet_analysis::Context;
use mallet_core::facts::{ExtenderReport, InstallerFacts};
use mallet_core::{
    Facts, GameInfo, GamePath, InstalledPatch, LocationOrigin, LocationRole, ModInfo, ModSource,
    NoCache, NoModSource, ScanLocation, ScanReport, TextCache, report,
};
use mallet_game_bethesda::{Game, crash_logs, ingest, locate, skse_log};
use mallet_source_vortex::{VortexDeployment, VortexError};
use mallet_store::{Store, StoreError};

pub use actions::{
    CreatedPatch, ManagerLink, ManagerOptions, create_patch, install_manager_link, manager_link,
    patch_file, run_action, save_patch_report,
};
pub use anchors::object_anchors;
pub use area::{area_land, area_mesh, area_objects, area_texture, interior_objects, world_land};
pub use catalogue::catalogue;
pub use lod::{lod_mesh, lod_texture, lod_tiles};
pub use logging::{LOG_FILE, PREVIOUS_LOG_FILE, default_log_dir, start_logging};
pub use mesh::mesh_view;
pub use preview::file_preview;
pub use settings::{
    Settings, default_settings_path, known_games, load_settings, planned_locations, save_settings,
    select_game, setup_fingerprint, update_settings,
};
pub use toolbox::{game_art, restart_manager, toolbox};
pub use versions::{asset_versions, asset_view};
pub use walkways::hidden_walkways;

/// Where to look. Every field that is `None` is detected automatically.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScanOptions {
    /// The game to look at, by its identifier (`skyrimse`, `fallout4`,
    /// ...). Skyrim Special Edition if `None`.
    pub game: Option<String>,
    /// The directory the game is installed in.
    pub game_dir: Option<PathBuf>,
    /// The directory in which the game keeps `plugins.txt`, the list of
    /// active plugins.
    pub local_dir: Option<PathBuf>,
    /// The script extender's log file.
    pub extender_log: Option<PathBuf>,
    /// The Vortex deployment manifest of the game's `Data` directory.
    pub deployment_manifest: Option<PathBuf>,
    /// The backup of Vortex's state, which lists the installed mods and
    /// the options chosen in their installers.
    pub manager_state: Option<PathBuf>,
    /// The copy of Vortex's state that the Mod Mallet link keeps up to
    /// date. It is used instead of the backup when it is the newer of the
    /// two. Detected automatically only if `manager_state` is, too.
    pub manager_live_state: Option<PathBuf>,
    /// The directory in which the mod manager keeps downloaded archives.
    pub downloads_dir: Option<PathBuf>,
    /// The folder in which the mod manager keeps every mod's files, of
    /// which only the names are read. Taken from the deployment manifest
    /// if `None`.
    pub staging_dir: Option<PathBuf>,
    /// The 7-Zip program used to look into mod archives.
    pub unpacker: Option<PathBuf>,
    /// The LOOT masterlist, whose rules about plugins are checked.
    /// Detected automatically only if `manager_state` is, too.
    pub masterlist: Option<PathBuf>,
}

impl ScanOptions {
    /// The game these options are about.
    ///
    /// # Errors
    ///
    /// If the game's identifier is not one Mod Mallet knows.
    pub(crate) fn game(&self) -> Result<Game, EngineError> {
        match &self.game {
            None => Ok(Game::default()),
            Some(id) => Game::from_id(id).ok_or_else(|| EngineError::UnknownGame(id.clone())),
        }
    }

    /// The identifier of the game these options are about, as the mod
    /// manager knows it.
    pub(crate) fn game_id(&self) -> &str {
        self.game.as_deref().unwrap_or_else(|| Game::default().id())
    }

    /// The game's directory: the one named, or the one found on this
    /// machine, and how it came to be known. `None` for an unknown game or
    /// one that is not found.
    pub(crate) fn located_game_dir(&self) -> Option<(PathBuf, LocationOrigin)> {
        match &self.game_dir {
            Some(given) => Some((given.clone(), LocationOrigin::Given)),
            None => locate::game_dir(self.game().ok()?),
        }
    }
}

/// Why a scan could not be completed.
#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    /// The game's directory is unknown.
    #[error(
        "the game directory could not be found automatically; pass it explicitly (--game-dir on the command line)"
    )]
    GameNotFound,
    /// A game was named that Mod Mallet does not know.
    #[error("Mod Mallet does not know the game `{0}`")]
    UnknownGame(String),
    /// A patch was asked for in a game Base Object Swapper does not run in.
    #[error("patches hide objects through Base Object Swapper, which this game does not have")]
    NoObjectSwapper,
    /// The mod manager's data directory could not be found.
    #[error("Vortex's data directory could not be found; is Vortex installed for this user?")]
    ManagerNotFound,
    /// The game's directory was given but does not exist.
    #[error("the game directory `{}` does not exist", .0.display())]
    GameDirMissing(PathBuf),
    /// A file exists but could not be read.
    #[error("`{}` could not be read: {source}", .path.display())]
    Read {
        /// The file that could not be read.
        path: PathBuf,
        /// What the system reported.
        source: std::io::Error,
    },
    /// The settings file exists but does not hold settings.
    #[error("the settings file `{}` is not valid: {reason}", .path.display())]
    Settings {
        /// The settings file.
        path: PathBuf,
        /// What is wrong with it.
        reason: String,
    },
    /// The settings could not be written.
    #[error("the settings could not be written to `{}`: {source}", .path.display())]
    WriteSettings {
        /// The settings file.
        path: PathBuf,
        /// What the system reported.
        source: std::io::Error,
    },
    /// The cache directory could not be created.
    #[error("the cache directory `{}` could not be created: {source}", .path.display())]
    CreateCacheDir {
        /// The directory that could not be created.
        path: PathBuf,
        /// What the system reported.
        source: std::io::Error,
    },
    /// A file of Mod Mallet's own could not be written.
    #[error("`{}` could not be written: {source}", .path.display())]
    Write {
        /// The file or folder that could not be written.
        path: PathBuf,
        /// What the system reported.
        source: std::io::Error,
    },
    /// The mod manager could not be restarted.
    #[error("Vortex could not be restarted: {0}")]
    Restart(String),
    /// A patch was asked for that would hide nothing.
    #[error("no objects were chosen to hide")]
    NothingToHide,
    /// The mod manager's records could not be used.
    #[error(transparent)]
    Vortex(#[from] VortexError),
    /// The cache could not be used.
    #[error(transparent)]
    Store(#[from] StoreError),
}

/// Examines the setup described by `options` and reports what is wrong.
///
/// Nothing is remembered between calls, so everything is read again each
/// time. [`scan_and_store`] is faster from the second scan on.
pub fn scan(options: &ScanOptions) -> Result<ScanReport, EngineError> {
    scan_with(options, None)
}

fn scan_with(options: &ScanOptions, store: Option<&Store>) -> Result<ScanReport, EngineError> {
    let game = options.game()?;
    let (game_dir, game_dir_origin) = options
        .located_game_dir()
        .ok_or(EngineError::GameNotFound)?;
    if !game_dir.is_dir() {
        return Err(EngineError::GameDirMissing(game_dir));
    }
    tracing::info!(game = game.id(), game_dir = %game_dir.display(), "scanning");
    let mut locations = vec![ScanLocation {
        role: LocationRole::GameDir,
        path: game_dir.display().to_string(),
        origin: game_dir_origin,
        found: true,
    }];

    let started = Instant::now();
    let mut facts = match store {
        // Reading the setup writes thousands of small cache entries.
        Some(store) => {
            store.batched(|| gather_facts(options, game, &game_dir, store, &mut locations))??
        }
        None => gather_facts(options, game, &game_dir, &NoCache, &mut locations)?,
    };
    facts.knowledge = knowledge::read(options, game, &game_dir, &facts.plugins, &mut locations);
    let (mods, installers, mod_infos) = load_mod_source(options, &game_dir, store, &mut locations)?;
    facts.installers = installers;
    tracing::info!(
        plugins = facts.plugins.len(),
        archives = facts.archives.len(),
        mesh_overrides = facts.mesh_overrides.len(),
        loose_files = facts.loose_files.len(),
        elapsed_ms = started.elapsed().as_millis(),
        "read the setup"
    );

    // The objects Mod Mallet's patch hides, and the paths over them
    // (ADR 0051).
    let installed_patch = game
        .has_object_swapper()
        .then(|| installed_patch(&game_dir, mods.as_ref()))
        .flatten();
    facts.hidden_objects = walkways::hidden_by(installed_patch.as_ref());

    // Where objects of two mods meet by their boxes, their models decide
    // (ADR 0039). Only those models are read, and those of the hidden
    // objects.
    let started = Instant::now();
    let mut wanted =
        mallet_analysis::objects::models_to_check(&Context::new(&facts, mods.as_ref()));
    facts.navmeshes.near_hidden = walkways::near_hidden(
        game,
        &game_dir,
        options.local_dir.as_deref(),
        &facts,
        &mut wanted,
    );
    facts.models = mallet_game_bethesda::models::read_models(
        game,
        &game_dir,
        options.local_dir.as_deref(),
        &wanted,
    );
    tracing::info!(
        wanted = wanted.len(),
        read = facts.models.meshes.len(),
        elapsed_ms = started.elapsed().as_millis(),
        "read the models of objects that may collide"
    );

    let started = Instant::now();
    let context = Context::new(&facts, mods.as_ref());
    let findings = mallet_analysis::run_all(&context);
    let map = mallet_analysis::map::world_map(&context);
    let duels = mallet_analysis::duels::duels(&findings, &context);
    tracing::info!(
        findings = findings.len(),
        elapsed_ms = started.elapsed().as_millis(),
        "analysed the setup"
    );

    let extender = facts.extender.as_ref();
    Ok(ScanReport {
        schema_version: report::SCHEMA_VERSION,
        scanned_at: now(),
        game: GameInfo {
            id: game.id().to_owned(),
            name: game.name().to_owned(),
            version: extender.and_then(|report| report.game_version.clone()),
            extender_version: extender.and_then(|report| report.extender_version.clone()),
        },
        extender_plugins_checked: extender.map_or(0, |report| count(report.plugins.len())),
        plugins_checked: count(facts.plugins.len()),
        archives_checked: count(facts.archives.len()),
        installers_checked: count(facts.installers.len()),
        meshes_checked: count(facts.mesh_overrides.len()),
        locations,
        findings,
        mods: mod_infos,
        map,
        object_swapper: game.has_object_swapper()
            && game_dir
                .join("Data")
                .join(mallet_game_bethesda::swap::SWAPPER_PLUGIN)
                .is_file(),
        installed_patch,
        duels,
    })
}

/// Swap files larger than this are not read: a part of a patch is a few
/// kilobytes.
const MAX_SWAP_FILE_BYTES: u64 = 4 * 1024 * 1024;

/// The patches Mod Mallet wrote, here or elsewhere, as the game sees them
/// now: the swap files in the data directory that describe themselves as
/// parts of a patch (ADR 0053), and the one file of patches written before
/// patches had parts. Each patch with the parts that are installed and the
/// mod it came with. Only reads.
fn installed_patch(game_dir: &Path, mods: &dyn ModSource) -> Option<InstalledPatch> {
    use mallet_game_bethesda::swap;

    let data = game_dir.join("Data");
    let mut names: Vec<String> = std::fs::read_dir(&data)
        .map_err(|error| tracing::warn!(%error, "data directory not listed"))
        .ok()?
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .metadata()
                .is_ok_and(|meta| meta.is_file() && meta.len() <= MAX_SWAP_FILE_BYTES)
        })
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| swap::is_swap_file(name))
        .collect();
    names.sort_by_key(|name| name.to_lowercase());

    let mut patches: Vec<mallet_core::PatchInGame> = Vec::new();
    for name in names {
        let Ok(bytes) = std::fs::read(data.join(&name)) else {
            continue;
        };
        let text = String::from_utf8_lossy(&bytes);
        let patch = if let Some(part) = swap::read_part(&text) {
            part
        } else if name.eq_ignore_ascii_case(swap::SWAP_FILE) {
            old_patch(&text)
        } else {
            continue;
        };
        let provider = || mods.provider_of(&GamePath::new(&format!("data/{name}")));
        match patches
            .iter_mut()
            .find(|known| known.patch.name == patch.name)
        {
            Some(known) => known.patch.parts.extend(patch.parts),
            None => patches.push(mallet_core::PatchInGame {
                provider: provider(),
                patch,
            }),
        }
    }
    let first = patches.first()?;
    let provider = first.provider.clone();
    let objects: std::collections::BTreeSet<mallet_core::ObjectRef> = patches
        .iter()
        .flat_map(|known| known.patch.objects())
        .collect();
    tracing::info!(
        patches = patches.len(),
        parts = patches
            .iter()
            .map(|known| known.patch.parts.len())
            .sum::<usize>(),
        objects = objects.len(),
        "found Mod Mallet's patches in the game"
    );
    Some(InstalledPatch {
        objects: objects.into_iter().collect(),
        provider,
        patches,
    })
}

/// A patch written before patches had parts, from its one swap file: one
/// part per plugin, against none.
fn old_patch(text: &str) -> mallet_core::Patch {
    let mut parts: Vec<mallet_core::PatchPart> = Vec::new();
    for object in mallet_game_bethesda::swap::hidden_objects(text) {
        match parts.iter_mut().find(|part| part.plugin == object.plugin) {
            Some(part) => part.objects.push(object),
            None => parts.push(mallet_core::PatchPart {
                plugin: object.plugin.clone(),
                against: None,
                objects: vec![object],
            }),
        }
    }
    mallet_core::Patch {
        name: mallet_core::DEFAULT_PATCH_NAME.to_owned(),
        author: None,
        version: None,
        parts,
    }
}

/// Pixels per cell of the map's picture.
const MAP_PIXELS_PER_CELL: u32 = 8;

/// A picture of `world` from above, from the terrain LOD textures the game
/// uses, in its archives or loose, or `None` if the worldspace has none.
/// Only reads.
pub fn map_image(options: &ScanOptions, world: &str) -> Option<mallet_core::MapImage> {
    if !options.game().ok()?.has_terrain_lod_pictures() {
        return None;
    }
    let started = Instant::now();
    let library = lod::library(options, world)?;
    let image = mallet_game_bethesda::lod::world_image_in(&library, MAP_PIXELS_PER_CELL)?;
    let png = png::encode_rgb(image.width(), image.height(), &image.rgb)
        .map_err(|error| tracing::warn!(%error, "map picture not encoded"))
        .ok()?;
    tracing::info!(
        world,
        width = image.width(),
        height = image.height(),
        bytes = png.len(),
        elapsed_ms = started.elapsed().as_millis(),
        "drew the map's picture"
    );
    Some(mallet_core::MapImage {
        world: world.to_owned(),
        west: image.west,
        south: image.south,
        columns: image.columns,
        rows: image.rows,
        image: format!(
            "data:image/png;base64,{}",
            mallet_core::encoding::base64(&png)
        ),
    })
}

/// Runs a scan with the cache at `cache_path`: what is expensive to read
/// and has not changed is taken from the cache, and the result is
/// remembered for [`last_report`]. Each game has a cache of its own next
/// to `cache_path`.
pub fn scan_and_store(options: &ScanOptions, cache_path: &Path) -> Result<ScanReport, EngineError> {
    let mut store = open_store(&game_cache_path(cache_path, options.game()?))?;
    let report = scan_with(options, Some(&store))?;
    store.save_scan(&report)?;
    Ok(report)
}

/// The result of the last stored scan of the game `options` are about, or
/// `None` if there is none.
pub fn last_report(
    options: &ScanOptions,
    cache_path: &Path,
) -> Result<Option<ScanReport>, EngineError> {
    let cache_path = game_cache_path(cache_path, options.game()?);
    if !cache_path.exists() {
        return Ok(None);
    }
    match open_store(&cache_path)?.load_scan() {
        // The store is a cache (ADR 0006): a report another version of Mod
        // Mallet kept, which this one cannot read, is as good as none.
        Err(StoreError::Corrupt(error)) => {
            tracing::info!(%error, "the last report cannot be read; it is left for the next scan");
            Ok(None)
        }
        other => Ok(other?),
    }
}

/// The cache of `game`, next to `cache_path`. Skyrim Special Edition, the
/// first game, keeps the file as it is named; every other game has one
/// with its identifier added (`cache-fallout4.sqlite`).
fn game_cache_path(cache_path: &Path, game: Game) -> PathBuf {
    if game == Game::SkyrimSe {
        return cache_path.to_path_buf();
    }
    let stem = cache_path
        .file_stem()
        .map_or_else(|| "cache".into(), |stem| stem.to_string_lossy());
    let name = match cache_path.extension() {
        Some(extension) => format!("{stem}-{}.{}", game.id(), extension.to_string_lossy()),
        None => format!("{stem}-{}", game.id()),
    };
    cache_path.with_file_name(name)
}

/// Where the cache lives unless the caller chooses another place.
///
/// `None` if the system does not tell where applications may keep local
/// data.
pub fn default_cache_path() -> Option<PathBuf> {
    Some(
        dirs::data_local_dir()?
            .join("mod-mallet")
            .join("cache.sqlite"),
    )
}

fn open_store(cache_path: &Path) -> Result<Store, EngineError> {
    if let Some(directory) = cache_path
        .parent()
        .filter(|dir| !dir.as_os_str().is_empty())
    {
        std::fs::create_dir_all(directory).map_err(|source| EngineError::CreateCacheDir {
            path: directory.to_path_buf(),
            source,
        })?;
    }
    Ok(Store::open(cache_path)?)
}

fn gather_facts(
    options: &ScanOptions,
    game: Game,
    game_dir: &Path,
    cache: &dyn TextCache,
    locations: &mut Vec<ScanLocation>,
) -> Result<Facts, EngineError> {
    let log_path = match &options.extender_log {
        Some(given) => Some((given.clone(), LocationOrigin::Given)),
        None => {
            locate::extender_log(game, Some(game_dir)).map(|path| (path, LocationOrigin::Standard))
        }
    };
    // Crash loggers write next to the script extender's log.
    let extender_dir = log_path
        .as_ref()
        .and_then(|(path, _)| path.parent())
        .map(Path::to_path_buf);
    let extender = match log_path {
        Some((path, origin)) => {
            let text = read_if_exists(&path)?;
            locations.push(ScanLocation {
                role: LocationRole::ExtenderLog,
                path: path.display().to_string(),
                origin,
                found: text.is_some(),
            });
            text.map(|text| without_removed_plugins(skse_log::parse(game, &text), game_dir))
        }
        None => None,
    };
    if extender.is_none() {
        tracing::info!("no script extender log found");
    }

    let mut facts = Facts {
        extender,
        crashes: read_crash_logs(extender_dir.as_deref(), game_dir, locations),
        ..Facts::default()
    };
    // Without a load order there is nothing to say about plugins and
    // archives. The scan still reports what it can, and says what it could
    // not look at.
    match ingest::read_setup(game, game_dir, options.local_dir.as_deref(), cache) {
        Ok(setup) => {
            locations.push(ScanLocation {
                role: LocationRole::PluginList,
                path: setup.plugin_list.display().to_string(),
                origin: if options.local_dir.is_some() {
                    LocationOrigin::Given
                } else {
                    LocationOrigin::Standard
                },
                found: setup.plugin_list.is_file(),
            });
            facts.loose_files = setup.loose_files;
            facts.archives = setup.archives;
            facts.plugins = setup.plugins;
            facts.unreadable = setup.unreadable;
            facts.mesh_overrides = setup.mesh_overrides;
            facts.foreign_meshes = setup.foreign_meshes;
            facts.mesh_textures = setup.mesh_textures;
            facts.objects = setup.objects;
            facts.terrain = setup.terrain;
            facts.markers = setup.markers;
            facts.navmeshes = setup.navmeshes;
            tracing::info!(
                placed = facts.objects.placed.len(),
                bases = facts.objects.bases.len(),
                "read placed objects"
            );
            let changed = facts
                .navmeshes
                .changes
                .iter()
                .flat_map(|plugin| &plugin.navmeshes);
            let (with_doors, without): (Vec<_>, Vec<_>) =
                changed.partition(|navmesh| navmesh.doors.is_some());
            tracing::info!(
                changed = with_doors.len() + without.len(),
                unreadable = without.len(),
                "read navigation meshes that mods change"
            );
        }
        Err(error) => {
            tracing::warn!(%error, "the installation could not be read");
            facts.load_order_problem = Some(error.to_string());
        }
    }
    Ok(facts)
}

/// What is learned from the mod manager: which mod each file belongs to,
/// the mods' installers, and what is known about each mod.
type ManagerParts = (Box<dyn ModSource>, Vec<InstallerFacts>, Vec<ModInfo>);

fn load_mod_source(
    options: &ScanOptions,
    game_dir: &Path,
    store: Option<&Store>,
    locations: &mut Vec<ScanLocation>,
) -> Result<ManagerParts, EngineError> {
    let (manifest_path, origin) = match &options.deployment_manifest {
        Some(given) => (given.clone(), LocationOrigin::Given),
        None => (
            mallet_source_vortex::data_manifest_path(game_dir),
            LocationOrigin::Standard,
        ),
    };
    let manifest = read_if_exists(&manifest_path)?;
    locations.push(ScanLocation {
        role: LocationRole::DeploymentManifest,
        path: manifest_path.display().to_string(),
        origin,
        found: manifest.is_some(),
    });
    let Some(manifest) = manifest else {
        tracing::info!("no Vortex deployment found; files will not be attributed to mods");
        return Ok((Box::new(NoModSource), Vec::new(), Vec::new()));
    };
    let deployment = VortexDeployment::parse(&manifest, &game_dir.to_string_lossy())?;
    let deployment = staging::read(options, deployment, store, locations);
    tracing::info!(
        files = deployment.file_count(),
        mods = deployment.mod_count(),
        "read Vortex deployment"
    );
    let manager = installers::read(options, &deployment, store, locations)?;
    let switched_off: Vec<String> = manager
        .mods
        .iter()
        .filter(|info| !info.enabled)
        .map(|info| info.mod_ref.id.clone())
        .collect();
    let deployment = staging::read_switched_off(options, deployment, &switched_off, store);
    Ok((
        Box::new(deployment.with_names(manager.names)),
        manager.installers,
        manager.mods,
    ))
}

/// Reads a text file. A file that does not exist is `None`, not an error:
/// a setup without a script extender or without Vortex is a valid setup.
///
/// Bytes that are not valid UTF-8 are replaced, because logs and manifests
/// written on Windows may contain file names in a legacy encoding.
/// Leaves out the plugins of the extender's log that are no longer in the
/// game. The log describes the last start of the game; a plugin removed
/// since, by switching its mod off for example, will not be tried again,
/// and reporting it would ask the user to fix what they already fixed.
fn without_removed_plugins(mut report: ExtenderReport, game_dir: &Path) -> ExtenderReport {
    // Without the game's data folder there is nothing to compare with; the
    // scan reports that on its own.
    if !game_dir.join("Data").is_dir() {
        return report;
    }
    let dir = game_dir.join(report.plugin_dir.as_str());
    report.plugins.retain(|plugin| {
        let present = dir.join(&plugin.file_name).is_file();
        if !present {
            tracing::info!(
                plugin = %plugin.file_name,
                "left out: named in the extender's log, but no longer in the game"
            );
        }
        present
    });
    report
}

/// Reads the recent reports of the crash loggers, which write next to the
/// script extender's log in `extender_dir` and below `game_dir`, and notes
/// each of their folders that exists among the places looked at.
fn read_crash_logs(
    extender_dir: Option<&Path>,
    game_dir: &Path,
    locations: &mut Vec<ScanLocation>,
) -> mallet_core::facts::CrashFacts {
    let folders = crash_logs::folders(extender_dir, game_dir);
    for (folder, _) in folders.iter().filter(|(folder, _)| folder.is_dir()) {
        locations.push(ScanLocation {
            role: LocationRole::CrashLogs,
            path: folder.display().to_string(),
            origin: LocationOrigin::Standard,
            found: true,
        });
    }
    let crashes = crash_logs::read(&folders, std::time::SystemTime::now());
    tracing::info!(
        reports = crashes.logs.len(),
        days = crashes.window_days,
        "read crash reports"
    );
    crashes
}

fn read_if_exists(path: &Path) -> Result<Option<String>, EngineError> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Some(String::from_utf8_lossy(&bytes).into_owned())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(EngineError::Read {
            path: path.to_path_buf(),
            source,
        }),
    }
}

fn count(len: usize) -> u32 {
    u32::try_from(len).unwrap_or(u32::MAX)
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs())
}
