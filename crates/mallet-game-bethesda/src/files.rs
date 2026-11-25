//! Every version of one file that a setup has: the copies inside the
//! archives the game loads, and the loose file in the data directory. The
//! game uses the last of them; the others are what it would use if that one
//! were gone. Only reads.

use std::fs::File;
use std::path::Path;

use mallet_formats_bethesda::dds;
pub use mallet_formats_bethesda::pex::{FunctionKind, PexScript};

use crate::game::Game;
use crate::{archive, archives, ingest};

/// Files larger than this are not read for showing.
const MAX_BYTES: u64 = 64 * 1024 * 1024;

/// Where a version of a file comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Origin {
    /// A loose file in the data directory.
    Loose,
    /// An archive, by its file name.
    Archive(String),
}

/// One version of a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileVersion {
    /// Where it comes from.
    pub origin: Origin,
    /// Its content.
    pub bytes: Vec<u8>,
}

/// A path as the game compares them: lower case, `/` between folders,
/// without a leading `data/`.
pub fn normalize(path: &str) -> String {
    let lower = path.replace('\\', "/").to_ascii_lowercase();
    let trimmed = lower.trim_start_matches('/');
    trimmed.strip_prefix("data/").unwrap_or(trimmed).to_owned()
}

/// Every version of `path` (below the data directory) in the setup of
/// `game` in `game_dir`, the one the game uses last. `local_dir` holds `plugins.txt`,
/// as for [`ingest::read_setup`]. Archives and files that cannot be read are
/// left out.
pub fn versions(
    game: Game,
    game_dir: &Path,
    local_dir: Option<&Path>,
    path: &str,
) -> Vec<FileVersion> {
    versions_of_each(game, game_dir, local_dir, &[path])
        .into_iter()
        .next()
        .unwrap_or_default()
}

/// Every version of each of `paths`, as [`versions`] finds them, in the
/// same order as `paths`. Each archive is read once for all of them.
pub fn versions_of_each(
    game: Game,
    game_dir: &Path,
    local_dir: Option<&Path>,
    paths: &[&str],
) -> Vec<Vec<FileVersion>> {
    let wanted: Vec<Option<String>> = paths
        .iter()
        .map(|path| {
            let wanted = normalize(path);
            (!wanted.is_empty() && !wanted.split('/').any(|part| part == "..")).then_some(wanted)
        })
        .collect();
    let mut found: Vec<Vec<FileVersion>> = paths.iter().map(|_| Vec::new()).collect();
    if wanted.iter().all(Option::is_none) {
        return found;
    }
    let data_dir = game_dir.join("Data");

    let active = ingest::active_plugin_names(game, game_dir, local_dir).unwrap_or_default();
    let on_disk = ingest::archives_in(game, &data_dir).unwrap_or_default();
    for name in archives::load_order(game, &on_disk, &active) {
        for (slot, bytes) in from_archive(&data_dir.join(&name), &wanted) {
            if let Some(versions) = found.get_mut(slot) {
                versions.push(FileVersion {
                    origin: Origin::Archive(name.clone()),
                    bytes,
                });
            }
        }
    }

    for (versions, wanted) in found.iter_mut().zip(&wanted) {
        let Some(wanted) = wanted else {
            continue;
        };
        let loose = data_dir.join(wanted);
        let small =
            std::fs::metadata(&loose).is_ok_and(|meta| meta.is_file() && meta.len() <= MAX_BYTES);
        if small && let Ok(bytes) = std::fs::read(&loose) {
            versions.push(FileVersion {
                origin: Origin::Loose,
                bytes,
            });
        }
    }
    found
}

/// The name and functions of the compiled script in `bytes`; `None` if
/// it is not one that can be read.
pub fn script(bytes: &[u8]) -> Option<PexScript> {
    mallet_formats_bethesda::pex::read_script(bytes).ok()
}

