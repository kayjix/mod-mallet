//! The paths characters walk on that run over hidden objects (ADR 0051):
//! for the scan, those of the patch in the game; for the window, those of
//! objects the user is about to hide, and what taking those paths out
//! would leave (ADR 0052). Only reads.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::Path;

use mallet_analysis::path_repair;
use mallet_analysis::walkways::{self, Carried, Hidden};
use mallet_core::facts::RecordId;
use mallet_core::facts::models::ModelMesh;
use mallet_core::facts::navmeshes::NearHidden;
use mallet_core::facts::objects::{Place, PlacedObject};
use mallet_core::mesh::{CutOffPath, HiddenWalkways, PathRepair, Walkway};
use mallet_core::{Facts, InstalledPatch, ObjectRef};
use mallet_game_bethesda::Game;
use mallet_game_bethesda::area::AreaObject;
use mallet_game_bethesda::walkways::Around;

use crate::ScanOptions;
use crate::area::{library, reader};

/// How far around the point the objects asked about are looked for, in
/// game units: the cell they are in and its neighbours.
const SEARCH_RADIUS: f32 = 4096.0;

/// The objects the patch in the game hides, by the identity the plugins
/// use.
pub(crate) fn hidden_by(patch: Option<&InstalledPatch>) -> Vec<RecordId> {
    patch
        .iter()
        .flat_map(|patch| &patch.objects)
        .map(record_id)
        .collect()
}

/// The navigation meshes and the ground around the objects that the
/// patch in the game hides, as far as `facts` know them: their models are
/// added to `models`, to be read with the other models.
pub(crate) fn near_hidden(
    game: Game,
    game_dir: &Path,
    local_dir: Option<&Path>,
    facts: &Facts,
    models: &mut BTreeSet<String>,
) -> NearHidden {
    let needs = walkways::hidden_needs(facts);
    models.extend(needs.models);
    let names: BTreeMap<&RecordId, &str> = facts
        .objects
        .place_names
        .iter()
        .map(|place| (&place.id, place.editor_id.as_str()))
        .collect();
    let mut around = Around {
        interiors: needs.interiors,
        ..Around::default()
    };
    for (world, cells) in needs.worlds {
        if let Some(name) = names.get(&world) {
            around
                .worlds
                .entry((*name).to_owned())
                .or_default()
                .extend(cells);
        }
    }
    if around.is_empty() {
        return NearHidden::default();
    }
    let started = std::time::Instant::now();
    let near = mallet_game_bethesda::walkways::read(game, game_dir, local_dir, &around);
    tracing::info!(
        hidden = facts.hidden_objects.len(),
        navmeshes = near.navmeshes.len(),
        elapsed_ms = started.elapsed().as_millis(),
        "read the paths around the objects the patch hides"
    );
    near
}

/// The paths characters walk on that run over `objects`, which stand in
/// the worldspace `place` near `centre`, or, without a centre, in the
/// interior cell `place` (by its editor ID). Every plugin's version of
/// each path is given, with whether the game uses it. `None` if the setup
/// cannot be read.
pub fn hidden_walkways(
    options: &ScanOptions,
    place: &str,
    centre: Option<[f32; 2]>,
    objects: &[ObjectRef],
) -> Option<HiddenWalkways> {
    let started = std::time::Instant::now();
    let wanted: HashSet<RecordId> = objects.iter().map(record_id).collect();
    let reader = reader(options)?;
    let mut reader = reader.lock().ok()?;
    let (placed, cell) = if let Some(centre) = centre {
        (reader.objects(place, centre, SEARCH_RADIUS), None)
    } else {
        let cell = reader.interior_cell(place)?;
        (reader.interior(place), Some(cell))
    };
    let found: Vec<AreaObject> = placed
        .into_iter()
        .filter(|object| wanted.contains(&object.id))
        .collect();

    // Their models, from the files the game uses.
    let library = library(options)?;
    let mut meshes: BTreeMap<String, ModelMesh> = BTreeMap::new();
    let mut models: Vec<Option<String>> = Vec::new();
    for object in &found {
        let model = reader.base(&object.base).map(|base| base.model.clone());
        if let Some(model) = &model
            && !meshes.contains_key(model)
            && let Some(mesh) = library
                .read(model)
                .and_then(|bytes| mallet_game_bethesda::models::model_mesh(&bytes))
        {
            meshes.insert(model.clone(), mesh);
        }
        models.push(model);
    }

    let placed: Vec<PlacedObject> = found
        .iter()
        .filter_map(|object| placed_object(object, cell.as_ref()))
        .collect();
    let hidden: Vec<Hidden<'_>> = placed
        .iter()
        .zip(&models)
        .map(|(object, model)| Hidden {
            object,
            mesh: model.as_ref().and_then(|model| meshes.get(model)),
            bounds: None,
        })
        .collect();

    let mut around = Around::default();
    if let Some(cell) = &cell {
        around.interiors.insert(cell.clone());
    } else {
        let cells = around.worlds.entry(place.to_owned()).or_default();
        for object in &hidden {
            cells.extend(walkways::cells_reached(object));
        }
    }
    let near = if hidden.is_empty() {
        NearHidden::default()
    } else {
        reader.navmeshes(&around)
    };

    let carried = walkways::carried(&hidden, &near);
    let repair = path_repair(&near, &carried);
    let walkways: Vec<Walkway> = carried
        .into_iter()
        .filter_map(|carried| {
            let object = hidden.get(carried.hidden)?.object;
            let shape = near.navmeshes.get(carried.navmesh)?;
            // The object as the window named it.
            let asked = objects.iter().find(|asked| record_id(asked) == object.id)?;
            Some(Walkway {
                object: asked.clone(),
                navmesh: shape.id.to_string(),
                plugin: shape.plugin.clone(),
                used: shape.used,
                cell: shape.place_name.clone(),
                triangles: u32::try_from(carried.triangles.len()).unwrap_or(u32::MAX),
            })
        })
        .collect();
    tracing::info!(
        place,
        objects = objects.len(),
        found = hidden.len(),
        walkways = walkways.len(),
        repair_safe = repair.safe,
        elapsed_ms = started.elapsed().as_millis(),
        "looked for paths over objects to hide"
    );
    Some(HiddenWalkways {
        place: place.to_owned(),
        found: objects
            .iter()
            .filter(|asked| placed.iter().any(|known| known.id == record_id(asked)))
            .cloned()
            .collect(),
        walkways,
        repair,
    })
}

