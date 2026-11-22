//! The paths characters walk on that run over hidden objects (ADR 0051):
//! for the scan, those of the patch in the game; for the window, those of
//! objects the user is about to hide, and what taking those paths out
//! would leave (ADR 0052). Only reads.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::Path;

use mallet_analysis::navmesh_repair::{self, PathsRepair};
use mallet_analysis::path_repair;
use mallet_analysis::walkways::{self, Carried, Hidden};
use mallet_core::facts::RecordId;
use mallet_core::facts::models::ModelMesh;
use mallet_core::facts::navmeshes::{NavmeshShape, NearHidden, PlacedDestination};
use mallet_core::facts::objects::{Place, PlacedObject};
use mallet_core::mesh::{CutOffPath, HiddenWalkways, PathRepair, Walkway};
use mallet_core::walkway_repair::{RepairedPaths, WalkwayRepair};
use mallet_core::{Facts, InstalledPatch, ObjectRef};
use mallet_game_bethesda::Game;
use mallet_game_bethesda::area::AreaObject;
pub use mallet_game_bethesda::navmesh_records::NavmeshRecord;
use mallet_game_bethesda::navmesh_records::Repaired;
use mallet_game_bethesda::walkways::Around;

use crate::ScanOptions;
use crate::area::{library, reader};

/// How far around the point the objects asked about are looked for, in
/// game units: the cell they are in and its neighbours.
pub(crate) const SEARCH_RADIUS: f32 = 4096.0;

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
/// patch in the game hides, as far as `facts` know them, and the places
/// characters walk to there (ADR 0059): their models are added to
/// `models`, to be read with the other models.
pub(crate) fn near_hidden(
    game: Game,
    game_dir: &Path,
    local_dir: Option<&Path>,
    facts: &Facts,
    models: &mut BTreeSet<String>,
) -> (NearHidden, Vec<PlacedDestination>) {
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
        return (NearHidden::default(), Vec::new());
    }
    let started = std::time::Instant::now();
    let (near, destinations) =
        mallet_game_bethesda::walkways::read_with_destinations(game, game_dir, local_dir, &around);
    tracing::info!(
        hidden = facts.hidden_objects.len(),
        navmeshes = near.navmeshes.len(),
        destinations = destinations.len(),
        elapsed_ms = started.elapsed().as_millis(),
        "read the paths around the objects the patch hides"
    );
    (near, destinations)
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
    let gathered = gather(options, place, centre, objects, false)?;
    let Gathered { placed, near, .. } = &gathered;
    let hidden = gathered.hidden();
    let carried = walkways::carried(&hidden, near);
    let repair = path_repair(near, &carried);
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

/// The objects asked about as the analyses know them, their models, and
/// the navigation meshes and ground of their cells.
struct Gathered {
    placed: Vec<PlacedObject>,
    models: Vec<Option<String>>,
    meshes: BTreeMap<String, ModelMesh>,
    near: NearHidden,
    around: Around,
}

impl Gathered {
    fn hidden(&self) -> Vec<Hidden<'_>> {
        self.placed
            .iter()
            .zip(&self.models)
            .map(|(object, model)| Hidden {
                object,
                mesh: model.as_ref().and_then(|model| self.meshes.get(model)),
                bounds: None,
            })
            .collect()
    }
}

/// Reads `objects`, which stand in the worldspace `place` near `centre`,
/// or, without a centre, in the interior cell `place`, with the
/// navigation meshes and the ground of the cells they reach into; with
/// `wider`, also of the cells around those outside, whose meshes may lead
/// onto theirs.
fn gather(
    options: &ScanOptions,
    place: &str,
    centre: Option<[f32; 2]>,
    objects: &[ObjectRef],
    wider: bool,
) -> Option<Gathered> {
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
    let mut gathered = Gathered {
        placed,
        models,
        meshes,
        near: NearHidden::default(),
        around: Around::default(),
    };
    let mut around = Around::default();
    if let Some(cell) = &cell {
        around.interiors.insert(cell.clone());
    } else {
        let cells = around.worlds.entry(place.to_owned()).or_default();
        for object in &gathered.hidden() {
            cells.extend(walkways::cells_reached(object));
        }
        if wider {
            let reached: Vec<(i32, i32)> = cells.iter().copied().collect();
            for (x, y) in reached {
                for dx in -1..=1 {
                    for dy in -1..=1 {
                        cells.insert((x + dx, y + dy));
                    }
                }
            }
        }
    }
    if !gathered.placed.is_empty() {
        // A repair starts from the meshes without its own earlier copy.
        gathered.near = if wider {
            reader.navmeshes_to_repair(&around)
        } else {
            reader.navmeshes(&around)
        };
    }
    gathered.around = around;
    Some(gathered)
}

