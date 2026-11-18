//! A place of a worldspace up close (ADR 0038): everything placed around a
//! point, by the game and by mods, with the meshes and textures to draw it,
//! and the ground they stand on (ADR 0041).
//! Only reads.

use std::sync::{Arc, Mutex};

use mallet_core::ObjectRef;
use mallet_core::encoding::base64;
use mallet_core::mesh::{
    AreaLand, AreaObject, AreaObjects, LandCell, LandQuarter, LandTextureLayer, LodMesh,
};
use mallet_game_bethesda::area::AreaReader;
use mallet_game_bethesda::files;
use mallet_game_bethesda::land_textures::{LandTextures, Quarter};
use mallet_game_bethesda::lod_library::LodLibrary;

use crate::ScanOptions;
use crate::lod::{mesh_of, picture_of};
use crate::mesh::{floats, texture_path};

/// The largest distance around a point that is read, in game units: three
/// cells in each direction.
const MAX_RADIUS: f32 = 3.0 * 4096.0;

/// The plugins of the setup last asked about, by game and folder.
static READER: Mutex<Option<(String, Arc<Mutex<AreaReader>>)>> = Mutex::new(None);

/// Every mesh and texture of the setup last asked about.
static FILES: Mutex<Option<(String, Arc<LodLibrary>)>> = Mutex::new(None);

/// The landscape textures of the setup last asked about (ADR 0047).
static LAND_TEXTURES: Mutex<Option<(String, Arc<LandTextures>)>> = Mutex::new(None);

/// A key for the setup `options` describe.
fn setup_key(options: &ScanOptions) -> Option<String> {
    let game = options.game().ok()?;
    let (game_dir, _) = options.located_game_dir()?;
    Some(format!("{}|{}", game.id(), game_dir.display()))
}

pub(crate) fn reader(options: &ScanOptions) -> Option<Arc<Mutex<AreaReader>>> {
    let key = setup_key(options)?;
    let mut kept = READER.lock().ok()?;
    if let Some((known, reader)) = kept.as_ref()
        && *known == key
    {
        return Some(Arc::clone(reader));
    }
    let game = options.game().ok()?;
    let (game_dir, _) = options.located_game_dir()?;
    let reader = Arc::new(Mutex::new(AreaReader::open(
        game,
        &game_dir,
        options.local_dir.as_deref(),
    )));
    *kept = Some((key, Arc::clone(&reader)));
    Some(reader)
}

pub(crate) fn library(options: &ScanOptions) -> Option<Arc<LodLibrary>> {
    let key = setup_key(options)?;
    let mut kept = FILES.lock().ok()?;
    if let Some((known, library)) = kept.as_ref()
        && *known == key
    {
        return Some(Arc::clone(library));
    }
    let game = options.game().ok()?;
    let (game_dir, _) = options.located_game_dir()?;
    let started = std::time::Instant::now();
    let library = Arc::new(LodLibrary::with_prefixes(
        game,
        &game_dir,
        options.local_dir.as_deref(),
        &["meshes/".to_owned(), "textures/".to_owned()],
    ));
    tracing::info!(
        elapsed_ms = started.elapsed().as_millis(),
        "meshes and textures found"
    );
    *kept = Some((key, Arc::clone(&library)));
    Some(library)
}

/// Everything with a model placed within `radius` game units (at most
/// three cells) of `centre` in the worldspace `world`. `None` if the
/// setup cannot be read.
pub fn area_objects(
    options: &ScanOptions,
    world: &str,
    centre: [f32; 2],
    radius: f32,
) -> Option<AreaObjects> {
    let started = std::time::Instant::now();
    let radius = radius.clamp(256.0, MAX_RADIUS);
    let reader = reader(options)?;
    let mut reader = reader.lock().ok()?;
    let placed = reader.objects(world, centre, radius);
    let objects = drawable(&mut reader, placed);
    tracing::info!(
        world,
        objects = objects.len(),
        elapsed_ms = started.elapsed().as_millis(),
        "read a place up close"
    );
    Some(AreaObjects {
        world: world.to_owned(),
        centre,
        radius,
        objects,
    })
}

