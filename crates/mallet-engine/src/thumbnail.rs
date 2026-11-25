//! Small pictures of meshes for lists (ADR 0085): each version drawn once
//! from a fixed three-quarter angle, kept in the cache, and otherwise the
//! reason there is nothing to see. Only reads the setup.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Instant;

use mallet_core::encoding::base64;
use mallet_core::mesh::{MeshGap, MeshThumbnail, MeshThumbnailRequest};
use mallet_game_bethesda::files;
use mallet_store::Store;
use serde::{Deserialize, Serialize};

use crate::ScanOptions;
use crate::mesh::{below, texture_path};
use crate::raster::{self, Surface, Texture};

/// Where the pictures are kept in the cache.
const CACHE_NAMESPACE: &str = "mesh-thumbnail";

/// Changes whenever the pictures would come out differently, so that
/// older ones are drawn again.
const RENDERER: &str = "r1";

/// The largest side of a texture read for a picture: the picture itself
/// is small.
const TEXTURE_SIDE: u32 = 128;

/// The smallest and largest side of a picture.
const SIDES: (u32, u32) = (32, 512);

/// Surface kinds that the reader of meshes knows; a mesh with other
/// surfaces and nothing drawn has surfaces it cannot read yet.
const SURFACES: [&str; 8] = [
    "BSTriShape",
    "BSMeshLODTriShape",
    "BSSubIndexTriShape",
    "BSDynamicTriShape",
    "BSLODTriShape",
    "NiTriShape",
    "NiTriStrips",
    "BSGeometry",
];

/// The cache the pictures are kept in, opened once, with the file it is.
static KEPT: Mutex<Option<(PathBuf, Store)>> = Mutex::new(None);

/// What is kept for one version of a mesh.
#[derive(Serialize, Deserialize)]
struct Kept {
    image: Option<String>,
    gap: Option<MeshGap>,
}

/// A picture of `side` pixels of each mesh version in `wanted`, in the
/// same order, from the cache at `cache_path` where it was drawn before.
/// Pictures are drawn again when the setup changed.
pub fn mesh_thumbnails(
    options: &ScanOptions,
    cache_path: Option<&Path>,
    wanted: &[MeshThumbnailRequest],
    side: u32,
) -> Vec<MeshThumbnail> {
    let started = Instant::now();
    let side = side.clamp(SIDES.0, SIDES.1);
    let fingerprint = format!("{RENDERER}|{}", crate::setup_fingerprint(options));
    let key = |request: &MeshThumbnailRequest| {
        format!(
            "{side}|{}|{}",
            files::normalize(&request.path),
            request.version.as_deref().unwrap_or("")
        )
    };
    let mut cached = 0usize;
    let mut sources = None;
    let found = wanted
        .iter()
        .map(|request| {
            if let Some(kept) = with_cache(cache_path, |store| {
                store
                    .cached_text(CACHE_NAMESPACE, &key(request), &fingerprint)
                    .ok()
                    .flatten()
            })
            .flatten()
            .and_then(|text| serde_json::from_str::<Kept>(&text).ok())
            {
                cached += 1;
                return thumbnail(request, kept);
            }
            let sources = sources.get_or_insert_with(|| crate::versions::sources(options));
            let kept = match sources {
                Some(sources) => {
                    let read = |file: &str| match &request.version {
                        Some(version) => sources
                            .read(file, version)
                            .or_else(|| sources.read_shown(file)),
                        None => sources.read_shown(file),
                    };
                    draw(&request.path, &read, side)
                }
                None => Kept {
                    image: None,
                    gap: Some(MeshGap::Missing),
                },
            };
            if let Ok(text) = serde_json::to_string(&kept) {
                with_cache(cache_path, |store| {
                    if let Err(error) =
                        store.cache_text(CACHE_NAMESPACE, &key(request), &fingerprint, &text)
                    {
                        tracing::warn!(%error, "picture of a mesh could not be cached");
                    }
                });
            }
            thumbnail(request, kept)
        })
        .collect();
    tracing::debug!(
        asked = wanted.len(),
        cached,
        elapsed_ms = started.elapsed().as_millis(),
        "pictures of meshes"
    );
    found
}

fn thumbnail(request: &MeshThumbnailRequest, kept: Kept) -> MeshThumbnail {
    MeshThumbnail {
        path: request.path.clone(),
        version: request.version.clone(),
        image: kept.image,
        gap: kept.gap,
    }
}

/// Runs `work` with the cache at `cache_path`, opening it the first time.
/// `None` without a cache or where it cannot be opened.
pub(crate) fn with_cache<T>(
    cache_path: Option<&Path>,
    work: impl FnOnce(&Store) -> T,
) -> Option<T> {
    let cache_path = cache_path?;
    let mut kept = KEPT.lock().ok()?;
    if kept.as_ref().is_none_or(|(path, _)| path != cache_path) {
        match crate::open_store(cache_path) {
            Ok(store) => *kept = Some((cache_path.to_path_buf(), store)),
            Err(error) => {
                tracing::warn!(%error, "cache for pictures of meshes could not be opened");
                return None;
            }
        }
    }
    kept.as_ref().map(|(_, store)| work(store))
}