/// What repairing the paths over `objects` does (ADR 0058), for the
/// window to show: the triangles taken out and those laid instead, by
/// navigation mesh. `objects` stand in `place` as for [`hidden_walkways`].
/// `None` if the setup cannot be read.
pub fn walkway_repair(
    options: &ScanOptions,
    place: &str,
    centre: Option<[f32; 2]>,
    objects: &[ObjectRef],
) -> Option<WalkwayRepair> {
    let started = std::time::Instant::now();
    let (gathered, repair, records) = repaired(options, place, centre, objects)?;
    let corners = |shape: &mallet_core::facts::navmeshes::NavmeshShape, triangle: u32| {
        let corners = shape.triangles.get(usize::try_from(triangle).ok()?)?;
        let at = |vertex: u32| shape.vertices.get(usize::try_from(vertex).ok()?).copied();
        Some([at(corners[0])?, at(corners[1])?, at(corners[2])?])
    };
    let navmeshes: Vec<RepairedPaths> = repair
        .navmeshes
        .iter()
        .filter(|mesh| !mesh.removed.is_empty() || !mesh.added.is_empty())
        .filter_map(|mesh| {
            let before = gathered.near.navmeshes.get(mesh.navmesh)?;
            Some(RepairedPaths {
                navmesh: before.id.to_string(),
                plugin: before.plugin.clone(),
                cell: before.place_name.clone(),
                removed: mesh
                    .removed
                    .iter()
                    .filter_map(|triangle| corners(before, *triangle))
                    .collect(),
                added: mesh
                    .added
                    .iter()
                    .filter_map(|triangle| corners(&mesh.after, *triangle))
                    .collect(),
                water: u32::try_from(mesh.water.len()).unwrap_or(u32::MAX),
                writable: records.iter().any(|record| record.navmesh == before.id),
            })
        })
        .collect();
    let laid = repair.laid.len();
    let bare = repair.bare;
    tracing::info!(
        place,
        objects = objects.len(),
        removed = repair.removed(),
        added = repair.added(),
        areas_laid = laid,
        bare_objects = bare,
        records = records.len(),
        elapsed_ms = started.elapsed().as_millis(),
        "worked out how to repair the paths over objects to hide"
    );
    Some(WalkwayRepair {
        place: place.to_owned(),
        navmeshes,
        areas_laid: u32::try_from(laid).unwrap_or(u32::MAX),
        bare_objects: u32::try_from(bare).unwrap_or(u32::MAX),
        lost_doors: repair.lost_doors.iter().map(ToString::to_string).collect(),
    })
}

/// The navigation mesh records to override so that the paths over
/// `objects` are repaired (ADR 0058), for the patch writer: one per mesh
/// the repair changes, a copy of the version the game uses with its
/// geometry repaired. `objects` are all the objects the patch hides in
/// `place`, as for [`hidden_walkways`]; `navmeshes`, if given, keeps only
/// the meshes the user chose to repair, named as in
/// [`RepairedPaths::navmesh`]. `None` if the setup cannot be read.
pub fn repaired_navmeshes(
    options: &ScanOptions,
    place: &str,
    centre: Option<[f32; 2]>,
    objects: &[ObjectRef],
    navmeshes: Option<&BTreeSet<String>>,
) -> Option<Vec<NavmeshRecord>> {
    let (_, _, records) = repaired(options, place, centre, objects)?;
    Some(
        records
            .into_iter()
            .filter(|record| {
                navmeshes.is_none_or(|chosen| chosen.contains(&record.navmesh.to_string()))
            })
            .collect(),
    )
}

/// The versions the game uses of the meshes in `near` with the paths over
/// `hidden` repaired (ADR 0058), for following where characters go (ADR
/// 0059). Triangles taken out stay, leading nowhere.
pub(crate) fn repaired_shapes(
    near: &NearHidden,
    hidden: &[Hidden<'_>],
    carried: &[Carried],
) -> Vec<NavmeshShape> {
    let repair = navmesh_repair::repair(near, hidden, carried);
    let mut changed: BTreeMap<usize, NavmeshShape> = repair
        .navmeshes
        .into_iter()
        .map(|mesh| (mesh.navmesh, mesh.after))
        .collect();
    near.navmeshes
        .iter()
        .enumerate()
        .filter(|(_, shape)| shape.used)
        .map(|(index, shape)| changed.remove(&index).unwrap_or_else(|| shape.clone()))
        .collect()
}

/// The repair of the paths over `objects`, and the records it writes.
fn repaired(
    options: &ScanOptions,
    place: &str,
    centre: Option<[f32; 2]>,
    objects: &[ObjectRef],
) -> Option<(Gathered, PathsRepair, Vec<NavmeshRecord>)> {
    let gathered = gather(options, place, centre, objects, true)?;
    let hidden = gathered.hidden();
    let carried = walkways::carried(&hidden, &gathered.near);
    let repair = navmesh_repair::repair(&gathered.near, &hidden, &carried);
    let empty = BTreeSet::new();
    let repaired: Vec<Repaired<'_>> = repair
        .navmeshes
        .iter()
        .filter_map(|mesh| {
            Some(Repaired {
                before: gathered.near.navmeshes.get(mesh.navmesh)?,
                after: &mesh.after,
                removed: if mesh.removed.is_empty() {
                    &empty
                } else {
                    &mesh.removed
                },
                water: &mesh.water,
            })
        })
        .collect();
    let records = if repaired.is_empty() {
        Vec::new()
    } else {
        let reader = reader(options)?;
        let reader = reader.lock().ok()?;
        reader.navmesh_records(&gathered.around, &repaired)
    };
    drop(hidden);
    Some((gathered, repair, records))
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
pub(crate) fn record_id(object: &ObjectRef) -> RecordId {
    RecordId {
        plugin: object.plugin.to_lowercase(),
        local_id: object.local_id,
    }
}

/// An object read up close as the analyses know placed objects.
pub(crate) fn placed_object(object: &AreaObject, cell: Option<&RecordId>) -> Option<PlacedObject> {
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
