//! What taking the paths over hidden objects out of the navigation meshes
//! would leave (ADR 0052).
//!
//! Where a navigation mesh runs over an object that is hidden, the repair
//! takes the triangles on the object out of the paths: nothing leads onto
//! them any more. The game's characters then find their way over what is
//! left, such as the bridge that was there before a mod added a second
//! one. That only works where something is left: a jetty reached only
//! over the hidden bridge, a boat moored at it, the door of a house on it
//! would be cut off. This module finds such parts before anything is
//! written.
//!
//! The paths are followed through every navigation mesh that was read and
//! that the game uses: from triangle to triangle within a mesh, and along
//! the links between meshes. A link to a mesh that was not read leads out
//! into the rest of the world. A part that still leads out is reachable;
//! of the parts of what hung together before that do not, all are cut
//! off. Where nothing leads out, as in an interior cell, the part with the
//! most doors counts as the one characters come from.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use mallet_core::facts::RecordId;
use mallet_core::facts::navmeshes::{Beyond, NavmeshShape, NearHidden};

use crate::contact::Point;
use crate::walkways::Carried;

/// The smallest part, in square game units, that counts as cut off when
/// it has no door: about two by two metres. A sliver of path beside the
/// hidden object that nobody stands on is left alone.
pub const MIN_AREA: f64 = 20_000.0;

/// What taking the triangles over hidden objects out of the paths leaves.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Repair {
    /// By navigation mesh, as its place in [`NearHidden::navmeshes`], the
    /// triangles to take out. Only versions the game uses.
    pub cuts: BTreeMap<usize, BTreeSet<u32>>,
    /// The parts of the paths that nobody could reach any more.
    pub cut_off: Vec<CutOff>,
    /// The doors whose triangles would be taken out: characters could no
    /// longer step through them.
    pub lost_doors: Vec<RecordId>,
}

impl Repair {
    /// Whether the repair leaves every remaining path reachable.
    pub fn is_safe(&self) -> bool {
        self.cut_off.is_empty() && self.lost_doors.is_empty()
    }
}

/// A part of the paths that would be cut off.
#[derive(Debug, Clone, PartialEq)]
pub struct CutOff {
    /// The navigation meshes it lies in, as their places in
    /// [`NearHidden::navmeshes`].
    pub navmeshes: BTreeSet<usize>,
    /// How many triangles it has.
    pub triangles: usize,
    /// How large it is, in square game units.
    pub area: f64,
    /// Its middle, in the coordinates of the worldspace or the cell.
    pub centre: Point,
    /// The doors in it.
    pub doors: Vec<RecordId>,
}

/// What taking the triangles in `carried` out of the paths of the
/// navigation meshes in `near` leaves. Versions of meshes the game does
/// not use are ignored.
pub fn repair(near: &NearHidden, carried: &[Carried]) -> Repair {
    let net = Net::new(near);
    let mut cuts: BTreeMap<usize, BTreeSet<u32>> = BTreeMap::new();
    for carried in carried {
        if net.base.contains_key(&carried.navmesh) {
            cuts.entry(carried.navmesh)
                .or_default()
                .extend(carried.triangles.iter().copied());
        }
    }
    let cut: Vec<bool> = net.nodes_of(&cuts);

    let mut before = Parts::new(net.len());
    let mut after = Parts::new(net.len());
    for &(from, to) in &net.ways {
        before.join(from, to);
        if !flag(&cut, from) && !flag(&cut, to) {
            after.join(from, to);
        }
    }

    // The parts after the cut, grouped by what hung together before.
    let mut wholes: BTreeMap<usize, BTreeMap<usize, Vec<usize>>> = BTreeMap::new();
    for node in 0..net.len() {
        if flag(&cut, node) {
            continue;
        }
        wholes
            .entry(before.root(node))
            .or_default()
            .entry(after.root(node))
            .or_default()
            .push(node);
    }
    // What hung together before and was cut into, and whether it led out.
    let mut touched: BTreeMap<usize, bool> = BTreeMap::new();
    for node in 0..net.len() {
        if flag(&cut, node) {
            touched.insert(before.root(node), false);
        }
    }
    for node in 0..net.len() {
        if flag(&net.out, node)
            && let Some(led_out) = touched.get_mut(&before.root(node))
        {
            *led_out = true;
        }
    }

    let mut cut_off = Vec::new();
    for (whole, parts) in wholes {
        let Some(&led_out) = touched.get(&whole) else {
            continue;
        };
        let parts: Vec<Part> = parts.values().map(|nodes| net.part(nodes)).collect();
        let reference = if led_out {
            None
        } else {
            // Nothing leads out: characters come from the part with the
            // most doors, else the largest.
            parts
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| {
                    a.doors
                        .len()
                        .cmp(&b.doors.len())
                        .then(a.area.total_cmp(&b.area))
                })
                .map(|(index, _)| index)
        };
        for (index, part) in parts.into_iter().enumerate() {
            if part.leads_out || Some(index) == reference {
                continue;
            }
            if part.doors.is_empty() && part.area < MIN_AREA {
                continue;
            }
            cut_off.push(part.into_cut_off());
        }
    }
    cut_off.sort_by(|a, b| b.area.total_cmp(&a.area));

    let lost_doors = net
        .doors
        .iter()
        .filter(|(node, _)| flag(&cut, *node))
        .map(|(_, door)| (*door).clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    Repair {
        cuts,
        cut_off,
        lost_doors,
    }
}

