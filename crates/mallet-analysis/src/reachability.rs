//! Whether characters still reach where they need to go once the paths
//! over hidden objects change (ADR 0059).
//!
//! The game's characters find their way over the triangles of navigation
//! meshes: from triangle to triangle within a mesh, along links to the
//! neighbouring meshes, and through doors that lead into another cell.
//! What they walk to are destinations: doors, furniture, idle markers, map
//! markers, objects a quest or a routine names. Each is reached on the
//! triangle nearest to it.
//!
//! [`reachability`] compares two versions of the paths, such as those the
//! game uses and the same with the triangles over hidden objects cut out
//! (ADR 0052) or repaired with new ones. A destination is reachable when a
//! way leads there from where the paths lead on into the rest of the
//! world: a link to a mesh that was not read, or a door whose other side
//! was not read. Of the destinations near what changed, it tells which
//! lose what they stood on, which can no longer be reached, and which only
//! by a much longer way, with the ways before and after.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, HashSet};

use mallet_core::facts::RecordId;
use mallet_core::facts::navmeshes::{Beyond, NavmeshShape, PlacedDestination};
use mallet_core::facts::objects::Place;

use crate::walkways::Carried;

/// How far a destination may stand from the triangle it is reached on, in
/// game units: a door's frame or a bed stands beside the paths, not on
/// them. About three and a half metres.
pub const SNAP: f64 = 256.0;

/// How much farther from the paths a destination may stand after the
/// change than before before what it stood on counts as gone: the
/// triangle it was reached on is gone, and the nearest one left is
/// another, farther away, such as the ground below a deck.
pub const GONE_SLACK: f64 = 64.0;

/// A way counts as much longer when it is at least this many times as
/// long as before...
pub const LONGER_RATIO: f64 = 1.5;

/// ...and longer by at least this many game units, about 15 metres.
pub const LONGER_BY: f64 = 1024.0;

/// Destinations farther than this from every triangle that changed are
/// not checked: about one cell.
pub const NEAR_CHANGE: f64 = 4096.0;

/// The most destinations checked, the nearest to what changed first.
pub const MOST_CHECKED: usize = 48;

/// A way characters walk.
#[derive(Debug, Clone, PartialEq)]
pub struct Route {
    /// Its length, in game units.
    pub length: f64,
    /// Its points from start to end, in the coordinates of the worldspace
    /// or the cell.
    pub points: Vec<[f64; 3]>,
}

/// What happens to the way to a destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// What characters stood on to use it is gone.
    Gone,
    /// No way leads there any more.
    CutOff,
    /// A way still leads there, but from the destination `from`, by its
    /// place in the list given, a much longer one, or one only through the
    /// places around that were not read.
    Longer {
        /// Where the way is measured from.
        from: usize,
    },
}

/// One destination whose way changes.
#[derive(Debug, Clone, PartialEq)]
pub struct Reach {
    /// The destination, by its place in the list given.
    pub destination: usize,
    /// What happens to its way.
    pub outcome: Outcome,
    /// The way before: for a destination gone or cut off, from where the
    /// paths lead on into the rest of the world; for a longer one, from
    /// the other destination.
    pub before: Option<Route>,
    /// The way after, if there is one.
    pub after: Option<Route>,
}

/// What a change of the paths does to the ways to the destinations near
/// it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Reachability {
    /// How many destinations were checked: those near what changed that
    /// could be reached before.
    pub checked: usize,
    /// The destinations whose way changes: gone first, then cut off, then
    /// longer, each the nearest to what changed first.
    pub reaches: Vec<Reach>,
}

/// What changing the paths `before` into `after` does to the ways to
/// `destinations`. Only versions of meshes the game uses count, in both;
/// a mesh in `after` replaces the one with the same identity in `before`.
pub fn reachability(
    before: &[NavmeshShape],
    after: &[NavmeshShape],
    destinations: &[PlacedDestination],
) -> Reachability {
    let changes = changed_points(before, after);
    if changes.is_empty() {
        return Reachability::default();
    }
    let old = Graph::new(before, destinations);
    let new = Graph::new(after, destinations);
    let mut old_out = old.exits.clone();
    let mut new_out = new.exits.clone();
    if !old_out.iter().any(|exit| *exit) {
        // Nothing leads out, as in a cell without doors: characters come
        // from the largest part that stays.
        if let Some(key) = old.largest_kept(&new) {
            set(&mut old_out, old.node(&key));
            set(&mut new_out, new.node(&key));
        }
    }
    let from_old = old.paths(&sources(&old_out), &[]);
    let from_new = new.paths(&sources(&new_out), &[]);
    let placed = place(destinations, &changes, &old, &new, &from_old);

    let mut gone = Vec::new();
    let mut cut_off = Vec::new();
    let mut still: Vec<&Placed> = Vec::new();
    for entry in &placed {
        let Some(destination) = destinations.get(entry.index) else {
            continue;
        };
        let end = position(destination);
        let before = from_old.route(&old, entry.old, None, Some(end));
        match entry.new {
            _ if entry.gone => gone.push(Reach {
                destination: entry.index,
                outcome: Outcome::Gone,
                before,
                after: None,
            }),
            Some(node) if from_new.reached(node) => still.push(entry),
            _ => cut_off.push(Reach {
                destination: entry.index,
                outcome: Outcome::CutOff,
                before,
                after: None,
            }),
        }
    }
    let longer = longer_ways(&old, &new, &still, destinations);

    let checked = placed.len();
    let mut reaches = gone;
    reaches.extend(cut_off);
    reaches.extend(longer);
    Reachability { checked, reaches }
}

/// How far from a spot the nearest ground characters reach is looked for,
/// in game units: about 30 metres.
pub const GROUND_WITHIN: f64 = 2048.0;

/// Whether characters reach a spot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpotState {
    /// A path characters reach leads to it.
    Reached,
    /// It stands on paths no way leads to from the rest of the world.
    CutOff,
    /// No path is near enough to use it from.
    OffPaths,
}

