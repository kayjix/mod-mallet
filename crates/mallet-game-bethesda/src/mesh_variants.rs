//! Loose meshes made for another game, or for another edition of this one.
//!
//! Every game reads the mesh format in its own variant, numbered by
//! Bethesda: Skyrim (2011) writes 83, Skyrim Special Edition 100. A mesh
//! taken over from a mod for the old Skyrim without converting it can make
//! the new one crash or show nothing. Only the start of each mesh is read
//! ([`nif::variant`]).

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use mallet_core::GamePath;
use mallet_core::facts::ForeignMesh;
use mallet_formats_bethesda::nif::{self, NifError};

use crate::game::Game;

/// How much of a mesh is read at first; its header nearly always fits.
const FIRST_READ: u64 = 4 * 1024;

/// How much is read when the header did not fit.
const LONGER_READ: u64 = 64 * 1024;

/// Where the game looks for meshes, as the game sees the path.
const MESHES_FOLDER: &str = "data/meshes/";

/// The variant of the mesh format `game` reads, or `None` for a game whose
/// meshes do not say (Oblivion).
pub fn native_version(game: Game) -> Option<u32> {
    match game {
        Game::SkyrimSe | Game::SkyrimVr => Some(100),
        Game::Skyrim => Some(83),
        Game::Fallout4 | Game::Fallout4Vr => Some(130),
        Game::FalloutNv | Game::Fallout3 => Some(34),
        Game::Oblivion => None,
    }
}

/// The meshes among `loose` files (the path the game sees, and the file)
/// whose variant is not the one `game` reads. Only the meshes folder
/// counts; the game loads no mesh from anywhere else. Meshes that cannot
/// be read are left to the other checks.
pub fn foreign_meshes(game: Game, loose: &[(GamePath, PathBuf)]) -> Vec<ForeignMesh> {
    let Some(native) = native_version(game) else {
        return Vec::new();
    };
    loose
        .iter()
        .filter(|(path, file)| {
            path.as_str().starts_with(MESHES_FOLDER)
                && file
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("nif"))
        })
        .filter_map(|(path, file)| {
            let variant = read_variant(file)?;
            (variant.bethesda_version != native).then(|| ForeignMesh {
                path: path.clone(),
                bethesda_version: variant.bethesda_version,
                skinned: variant.skinned,
            })
        })
        .collect()
}

/// The variant of the mesh in `file`, reading no more of it than needed.
fn read_variant(file: &Path) -> Option<nif::Variant> {
    let mut start = Vec::new();
    let mut reader = File::open(file).ok()?;
    reader
        .by_ref()
        .take(FIRST_READ)
        .read_to_end(&mut start)
        .ok()?;
    match nif::variant(&start) {
        Err(NifError::Damaged) if u64::try_from(start.len()).ok() == Some(FIRST_READ) => {
            reader
                .take(LONGER_READ - FIRST_READ)
                .read_to_end(&mut start)
                .ok()?;
            nif::variant(&start).ok()
        }
        result => result.ok(),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use mallet_formats_bethesda::fixture;

    use super::*;

    /// A mesh like the fixture's, in the variant `version`.
    fn mesh(version: u32) -> Vec<u8> {
        let mut mesh = fixture::nif(&[&["textures/wall.dds"]]);
        let after_line = mesh.iter().position(|&byte| byte == b'\n').unwrap() + 1;
        let at = after_line + 4 + 1 + 4 + 4;
        mesh[at..at + 4].copy_from_slice(&version.to_le_bytes());
        mesh
    }

    #[test]
    fn meshes_of_the_old_skyrim_are_found_in_the_new_one() {
        let dir = tempfile::tempdir().unwrap();
        let file = |name: &str, data: &[u8]| {
            let file = dir.path().join(name);
            fs::write(&file, data).unwrap();
            (GamePath::new("data/meshes").join(name), file)
        };
        let loose = vec![
            file("old.nif", &mesh(83)),
            file("new.nif", &mesh(100)),
            file("broken.nif", b"not a mesh"),
            file("readme.txt", &mesh(83)),
        ];
        // A mesh a tool keeps for itself, outside the meshes folder.
        let (_, script) = file("lod.nif", &mesh(83));
        let mut loose = loose;
        loose.push((GamePath::new("data/edit scripts/lod.nif"), script));

        let found = foreign_meshes(Game::SkyrimSe, &loose);
        assert_eq!(
            found,
            vec![ForeignMesh {
                path: GamePath::new("data/meshes/old.nif"),
                bethesda_version: 83,
                skinned: false,
            }]
        );
        // In the old Skyrim it is the other way round.
        let found = foreign_meshes(Game::Skyrim, &loose);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].path, GamePath::new("data/meshes/new.nif"));
        assert_eq!(foreign_meshes(Game::Oblivion, &loose), Vec::new());
    }
}