/// The picture of the mesh at `path`, its files read through `read`, or
/// why there is none.
fn draw(path: &str, read: &dyn Fn(&str) -> Option<Vec<u8>>, side: u32) -> Kept {
    let started = Instant::now();
    let found = read(path).ok_or(MeshGap::Missing).and_then(|bytes| {
        let picture = picture(&bytes, read, side)?;
        let png =
            crate::png::encode_rgba(picture.width, picture.height, &picture.rgb, &picture.alpha)
                .map_err(|_| MeshGap::Unreadable)?;
        Ok(format!("data:image/png;base64,{}", base64(&png)))
    });
    tracing::debug!(
        path,
        drawn = found.is_ok(),
        elapsed_ms = started.elapsed().as_millis(),
        "picture of a mesh"
    );
    match found {
        Ok(image) => Kept {
            image: Some(image),
            gap: None,
        },
        Err(gap) => Kept {
            image: None,
            gap: Some(gap),
        },
    }
}

/// The mesh `bytes` drawn with its textures, or why it cannot be.
fn picture(
    bytes: &[u8],
    read: &dyn Fn(&str) -> Option<Vec<u8>>,
    side: u32,
) -> Result<raster::Picture, MeshGap> {
    let shapes = files::readable_mesh_shapes(bytes).ok_or(MeshGap::Unreadable)?;
    if shapes.is_empty() {
        return Err(if files::mesh_bones(bytes).is_empty() {
            gap_of(&files::mesh_parts(bytes))
        } else {
            MeshGap::Skeleton
        });
    }
    let mut textures: Vec<(String, Option<Texture>)> = Vec::new();
    let mut index_of = |raw: &str| -> Option<usize> {
        let normalized = texture_path(raw);
        if let Some(at) = textures.iter().position(|(known, _)| *known == normalized) {
            return Some(at);
        }
        let texture = read(&normalized)
            .and_then(|bytes| files::thumbnail(&bytes, TEXTURE_SIDE))
            .map(|image| Texture {
                width: image.width,
                height: image.height,
                rgb: image.pixels,
                alpha: image.alpha,
            });
        textures.push((normalized, texture));
        Some(textures.len() - 1)
    };
    let named: Vec<Option<usize>> = shapes
        .iter()
        .map(|shape| {
            let from_material = shape.material.as_deref().and_then(|material| {
                let normalized = below(material, "materials/");
                files::material_texture(&read(&normalized)?)
            });
            from_material
                .or_else(|| shape.texture.clone())
                .and_then(|raw| index_of(raw.as_str()))
        })
        .collect();
    let surfaces: Vec<Surface<'_>> = shapes
        .iter()
        .zip(&named)
        .map(|(shape, texture)| Surface {
            positions: &shape.positions,
            uvs: &shape.uvs,
            indices: &shape.indices,
            texture: texture
                .and_then(|at| textures.get(at))
                .and_then(|(_, texture)| texture.as_ref()),
            alpha_test: shape.alpha,
        })
        .collect();
    raster::draw(&surfaces, side).ok_or(MeshGap::Empty)
}

/// Why a mesh made of the blocks `parts` has nothing to draw.
fn gap_of(parts: &[String]) -> MeshGap {
    let has = |wanted: &dyn Fn(&str) -> bool| parts.iter().any(|part| wanted(part));
    if has(&|part| part.contains("Particle") || part == "BSEffectShaderProperty") {
        MeshGap::Effects
    } else if has(&|part| SURFACES.contains(&part)) {
        MeshGap::Unsupported
    } else {
        MeshGap::Empty
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parts(names: &[&str]) -> Vec<String> {
        names.iter().map(|&name| name.to_owned()).collect()
    }

    #[test]
    fn a_mesh_without_surfaces_says_what_it_is_instead() {
        assert_eq!(
            gap_of(&parts(&["BSFadeNode", "NiParticleSystem"])),
            MeshGap::Effects
        );
        assert_eq!(
            gap_of(&parts(&[
                "BSFadeNode",
                "BSTriShape",
                "BSEffectShaderProperty"
            ])),
            MeshGap::Effects
        );
        assert_eq!(
            gap_of(&parts(&[
                "BSFadeNode",
                "BSTriShape",
                "BSLightingShaderProperty"
            ])),
            MeshGap::Unsupported
        );
        assert_eq!(gap_of(&parts(&["NiNode", "NiCamera"])), MeshGap::Empty);
        assert_eq!(gap_of(&[]), MeshGap::Empty);
    }

    #[test]
    fn what_cannot_be_read_or_found_gives_a_reason() {
        let nothing = |_: &str| None;
        let missing = draw("meshes/a.nif", &nothing, 64);
        assert_eq!(missing.gap, Some(MeshGap::Missing));
        let text = |_: &str| Some(b"not a mesh".to_vec());
        let unreadable = draw("meshes/a.nif", &text, 64);
        assert_eq!(unreadable.gap, Some(MeshGap::Unreadable));
        assert!(unreadable.image.is_none());
    }

    #[test]
    fn pictures_are_kept_and_found_again() {
        let directory = tempfile::tempdir().unwrap();
        let cache = directory.path().join("cache.sqlite");
        let stored = with_cache(Some(&cache), |store| {
            store.cache_text(CACHE_NAMESPACE, "k", "f", "v").is_ok()
        });
        assert_eq!(stored, Some(true));
        let found = with_cache(Some(&cache), |store| {
            store.cached_text(CACHE_NAMESPACE, "k", "f").ok().flatten()
        });
        assert_eq!(found.flatten().as_deref(), Some("v"));
        assert!(with_cache(None, |_| ()).is_none());
    }
}