/// Square game units per square metre: a metre is about 70 units.
const UNITS_PER_SQUARE_METRE: f64 = 70.0 * 70.0;

/// What taking the triangles in `carried` out of the paths in `near`
/// would leave, for the window.
fn path_repair(near: &NearHidden, carried: &[Carried]) -> PathRepair {
    let repair = path_repair::repair(near, carried);
    let triangles: usize = repair.cuts.values().map(BTreeSet::len).sum();
    PathRepair {
        triangles: u32::try_from(triangles).unwrap_or(u32::MAX),
        safe: repair.is_safe(),
        cut_off: repair
            .cut_off
            .iter()
            .map(|part| CutOffPath {
                cell: part
                    .navmeshes
                    .iter()
                    .find_map(|index| near.navmeshes.get(*index)?.place_name.clone()),
                centre: part.centre.map(whole_f64),
                square_metres: square_metres(part.area),
                doors: part.doors.iter().map(ToString::to_string).collect(),
            })
            .collect(),
        lost_doors: repair.lost_doors.iter().map(ToString::to_string).collect(),
    }
}

/// An area in square game units, in whole square metres.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn square_metres(area: f64) -> u32 {
    // Clamped before converting.
    (area / UNITS_PER_SQUARE_METRE)
        .round()
        .clamp(0.0, f64::from(u32::MAX)) as u32
}

/// A number rounded to whole units.
#[allow(clippy::cast_possible_truncation)]
fn whole_f64(value: f64) -> i32 {
    // Bounded before converting; positions of the games are far inside.
    value.round().clamp(-1.0e9, 1.0e9) as i32
}

/// An object as the window names it, by the identity the plugins use.
fn record_id(object: &ObjectRef) -> RecordId {
    RecordId {
        plugin: object.plugin.to_lowercase(),
        local_id: object.local_id,
    }
}

/// An object read up close as the analyses know placed objects.
fn placed_object(object: &AreaObject, cell: Option<&RecordId>) -> Option<PlacedObject> {
    let place = match (&object.world, cell) {
        (Some(world), _) => Place::Exterior {
            worldspace: world.clone(),
        },
        (None, Some(cell)) => Place::Interior { cell: cell.clone() },
        (None, None) => return None,
    };
    Some(PlacedObject {
        id: object.id.clone(),
        plugin: object.plugin.clone(),
        base: object.base.clone(),
        place,
        position: object.position.map(whole),
        rotation: object.rotation.map(|angle| whole(angle * 1000.0)),
        scale: whole(object.scale * 1000.0),
        persistent: false,
        large: false,
        moved: None,
    })
}

/// A number rounded to whole units.
#[allow(clippy::cast_possible_truncation)]
fn whole(value: f32) -> i32 {
    // Bounded before converting; positions of the games are far inside.
    value.round().clamp(-1.0e9, 1.0e9) as i32
}