/// Everything with a model placed in the interior cell whose editor ID is
/// `cell` (ADR 0042), in the shape of an area: `world` is the cell, and
/// `centre` and `radius` frame what is placed in it. `None` if the setup
/// cannot be read or has no such cell.
pub fn interior_objects(options: &ScanOptions, cell: &str) -> Option<AreaObjects> {
    let started = std::time::Instant::now();
    let reader = reader(options)?;
    let mut reader = reader.lock().ok()?;
    let placed = reader.interior(cell);
    if placed.is_empty() {
        tracing::info!(cell, "interior cell not found or empty");
        return None;
    }
    let objects = drawable(&mut reader, placed);
    let (centre, radius) = frame(&objects);
    tracing::info!(
        cell,
        objects = objects.len(),
        elapsed_ms = started.elapsed().as_millis(),
        "read an interior cell"
    );
    Some(AreaObjects {
        world: cell.to_owned(),
        centre,
        radius,
        objects,
    })
}

/// The middle of the objects, east and north, and how far they reach from
/// it in either direction, in game units.
fn frame(objects: &[AreaObject]) -> ([f32; 2], f32) {
    let mut low = [f32::MAX; 2];
    let mut high = [f32::MIN; 2];
    for object in objects {
        for (axis, value) in object.position.iter().take(2).enumerate() {
            if let (Some(low), Some(high)) = (low.get_mut(axis), high.get_mut(axis)) {
                *low = low.min(*value);
                *high = high.max(*value);
            }
        }
    }
    if objects.is_empty() {
        return ([0.0, 0.0], 0.0);
    }
    let [west, south] = low;
    let [east, north] = high;
    (
        [f32::midpoint(west, east), f32::midpoint(south, north)],
        ((east - west).max(north - south) / 2.0).max(256.0),
    )
}

/// The placed objects that can be drawn, with their models.
fn drawable(
    reader: &mut mallet_game_bethesda::area::AreaReader,
    placed: Vec<mallet_game_bethesda::area::AreaObject>,
) -> Vec<AreaObject> {
    placed
        .into_iter()
        .filter_map(|object| {
            let base = reader.base(&object.base)?;
            Some(AreaObject {
                id: ObjectRef {
                    plugin: object.id.plugin,
                    local_id: object.id.local_id,
                },
                plugin: object.plugin,
                model: base.model.clone(),
                name: base.name.clone(),
                position: object.position,
                rotation: object.rotation,
                scale: object.scale,
                anchored: object.anchored,
            })
        })
        .collect()
}

/// The bytes of a file of the setup as the game uses it.
fn file(options: &ScanOptions, path: &str) -> Option<Vec<u8>> {
    library(options)?.read(path).or_else(|| {
        let game = options.game().ok()?;
        let (game_dir, _) = options.located_game_dir()?;
        files::versions(game, &game_dir, options.local_dir.as_deref(), path)
            .pop()
            .map(|version| version.bytes)
    })
}

/// The mesh at `path` below the data directory, in its own units, with
/// its textures named by path.
pub fn area_mesh(options: &ScanOptions, path: &str) -> Option<LodMesh> {
    let bytes = file(options, path)?;
    Some(mesh_of(&bytes, [0.0, 0.0]))
}

/// The texture at `path` below the data directory as a PNG `data:` address
/// no wider or higher than `max_side`.
pub fn area_texture(options: &ScanOptions, path: &str, max_side: u32) -> Option<String> {
    let bytes = file(options, &texture_path(path))?;
    picture_of(&bytes, max_side)
}

/// The ground of the outside cells within `radius` game units (at most
/// three cells) of `centre` in the worldspace `world`, in the version the
/// game uses (ADR 0041); the same cells [`area_objects`] reads. `None` if
/// the setup cannot be read.
pub fn area_land(
    options: &ScanOptions,
    world: &str,
    centre: [f32; 2],
    radius: f32,
) -> Option<AreaLand> {
    land_within(options, world, centre, radius.clamp(256.0, MAX_RADIUS))
}

