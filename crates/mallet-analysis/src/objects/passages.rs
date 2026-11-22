//! Passages: where two mods build the same thing twice (ADR 0072).
//!
//! A collision spot is one pair of parts. Where two mods build the same
//! bridge or the same forge, they collide at dozens of spots, and yet the
//! user decides one thing: whose version stays. A passage gathers those
//! spots with the parts around them: the parts of both mods that collide,
//! grown through each mod's own structure (parts of the same mod whose
//! boxes touch), but only as far as the other mod's parts in the passage
//! come close. So a passage ends where the two versions part ways, not
//! where either mod's town ends.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, HashMap};

use mallet_core::facts::RecordId;
use mallet_core::facts::objects::Place;
use mallet_core::{MapPassage, PassageSeverity, SpotPart};

use super::{Clash, MAX_BUCKETS, Owners, Solid, Vector, console, location_of, spot_part, square};

/// Parts of the same mod whose boxes come this close belong to one
/// structure, in game units (about 11 cm).
const TOUCHING: f64 = 8.0;

/// A part grown from the passage joins it only where it comes this close
/// to the other mod's parts in it, in game units (about 90 cm): there the
/// two versions stand in each other or nearly coincide.
const NEAR: f64 = 64.0;

/// No part farther than this from a collision of the two mods joins, in
/// game units (about 30 m).
const REACH: f64 = 2048.0;

/// Two parts with the same model this close are twins, in game units
/// (about 17 cm).
const TWIN: f64 = 12.0;

/// A passage has at least this many collision spots ...
const MIN_SPOTS: usize = 2;

/// ... at least this many parts of each mod ...
const MIN_SIDE: usize = 2;

/// ... and at least this many parts in all. Fewer are single spots.
const MIN_PARTS: usize = 5;

/// A side grows to this many parts at most.
const MOST: usize = 3000;

/// Points sampled along each side of a box to tell how close it comes to
/// another.
const NEAR_SAMPLES: u32 = 5;

/// Points sampled along each side of a box to tell how much of it lies in
/// the other side's parts.
const INSIDE_SAMPLES: u32 = 3;

/// Cubic game units in a litre: a metre is about 70 units.
const UNITS_PER_LITRE: f64 = 343.0;

/// The passages among the collisions `collisions` (pairs of indices into
/// `solids`), the most serious first.
pub(super) fn find(
    solids: &[Solid<'_>],
    collisions: &[(usize, usize, Clash)],
    owners: &Owners<'_>,
    names: &HashMap<&RecordId, &str>,
) -> Vec<MapPassage> {
    let groups = groups_of(solids, collisions);
    if groups.is_empty() {
        return Vec::new();
    }
    let involved: BTreeSet<&str> = groups
        .keys()
        .flat_map(|(first, second, _)| [*first, *second])
        .collect();
    let grid = Grid::new(solids, &involved);
    let mut passages: Vec<MapPassage> = Vec::new();
    for ((first, second, place), seeds) in &groups {
        let members = grow(solids, &grid, [first, second], place, seeds);
        for component in components(solids, &members, seeds) {
            if let Some(passage) = passage_of(solids, &component, seeds, owners, names) {
                passages.push(passage);
            }
        }
    }
    passages.sort_by(|a, b| {
        let total = |passage: &MapPassage| passage.parts[0].len() + passage.parts[1].len();
        let first = |passage: &MapPassage| {
            passage
                .parts
                .iter()
                .flatten()
                .next()
                .map(|part| (part.object.plugin.to_lowercase(), part.object.local_id))
        };
        a.severity
            .cmp(&b.severity)
            .then_with(|| Reverse(total(a)).cmp(&Reverse(total(b))))
            .then_with(|| a.plugins.cmp(&b.plugins))
            .then_with(|| first(a).cmp(&first(b)))
    });
    passages
}

/// The collisions by the pair of plugins (in the order of their names) and
/// the place, each as the first plugin's part and the second's.
type Groups<'s> = BTreeMap<(&'s str, &'s str, &'s Place), Vec<[usize; 2]>>;

