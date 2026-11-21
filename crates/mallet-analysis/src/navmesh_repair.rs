//! Repairs the paths characters walk on where Mod Mallet's patch hides the
//! objects they ran over (ADR 0058).
//!
//! A navigation mesh that runs over a deck, a pier or a floor piece leaves
//! characters walking on air once the object is hidden (ADR 0051). The
//! repair does three things, in the versions of the meshes the game uses:
//!
//! 1. **It takes out the triangles on the hidden objects.** Nothing leads
//!    onto them or away from them any more, within their mesh or from a
//!    neighbouring one, and a door on them loses its way through. They
//!    keep their numbers, so that links of meshes the repair does not
//!    touch, door triangles and the search grid stay valid (ADR 0052);
//!    the writer also takes them out of the search grid.
//! 2. **It lays new paths on what lies below.** Where an object stood,
//!    the paths on the ground run around it. Once it is hidden there is
//!    ground there: outside the land, inside the floor the object stood
//!    on. The open edges of the paths around the object, at that height,
//!    are joined by new triangles laid over the object's place, through
//!    points on the land every 128 units outside. The new triangles join
//!    those edges, with links where an edge belongs to another mesh; under
//!    water they are marked as water, as the game's own are.
//! 3. **Where nothing walkable lies below, the paths go around.** No
//!    triangle is laid where paths already run, too steep for characters,
//!    away from the hidden object, or where it would join no path at all;
//!    what led onto the cut triangles ends there, and characters find their
//!    way around.
//!
//! The result keeps the triangle representation of the analyses
//! ([`NavmeshShape`]), so that whether everything is still reachable can be
//! checked on it; new triangles are numbered after the old ones.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use mallet_core::facts::RecordId;
use mallet_core::facts::navmeshes::{Beyond, NavmeshShape, NearHidden};
use mallet_core::facts::objects::Place;

use crate::contact::Point;
use crate::tin::{Tin, orient};
use crate::walkways::{Carried, Ground, Hidden, RAISED, Surface, cell_of};

/// How far from a hidden object, seen from above, new paths are laid and
/// the paths around it are joined, in game units. The Creation Kit keeps
/// its paths some distance from what stands in the way; at Falskaar's
/// harbour up to about 300 units around a ship's hull.
pub const MARGIN: f64 = 320.0;

/// How far above or below the land (outside) or the foot of the hidden
/// object (inside) an edge of the paths around it may lie to be joined.
pub const AT_THE_BASE: f64 = 64.0;

/// How far apart the points on the land are, outside: the land's own
/// spacing.
const LAND_STEP: f64 = 128.0;

/// How far from a hidden object, seen from above, points on the land are
/// laid: new paths reach a little beyond where it stood, and further only
/// to join the paths around.
const LAND_MARGIN: f64 = 64.0;

/// How close to a corner of the paths around a point on the land may lie.
const MIN_SPACING: f64 = 48.0;

/// How far above or below a path that is already there a new triangle
/// would lie on it.
const ON_A_PATH: f64 = 96.0;

/// The least share of a new triangle's slope that points up: about 60
/// degrees of slope, steeper than characters walk.
const MIN_UPWARD: f64 = 0.5;

/// New triangles smaller than this, in square game units, are left out.
const MIN_AREA: f64 = 1.0;

/// Below this, numbers count as zero.
const EPSILON: f64 = 1e-6;

/// What the repair makes of the navigation meshes around hidden objects.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PathsRepair {
    /// Every mesh that changes, in the order of [`NearHidden::navmeshes`].
    pub navmeshes: Vec<RepairedNavmesh>,
    /// The areas laid with new triangles, each joined to the paths
    /// around.
    pub laid: Vec<Laid>,
    /// How many hidden objects whose paths were taken out got none laid
    /// below them: characters go around.
    pub bare: usize,
    /// The doors whose triangles are taken out: characters can no longer
    /// step through them.
    pub lost_doors: Vec<RecordId>,
}

impl PathsRepair {
    /// How many triangles are taken out, in every mesh.
    pub fn removed(&self) -> usize {
        self.navmeshes.iter().map(|mesh| mesh.removed.len()).sum()
    }

    /// How many new triangles are laid, in every mesh.
    pub fn added(&self) -> usize {
        self.navmeshes.iter().map(|mesh| mesh.added.len()).sum()
    }
}

/// One navigation mesh after the repair.
#[derive(Debug, Clone, PartialEq)]
pub struct RepairedNavmesh {
    /// The version the game uses, by its place in [`NearHidden::navmeshes`].
    pub navmesh: usize,
    /// The mesh after the repair. Triangles in [`Self::removed`] keep
    /// their numbers but nothing leads onto them or away from them: they
    /// are no part of the paths. New triangles follow the old ones.
    pub after: NavmeshShape,
    /// The triangles taken out, by number.
    pub removed: BTreeSet<u32>,
    /// The new triangles, by number in [`Self::after`].
    pub added: Vec<u32>,
    /// Of the new triangles, those under water.
    pub water: BTreeSet<u32>,
}

/// An area of new triangles, joined to the paths around it.
#[derive(Debug, Clone, PartialEq)]
pub struct Laid {
    /// Where it lies: the worldspace or the cell.
    pub place: Place,
    /// Its middle, in the coordinates of the worldspace or the cell.
    pub centre: Point,
    /// How many new triangles it has.
    pub triangles: usize,
    /// How many edges of the paths around it joins.
    pub joined: usize,
}

