//! Reads an installation of one of the games into facts.
//!
//! This is the part of the adapter that touches the disk. It only reads
//! (ADR 0005), and it reads as little as possible: the directory listing of
//! the data directory, the file list at the start of each archive, the
//! character and base object records of each active plugin, and the placed
//! objects of each plugin that is not the game's own.
//!
//! A file that cannot be read does not stop the scan. It is recorded in
//! [`SetupFacts::unreadable`], and the scan carries on with the rest
//! (ADR 0013).

use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufReader};
use std::path::{Path, PathBuf};

use loadorder::GameSettings;
use mallet_core::facts::objects::PlaceName;
use mallet_core::facts::{
    ArchiveFacts, ForeignMesh, MarkerFacts, MeshOverride, MeshTextures, NavmeshFacts, ObjectFacts,
    PluginFacts, TerrainFacts, UnreadableFile,
};
use mallet_core::{GamePath, Progress, ScanStep, TextCache, WorldOrigin};
use mallet_formats_bethesda::plugin;

use crate::game::Game;
use crate::meshes::{self, LoadedArchive};
use crate::{
    DATA_DIR, archive, archive_meshes, archives, loose_meshes, markers, navmeshes, npc, objects,
    terrain,
};

/// What was read from the installation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SetupFacts {
    /// The file the list of active plugins was read from.
    pub plugin_list: PathBuf,
    /// Every ordinary file in the data directory.
    pub loose_files: Vec<GamePath>,
    /// The archives the game loads, from lowest to highest precedence.
    pub archives: Vec<ArchiveFacts>,
    /// The active plugins, in load order.
    pub plugins: Vec<PluginFacts>,
    /// Files that could not be read.
    pub unreadable: Vec<UnreadableFile>,
    /// Loose meshes that replace a mesh from an archive.
    pub mesh_overrides: Vec<MeshOverride>,
    /// Loose meshes made for another game or edition.
    pub foreign_meshes: Vec<ForeignMesh>,
    /// The textures every loose mesh names.
    pub mesh_textures: Vec<MeshTextures>,
    /// The objects mods placed in the world.
    pub objects: ObjectFacts,
    /// The shape of the land.
    pub terrain: TerrainFacts,
    /// The places of the world map.
    pub markers: MarkerFacts,
    /// The navigation meshes of other plugins that mods change.
    pub navmeshes: NavmeshFacts,
}

/// Why an installation could not be read at all.
#[derive(Debug, thiserror::Error)]
pub enum IngestError {
    /// The load order could not be determined.
    #[error("the load order could not be determined: {0}")]
    LoadOrder(String),
    /// The data directory could not be listed.
    #[error("the game's data directory could not be read: {0}")]
    DataDir(#[from] io::Error),
}

/// Folders whose files the game sees in its data directory as if they were
/// there, as Mod Organizer shows them through its virtual file system
/// (ADR 0064). Empty for a mod manager that puts the files into the data
/// directory itself.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VirtualData {
    /// The folders, the one whose files win first. Behind all of them lies
    /// the game's own data directory.
    pub folders: Vec<PathBuf>,
}

impl VirtualData {
    /// The file the game sees as `name` directly in its data directory: the
    /// copy in the first folder that has one, else the one in `data_dir`.
    fn resolve(&self, data_dir: &Path, name: &str) -> PathBuf {
        self.folders
            .iter()
            .map(|folder| folder.join(name))
            .find(|file| file.is_file())
            .unwrap_or_else(|| data_dir.join(name))
    }

    /// The archives of `game` the game sees in its data directory, by file
    /// name: those in `data_dir` and in the folders. A folder that cannot
    /// be listed adds none.
    fn archives(&self, game: Game, data_dir: &Path) -> io::Result<Vec<String>> {
        let mut names = archives_in(game, data_dir)?;
        for folder in &self.folders {
            for name in archives_in(game, folder).unwrap_or_default() {
                if !names.iter().any(|known| known.eq_ignore_ascii_case(&name)) {
                    names.push(name);
                }
            }
        }
        Ok(names)
    }
}

/// Reads the installation of `game` in `game_dir`.
///
/// `local_dir` is the directory that holds `plugins.txt`. It is detected if
/// `None`, which works on Windows only. `cache` keeps what is slow to read
/// until the files it came from change.
pub fn read_setup(
    game: Game,
    game_dir: &Path,
    local_dir: Option<&Path>,
    cache: &dyn TextCache,
) -> Result<SetupFacts, IngestError> {
    read_setup_with(game, game_dir, local_dir, cache, &Progress::none())
}

/// Like [`read_setup`], and tells `progress` which archives and plugins it
/// is reading.
pub fn read_setup_with(
    game: Game,
    game_dir: &Path,
    local_dir: Option<&Path>,
    cache: &dyn TextCache,
    progress: &Progress<'_>,
) -> Result<SetupFacts, IngestError> {
    read_setup_in(
        game,
        game_dir,
        local_dir,
        &VirtualData::default(),
        cache,
        progress,
    )
}