/// Whether characters reach one spot, and the nearest ground they reach.
#[derive(Debug, Clone, PartialEq)]
pub struct SpotReach {
    /// Whether they reach it.
    pub state: SpotState,
    /// How far it stands from the nearest path characters reach, in game
    /// units, if one lies within [`GROUND_WITHIN`].
    pub off: Option<f64>,
    /// The nearest point on paths characters reach, within
    /// [`GROUND_WITHIN`].
    pub ground: Option<[f64; 3]>,
}

/// Whether characters reach each of `spots`, over the paths `shapes` of
/// one worldspace or interior cell, with `destinations` for the doors
/// that lead through. A spot is reached where it stands within [`SNAP`] of
/// a triangle a way leads to from where the paths lead on into the rest
/// of the world, or, where nothing leads out, from their largest part.
pub fn reach_spots(
    shapes: &[NavmeshShape],
    destinations: &[PlacedDestination],
    spots: &[[f64; 3]],
) -> Vec<SpotReach> {
    let graph = Graph::new(shapes, destinations);
    let mut out = graph.exits.clone();
    if !out.iter().any(|exit| *exit)
        && let Some(key) = graph.largest_kept(&graph)
    {
        set(&mut out, graph.node(&key));
    }
    let paths = graph.paths(&sources(&out), &[]);
    spots
        .iter()
        .map(|&at| {
            let ground = graph.nearest_reached(&paths, at);
            let off = ground.map(|point| distance(point, at));
            let state = match graph.snap_anywhere(at) {
                Some((node, _)) if paths.reached(node) => SpotState::Reached,
                Some(_) => SpotState::CutOff,
                None => SpotState::OffPaths,
            };
            SpotReach { state, off, ground }
        })
        .collect()
}

/// The destinations near what changed that could be reached before, the
/// nearest first, with where each lies on the paths before and after.
fn place(
    destinations: &[PlacedDestination],
    changes: &[(&Place, [f64; 3])],
    old: &Graph<'_>,
    new: &Graph<'_>,
    from_old: &Paths,
) -> Vec<Placed> {
    // Those in other cells, such as behind a door, come last.
    let mut near: Vec<(usize, f64)> = destinations
        .iter()
        .enumerate()
        .filter_map(|(index, destination)| {
            let at = position(destination);
            let distance = changes
                .iter()
                .filter(|(place, _)| *place == &destination.place)
                .map(|(_, point)| flat_distance(*point, at))
                .reduce(f64::min)
                .unwrap_or(NEAR_CHANGE);
            (distance <= NEAR_CHANGE).then_some((index, distance))
        })
        .collect();
    near.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));

    let mut placed: Vec<Placed> = Vec::new();
    for (index, _) in near {
        let Some(destination) = destinations.get(index) else {
            continue;
        };
        let at = position(destination);
        let Some((old_node, old_distance)) = old.snap(&destination.place, at) else {
            continue;
        };
        if !from_old.reached(old_node) {
            continue;
        }
        let new_snap = new.snap(&destination.place, at);
        let gone = new_snap.is_none_or(|(_, distance)| distance > old_distance + GONE_SLACK);
        placed.push(Placed {
            index,
            old: old_node,
            new: new_snap.map(|(node, _)| node),
            gone,
        });
        if placed.len() >= MOST_CHECKED {
            break;
        }
    }
    placed
}

/// Among the destinations `still` reachable, those reached from another
/// one only by a much longer way, each pair once.
fn longer_ways(
    old: &Graph<'_>,
    new: &Graph<'_>,
    still: &[&Placed],
    destinations: &[PlacedDestination],
) -> Vec<Reach> {
    let mut longer = Vec::new();
    let old_targets: Vec<usize> = still.iter().map(|entry| entry.old).collect();
    let new_targets: Vec<usize> = still.iter().filter_map(|entry| entry.new).collect();
    let mut reported: HashSet<(usize, usize)> = HashSet::new();
    for entry in still {
        let Some(new_node) = entry.new else {
            continue;
        };
        let old_paths = old.paths(&[entry.old], &old_targets);
        let new_paths = new.paths(&[new_node], &new_targets);
        // The other destination whose way grew the most; a way that now
        // leads only through the places around counts as the most.
        let mut worst: Option<(usize, f64, f64)> = None;
        for other in still {
            if other.index == entry.index {
                continue;
            }
            let Some(other_new) = other.new else {
                continue;
            };
            let Some(was) = old_paths.distance(other.old) else {
                continue;
            };
            let grown = match new_paths.distance(other_new) {
                Some(now) if now >= was * LONGER_RATIO && now - was >= LONGER_BY => now - was,
                Some(_) => continue,
                None => f64::INFINITY,
            };
            let better = worst.is_none_or(|(_, best, best_was)| {
                grown
                    .total_cmp(&best)
                    .then(best_was.total_cmp(&was))
                    .is_gt()
            });
            if better {
                worst = Some((other.index, grown, was));
            }
        }
        let Some((from, _, _)) = worst else {
            continue;
        };
        let pair = (entry.index.min(from), entry.index.max(from));
        if !reported.insert(pair) {
            continue;
        }
        let (Some(origin), Some(destination)) = (
            still.iter().find(|other| other.index == from),
            destinations.get(entry.index),
        ) else {
            continue;
        };
        let start = destinations.get(from).map(position);
        let end = Some(position(destination));
        let before = old_paths.route(old, origin.old, end, start).map(reverse);
        let after = origin
            .new
            .and_then(|node| new_paths.route(new, node, end, start))
            .map(reverse);
        longer.push(Reach {
            destination: entry.index,
            outcome: Outcome::Longer { from },
            before,
            after,
        });
    }
    longer
}

