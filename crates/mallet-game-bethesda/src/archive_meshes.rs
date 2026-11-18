//! What the meshes packed in the archives of mods say about themselves:
//! the same as for loose meshes ([`crate::loose_meshes`]).
//!
//! The game's own archives are left out; their meshes fit the game. A
//! mod's archive holds hundreds or thousands of meshes, each unpacked
//! whole, so what was read is kept in the cache under the size and date of
//! the archive and read again only when the archive changes.

use std::fs::File;

use mallet_core::{GamePath, TextCache};

use crate::DATA_DIR;
use crate::game::Game;
use crate::loose_meshes::{Entry, LooseMeshes, native_version};
use crate::meshes::{LoadedArchive, file_state};

/// Under which name what the meshes of an archive say is kept in the cache.
const CACHE_NAMESPACE: &str = "archive-meshes";

/// Separates the meshes of an archive in its cache entry. No line of an
/// [`Entry`] is empty.
const CACHE_SEPARATOR: &str = "\n\n";

/// Adds what the meshes in the archives of mods say to `meshes`.
pub(crate) fn read(
    game: Game,
    archives: &[LoadedArchive],
    cache: &dyn TextCache,
    meshes: &mut LooseMeshes,
) {
    let native = native_version(game);
    for archive in archives
        .iter()
        .filter(|archive| !is_games_own(game, &archive.path))
    {
        let state = file_state(&archive.file);
        let cached = state
            .as_deref()
            .and_then(|state| cache.get(CACHE_NAMESPACE, archive.path.as_str(), state))
            .and_then(|text| from_text(&text));
        let entries = if let Some(entries) = cached {
            entries
        } else {
            let entries = read_archive(archive);
            if let Some(state) = &state {
                cache.put(
                    CACHE_NAMESPACE,
                    archive.path.as_str(),
                    state,
                    &to_text(&entries),
                );
            }
            entries
        };
        for (path, entry) in entries {
            entry.add_to(meshes, native, &path, Some(&archive.path));
        }
    }
}

/// Whether the archive at `path` is one of the game's own: one the game's
/// settings list, or one of an official plugin.
fn is_games_own(game: Game, path: &GamePath) -> bool {
    let name = path.file_name();
    let stem = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
    game.base_archive_prefixes()
        .iter()
        .any(|prefix| name.starts_with(prefix))
        || game.base_archives_last().contains(&name)
        || game.official_plugins().iter().any(|plugin| {
            plugin.rsplit_once('.').is_some_and(|(official, _)| {
                stem == official || stem.starts_with(&format!("{official} - "))
            })
        })
}

/// Unpacks the meshes of `archive`, by the path the game sees them under.
/// Meshes that cannot be read are left out.
fn read_archive(archive: &LoadedArchive) -> Vec<(GamePath, Entry)> {
    let Ok(mut file) = File::open(&archive.file) else {
        return Vec::new();
    };
    let data = GamePath::new(DATA_DIR);
    archive
        .directory
        .paths()
        .into_iter()
        .enumerate()
        .filter_map(|(index, entry)| {
            let path = data.join(entry);
            let is_mesh = std::path::Path::new(path.as_str())
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("nif"));
            if !path.as_str().starts_with("data/meshes/") || !is_mesh {
                return None;
            }
            let bytes = archive.directory.read(&mut file, index)?;
            Some((path, Entry::from_bytes(&bytes)?))
        })
        .collect()
}

/// The meshes of an archive as kept in the cache: each mesh's path on a
/// line, followed by its entry, meshes separated by an empty line.
fn to_text(entries: &[(GamePath, Entry)]) -> String {
    entries
        .iter()
        .map(|(path, entry)| format!("{}\n{}", path.as_str(), entry.to_text()))
        .collect::<Vec<_>>()
        .join(CACHE_SEPARATOR)
}

/// The meshes of an archive from the cache; `None` if the text is not
/// such a list.
fn from_text(text: &str) -> Option<Vec<(GamePath, Entry)>> {
    if text.is_empty() {
        return Some(Vec::new());
    }
    text.split(CACHE_SEPARATOR)
        .map(|mesh| {
            let (path, entry) = mesh.split_once('\n')?;
            Some((GamePath::new(path), Entry::from_text(entry)?))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use mallet_core::NoCache;
    use mallet_core::facts::{ForeignMesh, MeshTextures};
    use mallet_formats_bethesda::fixture;
    use proptest::prelude::*;

    use super::*;
    use crate::archive;

    /// A mesh with one surface in the variant `version`.
    fn mesh(version: u32) -> Vec<u8> {
        let mut mesh = fixture::nif(&[&[r"textures\rock.dds", r"textures\rock_n.dds"]]);
        let after_line = mesh.iter().position(|&byte| byte == b'\n').unwrap() + 1;
        let at = after_line + 4 + 1 + 4 + 4;
        mesh[at..at + 4].copy_from_slice(&version.to_le_bytes());
        mesh
    }

    fn archive(dir: &std::path::Path, name: &str, files: &[(&str, &[u8])]) -> LoadedArchive {
        let file = dir.join(name);
        fs::write(
            &file,
            fixture::bsa_with(
                files,
                fixture::BsaOptions {
                    version: 105,
                    compressed: true,
                    embedded_names: false,
                },
            ),
        )
        .unwrap();
        LoadedArchive {
            directory: archive::read_directory(&file).unwrap(),
            path: GamePath::new(DATA_DIR).join(name),
            file,
        }
    }

    #[test]
    fn meshes_in_the_archives_of_mods_are_read_and_the_games_own_left_out() {
        let dir = tempfile::tempdir().unwrap();
        let old = mesh(83);
        let archives = vec![
            archive(
                dir.path(),
                "Skyrim - Meshes0.bsa",
                &[(r"meshes\a.nif", &old)],
            ),
            archive(dir.path(), "Dawnguard.bsa", &[(r"meshes\b.nif", &old)]),
            archive(
                dir.path(),
                "Rocks.bsa",
                &[
                    (r"meshes\rocks\rock.nif", &old),
                    (r"textures\rock.dds", b"DDS "),
                ],
            ),
        ];
        let mut meshes = LooseMeshes::default();
        read(Game::SkyrimSe, &archives, &NoCache, &mut meshes);

        let rock = GamePath::new("data/meshes/rocks/rock.nif");
        let rocks = Some(GamePath::new("data/rocks.bsa"));
        assert_eq!(
            meshes.foreign,
            vec![ForeignMesh {
                path: rock.clone(),
                archive: rocks.clone(),
                bethesda_version: 83,
                skinned: false,
            }]
        );
        assert_eq!(
            meshes.textures,
            vec![MeshTextures {
                path: rock,
                archive: rocks,
                textures: vec![
                    GamePath::new("data/textures/rock.dds"),
                    GamePath::new("data/textures/rock_n.dds"),
                ],
            }]
        );
    }

    #[test]
    fn the_meshes_of_an_archive_survive_the_cache() {
        let entries = vec![
            (
                GamePath::new("data/meshes/a.nif"),
                Entry::from_bytes(&mesh(83)).unwrap(),
            ),
            (
                GamePath::new("data/meshes/b.nif"),
                Entry::from_bytes(&mesh(100)).unwrap(),
            ),
        ];
        assert_eq!(from_text(&to_text(&entries)), Some(entries));
        assert_eq!(from_text(""), Some(Vec::new()));
    }

    proptest! {
        #[test]
        fn never_panics_on_any_cache_text(text in "(.|\n){0,64}") {
            let _ = from_text(&text);
        }
    }
}
