//! Whether characters still reach where they need to go once objects are
//! hidden and the paths over them cut out or repaired (ADR 0059), for the
//! window. Only reads.

use std::collections::{BTreeMap, HashSet};

use mallet_analysis::reachability::{self, Outcome, Route as FoundRoute};
use mallet_analysis::walkways::{self, Carried, Hidden};
use mallet_core::ObjectRef;
use mallet_core::anchors::AnchorKind;
use mallet_core::facts::RecordId;
use mallet_core::facts::models::ModelMesh;
use mallet_core::facts::navmeshes::{NavmeshShape, NearHidden, PlacedDestination};
use mallet_core::facts::objects::{Place, PlacedObject};
use mallet_core::reach::{
    Destination, DestinationKind, ReachAfter, ReachChange, ReachOutcome, Route, WalkwayReach,
};
use mallet_game_bethesda::area::AreaObject;
use mallet_game_bethesda::walkways::Around;

use crate::ScanOptions;
use crate::area::{library, reader};
use crate::walkways::{SEARCH_RADIUS, placed_object, record_id};

/// Game units per metre, about.
const UNITS_PER_METRE: f64 = 70.0;

/// How many cells around those of the objects are read outside, so that a
/// way round a hidden bridge is found where it leaves their cells.
const MARGIN: i32 = 1;

/// What hiding `objects`, which stand in the worldspace `place` near
/// `centre`, or, without a centre, in the interior cell `place` (by its
/// editor ID), and cutting the paths over them out (ADR 0052), does to the
/// ways characters take near them. `None` if the setup cannot be read.
pub fn walkway_reach(
    options: &ScanOptions,
    place: &str,
    centre: Option<[f32; 2]>,
    objects: &[ObjectRef],
) -> Option<WalkwayReach> {
    walkway_reach_after(
        options,
        place,
        centre,
        objects,
        ReachAfter::Cut,
        &|near, carried| reachability::cut(&near.navmeshes, carried),
    )
}

/// As [`walkway_reach`], with the paths after hiding made by `after` from
/// every version of the meshes around the objects and the triangles that
/// lie on them, such as the paths repaired with new triangles. `kind` says
/// what `after` makes.
pub fn walkway_reach_after(
    options: &ScanOptions,
    place: &str,
    centre: Option<[f32; 2]>,
    objects: &[ObjectRef],
    kind: ReachAfter,
    after: &dyn Fn(&NearHidden, &[Carried]) -> Vec<NavmeshShape>,
) -> Option<WalkwayReach> {
    let started = std::time::Instant::now();
    let Looked {
        around_objects,
        cell,
        placed,
        near,
        mut destinations,
    } = look(options, place, centre, objects)?;

    // The models of the objects, from the files the game uses.
    let library = library(options)?;
    let mut meshes: BTreeMap<String, ModelMesh> = BTreeMap::new();
    for model in placed.iter().filter_map(|(_, model)| model.as_ref()) {
        if !meshes.contains_key(model)
            && let Some(mesh) = library
                .read(model)
                .and_then(|bytes| mallet_game_bethesda::models::model_mesh(&bytes))
        {
            meshes.insert(model.clone(), mesh);
        }
    }
    let hidden: Vec<Hidden<'_>> = placed
        .iter()
        .map(|(object, model)| Hidden {
            object,
            mesh: model.as_ref().and_then(|model| meshes.get(model)),
            bounds: None,
        })
        .collect();

    // What quests and routines send characters to, among the objects
    // around (ADR 0046).
    let mut used_by: BTreeMap<RecordId, String> = BTreeMap::new();
    if !hidden.is_empty() {
        add_used(
            options,
            place,
            &around_objects,
            cell.as_ref(),
            &mut destinations,
            &mut used_by,
        );
    }

    let carried = walkways::carried(&hidden, &near);
    let after_shapes = if carried.is_empty() {
        Vec::new()
    } else {
        after(&near, &carried)
    };
    let found = if carried.is_empty() {
        reachability::Reachability::default()
    } else {
        reachability::reachability(&near.navmeshes, &after_shapes, &destinations)
    };
    let inside = cell.is_some();
    let changes = changes_of(&found, &destinations, &used_by, |destination| {
        cell_name(place, inside, destination)
    });
    tracing::info!(
        place,
        objects = objects.len(),
        found = hidden.len(),
        carried = carried.len(),
        destinations = destinations.len(),
        checked = found.checked,
        changes = changes.len(),
        elapsed_ms = started.elapsed().as_millis(),
        "followed the ways to where characters go near objects to hide"
    );
    Some(WalkwayReach {
        place: place.to_owned(),
        after: kind,
        checked: u32::try_from(found.checked).unwrap_or(u32::MAX),
        changes,
    })
}

