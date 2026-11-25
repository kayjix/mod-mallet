//! Meshes of the setup, ready to draw: points, triangles and textures of
//! one version of a `.nif`, read from the user's own files.

use std::collections::HashMap;

use mallet_core::encoding::base64;
use mallet_core::mesh::{MeshShape, MeshView};

use mallet_game_bethesda::files::{self, Origin};

use crate::ScanOptions;

/// The largest width or height of a texture sent to the window.
const TEXTURE_SIDE: u32 = 512;

/// The folders whose files the index of the setup's meshes and textures
/// has.
const INDEXED: [&str; 2] = ["textures/", "meshes/"];

/// Where face meshes lie, and where their face textures.
const FACE_MESHES: &str = "meshes/actors/character/facegendata/facegeom/";
const FACE_TEXTURES: &str = "textures/actors/character/facegendata/facetint/";

/// Version `version` (counted from the lowest in precedence; `None` for the
/// one the game uses) of the mesh at `path` below the data directory, with
/// its textures as the setup has them; for a skeleton, its bones. `None` if
/// there is no such version, or it is not a mesh that can be read, or it has
/// neither surfaces nor bones. Only reads.
pub fn mesh_view(options: &ScanOptions, path: &str, version: Option<usize>) -> Option<MeshView> {
    let game = options.game().ok()?;
    let (game_dir, _) = options.located_game_dir()?;
    let local_dir = options.local_dir.as_deref();
    let mut versions = files::versions(game, &game_dir, local_dir, path);
    let index = version.unwrap_or(versions.len().checked_sub(1)?);
    if index >= versions.len() {
        return None;
    }
    let chosen = versions.swap_remove(index);
    let archive = match chosen.origin {
        Origin::Loose => None,
        Origin::Archive(name) => Some(name),
    };
    // Textures from the index of every mesh and texture: looking for each
    // in every archive again took a third of a second per texture. Other
    // files, such as materials, are looked for as before.
    let library = crate::area::library(options);
    let read = |path: &str| {
        let normalized = files::normalize(path);
        match &library {
            Some(library) if INDEXED.iter().any(|folder| normalized.starts_with(folder)) => {
                library.read(&normalized)
            }
            _ => files::versions(game, &game_dir, local_dir, path)
                .pop()
                .map(|version| version.bytes),
        }
    };
    render(path, &chosen.bytes, archive, &read)
}

/// The mesh `bytes`, read from `path` (in `archive`, if it came from one),
/// ready to draw, with its textures, materials and face tint taken from
/// `read`, which gives the bytes of a file below the data directory.
/// For a skeleton, its bones. `None` if it is not a mesh that can be read
/// or has neither surfaces nor bones.
pub(crate) fn render(
    path: &str,
    bytes: &[u8],
    archive: Option<String>,
    read: &dyn Fn(&str) -> Option<Vec<u8>>,
) -> Option<MeshView> {
    let shapes = files::mesh_shapes(bytes);
    // A skeleton has bones and no surfaces; its bones are drawn instead.
    let bones = if shapes.is_empty() {
        files::mesh_bones(bytes)
    } else {
        Vec::new()
    };
    if shapes.is_empty() && bones.is_empty() {
        return None;
    }
    let bones: Vec<f32> = bones
        .iter()
        .flat_map(|bone| bone.from.into_iter().chain(bone.to))
        .collect();

    let mut textures: HashMap<String, Option<String>> = HashMap::new();
    let mut texture = |raw: &str| -> Option<String> {
        let normalized = texture_path(raw);
        textures
            .entry(normalized.clone())
            .or_insert_with(|| picture(&read(&normalized)?))
            .clone()
    };

    // From Fallout 4 on, the material a shader names decides the texture,
    // also where the mesh names one of its own.
    let mut materials: HashMap<String, Option<String>> = HashMap::new();
    let mut material_texture = |raw: &str| -> Option<String> {
        let normalized = below(raw, "materials/");
        materials
            .entry(normalized.clone())
            .or_insert_with(|| files::material_texture(&read(&normalized)?))
            .clone()
    };

    let tint = face_tint(path).and_then(|tint_path| picture(&read(&tint_path)?));

    let shapes = shapes
        .into_iter()
        .map(|shape| {
            let named = shape
                .material
                .as_deref()
                .and_then(&mut material_texture)
                .or(shape.texture);
            let tinted = tint.is_some()
                && named
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
                texture: named.as_deref().and_then(&mut texture),
                tinted,
                alpha_test: shape.alpha,
            }
        })
        .collect();
    Some(MeshView {
        path: path.to_owned(),
        archive,
        shapes,
        tint,
        bones: base64(&floats(&bones)),
    })
}

/// A path from a mesh as the game looks it up below the data directory, in
/// the folder `folder` (`textures/`). Meshes spell such paths relative to
/// that folder, with it, or with the folders of the machine they were made
/// on in front (`C:\Projects\...\Data\materials\...`); the game reads
/// from the folder's name on.
pub(crate) fn below(raw: &str, folder: &str) -> String {
    let normalized = raw.replace('\\', "/").to_ascii_lowercase();
    let normalized = normalized.trim_start_matches('/');
    if normalized.starts_with(folder) {
        return normalized.to_owned();
    }
    match normalized.find(&format!("/{folder}")) {
        Some(at) => normalized.get(at + 1..).unwrap_or_default().to_owned(),
        None => format!("{folder}{normalized}"),
    }
}

/// A texture as a mesh names it, as a path below the data directory.
pub(crate) fn texture_path(raw: &str) -> String {
    below(raw, "textures/")
}

/// Numbers as their little-endian bytes.
pub(crate) fn floats(values: &[f32]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect()
}

/// The face tint texture that belongs to the face mesh at `path`, if it
/// is one.
pub(crate) fn face_tint(path: &str) -> Option<String> {
    let lower = path.replace('\\', "/").to_ascii_lowercase();
    let at = lower.find(FACE_MESHES)?;
    let rest = lower.get(at + FACE_MESHES.len()..).unwrap_or_default();
    Some(format!("{FACE_TEXTURES}{}", rest.replace(".nif", ".dds")))
}

/// A texture as a PNG `data:` address, no larger than the window needs.
fn picture(bytes: &[u8]) -> Option<String> {
    let image = files::thumbnail(bytes, TEXTURE_SIDE)?;
    let png =
        crate::png::encode_rgba(image.width, image.height, &image.pixels, &image.alpha).ok()?;
    Some(format!("data:image/png;base64,{}", base64(&png)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_are_read_from_the_folders_name_on() {
        assert_eq!(
            below(r"Props\Bottle_d.dds", "textures/"),
            "textures/props/bottle_d.dds"
        );
        assert_eq!(
            below(r"textures\Props\Bottle_d.dds", "textures/"),
            "textures/props/bottle_d.dds"
        );
        assert_eq!(
            below(
                r"C:\Projects\Fallout4\Build\PC\Data\materials\Architecture\Roof.BGSM",
                "materials/"
            ),
            "materials/architecture/roof.bgsm"
        );
    }
}