/// Repairs the paths over the `hidden` objects: `carried` tells which
/// triangles of which meshes in `near` lie on them (see
/// [`crate::walkways::carried`]). Versions of meshes the game does not use
/// are left alone.
pub fn repair(near: &NearHidden, hidden: &[Hidden<'_>], carried: &[Carried]) -> PathsRepair {
    let mut work = Work::new(near);
    let mut stripped: BTreeSet<usize> = BTreeSet::new();
    for carried in carried {
        if near
            .navmeshes
            .get(carried.navmesh)
            .is_some_and(|shape| shape.used)
            && !carried.triangles.is_empty()
        {
            stripped.insert(carried.hidden);
            work.removed
                .entry(carried.navmesh)
                .or_default()
                .extend(carried.triangles.iter().copied());
        }
    }
    work.cut();

    // New paths are laid only where paths were taken out: under the
    // objects that carried them. A hidden rock nobody walked over needs
    // no path where it stood.
    let ground = Ground::new(near);
    let surfaces: Vec<(usize, &Place, Surface)> = hidden
        .iter()
        .enumerate()
        .filter(|(index, _)| stripped.contains(index))
        .filter_map(|(index, object)| Some((index, &object.object.place, Surface::of(object)?)))
        .collect();
    let places: BTreeSet<&Place> = surfaces.iter().map(|(_, place, _)| *place).collect();
    let mut laid = Vec::new();
    for place in places {
        let footprints: Vec<&Surface> = surfaces
            .iter()
            .filter(|(_, at, _)| *at == place)
            .map(|(_, _, surface)| surface)
            .collect();
        laid.extend(work.lay_place(place, &footprints, &ground));
    }
    // The objects stripped of paths that got none laid below them.
    let bare = surfaces
        .iter()
        .filter(|(index, place, surface)| {
            stripped.contains(index) && !work.laid_under(place, surface)
        })
        .count()
        + stripped
            .iter()
            .filter(|index| !surfaces.iter().any(|(known, _, _)| known == *index))
            .count();
    work.finish(laid, bare)
}

/// The meshes being repaired: copies of those that change.
struct Work<'a> {
    near: &'a NearHidden,
    /// The versions the game uses, by mesh.
    used: HashMap<&'a RecordId, usize>,
    /// Copies of the meshes that change, by their place in `near`.
    changed: BTreeMap<usize, NavmeshShape>,
    removed: BTreeMap<usize, BTreeSet<u32>>,
    added: BTreeMap<usize, Vec<u32>>,
    water: BTreeMap<usize, BTreeSet<u32>>,
    lost_doors: BTreeSet<RecordId>,
}

impl<'a> Work<'a> {
    fn new(near: &'a NearHidden) -> Self {
        Self {
            near,
            used: near
                .navmeshes
                .iter()
                .enumerate()
                .filter(|(_, shape)| shape.used)
                .map(|(index, shape)| (&shape.id, index))
                .collect(),
            changed: BTreeMap::new(),
            removed: BTreeMap::new(),
            added: BTreeMap::new(),
            water: BTreeMap::new(),
            lost_doors: BTreeSet::new(),
        }
    }

    /// The mesh as it is now.
    fn shape(&self, index: usize) -> Option<&NavmeshShape> {
        self.changed
            .get(&index)
            .or_else(|| self.near.navmeshes.get(index))
    }

    /// The mesh, to be changed.
    fn edit(&mut self, index: usize) -> Option<&mut NavmeshShape> {
        if !self.changed.contains_key(&index) {
            let shape = self.near.navmeshes.get(index)?.clone();
            self.changed.insert(index, shape);
        }
        self.changed.get_mut(&index)
    }

    fn is_removed(&self, index: usize, triangle: u32) -> bool {
        self.removed
            .get(&index)
            .is_some_and(|removed| removed.contains(&triangle))
    }

    /// The meshes the game uses in `place`, in the order of `near`.
    fn used_in(&self, place: &Place) -> Vec<usize> {
        let mut meshes: Vec<usize> = self
            .used
            .values()
            .copied()
            .filter(|index| {
                self.near
                    .navmeshes
                    .get(*index)
                    .is_some_and(|shape| &shape.place == place)
            })
            .collect();
        meshes.sort_unstable();
        meshes
    }

    /// Takes the triangles in `removed` out of the paths.
    fn cut(&mut self) {
        let cut_ids: HashMap<RecordId, BTreeSet<u32>> = self
            .removed
            .iter()
            .filter_map(|(index, triangles)| {
                Some((
                    self.near.navmeshes.get(*index)?.id.clone(),
                    triangles.clone(),
                ))
            })
            .collect();
        let mut used: Vec<usize> = self.used.values().copied().collect();
        used.sort_unstable();
        for index in used {
            let own = self.removed.get(&index).cloned().unwrap_or_default();
            let Some(shape) = self.shape(index) else {
                continue;
            };
            let leads_onto_cut = |beyond: &Beyond| match beyond {
                Beyond::Nothing => false,
                Beyond::Triangle(other) => own.contains(other),
                Beyond::Link { navmesh, triangle } => cut_ids
                    .get(navmesh)
                    .is_some_and(|cut| cut.contains(triangle)),
            };
            let touched = !own.is_empty() || shape.edges.iter().flatten().any(leads_onto_cut);
            if !touched {
                continue;
            }
            let Some(shape) = self.edit(index) else {
                continue;
            };
            for (number, edges) in (0u32..).zip(shape.edges.iter_mut()) {
                for edge in edges.iter_mut() {
                    if own.contains(&number) || leads_onto_cut(edge) {
                        *edge = Beyond::Nothing;
                    }
                }
            }
            let mut lost = Vec::new();
            shape.doors.retain(|(triangle, door)| {
                let keep = !own.contains(triangle);
                if !keep {
                    lost.push(door.clone());
                }
                keep
            });
            self.lost_doors.extend(lost);
        }
    }