fn groups_of<'s>(solids: &'s [Solid<'s>], collisions: &[(usize, usize, Clash)]) -> Groups<'s> {
    let mut groups: Groups<'s> = BTreeMap::new();
    for &(a, b, _) in collisions {
        let (Some(first), Some(second)) = (solids.get(a), solids.get(b)) else {
            continue;
        };
        let pair = if first.owner() <= second.owner() {
            [a, b]
        } else {
            [b, a]
        };
        let (Some(low), Some(high)) = (solids.get(pair[0]), solids.get(pair[1])) else {
            continue;
        };
        groups
            .entry((low.owner(), high.owner(), &low.object.place))
            .or_default()
            .push(pair);
    }
    groups
}

/// The solids of the plugins involved in a collision, by the squares their
/// boxes reach into, to find what touches a part. Pieces of landscape are
/// left out: a structure does not grow through rocks and trees.
struct Grid<'s> {
    squares: HashMap<(&'s Place, i64, i64), Vec<usize>>,
}

impl<'s> Grid<'s> {
    fn new(solids: &'s [Solid<'s>], involved: &BTreeSet<&str>) -> Self {
        let mut squares: HashMap<(&'s Place, i64, i64), Vec<usize>> = HashMap::new();
        for (index, solid) in solids.iter().enumerate() {
            if solid.base.natural || !involved.contains(solid.owner()) {
                continue;
            }
            let (x0, y0, x1, y1) = solid.squares();
            if (x1 - x0 + 1) * (y1 - y0 + 1) > MAX_BUCKETS {
                continue;
            }
            for x in x0..=x1 {
                for y in y0..=y1 {
                    squares
                        .entry((&solid.object.place, x, y))
                        .or_default()
                        .push(index);
                }
            }
        }
        Self { squares }
    }

    /// The solids of `owner` in `place` whose squares are near those of
    /// `solid`, each once.
    fn around(&self, solid: &Solid<'_>, owner: &str, solids: &[Solid<'_>]) -> BTreeSet<usize> {
        let place = &solid.object.place;
        let mut found = BTreeSet::new();
        let (x0, y0) = (
            square(solid.world_min.x - TOUCHING),
            square(solid.world_min.y - TOUCHING),
        );
        let (x1, y1) = (
            square(solid.world_max.x + TOUCHING),
            square(solid.world_max.y + TOUCHING),
        );
        if (x1 - x0 + 1) * (y1 - y0 + 1) > MAX_BUCKETS {
            return found;
        }
        for x in x0..=x1 {
            for y in y0..=y1 {
                let Some(members) = self.squares.get(&(place, x, y)) else {
                    continue;
                };
                found.extend(members.iter().copied().filter(|&index| {
                    solids
                        .get(index)
                        .is_some_and(|other| other.owner() == owner)
                }));
            }
        }
        found
    }
}