/// The versions the game uses of the meshes in `navmeshes`, with the
/// triangles in `carried` taken out of the paths as ADR 0052 cuts them:
/// none of their edges leads anywhere, and no triangle of these meshes
/// leads onto them. Carried triangles of versions the game does not use
/// are ignored.
pub fn cut(navmeshes: &[NavmeshShape], carried: &[Carried]) -> Vec<NavmeshShape> {
    let mut cuts: BTreeMap<&RecordId, BTreeSet<u32>> = BTreeMap::new();
    for carried in carried {
        if let Some(shape) = navmeshes.get(carried.navmesh).filter(|shape| shape.used) {
            cuts.entry(&shape.id)
                .or_default()
                .extend(carried.triangles.iter().copied());
        }
    }
    navmeshes
        .iter()
        .filter(|shape| shape.used)
        .map(|shape| {
            let own = cuts.get(&shape.id);
            let is_cut = |navmesh: &RecordId, triangle: &u32| {
                cuts.get(navmesh).is_some_and(|cut| cut.contains(triangle))
            };
            let mut shape = shape.clone();
            for (number, edges) in (0u32..).zip(shape.edges.iter_mut()) {
                if own.is_some_and(|cut| cut.contains(&number)) {
                    *edges = [Beyond::Nothing, Beyond::Nothing, Beyond::Nothing];
                    continue;
                }
                for edge in edges.iter_mut() {
                    let leads_onto_cut = match edge {
                        Beyond::Nothing => false,
                        Beyond::Triangle(other) => own.is_some_and(|cut| cut.contains(other)),
                        Beyond::Link { navmesh, triangle } => is_cut(navmesh, triangle),
                    };
                    if leads_onto_cut {
                        *edge = Beyond::Nothing;
                    }
                }
            }
            shape
        })
        .collect()
}

/// The middles of the triangles that differ between `before` and `after`,
/// with the place they lie in.
fn changed_points<'a>(
    before: &'a [NavmeshShape],
    after: &'a [NavmeshShape],
) -> Vec<(&'a Place, [f64; 3])> {
    let used = |shapes: &'a [NavmeshShape]| -> BTreeMap<&'a RecordId, &'a NavmeshShape> {
        shapes
            .iter()
            .filter(|shape| shape.used)
            .map(|shape| (&shape.id, shape))
            .collect()
    };
    let (old, new) = (used(before), used(after));
    let mut points = Vec::new();
    for (id, shape) in &old {
        let Some(other) = new.get(id) else {
            push_all(&mut points, shape, 0);
            continue;
        };
        let same_corners = shape.vertices == other.vertices;
        for index in 0..shape.triangles.len() {
            let same = same_corners
                && shape.triangles.get(index) == other.triangles.get(index)
                && shape.edges.get(index) == other.edges.get(index);
            if !same && let Some(middle) = middle_of(shape, index) {
                points.push((&shape.place, middle));
            }
        }
        push_all(&mut points, other, shape.triangles.len());
    }
    for (id, shape) in &new {
        if !old.contains_key(id) {
            push_all(&mut points, shape, 0);
        }
    }
    points
}

/// Adds the middles of the triangles of `shape` from the number `from` on.
fn push_all<'a>(points: &mut Vec<(&'a Place, [f64; 3])>, shape: &'a NavmeshShape, from: usize) {
    for index in from..shape.triangles.len() {
        if let Some(middle) = middle_of(shape, index) {
            points.push((&shape.place, middle));
        }
    }
}

/// A destination near what changed, where it lies on the paths.
struct Placed {
    /// The destination, by its place in the list given.
    index: usize,
    /// Its triangle before.
    old: usize,
    /// Its triangle after, if one is near.
    new: Option<usize>,
    /// Whether what it stood on is gone.
    gone: bool,
}

/// A triangle, by its mesh and its number.
type Key<'a> = (&'a RecordId, u32);

/// The triangles of the versions the game uses, as numbered nodes, and the
/// ways between them.
struct Graph<'a> {
    /// For each node, its mesh, its triangle and its corners.
    nodes: Vec<(&'a NavmeshShape, u32, [[f64; 3]; 3])>,
    /// The meshes by identity, with the number of their first node.
    first: HashMap<&'a RecordId, (usize, &'a NavmeshShape)>,
    /// For each node, the nodes characters can step to and how far.
    ways: Vec<Vec<(usize, f64)>>,
    /// For each node, whether the paths lead on from it into the rest of
    /// the world.
    exits: Vec<bool>,
}

impl<'a> Graph<'a> {
    fn new(shapes: &'a [NavmeshShape], destinations: &[PlacedDestination]) -> Self {
        let mut first: HashMap<&RecordId, (usize, &NavmeshShape)> = HashMap::new();
        let mut order: Vec<&NavmeshShape> = Vec::new();
        for shape in shapes.iter().filter(|shape| shape.used) {
            // A later version of the same mesh replaces an earlier one.
            order.retain(|known| known.id != shape.id);
            order.push(shape);
        }
        let mut nodes = Vec::new();
        for shape in order {
            first.insert(&shape.id, (nodes.len(), shape));
            for (number, triangle) in (0u32..).zip(&shape.triangles) {
                let corner = |index: u32| -> Option<[f64; 3]> {
                    let [x, y, z] = *shape.vertices.get(usize::try_from(index).ok()?)?;
                    Some([f64::from(x), f64::from(y), f64::from(z)])
                };
                let corners = match (
                    corner(triangle[0]),
                    corner(triangle[1]),
                    corner(triangle[2]),
                ) {
                    (Some(a), Some(b), Some(c)) => [a, b, c],
                    // A triangle with a corner the mesh lacks: nobody
                    // stands on it, but it keeps its number.
                    _ => [[f64::NAN; 3]; 3],
                };
                nodes.push((shape, number, corners));
            }
        }
        let mut graph = Self {
            ways: vec![Vec::new(); nodes.len()],
            exits: vec![false; nodes.len()],
            nodes,
            first,
        };
        let mut doors: HashMap<&RecordId, Vec<usize>> = HashMap::new();
        let shapes: Vec<(usize, &NavmeshShape)> = graph.first.values().copied().collect();
        for (start, shape) in shapes {
            for (number, edges) in (0u32..).zip(&shape.edges) {
                let Some(from) = start.checked_add(usize::try_from(number).unwrap_or(usize::MAX))
                else {
                    continue;
                };
                if number >= count(shape) {
                    continue;
                }
                for edge in edges {
                    match edge {
                        Beyond::Nothing => {}
                        Beyond::Triangle(other) => {
                            if let Some(to) = graph.node(&(&shape.id, *other)) {
                                graph.join(from, to, None);
                            }
                        }
                        Beyond::Link { navmesh, triangle } => {
                            if graph.first.contains_key(navmesh) {
                                if let Some(to) = graph.node(&(navmesh, *triangle)) {
                                    graph.join(from, to, None);
                                }
                            } else {
                                set(&mut graph.exits, Some(from));
                            }
                        }
                    }
                }
            }
            for (triangle, door) in &shape.doors {
                if let Some(node) = graph.node(&(&shape.id, *triangle)) {
                    doors.entry(door).or_default().push(node);
                }
            }
        }
        // Doors lead through to their other side where it was read, else
        // out into the rest of the world. A door whose triangle leads
        // nowhere, as when it is cut out, can no longer be used.
        for nodes in doors.values_mut() {
            nodes.retain(|node| graph.ways.get(*node).is_some_and(|ways| !ways.is_empty()));
        }
        let mut through: HashSet<&RecordId> = HashSet::new();
        for destination in destinations {
            let Some(other) = &destination.leads_to else {
                continue;
            };
            if let (Some(here), Some(there)) = (doors.get(&destination.id), doors.get(other)) {
                for &a in here {
                    for &b in there {
                        graph.join(a, b, Some(0.0));
                    }
                }
                through.insert(&destination.id);
                through.insert(other);
            }
        }
        for (door, nodes) in &doors {
            if !through.contains(door) {
                for node in nodes {
                    set(&mut graph.exits, Some(*node));
                }
            }
        }
        graph
    }