    /// Lays new paths under the hidden objects in `place`, whose surfaces
    /// are `footprints`, joined to the paths around them.
    fn lay_place(
        &mut self,
        place: &Place,
        footprints: &[&Surface],
        ground: &Ground<'_>,
    ) -> Vec<Laid> {
        let world = match place {
            Place::Exterior { worldspace } => Some(worldspace),
            Place::Interior { .. } => None,
        };
        let land = |x: f64, y: f64| world.and_then(|world| ground.height(world, x, y));
        let edges = self.open_edges(place, footprints, &land);
        let paths = self.paths_near(place, footprints);

        // The corners of the open edges, once each, and the points on the
        // land between them.
        let mut points: Vec<Point> = Vec::new();
        let mut known: HashMap<[i32; 3], usize> = HashMap::new();
        let mut ends = Vec::with_capacity(edges.len());
        for edge in &edges {
            let mut index_of = |point: Point| {
                *known.entry(whole_point(point)).or_insert_with(|| {
                    points.push(point);
                    points.len() - 1
                })
            };
            ends.push((index_of(edge.from), index_of(edge.to)));
        }
        if world.is_some() {
            for point in land_points(footprints, &land) {
                let crowded = points
                    .iter()
                    .any(|other| (other[0] - point[0]).hypot(other[1] - point[1]) < MIN_SPACING);
                if !crowded && !on_a_path(&paths, point) {
                    points.push(point);
                }
            }
        }
        let Some(mut tin) = Tin::new(points) else {
            return Vec::new();
        };
        for (from, to) in &ends {
            let _ = tin.keep(*from, *to);
        }
        tin.even_out();

        // The sides of the paths around, by the directed edge a new
        // triangle beyond them has.
        let sides: HashMap<(usize, usize), &OpenEdge> = ends
            .iter()
            .zip(&edges)
            .map(|(&(from, to), edge)| ((from, to), edge))
            .collect();
        let kept: Vec<[usize; 3]> = tin
            .triangles()
            .into_iter()
            .filter(|triangle| worth_laying(&tin.points, triangle, footprints, &sides, &paths))
            .collect();
        // Only areas that join the paths around: others nobody reaches.
        let groups = joined_groups(&kept, &sides);
        let mut areas = Vec::new();
        for group in groups {
            let triangles: Vec<[usize; 3]> = group
                .iter()
                .filter_map(|index| kept.get(*index).copied())
                .collect();
            let joined = triangles
                .iter()
                .flat_map(directed_edges)
                .filter(|edge| sides.contains_key(edge))
                .count();
            let middles: Vec<Point> = triangles
                .iter()
                .filter_map(|triangle| Some(centroid(&corners_of(&tin.points, triangle)?)))
                .collect();
            self.lay(
                place,
                &tin.points,
                &triangles,
                &sides,
                world.map(|world| (world, ground)),
            );
            areas.push(Laid {
                place: place.clone(),
                centre: centroid(&middles),
                triangles: triangles.len(),
                joined,
            });
        }
        areas
    }

    /// The open edges of the paths in `place` near the hidden objects, at
    /// the height of what they stood on, each turned so that the open side
    /// lies on its left.
    fn open_edges(
        &self,
        place: &Place,
        footprints: &[&Surface],
        land: &dyn Fn(f64, f64) -> Option<f64>,
    ) -> Vec<OpenEdge> {
        let at_the_base = |[x, y, z]: Point| match land(x, y) {
            Some(ground) => (z - ground).abs() <= AT_THE_BASE,
            None => footprints.iter().any(|surface| {
                let (min, max) = surface.bounds();
                x >= min[0] - MARGIN
                    && x <= max[0] + MARGIN
                    && y >= min[1] - MARGIN
                    && y <= max[1] + MARGIN
                    && z >= min[2] - AT_THE_BASE
                    && z <= min[2] + AT_THE_BASE + RAISED
            }),
        };
        let mut found = Vec::new();
        for index in self.used_in(place) {
            let Some(shape) = self.shape(index) else {
                continue;
            };
            for (number, (corners, edges)) in (0u32..).zip(shape.triangles.iter().zip(&shape.edges))
            {
                if self.is_removed(index, number) {
                    continue;
                }
                for (edge, beyond) in edges.iter().enumerate() {
                    if *beyond != Beyond::Nothing {
                        continue;
                    }
                    let (Some(&start), Some(&end)) =
                        (corners.get(edge), corners.get((edge + 1) % 3))
                    else {
                        continue;
                    };
                    let (Some(from), Some(to)) = (vertex(shape, start), vertex(shape, end)) else {
                        continue;
                    };
                    let middle = midpoint(from, to);
                    let near_hidden = footprints
                        .iter()
                        .any(|surface| surface.covers(middle[0], middle[1], MARGIN));
                    if !near_hidden || !at_the_base(from) || !at_the_base(to) {
                        continue;
                    }
                    // The triangle lies left of its edge; the open side right.
                    found.push(OpenEdge {
                        navmesh: index,
                        triangle: number,
                        edge,
                        from: to,
                        to: from,
                        from_vertex: end,
                        to_vertex: start,
                    });
                }
            }
        }
        found
    }

