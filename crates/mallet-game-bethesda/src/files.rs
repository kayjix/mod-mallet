//! Every version of one file that a setup has: the copies inside the
//! archives the game loads, and the loose file in the data directory. The
//! game uses the last of them; the others are what it would use if that one
//! were gone. Only reads.

use std::fs::File;
use std::path::Path;

use mallet_formats_bethesda::dds;

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
pub(crate) fn normalize(path: &str) -> String {
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
    let wanted = normalize(path);
    if wanted.is_empty() || wanted.split('/').any(|part| part == "..") {
        return Vec::new();
    }
    let data_dir = game_dir.join("Data");
    let mut found = Vec::new();

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
        if let Some(bytes) = from_archive(&data_dir.join(&name), &wanted) {
            found.push(FileVersion {
                origin: Origin::Archive(name),
                bytes,
            });
        }
    }

    let loose = data_dir.join(&wanted);
    let small =
        std::fs::metadata(&loose).is_ok_and(|meta| meta.is_file() && meta.len() <= MAX_BYTES);
    if small && let Ok(bytes) = std::fs::read(&loose) {
        found.push(FileVersion {
            origin: Origin::Loose,
            bytes,
        });
    }
    found
}

/// The surfaces of a mesh, for drawing it; empty if it cannot be read.
pub fn mesh_shapes(bytes: &[u8]) -> Vec<mallet_formats_bethesda::nif_geometry::Shape> {
    mallet_formats_bethesda::nif_geometry::shapes(bytes).unwrap_or_default()
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

/// The file `wanted` (normalized) from the archive at `archive`, if it has it.
fn from_archive(archive: &Path, wanted: &str) -> Option<Vec<u8>> {
    let directory = archive::read_directory(archive).ok()?;
    let index = directory
        .paths()
        .into_iter()
        .position(|path| normalize(path) == wanted)?;
    let mut file = File::open(archive).ok()?;
    let bytes = directory.read(&mut file, index)?;
    (bytes.len() as u64 <= MAX_BYTES).then_some(bytes)
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
