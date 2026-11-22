//! Doors that mods stack on top of each other (ADR 0067).
//!
//! Many mods add doors. Another mod adds one of its own at the same spot,
//! a few centimetres in front of or behind the first because its wall is
//! thicker or its frame moved, or a mod puts its door where the game has
//! one. The game shows both: one flickers through the other, the player
//! opens whichever is in front, and characters may walk through the one
//! that leads somewhere else. A real second door right behind the first,
//! a porch with an inner door, is rare.
//!
//! Doors of the same place stack when they stand almost on the same spot,
//! turned alike, their openings largely over each other, or when they
//! stand close and lead to the same place. For each group the analysis
//! tells where each door leads, whether the door there leads back, and
//! whether characters walk through it, suggests the door to keep, and
//! says for each door what keeping it takes: which doors to hide, which
//! teleports and door triangles of navigation meshes have to name the
//! kept door instead, and which places can then no longer be entered.
//!
//! Doors of the game that no mod touches are compared as well, since a
//! mod's door on top of one of the game's is the usual case; a group made
//! of the game's doors alone is not reported.

use std::collections::{BTreeMap, HashMap, HashSet};

use mallet_core::facts::RecordId;
use mallet_core::facts::doors::PlacedDoor;
use mallet_core::facts::objects::Place;
use mallet_core::{
    DoorDestination, DoorGroup, DoorMerge, Finding, MapSpot, ObjectRef, RecordChange, SpotKind,
    SpotLocation, SpotPart, StackedDoor, kinds,
};

use crate::objects::{Owners, cell_of, console};
use crate::{Analysis, Context, examples};

/// How many examples a finding names.
const EXAMPLE_LIMIT: usize = 5;

/// Doors farther apart than this (in game units, about 1.4 m) never stack.
const NEAR: f64 = 96.0;

/// Doors turned alike, whose openings lie over each other, stack when one
/// stands at most this far in front of the other (about 70 cm) ...
const STACK_DEPTH: f64 = 48.0;

/// ... and at most this far above or below it (about 45 cm).
const STACK_HEIGHT: f64 = 32.0;

/// Doors count as turned alike, or as turned round, within this many
/// thousandths of a radian (about 15 degrees).
const SAME_TURN: i64 = 262;

/// Where the box of a door is not known, its openings count as over each
/// other when they are this close sideways (about 45 cm).
const SIDEWAYS_UNKNOWN: f64 = 32.0;

/// Two doors this close in depth stand in the same plane and flicker.
const SAME_PLANE: f64 = 2.0;

/// Two doors of the same plugin stack only as copies: the same door this
/// close (about 11 cm).
const COPY: f64 = 8.0;

/// Outside, two destinations this close (in game units, about 7 m) are the
/// same place.
const SAME_DESTINATION: f64 = 512.0;

/// A kept door this far from a hidden one moves where the player arrives.
const MOVE_ARRIVAL: f64 = 4.0;

/// Side of the squares doors are sorted into to find neighbours.
const BUCKET: f64 = 256.0;

/// A full turn and half a turn, in thousandths of a radian.
const FULL_TURN: i64 = 6283;
const HALF_TURN: i64 = 3142;

/// Reports doors that stand on top of each other.
pub struct StackedDoors;

impl Analysis for StackedDoors {
    fn id(&self) -> &'static str {
        "stacked-doors"
    }

    fn run(&self, context: &Context<'_>) -> Vec<Finding> {
        context.doors().findings.clone()
    }
}

/// What the comparison of doors found.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DoorGroups {
    /// One finding for each pair of plugins whose doors stack.
    pub findings: Vec<Finding>,
    /// One spot for each group, with its doors and plans.
    pub spots: Vec<MapSpot>,
    /// Which group each door that is in one belongs to, by the index of
    /// its spot.
    pub members: HashMap<RecordId, usize>,
}

/// What is known about the doors of a setup, to answer questions about
/// one group.
struct Doors<'a> {
    by_id: HashMap<&'a RecordId, &'a PlacedDoor>,
    /// The doors that lead to each door.
    leading_here: HashMap<&'a RecordId, Vec<&'a PlacedDoor>>,
    /// The door triangles naming each door.
    links: HashMap<&'a RecordId, Vec<&'a mallet_core::facts::doors::DoorLink>>,
    names: HashMap<&'a RecordId, &'a str>,
}

impl<'a> Doors<'a> {
    fn new(facts: &'a mallet_core::Facts) -> Self {
        let doors = &facts.doors;
        let by_id = doors.doors.iter().map(|door| (&door.id, door)).collect();
        let mut leading_here: HashMap<&RecordId, Vec<&PlacedDoor>> = HashMap::new();
        for door in &doors.doors {
            if let Some(teleport) = &door.teleport {
                leading_here.entry(&teleport.door).or_default().push(door);
            }
        }
        let mut links: HashMap<&RecordId, Vec<_>> = HashMap::new();
        for link in &doors.links {
            links.entry(&link.door).or_default().push(link);
        }
        let names = doors
            .place_names
            .iter()
            .chain(&facts.objects.place_names)
            .map(|name| (&name.id, name.editor_id.as_str()))
            .collect();
        Self {
            by_id,
            leading_here,
            links,
            names,
        }
    }