    /// The triangles of the paths in `place` that stay, near the hidden
    /// objects.
    fn paths_near(&self, place: &Place, footprints: &[&Surface]) -> Vec<[Point; 3]> {
        let reach = 2.0 * MARGIN;
        let near = |point: Point| {
            footprints.iter().any(|surface| {
                let (min, max) = surface.bounds();
                point[0] >= min[0] - reach
                    && point[0] <= max[0] + reach
                    && point[1] >= min[1] - reach
                    && point[1] <= max[1] + reach
            })
        };
        let mut found = Vec::new();
        for index in self.used_in(place) {
            let Some(shape) = self.shape(index) else {
                continue;
            };
            for (number, corners) in (0u32..).zip(&shape.triangles) {
                if self.is_removed(index, number) {
                    continue;
                }
                let (Some(a), Some(b), Some(c)) = (
                    vertex(shape, corners[0]),
                    vertex(shape, corners[1]),
                    vertex(shape, corners[2]),
                ) else {
                    continue;
                };
                if near(a) || near(b) || near(c) {
                    found.push([a, b, c]);
                }
            }
        }
        found
    }

    /// Writes the new triangles into the meshes, joined to the `sides` of
    /// the paths around and to each other.
    fn lay(
        &mut self,
        place: &Place,
        points: &[Point],
        triangles: &[[usize; 3]],
        sides: &HashMap<(usize, usize), &OpenEdge>,
        water: Option<(&RecordId, &Ground<'_>)>,
    ) {
        let owners = self.owners(place, points, triangles, sides);
        // Numbers of the new triangles.
        let mut numbers = Vec::with_capacity(triangles.len());
        let mut next: BTreeMap<usize, u32> = BTreeMap::new();
        for owner in &owners {
            let number = match next.get(owner) {
                Some(number) => *number,
                None => self
                    .shape(*owner)
                    .and_then(|shape| u32::try_from(shape.triangles.len()).ok())
                    .unwrap_or(u32::MAX),
            };
            numbers.push(number);
            next.insert(*owner, number.saturating_add(1));
        }
        // The vertices the meshes already have at the corners.
        let mut vertices: HashMap<(usize, usize), u32> = HashMap::new();
        let mut ordered: Vec<(&(usize, usize), &&OpenEdge)> = sides.iter().collect();
        ordered.sort_by_key(|(ends, _)| **ends);
        for (&(from, to), side) in ordered {
            vertices.insert((side.navmesh, from), side.from_vertex);
            vertices.insert((side.navmesh, to), side.to_vertex);
        }
        // Which new triangle lies beyond each directed edge.
        let mut beyond_edge: HashMap<(usize, usize), usize> = HashMap::new();
        for (index, triangle) in triangles.iter().enumerate() {
            for edge in directed_edges(triangle) {
                beyond_edge.insert(edge, index);
            }
        }
        let ids: BTreeMap<usize, RecordId> = owners
            .iter()
            .chain(sides.values().map(|side| &side.navmesh))
            .filter_map(|index| Some((*index, self.shape(*index)?.id.clone())))
            .collect();
        let beyond = |from: usize, to: usize, triangle: u32| -> Option<Beyond> {
            if from == to {
                Some(Beyond::Triangle(triangle))
            } else {
                Some(Beyond::Link {
                    navmesh: ids.get(&to)?.clone(),
                    triangle,
                })
            }
        };

        for ((triangle, owner), number) in triangles.iter().zip(&owners).zip(&numbers) {
            let mut corners = [0u32; 3];
            for (slot, point) in corners.iter_mut().zip(triangle) {
                if let Some(vertex) = vertices.get(&(*owner, *point)) {
                    *slot = *vertex;
                    continue;
                }
                let at = points.get(*point).copied().unwrap_or_default();
                let Some(shape) = self.edit(*owner) else {
                    continue;
                };
                let vertex = u32::try_from(shape.vertices.len()).unwrap_or(u32::MAX);
                shape.vertices.push(whole_point(at));
                vertices.insert((*owner, *point), vertex);
                *slot = vertex;
            }
            let mut edges = [Beyond::Nothing, Beyond::Nothing, Beyond::Nothing];
            for (slot, (a, b)) in edges.iter_mut().zip(directed_edges(triangle)) {
                if let Some(other) = beyond_edge.get(&(b, a)) {
                    if let (Some(other_owner), Some(other_number)) =
                        (owners.get(*other), numbers.get(*other))
                        && let Some(link) = beyond(*owner, *other_owner, *other_number)
                    {
                        *slot = link;
                    }
                } else if let Some(side) = sides.get(&(a, b)) {
                    if let Some(link) = beyond(*owner, side.navmesh, side.triangle) {
                        *slot = link;
                    }
                    if let Some(back) = beyond(side.navmesh, *owner, *number)
                        && let Some(shape) = self.edit(side.navmesh)
                        && let Some(old) = usize::try_from(side.triangle)
                            .ok()
                            .and_then(|index| shape.edges.get_mut(index))
                            .and_then(|edges| edges.get_mut(side.edge))
                    {
                        *old = back;
                    }
                }
            }
            let in_water = water.is_some_and(|(world, ground)| {
                corners_of(points, triangle).is_some_and(|corners| {
                    let [x, y, z] = centroid(&corners);
                    ground.water(world, x, y).is_some_and(|surface| z < surface)
                })
            });
            let Some(shape) = self.edit(*owner) else {
                continue;
            };
            shape.triangles.push(corners);
            shape.edges.push(edges);
            self.added.entry(*owner).or_default().push(*number);
            if in_water {
                self.water.entry(*owner).or_default().insert(*number);
            }
        }
    }

    /// Which mesh each new triangle goes into. Outside, a mesh of the
    /// cell its middle lies in, the one that has the most edges it joins;
    /// inside, and where the cell has none, the mesh with the most such
    /// edges.
    fn owners(
        &self,
        place: &Place,
        points: &[Point],
        triangles: &[[usize; 3]],
        sides: &HashMap<(usize, usize), &OpenEdge>,
    ) -> Vec<usize> {
        let mut counts: BTreeMap<usize, usize> = BTreeMap::new();
        for side in sides.values() {
            *counts.entry(side.navmesh).or_default() += 1;
        }
        let most = counts
            .iter()
            .max_by_key(|(index, count)| (**count, std::cmp::Reverse(**index)))
            .map_or(0, |(index, _)| *index);
        let outside = matches!(place, Place::Exterior { .. });
        // The meshes of each outside cell, by the cell of their middle.
        let mut by_cell: BTreeMap<(i32, i32), Vec<usize>> = BTreeMap::new();
        if outside {
            for index in self.used_in(place) {
                if let Some(cell) = self.shape(index).and_then(cell_of_mesh) {
                    by_cell.entry(cell).or_default().push(index);
                }
            }
        }
        triangles
            .iter()
            .map(|triangle| {
                let Some(corners) = corners_of(points, triangle) else {
                    return most;
                };
                let [x, y, _] = centroid(&corners);
                let Some(meshes) = by_cell.get(&(cell_of(x), cell_of(y))) else {
                    return most;
                };
                meshes
                    .iter()
                    .max_by_key(|index| {
                        (
                            counts.get(*index).copied().unwrap_or_default(),
                            self.shape(**index).map_or(0, |shape| shape.triangles.len()),
                        )
                    })
                    .copied()
                    .unwrap_or(most)
            })
            .collect()
    }

    /// Whether new triangles were laid under `surface` in `place`.
    fn laid_under(&self, place: &Place, surface: &Surface) -> bool {
        self.added.iter().any(|(index, numbers)| {
            let Some(shape) = self.shape(*index) else {
                return false;
            };
            &shape.place == place
                && numbers.iter().any(|number| {
                    let Some(corners) = usize::try_from(*number)
                        .ok()
                        .and_then(|number| shape.triangles.get(number))
                    else {
                        return false;
                    };
                    let points: Vec<Point> = corners
                        .iter()
                        .filter_map(|corner| vertex(shape, *corner))
                        .collect();
                    let [x, y, _] = centroid(&points);
                    surface.covers(x, y, 0.0)
                })
        })
    }

    fn finish(self, laid: Vec<Laid>, bare: usize) -> PathsRepair {
        let Self {
            changed,
            mut removed,
            mut added,
            mut water,
            lost_doors,
            ..
        } = self;
        PathsRepair {
            navmeshes: changed
                .into_iter()
                .map(|(navmesh, after)| RepairedNavmesh {
                    navmesh,
                    after,
                    removed: removed.remove(&navmesh).unwrap_or_default(),
                    added: added.remove(&navmesh).unwrap_or_default(),
                    water: water.remove(&navmesh).unwrap_or_default(),
                })
                .collect(),
            laid,
            bare,
            lost_doors: lost_doors.into_iter().collect(),
        }
    }
}

/// An open edge of the paths, turned so that its open side lies on its
/// left.
#[derive(Debug, Clone)]
struct OpenEdge {
    navmesh: usize,
    triangle: u32,
    /// Which edge of the triangle it is.
    edge: usize,
    from: Point,
    to: Point,
    /// The vertices of `from` and `to` in the mesh.
    from_vertex: u32,
    to_vertex: u32,
}

/// Whether a new triangle through `points` is worth laying: large enough,
/// not too steep, near the hidden objects with all its corners and its
/// edges but those of the paths around (so no long slivers out to far
/// corners), and not on a path that is there already.
fn worth_laying(
    points: &[Point],
    triangle: &[usize; 3],
    footprints: &[&Surface],
    sides: &HashMap<(usize, usize), &OpenEdge>,
    paths: &[[Point; 3]],
) -> bool {
    let Some(corners) = corners_of(points, triangle) else {
        return false;
    };
    let [a, b, c] = corners;
    let middle = centroid(&corners);
    let near_hidden = |point: Point| {
        footprints
            .iter()
            .any(|surface| surface.covers(point[0], point[1], MARGIN))
    };
    let edges_near = directed_edges(triangle).iter().all(|&(u, v)| {
        sides.contains_key(&(u, v))
            || points
                .get(u)
                .zip(points.get(v))
                .is_some_and(|(pu, pv)| near_hidden(midpoint(*pu, *pv)))
    });
    orient(a, b, c) / 2.0 >= MIN_AREA
        && upward(corners) >= MIN_UPWARD
        && corners.iter().all(|corner| near_hidden(*corner))
        && near_hidden(middle)
        && edges_near
        && !on_a_path(paths, middle)
        && ![(a, b), (b, c), (c, a)]
            .iter()
            .any(|(u, v)| on_a_path(paths, midpoint(*u, *v)))
}

/// Points on the land every [`LAND_STEP`] units under the `footprints`,
/// outside.
fn land_points(footprints: &[&Surface], land: &dyn Fn(f64, f64) -> Option<f64>) -> Vec<Point> {
    let mut seen: HashSet<(i64, i64)> = HashSet::new();
    let mut found = Vec::new();
    for surface in footprints {
        let (min, max) = surface.bounds();
        let first = |value: f64| (value / LAND_STEP).floor();
        let (west, south) = (first(min[0] - MARGIN), first(min[1] - MARGIN));
        let (east, north) = (first(max[0] + MARGIN) + 1.0, first(max[1] + MARGIN) + 1.0);
        let mut column = west;
        while column <= east {
            let mut row = south;
            while row <= north {
                let (x, y) = (column * LAND_STEP, row * LAND_STEP);
                if seen.insert((whole(column), whole(row)))
                    && surface.covers(x, y, LAND_MARGIN)
                    && let Some(height) = land(x, y)
                {
                    found.push([x, y, height]);
                }
                row += 1.0;
            }
            column += 1.0;
        }
    }
    found
}

/// Whether `point` lies on one of `paths`, seen from above and within a
/// step in height.
fn on_a_path(paths: &[[Point; 3]], point: Point) -> bool {
    paths.iter().any(|[a, b, c]| {
        let inside = orient(*a, *b, point) > EPSILON
            && orient(*b, *c, point) > EPSILON
            && orient(*c, *a, point) > EPSILON;
        inside
            && height_in(*a, *b, *c, point)
                .is_some_and(|height| (height - point[2]).abs() < ON_A_PATH)
    })
}

/// The height of the plane through a triangle at a point, seen from
/// above.
fn height_in(a: Point, b: Point, c: Point, point: Point) -> Option<f64> {
    let total = orient(a, b, c);
    if total.abs() < EPSILON {
        return None;
    }
    let first = orient(b, c, point) / total;
    let second = orient(c, a, point) / total;
    let third = 1.0 - first - second;
    Some(first * a[2] + second * b[2] + third * c[2])
}

/// The groups of `triangles` that hang together and join at least one of
/// the `sides`, as lists of their places in `triangles`.
fn joined_groups(
    triangles: &[[usize; 3]],
    sides: &HashMap<(usize, usize), &OpenEdge>,
) -> Vec<Vec<usize>> {
    let mut beyond_edge: HashMap<(usize, usize), usize> = HashMap::new();
    for (index, triangle) in triangles.iter().enumerate() {
        for edge in directed_edges(triangle) {
            beyond_edge.insert(edge, index);
        }
    }
    let mut group_of = vec![usize::MAX; triangles.len()];
    let mut groups = Vec::new();
    for start in 0..triangles.len() {
        if group_of.get(start).copied() != Some(usize::MAX) {
            continue;
        }
        let number = groups.len();
        let mut members = Vec::new();
        let mut stack = vec![start];
        while let Some(index) = stack.pop() {
            match group_of.get_mut(index) {
                Some(slot) if *slot == usize::MAX => *slot = number,
                _ => continue,
            }
            members.push(index);
            let Some(triangle) = triangles.get(index) else {
                continue;
            };
            for (a, b) in directed_edges(triangle) {
                if let Some(other) = beyond_edge.get(&(b, a)) {
                    stack.push(*other);
                }
            }
        }
        groups.push(members);
    }
    groups
        .into_iter()
        .filter(|members| {
            members.iter().any(|index| {
                triangles.get(*index).is_some_and(|triangle| {
                    directed_edges(triangle)
                        .iter()
                        .any(|edge| sides.contains_key(edge))
                })
            })
        })
        .map(|mut members| {
            members.sort_unstable();
            members
        })
        .collect()
}

/// The three edges of a triangle, each from a corner to the next.
fn directed_edges(triangle: &[usize; 3]) -> [(usize, usize); 3] {
    let [a, b, c] = *triangle;
    [(a, b), (b, c), (c, a)]
}

fn corners_of(points: &[Point], triangle: &[usize; 3]) -> Option<[Point; 3]> {
    Some([
        points.get(triangle[0]).copied()?,
        points.get(triangle[1]).copied()?,
        points.get(triangle[2]).copied()?,
    ])
}

/// How much of a triangle's upward direction points up: 1 for flat.
fn upward([a, b, c]: [Point; 3]) -> f64 {
    let (along, across) = (
        [b[0] - a[0], b[1] - a[1], b[2] - a[2]],
        [c[0] - a[0], c[1] - a[1], c[2] - a[2]],
    );
    let normal = [
        along[1] * across[2] - along[2] * across[1],
        along[2] * across[0] - along[0] * across[2],
        along[0] * across[1] - along[1] * across[0],
    ];
    let length = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
    if length < EPSILON {
        0.0
    } else {
        normal[2] / length
    }
}

fn centroid(points: &[Point]) -> Point {
    #[allow(clippy::cast_precision_loss)]
    let count = points.len().max(1) as f64;
    let sum = points.iter().fold([0.0; 3], |sum, point| {
        [sum[0] + point[0], sum[1] + point[1], sum[2] + point[2]]
    });
    sum.map(|value| value / count)
}

fn midpoint(a: Point, b: Point) -> Point {
    [
        f64::midpoint(a[0], b[0]),
        f64::midpoint(a[1], b[1]),
        f64::midpoint(a[2], b[2]),
    ]
}

/// A vertex of a mesh.
fn vertex(shape: &NavmeshShape, index: u32) -> Option<Point> {
    let [x, y, z] = *shape.vertices.get(usize::try_from(index).ok()?)?;
    Some([f64::from(x), f64::from(y), f64::from(z)])
}

/// The outside cell a mesh lies in: that of the middle of the box around
/// its vertices.
fn cell_of_mesh(shape: &NavmeshShape) -> Option<(i32, i32)> {
    let first = shape.vertices.first()?;
    let start = [f64::from(first[0]), f64::from(first[1])];
    let (min, max) = shape
        .vertices
        .iter()
        .fold((start, start), |(min, max), vertex| {
            let (x, y) = (f64::from(vertex[0]), f64::from(vertex[1]));
            (
                [min[0].min(x), min[1].min(y)],
                [max[0].max(x), max[1].max(y)],
            )
        });
    Some((
        cell_of(f64::midpoint(min[0], max[0])),
        cell_of(f64::midpoint(min[1], max[1])),
    ))
}

/// A point in whole game units.
fn whole_point(point: Point) -> [i32; 3] {
    point.map(|value| i32::try_from(whole(value)).unwrap_or_default())
}

/// A number rounded to a whole one.
#[allow(clippy::cast_possible_truncation)]
fn whole(value: f64) -> i64 {
    // Bounded before converting; coordinates of the games are far inside.
    value.round().clamp(-1.0e9, 1.0e9) as i64
}

#[cfg(test)]
mod tests {
    use mallet_core::facts::navmeshes::GroundCell;
    use mallet_core::facts::objects::{Bounds, PlacedObject};
    use proptest::prelude::*;