/// [`read_setup_with`], for a game whose data directory a mod manager
/// extends with the folders of `virtual_data`: plugins, archives and loose
/// files are taken from them as the game would see them.
pub fn read_setup_in(
    game: Game,
    game_dir: &Path,
    local_dir: Option<&Path>,
    virtual_data: &VirtualData,
    cache: &dyn TextCache,
    progress: &Progress<'_>,
) -> Result<SetupFacts, IngestError> {
    let (active_plugins, plugin_list) =
        active_plugins_in(game, game_dir, local_dir, &virtual_data.folders)?;
    let data_dir = game_dir.join("Data");
    let data_path = GamePath::new(DATA_DIR);

    let mut loose = loose_files(&data_dir, &data_path)?;
    if !virtual_data.folders.is_empty() {
        loose = merged_loose_files(loose, &virtual_data.folders, &data_path);
    }
    let mut facts = SetupFacts {
        plugin_list,
        loose_files: loose.iter().map(|(path, _)| path.clone()).collect(),
        ..SetupFacts::default()
    };
    let archive_names = archives::load_order(
        game,
        &virtual_data.archives(game, &data_dir)?,
        &active_plugins,
    );
    let loaded_archives = read_archives(
        &archive_names,
        &mut facts,
        virtual_data,
        &data_dir,
        progress,
    );
    progress.step(ScanStep::Meshes);
    // The meshes in the data folder are what the game draws, except in a
    // game drawn by Unreal Engine from assets of its own (ADR 0065).
    if game.draws_its_data() {
        facts.mesh_overrides = meshes::read_overrides(&loose, &loaded_archives, cache);
        add_meshes(&mut facts, game, &loose, &loaded_archives, cache);
    }

    let mut placed = objects::Collector::new(game);
    let mut land = terrain::Collector::default();
    let mut game_plugins = Vec::new();
    let mut map_markers = MarkerReader::new(game, &loose, &loaded_archives, cache);
    progress.step(ScanStep::Plugins);
    let plugin_total = active_plugins.len();
    for (index, name) in active_plugins.into_iter().enumerate() {
        progress.count(ScanStep::Plugins, index + 1, plugin_total);
        let path = data_path.join(&name);
        let file = virtual_data.resolve(&data_dir, &name);
        let official = game.is_official(&name);
        if official {
            game_plugins.push((name.clone(), file.clone()));
        }
        let (cached_land, reading) = cached_land(&name, &file, official, cache);
        match read_plugin(&file, reading) {
            Ok(plugin) => {
                if plugin.damaged > 0 {
                    facts.unreadable.push(UnreadableFile {
                        path: path.clone(),
                        reason: format!(
                            "{} damaged records or groups were skipped",
                            plugin.damaged
                        ),
                    });
                }
                placed.add(&name, &plugin);
                add_navmeshes(&mut facts.navmeshes, reading, &name, &plugin);
                map_markers.add(&name, &file, official, &plugin);
                land.merge(cached_land.unwrap_or_else(|| {
                    let names = map_markers.names(&name, &plugin);
                    read_land(&name, &file, official, &plugin, &names, cache)
                }));
                facts.plugins.push(PluginFacts {
                    npcs: if game.has_face_meshes() {
                        npc::npc_facts(game, &name, &plugin)
                    } else {
                        Vec::new()
                    },
                    masters: plugin.header.masters.clone(),
                    name,
                    path,
                });
            }
            Err(reason) => facts.unreadable.push(UnreadableFile { path, reason }),
        }
    }
    add_game_versions(
        &mut placed,
        &game_plugins,
        &data_path,
        &mut facts.unreadable,
    );
    facts.objects = placed.finish();
    let shipped = crate::locate::shipped_plugins(game, game_dir);
    facts.terrain = land.finish(game, &|plugin| origin(game, &shipped, plugin));
    name_worldspaces(&mut facts.objects, &facts.terrain);
    facts.markers = map_markers.finish();

    Ok(facts)
}

/// Under which name the markers of a plugin are kept in the cache. Raised
/// when the way they are read changes, so that nothing read the old way
/// is shown again (the second version prefers Fallout 4's English names).
const MARKER_CACHE: &str = "markers-2";

/// Gathers the map markers of the plugins, with the files their names
/// come from.
struct MarkerReader<'a> {
    /// The loose files, for string tables.
    loose: &'a [(GamePath, PathBuf)],
    /// The archives, for string tables.
    archives: &'a [LoadedArchive],
    /// Keeps the markers of the game's own plugins.
    cache: &'a dyn TextCache,
    collector: markers::Collector,
}