/// The parts of the two plugins in a passage, grown from the collisions
/// `seeds`: a side grows through its own parts that touch it, but only by
/// those that come near the other side's parts, and none farther than
/// [`REACH`] from a collision.
fn grow(
    solids: &[Solid<'_>],
    grid: &Grid<'_>,
    owners: [&str; 2],
    place: &Place,
    seeds: &[[usize; 2]],
) -> [BTreeSet<usize>; 2] {
    let mut first: BTreeSet<usize> = seeds.iter().map(|[a, _]| *a).collect();
    let mut second: BTreeSet<usize> = seeds.iter().map(|[_, b]| *b).collect();
    let spots: Vec<Vector> = seeds
        .iter()
        .filter_map(|[a, b]| {
            let (a, b) = (solids.get(*a)?, solids.get(*b)?);
            Some(a.position.plus(b.position).scaled(0.5))
        })
        .collect();
    let within_reach = |solid: &Solid<'_>| {
        spots.iter().any(|spot| {
            let apart = solid.position.minus(*spot);
            apart.dot(apart) <= REACH * REACH
        })
    };
    let [first_owner, second_owner] = owners;
    let mut fresh: [Vec<usize>; 2] = [
        first.iter().copied().collect(),
        second.iter().copied().collect(),
    ];
    let mut waiting: [BTreeSet<usize>; 2] = [BTreeSet::new(), BTreeSet::new()];
    loop {
        // What touches the parts that joined last.
        for (side, owner) in [(0, first_owner), (1, second_owner)] {
            let own = if side == 0 { &first } else { &second };
            let mut found = BTreeSet::new();
            for &member in fresh.get(side).map_or(&[][..], Vec::as_slice) {
                let Some(from) = solids.get(member) else {
                    continue;
                };
                for candidate in grid.around(from, owner, solids) {
                    if own.contains(&candidate) {
                        continue;
                    }
                    let Some(other) = solids.get(candidate) else {
                        continue;
                    };
                    if &other.object.place == place
                        && from.boxes_meet_within(other, TOUCHING)
                        && within_reach(other)
                    {
                        found.insert(candidate);
                    }
                }
            }
            if let Some(waiting) = waiting.get_mut(side) {
                waiting.extend(found);
            }
        }
        fresh = [Vec::new(), Vec::new()];
        // Of those, the ones that come near the other side join.
        let mut grew = false;
        for side in [0, 1] {
            let (own, other) = if side == 0 {
                (&mut first, &second)
            } else {
                (&mut second, &first)
            };
            let Some(candidates) = waiting.get_mut(side) else {
                continue;
            };
            let joining: Vec<usize> = candidates
                .iter()
                .copied()
                .filter(|&candidate| {
                    solids.get(candidate).is_some_and(|solid| {
                        other.iter().any(|&member| {
                            solids
                                .get(member)
                                .is_some_and(|member| solid.near(member, NEAR))
                        })
                    })
                })
                .take(MOST.saturating_sub(own.len()))
                .collect();
            for candidate in joining {
                candidates.remove(&candidate);
                own.insert(candidate);
                if let Some(fresh) = fresh.get_mut(side) {
                    fresh.push(candidate);
                }
                grew = true;
            }
        }
        if !grew {
            break;
        }
    }
    [first, second]
}