    use super::*;
    use crate::walkways::carried;

    fn id(plugin: &str, local_id: u32) -> RecordId {
        RecordId {
            plugin: plugin.to_lowercase(),
            local_id,
        }
    }

    fn world() -> RecordId {
        id("Skyrim.esm", 0x3C)
    }

    fn outside() -> Place {
        Place::Exterior {
            worldspace: world(),
        }
    }

    /// A mesh with the given vertices and triangles; the edges are found
    /// where two triangles share one.
    fn mesh(local_id: u32, vertices: Vec<[i32; 3]>, triangles: Vec<[u32; 3]>) -> NavmeshShape {
        let mut edges = vec![[Beyond::Nothing, Beyond::Nothing, Beyond::Nothing]; triangles.len()];
        for (index, triangle) in triangles.iter().enumerate() {
            for edge in 0..3 {
                let (a, b) = (triangle[edge], triangle[(edge + 1) % 3]);
                if let Some(other) = triangles
                    .iter()
                    .position(|other| (0..3).any(|k| other[k] == b && other[(k + 1) % 3] == a))
                {
                    edges[index][edge] = Beyond::Triangle(u32::try_from(other).unwrap());
                }
            }
        }
        NavmeshShape {
            id: id("Skyrim.esm", local_id),
            plugin: "Docks.esp".to_owned(),
            used: true,
            place: outside(),
            place_name: Some("Tamriel 0,0".to_owned()),
            vertices,
            triangles,
            edges,
            doors: Vec::new(),
        }
    }