/// The changes of `found` for the window, with `cell` naming where each
/// destination lies. Two mods may place the same thing at the same spot:
/// it is told once.
fn changes_of(
    found: &reachability::Reachability,
    destinations: &[PlacedDestination],
    used_by: &BTreeMap<RecordId, String>,
    cell: impl Fn(&PlacedDestination) -> String,
) -> Vec<ReachChange> {
    let describe = |index: usize| -> Option<Destination> {
        let destination = destinations.get(index)?;
        Some(Destination {
            object: ObjectRef {
                plugin: destination.id.plugin.clone(),
                local_id: destination.id.local_id,
            },
            kind: destination.kind,
            name: destination.name.clone(),
            used_by: used_by.get(&destination.id).cloned(),
            position: destination.position,
            cell: Some(cell(destination)),
        })
    };
    let mut told = HashSet::new();
    found
        .reaches
        .iter()
        .filter(|reach| {
            destinations
                .get(reach.destination)
                .is_none_or(|destination| {
                    told.insert((
                        destination.kind,
                        destination.name.clone(),
                        destination.position,
                    ))
                })
        })
        .filter_map(|reach| {
            let (outcome, from) = match reach.outcome {
                Outcome::Gone => (ReachOutcome::Gone, None),
                Outcome::CutOff => (ReachOutcome::CutOff, None),
                Outcome::Longer { from } => (ReachOutcome::Longer, describe(from)),
            };
            Some(ReachChange {
                outcome,
                destination: describe(reach.destination)?,
                from,
                before: reach.before.as_ref().map(route),
                after: reach.after.as_ref().map(route),
            })
        })
        .collect()
}

/// What is read around the objects asked about.
struct Looked {
    /// Every object around, the game's as well as the mods'.
    around_objects: Vec<AreaObject>,
    /// Inside, the cell.
    cell: Option<RecordId>,
    /// The objects asked about that were found, with their models.
    placed: Vec<(PlacedObject, Option<String>)>,
    /// The navigation meshes of their cells and those around.
    near: NearHidden,
    /// The places characters walk to there.
    destinations: Vec<PlacedDestination>,
}

/// Reads the objects around `centre` in the worldspace `place`, or in the
/// interior cell `place`, and the navigation meshes of the cells that
/// `objects` reach and those around them.
fn look(
    options: &ScanOptions,
    place: &str,
    centre: Option<[f32; 2]>,
    objects: &[ObjectRef],
) -> Option<Looked> {
    let wanted: HashSet<RecordId> = objects.iter().map(record_id).collect();
    let reader = reader(options)?;
    let mut reader = reader.lock().ok()?;
    let (around_objects, cell) = if let Some(centre) = centre {
        (reader.objects(place, centre, SEARCH_RADIUS), None)
    } else {
        let cell = reader.interior_cell(place)?;
        (reader.interior(place), Some(cell))
    };
    let mut placed = Vec::new();
    for object in around_objects
        .iter()
        .filter(|object| wanted.contains(&object.id))
    {
        let model = reader.base(&object.base).map(|base| base.model.clone());
        if let Some(found) = placed_object(object, cell.as_ref()) {
            placed.push((found, model));
        }
    }
    let mut around = Around::default();
    if let Some(cell) = &cell {
        around.interiors.insert(cell.clone());
    } else {
        let cells = around.worlds.entry(place.to_owned()).or_default();
        for (object, _) in &placed {
            let hidden = Hidden {
                object,
                mesh: None,
                bounds: None,
            };
            for (x, y) in walkways::cells_reached(&hidden) {
                for dx in -MARGIN..=MARGIN {
                    for dy in -MARGIN..=MARGIN {
                        cells.insert((x + dx, y + dy));
                    }
                }
            }
        }
    }
    let (near, destinations) = if placed.is_empty() {
        (NearHidden::default(), Vec::new())
    } else {
        reader.navmeshes_with_destinations(&around)
    };
    Some(Looked {
        around_objects,
        cell,
        placed,
        near,
        destinations,
    })
}

