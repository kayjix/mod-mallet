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

/// A part rests on another when a point of its underside lies this close to
/// the other's box, in game units (about 23 cm) ...
const REST: f64 = 16.0;

/// ... and its middle stands above the other's by at least this share of
/// the other's height: a roof on a wall, not a wall beside a wall.
const ABOVE: f64 = 0.25;

/// A part is embedded in another when at least this share of it lies inside.
const EMBEDDED: f64 = 0.25;

/// A part that rests on or in another is at most this many times as long as
/// it, unless it is shorter than [`LARGE`]: a road or a slab of ground of
/// the same mod touches half a town (as the magic wand, ADR 0055).
const LARGER: f64 = 4.0;

/// About 7 m, in game units.
const LARGE: f64 = 490.0;

/// A part held by a single other part, set into it, is at most this many
/// times as long as it; a roof on two walls is part of them ...
const ALONE: f64 = 2.0;

/// ... and resting on a single part (not set into it), at most this share
/// of its length: a barrel on a walkway, not a house.
const ALONE_ON: f64 = 0.5;

/// A part at least [`LARGE`] long and this many times as long as another
/// carries none of it: a cliff or a slab under barrels and stalls is the
/// ground they stand on, not the structure they belong to.
const DWARF: f64 = 8.0;

/// Nothing rests on a flat part this wide (a floor, a slab of ground, a
/// road): it carries the town, not one structure ...
const GROUND_WIDE: f64 = 490.0;

/// ... flat being at most this share of its width tall ...
const GROUND_FLAT: f64 = 1.0 / 6.0;

/// ... and lying at the bottom of the passage, within this height of its
/// lowest point, in game units (about 1.5 m). A flat roof up high carries
/// what sits on it.
const GROUND_LEVEL: f64 = 105.0;

/// What a passage takes along lies within this distance of its parts' box,
/// in game units (about 6 m).
const ATTACH_MARGIN: f64 = 420.0;

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
            let component = attach(solids, &grid, [first, second], place, component);
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
                        && !other.ground_under(from)
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