    /// Ground around a square hole from 400 to 600, and a deck at 200 over
    /// it that leads nowhere: triangles 8 and 9, with a door on 9.
    fn ring_and_deck() -> NavmeshShape {
        let vertices = vec![
            [0, 0, 0],
            [1000, 0, 0],
            [1000, 1000, 0],
            [0, 1000, 0],
            [400, 400, 0],
            [600, 400, 0],
            [600, 600, 0],
            [400, 600, 0],
            [420, 420, 200],
            [580, 420, 200],
            [580, 580, 200],
            [420, 580, 200],
        ];
        let triangles = vec![
            [0, 1, 5],
            [0, 5, 4],
            [1, 2, 6],
            [1, 6, 5],
            [2, 3, 7],
            [2, 7, 6],
            [3, 0, 4],
            [3, 4, 7],
            [8, 9, 10],
            [8, 10, 11],
        ];
        let mut shape = mesh(0x100, vertices, triangles);
        shape.doors = vec![(9, id("Docks.esp", 0x900))];
        shape
    }

    /// A box 180 units wide whose top is 190 units above the ground, in
    /// the middle of the hole.
    fn hull() -> (PlacedObject, Bounds) {
        let object = PlacedObject {
            id: id("Docks.esp", 0x800),
            plugin: "Docks.esp".to_owned(),
            base: id("Docks.esp", 0x801),
            place: outside(),
            position: [500, 500, 0],
            rotation: [0, 0, 0],
            scale: 1000,
            persistent: false,
            large: false,
            moved: None,
        };
        let bounds = Bounds {
            min: [-90, -90, 0],
            max: [90, 90, 190],
        };
        (object, bounds)
    }