/// Adds the objects among `objects` that a quest or a character's routine
/// sends characters to and that are not destinations yet, with who uses
/// them in `used_by`.
fn add_used(
    options: &ScanOptions,
    place: &str,
    objects: &[AreaObject],
    cell: Option<&RecordId>,
    destinations: &mut Vec<PlacedDestination>,
    used_by: &mut BTreeMap<RecordId, String>,
) {
    let refs: Vec<ObjectRef> = objects
        .iter()
        .map(|object| ObjectRef {
            plugin: object.id.plugin.clone(),
            local_id: object.id.local_id,
        })
        .collect();
    let Some(anchors) = crate::anchors::object_anchors(options, place, &refs) else {
        return;
    };
    let known: HashSet<RecordId> = destinations.iter().map(|known| known.id.clone()).collect();
    for (object, anchors) in objects.iter().zip(anchors) {
        let user = anchors.reasons.iter().find_map(|reason| {
            matches!(
                reason.kind,
                AnchorKind::Package | AnchorKind::QuestAlias | AnchorKind::LocationRole
            )
            .then(|| {
                reason
                    .other
                    .as_ref()
                    .and_then(|other| other.name.clone())
                    .or_else(|| reason.keyword.clone())
            })
            .flatten()
        });
        let Some(user) = user else {
            continue;
        };
        used_by.insert(object.id.clone(), user);
        if known.contains(&object.id) {
            continue;
        }
        let place = match (&object.world, cell) {
            (Some(world), _) => Place::Exterior {
                worldspace: world.clone(),
            },
            (None, Some(cell)) => Place::Interior { cell: cell.clone() },
            (None, None) => continue,
        };
        destinations.push(PlacedDestination {
            id: object.id.clone(),
            plugin: object.plugin.clone(),
            kind: DestinationKind::Used,
            name: None,
            place,
            position: object.position.map(whole),
            leads_to: None,
        });
    }
}

/// Where a destination lies, for people: the interior cell, or the
/// worldspace and the cell's grid position.
fn cell_name(place: &str, inside: bool, destination: &PlacedDestination) -> String {
    if inside {
        return place.to_owned();
    }
    let [x, y, _] = destination.position;
    let cell = |value: i32| value.div_euclid(4096);
    format!("{place} {},{}", cell(x), cell(y))
}

/// A way as the window draws it.
fn route(found: &FoundRoute) -> Route {
    Route {
        metres: metres(found.length),
        points: found
            .points
            .iter()
            .map(|point| point.map(whole_f64))
            .collect(),
    }
}

/// A length in game units, in whole metres.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn metres(length: f64) -> u32 {
    // Clamped before converting.
    (length / UNITS_PER_METRE)
        .round()
        .clamp(0.0, f64::from(u32::MAX)) as u32
}

/// A number rounded to whole units.
#[allow(clippy::cast_possible_truncation)]
fn whole_f64(value: f64) -> i32 {
    // Bounded before converting; positions of the games are far inside.
    value.round().clamp(-1.0e9, 1.0e9) as i32
}

/// A number rounded to whole units.
#[allow(clippy::cast_possible_truncation)]
fn whole(value: f32) -> i32 {
    // Bounded before converting; positions of the games are far inside.
    value.round().clamp(-1.0e9, 1.0e9) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lengths_are_given_in_metres() {
        assert_eq!(metres(700.0), 10);
        assert_eq!(metres(-5.0), 0);
        assert_eq!(metres(f64::INFINITY), u32::MAX);
    }

    #[test]
    fn a_destination_outside_is_placed_by_its_cell() {
        let destination = PlacedDestination {
            id: RecordId {
                plugin: "skyrim.esm".to_owned(),
                local_id: 1,
            },
            plugin: "Skyrim.esm".to_owned(),
            kind: DestinationKind::Door,
            name: None,
            place: Place::Interior {
                cell: RecordId {
                    plugin: "skyrim.esm".to_owned(),
                    local_id: 2,
                },
            },
            position: [6 * 4096 + 5, -1, 0],
            leads_to: None,
        };
        assert_eq!(cell_name("Tamriel", false, &destination), "Tamriel 6,-1");
        assert_eq!(cell_name("Inn", true, &destination), "Inn");
    }
}
