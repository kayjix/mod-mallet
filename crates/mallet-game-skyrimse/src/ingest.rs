//! Reads a Skyrim Special Edition installation into facts.
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

use std::fs::File;
use std::io::{self, BufReader, Read};
use std::path::{Path, PathBuf};

use loadorder::{GameId, GameSettings};
use mallet_core::facts::{
    ArchiveFacts, MeshOverride, ObjectFacts, PluginFacts, TerrainFacts, UnreadableFile,
};
use mallet_core::{GamePath, TextCache};
use mallet_formats_bethesda::{bsa, plugin};

use crate::meshes::{self, LoadedArchive};
use crate::{DATA_DIR, archives, npc, objects, terrain};

/// The largest archive file list that is read. Far above any real archive;
/// it keeps a damaged header from asking for gigabytes of memory.
const MAX_LISTING: u64 = 512 * 1024 * 1024;

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
    /// The objects mods placed in the world.
    pub objects: ObjectFacts,
    /// The shape of the land.
    pub terrain: TerrainFacts,
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

/// Reads the installation in `game_dir`.
///
/// `local_dir` is the directory that holds `plugins.txt`. It is detected if
/// `None`, which works on Windows only. `cache` keeps what is slow to read
/// until the files it came from change.
pub fn read_setup(
    game_dir: &Path,
    local_dir: Option<&Path>,
    cache: &dyn TextCache,
) -> Result<SetupFacts, IngestError> {
    let (active_plugins, plugin_list) = active_plugins(game_dir, local_dir)?;
    let data_dir = game_dir.join("Data");
    let data_path = GamePath::new(DATA_DIR);

    let loose = loose_files(&data_dir, &data_path)?;
    let mut facts = SetupFacts {
        plugin_list,
        loose_files: loose.iter().map(|(path, _)| path.clone()).collect(),
        ..SetupFacts::default()
    };
    let mut loaded_archives = Vec::new();

    let archives_on_disk: Vec<String> = std::fs::read_dir(&data_dir)?
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.to_ascii_lowercase().ends_with(".bsa"))
        .collect();
    for name in archives::load_order(&archives_on_disk, &active_plugins) {
        let path = data_path.join(&name);
        let file = data_dir.join(&name);
        match read_archive_directory(&file) {
            Ok(directory) => {
                facts.archives.push(ArchiveFacts {
                    path: path.clone(),
                    files: directory
                        .entries
                        .iter()
                        .map(|entry| data_path.join(&entry.path))
                        .collect(),
                });
                loaded_archives.push(LoadedArchive {
                    file,
                    path,
                    directory,
                });
            }
            Err(reason) => facts.unreadable.push(UnreadableFile { path, reason }),
        }
    }
    facts.mesh_overrides = meshes::read_overrides(&loose, &loaded_archives, cache);

    let mut placed = objects::Collector::default();
    let mut land = terrain::Collector::default();
    for name in active_plugins {
        let path = data_path.join(&name);
        let file = data_dir.join(&name);
        let official = objects::is_official(&name);
        // The land of the game's own plugins is large and does not change;
        // it is read once and then taken from the cache.
        let cached_land = official
            .then(|| {
                let fingerprint = fingerprint(&file)?;
                let text = cache.get(LAND_CACHE, &name.to_lowercase(), &fingerprint)?;
                terrain::Contribution::from_text(&text)
            })
            .flatten();
        let reading = if !official {
            Reading::Everything
        } else if cached_land.is_some() {
            Reading::Records
        } else {
            Reading::Land
        };
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
                if let Some(contribution) = cached_land {
                    land.merge(contribution);
                } else {
                    let contribution = terrain::Contribution::of(&name, &plugin);
                    if official && let Some(fingerprint) = fingerprint(&file) {
                        cache.put(
                            LAND_CACHE,
                            &name.to_lowercase(),
                            &fingerprint,
                            &contribution.to_text(),
                        );
                    }
                    land.merge(contribution);
                }
                facts.plugins.push(PluginFacts {
                    npcs: npc::npc_facts(&name, &plugin),
                    masters: plugin.header.masters.clone(),
                    name,
                    path,
                });
            }
            Err(reason) => facts.unreadable.push(UnreadableFile { path, reason }),
        }
    }
    facts.objects = placed.finish();
    facts.terrain = land.finish();

    Ok(facts)
}

/// The file names of the active plugins, in load order, and the file the
/// list was read from.
/// The file in which the game keeps the list of active plugins, without
/// reading it. `None` if that cannot be told for this installation.
pub fn plugin_list_path(game_dir: &Path, local_dir: Option<&Path>) -> Option<PathBuf> {
    let settings = match local_dir {
        Some(local_dir) => GameSettings::with_local_path(GameId::SkyrimSE, game_dir, local_dir),
        None => GameSettings::new(GameId::SkyrimSE, game_dir),
    }
    .ok()?;
    Some(settings.active_plugins_file().clone())
}

fn active_plugins(
    game_dir: &Path,
    local_dir: Option<&Path>,
) -> Result<(Vec<String>, PathBuf), IngestError> {
    let load_order_error = |error: loadorder::Error| IngestError::LoadOrder(error.to_string());

    let settings = match local_dir {
        Some(local_dir) => GameSettings::with_local_path(GameId::SkyrimSE, game_dir, local_dir),
        None => GameSettings::new(GameId::SkyrimSE, game_dir),
    }
    .map_err(load_order_error)?;
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

/// The directory of the archive at `path`.
fn read_archive_directory(path: &Path) -> Result<bsa::Directory, String> {
    let read = || -> io::Result<Result<bsa::Directory, bsa::BsaError>> {
        let mut file = File::open(path)?;
        let mut data = vec![0u8; bsa::HEADER_LEN];
        file.read_exact(&mut data)?;
        let listing_len = match bsa::listing_len(&data) {
            Ok(len) => len.min(MAX_LISTING),
            Err(error) => return Ok(Err(error)),
        };
        let rest = listing_len.saturating_sub(bsa::HEADER_LEN as u64);
        file.take(rest).read_to_end(&mut data)?;
        Ok(bsa::parse_directory(&data))
    };
    match read() {
        Ok(Ok(directory)) => Ok(directory),
        Ok(Err(error)) => Err(error.to_string()),
        Err(error) => Err(error.to_string()),
    }
}

/// The header and the character records of the plugin at `path`.
/// Under which name the land of a plugin is kept in the cache.
const LAND_CACHE: &str = "land";

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

/// Reads the characters and base objects of a plugin, and as much more as
/// `reading` says.
fn read_plugin(path: &Path, reading: Reading) -> Result<plugin::Plugin, String> {
    let file = File::open(path).map_err(|error| error.to_string())?;
    let flat: Vec<plugin::Signature> = std::iter::once(npc::NPC)
        .chain(objects::wanted_base_types())
        .collect();
    let request = plugin::Request {
        flat: &flat,
        nested: match reading {
            Reading::Everything => &objects::PLACE_GROUPS,
            Reading::Land => &objects::WORLD_GROUP,
            Reading::Records => &[],
        },
        nested_records: if reading == Reading::Everything {
            &objects::PLACE_RECORDS
        } else {
            &objects::LAND_RECORDS
        },
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
            read_setup(&self.game_dir(), Some(&self.local_dir()), &NoCache).unwrap()
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
        let result = read_setup(&root.path().join("Nowhere"), Some(root.path()), &NoCache);
        assert!(result.is_err());
    }
}