    fn name_of(&self, id: &RecordId) -> String {
        self.names
            .get(id)
            .map_or_else(|| id.to_string(), |name| (*name).to_owned())
    }

    /// Where a door at `position` in `place` is, as a spot is.
    fn location(&self, place: &Place, position: [i32; 3]) -> SpotLocation {
        match place {
            Place::Interior { cell } => SpotLocation::Interior {
                cell: self.name_of(cell),
            },
            Place::Exterior { worldspace } => SpotLocation::Exterior {
                world: self.name_of(worldspace),
                x: cell_of(position[0]),
                y: cell_of(position[1]),
            },
        }
    }

    /// The door on the other side of `door`, if it leads somewhere that is
    /// there.
    fn other_side(&self, door: &PlacedDoor) -> Option<&'a PlacedDoor> {
        self.by_id.get(&door.teleport.as_ref()?.door).copied()
    }

    /// The editor ID of the interior cell of `place`; `None` outside.
    fn cell_name(&self, place: &Place) -> Option<String> {
        match place {
            Place::Interior { cell } => Some(self.name_of(cell)),
            Place::Exterior { .. } => None,
        }
    }

    fn walkable(&self, door: &RecordId) -> bool {
        self.links.get(door).is_some_and(|links| !links.is_empty())
    }
}

/// A group as a spot, with an example for its finding and its doors.
type Described = (MapSpot, String, Vec<RecordId>);

/// Compares the doors. Called once per scan through [`Context::doors`].
pub(crate) fn compute(context: &Context<'_>) -> DoorGroups {
    let facts = context.facts;
    if facts.doors.doors.is_empty() {
        return DoorGroups::default();
    }
    let doors = Doors::new(facts);
    let groups = groups_of(&facts.doors.doors, &doors);
    let owners = Owners::new(context);
    let mut pairs: BTreeMap<(String, String), Vec<Described>> = BTreeMap::new();
    for group in groups {
        let Some((spot, example, first, second)) = describe(&group, &doors, &owners) else {
            continue;
        };
        let ids = group.iter().map(|door| door.id.clone()).collect();
        pairs
            .entry((first, second))
            .or_default()
            .push((spot, example, ids));
    }
    // Spots are numbered in the order of the findings.
    let mut result = DoorGroups::default();
    for ((first, second), found) in pairs {
        let finding = Finding::new(&kinds::DOORS_STACKED, owners.pair(&first, &second))
            .with_arg("first", owners.display(&first))
            .with_arg("second", owners.display(&second))
            .with_arg("count", i64::try_from(found.len()).unwrap_or(i64::MAX))
            .with_arg(
                "examples",
                examples(
                    found.iter().map(|(_, example, _)| example.as_str()),
                    EXAMPLE_LIMIT,
                ),
            );
        for (mut spot, _, ids) in found {
            spot.finding = finding.id.clone();
            let index = result.spots.len();
            for id in ids {
                result.members.insert(id, index);
            }
            result.spots.push(spot);
        }
        result.findings.push(finding);
    }
    result
}

/// The groups of doors that stack, each sorted by identity.
fn groups_of<'a>(all: &'a [PlacedDoor], doors: &Doors<'a>) -> Vec<Vec<&'a PlacedDoor>> {
    let mut buckets: HashMap<(&Place, i64, i64), Vec<usize>> = HashMap::new();
    for (index, door) in all.iter().enumerate() {
        let (x, y) = bucket(door.position);
        buckets.entry((&door.place, x, y)).or_default().push(index);
    }
    let mut parent: Vec<usize> = (0..all.len()).collect();
    for (index, door) in all.iter().enumerate() {
        let (x, y) = bucket(door.position);
        for dx in -1..=1 {
            for dy in -1..=1 {
                let Some(others) = buckets.get(&(&door.place, x + dx, y + dy)) else {
                    continue;
                };
                for &other in others.iter().filter(|other| **other > index) {
                    if let Some(other_door) = all.get(other)
                        && stack(door, other_door, doors)
                    {
                        let (a, b) = (root(&mut parent, index), root(&mut parent, other));
                        if let Some(slot) = parent.get_mut(a.max(b)) {
                            *slot = a.min(b);
                        }
                    }
                }
            }
        }
    }
    let mut groups: BTreeMap<usize, Vec<&PlacedDoor>> = BTreeMap::new();
    for index in 0..all.len() {
        let group = root(&mut parent, index);
        if let Some(door) = all.get(index) {
            groups.entry(group).or_default().push(door);
        }
    }
    groups
        .into_values()
        .filter(|group| group.len() > 1)
        // The game's own doors alone are the game's business.
        .filter(|group| group.iter().any(|door| !door.official))
        .collect()
}