fn flag(flags: &[bool], node: usize) -> bool {
    flags.get(node).copied().unwrap_or(false)
}

/// The triangles of every mesh the game uses, as numbered nodes, and the
/// ways between them.
struct Net<'a> {
    /// The meshes, by their place in `near`, with the number of their
    /// first triangle among the nodes.
    base: HashMap<usize, usize>,
    /// For each node, its mesh and its triangle.
    nodes: Vec<(&'a NavmeshShape, usize, u32)>,
    /// Pairs of nodes characters can step between.
    ways: Vec<(usize, usize)>,
    /// The nodes that lead out into meshes that were not read.
    out: Vec<bool>,
    /// The nodes where characters step through a door, with the door.
    doors: Vec<(usize, &'a RecordId)>,
}

impl<'a> Net<'a> {
    fn new(near: &'a NearHidden) -> Self {
        let mut base = HashMap::new();
        let mut by_id: HashMap<&RecordId, usize> = HashMap::new();
        let mut nodes = Vec::new();
        for (index, shape) in near.navmeshes.iter().enumerate() {
            if !shape.used {
                continue;
            }
            base.insert(index, nodes.len());
            by_id.insert(&shape.id, index);
            nodes.extend(
                (0u32..)
                    .zip(&shape.triangles)
                    .map(|(number, _)| (shape, index, number)),
            );
        }
        let mut ways = Vec::new();
        let mut out = vec![false; nodes.len()];
        let mut doors = Vec::new();
        for (&index, &first) in &base {
            let Some(shape) = near.navmeshes.get(index) else {
                continue;
            };
            let node_of = |triangle: u32| {
                let triangle = usize::try_from(triangle).ok()?;
                (triangle < shape.triangles.len()).then(|| first + triangle)
            };
            for (number, edges) in (0u32..).zip(&shape.edges) {
                let Some(from) = node_of(number) else {
                    continue;
                };
                for edge in edges {
                    match edge {
                        Beyond::Nothing => {}
                        Beyond::Triangle(other) => {
                            if let Some(to) = node_of(*other) {
                                ways.push((from, to));
                            }
                        }
                        Beyond::Link { navmesh, triangle } => {
                            let target = by_id.get(navmesh).and_then(|other| {
                                let other_first = base.get(other)?;
                                let other_shape = near.navmeshes.get(*other)?;
                                let triangle = usize::try_from(*triangle).ok()?;
                                (triangle < other_shape.triangles.len())
                                    .then(|| other_first + triangle)
                            });
                            match target {
                                Some(to) => ways.push((from, to)),
                                None if !by_id.contains_key(navmesh) => {
                                    if let Some(slot) = out.get_mut(from) {
                                        *slot = true;
                                    }
                                }
                                None => {}
                            }
                        }
                    }
                }
            }
            for (triangle, door) in &shape.doors {
                if let Some(node) = node_of(*triangle) {
                    doors.push((node, door));
                }
            }
        }
        Self {
            base,
            nodes,
            ways,
            out,
            doors,
        }
    }

    fn len(&self) -> usize {
        self.nodes.len()
    }

    /// For each node, whether it is among `cuts`.
    fn nodes_of(&self, cuts: &BTreeMap<usize, BTreeSet<u32>>) -> Vec<bool> {
        let mut cut = vec![false; self.len()];
        for (navmesh, triangles) in cuts {
            let Some(first) = self.base.get(navmesh) else {
                continue;
            };
            for triangle in triangles {
                let node = usize::try_from(*triangle)
                    .ok()
                    .and_then(|triangle| first.checked_add(triangle));
                let in_mesh = node
                    .and_then(|node| self.nodes.get(node))
                    .is_some_and(|(_, index, _)| index == navmesh);
                if let (Some(node), true) = (node, in_mesh)
                    && let Some(slot) = cut.get_mut(node)
                {
                    *slot = true;
                }
            }
        }
        cut
    }

    /// What is known of the part made of `nodes`.
    fn part(&self, nodes: &[usize]) -> Part {
        let mut part = Part {
            navmeshes: BTreeSet::new(),
            triangles: nodes.len(),
            area: 0.0,
            weighted: [0.0; 3],
            leads_out: false,
            doors: Vec::new(),
        };
        let members: BTreeSet<usize> = nodes.iter().copied().collect();
        for &node in nodes {
            part.leads_out |= flag(&self.out, node);
            let Some((shape, index, triangle)) = self.nodes.get(node) else {
                continue;
            };
            part.navmeshes.insert(*index);
            if let Some((area, middle)) = measure(shape, *triangle) {
                part.area += area;
                for (sum, value) in part.weighted.iter_mut().zip(middle) {
                    *sum += value * area;
                }
            }
        }
        part.doors = self
            .doors
            .iter()
            .filter(|(node, _)| members.contains(node))
            .map(|(_, door)| (*door).clone())
            .collect();
        part
    }
}

/// A part of the paths after the cut.
struct Part {
    navmeshes: BTreeSet<usize>,
    triangles: usize,
    area: f64,
    /// The middles of its triangles, weighted by their areas.
    weighted: Point,
    leads_out: bool,
    doors: Vec<RecordId>,
}

impl Part {
    fn into_cut_off(self) -> CutOff {
        let centre = if self.area > 0.0 {
            self.weighted.map(|sum| sum / self.area)
        } else {
            self.weighted
        };
        CutOff {
            navmeshes: self.navmeshes,
            triangles: self.triangles,
            area: self.area,
            centre,
            doors: self.doors,
        }
    }
}

/// The area and the middle of a triangle of `shape`.
fn measure(shape: &NavmeshShape, triangle: u32) -> Option<(f64, Point)> {
    let corners = shape.triangles.get(usize::try_from(triangle).ok()?)?;
    let corner = |index: u32| -> Option<Point> {
        let [x, y, z] = *shape.vertices.get(usize::try_from(index).ok()?)?;
        Some([f64::from(x), f64::from(y), f64::from(z)])
    };
    let [a, b, c] = [
        corner(corners[0])?,
        corner(corners[1])?,
        corner(corners[2])?,
    ];
    let (along, across) = (
        [b[0] - a[0], b[1] - a[1], b[2] - a[2]],
        [c[0] - a[0], c[1] - a[1], c[2] - a[2]],
    );
    let cross = [
        along[1] * across[2] - along[2] * across[1],
        along[2] * across[0] - along[0] * across[2],
        along[0] * across[1] - along[1] * across[0],
    ];
    let area = (cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2]).sqrt() / 2.0;
    let middle = [
        (a[0] + b[0] + c[0]) / 3.0,
        (a[1] + b[1] + c[1]) / 3.0,
        (a[2] + b[2] + c[2]) / 3.0,
    ];
    Some((area, middle))
}