/// The parts of both sides that hang together: through their own
/// structure, through coming near each other, or through a collision.
fn components(
    solids: &[Solid<'_>],
    members: &[BTreeSet<usize>; 2],
    seeds: &[[usize; 2]],
) -> Vec<[Vec<usize>; 2]> {
    let [first, second] = members;
    let all: Vec<(usize, usize)> = first
        .iter()
        .map(|&index| (0, index))
        .chain(second.iter().map(|&index| (1, index)))
        .collect();
    let position: HashMap<(usize, usize), usize> = all
        .iter()
        .enumerate()
        .map(|(at, &member)| (member, at))
        .collect();
    let mut root: Vec<usize> = (0..all.len()).collect();
    let find = |root: &mut Vec<usize>, start: usize| {
        let mut at = start;
        while let Some(&up) = root.get(at) {
            if up == at {
                break;
            }
            at = up;
        }
        at
    };
    let join = |root: &mut Vec<usize>, a: usize, b: usize| {
        let (a, b) = (find(root, a), find(root, b));
        if a != b
            && let Some(slot) = root.get_mut(b)
        {
            *slot = a;
        }
    };
    for [a, b] in seeds {
        if let (Some(&a), Some(&b)) = (position.get(&(0, *a)), position.get(&(1, *b))) {
            join(&mut root, a, b);
        }
    }
    for (i, &(side_a, a)) in all.iter().enumerate() {
        let Some(solid_a) = solids.get(a) else {
            continue;
        };
        for (j, &(side_b, b)) in all.iter().enumerate().skip(i + 1) {
            let Some(solid_b) = solids.get(b) else {
                continue;
            };
            let linked = if side_a == side_b {
                solid_a.boxes_meet_within(solid_b, TOUCHING)
            } else {
                solid_a.near(solid_b, NEAR)
            };
            if linked {
                join(&mut root, i, j);
            }
        }
    }
    let mut joined: BTreeMap<usize, [Vec<usize>; 2]> = BTreeMap::new();
    for (at, &(side, index)) in all.iter().enumerate() {
        let top = find(&mut root, at);
        let entry = joined.entry(top).or_default();
        if let Some(list) = entry.get_mut(side) {
            list.push(index);
        }
    }
    joined.into_values().collect()
}

/// The passage of one component, if it is more than a single spot.
fn passage_of(
    solids: &[Solid<'_>],
    component: &[Vec<usize>; 2],
    seeds: &[[usize; 2]],
    owners: &Owners<'_>,
    names: &HashMap<&RecordId, &str>,
) -> Option<MapPassage> {
    let [first, second] = component;
    let spots = seeds
        .iter()
        .filter(|[a, b]| first.contains(a) && second.contains(b))
        .count();
    if spots < MIN_SPOTS
        || first.len() < MIN_SIDE
        || second.len() < MIN_SIDE
        || first.len() + second.len() < MIN_PARTS
    {
        return None;
    }
    let side = |indices: &[usize]| -> Vec<&Solid<'_>> {
        indices.iter().filter_map(|&at| solids.get(at)).collect()
    };
    let (a, b) = (side(first), side(second));
    let all: Vec<&Solid<'_>> = a.iter().chain(&b).copied().collect();
    let anchor = all.first()?;
    let count = i64::try_from(all.len()).unwrap_or(i64::MAX).max(1);
    let mut sum = [0i64; 3];
    for solid in &all {
        for (total, value) in sum.iter_mut().zip(solid.object.position) {
            *total += i64::from(value);
        }
    }
    let centre = sum.map(|total| i32::try_from(total / count).unwrap_or_default());
    let mut low = Vector::new(f64::MAX, f64::MAX, f64::MAX);
    let mut high = Vector::new(f64::MIN, f64::MIN, f64::MIN);
    for solid in &all {
        low = low.min(solid.world_min);
        high = high.max(solid.world_max);
    }
    let location = location_of(&anchor.object.place, centre, names);
    let parts_of = |solids: &[&Solid<'_>]| -> Vec<SpotPart> {
        let mut parts: Vec<SpotPart> = solids
            .iter()
            .map(|solid| {
                let mut part = spot_part(solid);
                part.object.plugin = owners.file_name(solid.owner());
                part
            })
            .collect();
        parts.sort_by(|x, y| {
            (x.object.plugin.to_lowercase(), x.object.local_id)
                .cmp(&(y.object.plugin.to_lowercase(), y.object.local_id))
        });
        parts
    };
    let duplicates = twins(&a, &b);
    let inside = [share_inside(&a, &b), share_inside(&b, &a)];
    let volumes = [litres(&a), litres(&b)];
    let smaller_side = a.len().min(b.len());
    // The side with less room: how much of it stands in the other.
    let inside_of_smaller = if volumes[0] <= volumes[1] {
        inside[0]
    } else {
        inside[1]
    };
    let severity = if duplicates >= 2 && duplicates * 5 >= smaller_side {
        PassageSeverity::Doubled
    } else if inside_of_smaller >= 250 {
        PassageSeverity::Inside
    } else {
        PassageSeverity::Crossing
    };
    let first_plugin = owners.file_name(a.first()?.owner());
    let second_plugin = owners.file_name(b.first()?.owner());
    Some(MapPassage {
        plugins: [first_plugin, second_plugin],
        console: console(&location),
        location,
        centre,
        min: [low.x, low.y, low.z].map(whole),
        max: [high.x, high.y, high.z].map(whole),
        parts: [parts_of(&a), parts_of(&b)],
        spots: u32::try_from(spots).unwrap_or(u32::MAX),
        duplicates: u32::try_from(duplicates).unwrap_or(u32::MAX),
        inside,
        volumes,
        severity,
    })
}

/// How many parts of `a` have a twin among `b`: the same model, or the
/// same base object, within [`TWIN`]. Each part of `b` is one twin at most.
fn twins(a: &[&Solid<'_>], b: &[&Solid<'_>]) -> usize {
    let mut taken = vec![false; b.len()];
    let mut count = 0;
    for one in a {
        let found = b.iter().enumerate().position(|(at, other)| {
            !taken.get(at).copied().unwrap_or(true)
                && same_model(one, other)
                && one.distance(other) <= TWIN
        });
        if let Some(slot) = found.and_then(|at| taken.get_mut(at)) {
            *slot = true;
            count += 1;
        }
    }
    count
}

fn same_model(a: &Solid<'_>, b: &Solid<'_>) -> bool {
    a.object.base == b.object.base
        || a.base
            .model
            .as_deref()
            .is_some_and(|model| b.base.model.as_deref() == Some(model))
}

/// How much of the room `parts` take lies inside the parts of `other`, in
/// thousandths, measured at points spread through each part's box.
fn share_inside(parts: &[&Solid<'_>], other: &[&Solid<'_>]) -> u32 {
    let step = |index: u32| (f64::from(index) + 0.5) / f64::from(INSIDE_SAMPLES);
    let mut total = 0.0;
    let mut inside = 0.0;
    for solid in parts {
        let volume = solid.volume();
        total += volume;
        let near: Vec<&&Solid<'_>> = other
            .iter()
            .filter(|candidate| solid.boxes_meet(candidate))
            .collect();
        if near.is_empty() {
            continue;
        }
        let size = solid.max.minus(solid.min);
        let mut hits = 0u32;
        for i in 0..INSIDE_SAMPLES {
            for j in 0..INSIDE_SAMPLES {
                for k in 0..INSIDE_SAMPLES {
                    let local = Vector::new(
                        solid.min.x + size.x * step(i),
                        solid.min.y + size.y * step(j),
                        solid.min.z + size.z * step(k),
                    );
                    let world = solid.to_world(local);
                    if near.iter().any(|candidate| candidate.contains(world)) {
                        hits += 1;
                    }
                }
            }
        }
        inside += volume * f64::from(hits) / f64::from(INSIDE_SAMPLES.pow(3));
    }
    if total <= 0.0 {
        return 0;
    }
    thousandths(inside / total)
}

/// The room `side` takes, in litres.
fn litres(side: &[&Solid<'_>]) -> u64 {
    let units: f64 = side.iter().map(|solid| solid.volume()).sum();
    whole_litres(units / UNITS_PER_LITRE)
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn thousandths(share: f64) -> u32 {
    // Bounded before converting.
    (share * 1000.0).round().clamp(0.0, 1000.0) as u32
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn whole_litres(litres: f64) -> u64 {
    // Bounded before converting; f64 holds every litre count of a world.
    litres.round().clamp(0.0, 1.0e15) as u64
}

#[allow(clippy::cast_possible_truncation)]
fn whole(coordinate: f64) -> i32 {
    // Bounded before converting.
    coordinate
        .round()
        .clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32
}

impl Solid<'_> {
    /// Whether the boxes around the two turned objects come within
    /// `margin` of each other.
    fn boxes_meet_within(&self, other: &Self, margin: f64) -> bool {
        let grown = |corner: Vector, by: f64| corner.map(|value| value + by);
        grown(self.world_min, -margin).at_most(other.world_max)
            && grown(other.world_min, -margin).at_most(self.world_max)
    }

    /// Whether the point lies in the object's box grown by `margin` on
    /// every side.
    fn contains_within(&self, world: Vector, margin: f64) -> bool {
        let local = self.to_local(world);
        let low = self.min.map(|value| value - margin);
        let high = self.max.map(|value| value + margin);
        local.at_least(low) && high.at_least(local)
    }

    /// Whether the two turned boxes come within `margin` of each other,
    /// measured at points spread over and through each box.
    fn near(&self, other: &Self, margin: f64) -> bool {
        self.boxes_meet_within(other, margin)
            && (self.samples_near(other, margin) || other.samples_near(self, margin))
    }

    fn samples_near(&self, other: &Self, margin: f64) -> bool {
        let size = self.max.minus(self.min);
        let last = f64::from(NEAR_SAMPLES - 1);
        let step = |index: u32| f64::from(index) / last;
        for i in 0..NEAR_SAMPLES {
            for j in 0..NEAR_SAMPLES {
                for k in 0..NEAR_SAMPLES {
                    let local = Vector::new(
                        self.min.x + size.x * step(i),
                        self.min.y + size.y * step(j),
                        self.min.z + size.z * step(k),
                    );
                    if other.contains_within(self.to_world(local), margin) {
                        return true;
                    }
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use mallet_core::facts::PluginFacts;
    use mallet_core::facts::objects::{
        BaseKind, BaseObject, Bounds, ObjectFacts, PlaceName, PlacedObject,
    };
    use mallet_core::{Facts, GamePath, NoModSource, SpotLocation};

    use super::*;
    use crate::Context;

    /// A plank of a bridge or a forge: 200 units long, 100 wide, 20 thick.
    const PLANK: u32 = 0x10;
    /// Another mod's plank of the same size, with a model of its own.
    const BOARD: u32 = 0x11;
    /// A plank with the same model as [`PLANK`], but a base object of
    /// its own.
    const PLANK_TWIN: u32 = 0x12;
    /// A rock.
    const ROCK: u32 = 0x13;

    fn id(plugin: &str, local_id: u32) -> RecordId {
        RecordId {
            plugin: plugin.to_lowercase(),
            local_id,
        }
    }

    fn base(local_id: u32, model: &str, natural: bool) -> BaseObject {
        BaseObject {
            id: id("Skyrim.esm", local_id),
            editor_id: Some(format!("Base{local_id:X}")),
            name: None,
            model: Some(model.to_owned()),
            kind: BaseKind::Static,
            solid: true,
            natural,
            bounds: Bounds {
                min: [-100, -50, 0],
                max: [100, 50, 20],
            },
        }
    }

    fn world() -> RecordId {
        id("Skyrim.esm", 0x3C)
    }

    fn placed(plugin: &str, local_id: u32, base: u32, position: [i32; 3]) -> PlacedObject {
        PlacedObject {
            id: id(plugin, local_id),
            plugin: plugin.to_owned(),
            base: id("Skyrim.esm", base),
            place: Place::Exterior {
                worldspace: world(),
            },
            position,
            rotation: [0; 3],
            scale: 1000,
            persistent: false,
            large: false,
            moved: None,
        }
    }

    fn plugin(name: &str) -> PluginFacts {
        PluginFacts {
            name: name.to_owned(),
            path: GamePath::new("data").join(name),
            masters: if name == "Skyrim.esm" {
                Vec::new()
            } else {
                vec!["Skyrim.esm".to_owned()]
            },
            npcs: Vec::new(),
        }
    }

    fn passages(placed: Vec<PlacedObject>) -> Vec<MapPassage> {
        let facts = Facts {
            plugins: vec![
                plugin("Skyrim.esm"),
                plugin("Forge.esp"),
                plugin("Temple.esp"),
            ],
            objects: ObjectFacts {
                placed,
                bases: vec![
                    base(PLANK, "meshes/plank.nif", false),
                    base(BOARD, "meshes/board.nif", false),
                    base(PLANK_TWIN, "meshes/plank.nif", false),
                    base(ROCK, "meshes/rock.nif", true),
                ],
                place_names: vec![PlaceName {
                    id: world(),
                    editor_id: "Tamriel".to_owned(),
                }],
            },
            ..Facts::default()
        };
        Context::new(&facts, &NoModSource)
            .collisions()
            .passages
            .clone()
    }

    /// A row of planks of one plugin along x, touching end to end.
    fn row(
        plugin: &str,
        first_id: u32,
        base: u32,
        start: [i32; 3],
        count: i32,
    ) -> Vec<PlacedObject> {
        (0..count)
            .zip(first_id..)
            .map(|(at, local_id)| {
                let [x, y, z] = start;
                placed(plugin, local_id, base, [x + 200 * at, y, z])
            })
            .collect()
    }

    fn ids(parts: &[SpotPart]) -> Vec<u32> {
        parts.iter().map(|part| part.object.local_id).collect()
    }

    #[test]
    fn the_same_forge_built_twice_is_one_passage_of_twins() {
        // Both mods build the same six planks, a few units apart; the
        // forge's mod goes on with five more where the temple has nothing.
        let mut objects = row("Forge.esp", 0x800, PLANK, [0, 0, 0], 11);
        objects.extend(row("Temple.esp", 0x900, PLANK_TWIN, [3, 2, 0], 6));
        let found = passages(objects);

        let [passage] = found.as_slice() else {
            panic!("expected one passage, got {found:?}");
        };
        assert_eq!(
            passage.plugins,
            ["Forge.esp".to_owned(), "Temple.esp".to_owned()]
        );
        assert_eq!(passage.severity, PassageSeverity::Doubled);
        assert_eq!(passage.duplicates, 6);
        // The forge's own row joins as far as the temple's reaches: its
        // seventh plank touches the temple's last one and comes along, the
        // ones beyond do not.
        assert_eq!(ids(&passage.parts[0]), (0x800..=0x806).collect::<Vec<_>>());
        assert_eq!(ids(&passage.parts[1]), (0x900..=0x905).collect::<Vec<_>>());
        assert!(passage.inside[1] > 900, "{:?}", passage.inside);
        assert_eq!(
            passage.location,
            SpotLocation::Exterior {
                world: "Tamriel".to_owned(),
                x: 0,
                y: 0
            }
        );
        assert!(passage.max[0] <= 200 * 7);
    }

    #[test]
    fn a_bridge_built_differently_ends_where_the_two_part_ways() {
        // The first mod's bridge runs on into its road far beyond the
        // river; the second mod's bridge stands in the first one's with
        // boards of its own, and its road turns away north.
        let mut objects = row("Forge.esp", 0x800, PLANK, [0, 0, 0], 30);
        objects.extend(row("Temple.esp", 0x900, BOARD, [20, 0, 5], 8));
        objects.extend((0..10).zip(0xA00..).map(|(at, local_id)| {
            placed("Temple.esp", local_id, BOARD, [1500, 100 + 100 * at, 200])
        }));
        let found = passages(objects);

        let [passage] = found.as_slice() else {
            panic!("expected one passage, got {found:?}");
        };
        assert_eq!(passage.duplicates, 0);
        assert_eq!(passage.severity, PassageSeverity::Inside);
        let first = ids(&passage.parts[0]);
        assert!(first.len() <= 9, "{first:x?}");
        assert!(
            first.contains(&0x800) && first.contains(&0x807),
            "{first:x?}"
        );
        assert_eq!(ids(&passage.parts[1]), (0x900..=0x907).collect::<Vec<_>>());
        assert_eq!(passage.spots, 8);
    }

    #[test]
    fn a_single_spot_is_no_passage() {
        let found = passages(vec![
            placed("Forge.esp", 0x800, PLANK, [0, 0, 0]),
            placed("Temple.esp", 0x900, BOARD, [20, 0, 5]),
        ]);

        assert_eq!(found, Vec::new());
    }

    #[test]
    fn the_same_two_mods_in_two_places_make_two_passages() {
        let mut objects = row("Forge.esp", 0x800, PLANK, [0, 0, 0], 4);
        objects.extend(row("Temple.esp", 0x900, PLANK_TWIN, [0, 0, 0], 4));
        objects.extend(row("Forge.esp", 0x810, PLANK, [40_000, 0, 0], 3));
        objects.extend(row("Temple.esp", 0x910, BOARD, [40_020, 0, 5], 3));
        let found = passages(objects);

        let [doubled, inside] = found.as_slice() else {
            panic!("expected two passages, got {found:?}");
        };
        // Twins rate highest.
        assert_eq!(doubled.severity, PassageSeverity::Doubled);
        assert_eq!(ids(&doubled.parts[0]), vec![0x800, 0x801, 0x802, 0x803]);
        assert_eq!(inside.severity, PassageSeverity::Inside);
        assert_eq!(ids(&inside.parts[1]), vec![0x910, 0x911, 0x912]);
    }

    #[test]
    fn a_passage_does_not_grow_through_rocks() {
        let mut objects = row("Forge.esp", 0x800, PLANK, [0, 0, 0], 3);
        objects.extend(row("Temple.esp", 0x900, PLANK_TWIN, [0, 0, 0], 3));
        // A rock of the forge's mod touches its last plank and lies near
        // the temple's: it stays out.
        objects.push(placed("Forge.esp", 0x8FF, ROCK, [600, 0, 0]));
        let found = passages(objects);

        let [passage] = found.as_slice() else {
            panic!("expected one passage, got {found:?}");
        };
        assert_eq!(ids(&passage.parts[0]), vec![0x800, 0x801, 0x802]);
    }
}