fn root(parent: &mut [usize], index: usize) -> usize {
    let mut at = index;
    while let Some(&next) = parent.get(at) {
        if next == at {
            break;
        }
        at = next;
    }
    // Shorten the way for the next time.
    if let Some(slot) = parent.get_mut(index) {
        *slot = at;
    }
    at
}

#[allow(clippy::cast_possible_truncation)]
fn bucket([x, y, _]: [i32; 3]) -> (i64, i64) {
    (
        (f64::from(x) / BUCKET).floor() as i64,
        (f64::from(y) / BUCKET).floor() as i64,
    )
}

/// Whether two doors of the same place stand on top of each other.
fn stack(a: &PlacedDoor, b: &PlacedDoor, doors: &Doors<'_>) -> bool {
    let delta = [
        f64::from(b.position[0]) - f64::from(a.position[0]),
        f64::from(b.position[1]) - f64::from(a.position[1]),
        f64::from(b.position[2]) - f64::from(a.position[2]),
    ];
    let distance = delta.iter().map(|value| value * value).sum::<f64>().sqrt();
    // A door without a model is not seen; mods put such a door where they
    // take the game's door away and keep its record for quests.
    if distance > NEAR || !a.visible || !b.visible {
        return false;
    }
    let same_destination = same_destination(a, b, doors);
    if a.id.plugin == b.id.plugin && a.plugin == b.plugin {
        // One plugin's doors side by side are mostly meant so: a gate of
        // two wings, a door behind a door. Only copies count.
        let leads_to_same_door = matches!(
            (&a.teleport, &b.teleport),
            (Some(first), Some(second)) if first.door == second.door
        );
        // Two doors that each lead to a door that leads back are two ways,
        // such as a ladder up and one down.
        let two_ways = !leads_to_same_door && leads_back(a, doors) && leads_back(b, doors);
        return leads_to_same_door || (a.base == b.base && distance <= COPY && !two_ways);
    }
    if same_destination {
        return true;
    }
    let turned_alike = {
        let difference =
            (i64::from(a.rotation[2]) - i64::from(b.rotation[2])).rem_euclid(FULL_TURN);
        let difference = difference.min(FULL_TURN - difference);
        difference <= SAME_TURN || (difference - HALF_TURN).abs() <= SAME_TURN
    };
    if !turned_alike || delta[2].abs() > STACK_HEIGHT {
        return false;
    }
    // In the frame of the first door: sideways along its width, and in
    // depth through its opening.
    let turn = f64::from(a.rotation[2]) / 1000.0;
    let (sin, cos) = turn.sin_cos();
    let sideways = (delta[0] * cos - delta[1] * sin).abs();
    let depth = (delta[0] * sin + delta[1] * cos).abs();
    let half_width = |door: &PlacedDoor| {
        door.bounds
            .map(|[min, max]| f64::from(max[0] - min[0]) / 2.0)
            .filter(|half| *half > 0.0)
    };
    let sideways_limit = match (half_width(a), half_width(b)) {
        (Some(first), Some(second)) => first.min(second),
        _ => SIDEWAYS_UNKNOWN,
    };
    sideways <= sideways_limit && depth <= STACK_DEPTH
}

/// Whether the door on the other side of `door` leads back to it.
fn leads_back(door: &PlacedDoor, doors: &Doors<'_>) -> bool {
    doors
        .other_side(door)
        .and_then(|other| other.teleport.as_ref())
        .is_some_and(|back| back.door == door.id)
}

/// Whether two doors lead to the same place: the same door, the same
/// room, or, outside, places close to each other.
fn same_destination(a: &PlacedDoor, b: &PlacedDoor, doors: &Doors<'_>) -> bool {
    let (Some(first), Some(second)) = (&a.teleport, &b.teleport) else {
        return false;
    };
    if first.door == second.door {
        return true;
    }
    let (Some(first_door), Some(second_door)) = (doors.other_side(a), doors.other_side(b)) else {
        return false;
    };
    match (&first_door.place, &second_door.place) {
        (Place::Interior { cell: first }, Place::Interior { cell: second }) => first == second,
        (Place::Exterior { worldspace: first }, Place::Exterior { worldspace: second }) => {
            first == second
                && distance(first_door.position, second_door.position) <= SAME_DESTINATION
        }
        _ => false,
    }
}

fn distance(a: [i32; 3], b: [i32; 3]) -> f64 {
    a.iter()
        .zip(b)
        .map(|(a, b)| (f64::from(*a) - f64::from(b)).powi(2))
        .sum::<f64>()
        .sqrt()
}