    fn run(near: &NearHidden) -> PathsRepair {
        let (object, bounds) = hull();
        let hidden = [Hidden {
            object: &object,
            mesh: None,
            bounds: Some(&bounds),
        }];
        let carried = carried(&hidden, near);
        repair(near, &hidden, &carried)
    }

    /// The meshes on flat land at height 0, with water at `water`.
    fn near(navmeshes: Vec<NavmeshShape>, water: Option<i32>) -> NearHidden {
        NearHidden {
            navmeshes,
            ground: vec![GroundCell {
                worldspace: world(),
                x: 0,
                y: 0,
                heights: vec![0; 33 * 33],
                water,
            }],
        }
    }

    #[test]
    fn the_deck_is_taken_out_and_the_hole_below_is_laid() {
        let near = near(vec![ring_and_deck()], None);
        let repair = run(&near);
        assert_eq!(repair.navmeshes.len(), 1);
        let mesh = &repair.navmeshes[0];
        assert_eq!(mesh.removed, BTreeSet::from([8, 9]));
        // The hole and the point on the land in its middle: four triangles.
        assert_eq!(mesh.added, vec![10, 11, 12, 13]);
        assert!(mesh.water.is_empty());
        assert_eq!(repair.laid.len(), 1);
        assert_eq!(repair.laid[0].joined, 4);
        assert_eq!(repair.bare, 0);
        assert_eq!(repair.lost_doors, vec![id("Docks.esp", 0x900)]);

        let after = &mesh.after;
        // The deck keeps its place but leads nowhere, and its door is gone.
        assert_eq!(after.triangles.len(), 14);
        assert_eq!(
            after.edges[8],
            [Beyond::Nothing, Beyond::Nothing, Beyond::Nothing]
        );
        assert_eq!(after.doors, Vec::new());
        // The ground's inner edges lead onto the new triangles, and back.
        for ground in [1usize, 3, 5, 7] {
            let Beyond::Triangle(new) = after.edges[ground][1] else {
                panic!("triangle {ground} leads nowhere");
            };
            assert!(new >= 10);
            let back = Beyond::Triangle(u32::try_from(ground).unwrap());
            assert!(after.edges[new as usize].contains(&back));
        }
        // New triangles lie on the land, anticlockwise.
        for number in &mesh.added {
            let corners = after.triangles[*number as usize].map(|v| after.vertices[v as usize]);
            assert!(corners.iter().all(|corner| corner[2] == 0));
            let [a, b, c] = corners.map(|v| [f64::from(v[0]), f64::from(v[1]), 0.0]);
            assert!(orient(a, b, c) > 0.0);
        }
        // One new vertex: the point on the land in the middle.
        assert_eq!(after.vertices.len(), 13);
        assert_eq!(after.vertices[12], [512, 512, 0]);
    }