/// Which nodes hang together: a union-find.
struct Parts {
    parent: Vec<usize>,
}

impl Parts {
    fn new(len: usize) -> Self {
        Self {
            parent: (0..len).collect(),
        }
    }

    fn root(&mut self, mut node: usize) -> usize {
        while let Some(&parent) = self.parent.get(node) {
            if parent == node {
                break;
            }
            // Halve the way for the next time.
            let grandparent = self.parent.get(parent).copied().unwrap_or(parent);
            if let Some(slot) = self.parent.get_mut(node) {
                *slot = grandparent;
            }
            node = grandparent;
        }
        node
    }

    fn join(&mut self, a: usize, b: usize) {
        let (a, b) = (self.root(a), self.root(b));
        if a != b
            && let Some(slot) = self.parent.get_mut(a.max(b))
        {
            *slot = a.min(b);
        }
    }
}

#[cfg(test)]
mod tests {
    use mallet_core::facts::objects::Place;

    use super::*;

    fn id(plugin: &str, local_id: u32) -> RecordId {
        RecordId {
            plugin: plugin.to_lowercase(),
            local_id,
        }
    }

    /// The side of a square of the grids, in game units.
    const SIDE: i32 = 200;

    /// A navigation mesh over the squares `cells` (column, row) of a grid
    /// whose squares are `side` units wide, two triangles each, in the
    /// order given: the first square has triangles 0 and 1. Triangles
    /// that share an edge lead onto each other.
    fn grid(local_id: u32, used: bool, cells: &[(i32, i32)], side: i32) -> NavmeshShape {
        let mut vertices: Vec<[i32; 3]> = Vec::new();
        let mut vertex = |x: i32, y: i32| -> u32 {
            let point = [x * side, y * side, 0];
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
            used,
            place: Place::Exterior {
                worldspace: id("Skyrim.esm", 0x3C),
            },
            place_name: Some("Tamriel 0,0".to_owned()),
            vertices,
            triangles,
            edges,
            doors: Vec::new(),
        }
    }