impl<'a> MarkerReader<'a> {
    fn new(
        game: Game,
        loose: &'a [(GamePath, PathBuf)],
        archives: &'a [LoadedArchive],
        cache: &'a dyn TextCache,
    ) -> Self {
        Self {
            loose,
            archives,
            cache,
            collector: markers::Collector::new(game),
        }
    }

    /// Takes in the map markers of the plugin `name` at `file`, already
    /// read as `plugin`. The game's own plugins were not read for their
    /// places; their markers come from the cache or a reading of their own.
    fn add(&mut self, name: &str, file: &Path, official: bool, plugin: &plugin::Plugin) {
        let contribution = if official {
            self.official(name, file).unwrap_or_default()
        } else {
            markers::Contribution::of(name, plugin, &self.names(name, plugin))
        };
        self.collector.merge(name, contribution);
    }

    /// The markers of all plugins taken in.
    fn finish(self) -> MarkerFacts {
        self.collector.finish()
    }

    /// The names of a localized plugin's strings; empty for any other.
    fn names(&self, name: &str, plugin: &plugin::Plugin) -> HashMap<u32, String> {
        if plugin.header.is_localized {
            markers::string_table(name, self.loose, self.archives)
        } else {
            HashMap::new()
        }
    }

    /// The map markers of one of the game's own plugins: from the cache, or
    /// read from its persistent cells and then kept. `None` if the plugin
    /// cannot be read; the main reading of it reports why.
    fn official(&self, name: &str, file: &Path) -> Option<markers::Contribution> {
        let fingerprint = fingerprint(file);
        let key = name.to_lowercase();
        if let Some(fingerprint) = &fingerprint
            && let Some(text) = self.cache.get(MARKER_CACHE, &key, fingerprint)
            && let Some(contribution) = markers::Contribution::from_text(&text)
        {
            return Some(contribution);
        }
        let reader = File::open(file).ok()?;
        let plugin = plugin::read_request_where(
            &mut BufReader::new(reader),
            &plugin::Request {
                flat: &[],
                nested: &markers::MARKER_GROUP,
                nested_records: &markers::MARKER_RECORDS,
                nested_overrides: &[],
            },
            &markers::may_hold_markers,
        )
        .ok()?;
        let contribution = markers::Contribution::of(name, &plugin, &self.names(name, &plugin));
        if let Some(fingerprint) = fingerprint {
            self.cache
                .put(MARKER_CACHE, &key, &fingerprint, &contribution.to_text());
        }
        Some(contribution)
    }
}

/// The file names of the active plugins, in load order, and the file the
/// list was read from.
/// The file in which the game keeps the list of active plugins, without
/// reading it. `None` if that cannot be told for this installation.
pub fn plugin_list_path(game: Game, game_dir: &Path, local_dir: Option<&Path>) -> Option<PathBuf> {
    let settings = load_order_settings(game, game_dir, local_dir).ok()?;
    Some(settings.active_plugins_file().clone())
}

/// The load order library's view of the installation. The library wants
/// the directory the game is installed in, which for Oblivion Remastered
/// lies above the folder the checks read (ADR 0065).
fn load_order_settings(
    game: Game,
    game_dir: &Path,
    local_dir: Option<&Path>,
) -> Result<GameSettings, loadorder::Error> {
    let install = crate::locate::install_dir(game, game_dir);
    match local_dir {
        Some(local_dir) => GameSettings::with_local_path(game.load_order_id(), &install, local_dir),
        None => GameSettings::new(game.load_order_id(), &install),
    }
}

/// The archives of `game` in `data_dir`, by file name.
fn archives_in(game: Game, data_dir: &Path) -> io::Result<Vec<String>> {
    let extension = format!(".{}", game.archive_extension());
    Ok(std::fs::read_dir(data_dir)?
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.to_ascii_lowercase().ends_with(&extension))
        .collect())
}

fn active_plugins(
    game: Game,
    game_dir: &Path,
    local_dir: Option<&Path>,
) -> Result<(Vec<String>, PathBuf), IngestError> {
    active_plugins_in(game, game_dir, local_dir, &[])
}

/// [`active_plugins`], with plugins also in `folders`, the folder whose
/// copy wins first.
fn active_plugins_in(
    game: Game,
    game_dir: &Path,
    local_dir: Option<&Path>,
    folders: &[PathBuf],
) -> Result<(Vec<String>, PathBuf), IngestError> {
    let load_order_error = |error: loadorder::Error| IngestError::LoadOrder(error.to_string());

    let mut settings = load_order_settings(game, game_dir, local_dir).map_err(load_order_error)?;
    if !folders.is_empty() {
        // The load order library takes the first folder that has a plugin.
        settings.set_additional_plugins_directories(folders.to_vec());
    }
    let plugin_list = settings.active_plugins_file().clone();
    let mut load_order = settings.into_load_order();
    load_order.load().map_err(load_order_error)?;
    let names = load_order
        .active_plugin_names()
        .into_iter()
        .map(str::to_owned)
        .collect();
    Ok((names, plugin_list))
}