    #[test]
    fn under_water_the_new_triangles_are_water() {
        let repair = run(&near(vec![ring_and_deck()], Some(50)));
        let mesh = &repair.navmeshes[0];
        assert_eq!(mesh.water.len(), mesh.added.len());
        assert!(!mesh.water.is_empty());
    }

    #[test]
    fn where_nothing_is_around_nothing_is_laid_and_paths_go_around() {
        // Only the deck, and a pier of another mesh that leads onto it.
        let full = ring_and_deck();
        let mut deck = mesh(0x100, full.vertices.clone(), vec![[8, 9, 10], [8, 10, 11]]);
        deck.doors.clear();
        let mut pier = mesh(
            0x200,
            vec![
                [420, 300, 200],
                [580, 300, 200],
                [580, 420, 200],
                [420, 420, 200],
            ],
            vec![[0, 1, 2], [0, 2, 3]],
        );
        pier.edges[1][1] = Beyond::Link {
            navmesh: id("Skyrim.esm", 0x100),
            triangle: 0,
        };
        let repair = run(&near(vec![deck, pier], None));
        assert_eq!(repair.removed(), 2);
        assert_eq!(repair.added(), 0);
        assert_eq!(repair.bare, 1);
        // The pier no longer leads onto the deck, and loses nothing.
        let pier = repair
            .navmeshes
            .iter()
            .find(|mesh| mesh.navmesh == 1)
            .unwrap();
        assert_eq!(pier.after.edges[1][1], Beyond::Nothing);
        assert!(pier.removed.is_empty());
    }

    #[test]
    fn a_hole_across_two_meshes_is_joined_with_links_both_ways() {
        // The ring's left side in a second mesh.
        let full = ring_and_deck();
        let mut triangles = full.triangles.clone();
        let left_side: Vec<[u32; 3]> = triangles.drain(6..8).collect();
        let right = mesh(0x100, full.vertices.clone(), triangles);
        let left = mesh(0x300, full.vertices.clone(), left_side);
        let repair = run(&near(vec![right, left], None));
        assert_eq!(repair.added(), 4);
        let mut links = 0;
        for mesh in &repair.navmeshes {
            for (number, edges) in mesh.after.edges.iter().enumerate() {
                for edge in edges {
                    let Beyond::Link { navmesh, triangle } = edge else {
                        continue;
                    };
                    links += 1;
                    let other = repair
                        .navmeshes
                        .iter()
                        .find(|other| &other.after.id == navmesh)
                        .unwrap();
                    let back = Beyond::Link {
                        navmesh: mesh.after.id.clone(),
                        triangle: u32::try_from(number).unwrap(),
                    };
                    assert!(other.after.edges[*triangle as usize].contains(&back));
                }
            }
        }
        assert!(links >= 2);
    }

    #[test]
    fn versions_the_game_does_not_use_are_left_alone() {
        let mut unused = ring_and_deck();
        unused.used = false;
        assert_eq!(run(&near(vec![unused], None)), PathsRepair::default());
    }

    proptest! {
        #[test]
        fn never_panics_on_odd_meshes(
            vertices in prop::collection::vec(any::<[i16; 3]>(), 0..12),
            triangles in prop::collection::vec(any::<[u8; 3]>(), 0..12),
        ) {
            let vertices: Vec<[i32; 3]> = vertices
                .iter()
                .map(|v| [i32::from(v[0]) / 32 + 500, i32::from(v[1]) / 32 + 500, i32::from(v[2]) / 64])
                .collect();
            let triangles: Vec<[u32; 3]> = triangles.iter().map(|t| t.map(|v| u32::from(v) % 14)).collect();
            let _ = run(&near(vec![mesh(0x100, vertices, triangles)], Some(0)));
        }
    }
}
