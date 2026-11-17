//! Loose meshes that replace a mesh from an archive, and the textures both
//! versions use.
//!
//! Only meshes that exist twice are read: once as a loose file and once in
//! an archive. For each, the loose file and the version in the archive are
//! opened just far enough to list their texture paths. That is what an
//! analysis needs to tell whether a replacement mesh still uses the
//! textures a texture mod replaces.
//!
//! A large setup has thousands of such meshes, and opening them all takes
//! a minute or more when the disk has not seen them recently. The result
//! for each mesh is therefore kept in a cache under the size and date of
//! both files, and read again only when one of them changed.

use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use mallet_core::facts::MeshOverride;
use mallet_core::{GamePath, TextCache};
use mallet_formats_bethesda::nif;

use crate::{DATA_DIR, archive};

/// File extension of meshes.
const MESH_EXTENSION: &str = ".nif";

/// Under which name the textures of meshes are kept in the cache.
const CACHE_NAMESPACE: &str = "mesh-textures";

/// Separates the textures of the loose mesh from those of the replaced one
/// in a cached entry. Each texture is on a line of its own.
const CACHE_SEPARATOR: &str = "\n|\n";

/// An archive that was read: where it lies, and its directory.
pub(crate) struct LoadedArchive {
    /// The archive on disk.
    pub(crate) file: PathBuf,
    /// The archive's path in the game.
    pub(crate) path: GamePath,
    /// Its directory.
    pub(crate) directory: archive::Directory,
}

/// The game path a texture path in a mesh refers to.
///
/// Meshes spell these paths in many ways: with or without a leading `data`,
/// sometimes with the drive and folders of the author's machine in front.
/// What counts is the part from `textures` on. `None` if there is no such
/// part.
pub fn texture_path(raw: &str) -> Option<GamePath> {
    let normalized = GamePath::new(raw);
    let path = normalized.as_str();
    let start = if path.starts_with("textures/") {
        0
    } else {
        path.find("/textures/")? + 1
    };
    Some(GamePath::new(DATA_DIR).join(path.get(start..)?))
}

/// Reads every loose mesh that also exists in an archive, together with the
/// version in the archive the game would otherwise use.
///
/// `loose` pairs each loose file's game path with its place on disk.
/// `archives` are the loaded archives from lowest to highest precedence. A
/// mesh that cannot be read, in either version, is left out.
pub(crate) fn read_overrides(
    loose: &[(GamePath, PathBuf)],
    archives: &[LoadedArchive],
    cache: &dyn TextCache,
) -> Vec<MeshOverride> {
    let data_path = GamePath::new(DATA_DIR);

    // For every mesh in an archive, the archive that wins and the entry.
    let mut packed: HashMap<GamePath, (usize, usize)> = HashMap::new();
    for (archive_index, archive) in archives.iter().enumerate() {
        for (entry_index, path) in archive.directory.paths().into_iter().enumerate() {
            if path.to_ascii_lowercase().ends_with(MESH_EXTENSION) {
                packed.insert(data_path.join(path), (archive_index, entry_index));
            }
        }
    }

    let archive_states: Vec<String> = archives
        .iter()
        .map(|archive| file_state(&archive.file).unwrap_or_default())
        .collect();
    let mut open_archives: HashMap<usize, File> = HashMap::new();
    let mut overrides = Vec::new();
    for (path, file) in loose {
        let Some(&(archive_index, entry_index)) = packed.get(path) else {
            continue;
        };
        let Some(archive) = archives.get(archive_index) else {
            continue;
        };
        // The entry changes when the loose file, the archive, or the place
        // of the mesh in the archive does.
        let fingerprint = format!(
            "{};{};{entry_index}",
            file_state(file).unwrap_or_default(),
            archive_states.get(archive_index).map_or("", String::as_str),
        );
        let cached = cache
            .get(CACHE_NAMESPACE, path.as_str(), &fingerprint)
            .and_then(|entry| from_cache_entry(&entry));
        let (textures, replaced_textures) = if let Some(cached) = cached {
            cached
        } else {
            let Some(textures) = loose_textures(file) else {
                continue;
            };
            let Some(replaced_textures) =
                packed_textures(archive, archive_index, entry_index, &mut open_archives)
            else {
                continue;
            };
            cache.put(
                CACHE_NAMESPACE,
                path.as_str(),
                &fingerprint,
                &to_cache_entry(&textures, &replaced_textures),
            );
            (textures, replaced_textures)
        };
        overrides.push(MeshOverride {
            path: path.clone(),
            textures,
            replaced_in: archive.path.clone(),
            replaced_textures,
        });
    }
    overrides
}

