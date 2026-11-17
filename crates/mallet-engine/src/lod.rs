//! The game's level of detail of a worldspace, for the map (ADR 0037): which
//! blocks there are, the land and objects of one block, and the textures
//! they use. Only reads.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use mallet_core::encoding::base64;
use mallet_core::mesh::{LodMesh, LodShape, LodTile, LodTiles};
use mallet_game_bethesda::files;
use mallet_game_bethesda::lod_library::{Block, LodLibrary};

use crate::ScanOptions;
use crate::mesh::{floats, texture_path};

/// How many worldspaces' indexes are kept at once.
const KEPT: usize = 3;

/// The smallest and largest side of a texture sent to the window.
const TEXTURE_SIDES: (u32, u32) = (16, 2048);

/// The indexes made so far, the most recent last, by game, folder and
/// worldspace. Making one reads the file lists of every archive; keeping it
/// makes each block a single read.
static LIBRARIES: Mutex<Vec<(String, Arc<LodLibrary>)>> = Mutex::new(Vec::new());

/// The level-of-detail files of `world` in the setup `options` describe.
fn library(options: &ScanOptions, world: &str) -> Option<Arc<LodLibrary>> {
    let game = options.game().ok()?;
    let (game_dir, _) = options.located_game_dir()?;
    let key = format!(
        "{}|{}|{}",
        game.id(),
        game_dir.display(),
        world.to_ascii_lowercase()
    );
    let mut kept = LIBRARIES.lock().ok()?;
    if let Some(at) = kept.iter().position(|(known, _)| *known == key) {
        let found = kept.remove(at);
        let library = Arc::clone(&found.1);
        kept.push(found);
        return Some(library);
    }
    let started = std::time::Instant::now();
    let library = Arc::new(LodLibrary::open(
        game,
        &game_dir,
        options.local_dir.as_deref(),
        world,
    ));
    tracing::info!(
        world,
        elapsed_ms = started.elapsed().as_millis(),
        "level-of-detail files found"
    );
    kept.push((key, Arc::clone(&library)));
    if kept.len() > KEPT {
        kept.remove(0);
    }
    Some(library)
}

/// The blocks of level of detail `world` has, coarsest first.
pub fn lod_tiles(options: &ScanOptions, world: &str) -> LodTiles {
    let mut tiles: BTreeMap<Block, LodTile> = BTreeMap::new();
    if let Some(library) = library(options, world) {
        let (land, things) = library.blocks();
        for (block, is_land) in land
            .into_iter()
            .map(|block| (block, true))
            .chain(things.into_iter().map(|block| (block, false)))
        {
            let tile = tiles.entry(block).or_insert(LodTile {
                level: block.level,
                x: block.x,
                y: block.y,
                terrain: false,
                objects: false,
            });
            if is_land {
                tile.terrain = true;
            } else {
                tile.objects = true;
            }
        }
    }
    let mut tiles: Vec<LodTile> = tiles.into_values().collect();
    tiles.sort_by(|a, b| {
        b.level
            .cmp(&a.level)
            .then(a.x.cmp(&b.x))
            .then(a.y.cmp(&b.y))
    });
    LodTiles {
        world: world.to_owned(),
        tiles,
    }
}

/// The land of a block, or with `objects` its objects; `None` if the
/// setup has no such file or it cannot be read.
pub fn lod_mesh(
    options: &ScanOptions,
    world: &str,
    level: u32,
    x: i32,
    y: i32,
    objects: bool,
) -> Option<LodMesh> {
    let library = library(options, world)?;
    let block = Block { level, x, y };
    let bytes = if objects {
        library.objects(block)?
    } else {
        library.terrain(block)?
    };
    // Land is kept from its block's south-west corner; objects are kept
    // where they stand in the world.
    let corner = if objects {
        [0.0, 0.0]
    } else {
        [cell_units(x), cell_units(y)]
    };
    let shapes = files::mesh_shapes(&bytes)
        .into_iter()
        .map(|mut shape| {
            for point in shape.positions.as_chunks_mut::<3>().0 {
                point[0] += corner[0];
                point[1] += corner[1];
            }
            shape
        })
        .map(|shape| LodShape {
            positions: base64(&floats(&shape.positions)),
            uvs: base64(&floats(&shape.uvs)),
            indices: base64(
                &shape
                    .indices
                    .iter()
                    .flat_map(|index| index.to_le_bytes())
                    .collect::<Vec<u8>>(),
            ),
            texture: shape.texture.as_deref().map(texture_path),
            alpha_test: shape.alpha,
        })
        .collect();
    Some(LodMesh { shapes })
}

/// Game units from the world's origin to the edge of cell `cell`.
#[allow(clippy::cast_precision_loss)]
fn cell_units(cell: i32) -> f32 {
    // Cells lie within a few hundred of the origin; exact in a float.
    (cell as f32) * 4096.0
}

/// The texture at `path` below the data directory as a PNG `data:` address
/// no wider or higher than `max_side`; from the worldspace's own files where
/// they have it, and else as the game finds it.
pub fn lod_texture(
    options: &ScanOptions,
    world: &str,
    path: &str,
    max_side: u32,
) -> Option<String> {
    let started = std::time::Instant::now();
    let path = texture_path(path);
    let picture = lod_picture(options, world, &path, max_side);
    if picture.is_none() {
        tracing::info!(path, "level-of-detail texture cannot be shown");
    } else {
        tracing::debug!(
            path,
            elapsed_ms = started.elapsed().as_millis(),
            "level-of-detail texture"
        );
    }
    picture
}

/// The texture at `path` (normalized) as for [`lod_texture`].
fn lod_picture(options: &ScanOptions, world: &str, path: &str, max_side: u32) -> Option<String> {
    let library = library(options, world)?;
    let bytes = library.read(path).or_else(|| {
        let game = options.game().ok()?;
        let (game_dir, _) = options.located_game_dir()?;
        files::versions(game, &game_dir, options.local_dir.as_deref(), path)
            .pop()
            .map(|version| version.bytes)
    })?;
    let image = files::thumbnail(&bytes, max_side.clamp(TEXTURE_SIDES.0, TEXTURE_SIDES.1))?;
    let png =
        crate::png::encode_rgba(image.width, image.height, &image.pixels, &image.alpha).ok()?;
    Some(format!("data:image/png;base64,{}", base64(&png)))
}
