//! The level-of-detail files of a worldspace, in the versions the game
//! uses: the terrain meshes and textures it shows from a distance, and the
//! reduced objects standing on them (ADR 0037).
//!
//! The files are found once, in the archives in load order and then the
//! loose files, and then read on demand. Nothing is written.

use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};

use crate::archive::{self, Directory};
use crate::game::Game;
use crate::{archives, files, ingest};

/// The levels of detail the game uses, coarsest first: how many cells a
/// block covers in each direction.
pub const LEVELS: [u32; 4] = [32, 16, 8, 4];

/// Loose files larger than this are not read.
const MAX_LOOSE: u64 = 64 * 1024 * 1024;

/// Where a file is read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Source {
    Loose,
    Archive { archive: usize, index: usize },
}

/// One block of level of detail, as the files name it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Block {
    /// Cells per side: one of [`LEVELS`].
    pub level: u32,
    /// The cell column at the south-west corner.
    pub x: i32,
    /// The cell row at the south-west corner.
    pub y: i32,
}

/// The level-of-detail files of one worldspace.
#[derive(Debug, Default)]
pub struct LodLibrary {
    data_dir: PathBuf,
    world: String,
    archives: Vec<(PathBuf, Directory)>,
    files: HashMap<String, Source>,
}

impl LodLibrary {
    /// Finds the level-of-detail files of `world` (its internal name, such
    /// as `Tamriel`) in the setup of `game` in `game_dir`. `local_dir`
    /// holds `plugins.txt`, as for [`ingest::read_setup`].
    pub fn open(game: Game, game_dir: &Path, local_dir: Option<&Path>, world: &str) -> Self {
        let world = world.to_ascii_lowercase();
        let data_dir = game_dir.join("Data");
        let prefixes = [
            format!("meshes/terrain/{world}/"),
            format!("textures/terrain/{world}/"),
        ];
        let wanted = |path: &str| prefixes.iter().any(|prefix| path.starts_with(prefix));
        let mut library = Self {
            data_dir: data_dir.clone(),
            world,
            ..Self::default()
        };

        let active = ingest::active_plugin_names(game, game_dir, local_dir).unwrap_or_default();
        let extension = format!(".{}", game.archive_extension());
        let on_disk: Vec<String> = std::fs::read_dir(&data_dir).map_or_else(
            |_| Vec::new(),
            |entries| {
                entries
                    .filter_map(Result::ok)
                    .filter_map(|entry| entry.file_name().into_string().ok())
                    .filter(|name| name.to_ascii_lowercase().ends_with(&extension))
                    .collect()
            },
        );
        for name in archives::load_order(game, &on_disk, &active) {
            let path = data_dir.join(&name);
            let Ok(directory) = archive::read_directory(&path) else {
                continue;
            };
            let archive = library.archives.len();
            let mut used = false;
            for (index, entry) in directory.paths().into_iter().enumerate() {
                let normalized = files::normalize(entry);
                if wanted(&normalized) {
                    library
                        .files
                        .insert(normalized, Source::Archive { archive, index });
                    used = true;
                }
            }
            if used {
                library.archives.push((path, directory));
            }
        }
        for prefix in &prefixes {
            for file in loose_files(&data_dir, prefix) {
                library.files.insert(file, Source::Loose);
            }
        }
        library
    }

    /// The blocks that have land, and those that have objects.
    pub fn blocks(&self) -> (Vec<Block>, Vec<Block>) {
        let terrain = format!("meshes/terrain/{0}/{0}.", self.world);
        let objects = format!("meshes/terrain/{0}/objects/{0}.", self.world);
        let mut land = Vec::new();
        let mut things = Vec::new();
        for path in self.files.keys() {
            if let Some(rest) = path.strip_prefix(&objects) {
                things.extend(block_of(rest, ".bto"));
            } else if let Some(rest) = path.strip_prefix(&terrain) {
                land.extend(block_of(rest, ".btr"));
            }
        }
        land.sort_unstable();
        things.sort_unstable();
        (land, things)
    }

    /// The land of `block`, as the game's mesh file.
    pub fn terrain(&self, block: Block) -> Option<Vec<u8>> {
        self.read(&format!(
            "meshes/terrain/{0}/{0}.{1}.{2}.{3}.btr",
            self.world, block.level, block.x, block.y
        ))
    }