/// What is said about one group: its spot, an example for its finding,
/// and the two plugins of the finding. `None` for a group without doors.
fn describe(
    group: &[&PlacedDoor],
    doors: &Doors<'_>,
    owners: &Owners<'_>,
) -> Option<(MapSpot, String, String, String)> {
    let first = group.first()?;
    let object = |id: &RecordId| ObjectRef {
        plugin: owners.file_name(&id.plugin),
        local_id: id.local_id,
    };
    let fronts = fronts(group, doors);
    let flicker = fronts.flicker;
    let mut stacked: Vec<StackedDoor> = group
        .iter()
        .enumerate()
        .map(|(index, door)| {
            let other_side = doors.other_side(door);
            StackedDoor {
                object: object(&door.id),
                plugin: owners.file_name(&door.plugin),
                name: door
                    .base_name
                    .clone()
                    .unwrap_or_else(|| door.base.to_string()),
                leads_to: destination(door, doors, &object),
                broken: door.teleport.is_some() && other_side.is_none(),
                leads_back: other_side
                    .and_then(|other| other.teleport.as_ref())
                    .is_some_and(|back| back.door == door.id),
                walkable: doors.walkable(&door.id),
                other_side_walkable: other_side.is_some_and(|other| doors.walkable(&other.id)),
                references: references(door, doors),
                front: fronts.front == Some(index),
                offset: whole(distance(door.position, first.position)),
                suggested: false,
            }
        })
        .collect();
    // Doors that lead to places apart are not merged: the game cannot
    // take the player to both.
    let leads = |door: &&PlacedDoor| doors.other_side(door).is_some();
    let apart = group.iter().enumerate().any(|(index, a)| {
        leads(a)
            && group
                .iter()
                .skip(index + 1)
                .any(|b| leads(b) && !same_destination(a, b, doors))
    });
    let mut plans: Vec<DoorMerge> = group
        .iter()
        .map(|keep| plan(keep, group, apart, doors, owners, &object))
        .collect();
    suggest(&mut stacked, &mut plans, group, &object);

    let location = doors.location(&first.place, first.position);
    let place = match &location {
        SpotLocation::Interior { cell } => cell.clone(),
        SpotLocation::Exterior { world, x, y } => format!("{world} {x}, {y}"),
    };
    let mut names: Vec<&str> = stacked.iter().map(|door| door.name.as_str()).collect();
    names.sort_unstable();
    names.dedup();
    let objects = names.join(" / ");
    let parts = stacked
        .iter()
        .map(|door| SpotPart {
            object: door.object.clone(),
            name: door.name.clone(),
            // A teleport is taken care of by the merge; what else needs
            // the door, the 3D view tells (ADR 0046).
            hideable: true,
            suggested: !door.suggested,
        })
        .collect();
    let mut plugins: Vec<String> = group.iter().map(|door| door.id.plugin.clone()).collect();
    plugins.sort_unstable();
    plugins.dedup();
    let first_plugin = plugins.first()?.clone();
    let second_plugin = plugins
        .get(1)
        .cloned()
        .unwrap_or_else(|| first_plugin.clone());
    let centre = centre(group);
    let spot = MapSpot {
        finding: mallet_core::FindingId::derive("", &mallet_core::Subject::Setup),
        console: console(&location),
        location,
        objects: objects.clone(),
        parts,
        position: Some(centre),
        kind: SpotKind::Door,
        depth: None,
        doors: Some(DoorGroup {
            doors: stacked,
            plans,
            flicker,
            apart,
        }),
    };
    Some((
        spot,
        format!("{objects} ({place})"),
        first_plugin,
        second_plugin,
    ))
}

/// Where a door leads, if the door on the other side is there.
fn destination(
    door: &PlacedDoor,
    doors: &Doors<'_>,
    object: &dyn Fn(&RecordId) -> ObjectRef,
) -> Option<DoorDestination> {
    let other = doors.other_side(door)?;
    Some(DoorDestination {
        location: doors.location(&other.place, other.position),
        door: Some(object(&other.id)),
    })
}

/// How many doors lead to a door, how many door triangles name it, and
/// how many plugins change it.
fn references(door: &PlacedDoor, doors: &Doors<'_>) -> u32 {
    let count = doors.leading_here.get(&door.id).map_or(0, Vec::len)
        + doors.links.get(&door.id).map_or(0, Vec::len)
        + door.changed_by.len();
    u32::try_from(count).unwrap_or(u32::MAX)
}

/// Which door of a group the player sees in front, by its index, and
/// whether two stand in the same plane.
struct Fronts {
    front: Option<usize>,
    flicker: bool,
}