/// The size and modification time of a file, as a text that changes when
/// the file does.
pub(crate) fn file_state(file: &Path) -> Option<String> {
    let metadata = std::fs::metadata(file).ok()?;
    let modified = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |elapsed| elapsed.as_secs());
    Some(format!("{}:{modified}", metadata.len()))
}

fn to_cache_entry(textures: &[GamePath], replaced_textures: &[GamePath]) -> String {
    let lines = |paths: &[GamePath]| {
        paths
            .iter()
            .map(GamePath::as_str)
            .collect::<Vec<_>>()
            .join("\n")
    };
    format!(
        "{}{CACHE_SEPARATOR}{}",
        lines(textures),
        lines(replaced_textures)
    )
}

fn from_cache_entry(entry: &str) -> Option<(Vec<GamePath>, Vec<GamePath>)> {
    let paths = |lines: &str| {
        lines
            .lines()
            .filter(|line| !line.is_empty())
            .map(GamePath::new)
            .collect()
    };
    let (textures, replaced_textures) = entry.split_once(CACHE_SEPARATOR)?;
    Some((paths(textures), paths(replaced_textures)))
}

pub(crate) fn loose_textures(file: &Path) -> Option<Vec<GamePath>> {
    // Only the header and the texture blocks are read, not the geometry.
    let mut reader = BufReader::new(File::open(file).ok()?);
    Some(game_paths(&nif::read_texture_paths(&mut reader).ok()?))
}

fn packed_textures(
    archive: &LoadedArchive,
    archive_index: usize,
    entry_index: usize,
    open_archives: &mut HashMap<usize, File>,
) -> Option<Vec<GamePath>> {
    let file = match open_archives.entry(archive_index) {
        std::collections::hash_map::Entry::Occupied(open) => open.into_mut(),
        std::collections::hash_map::Entry::Vacant(vacant) => {
            vacant.insert(File::open(&archive.file).ok()?)
        }
    };
    let data = archive.directory.read(file, entry_index)?;
    Some(game_paths(&nif::texture_paths(&data).ok()?))
}

pub(crate) fn game_paths(raw: &[String]) -> Vec<GamePath> {
    let mut paths: Vec<GamePath> = raw.iter().filter_map(|raw| texture_path(raw)).collect();
    paths.sort();
    paths.dedup();
    paths
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn texture_paths_are_taken_from_textures_on() {
        let expected = Some(GamePath::new("data/textures/architecture/wall.dds"));
        for raw in [
            r"textures\architecture\wall.dds",
            r"Textures/Architecture/Wall.DDS",
            r"data\textures\architecture\wall.dds",
            r"C:\Projects\My Mod\Data\Textures\architecture\wall.dds",
        ] {
            assert_eq!(texture_path(raw), expected, "{raw}");
        }
    }

    #[test]
    fn a_cache_entry_gives_back_both_texture_lists() {
        let new = vec![GamePath::new("data/textures/a.dds")];
        let old = vec![
            GamePath::new("data/textures/b.dds"),
            GamePath::new("data/textures/c.dds"),
        ];
        for (textures, replaced) in [
            (new.clone(), old.clone()),
            (Vec::new(), old.clone()),
            (new.clone(), Vec::new()),
            (Vec::new(), Vec::new()),
        ] {
            let entry = to_cache_entry(&textures, &replaced);
            assert_eq!(from_cache_entry(&entry), Some((textures, replaced)));
        }
        assert_eq!(from_cache_entry("not an entry"), None);
    }

    #[test]
    fn paths_that_do_not_lead_into_textures_are_dropped() {
        assert_eq!(texture_path(""), None);
        assert_eq!(texture_path(r"meshes\wall.nif"), None);
        assert_eq!(texture_path(r"mytextures\wall.dds"), None);
    }
}