/// A component with what is structurally part of each side's version: the
/// same mod's parts that rest on or are embedded in what the side already
/// takes, link by link (a roof on the walls, a trim in the roof). The
/// other side need not come near them: when a version goes, nothing of it
/// is to hang in the air. Parts beside (the next building's wall), below (a
/// floor, the ground) or far from the passage are not taken.
fn attach(
    solids: &[Solid<'_>],
    grid: &Grid<'_>,
    owners: [&str; 2],
    place: &Place,
    component: [Vec<usize>; 2],
) -> [Vec<usize>; 2] {
    let mut low = Vector::new(f64::MAX, f64::MAX, f64::MAX);
    let mut high = Vector::new(f64::MIN, f64::MIN, f64::MIN);
    for solid in component.iter().flatten().filter_map(|&at| solids.get(at)) {
        low = low.min(solid.world_min);
        high = high.max(solid.world_max);
    }
    let near_passage = |solid: &Solid<'_>| {
        let centre = solid.centre();
        centre.x >= low.x - ATTACH_MARGIN
            && centre.x <= high.x + ATTACH_MARGIN
            && centre.y >= low.y - ATTACH_MARGIN
            && centre.y <= high.y + ATTACH_MARGIN
    };
    let mut taken: BTreeSet<usize> = component.iter().flatten().copied().collect();
    let mut sides = component;
    for (side, owner) in sides.iter_mut().zip(owners) {
        // Until nothing more comes along: a part goes with the side when
        // more than half of what holds it goes too, so that a roof shared
        // with the next building stays on that building; when exactly half
        // goes, only if one of those holds its middle, so that a bridge set
        // into a quay at one end and resting on a pier at the other stays.
        loop {
            let mut candidates: BTreeSet<usize> = BTreeSet::new();
            for holder in side.iter().filter_map(|&at| solids.get(at)) {
                candidates.extend(grid.around(holder, owner, solids));
            }
            let mut grew = false;
            for candidate in candidates {
                if side.len() >= MOST || taken.contains(&candidate) {
                    continue;
                }
                let Some(part) = solids.get(candidate) else {
                    continue;
                };
                if &part.object.place != place || !near_passage(part) {
                    continue;
                }
                let holders: Vec<usize> = grid
                    .around(part, owner, solids)
                    .into_iter()
                    .filter(|&other| {
                        other != candidate
                            && solids.get(other).is_some_and(|holder| {
                                &holder.object.place == place
                                    && part.part_of(holder, low.z + GROUND_LEVEL)
                            })
                    })
                    .collect();
                let held: Vec<&Solid<'_>> = holders
                    .iter()
                    .filter(|at| taken.contains(at))
                    .filter_map(|&at| solids.get(at))
                    .collect();
                let longest = held
                    .iter()
                    .map(|holder| holder.longest_side())
                    .fold(0.0, f64::max);
                if !held.is_empty()
                    && (held.len() * 2 > holders.len()
                        || (held.len() * 2 == holders.len()
                            && held.iter().any(|holder| holder.under_middle_of(part))))
                    && part.longest_side() <= LARGE.max(LARGER * longest)
                    && (held.len() >= 2
                        || part.longest_side()
                            <= longest
                                * if held
                                    .iter()
                                    .any(|holder| part.share_in(holder, TOUCHING) >= EMBEDDED)
                                {
                                    ALONE
                                } else {
                                    ALONE_ON
                                })
                {
                    taken.insert(candidate);
                    side.push(candidate);
                    grew = true;
                }
            }
            if !grew {
                break;
            }
        }
        side.sort_unstable();
    }
    sides
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
    /// The middle of the turned box, in the world.
    fn centre(&self) -> Vector {
        self.to_world(self.min.plus(self.max).scaled(0.5))
    }

    /// Whether this part is wide and flat and lies under `part`: a floor, a
    /// slab of ground or a road it stands on, which a passage does not
    /// grow into, as it carries more than this one structure.
    fn ground_under(&self, part: &Self) -> bool {
        self.wide_and_flat() && self.world_max.z <= part.world_min.z + REST
    }

    /// Whether this part lies under (or around) the middle of `part`: the
    /// point straight below or above that middle, at this part's own height,
    /// is in its box or within [`REST`] of it.
    fn under_middle_of(&self, part: &Self) -> bool {
        let middle = part.centre();
        let level = self.centre();
        self.contains_within(Vector::new(middle.x, middle.y, level.z), REST)
    }

    /// Whether this part is so much larger than `part` that it is the
    /// ground `part` stands on rather than what it belongs to.
    fn dwarfs(&self, part: &Self) -> bool {
        let longest = self.longest_side();
        longest >= LARGE && longest >= DWARF * part.longest_side()
    }

    /// Whether the box is at least [`GROUND_WIDE`] wide and at most
    /// [`GROUND_FLAT`] of that tall.
    fn wide_and_flat(&self) -> bool {
        let size = self.world_max.minus(self.world_min);
        let wide = size.x.max(size.y);
        wide >= GROUND_WIDE && size.z <= wide * GROUND_FLAT
    }

    /// Whether this part is structurally part of `holder`: it is embedded
    /// in it, or it rests on it from above. A part beside it at the same
    /// height is neither, and nothing rests on a floor or the ground (flat
    /// parts no higher than `ground`).
    fn part_of(&self, holder: &Self, ground: f64) -> bool {
        if !self.boxes_meet_within(holder, TOUCHING) || holder.dwarfs(self) {
            return false;
        }
        self.share_in(holder, TOUCHING) >= EMBEDDED || self.rests_on(holder, ground)
    }

    /// How much of this part lies inside `other` grown by `margin`, from 0
    /// to 1, measured at points spread through its box.
    fn share_in(&self, other: &Self, margin: f64) -> f64 {
        let size = self.max.minus(self.min);
        let step = |index: u32| (f64::from(index) + 0.5) / f64::from(INSIDE_SAMPLES);
        let mut inside = 0u32;
        for i in 0..INSIDE_SAMPLES {
            for j in 0..INSIDE_SAMPLES {
                for k in 0..INSIDE_SAMPLES {
                    let local = Vector::new(
                        self.min.x + size.x * step(i),
                        self.min.y + size.y * step(j),
                        self.min.z + size.z * step(k),
                    );
                    if other.contains_within(self.to_world(local), margin) {
                        inside += 1;
                    }
                }
            }
        }
        f64::from(inside) / f64::from(INSIDE_SAMPLES.pow(3))
    }

    /// Whether this part rests on `holder`: a point of its underside lies on
    /// or in it, and its middle stands clearly higher. A wide flat holder
    /// lying no higher than `ground` (a floor, a slab of ground, a road)
    /// carries nothing.
    fn rests_on(&self, holder: &Self, ground: f64) -> bool {
        let holder_size = holder.world_max.minus(holder.world_min);
        if holder.wide_and_flat() && holder.world_min.z <= ground {
            return false;
        }
        if self.centre().z < holder.centre().z + ABOVE * holder_size.z {
            return false;
        }
        let size = self.max.minus(self.min);
        let last = f64::from(NEAR_SAMPLES - 1);
        let step = |index: u32| f64::from(index) / last;
        (0..NEAR_SAMPLES).any(|i| {
            (0..NEAR_SAMPLES).any(|j| {
                let local = Vector::new(
                    self.min.x + size.x * step(i),
                    self.min.y + size.y * step(j),
                    self.min.z,
                );
                holder.contains_within(self.to_world(local), REST)
            })
        })
    }

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
    /// A stable's wall along x: 6 m long, 3 m tall.
    const WALL: u32 = 0x20;
    /// A stable's wall along y: 4 m long.
    const SIDE: u32 = 0x21;
    /// A roof over the stable.
    const ROOF: u32 = 0x22;
    /// A trim set into the roof's edge.
    const TRIM: u32 = 0x23;
    /// A ridge piece sitting on the roof.
    const RIDGE: u32 = 0x24;
    /// A slab of ground under the stable and the house beside it.
    const FLOOR: u32 = 0x25;
    /// The other mod's lean-to along a wall, half as tall.
    const LEAN: u32 = 0x26;
    /// The other mod's lean-to along a side.
    const LEAN_SIDE: u32 = 0x27;
    /// A footbridge, 6 m long.
    const BRIDGE: u32 = 0x28;

    fn id(plugin: &str, local_id: u32) -> RecordId {
        RecordId {
            plugin: plugin.to_lowercase(),
            local_id,
        }
    }

    fn base(local_id: u32, model: &str, natural: bool) -> BaseObject {
        sized(local_id, model, natural, [-100, -50, 0], [100, 50, 20])
    }

    fn sized(
        local_id: u32,
        model: &str,
        natural: bool,
        min: [i32; 3],
        max: [i32; 3],
    ) -> BaseObject {
        BaseObject {
            id: id("Skyrim.esm", local_id),
            editor_id: Some(format!("Base{local_id:X}")),
            name: None,
            model: Some(model.to_owned()),
            kind: BaseKind::Static,
            solid: true,
            natural,
            bounds: Bounds { min, max },
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
                    sized(
                        WALL,
                        "meshes/wall.nif",
                        false,
                        [-300, -10, 0],
                        [300, 10, 300],
                    ),
                    sized(
                        SIDE,
                        "meshes/side.nif",
                        false,
                        [-10, -200, 0],
                        [10, 200, 300],
                    ),
                    sized(
                        ROOF,
                        "meshes/roof.nif",
                        false,
                        [-320, -220, 0],
                        [320, 220, 40],
                    ),
                    sized(
                        TRIM,
                        "meshes/trim.nif",
                        false,
                        [-300, -15, -10],
                        [300, 15, 15],
                    ),
                    sized(
                        RIDGE,
                        "meshes/ridge.nif",
                        false,
                        [-50, -50, 0],
                        [50, 50, 60],
                    ),
                    sized(
                        FLOOR,
                        "meshes/floor.nif",
                        false,
                        [-700, -300, -20],
                        [700, 300, 0],
                    ),
                    sized(
                        LEAN,
                        "meshes/lean.nif",
                        false,
                        [-300, -10, 0],
                        [300, 10, 150],
                    ),
                    sized(
                        BRIDGE,
                        "meshes/bridge.nif",
                        false,
                        [-300, -50, 0],
                        [300, 50, 30],
                    ),
                    sized(
                        LEAN_SIDE,
                        "meshes/leanside.nif",
                        false,
                        [-10, -200, 0],
                        [10, 200, 150],
                    ),
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

    /// A stable of the first mod at the origin: four walls, a roof on them,
    /// a trim set into the roof's edge and a ridge piece on top, on a slab
    /// of ground; a house of the same mod 3 m to the east on the same slab;
    /// and the second mod's lean-to set into two of the stable's walls,
    /// half as tall, so that it comes nowhere near the roof.
    fn stable() -> Vec<PlacedObject> {
        let first = "Forge.esp";
        let mut objects = vec![
            placed(first, 0x800, WALL, [0, -200, 0]),
            placed(first, 0x801, WALL, [0, 200, 0]),
            placed(first, 0x802, SIDE, [-300, 0, 0]),
            placed(first, 0x803, SIDE, [300, 0, 0]),
            placed(first, 0x804, ROOF, [0, 0, 300]),
            placed(first, 0x805, TRIM, [0, -200, 320]),
            placed(first, 0x806, RIDGE, [0, 0, 340]),
            placed(first, 0x807, FLOOR, [400, 0, 0]),
            // The house beside it, 3 m away.
            placed(first, 0x810, WALL, [810, -200, 0]),
            placed(first, 0x811, WALL, [810, 200, 0]),
            placed(first, 0x812, SIDE, [510, 0, 0]),
            placed(first, 0x813, SIDE, [1110, 0, 0]),
            placed(first, 0x814, ROOF, [810, 0, 300]),
        ];
        objects.extend([
            placed("Temple.esp", 0x900, LEAN, [0, -190, 0]),
            placed("Temple.esp", 0x901, LEAN_SIDE, [-290, 0, 0]),
        ]);
        objects
    }

    #[test]
    fn hiding_a_stable_takes_its_roof_along_but_not_the_house_beside_it() {
        let found = passages(stable());

        let [passage] = found.as_slice() else {
            panic!("expected one passage, got {found:?}");
        };
        // The four walls collide or come near the lean-to; the roof, the
        // trim in it and the ridge on it rest on them and come along. The
        // slab of ground below and the house beside it stay.
        assert_eq!(
            ids(&passage.parts[0]),
            vec![0x800, 0x801, 0x802, 0x803, 0x804, 0x805, 0x806]
        );
        assert_eq!(ids(&passage.parts[1]), vec![0x900, 0x901]);
        assert_eq!(passage.spots, 2);
    }

    #[test]
    fn a_house_that_touches_the_stable_stays_unless_it_rests_on_it() {
        // The house moved up against the stable: its side wall stands
        // beside the stable's, at the same height. Beside is not part of.
        let mut objects: Vec<PlacedObject> = stable()
            .into_iter()
            .map(|mut object| {
                if (0x810..=0x814).contains(&object.id.local_id) {
                    object.position[0] -= 180;
                }
                object
            })
            .collect();
        objects.retain(|object| object.id.local_id != 0x812);
        let found = passages(objects);

        let [passage] = found.as_slice() else {
            panic!("expected one passage, got {found:?}");
        };
        assert!(
            ids(&passage.parts[0]).iter().all(|id| *id < 0x810),
            "{:x?}",
            ids(&passage.parts[0])
        );
    }

    #[test]
    fn a_bridge_from_the_stable_to_a_pier_that_stays_stays_too() {
        // A footbridge of the first mod is set into the stable's roof at one
        // end and rests on a pier of its own at the other, as a bridge set
        // into a quay. Half of what holds it goes with the stable, but not
        // what holds its middle: it stays, as does the pier.
        let mut objects: Vec<PlacedObject> = stable()
            .into_iter()
            .filter(|object| !(0x810..=0x814).contains(&object.id.local_id))
            .collect();
        objects.extend([
            placed("Forge.esp", 0x820, BRIDGE, [450, 0, 320]),
            placed("Forge.esp", 0x821, SIDE, [750, 0, 20]),
        ]);
        let found = passages(objects);

        let [passage] = found.as_slice() else {
            panic!("expected one passage, got {found:?}");
        };
        assert_eq!(
            ids(&passage.parts[0]),
            vec![0x800, 0x801, 0x802, 0x803, 0x804, 0x805, 0x806]
        );
    }

    #[test]
    fn a_bridge_resting_on_two_walls_of_the_stable_goes_with_it() {
        // The same footbridge across the stable, from wall to wall: all that
        // holds it goes, so it goes too.
        let mut objects: Vec<PlacedObject> = stable()
            .into_iter()
            .filter(|object| {
                !(0x810..=0x814).contains(&object.id.local_id) && object.id.local_id != 0x804
            })
            .collect();
        objects.push(placed("Forge.esp", 0x820, BRIDGE, [0, 0, 250]));
        let found = passages(objects);

        let [passage] = found.as_slice() else {
            panic!("expected one passage, got {found:?}");
        };
        assert!(
            ids(&passage.parts[0]).contains(&0x820),
            "{:x?}",
            ids(&passage.parts[0])
        );
    }
}