/// The front is the side where the player arrives through one of the
/// doors: the doors are measured along the way from the group to that
/// point.
fn fronts(group: &[&PlacedDoor], doors: &Doors<'_>) -> Fronts {
    let centre = centre(group);
    let arrival = group
        .iter()
        .flat_map(|door| doors.leading_here.get(&door.id).into_iter().flatten())
        .filter_map(|other| other.teleport.as_ref())
        .map(|teleport| teleport.position)
        .min();
    let direction = arrival.and_then(|arrival| {
        let x = f64::from(arrival[0]) - f64::from(centre[0]);
        let y = f64::from(arrival[1]) - f64::from(centre[1]);
        let length = x.hypot(y);
        (length >= 1.0).then(|| (x / length, y / length))
    });
    let Some((x, y)) = direction else {
        let flicker = group.iter().enumerate().any(|(index, a)| {
            group
                .iter()
                .skip(index + 1)
                .any(|b| distance(a.position, b.position) <= SAME_PLANE)
        });
        return Fronts {
            front: None,
            flicker,
        };
    };
    let depths: Vec<f64> = group
        .iter()
        .map(|door| {
            (f64::from(door.position[0]) - f64::from(centre[0])) * x
                + (f64::from(door.position[1]) - f64::from(centre[1])) * y
        })
        .collect();
    let mut sorted = depths.clone();
    sorted.sort_by(f64::total_cmp);
    let flicker = sorted
        .windows(2)
        .any(|pair| matches!(pair, [a, b] if b - a <= SAME_PLANE));
    let front = match sorted.as_slice() {
        [.., second, first] if first - second > SAME_PLANE => depths
            .iter()
            .position(|depth| depth.total_cmp(first).is_eq()),
        _ => None,
    };
    Fronts { front, flicker }
}