    /// Lets triangle `triangle` of `shape` lead out onto a mesh that was
    /// not read, over its first edge.
    fn leads_out(mut shape: NavmeshShape, triangle: usize) -> NavmeshShape {
        if let Some(edges) = shape.edges.get_mut(triangle) {
            edges[0] = Beyond::Link {
                navmesh: id("Skyrim.esm", 0xFFFF),
                triangle: 0,
            };
        }
        shape
    }

    /// The triangles of the squares at the given places in a grid's list.
    fn squares(places: &[u32]) -> Vec<u32> {
        places
            .iter()
            .flat_map(|place| [2 * place, 2 * place + 1])
            .collect()
    }

    fn carried(navmesh: usize, triangles: Vec<u32>) -> Carried {
        Carried {
            hidden: 0,
            navmesh,
            triangles,
        }
    }

    fn near(navmeshes: Vec<NavmeshShape>) -> NearHidden {
        NearHidden {
            navmeshes,
            ground: Vec::new(),
        }
    }

    fn repair_on(near: &NearHidden, places: &[u32]) -> Repair {
        repair(near, &[carried(0, squares(places))])
    }

    /// A river crossed twice: the game's bridge in the north, a mod's in
    /// the south, banks in the west and the east. The squares form a ring
    /// around the river square (1, 1).
    fn two_bridges() -> NavmeshShape {
        let ring = [
            (0, 0),
            (1, 0),
            (2, 0),
            (2, 1),
            (2, 2),
            (1, 2),
            (0, 2),
            (0, 1),
        ];
        leads_out(grid(0x100, true, &ring, SIDE), 14)
    }

    #[test]
    fn taking_out_one_of_two_bridges_cuts_nothing_off() {
        let near = near(vec![two_bridges()]);
        // The mod's bridge, square (1, 0).
        let repair = repair_on(&near, &[1]);
        assert_eq!(repair.cuts, BTreeMap::from([(0, BTreeSet::from([2, 3]))]));
        assert_eq!(repair.cut_off, []);
        assert_eq!(repair.lost_doors, []);
        assert!(repair.is_safe());
    }

    /// A bank, a bridge and a jetty of two squares in a row; the bank
    /// leads out.
    fn jetty() -> NavmeshShape {
        leads_out(
            grid(0x200, true, &[(0, 0), (1, 0), (2, 0), (3, 0)], SIDE),
            0,
        )
    }