/// Every ordinary file below `directory`, as a path below `base` and as
/// the file on disk.
///
/// A subdirectory that cannot be listed is skipped; only a failure to list
/// `directory` itself is an error.
fn loose_files(directory: &Path, base: &GamePath) -> io::Result<Vec<(GamePath, PathBuf)>> {
    let mut files = Vec::new();
    let mut pending = vec![(directory.to_path_buf(), base.clone())];
    let mut is_root = true;

    while let Some((directory, path)) = pending.pop() {
        let entries = match std::fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(error) if is_root => return Err(error),
            Err(_) => continue,
        };
        is_root = false;
        for entry in entries.filter_map(Result::ok) {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            let child = path.join(&entry.file_name().to_string_lossy());
            if file_type.is_dir() {
                pending.push((entry.path(), child));
            } else {
                files.push((child, entry.path()));
            }
        }
    }
    Ok(files)
}

/// Reads the file lists of the archives `names`, in load order, as the
/// game sees them in `data_dir` and the folders of `virtual_data`, into
/// `facts`, and returns those that could be read.
fn read_archives(
    names: &[String],
    facts: &mut SetupFacts,
    virtual_data: &VirtualData,
    data_dir: &Path,
    progress: &Progress<'_>,
) -> Vec<LoadedArchive> {
    let data_path = GamePath::new(DATA_DIR);
    let mut loaded = Vec::new();
    progress.step(ScanStep::Archives);
    for (index, name) in names.iter().enumerate() {
        progress.count(ScanStep::Archives, index + 1, names.len());
        let path = data_path.join(name);
        let file = virtual_data.resolve(data_dir, name);
        match archive::read_directory(&file) {
            Ok(directory) => {
                facts.archives.push(ArchiveFacts {
                    path: path.clone(),
                    files: directory
                        .paths()
                        .into_iter()
                        .map(|entry| data_path.join(entry))
                        .collect(),
                });
                loaded.push(LoadedArchive {
                    file,
                    path,
                    directory,
                });
            }
            Err(reason) => facts.unreadable.push(UnreadableFile { path, reason }),
        }
    }
    loaded
}

/// The loose files of the data directory, `in_data`, with the files of
/// `folders` laid over them: of two files with the same path, the game sees
/// the one in the folder that comes first. A folder that cannot be listed
/// adds nothing.
fn merged_loose_files(
    in_data: Vec<(GamePath, PathBuf)>,
    folders: &[PathBuf],
    base: &GamePath,
) -> Vec<(GamePath, PathBuf)> {
    let mut seen: std::collections::HashSet<GamePath> = std::collections::HashSet::new();
    let layers = folders
        .iter()
        .map(|folder| loose_files(folder, base).unwrap_or_default())
        .chain(std::iter::once(in_data));
    layers
        .flatten()
        .filter(|(path, _)| seen.insert(path.clone()))
        .collect()
}

/// The names of the active plugins in load order, as `plugins.txt` in
/// `local_dir` (or its standard place) has them.
pub(crate) fn active_plugin_names(
    game: Game,
    game_dir: &Path,
    local_dir: Option<&Path>,
) -> Result<Vec<String>, IngestError> {
    active_plugins(game, game_dir, local_dir).map(|(names, _)| names)
}

/// The header and the character records of the plugin at `path`.
/// The land of the plugin `name` at `file`, read as `plugin`; `names` are
/// its strings if it is localized. The land of the game's own plugins is
/// kept in `cache`.
fn read_land(
    name: &str,
    file: &Path,
    official: bool,
    plugin: &plugin::Plugin,
    names: &HashMap<u32, String>,
    cache: &dyn TextCache,
) -> terrain::Contribution {
    let contribution = terrain::Contribution::of(name, plugin, names);
    if official && let Some(fingerprint) = fingerprint(file) {
        cache.put(
            LAND_CACHE,
            &name.to_lowercase(),
            &fingerprint,
            &contribution.to_text(),
        );
    }
    contribution
}

/// Whether `plugin` comes with `game`, with the Creation Club content the
/// game lists as its own (`shipped`), or with a mod.
pub(crate) fn origin(game: Game, shipped: &[String], plugin: &str) -> WorldOrigin {
    if game.is_official(plugin) {
        WorldOrigin::Game
    } else if shipped
        .iter()
        .any(|known| known.eq_ignore_ascii_case(plugin))
    {
        WorldOrigin::CreationClub
    } else {
        WorldOrigin::Mod
    }
}

/// Gives every worldspace its internal name among the names of places.
/// The worldspaces of the game's own plugins are named there only where a
/// mod changes their record; their land, kept in the cache, names them all.
fn name_worldspaces(objects: &mut ObjectFacts, terrain: &TerrainFacts) {
    for world in &terrain.worldspaces {
        if let Some(editor_id) = &world.editor_id
            && !objects.place_names.iter().any(|name| name.id == world.id)
        {
            objects.place_names.push(PlaceName {
                id: world.id.clone(),
                editor_id: editor_id.clone(),
            });
        }
    }
    objects.place_names.sort_by(|a, b| a.id.cmp(&b.id));
}