/// The point in the middle of a group.
fn centre(group: &[&PlacedDoor]) -> [i32; 3] {
    let count = i64::try_from(group.len()).unwrap_or(1).max(1);
    let mut sum = [0i64; 3];
    for door in group {
        for (total, value) in sum.iter_mut().zip(door.position) {
            *total += i64::from(value);
        }
    }
    sum.map(|total| i32::try_from(total / count).unwrap_or_default())
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn whole(length: f64) -> u32 {
    length.round().clamp(0.0, f64::from(u32::MAX)) as u32
}

/// What keeping `keep` of `group` means.
fn plan(
    keep: &PlacedDoor,
    group: &[&PlacedDoor],
    apart: bool,
    doors: &Doors<'_>,
    owners: &Owners<'_>,
    object: &dyn Fn(&RecordId) -> ObjectRef,
) -> DoorMerge {
    let in_group: HashSet<&RecordId> = group.iter().map(|door| &door.id).collect();
    let hidden: Vec<&PlacedDoor> = group
        .iter()
        .filter(|door| door.id != keep.id)
        .copied()
        .collect();
    let kept_destination = destination(keep, doors, object).map(|place| place.location);
    let mut changes: Vec<RecordChange> = Vec::new();
    let mut lost: Vec<DoorDestination> = Vec::new();
    // A door whose teleport leads to a door that is not there leads
    // nowhere, as one without a teleport.
    let mut takes_teleport = kept_destination.is_none();
    for door in &hidden {
        if apart {
            // The doors lead to places apart: the kept one keeps its own,
            // the others' are no longer entered here. Nothing is merged.
            if let Some(place) = destination(door, doors, object)
                && kept_destination.as_ref() != Some(&place.location)
                && !lost.iter().any(|known| known.location == place.location)
            {
                lost.push(place);
            }
            continue;
        }
        // The doors on the other side that led to the hidden door lead to
        // the kept one, the player arriving as far moved as the doors are.
        let shift = [
            keep.position[0] - door.position[0],
            keep.position[1] - door.position[1],
            keep.position[2] - door.position[2],
        ];
        let moves = distance(keep.position, door.position) >= MOVE_ARRIVAL;
        for other in doors
            .leading_here
            .get(&door.id)
            .into_iter()
            .flatten()
            .filter(|other| !in_group.contains(&other.id))
        {
            let Some(teleport) = &other.teleport else {
                continue;
            };
            changes.push(RecordChange::Teleport {
                door: object(&other.id),
                at: other.position,
                cell: doors.cell_name(&other.place),
                plugin: owners.file_name(&other.plugin),
                from: Some(object(&door.id)),
                to: object(&keep.id),
                arrival: moves.then(|| {
                    [
                        teleport.position[0].saturating_add(shift[0]),
                        teleport.position[1].saturating_add(shift[1]),
                        teleport.position[2].saturating_add(shift[2]),
                    ]
                }),
                facing: None,
            });
        }
        // A kept door that led nowhere takes over where the hidden one led.
        if takes_teleport
            && destination(door, doors, object).is_some()
            && let Some(teleport) = &door.teleport
        {
            takes_teleport = false;
            changes.push(RecordChange::Teleport {
                door: object(&keep.id),
                at: keep.position,
                cell: doors.cell_name(&keep.place),
                plugin: owners.file_name(&keep.plugin),
                from: None,
                to: object(&teleport.door),
                arrival: Some(teleport.position),
                facing: Some(teleport.rotation),
            });
        }
        // Characters step through the kept door where they stepped through
        // the hidden one.
        for link in doors.links.get(&door.id).into_iter().flatten() {
            changes.push(RecordChange::NavmeshDoor {
                navmesh: object(&link.navmesh),
                plugin: owners.file_name(&link.plugin),
                triangle: link.triangle,
                from: object(&door.id),
                to: object(&keep.id),
            });
        }
    }
    DoorMerge {
        keep: object(&keep.id),
        hide: hidden.iter().map(|door| object(&door.id)).collect(),
        changes,
        lost,
    }
}

/// Puts the door suggested to keep first, with its plan, and measures the
/// others from it. The door to keep: not one that the plugin of another
/// door of the group changed, which put it aside; its other side leads
/// back and characters walk through both, many things lead to it; then the plan
/// that changes the least, then the door the player sees.
fn suggest(
    stacked: &mut Vec<StackedDoor>,
    plans: &mut Vec<DoorMerge>,
    group: &[&PlacedDoor],
    object: &dyn Fn(&RecordId) -> ObjectRef,
) {
    let score = |door: &StackedDoor| {
        u32::from(door.leads_back) * 4
            + u32::from(door.walkable) * 2
            + u32::from(door.other_side_walkable) * 2
            + door.references.min(5)
    };
    // A door changed by the plugin that placed another door of the group
    // was put aside by it: that plugin meant its own door to be used.
    let put_aside = |door: &StackedDoor| {
        !door.plugin.eq_ignore_ascii_case(&door.object.plugin)
            && stacked
                .iter()
                .any(|other| other.object.plugin.eq_ignore_ascii_case(&door.plugin))
    };
    let best = stacked
        .iter()
        .zip(plans.iter())
        .enumerate()
        .max_by_key(|(index, (door, plan))| {
            (
                !put_aside(door),
                score(door),
                std::cmp::Reverse(plan.changes.len() + plan.lost.len()),
                door.front,
                std::cmp::Reverse(*index),
            )
        })
        .map_or(0, |(index, _)| index);
    if best < stacked.len() && best < plans.len() {
        let kept = stacked.remove(best);
        stacked.insert(0, kept);
        let plan = plans.remove(best);
        plans.insert(0, plan);
    }
    let Some(kept) = stacked.first_mut() else {
        return;
    };
    kept.suggested = true;
    let kept_object = kept.object.clone();
    let Some(kept_at) = group
        .iter()
        .find(|door| object(&door.id) == kept_object)
        .map(|door| door.position)
    else {
        return;
    };
    for door in stacked.iter_mut() {
        if let Some(placed) = group
            .iter()
            .find(|placed| object(&placed.id) == door.object)
        {
            door.offset = whole(distance(placed.position, kept_at));
        }
    }
}

#[cfg(test)]
mod tests {
    use mallet_core::facts::doors::{DoorFacts, DoorLink, Teleport};
    use mallet_core::facts::objects::PlaceName;
    use mallet_core::{Facts, NoModSource};

    use super::*;

    fn id(plugin: &str, local_id: u32) -> RecordId {
        RecordId {
            plugin: plugin.to_owned(),
            local_id,
        }
    }

    fn outside() -> Place {
        Place::Exterior {
            worldspace: id("skyrim.esm", 0x3C),
        }
    }

    fn room(local_id: u32) -> Place {
        Place::Interior {
            cell: id("skyrim.esm", local_id),
        }
    }

    fn door(
        plugin: &str,
        local_id: u32,
        place: Place,
        position: [i32; 3],
        leads_to: Option<(&str, u32, [i32; 3])>,
    ) -> PlacedDoor {
        PlacedDoor {
            id: id(plugin, local_id),
            plugin: plugin.to_owned(),
            official: plugin == "skyrim.esm",
            changed_by: Vec::new(),
            base: id("skyrim.esm", 0xD00),
            base_name: Some("HouseDoor".to_owned()),
            visible: true,
            bounds: Some([[-60, -10, 0], [60, 10, 200]]),
            place,
            position,
            rotation: [0, 0, 0],
            persistent: true,
            enable_parent: None,
            teleport: leads_to.map(|(plugin, local_id, position)| Teleport {
                door: id(plugin, local_id),
                position,
                rotation: [0, 0, 0],
                flags: 0,
            }),
        }
    }

    fn link(plugin: &str, navmesh: u32, triangle: u32, door: RecordId) -> DoorLink {
        DoorLink {
            navmesh: id(plugin, navmesh),
            plugin: plugin.to_owned(),
            triangle,
            door,
        }
    }

    /// The game's house: a door outside and one inside, leading to each
    /// other, both on the paths. A mod puts its own door 6 units in front
    /// of the game's, leading into the same room but not back.
    fn house() -> Facts {
        Facts {
            doors: DoorFacts {
                doors: vec![
                    door(
                        "skyrim.esm",
                        0xA01,
                        room(0xE00),
                        [0, 0, 0],
                        Some(("skyrim.esm", 0xA02, [1000, -100, 0])),
                    ),
                    door(
                        "skyrim.esm",
                        0xA02,
                        outside(),
                        [1000, 0, 0],
                        Some(("skyrim.esm", 0xA01, [0, 100, 0])),
                    ),
                    door(
                        "town.esp",
                        0x800,
                        outside(),
                        [1000, -6, 0],
                        Some(("skyrim.esm", 0xA01, [0, 100, 0])),
                    ),
                    // A door far away, not part of anything.
                    door("town.esp", 0x801, outside(), [5000, 0, 0], None),
                ],
                links: vec![
                    link("skyrim.esm", 0xB00, 3, id("skyrim.esm", 0xA01)),
                    link("skyrim.esm", 0xB01, 7, id("skyrim.esm", 0xA02)),
                ],
                place_names: vec![
                    PlaceName {
                        id: id("skyrim.esm", 0xE00),
                        editor_id: "HouseInterior".to_owned(),
                    },
                    PlaceName {
                        id: id("skyrim.esm", 0x3C),
                        editor_id: "Tamriel".to_owned(),
                    },
                ],
            },
            ..Facts::default()
        }
    }

    fn groups(facts: &Facts) -> DoorGroups {
        let context = Context::new(facts, &NoModSource);
        compute(&context)
    }

    fn object(plugin: &str, local_id: u32) -> ObjectRef {
        ObjectRef {
            plugin: plugin.to_owned(),
            local_id,
        }
    }

    #[test]
    fn a_mods_door_in_front_of_the_games_is_found() {
        let found = groups(&house());
        assert_eq!(found.findings.len(), 1);
        let finding = found.findings.first().unwrap();
        assert_eq!(finding.kind, "doors.stacked");
        assert_eq!(found.spots.len(), 1);
        let spot = found.spots.first().unwrap();
        assert_eq!(spot.kind, SpotKind::Door);
        assert_eq!(spot.finding, finding.id);
        assert_eq!(found.members.len(), 2);
        let group = spot.doors.as_ref().unwrap();
        assert_eq!(group.doors.len(), 2);
        // The game's door leads back and is on the paths: it is kept.
        let kept = group.doors.first().unwrap();
        assert!(kept.suggested);
        assert_eq!(kept.object, object("skyrim.esm", 0xA02));
        assert!(kept.leads_back && kept.walkable && kept.other_side_walkable);
        let other = group.doors.get(1).unwrap();
        assert!(!other.leads_back && !other.walkable);
        assert_eq!(other.offset, 6);
        // Both lead into the house.
        assert_eq!(
            other.leads_to.as_ref().unwrap().location,
            SpotLocation::Interior {
                cell: "HouseInterior".to_owned()
            }
        );
        // The room's door sends the player to y = -100, in front of the
        // mod's door at y = -6.
        assert!(other.front);
        assert!(!group.flicker);
    }

    #[test]
    fn keeping_the_suggested_door_needs_no_change() {
        let found = groups(&house());
        let group = found.spots.first().unwrap().doors.as_ref().unwrap();
        let plan = group.plans.first().unwrap();
        assert_eq!(plan.keep, object("skyrim.esm", 0xA02));
        assert_eq!(plan.hide, vec![object("town.esp", 0x800)]);
        assert_eq!(plan.changes.len(), 0);
        assert_eq!(plan.lost.len(), 0);
    }

    #[test]
    fn keeping_the_other_door_names_every_change() {
        let found = groups(&house());
        let group = found.spots.first().unwrap().doors.as_ref().unwrap();
        let plan = group.plans.get(1).unwrap();
        assert_eq!(plan.keep, object("town.esp", 0x800));
        assert_eq!(
            plan.changes,
            vec![
                RecordChange::Teleport {
                    door: object("skyrim.esm", 0xA01),
                    at: [0, 0, 0],
                    cell: Some("HouseInterior".to_owned()),
                    plugin: "skyrim.esm".to_owned(),
                    from: Some(object("skyrim.esm", 0xA02)),
                    to: object("town.esp", 0x800),
                    arrival: Some([1000, -106, 0]),
                    facing: None,
                },
                RecordChange::NavmeshDoor {
                    navmesh: object("skyrim.esm", 0xB01),
                    plugin: "skyrim.esm".to_owned(),
                    triangle: 7,
                    from: object("skyrim.esm", 0xA02),
                    to: object("town.esp", 0x800),
                },
            ]
        );
        assert_eq!(plan.lost.len(), 0);
    }

    #[test]
    fn doors_leading_to_places_apart_are_not_merged() {
        let mut facts = house();
        facts.doors.doors.push(door(
            "town.esp",
            0x900,
            room(0xE01),
            [0, 0, 0],
            Some(("town.esp", 0x800, [1000, -120, 0])),
        ));
        if let Some(own) = facts.doors.doors.get_mut(2) {
            own.teleport = Some(Teleport {
                door: id("town.esp", 0x900),
                position: [0, 100, 0],
                rotation: [0, 0, 0],
                flags: 0,
            });
        }
        let found = groups(&facts);
        let group = found.spots.first().unwrap().doors.as_ref().unwrap();
        assert!(group.apart);
        let keep_game = group
            .plans
            .iter()
            .find(|plan| plan.keep == object("skyrim.esm", 0xA02))
            .unwrap();
        assert_eq!(
            keep_game.lost.first().map(|place| &place.location),
            Some(&SpotLocation::Interior {
                cell: "skyrim.esm:000E01".to_owned()
            })
        );
        // Each keeps its own way: nothing is led elsewhere.
        assert_eq!(keep_game.changes, []);
    }

    #[test]
    fn a_kept_door_whose_way_is_broken_takes_over_the_hidden_ones() {
        // The mod's door leads to a door that is not there; keeping it
        // still keeps the way into the house, both ways.
        let mut facts = house();
        if let Some(own) = facts.doors.doors.get_mut(2) {
            own.teleport = Some(Teleport {
                door: id("town.esp", 0x999),
                position: [0, 0, 0],
                rotation: [0, 0, 0],
                flags: 0,
            });
        }
        let found = groups(&facts);
        let group = found.spots.first().unwrap().doors.as_ref().unwrap();
        assert!(!group.apart);
        let keep_mod = group
            .plans
            .iter()
            .find(|plan| plan.keep == object("town.esp", 0x800))
            .unwrap();
        assert_eq!(keep_mod.lost, []);
        let led = |from: ObjectRef, to: ObjectRef| {
            keep_mod.changes.iter().any(|change| matches!(
                change,
                RecordChange::Teleport { door, to: target, .. } if *door == from && *target == to
            ))
        };
        // It leads into the house, and the house's door leads back to it.
        assert!(led(object("town.esp", 0x800), object("skyrim.esm", 0xA01)));
        assert!(led(object("skyrim.esm", 0xA01), object("town.esp", 0x800)));
    }

    #[test]
    fn doors_turned_across_or_far_apart_do_not_stack() {
        let mut facts = house();
        if let Some(own) = facts.doors.doors.get_mut(2) {
            own.rotation = [0, 0, 1571];
            own.teleport = None;
        }
        assert_eq!(groups(&facts).spots.len(), 0);
        let mut facts = house();
        if let Some(own) = facts.doors.doors.get_mut(2) {
            own.position = [1000, -80, 0];
            own.teleport = None;
        }
        assert_eq!(groups(&facts).spots.len(), 0);
    }

    #[test]
    fn the_games_doors_alone_are_not_reported() {
        let mut facts = house();
        if let Some(own) = facts.doors.doors.get_mut(2) {
            own.id = id("skyrim.esm", 0x800);
            own.plugin = "skyrim.esm".to_owned();
            own.official = true;
        }
        assert_eq!(groups(&facts).spots.len(), 0);
    }

    #[test]
    fn two_doors_of_one_plugin_stack_only_as_copies() {
        let mut facts = Facts::default();
        facts.doors.doors = vec![
            door("town.esp", 0x800, outside(), [0, 0, 0], None),
            door("town.esp", 0x801, outside(), [0, 30, 0], None),
        ];
        assert_eq!(groups(&facts).spots.len(), 0);
        facts.doors.doors = vec![
            door("town.esp", 0x800, outside(), [0, 0, 0], None),
            door("town.esp", 0x801, outside(), [0, 1, 0], None),
        ];
        let found = groups(&facts);
        let group = found.spots.first().unwrap().doors.as_ref().unwrap();
        assert!(group.flicker);
    }

    #[test]
    fn a_door_the_other_mod_put_aside_is_not_suggested() {
        let mut games = door("skyrim.esm", 0xA02, outside(), [0, 0, 0], None);
        games.plugin = "town.esp".to_owned();
        games.changed_by = vec!["skyrim.esm".to_owned()];
        let facts = Facts {
            doors: DoorFacts {
                doors: vec![games, door("town.esp", 0x800, outside(), [0, -7, 0], None)],
                ..DoorFacts::default()
            },
            ..Facts::default()
        };
        let found = groups(&facts);
        let group = found.spots.first().unwrap().doors.as_ref().unwrap();
        assert_eq!(
            group.doors.first().unwrap().object,
            object("town.esp", 0x800)
        );
    }
}