    #[test]
    fn a_jetty_reached_only_over_the_bridge_is_cut_off() {
        let near = near(vec![jetty()]);
        let repair = repair_on(&near, &[1]);
        assert_eq!(repair.cut_off.len(), 1);
        let jetty = repair.cut_off.first().unwrap();
        assert_eq!(jetty.triangles, 4);
        assert_eq!(jetty.navmeshes, BTreeSet::from([0]));
        assert!((jetty.area - 80_000.0).abs() < 1e-6);
        // The middle of squares (2, 0) and (3, 0).
        assert!((jetty.centre[0] - 600.0).abs() < 1e-6);
        assert!((jetty.centre[1] - 100.0).abs() < 1e-6);
        assert!(!repair.is_safe());
    }

    #[test]
    fn a_sliver_beside_the_object_does_not_count() {
        // The same jetty, ten times smaller.
        let small = leads_out(grid(0x200, true, &[(0, 0), (1, 0), (2, 0), (3, 0)], 20), 0);
        let repair = repair_on(&near(vec![small]), &[1]);
        assert_eq!(repair.cut_off, []);
    }

    #[test]
    fn a_door_beyond_the_bridge_is_cut_off_and_one_on_it_is_lost() {
        let mut shape = jetty();
        // Triangle 1 lies on the bank, triangle 6 on the jetty's end.
        shape.doors = vec![(1, id("Docks.esp", 0x810)), (6, id("Docks.esp", 0x811))];
        let near = near(vec![shape]);
        let repair = repair_on(&near, &[1]);
        assert_eq!(
            repair.cut_off.first().map(|part| part.doors.clone()),
            Some(vec![id("Docks.esp", 0x811)])
        );
        assert_eq!(repair.lost_doors, []);

        // Hiding the jetty's end itself takes its door away.
        let repair = repair_on(&near, &[3]);
        assert_eq!(repair.lost_doors, vec![id("Docks.esp", 0x811)]);
    }

    #[test]
    fn paths_are_followed_from_mesh_to_mesh() {
        // The bank and the bridge in one mesh, the jetty in another, linked
        // from the bridge's far end.
        let mut bank = leads_out(grid(0x300, true, &[(0, 0), (1, 0)], SIDE), 0);
        let jetty = grid(0x301, true, &[(2, 0), (3, 0)], SIDE);
        if let Some(edges) = bank.edges.get_mut(2) {
            edges[1] = Beyond::Link {
                navmesh: id("Skyrim.esm", 0x301),
                triangle: 1,
            };
        }
        let near = near(vec![bank, jetty]);
        assert!(repair(&near, &[]).is_safe());
        let repair = repair_on(&near, &[1]);
        assert_eq!(
            repair.cut_off.first().map(|part| part.navmeshes.clone()),
            Some(BTreeSet::from([1]))
        );
    }

    #[test]
    fn versions_the_game_does_not_use_are_ignored() {
        let old = jetty();
        let mut used = two_bridges();
        used.id = old.id.clone();
        let near = near(vec![NavmeshShape { used: false, ..old }, used]);
        // The old version would lose its jetty; the used one keeps a way.
        let repair = repair(
            &near,
            &[carried(0, squares(&[1])), carried(1, squares(&[1]))],
        );
        assert_eq!(repair.cuts.keys().copied().collect::<Vec<_>>(), vec![1]);
        assert!(repair.is_safe());
    }

    #[test]
    fn inside_the_part_with_the_most_doors_is_where_characters_come_from() {
        // A hall with the entrance door, stairs, and a gallery with none.
        let mut hall = grid(0x400, true, &[(0, 0), (1, 0), (2, 0), (3, 0)], SIDE);
        hall.doors = vec![(0, id("Skyrim.esm", 0x900))];
        let repair = repair_on(&near(vec![hall]), &[1]);
        assert_eq!(repair.cut_off.len(), 1);
        let gallery = repair.cut_off.first().unwrap();
        assert_eq!(gallery.doors, []);
        assert!((gallery.centre[0] - 600.0).abs() < 1e-6);
    }

    #[test]
    fn a_part_whose_only_way_out_was_cut_is_cut_off() {
        // The only link out lies on the square that is taken out.
        let shape = leads_out(grid(0x500, true, &[(0, 0), (1, 0), (2, 0)], SIDE), 0);
        let repair = repair_on(&near(vec![shape]), &[0]);
        assert_eq!(repair.cut_off.len(), 1);
        assert_eq!(repair.cut_off.first().unwrap().triangles, 4);
    }
}