    /// The objects of `block`, as the game's mesh file.
    pub fn objects(&self, block: Block) -> Option<Vec<u8>> {
        self.read(&format!(
            "meshes/terrain/{0}/objects/{0}.{1}.{2}.{3}.bto",
            self.world, block.level, block.x, block.y
        ))
    }

    /// The file at `path` below the data directory, if this library has
    /// it: a file of the worldspace's level of detail.
    pub fn read(&self, path: &str) -> Option<Vec<u8>> {
        let path = files::normalize(path);
        match *self.files.get(&path)? {
            Source::Loose => {
                let file = self.data_dir.join(&path);
                let small = std::fs::metadata(&file).is_ok_and(|meta| meta.len() <= MAX_LOOSE);
                small.then(|| std::fs::read(&file).ok()).flatten()
            }
            Source::Archive { archive, index } => {
                let (path, directory) = self.archives.get(archive)?;
                let mut file = File::open(path).ok()?;
                directory.read(&mut file, index)
            }
        }
    }
}

/// The block a file name such as `16.-32.0.btr` stands for.
fn block_of(rest: &str, extension: &str) -> Option<Block> {
    let name = rest.strip_suffix(extension)?;
    let mut parts = name.split('.');
    let level = parts.next()?.parse().ok()?;
    let x = parts.next()?.parse().ok()?;
    let y = parts.next()?.parse().ok()?;
    (parts.next().is_none() && LEVELS.contains(&level)).then_some(Block { level, x, y })
}

/// The loose files below `prefix` (normalized, ending in `/`) in
/// `data_dir`, normalized. Folder names are matched without regard to case.
fn loose_files(data_dir: &Path, prefix: &str) -> Vec<String> {
    let mut folder = data_dir.to_path_buf();
    for part in prefix.split('/').filter(|part| !part.is_empty()) {
        let found = std::fs::read_dir(&folder).ok().and_then(|entries| {
            entries.filter_map(Result::ok).find_map(|entry| {
                (entry.file_name().to_string_lossy().to_ascii_lowercase() == part)
                    .then(|| entry.path())
            })
        });
        let Some(found) = found else {
            return Vec::new();
        };
        folder = found;
    }
    let mut found = Vec::new();
    let mut pending = vec![(folder, prefix.to_owned())];
    while let Some((folder, name)) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&folder) else {
            continue;
        };
        for entry in entries.filter_map(Result::ok) {
            let child = format!(
                "{name}{}",
                entry.file_name().to_string_lossy().to_ascii_lowercase()
            );
            match entry.file_type() {
                Ok(kind) if kind.is_dir() => pending.push((entry.path(), format!("{child}/"))),
                Ok(kind) if kind.is_file() => found.push(child),
                _ => {}
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_names_are_read_and_others_are_not() {
        assert_eq!(
            block_of("16.-32.0.btr", ".btr"),
            Some(Block {
                level: 16,
                x: -32,
                y: 0
            })
        );
        assert_eq!(block_of("3.0.0.btr", ".btr"), None);
        assert_eq!(block_of("4.0.0.btr", ".bto"), None);
        assert_eq!(block_of("4.0.0.1.btr", ".btr"), None);
    }

    #[test]
    fn loose_files_are_found_and_win_over_nothing() {
        let game = tempfile::tempdir().unwrap();
        let folder = game.path().join("Data/Meshes/Terrain/Tamriel/Objects");
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join("Tamriel.4.0.0.bto"), b"objects").unwrap();
        std::fs::write(
            game.path()
                .join("Data/Meshes/Terrain/Tamriel/Tamriel.8.-8.0.btr"),
            b"land",
        )
        .unwrap();
        let library = LodLibrary::open(Game::SkyrimSe, game.path(), None, "Tamriel");
        let (land, things) = library.blocks();
        assert_eq!(
            land,
            [Block {
                level: 8,
                x: -8,
                y: 0
            }]
        );
        assert_eq!(
            things,
            [Block {
                level: 4,
                x: 0,
                y: 0
            }]
        );
        assert_eq!(library.objects(things[0]).unwrap(), b"objects");
        assert_eq!(library.read("textures/terrain/tamriel/none.dds"), None);
        // Files of other places are not this library's to read.
        assert_eq!(library.read("../secret.txt"), None);
    }
}