/// The colour texture the material file in `bytes` (`.bgsm`, `.bgem`)
/// names, as it spells it; `None` if it names none or is not a material.
pub fn material_texture(bytes: &[u8]) -> Option<String> {
    mallet_formats_bethesda::material::main_texture(bytes)
        .ok()
        .flatten()
}

/// The textures the surfaces of a mesh name (the colour and the normal map
/// of each), as the mesh spells them; empty if it cannot be read.
pub fn surface_textures(bytes: &[u8]) -> Vec<String> {
    mallet_formats_bethesda::nif::read_surface_textures(&mut std::io::Cursor::new(bytes))
        .unwrap_or_default()
}

/// The surfaces of a mesh, for drawing it; empty if it cannot be read.
pub fn mesh_shapes(bytes: &[u8]) -> Vec<mallet_formats_bethesda::nif_geometry::Shape> {
    mallet_formats_bethesda::nif_geometry::shapes(bytes).unwrap_or_default()
}

/// The bones of a mesh, for drawing a skeleton as lines; empty if it has
/// none or cannot be read.
pub fn mesh_bones(bytes: &[u8]) -> Vec<mallet_formats_bethesda::nif_geometry::Bone> {
    mallet_formats_bethesda::nif_geometry::bones(bytes)
}

/// A texture as a picture no wider or higher than `max_side`, from the
/// largest mipmap level that fits; `None` if it is not a texture that can
/// be decoded.
pub fn thumbnail(bytes: &[u8], max_side: u32) -> Option<dds::Rgb> {
    let size = dds::size(bytes).ok()?;
    let mut level = 0;
    while level + 1 < size.levels
        && ((size.width >> level) > max_side || (size.height >> level) > max_side)
    {
        level += 1;
    }
    dds::decode_level(bytes, level).ok()
}

/// The files of `wanted` (normalized) that the archive at `archive` has,
/// each with its position in `wanted`.
fn from_archive(archive: &Path, wanted: &[Option<String>]) -> Vec<(usize, Vec<u8>)> {
    let Ok(directory) = archive::read_directory(archive) else {
        return Vec::new();
    };
    let mut slots: std::collections::HashMap<&str, Vec<usize>> = std::collections::HashMap::new();
    for (slot, path) in wanted.iter().enumerate() {
        if let Some(path) = path {
            slots.entry(path.as_str()).or_default().push(slot);
        }
    }
    let mut found = Vec::new();
    let mut file = None;
    for (index, path) in directory.paths().into_iter().enumerate() {
        let Some(mine) = slots.get(normalize(path).as_str()) else {
            continue;
        };
        if file.is_none() {
            file = File::open(archive).ok();
        }
        let Some(open) = file.as_mut() else {
            break;
        };
        if let Some(bytes) = directory.read(open, index)
            && bytes.len() as u64 <= MAX_BYTES
        {
            for slot in mine {
                found.push((*slot, bytes.clone()));
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_are_compared_as_the_game_does() {
        assert_eq!(normalize("Data\\Textures\\A.DDS"), "textures/a.dds");
        assert_eq!(normalize("textures/a.dds"), "textures/a.dds");
    }

    #[test]
    fn a_loose_file_is_found_and_paths_out_of_the_data_directory_are_not() {
        let game = tempfile::tempdir().unwrap();
        let textures = game.path().join("Data/textures");
        std::fs::create_dir_all(&textures).unwrap();
        std::fs::write(textures.join("a.dds"), b"loose").unwrap();
        std::fs::write(game.path().join("secret.txt"), b"no").unwrap();
        let local = tempfile::tempdir().unwrap();
        std::fs::write(local.path().join("plugins.txt"), b"").unwrap();

        let found = versions(
            Game::SkyrimSe,
            game.path(),
            Some(local.path()),
            "Textures\\A.dds",
        );
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].origin, Origin::Loose);
        assert_eq!(found[0].bytes, b"loose");
        assert_eq!(
            versions(
                Game::SkyrimSe,
                game.path(),
                Some(local.path()),
                "../secret.txt"
            ),
            Vec::new()
        );
    }
}