/// The most cells of a worldspace whose whole land is read at once.
const MAX_WORLD_CELLS: i64 = 1024;

/// The ground of every outside cell within `bounds` (west, south, east,
/// north) of the worldspace `world`, for drawing a world that has no level
/// of detail from its land. `None` if the setup cannot be read
/// or the bounds hold more than 1024 cells.
pub fn world_land(options: &ScanOptions, world: &str, bounds: [i32; 4]) -> Option<AreaLand> {
    let [west, south, east, north] = bounds;
    let columns = i64::from(east) - i64::from(west) + 1;
    let rows = i64::from(north) - i64::from(south) + 1;
    if columns <= 0 || rows <= 0 || columns.max(rows).pow(2) > MAX_WORLD_CELLS {
        return None;
    }
    // A square of cells around the middle that ends just inside the
    // bounds' edges.
    let side = cells_to_units(columns.max(rows));
    let centre = [
        cells_to_units(i64::from(west)) + side / 2.0,
        cells_to_units(i64::from(south)) + side / 2.0,
    ];
    land_within(options, world, centre, side / 2.0 - 1.0)
}

/// Game units across `cells` cells.
#[allow(clippy::cast_precision_loss)]
fn cells_to_units(cells: i64) -> f32 {
    // Worldspaces span at most a few hundred cells: exact in a float.
    (cells * 4096) as f32
}

/// The ground of the cells within `radius` game units of `centre`.
fn land_within(
    options: &ScanOptions,
    world: &str,
    centre: [f32; 2],
    radius: f32,
) -> Option<AreaLand> {
    let started = std::time::Instant::now();
    let reader = reader(options)?;
    let mut reader = reader.lock().ok()?;
    let textures = land_textures(options);
    let cells: Vec<LandCell> = reader
        .land(world, centre, radius)
        .into_iter()
        .map(|cell| LandCell {
            x: cell.x,
            y: cell.y,
            heights: base64(&floats(&cell.heights)),
            colours: cell.colours.as_deref().map_or_default(base64),
            quarters: textures
                .as_deref()
                .map_or_else(Vec::new, |textures| painted(textures, &cell.quarters)),
        })
        .collect();
    tracing::info!(
        world,
        cells = cells.len(),
        elapsed_ms = started.elapsed().as_millis(),
        "read the land of a place up close"
    );
    Some(AreaLand {
        world: world.to_owned(),
        cells,
    })
}

/// The quarters of a cell with their landscape textures named by the
/// colour texture each uses (ADR 0047). Layers whose texture has no colour
/// texture are left out.
fn painted(textures: &LandTextures, quarters: &[Quarter]) -> Vec<LandQuarter> {
    let path = |id| textures.colour(id).map(texture_path);
    quarters
        .iter()
        .map(|quarter| LandQuarter {
            base: quarter.base.as_ref().and_then(path),
            layers: quarter
                .layers
                .iter()
                .filter_map(|layer| {
                    Some(LandTextureLayer {
                        texture: path(&layer.texture)?,
                        opacity: base64(&layer.opacity),
                    })
                })
                .collect(),
        })
        .collect()
}

/// The landscape textures of the setup `options` describe, read once.
fn land_textures(options: &ScanOptions) -> Option<Arc<LandTextures>> {
    let key = setup_key(options)?;
    let mut kept = LAND_TEXTURES.lock().ok()?;
    if let Some((known, textures)) = kept.as_ref()
        && *known == key
    {
        return Some(Arc::clone(textures));
    }
    let game = options.game().ok()?;
    let (game_dir, _) = options.located_game_dir()?;
    let started = std::time::Instant::now();
    let textures = Arc::new(LandTextures::open(
        game,
        &game_dir,
        options.local_dir.as_deref(),
    ));
    tracing::info!(
        elapsed_ms = started.elapsed().as_millis(),
        "landscape textures read"
    );
    *kept = Some((key, Arc::clone(&textures)));
    Some(textures)
}