/// Under which name the land of a plugin is kept in the cache. Raised when
/// what is kept changes: the second version keeps the worldspaces' own
/// records too, which the game's plugins are otherwise not read for once
/// their land is kept.
const LAND_CACHE: &str = "land-2";

/// How much of a plugin is read.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Reading {
    /// Characters and base objects only.
    Records,
    /// Also the land of its worldspaces.
    Land,
    /// Also its cells and what it places in the world.
    Everything,
}

/// The size and modification time of a file, which change when it does.
fn fingerprint(path: &Path) -> Option<String> {
    let metadata = std::fs::metadata(path).ok()?;
    let modified = metadata
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs();
    Some(format!("{}:{modified}", metadata.len()))
}

/// The land of one of the game's own plugins from the cache, if it is
/// there, and how much of the plugin then has to be read. The land of the
/// game's plugins is large and does not change; it is read once and then
/// taken from the cache.
fn cached_land(
    name: &str,
    file: &Path,
    official: bool,
    cache: &dyn TextCache,
) -> (Option<terrain::Contribution>, Reading) {
    if !official {
        return (None, Reading::Everything);
    }
    let cached = fingerprint(file)
        .and_then(|fingerprint| cache.get(LAND_CACHE, &name.to_lowercase(), &fingerprint))
        .and_then(|text| terrain::Contribution::from_text(&text));
    let reading = if cached.is_some() {
        Reading::Records
    } else {
        Reading::Land
    };
    (cached, reading)
}

/// Reads the game's own versions of the objects that mods change, so that
/// mods that move objects of the game are told apart from mods that only
/// change them. `game_plugins` are the game's plugins in load order, with
/// the file each lies in.
fn add_game_versions(
    placed: &mut objects::Collector,
    game_plugins: &[(String, PathBuf)],
    data_path: &GamePath,
    unreadable: &mut Vec<UnreadableFile>,
) {
    if !placed.wants_game_versions() {
        return;
    }
    for (name, file) in game_plugins {
        match read_references(file) {
            Ok(plugin) => placed.add_game_versions(name, &plugin),
            Err(reason) => unreadable.push(UnreadableFile {
                path: data_path.join(name),
                reason,
            }),
        }
    }
}

/// Adds what the meshes of mods, loose and in `archives`, say about
/// themselves: which variant of the format they are in, and which textures
/// they name.
fn add_meshes(
    facts: &mut SetupFacts,
    game: Game,
    loose: &[(GamePath, PathBuf)],
    archives: &[LoadedArchive],
    cache: &dyn TextCache,
) {
    let mut meshes = loose_meshes::read(game, loose, cache);
    archive_meshes::read(game, archives, cache, &mut meshes);
    facts.foreign_meshes = meshes.foreign;
    facts.mesh_textures = meshes.textures;
}

/// Adds the navigation meshes of other plugins that `plugin` changes, if
/// it was read with its places.
fn add_navmeshes(facts: &mut NavmeshFacts, reading: Reading, name: &str, plugin: &plugin::Plugin) {
    if reading == Reading::Everything {
        facts.changes.extend(navmeshes::changes(name, plugin));
    }
}

/// Reads only the placed objects of a plugin.
fn read_references(path: &Path) -> Result<plugin::Plugin, String> {
    let file = File::open(path).map_err(|error| error.to_string())?;
    let request = plugin::Request {
        flat: &[],
        nested: &objects::PLACE_GROUPS,
        nested_records: &[*b"REFR"],
        nested_overrides: &[],
    };
    plugin::read_request(&mut BufReader::new(file), &request).map_err(|error| error.to_string())
}

