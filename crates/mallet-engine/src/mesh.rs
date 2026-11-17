//! Meshes of the setup, ready to draw: points, triangles and textures of
//! one version of a `.nif`, read from the user's own files.

use std::collections::HashMap;

use mallet_core::encoding::base64;
use mallet_core::mesh::{MeshShape, MeshView};
use mallet_game_bethesda::files::{self, Origin};
use mallet_game_bethesda::locate;

use crate::ScanOptions;

/// The largest width or height of a texture sent to the window.
const TEXTURE_SIDE: u32 = 512;

/// Where face meshes lie, and where their face textures.
const FACE_MESHES: &str = "meshes/actors/character/facegendata/facegeom/";
const FACE_TEXTURES: &str = "textures/actors/character/facegendata/facetint/";

/// Version `version` (counted from the lowest in precedence; `None` for the
/// one the game uses) of the mesh at `path` below the data directory, with
/// its textures as the setup has them. `None` if there is no such version
/// or it is not a mesh that can be read. Only reads.
pub fn mesh_view(options: &ScanOptions, path: &str, version: Option<usize>) -> Option<MeshView> {
    let game_dir = options.game_dir.clone().or_else(locate::game_dir)?;
    let local_dir = options.local_dir.as_deref();
    let mut versions = files::versions(&game_dir, local_dir, path);
    let index = version.unwrap_or(versions.len().checked_sub(1)?);
    if index >= versions.len() {
        return None;
    }
    let chosen = versions.swap_remove(index);
    let shapes = files::mesh_shapes(&chosen.bytes);
    if shapes.is_empty() {
        return None;
    }

    let mut textures: HashMap<String, Option<String>> = HashMap::new();
    let mut texture = |raw: &str| -> Option<String> {
        let normalized = raw.replace('\\', "/").to_ascii_lowercase();
        let normalized = if normalized.starts_with("textures/") {
            normalized
        } else {
            format!("textures/{normalized}")
        };
        textures
            .entry(normalized.clone())
            .or_insert_with(|| picture(&game_dir, local_dir, &normalized))
            .clone()
    };

    let lower = path.replace('\\', "/").to_ascii_lowercase();
    let tint = lower
        .find(FACE_MESHES)
        .map(|at| {
            let rest = lower.get(at + FACE_MESHES.len()..).unwrap_or_default();
            format!("{FACE_TEXTURES}{}", rest.replace(".nif", ".dds"))
        })
        .and_then(|tint_path| picture(&game_dir, local_dir, &tint_path));

    let shapes = shapes
        .into_iter()
        .map(|shape| {
            let tinted = tint.is_some()
                && shape
                    .texture
                    .as_deref()
                    .is_some_and(|name| name.to_ascii_lowercase().contains("head"));
            MeshShape {
                positions: base64(&floats(&shape.positions)),
                uvs: base64(&floats(&shape.uvs)),
                indices: base64(
                    &shape
                        .indices
                        .iter()
                        .flat_map(|index| index.to_le_bytes())
                        .collect::<Vec<u8>>(),
                ),
                texture: shape.texture.as_deref().and_then(&mut texture),
                tinted,
                alpha_test: shape.alpha,
            }
        })
        .collect();
    Some(MeshView {
        path: path.to_owned(),
        archive: match chosen.origin {
            Origin::Loose => None,
            Origin::Archive(name) => Some(name),
        },
        shapes,
        tint,
    })
}

fn floats(values: &[f32]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect()
}

/// The texture the game uses at `path`, as a PNG `data:` address.
fn picture(
    game_dir: &std::path::Path,
    local_dir: Option<&std::path::Path>,
    path: &str,
) -> Option<String> {
    let version = files::versions(game_dir, local_dir, path).pop()?;
    let image = files::thumbnail(&version.bytes, TEXTURE_SIDE)?;
    let png =
        crate::png::encode_rgba(image.width, image.height, &image.pixels, &image.alpha).ok()?;
    Some(format!("data:image/png;base64,{}", base64(&png)))
}
