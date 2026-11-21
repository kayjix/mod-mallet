//! The shapes of models, read for telling whether placed objects of two
//! mods really touch (ADR 0039).
//!
//! Only the models asked for are read, each once, from the file the game
//! uses: a loose file, or else the archive loaded last that has it. Only
//! reads.

use std::collections::BTreeSet;
use std::path::Path;

use mallet_core::facts::models::{ModelFacts, ModelMesh};
use mallet_core::{Progress, ScanStep};

use crate::files;
use crate::game::Game;
use crate::lod_library::LodLibrary;

/// Models with more triangles than this are left out: no building of the
/// games comes close, and comparing them would take too long.
const MAX_TRIANGLES: usize = 200_000;

/// The shapes of the models at `paths` (below the data directory, such as
/// `meshes/architecture/whiterun/house01.nif`) in the setup of `game` in
/// `game_dir`. `local_dir` holds `plugins.txt`, as for
/// [`crate::ingest::read_setup`]. Models that cannot be read are left out.
pub fn read_models(
    game: Game,
    game_dir: &Path,
    local_dir: Option<&Path>,
    paths: &BTreeSet<String>,
) -> ModelFacts {
    read_models_with(game, game_dir, local_dir, paths, &Progress::none())
}

/// Like [`read_models`], and tells `progress` how far it is.
pub fn read_models_with(
    game: Game,
    game_dir: &Path,
    local_dir: Option<&Path>,
    paths: &BTreeSet<String>,
    progress: &Progress<'_>,
) -> ModelFacts {
    let mut facts = ModelFacts::default();
    if paths.is_empty() {
        return facts;
    }
    let library = LodLibrary::with_prefixes(game, game_dir, local_dir, &["meshes/".to_owned()]);
    for (index, path) in paths.iter().enumerate() {
        progress.count(ScanStep::Models, index + 1, paths.len());
        let Some(bytes) = library.read(path) else {
            continue;
        };
        if let Some(mesh) = model_mesh(&bytes) {
            facts.meshes.insert(path.clone(), mesh);
        }
    }
    facts
}

/// The surface of the model in `bytes`, all its parts in one, or `None` if
/// it has none that can be read.
pub fn model_mesh(bytes: &[u8]) -> Option<ModelMesh> {
    let mut mesh = ModelMesh::default();
    for shape in files::mesh_shapes(bytes) {
        let offset = u32::try_from(mesh.points.len()).ok()?;
        for &[x, y, z] in shape.positions.as_chunks::<3>().0 {
            mesh.points.push([whole(x), whole(y), whole(z)]);
        }
        for &[a, b, c] in shape.indices.as_chunks::<3>().0 {
            mesh.triangles.push([
                a.checked_add(offset)?,
                b.checked_add(offset)?,
                c.checked_add(offset)?,
            ]);
        }
        if mesh.triangles.len() > MAX_TRIANGLES {
            return None;
        }
    }
    (!mesh.triangles.is_empty()).then_some(mesh)
}

/// A coordinate in whole game units.
#[allow(clippy::cast_possible_truncation)]
fn whole(value: f32) -> i32 {
    // Bounded before converting; damaged numbers count as the origin.
    if value.is_finite() {
        value.round().clamp(-1.0e9, 1.0e9) as i32
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn something_that_is_not_a_mesh_has_no_shape() {
        assert_eq!(model_mesh(b"not a mesh"), None);
    }

    #[test]
    fn coordinates_are_rounded_and_damaged_ones_dropped_to_the_origin() {
        assert_eq!(whole(12.6), 13);
        assert_eq!(whole(-3.4), -3);
        assert_eq!(whole(f32::NAN), 0);
        assert_eq!(whole(f32::MAX), 1_000_000_000);
    }
}