/// Reads the characters and base objects of a plugin, and as much more as
/// `reading` says.
fn read_plugin(path: &Path, reading: Reading) -> Result<plugin::Plugin, String> {
    let file = File::open(path).map_err(|error| error.to_string())?;
    let flat: Vec<plugin::Signature> = std::iter::once(npc::NPC)
        .chain(objects::wanted_base_types())
        .collect();
    let places: Vec<plugin::Signature> = objects::PLACE_RECORDS
        .into_iter()
        .chain(navmeshes::RECORDS)
        .collect();
    let request = plugin::Request {
        flat: &flat,
        nested: match reading {
            Reading::Everything => &objects::PLACE_GROUPS,
            Reading::Land => &objects::WORLD_GROUP,
            Reading::Records => &[],
        },
        nested_records: if reading == Reading::Everything {
            &places
        } else {
            &objects::LAND_RECORDS
        },
        nested_overrides: &[],
    };
    plugin::read_request(&mut BufReader::new(file), &request).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use mallet_core::NoCache;
    use mallet_formats_bethesda::fixture::{self, RecordSpec};

    use super::*;

    const MASTER_FLAG: u32 = 1;

    /// A made-up installation: a game directory and the directory that
    /// holds `plugins.txt`.
    struct Installation {
        root: tempfile::TempDir,
    }

    impl Installation {
        fn new(active: &[&str]) -> Self {
            let installation = Self {
                root: tempfile::tempdir().unwrap(),
            };
            fs::create_dir_all(installation.data_dir()).unwrap();
            fs::create_dir_all(installation.local_dir()).unwrap();
            installation.write_data("Skyrim.esm", &fixture::plugin(&[], MASTER_FLAG, &[]));
            let lines = active
                .iter()
                .fold(String::new(), |lines, name| lines + "*" + name + "\n");
            fs::write(installation.local_dir().join("plugins.txt"), lines).unwrap();
            installation
        }

        fn game_dir(&self) -> std::path::PathBuf {
            self.root.path().join("Game")
        }

        fn data_dir(&self) -> std::path::PathBuf {
            self.game_dir().join("Data")
        }

        fn local_dir(&self) -> std::path::PathBuf {
            self.root.path().join("Local")
        }

        fn write_data(&self, relative: &str, content: &[u8]) {
            let path = self.data_dir().join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, content).unwrap();
        }

        fn read(&self) -> SetupFacts {
            read_setup(
                Game::SkyrimSe,
                &self.game_dir(),
                Some(&self.local_dir()),
                &NoCache,
            )
            .unwrap()
        }
    }

    fn names(plugins: &[PluginFacts]) -> Vec<&str> {
        plugins.iter().map(|plugin| plugin.name.as_str()).collect()
    }

    #[test]
    fn reads_the_active_plugins_in_load_order() {
        let installation = Installation::new(&["Second.esp", "First.esp"]);
        let plugin = fixture::plugin(&["Skyrim.esm"], 0, &[]);
        installation.write_data("First.esp", &plugin);
        installation.write_data("Second.esp", &plugin);
        installation.write_data("Inactive.esp", &plugin);

        let facts = installation.read();
        assert_eq!(
            names(&facts.plugins),
            ["Skyrim.esm", "Second.esp", "First.esp"]
        );
        assert_eq!(facts.plugins[1].path, GamePath::new("data/second.esp"));
        assert_eq!(facts.unreadable, Vec::new());
    }

    #[test]
    fn reads_the_characters_of_a_plugin() {
        let installation = Installation::new(&["Faces.esp"]);
        installation.write_data(
            "Faces.esp",
            &fixture::plugin(
                &["Skyrim.esm"],
                0,
                &[(
                    npc::NPC,
                    vec![
                        RecordSpec::new(npc::NPC, 0x0001_3BA1)
                            .with(*b"EDID", fixture::zstring("Uthgerd")),
                    ],
                )],
            ),
        );

        let facts = installation.read();
        let npcs = &facts.plugins.last().unwrap().npcs;
        assert_eq!(npcs.len(), 1);
        assert_eq!(npcs[0].editor_id.as_deref(), Some("Uthgerd"));
        assert_eq!(npcs[0].id.plugin, "skyrim.esm");
    }

    #[test]
    fn lists_loose_files_below_the_data_directory() {
        let installation = Installation::new(&[]);
        installation.write_data("Textures/Sky/Clouds.dds", b"x");

        let facts = installation.read();
        assert!(
            facts
                .loose_files
                .contains(&GamePath::new("data/textures/sky/clouds.dds"))
        );
        assert!(
            facts
                .loose_files
                .contains(&GamePath::new("data/skyrim.esm"))
        );
    }

    #[test]
    fn reads_the_archives_of_active_plugins_in_order() {
        let installation = Installation::new(&["Grass.esp"]);
        installation.write_data("Grass.esp", &fixture::plugin(&["Skyrim.esm"], 0, &[]));
        installation.write_data("Grass.bsa", &fixture::bsa(&[r"meshes\grass\fern.nif"]));
        installation.write_data("Skyrim - Meshes0.bsa", &fixture::bsa(&[r"meshes\rock.nif"]));
        installation.write_data("Orphan.bsa", &fixture::bsa(&[r"meshes\unused.nif"]));

        let facts = installation.read();
        let paths: Vec<_> = facts.archives.iter().map(|a| a.path.as_str()).collect();
        assert_eq!(paths, ["data/skyrim - meshes0.bsa", "data/grass.bsa"]);
        assert_eq!(
            facts.archives[1].files,
            [GamePath::new("data/meshes/grass/fern.nif")]
        );
    }

    /// Mod Organizer leaves the data directory alone and shows the game
    /// the mods' folders instead (ADR 0064).
    #[test]
    fn plugins_archives_and_loose_files_come_from_the_virtual_folders() {
        let installation = Installation::new(&["Grass.esp", "Patch.esp"]);
        installation.write_data("Textures/Sky.dds", b"game");
        let mods = installation.root.path().join("mods");
        let write_mod = |relative: &str, content: &[u8]| {
            let path = mods.join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, content).unwrap();
        };
        let plugin = fixture::plugin(&["Skyrim.esm"], 0, &[]);
        write_mod("Grass/Grass.esp", &plugin);
        write_mod(
            "Grass/Grass.bsa",
            &fixture::bsa(&[r"meshes\grass\fern.nif"]),
        );
        write_mod("Grass/Textures/Sky.dds", b"grass");
        write_mod("Patch/Patch.esp", &plugin);
        write_mod("Patch/Textures/Sky.dds", b"patch");
        let virtual_data = VirtualData {
            folders: vec![mods.join("Patch"), mods.join("Grass")],
        };

        let facts = read_setup_in(
            Game::SkyrimSe,
            &installation.game_dir(),
            Some(&installation.local_dir()),
            &virtual_data,
            &NoCache,
            &Progress::none(),
        )
        .unwrap();
        assert_eq!(
            names(&facts.plugins),
            ["Skyrim.esm", "Grass.esp", "Patch.esp"]
        );
        assert_eq!(facts.unreadable, Vec::new());
        let paths: Vec<_> = facts.archives.iter().map(|a| a.path.as_str()).collect();
        assert_eq!(paths, ["data/grass.bsa"]);
        // One path, seen once, though three folders have it.
        let skies = facts
            .loose_files
            .iter()
            .filter(|path| path.as_str() == "data/textures/sky.dds")
            .count();
        assert_eq!(skies, 1);
        assert!(
            facts
                .loose_files
                .contains(&GamePath::new("data/skyrim.esm"))
        );
        // Without the folders, the plugins are not in the game.
        assert_eq!(names(&installation.read().plugins), ["Skyrim.esm"]);
    }

    #[test]
    fn the_first_folder_with_a_file_wins() {
        let root = tempfile::tempdir().unwrap();
        let (high, low) = (root.path().join("high"), root.path().join("low"));
        for folder in [&high, &low] {
            fs::create_dir_all(folder.join("textures")).unwrap();
            fs::write(folder.join("textures").join("sky.dds"), b"x").unwrap();
        }
        let base = GamePath::new(DATA_DIR);
        let merged = merged_loose_files(Vec::new(), &[high.clone(), low], &base);
        assert_eq!(
            merged,
            [(
                GamePath::new("data/textures/sky.dds"),
                high.join("textures").join("sky.dds")
            )]
        );
        let virtual_data = VirtualData {
            folders: vec![root.path().join("missing"), high.join("textures")],
        };
        assert_eq!(
            virtual_data.resolve(root.path(), "sky.dds"),
            high.join("textures").join("sky.dds")
        );
        assert_eq!(
            virtual_data.resolve(root.path(), "other.dds"),
            root.path().join("other.dds")
        );
    }

    /// Writes `content` to `relative` below the data directory of a game
    /// directory in `root`.
    fn write(root: &Path, relative: &str, content: &[u8]) {
        let path = root.join("Game").join("Data").join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    #[test]
    fn fallout_4_is_read_with_its_ba2_archives() {
        let root = tempfile::tempdir().unwrap();
        let local = root.path().join("Local");
        fs::create_dir_all(&local).unwrap();
        fs::write(local.join("plugins.txt"), "*Town.esp\n").unwrap();
        write(
            root.path(),
            "Fallout4.esm",
            &fixture::plugin(&[], MASTER_FLAG, &[]),
        );
        write(
            root.path(),
            "Town.esp",
            &fixture::plugin(&["Fallout4.esm"], 0, &[]),
        );
        write(
            root.path(),
            "Town - Main.ba2",
            &fixture::ba2(&[r"meshes\town\hall.nif"]),
        );
        write(
            root.path(),
            "Town.bsa",
            &fixture::bsa(&[r"meshes\ignored.nif"]),
        );

        let facts = read_setup(
            Game::Fallout4,
            &root.path().join("Game"),
            Some(&local),
            &NoCache,
        )
        .unwrap();

        assert_eq!(names(&facts.plugins), ["Fallout4.esm", "Town.esp"]);
        let paths: Vec<_> = facts.archives.iter().map(|a| a.path.as_str()).collect();
        assert_eq!(paths, ["data/town - main.ba2"]);
        assert_eq!(
            facts.archives[0].files,
            [GamePath::new("data/meshes/town/hall.nif")]
        );
        assert_eq!(facts.unreadable, Vec::new());
    }

    #[test]
    fn oblivion_plugins_with_short_headers_are_read() {
        let root = tempfile::tempdir().unwrap();
        let local = root.path().join("Local");
        fs::create_dir_all(&local).unwrap();
        fs::write(local.join("plugins.txt"), "Oblivion.esm\nCastle.esp\n").unwrap();
        write(
            root.path(),
            "Oblivion.esm",
            &fixture::oblivion::plugin(&[], MASTER_FLAG, &[]),
        );
        write(
            root.path(),
            "Castle.esp",
            &fixture::oblivion::plugin(&["Oblivion.esm"], 0, &[]),
        );
        write(
            root.path(),
            "Castle - Meshes.bsa",
            &fixture::bsa_with_version(&[r"meshes\castle.nif"], 103),
        );

        let facts = read_setup(
            Game::Oblivion,
            &root.path().join("Game"),
            Some(&local),
            &NoCache,
        )
        .unwrap();

        assert_eq!(names(&facts.plugins), ["Oblivion.esm", "Castle.esp"]);
        assert_eq!(facts.plugins[1].masters, ["Oblivion.esm"]);
        let paths: Vec<_> = facts.archives.iter().map(|a| a.path.as_str()).collect();
        assert_eq!(paths, ["data/castle - meshes.bsa"]);
        assert_eq!(facts.unreadable, Vec::new());
    }

    #[test]
    fn oblivion_remastered_reads_its_plugin_list_from_the_old_games_data_folder() {
        // The shape of the real folders: the old game's data four folders
        // below the installation, with the list of plugins inside it.
        let install = tempfile::tempdir().unwrap();
        fs::write(install.path().join("OblivionRemastered.exe"), b"").unwrap();
        let root = install
            .path()
            .join("OblivionRemastered/Content/Dev/ObvData");
        let data = root.join("Data");
        fs::create_dir_all(&data).unwrap();
        fs::write(root.join("Oblivion.ini"), "[Archive]\n").unwrap();
        fs::write(
            data.join("Plugins.txt"),
            "## written by a mod manager\r\nOblivion.esm\r\nAltarESPMain.esp\r\nLanterns.esp\r\n#Switched Off.esp\r\n",
        )
        .unwrap();
        let master = fixture::oblivion::plugin(&[], MASTER_FLAG, &[]);
        let plugin = fixture::oblivion::plugin(&["Oblivion.esm"], 0, &[]);
        fs::write(data.join("Oblivion.esm"), &master).unwrap();
        fs::write(data.join("AltarESPMain.esp"), &plugin).unwrap();
        fs::write(data.join("Lanterns.esp"), &plugin).unwrap();
        fs::write(data.join("Switched Off.esp"), &plugin).unwrap();
        fs::write(
            data.join("Lanterns.bsa"),
            fixture::bsa_with_version(&[r"meshes\lantern.nif"], 103),
        )
        .unwrap();
        fs::write(data.join("meshes.nif"), b"not drawn").unwrap();

        let facts = read_setup(Game::OblivionRemastered, &root, None, &NoCache).unwrap();

        assert_eq!(
            names(&facts.plugins),
            ["Oblivion.esm", "AltarESPMain.esp", "Lanterns.esp"]
        );
        assert_eq!(facts.plugins[2].path, GamePath::new("data/lanterns.esp"));
        assert_eq!(facts.plugins[2].masters, ["Oblivion.esm"]);
        assert_eq!(facts.plugin_list, data.join("Plugins.txt"));
        let paths: Vec<_> = facts.archives.iter().map(|a| a.path.as_str()).collect();
        assert_eq!(paths, ["data/lanterns.bsa"]);
        // Unreal Engine draws the game; its meshes are not looked at.
        assert_eq!(facts.mesh_textures, Vec::new());
        assert_eq!(facts.mesh_overrides, Vec::new());
        assert_eq!(facts.unreadable, Vec::new());
    }

    #[test]
    fn a_damaged_archive_is_recorded_and_the_rest_is_still_read() {
        let installation = Installation::new(&["Broken.esp", "Fine.esp"]);
        let plugin = fixture::plugin(&["Skyrim.esm"], 0, &[]);
        installation.write_data("Broken.esp", &plugin);
        installation.write_data("Fine.esp", &plugin);
        installation.write_data("Broken.bsa", b"this is not an archive, just some text");
        installation.write_data("Fine.bsa", &fixture::bsa(&[r"meshes\a.nif"]));

        let facts = installation.read();
        assert_eq!(facts.archives.len(), 1);
        assert_eq!(facts.unreadable.len(), 1);
        assert_eq!(facts.unreadable[0].path, GamePath::new("data/broken.bsa"));
        assert_eq!(facts.unreadable[0].reason, "the file is not a BSA archive");
    }

    #[test]
    fn a_missing_game_directory_is_an_error() {
        let root = tempfile::tempdir().unwrap();
        let result = read_setup(
            Game::SkyrimSe,
            &root.path().join("Nowhere"),
            Some(root.path()),
            &NoCache,
        );
        assert!(result.is_err());
    }
}