    /// The node of a triangle, if this graph has it.
    fn node(&self, (navmesh, triangle): &Key<'_>) -> Option<usize> {
        let (start, shape) = self.first.get(navmesh)?;
        (*triangle < count(shape))
            .then(|| start.checked_add(usize::try_from(*triangle).ok()?))
            .flatten()
    }

    fn key(&self, node: usize) -> Option<Key<'a>> {
        self.nodes
            .get(node)
            .map(|(shape, number, _)| (&shape.id, *number))
    }

    fn join(&mut self, a: usize, b: usize, length: Option<f64>) {
        if a == b {
            return;
        }
        let length = length.unwrap_or_else(|| match (self.middle(a), self.middle(b)) {
            (Some(a), Some(b)) => distance(a, b),
            _ => f64::INFINITY,
        });
        if !length.is_finite() {
            return;
        }
        if let Some(ways) = self.ways.get_mut(a) {
            ways.push((b, length));
        }
        if let Some(ways) = self.ways.get_mut(b) {
            ways.push((a, length));
        }
    }

    fn middle(&self, node: usize) -> Option<[f64; 3]> {
        let (_, _, [a, b, c]) = self.nodes.get(node)?;
        let middle = [
            (a[0] + b[0] + c[0]) / 3.0,
            (a[1] + b[1] + c[1]) / 3.0,
            (a[2] + b[2] + c[2]) / 3.0,
        ];
        middle
            .iter()
            .all(|value| value.is_finite())
            .then_some(middle)
    }

    /// The triangle nearest to `at` in `place` on which characters can
    /// stand, within [`SNAP`], with how far it is.
    fn snap(&self, place: &Place, at: [f64; 3]) -> Option<(usize, f64)> {
        self.nodes
            .iter()
            .enumerate()
            .filter(|(node, (shape, _, _))| {
                &shape.place == place
                    && (self.ways.get(*node).is_some_and(|ways| !ways.is_empty())
                        || self.exits.get(*node).copied().unwrap_or(false))
            })
            .filter_map(|(node, (_, _, corners))| {
                let distance = triangle_distance(corners, at);
                (distance <= SNAP).then_some((node, distance))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
    }

    /// The triangle nearest to `at` on which characters can stand, in any
    /// place, within [`SNAP`], with how far it is.
    fn snap_anywhere(&self, at: [f64; 3]) -> Option<(usize, f64)> {
        self.nodes
            .iter()
            .enumerate()
            .filter(|(node, _)| {
                self.ways.get(*node).is_some_and(|ways| !ways.is_empty())
                    || self.exits.get(*node).copied().unwrap_or(false)
            })
            .filter_map(|(node, (_, _, corners))| {
                let distance = triangle_distance(corners, at);
                (distance <= SNAP).then_some((node, distance))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
    }

    /// The point nearest to `at` on a triangle that `paths` reach, within
    /// [`GROUND_WITHIN`].
    fn nearest_reached(&self, paths: &Paths, at: [f64; 3]) -> Option<[f64; 3]> {
        self.nodes
            .iter()
            .enumerate()
            .filter(|(node, _)| paths.reached(*node))
            .filter_map(|(_, (_, _, corners))| {
                let point = nearest_on_triangle(corners, at)?;
                let away = distance(point, at);
                (away <= GROUND_WITHIN).then_some((point, away))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(point, _)| point)
    }

    /// The largest part of this graph, by triangles, and of it a triangle
    /// that `other` has as well and that leads somewhere there.
    fn largest_kept(&self, other: &Graph<'_>) -> Option<(&'a RecordId, u32)> {
        let mut part = vec![usize::MAX; self.nodes.len()];
        let mut best: Option<(usize, usize)> = None;
        for start in 0..self.nodes.len() {
            if part.get(start) != Some(&usize::MAX) {
                continue;
            }
            let mut stack = vec![start];
            let mut size = 0;
            while let Some(node) = stack.pop() {
                match part.get_mut(node) {
                    Some(slot) if *slot == usize::MAX => *slot = start,
                    _ => continue,
                }
                size += 1;
                stack.extend(self.ways.get(node).into_iter().flatten().map(|(to, _)| *to));
            }
            if best.is_none_or(|(_, most)| size > most) {
                best = Some((start, size));
            }
        }
        let (root, _) = best?;
        (0..self.nodes.len())
            .filter(|node| part.get(*node) == Some(&root))
            .filter_map(|node| self.key(node))
            .find(|key| {
                other
                    .node(key)
                    .and_then(|node| other.ways.get(node))
                    .is_some_and(|ways| !ways.is_empty())
            })
    }

    /// The shortest ways from `sources`. With `targets`, it stops once
    /// every target is reached.
    fn paths(&self, sources: &[usize], targets: &[usize]) -> Paths {
        let mut distance = vec![f64::INFINITY; self.nodes.len()];
        let mut previous = vec![usize::MAX; self.nodes.len()];
        let mut heap = BinaryHeap::new();
        for &source in sources {
            if let Some(slot) = distance.get_mut(source) {
                *slot = 0.0;
                heap.push(Step(0.0, source));
            }
        }
        let mut left: HashSet<usize> = targets.iter().copied().collect();
        while let Some(Step(length, node)) = heap.pop() {
            if distance.get(node).is_none_or(|known| length > *known) {
                continue;
            }
            if !targets.is_empty() {
                left.remove(&node);
                if left.is_empty() {
                    break;
                }
            }
            for &(to, step) in self.ways.get(node).into_iter().flatten() {
                let next = length + step;
                if let Some(slot) = distance.get_mut(to)
                    && next < *slot
                {
                    *slot = next;
                    if let Some(back) = previous.get_mut(to) {
                        *back = node;
                    }
                    heap.push(Step(next, to));
                }
            }
        }
        Paths { distance, previous }
    }
}

/// The shortest ways from some nodes to every other.
struct Paths {
    distance: Vec<f64>,
    previous: Vec<usize>,
}

impl Paths {
    fn reached(&self, node: usize) -> bool {
        self.distance(node).is_some()
    }

    fn distance(&self, node: usize) -> Option<f64> {
        self.distance
            .get(node)
            .copied()
            .filter(|length| length.is_finite())
    }

    /// The way to `node` through the middles of the triangles, with
    /// `start` before the first and `end` after the last where given.
    fn route(
        &self,
        graph: &Graph<'_>,
        node: usize,
        start: Option<[f64; 3]>,
        end: Option<[f64; 3]>,
    ) -> Option<Route> {
        let length = self.distance(node)?;
        let mut points = Vec::new();
        let mut at = node;
        // A way visits each node once.
        for _ in 0..=self.previous.len() {
            points.extend(graph.middle(at));
            match self.previous.get(at) {
                Some(&back) if back != usize::MAX => at = back,
                _ => break,
            }
        }
        points.reverse();
        // The way runs from the source, so `start` is where it begins
        // after reversing, and `end` where it ends.
        let mut all: Vec<[f64; 3]> = start.into_iter().collect();
        all.extend(points);
        all.extend(end);
        Some(Route {
            length,
            points: all,
        })
    }
}

/// A way read backwards: the ways between destinations are followed from
/// the destination, and drawn from where they start.
fn reverse(mut route: Route) -> Route {
    route.points.reverse();
    route
}

/// An entry of the queue of the shortest ways, the shortest first.
struct Step(f64, usize);

impl PartialEq for Step {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Step {}

impl PartialOrd for Step {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Step {
    fn cmp(&self, other: &Self) -> Ordering {
        other.0.total_cmp(&self.0).then(other.1.cmp(&self.1))
    }
}

fn sources(exits: &[bool]) -> Vec<usize> {
    (0..exits.len())
        .filter(|node| exits.get(*node).copied().unwrap_or(false))
        .collect()
}

fn set(flags: &mut [bool], node: Option<usize>) {
    if let Some(slot) = node.and_then(|node| flags.get_mut(node)) {
        *slot = true;
    }
}

fn count(shape: &NavmeshShape) -> u32 {
    u32::try_from(shape.triangles.len()).unwrap_or(u32::MAX)
}

fn position(destination: &PlacedDestination) -> [f64; 3] {
    destination.position.map(f64::from)
}

fn middle_of(shape: &NavmeshShape, index: usize) -> Option<[f64; 3]> {
    let triangle = shape.triangles.get(index)?;
    let corner = |vertex: u32| -> Option<[f64; 3]> {
        let [x, y, z] = *shape.vertices.get(usize::try_from(vertex).ok()?)?;
        Some([f64::from(x), f64::from(y), f64::from(z)])
    };
    let [a, b, c] = [
        corner(triangle[0])?,
        corner(triangle[1])?,
        corner(triangle[2])?,
    ];
    Some([
        (a[0] + b[0] + c[0]) / 3.0,
        (a[1] + b[1] + c[1]) / 3.0,
        (a[2] + b[2] + c[2]) / 3.0,
    ])
}

fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

fn flat_distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

/// How far `at` lies from a triangle: straight above or below it where it
/// lies over it seen from above, else from its nearest edge.
fn triangle_distance(corners: &[[f64; 3]; 3], at: [f64; 3]) -> f64 {
    let [a, b, c] = *corners;
    let det = (b[1] - c[1]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[1] - c[1]);
    if det.abs() > 1e-9 {
        let first = ((b[1] - c[1]) * (at[0] - c[0]) + (c[0] - b[0]) * (at[1] - c[1])) / det;
        let second = ((c[1] - a[1]) * (at[0] - c[0]) + (a[0] - c[0]) * (at[1] - c[1])) / det;
        let third = 1.0 - first - second;
        if first >= 0.0 && second >= 0.0 && third >= 0.0 {
            let height = first * a[2] + second * b[2] + third * c[2];
            return (at[2] - height).abs();
        }
    }
    [(a, b), (b, c), (c, a)]
        .into_iter()
        .map(|(from, to)| segment_distance(from, to, at))
        .fold(f64::INFINITY, f64::min)
}

/// The point of a triangle nearest to `at`: straight above or below it
/// where it lies over the triangle seen from above, else the nearest point
/// of its edges. `None` for a triangle without a real shape.
fn nearest_on_triangle(corners: &[[f64; 3]; 3], at: [f64; 3]) -> Option<[f64; 3]> {
    let [a, b, c] = *corners;
    if corners.iter().flatten().any(|value| !value.is_finite()) {
        return None;
    }
    let det = (b[1] - c[1]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[1] - c[1]);
    if det.abs() > 1e-9 {
        let first = ((b[1] - c[1]) * (at[0] - c[0]) + (c[0] - b[0]) * (at[1] - c[1])) / det;
        let second = ((c[1] - a[1]) * (at[0] - c[0]) + (a[0] - c[0]) * (at[1] - c[1])) / det;
        let third = 1.0 - first - second;
        if first >= 0.0 && second >= 0.0 && third >= 0.0 {
            return Some([at[0], at[1], first * a[2] + second * b[2] + third * c[2]]);
        }
    }
    [(a, b), (b, c), (c, a)]
        .into_iter()
        .map(|(from, to)| segment_point(from, to, at))
        .min_by(|p, q| distance(*p, at).total_cmp(&distance(*q, at)))
}

/// The point of the segment from `from` to `to` nearest to `at`.
fn segment_point(from: [f64; 3], to: [f64; 3], at: [f64; 3]) -> [f64; 3] {
    let along = [to[0] - from[0], to[1] - from[1], to[2] - from[2]];
    let length = along[0] * along[0] + along[1] * along[1] + along[2] * along[2];
    let share = if length > 0.0 {
        (((at[0] - from[0]) * along[0]
            + (at[1] - from[1]) * along[1]
            + (at[2] - from[2]) * along[2])
            / length)
            .clamp(0.0, 1.0)
    } else {
        0.0
    };
    [
        from[0] + along[0] * share,
        from[1] + along[1] * share,
        from[2] + along[2] * share,
    ]
}

fn segment_distance(from: [f64; 3], to: [f64; 3], at: [f64; 3]) -> f64 {
    let along = [to[0] - from[0], to[1] - from[1], to[2] - from[2]];
    let length = along[0] * along[0] + along[1] * along[1] + along[2] * along[2];
    let share = if length > 0.0 {
        (((at[0] - from[0]) * along[0]
            + (at[1] - from[1]) * along[1]
            + (at[2] - from[2]) * along[2])
            / length)
            .clamp(0.0, 1.0)
    } else {
        0.0
    };
    distance(
        [
            from[0] + along[0] * share,
            from[1] + along[1] * share,
            from[2] + along[2] * share,
        ],
        at,
    )
}

#[cfg(test)]
mod tests {
    use mallet_core::reach::DestinationKind;
    use proptest::prelude::*;

    use super::*;

    fn id(plugin: &str, local_id: u32) -> RecordId {
        RecordId {
            plugin: plugin.to_lowercase(),
            local_id,
        }
    }

    fn world() -> Place {
        Place::Exterior {
            worldspace: id("Skyrim.esm", 0x3C),
        }
    }

    /// The side of a square of the grids, in game units.
    const SIDE: i32 = 300;

    /// A navigation mesh over the squares `cells` (column, row), two
    /// triangles each, in the order given: the first square has triangles
    /// 0 and 1. Triangles that share an edge lead onto each other.
    fn grid(local_id: u32, cells: &[(i32, i32)]) -> NavmeshShape {
        let mut vertices: Vec<[i32; 3]> = Vec::new();
        let mut vertex = |x: i32, y: i32| -> u32 {
            let point = [x * SIDE, y * SIDE, 0];
            let index = vertices
                .iter()
                .position(|known| *known == point)
                .unwrap_or_else(|| {
                    vertices.push(point);
                    vertices.len() - 1
                });
            u32::try_from(index).unwrap()
        };
        let mut triangles = Vec::new();
        for &(x, y) in cells {
            let corners = [
                vertex(x, y),
                vertex(x + 1, y),
                vertex(x + 1, y + 1),
                vertex(x, y + 1),
            ];
            triangles.push([corners[0], corners[1], corners[2]]);
            triangles.push([corners[0], corners[2], corners[3]]);
        }
        let edges = triangles
            .iter()
            .enumerate()
            .map(|(index, corners)| {
                let mut edges = [Beyond::Nothing, Beyond::Nothing, Beyond::Nothing];
                for (edge, slot) in edges.iter_mut().enumerate() {
                    let (a, b) = (corners[edge], corners[(edge + 1) % 3]);
                    if let Some(other) = triangles.iter().enumerate().position(|(other, them)| {
                        other != index && them.contains(&a) && them.contains(&b)
                    }) {
                        *slot = Beyond::Triangle(u32::try_from(other).unwrap());
                    }
                }
                edges
            })
            .collect();
        NavmeshShape {
            id: id("Skyrim.esm", local_id),
            plugin: "Skyrim.esm".to_owned(),
            used: true,
            place: world(),
            place_name: Some("Tamriel 0,0".to_owned()),
            vertices,
            triangles,
            edges,
            doors: Vec::new(),
        }
    }

    /// Lets triangle `triangle` lead out onto a mesh that was not read.
    fn leads_out(mut shape: NavmeshShape, triangle: usize) -> NavmeshShape {
        if let Some(edges) = shape.edges.get_mut(triangle) {
            edges[0] = Beyond::Link {
                navmesh: id("Skyrim.esm", 0xFFFF),
                triangle: 0,
            };
        }
        shape
    }

    /// A destination standing in the middle of square (`x`, `y`).
    fn at(local_id: u32, kind: DestinationKind, x: i32, y: i32) -> PlacedDestination {
        PlacedDestination {
            id: id("Docks.esp", local_id),
            plugin: "Docks.esp".to_owned(),
            kind,
            name: Some(format!("Thing{local_id:X}")),
            place: world(),
            position: [x * SIDE + SIDE / 2, y * SIDE + SIDE / 2, 0],
            leads_to: None,
        }
    }

    fn hidden(navmesh: usize, squares: &[u32]) -> Carried {
        Carried {
            hidden: 0,
            navmesh,
            triangles: squares
                .iter()
                .flat_map(|place| [2 * place, 2 * place + 1])
                .collect(),
        }
    }

    fn outcomes(found: &Reachability) -> Vec<(usize, Outcome)> {
        found
            .reaches
            .iter()
            .map(|reach| (reach.destination, reach.outcome))
            .collect()
    }

    /// A bank, a bridge and a jetty of two squares in a row; the bank
    /// leads out.
    fn jetty() -> NavmeshShape {
        leads_out(grid(0x200, &[(0, 0), (1, 0), (2, 0), (3, 0)]), 0)
    }

    #[test]
    fn a_bed_beyond_a_hidden_bridge_is_cut_off_and_one_on_it_is_gone() {
        let before = vec![jetty()];
        let after = cut(&before, &[hidden(0, &[1])]);
        let destinations = [
            at(0x10, DestinationKind::Furniture, 3, 0),
            at(0x11, DestinationKind::IdleMarker, 1, 0),
            at(0x12, DestinationKind::Door, 0, 0),
        ];
        let found = reachability(&before, &after, &destinations);
        assert_eq!(found.checked, 3);
        assert_eq!(
            outcomes(&found),
            vec![(1, Outcome::Gone), (0, Outcome::CutOff)]
        );
        // The way before runs from the bank over the bridge to the bed.
        let way = found.reaches[1].before.as_ref().unwrap();
        assert!(way.length > 2.0 * f64::from(SIDE));
        assert_eq!(way.points.last(), Some(&[1050.0, 150.0, 0.0]));
        assert!(way.points.first().unwrap()[0] < f64::from(SIDE));
        assert_eq!(found.reaches[1].after, None);
    }

    /// A river crossed twice, by a short bridge in the south and a long
    /// way round in the north. The squares form a ring around the river
    /// square (1, 1) in a 3 × 5 grid; the western bank leads out.
    fn two_bridges() -> NavmeshShape {
        let ring = [
            (0, 0),
            (1, 0),
            (2, 0),
            (2, 1),
            (2, 2),
            (2, 3),
            (2, 4),
            (1, 4),
            (0, 4),
            (0, 3),
            (0, 2),
            (0, 1),
        ];
        leads_out(grid(0x100, &ring), 22)
    }

    #[test]
    fn hiding_the_near_bridge_makes_the_way_round_longer() {
        let before = vec![two_bridges()];
        // The southern bridge, square (1, 0), third in the list.
        let after = cut(&before, &[hidden(0, &[1])]);
        let destinations = [
            at(0x20, DestinationKind::Furniture, 0, 0),
            at(0x21, DestinationKind::Furniture, 2, 0),
        ];
        let found = reachability(&before, &after, &destinations);
        assert_eq!(found.checked, 2);
        assert_eq!(outcomes(&found), vec![(0, Outcome::Longer { from: 1 })]);
        let reach = &found.reaches[0];
        let (before, after) = (
            reach.before.as_ref().unwrap(),
            reach.after.as_ref().unwrap(),
        );
        assert!(after.length > before.length * LONGER_RATIO);
        // Drawn from the other destination to this one.
        assert_eq!(before.points.first(), Some(&[750.0, 150.0, 0.0]));
        assert_eq!(after.points.last(), Some(&[150.0, 150.0, 0.0]));
        // The way round reaches the north.
        assert!(after.points.iter().any(|point| point[1] > 1200.0));
    }

    #[test]
    fn nothing_changed_changes_nothing() {
        let before = vec![jetty()];
        let destinations = [at(0x10, DestinationKind::Furniture, 3, 0)];
        assert_eq!(
            reachability(&before, &before, &destinations),
            Reachability::default()
        );
        // A destination far from the paths is not checked.
        let after = cut(&before, &[hidden(0, &[1])]);
        let far = [at(0x10, DestinationKind::Furniture, 3, 5)];
        assert_eq!(reachability(&before, &after, &far).checked, 0);
    }

    #[test]
    fn a_door_leads_through_to_its_other_side() {
        // The jetty's end and a house reached only through its door, whose
        // other side is the door of the house, in a mesh of its own.
        let mut outside = jetty();
        outside.doors = vec![(7, id("Docks.esp", 0x30))];
        let mut house = grid(0x300, &[(10, 10), (11, 10)]);
        house.place = Place::Interior {
            cell: id("Docks.esp", 0x400),
        };
        house.doors = vec![(0, id("Docks.esp", 0x31))];
        let door = |local_id: u32, other: u32, place: Place, x: i32, y: i32| PlacedDestination {
            place,
            leads_to: Some(id("Docks.esp", other)),
            ..at(local_id, DestinationKind::Door, x, y)
        };
        let mut bed = at(0x32, DestinationKind::Furniture, 11, 10);
        bed.place = house.place.clone();
        let destinations = [
            door(0x30, 0x31, world(), 3, 0),
            door(0x31, 0x30, house.place.clone(), 10, 10),
            bed,
        ];
        let before = vec![outside, house];
        let after = cut(&before, &[hidden(0, &[1])]);
        let found = reachability(&before, &after, &destinations);
        // The door outside is cut off, and with it the house behind it.
        assert_eq!(
            outcomes(&found),
            vec![
                (0, Outcome::CutOff),
                (1, Outcome::CutOff),
                (2, Outcome::CutOff)
            ]
        );

        // A door whose other side was not read leads out.
        let mut alone = jetty();
        alone.doors = vec![(7, id("Docks.esp", 0x30))];
        let before = vec![alone];
        let after = cut(&before, &[hidden(0, &[1])]);
        let found = reachability(&before, &after, &destinations[..1]);
        assert_eq!(found.reaches, Vec::new());

        // A door on the hidden bridge leads nowhere once its triangle is
        // cut out: the jetty beyond is cut off, and the door with it.
        let mut on_bridge = jetty();
        on_bridge.doors = vec![(2, id("Docks.esp", 0x30))];
        let before = vec![on_bridge];
        let after = cut(&before, &[hidden(0, &[1])]);
        let trapdoor = door(0x30, 0x31, world(), 1, 0);
        let bed = at(0x10, DestinationKind::Furniture, 3, 0);
        let found = reachability(&before, &after, &[trapdoor, bed]);
        assert_eq!(
            outcomes(&found),
            vec![(0, Outcome::Gone), (1, Outcome::CutOff)]
        );
    }

    #[test]
    fn cutting_takes_links_from_other_meshes_away() {
        let mut bank = leads_out(grid(0x300, &[(0, 0), (1, 0)]), 0);
        let far = grid(0x301, &[(2, 0), (3, 0)]);
        if let Some(edges) = bank.edges.get_mut(2) {
            edges[1] = Beyond::Link {
                navmesh: id("Skyrim.esm", 0x301),
                triangle: 1,
            };
        }
        let before = vec![bank, far];
        let after = cut(&before, &[hidden(1, &[0])]);
        assert_eq!(after[0].edges[2][1], Beyond::Nothing);
        assert_eq!(
            after[1].edges[0],
            [Beyond::Nothing, Beyond::Nothing, Beyond::Nothing]
        );
        let found = reachability(
            &before,
            &after,
            &[at(0x40, DestinationKind::MapMarker, 3, 0)],
        );
        assert_eq!(outcomes(&found), vec![(0, Outcome::CutOff)]);
    }

    #[test]
    fn a_repair_that_adds_a_way_keeps_the_jetty() {
        // The repair lays new triangles beside the hidden bridge: a second
        // row joins bank and jetty.
        let before = vec![jetty()];
        let repaired = leads_out(
            grid(
                0x200,
                &[(0, 0), (1, 0), (2, 0), (3, 0), (0, 1), (1, 1), (2, 1)],
            ),
            0,
        );
        let mut after = cut(&[repaired], &[hidden(0, &[1])]);
        after[0].place_name = before[0].place_name.clone();
        let found = reachability(
            &before,
            &after,
            &[at(0x10, DestinationKind::Furniture, 3, 0)],
        );
        assert_eq!(found.checked, 1);
        assert_eq!(found.reaches, Vec::new());
    }

    #[test]
    fn inside_without_a_way_out_the_largest_part_is_where_characters_come_from() {
        let mut hall = grid(0x500, &[(0, 0), (1, 0), (2, 0), (3, 0), (4, 0)]);
        hall.place = Place::Interior {
            cell: id("Docks.esp", 0x600),
        };
        let mut gallery = at(0x50, DestinationKind::Furniture, 4, 0);
        gallery.place = hall.place.clone();
        let before = vec![hall];
        let after = cut(&before, &[hidden(0, &[3])]);
        let found = reachability(&before, &after, &[gallery]);
        assert_eq!(outcomes(&found), vec![(0, Outcome::CutOff)]);
    }

    fn arbitrary_shape() -> impl Strategy<Value = NavmeshShape> {
        (
            prop::collection::vec(prop::array::uniform3(-2000i32..2000), 0..12),
            prop::collection::vec(prop::array::uniform3(0u32..14), 0..12),
            prop::collection::vec(prop::array::uniform3(0u32..16), 0..12),
            prop::collection::vec((0u32..14, 0u32..4), 0..3),
            any::<bool>(),
        )
            .prop_map(|(vertices, triangles, edges, doors, linked)| {
                let edges = edges
                    .into_iter()
                    .map(|numbers| {
                        numbers.map(|number| match number {
                            0..=11 => Beyond::Triangle(number),
                            12 if linked => Beyond::Link {
                                navmesh: id("Skyrim.esm", 0x700),
                                triangle: number,
                            },
                            12 => Beyond::Link {
                                navmesh: id("Skyrim.esm", 0x701),
                                triangle: 0,
                            },
                            _ => Beyond::Nothing,
                        })
                    })
                    .collect();
                NavmeshShape {
                    doors: doors
                        .into_iter()
                        .map(|(triangle, door)| (triangle, id("Docks.esp", door)))
                        .collect(),
                    edges,
                    ..NavmeshShape {
                        id: id("Skyrim.esm", 0x700),
                        plugin: "Skyrim.esm".to_owned(),
                        used: true,
                        place: world(),
                        place_name: None,
                        vertices,
                        triangles,
                        edges: Vec::new(),
                        doors: Vec::new(),
                    }
                }
            })
    }

    #[test]
    fn spots_are_reached_cut_off_or_off_the_paths_with_ground_nearby() {
        // A jetty whose bridge is hidden: the bank still leads out.
        let paths = cut(&[jetty()], &[hidden(0, &[1])]);
        let middle = |x: i32| [f64::from(x * SIDE + SIDE / 2), f64::from(SIDE / 2), 0.0];
        let found = reach_spots(
            &paths,
            &[],
            &[
                middle(0),
                middle(3),
                // High above the bank, as on a deck.
                [f64::from(SIDE / 2), f64::from(SIDE / 2), 400.0],
                // Far out to sea.
                [9000.0, 9000.0, 0.0],
            ],
        );
        let states: Vec<SpotState> = found.iter().map(|spot| spot.state).collect();
        assert_eq!(
            states,
            vec![
                SpotState::Reached,
                SpotState::CutOff,
                SpotState::OffPaths,
                SpotState::OffPaths
            ]
        );
        // The ground below the deck is the bank; the cut-off jetty's
        // nearest reachable ground is the edge of the bank.
        assert_eq!(
            found[2].ground,
            Some([f64::from(SIDE / 2), f64::from(SIDE / 2), 0.0])
        );
        assert_eq!(found[2].off, Some(400.0));
        let jetty_ground = found[1].ground.unwrap();
        assert!(
            (jetty_ground[0] - f64::from(SIDE)).abs() < 1e-6,
            "{jetty_ground:?}"
        );
        assert_eq!(found[3].ground, None);
    }

    proptest! {
        #[test]
        fn never_panics(
            before in arbitrary_shape(),
            after in arbitrary_shape(),
            cut_triangles in prop::collection::vec(0u32..16, 0..6),
            spots in prop::collection::vec((0u32..4, -2000i32..2000, -2000i32..2000, 0u32..5), 0..6),
        ) {
            let destinations: Vec<PlacedDestination> = spots
                .into_iter()
                .map(|(local_id, x, y, other)| PlacedDestination {
                    id: id("Docks.esp", local_id),
                    plugin: "Docks.esp".to_owned(),
                    kind: DestinationKind::Door,
                    name: None,
                    place: world(),
                    position: [x, y, 0],
                    leads_to: (other < 4).then(|| id("Docks.esp", other)),
                })
                .collect();
            let carried = [Carried { hidden: 0, navmesh: 0, triangles: cut_triangles }];
            let before = vec![before];
            let cut_before = cut(&before, &carried);
            let _ = reachability(&before, &cut_before, &destinations);
            let _ = reachability(&before, &[after], &destinations);
            let points: Vec<[f64; 3]> = destinations.iter().map(position).collect();
            let _ = reach_spots(&cut_before, &destinations, &points);
        }
    }
}
