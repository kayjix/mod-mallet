//! A place of a worldspace up close (ADR 0038): everything placed around a
//! point, by the game and by mods, with the meshes and textures to draw it,
//! and the ground they stand on (ADR 0041).
//! Only reads.

use std::sync::{Arc, Mutex};

use mallet_core::ObjectRef;
use mallet_core::encoding::base64;
use mallet_core::mesh::{AreaLand, AreaObject, AreaObjects, LandCell, LodMesh};
use mallet_game_bethesda::area::AreaReader;
use mallet_game_bethesda::files;
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

/// A key for the setup `options` describe.
fn setup_key(options: &ScanOptions) -> Option<String> {
    let game = options.game().ok()?;
    let (game_dir, _) = options.located_game_dir()?;
    Some(format!("{}|{}", game.id(), game_dir.display()))
}

fn reader(options: &ScanOptions) -> Option<Arc<Mutex<AreaReader>>> {
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

fn library(options: &ScanOptions) -> Option<Arc<LodLibrary>> {
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
    let objects: Vec<AreaObject> = placed
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
        .collect();
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
    let started = std::time::Instant::now();
    let radius = radius.clamp(256.0, MAX_RADIUS);
    let reader = reader(options)?;
    let mut reader = reader.lock().ok()?;
    let cells: Vec<LandCell> = reader
        .land(world, centre, radius)
        .into_iter()
        .map(|cell| LandCell {
            x: cell.x,
            y: cell.y,
            heights: base64(&floats(&cell.heights)),
            colours: cell.colours.as_deref().map_or_default(base64),
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
