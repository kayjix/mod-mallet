//! Paths characters walk on that run over objects Mod Mallet's patch hides
//! (ADR 0051).
//!
//! Base Object Swapper hides an object by showing an invisible marker in
//! its place: what is drawn and what stops characters are gone, but the
//! navigation meshes the game's characters find their way on stay as they
//! were made. Where such a mesh runs over a hidden deck, pier, bridge or
//! rock, characters walk on air or get stuck at its edge.
//!
//! A triangle of a navigation mesh runs over a hidden object when the
//! object's surface lies just under its middle: the game lays its meshes a
//! little above what characters stand on. Outside, a surface that is
//! hardly above the ground carries nobody the ground would not carry as
//! well, so it does not count; nor does an object as flat as a rug, which
//! lies on something that carries as well. The surface is that of the
//! object's model where it was read, else that of its box.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use mallet_core::facts::RecordId;
use mallet_core::facts::models::ModelMesh;
use mallet_core::facts::navmeshes::{NavmeshShape, NearHidden};
use mallet_core::facts::objects::{BaseObject, Bounds, Place, PlacedObject};
use mallet_core::{Facts, Finding, GamePath, Party, Subject, kinds};

use crate::contact::Point;
use crate::objects::placed_points;
use crate::{Analysis, Context, examples, reachability};

/// The file Base Object Swapper reads Mod Mallet's patch from (ADR 0025).
const PATCH_FILE: &str = "data/ModMallet_SWAP.ini";

/// How far above the surface that carries it a navigation mesh may lie,
/// in game units.
const ABOVE: f64 = 48.0;

/// How far below the surface it may lie.
const BELOW: f64 = 16.0;

/// How far above the ground a surface must be to carry anybody the ground
/// would not carry anyway: about a step. An object less high than this
/// lies on something that carries as well, as a rug lies on a floor.
pub(crate) const RAISED: f64 = 32.0;

/// The side of the squares the surface's triangles are sorted into.
const SQUARE: f64 = 64.0;

/// Most squares along a side; a larger object gets larger squares.
const MAX_SQUARES: f64 = 512.0;

/// Game units per side of an outside cell, and between its land heights.
const CELL: f64 = 4096.0;
const GROUND_STEP: f64 = 128.0;
const GROUND_POINTS: usize = 33;

/// How many names and places a finding gives.
const EXAMPLE_LIMIT: usize = 5;

/// Below this, numbers count as zero.
const EPSILON: f64 = 1e-9;

/// An object to be hidden, with what is known of its shape.
pub struct Hidden<'a> {
    /// The object as it is placed.
    pub object: &'a PlacedObject,
    /// Its model's surface, if it was read.
    pub mesh: Option<&'a ModelMesh>,
    /// The box around its model, if known: the surface where the model
    /// was not read.
    pub bounds: Option<&'a Bounds>,
}

/// A version of a navigation mesh that runs over a hidden object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Carried {
    /// The object, by its place in the list of hidden objects.
    pub hidden: usize,
    /// The navigation mesh, by its place in [`NearHidden::navmeshes`].
    pub navmesh: usize,
    /// The triangles that lie on the object, by their numbers.
    pub triangles: Vec<u32>,
}

/// Every version of a navigation mesh in `near` that runs over one of the
/// `hidden` objects, with the triangles that do. Versions the game
/// does not use are included; [`NavmeshShape::used`] tells them apart.
pub fn carried(hidden: &[Hidden<'_>], near: &NearHidden) -> Vec<Carried> {
    let ground = Ground::new(near);
    let mut found = Vec::new();
    for (index, object) in hidden.iter().enumerate() {
        let Some(surface) = Surface::of(object) else {
            continue;
        };
        // Flat as a rug: what it lies on carries everyone as well.
        if surface.max[2] - surface.min[2] < RAISED {
            continue;
        }
        let world = match &object.object.place {
            Place::Exterior { worldspace } => Some(worldspace),
            Place::Interior { .. } => None,
        };
        for (navmesh, shape) in near.navmeshes.iter().enumerate() {
            if shape.place != object.object.place {
                continue;
            }
            let triangles = surface.carried(shape, |x, y| {
                world.and_then(|world| ground.height(world, x, y))
            });
            if !triangles.is_empty() {
                found.push(Carried {
                    hidden: index,
                    navmesh,
                    triangles,
                });
            }
        }
    }
    found
}

/// What the engine must read for [`HiddenWalkways`]: the models of the
/// hidden objects, and the cells they reach into.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HiddenNeeds {
    /// The models, by their path as the base objects name them.
    pub models: BTreeSet<String>,
    /// Outside: by worldspace, the cells as column and row.
    pub worlds: BTreeMap<RecordId, BTreeSet<(i32, i32)>>,
    /// Inside: the cells.
    pub interiors: BTreeSet<RecordId>,
}

/// The models and cells of the objects Mod Mallet's patch hides.
pub fn hidden_needs(facts: &Facts) -> HiddenNeeds {
    let mut needs = HiddenNeeds::default();
    for (object, base) in hidden_objects(facts) {
        if let Some(model) = base.and_then(|base| base.model.as_ref()) {
            needs.models.insert(model.clone());
        }
        match &object.place {
            Place::Interior { cell } => {
                needs.interiors.insert(cell.clone());
            }
            Place::Exterior { worldspace } => {
                // The models are not read yet: their boxes tell the cells.
                let cells = needs.worlds.entry(worldspace.clone()).or_default();
                cells.extend(cells_reached(&Hidden {
                    object,
                    mesh: None,
                    bounds: base.map(|base| &base.bounds),
                }));
            }
        }
    }
    needs
}

/// Reports where Mod Mallet's patch hides objects that characters walk
/// on.
pub struct HiddenWalkways;

impl Analysis for HiddenWalkways {
    fn id(&self) -> &'static str {
        "hidden-walkways"
    }

    fn run(&self, context: &Context<'_>) -> Vec<Finding> {
        let facts = context.facts;
        let objects = hidden_objects(facts);
        let hidden: Vec<Hidden<'_>> = objects
            .iter()
            .map(|(object, base)| Hidden {
                object,
                mesh: base
                    .and_then(|base| base.model.as_ref())
                    .and_then(|model| facts.models.meshes.get(model)),
                bounds: base.map(|base| &base.bounds),
            })
            .collect();
        let near = &facts.navmeshes.near_hidden;

        // By the plugin whose objects are hidden.
        let mut by_plugin: BTreeMap<&str, Walked<'_>> = BTreeMap::new();
        for carried in carried(&hidden, near) {
            let (Some((object, base)), Some(shape)) = (
                objects.get(carried.hidden),
                near.navmeshes.get(carried.navmesh),
            ) else {
                continue;
            };
            if !shape.used {
                continue;
            }
            let walked = by_plugin.entry(object.id.plugin.as_str()).or_default();
            walked.objects.insert(&object.id);
            walked
                .names
                .extend(base.and_then(|base| base.editor_id.as_deref().or(base.name.as_deref())));
            walked.places.extend(shape.place_name.as_deref());
            walked.navmesh_plugins.insert(shape.plugin.as_str());
            walked.triangles += carried.triangles.len();
            walked.carried.push(carried);
        }
        // Where characters could no longer go if those paths were cut out
        // (ADR 0059): only over the hidden objects do they reach it now.
        for walked in by_plugin.values_mut() {
            walked.reach = Some(unreachable(facts, &walked.carried));
        }
        by_plugin
            .into_iter()
            .map(|(plugin, walked)| finding(context, plugin, &walked))
            .collect()
    }
}

/// What the patch hides of one plugin that characters walk on.
#[derive(Default)]
struct Walked<'a> {
    objects: BTreeSet<&'a RecordId>,
    names: BTreeSet<&'a str>,
    places: BTreeSet<&'a str>,
    navmesh_plugins: BTreeSet<&'a str>,
    triangles: usize,
    carried: Vec<Carried>,
    reach: Option<Unreachable>,
}

/// The destinations characters reach only over hidden objects.
#[derive(Default)]
struct Unreachable {
    /// How many can be reached no longer, or lose what they stood on.
    count: usize,
    /// Their names, where known.
    names: BTreeSet<String>,
    /// How many are reached only by a much longer way.
    longer: usize,
}

/// What cutting the triangles `carried` out of the paths the game uses
/// would cut off.
fn unreachable(facts: &Facts, carried: &[Carried]) -> Unreachable {
    let near = &facts.navmeshes.near_hidden;
    let destinations = &facts.navmeshes.destinations;
    let after = reachability::cut(&near.navmeshes, carried);
    let found = reachability::reachability(&near.navmeshes, &after, destinations);
    let markers: HashMap<&RecordId, &str> = facts
        .markers
        .markers
        .iter()
        .filter_map(|marker| Some((&marker.id, marker.last.name.as_deref()?)))
        .collect();
    let mut unreachable = Unreachable::default();
    // Two mods may place the same thing at the same spot: counted once.
    let mut counted = BTreeSet::new();
    for reach in &found.reaches {
        let Some(destination) = destinations.get(reach.destination) else {
            continue;
        };
        if !counted.insert((destination.kind, &destination.name, destination.position)) {
            continue;
        }
        if matches!(reach.outcome, reachability::Outcome::Longer { .. }) {
            unreachable.longer += 1;
            continue;
        }
        unreachable.count += 1;
        let name = markers
            .get(&destination.id)
            .copied()
            .or(destination.name.as_deref());
        unreachable.names.extend(name.map(str::to_owned));
    }
    unreachable
}

fn finding(context: &Context<'_>, plugin: &str, walked: &Walked<'_>) -> Finding {
    // The plugin as the load order spells it.
    let plugin = context
        .facts
        .plugins
        .iter()
        .find(|known| known.name.eq_ignore_ascii_case(plugin))
        .map_or(plugin, |known| known.name.as_str());
    let patch = GamePath::new(PATCH_FILE);
    let count = |value: usize| i64::try_from(value).unwrap_or(i64::MAX);
    Finding::new(
        &kinds::PATCH_HIDES_WALKWAY,
        Subject::Override {
            winner: Party::File {
                path: patch.clone(),
            },
            loser: Party::File {
                path: GamePath::new("data").join(plugin),
            },
        },
    )
    .with_arg("plugin", plugin)
    .with_arg("count", count(walked.objects.len()))
    .with_arg(
        "objects",
        examples(walked.names.iter().copied(), EXAMPLE_LIMIT),
    )
    .with_arg(
        "places",
        examples(walked.places.iter().copied(), EXAMPLE_LIMIT),
    )
    .with_arg("placeCount", count(walked.places.len()))
    .with_arg("triangles", count(walked.triangles))
    .with_arg(
        "navmeshes",
        examples(walked.navmesh_plugins.iter().copied(), EXAMPLE_LIMIT),
    )
    .with_arg(
        "unreachable",
        count(walked.reach.as_ref().map_or(0, |reach| reach.count)),
    )
    .with_arg(
        "unreachableNames",
        examples(
            walked
                .reach
                .iter()
                .flat_map(|reach| reach.names.iter().map(String::as_str)),
            EXAMPLE_LIMIT,
        ),
    )
    .with_arg(
        "longer",
        count(walked.reach.as_ref().map_or(0, |reach| reach.longer)),
    )
    .with_provider(context.mods.provider_of(&patch))
}

/// The hidden objects that mods placed or moved, with their base objects.
fn hidden_objects(facts: &Facts) -> Vec<(&PlacedObject, Option<&BaseObject>)> {
    if facts.hidden_objects.is_empty() {
        return Vec::new();
    }
    let hidden: BTreeSet<&RecordId> = facts.hidden_objects.iter().collect();
    let bases: HashMap<&RecordId, &BaseObject> = facts
        .objects
        .bases
        .iter()
        .map(|base| (&base.id, base))
        .collect();
    facts
        .objects
        .placed
        .iter()
        .filter(|object| hidden.contains(&object.id))
        .map(|object| (object, bases.get(&object.base).copied()))
        .collect()
}

/// The outside cells the model or else the box of `hidden` reaches into;
/// its own cell if neither is known. Where it lies outside, these are the
/// cells whose navigation meshes may run over it.
pub fn cells_reached(hidden: &Hidden<'_>) -> Vec<(i32, i32)> {
    let position = hidden.object.position.map(f64::from);
    let (mut west, mut south, mut east, mut north) =
        (position[0], position[1], position[0], position[1]);
    if let Some(surface) = Surface::of(hidden) {
        west = west.min(surface.min[0]);
        south = south.min(surface.min[1]);
        east = east.max(surface.max[0]);
        north = north.max(surface.max[1]);
    }
    // Nothing the games place spans more than a few cells.
    let (west, east) = (cell_of(west), cell_of(east).min(cell_of(west) + 3));
    let (south, north) = (cell_of(south), cell_of(north).min(cell_of(south) + 3));
    (west..=east)
        .flat_map(|x| (south..=north).map(move |y| (x, y)))
        .collect()
}

/// The surface of a placed object, its triangles sorted into squares.
pub(crate) struct Surface {
    triangles: Vec<[Point; 3]>,
    min: Point,
    max: Point,
    square: f64,
    squares: HashMap<(i64, i64), Vec<usize>>,
}

impl Surface {
    /// The surface of the model, or else the box, of `hidden`.
    pub(crate) fn of(hidden: &Hidden<'_>) -> Option<Self> {
        let (points, triangles): (Vec<Point>, Vec<[u32; 3]>) = match (hidden.mesh, hidden.bounds) {
            (Some(mesh), _) => (
                placed_points(hidden.object, &mesh.points)?,
                mesh.triangles.clone(),
            ),
            (None, Some(bounds)) => (
                placed_points(hidden.object, &box_points(bounds.min, bounds.max))?,
                BOX_TRIANGLES.to_vec(),
            ),
            (None, None) => return None,
        };
        let corners = |triangle: &[u32; 3]| -> Option<[Point; 3]> {
            let point = |index: u32| points.get(usize::try_from(index).ok()?).copied();
            Some([
                point(triangle[0])?,
                point(triangle[1])?,
                point(triangle[2])?,
            ])
        };
        let triangles: Vec<[Point; 3]> = triangles.iter().filter_map(corners).collect();
        let first = triangles.first()?.first().copied()?;
        let (min, max) = triangles
            .iter()
            .flatten()
            .fold((first, first), |(min, max), point| {
                (
                    [
                        min[0].min(point[0]),
                        min[1].min(point[1]),
                        min[2].min(point[2]),
                    ],
                    [
                        max[0].max(point[0]),
                        max[1].max(point[1]),
                        max[2].max(point[2]),
                    ],
                )
            });
        let side = (max[0] - min[0]).max(max[1] - min[1]);
        let square = SQUARE.max(side / MAX_SQUARES);
        let mut surface = Self {
            triangles,
            min,
            max,
            square,
            squares: HashMap::new(),
        };
        for (index, triangle) in surface.triangles.iter().enumerate() {
            let low = surface.square_of(
                triangle.iter().map(|p| p[0]).fold(f64::MAX, f64::min),
                triangle.iter().map(|p| p[1]).fold(f64::MAX, f64::min),
            );
            let high = surface.square_of(
                triangle.iter().map(|p| p[0]).fold(f64::MIN, f64::max),
                triangle.iter().map(|p| p[1]).fold(f64::MIN, f64::max),
            );
            for x in low.0..=high.0 {
                for y in low.1..=high.1 {
                    surface.squares.entry((x, y)).or_default().push(index);
                }
            }
        }
        Some(surface)
    }

    #[allow(clippy::cast_possible_truncation)]
    fn square_of(&self, x: f64, y: f64) -> (i64, i64) {
        // Bounded: the surface spans at most MAX_SQUARES squares a side.
        let index = |value: f64, origin: f64| {
            ((value - origin) / self.square)
                .floor()
                .clamp(-1.0, MAX_SQUARES + 1.0) as i64
        };
        (index(x, self.min[0]), index(y, self.min[1]))
    }

    /// The triangles of `shape` that lie on this surface, by their
    /// numbers. `ground` gives the height of the land at a point outside,
    /// if known.
    fn carried(&self, shape: &NavmeshShape, ground: impl Fn(f64, f64) -> Option<f64>) -> Vec<u32> {
        let vertex = |index: u32| -> Option<Point> {
            let [x, y, z] = *shape.vertices.get(usize::try_from(index).ok()?)?;
            Some([f64::from(x), f64::from(y), f64::from(z)])
        };
        (0u32..)
            .zip(&shape.triangles)
            .filter_map(|(number, triangle)| {
                let [a, b, c] = [
                    vertex(triangle[0])?,
                    vertex(triangle[1])?,
                    vertex(triangle[2])?,
                ];
                let middle = [
                    (a[0] + b[0] + c[0]) / 3.0,
                    (a[1] + b[1] + c[1]) / 3.0,
                    (a[2] + b[2] + c[2]) / 3.0,
                ];
                self.carries(middle, &ground).then_some(number)
            })
            .collect()
    }

    /// Whether this surface carries a point of a navigation mesh.
    fn carries(&self, [x, y, z]: Point, ground: &impl Fn(f64, f64) -> Option<f64>) -> bool {
        if x < self.min[0] || x > self.max[0] || y < self.min[1] || y > self.max[1] {
            return false;
        }
        if z < self.min[2] - BELOW || z > self.max[2] + ABOVE {
            return false;
        }
        let Some(members) = self.squares.get(&self.square_of(x, y)) else {
            return false;
        };
        let Some(surface) = members
            .iter()
            .filter_map(|&index| height_in(self.triangles.get(index)?, x, y))
            .filter(|height| *height >= z - ABOVE && *height <= z + BELOW)
            .reduce(f64::max)
        else {
            return false;
        };
        ground(x, y).is_none_or(|ground| surface - ground >= RAISED)
    }

    /// The least and greatest corners of the box around the surface.
    pub(crate) fn bounds(&self) -> (Point, Point) {
        (self.min, self.max)
    }

    /// Whether the surface lies over or under `x`, `y`, seen from above,
    /// or within `margin` of it (ADR 0058).
    pub(crate) fn covers(&self, x: f64, y: f64, margin: f64) -> bool {
        if x < self.min[0] - margin
            || x > self.max[0] + margin
            || y < self.min[1] - margin
            || y > self.max[1] + margin
        {
            return false;
        }
        let (low, high) = (
            self.square_of(x - margin, y - margin),
            self.square_of(x + margin, y + margin),
        );
        (low.0..=high.0).any(|column| {
            (low.1..=high.1).any(|row| {
                self.squares.get(&(column, row)).is_some_and(|members| {
                    members.iter().any(|&index| {
                        self.triangles
                            .get(index)
                            .is_some_and(|triangle| flat_distance(triangle, x, y) <= margin)
                    })
                })
            })
        })
    }
}

/// How far `x`, `y` lies from a triangle, seen from above: 0 inside.
#[allow(clippy::many_single_char_names)]
fn flat_distance(corners: &[Point; 3], x: f64, y: f64) -> f64 {
    if height_in(corners, x, y).is_some() {
        return 0.0;
    }
    let [a, b, c] = *corners;
    [(a, b), (b, c), (c, a)]
        .iter()
        .map(|(from, to)| {
            let (dx, dy) = (to[0] - from[0], to[1] - from[1]);
            let length = dx * dx + dy * dy;
            let along = if length < EPSILON {
                0.0
            } else {
                (((x - from[0]) * dx + (y - from[1]) * dy) / length).clamp(0.0, 1.0)
            };
            (x - from[0] - along * dx).hypot(y - from[1] - along * dy)
        })
        .fold(f64::MAX, f64::min)
}

/// The height of a triangle at `x`, `y`, if the point lies in it seen from
/// above.
fn height_in(corners: &[Point; 3], x: f64, y: f64) -> Option<f64> {
    let [[ax, ay, az], [bx, by, bz], [cx, cy, cz]] = *corners;
    let det = (by - cy) * (ax - cx) + (cx - bx) * (ay - cy);
    if det.abs() < EPSILON {
        // Standing on its edge: nobody stands on it.
        return None;
    }
    let first = ((by - cy) * (x - cx) + (cx - bx) * (y - cy)) / det;
    let second = ((cy - ay) * (x - cx) + (ax - cx) * (y - cy)) / det;
    let third = 1.0 - first - second;
    let inside = [first, second, third]
        .iter()
        .all(|share| *share >= -EPSILON);
    inside.then_some(first * az + second * bz + third * cz)
}

/// The heights of the land by worldspace and cell.
pub(crate) struct Ground<'a> {
    cells: HashMap<(&'a RecordId, i32, i32), &'a [i32]>,
    water: HashMap<(&'a RecordId, i32, i32), i32>,
}

impl<'a> Ground<'a> {
    pub(crate) fn new(near: &'a NearHidden) -> Self {
        Self {
            cells: near
                .ground
                .iter()
                .filter(|cell| cell.heights.len() == GROUND_POINTS * GROUND_POINTS)
                .map(|cell| ((&cell.worldspace, cell.x, cell.y), cell.heights.as_slice()))
                .collect(),
            water: near
                .ground
                .iter()
                .filter_map(|cell| Some(((&cell.worldspace, cell.x, cell.y), cell.water?)))
                .collect(),
        }
    }

    /// The height of the water's surface at `x`, `y` in `world`, if the
    /// cell there has water.
    pub(crate) fn water(&self, world: &RecordId, x: f64, y: f64) -> Option<f64> {
        self.water
            .get(&(world, cell_of(x), cell_of(y)))
            .map(|height| f64::from(*height))
    }

    /// The height of the land at `x`, `y` in `world`, between its four
    /// nearest points.
    pub(crate) fn height(&self, world: &RecordId, x: f64, y: f64) -> Option<f64> {
        let (column, row) = (cell_of(x), cell_of(y));
        let heights = self.cells.get(&(world, column, row))?;
        let local = |value: f64, cell: i32| {
            ((value - f64::from(cell) * CELL) / GROUND_STEP).clamp(0.0, 32.0)
        };
        let (east, north) = (local(x, column), local(y, row));
        // The point south-west of it, and how far it lies from there.
        let (point, line) = (whole_index(east).min(31), whole_index(north).min(31));
        let (along, across) = (east - index_value(point), north - index_value(line));
        let at = |point: usize, line: usize| {
            heights
                .get(line * GROUND_POINTS + point)
                .map(|height| f64::from(*height))
        };
        let southern = at(point, line)? * (1.0 - along) + at(point + 1, line)? * along;
        let northern = at(point, line + 1)? * (1.0 - along) + at(point + 1, line + 1)? * along;
        Some(southern * (1.0 - across) + northern * across)
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn whole_index(value: f64) -> usize {
    // Clamped to 0..=32 by the caller.
    value.floor().clamp(0.0, 32.0) as usize
}

#[allow(clippy::cast_precision_loss)]
fn index_value(index: usize) -> f64 {
    // At most 32.
    index as f64
}

/// The cell a coordinate lies in.
#[allow(clippy::cast_possible_truncation)]
pub(crate) fn cell_of(coordinate: f64) -> i32 {
    // Worldspaces lie within a few hundred cells of their origin.
    (coordinate / CELL).floor().clamp(-32_768.0, 32_767.0) as i32
}

/// The corners of a box: bit 1 of the number picks the larger x, bit 2
/// the larger y, bit 4 the larger z.
fn box_points(min: [i32; 3], max: [i32; 3]) -> Vec<[i32; 3]> {
    (0..8)
        .map(|corner: u8| {
            let pick = |bit: u8, axis: usize| {
                if corner & bit == 0 {
                    min.get(axis).copied().unwrap_or_default()
                } else {
                    max.get(axis).copied().unwrap_or_default()
                }
            };
            [pick(1, 0), pick(2, 1), pick(4, 2)]
        })
        .collect()
}

/// The faces of a box of [`box_points`], two triangles each.
const BOX_TRIANGLES: [[u32; 3]; 12] = [
    [0, 1, 3],
    [0, 3, 2],
    [4, 5, 7],
    [4, 7, 6],
    [0, 1, 5],
    [0, 5, 4],
    [2, 3, 7],
    [2, 7, 6],
    [0, 2, 6],
    [0, 6, 4],
    [1, 3, 7],
    [1, 7, 5],
];

#[cfg(test)]
mod tests {
    use mallet_core::facts::PluginFacts;
    use mallet_core::facts::navmeshes::{Beyond, GroundCell};
    use mallet_core::facts::objects::{BaseKind, ObjectFacts};
    use mallet_core::{ArgValue, NoModSource, Severity};

    use super::*;

    fn id(plugin: &str, local_id: u32) -> RecordId {
        RecordId {
            plugin: plugin.to_lowercase(),
            local_id,
        }
    }

    fn world() -> RecordId {
        id("Skyrim.esm", 0x3C)
    }

    /// A pier of `Docks.esp`: a box 400 units long, 200 wide, its deck
    /// 100 units above where it stands.
    fn pier(local_id: u32, position: [i32; 3]) -> (PlacedObject, BaseObject) {
        let base = BaseObject {
            id: id("Docks.esp", 0x800),
            editor_id: Some("DockPier01".to_owned()),
            name: None,
            model: Some("meshes/docks/pier01.nif".to_owned()),
            kind: BaseKind::Static,
            solid: true,
            natural: false,
            bounds: Bounds {
                min: [-200, -100, 0],
                max: [200, 100, 100],
            },
        };
        let object = PlacedObject {
            id: id("Docks.esp", local_id),
            plugin: "Docks.esp".to_owned(),
            base: base.id.clone(),
            place: Place::Exterior {
                worldspace: world(),
            },
            position,
            rotation: [0, 0, 0],
            scale: 1000,
            persistent: false,
            large: false,
            moved: None,
        };
        (object, base)
    }

    /// A navigation mesh of two triangles, a square of 100 units around
    /// `centre` at height `z`.
    fn square(plugin: &str, used: bool, centre: [i32; 2], z: i32) -> NavmeshShape {
        let [x, y] = centre;
        NavmeshShape {
            id: id("Skyrim.esm", 0x5000),
            plugin: plugin.to_owned(),
            used,
            place: Place::Exterior {
                worldspace: world(),
            },
            place_name: Some("Tamriel 0,0".to_owned()),
            vertices: vec![
                [x - 50, y - 50, z],
                [x + 50, y - 50, z],
                [x + 50, y + 50, z],
                [x - 50, y + 50, z],
            ],
            triangles: vec![[0, 1, 2], [0, 2, 3]],
            edges: vec![
                [Beyond::Nothing, Beyond::Nothing, Beyond::Triangle(1)],
                [Beyond::Triangle(0), Beyond::Nothing, Beyond::Nothing],
            ],
            doors: Vec::new(),
        }
    }

    fn flat_ground(height: i32) -> GroundCell {
        GroundCell {
            worldspace: world(),
            x: 0,
            y: 0,
            heights: vec![height; GROUND_POINTS * GROUND_POINTS],
            water: None,
        }
    }

    fn count(object: &PlacedObject, bounds: &Bounds, near: &NearHidden) -> Vec<(usize, usize)> {
        let hidden = [Hidden {
            object,
            mesh: None,
            bounds: Some(bounds),
        }];
        carried(&hidden, near)
            .iter()
            .map(|carried| (carried.navmesh, carried.triangles.len()))
            .collect()
    }

    #[test]
    fn a_path_over_a_raised_deck_runs_over_it() {
        let (object, base) = pier(0x900, [1000, 1000, 0]);
        let near = NearHidden {
            navmeshes: vec![
                // On the deck, a little above it.
                square("Docks.esp", true, [1000, 1000], 110),
                // Beside the pier, on the ground.
                square("Skyrim.esm", true, [1500, 1000], 0),
                // Under the pier, far below its deck.
                square("Other.esp", true, [1000, 1000], -300),
            ],
            ground: vec![flat_ground(-200)],
        };
        assert_eq!(count(&object, &base.bounds, &near), vec![(0, 2)]);
    }

    #[test]
    fn a_surface_hardly_above_the_ground_carries_nobody() {
        let (object, base) = pier(0x900, [1000, 1000, 0]);
        let mut near = NearHidden {
            navmeshes: vec![square("Docks.esp", true, [1000, 1000], 105)],
            ground: vec![flat_ground(90)],
        };
        assert_eq!(count(&object, &base.bounds, &near), vec![]);
        // Without known ground, as inside, the surface counts.
        near.ground.clear();
        assert_eq!(count(&object, &base.bounds, &near), vec![(0, 2)]);
    }

    #[test]
    fn a_flat_object_lies_on_what_carries_it() {
        // A rug, 4 units high: the floor under it carries the path.
        let (object, _) = pier(0x900, [1000, 1000, 0]);
        let rug = Bounds {
            min: [-200, -100, 0],
            max: [200, 100, 4],
        };
        let near = NearHidden {
            navmeshes: vec![square("Docks.esp", true, [1000, 1000], 10)],
            ground: Vec::new(),
        };
        assert_eq!(count(&object, &rug, &near), vec![]);
    }

    #[test]
    fn a_turned_model_carries_where_it_lies() {
        // Turned a quarter, the pier lies 400 units long in y and 200 wide
        // in x, so a path 150 units east of its middle misses it.
        let (mut object, base) = pier(0x900, [1000, 1000, 0]);
        object.rotation = [0, 0, 1571];
        let near = NearHidden {
            navmeshes: vec![
                square("Docks.esp", true, [1000, 1150], 110),
                square("Docks.esp", true, [1150, 1000], 110),
            ],
            ground: Vec::new(),
        };
        assert_eq!(count(&object, &base.bounds, &near), vec![(0, 2)]);
    }

    #[test]
    fn a_model_s_triangles_decide_over_its_box() {
        let (object, base) = pier(0x900, [1000, 1000, 0]);
        // Only a ramp from the west end, at height 0, to the east end at
        // height 100.
        let ramp = ModelMesh {
            points: vec![
                [-200, -100, 0],
                [200, -100, 100],
                [200, 100, 100],
                [-200, 100, 0],
            ],
            triangles: vec![[0, 1, 2], [0, 2, 3]],
        };
        let near = NearHidden {
            navmeshes: vec![
                square("Docks.esp", true, [1150, 1000], 90),
                square("Docks.esp", true, [850, 1000], 90),
            ],
            ground: Vec::new(),
        };
        let hidden = [Hidden {
            object: &object,
            mesh: Some(&ramp),
            bounds: Some(&base.bounds),
        }];
        let found: Vec<usize> = carried(&hidden, &near)
            .iter()
            .map(|carried| carried.navmesh)
            .collect();
        assert_eq!(found, vec![0]);
    }

    #[test]
    fn the_ground_of_a_cell_lies_between_its_points() {
        let mut cell = flat_ground(0);
        // The second point of the southern row is 128 units high.
        cell.heights[1] = 128;
        let near = NearHidden {
            navmeshes: Vec::new(),
            ground: vec![cell],
        };
        let ground = Ground::new(&near);
        let height = ground.height(&world(), 64.0, 0.0).unwrap();
        assert!((height - 64.0).abs() < 1e-6);
        assert_eq!(ground.height(&world(), 5000.0, 0.0), None);
    }

    fn facts(hidden: bool) -> Facts {
        let (object, base) = pier(0x900, [1000, 1000, 0]);
        Facts {
            plugins: vec![PluginFacts {
                name: "Docks.esp".to_owned(),
                path: GamePath::new("data/Docks.esp"),
                masters: vec!["Skyrim.esm".to_owned()],
                npcs: Vec::new(),
            }],
            objects: ObjectFacts {
                placed: vec![object.clone()],
                bases: vec![base],
                place_names: Vec::new(),
            },
            hidden_objects: if hidden { vec![object.id] } else { Vec::new() },
            navmeshes: mallet_core::facts::NavmeshFacts {
                changes: Vec::new(),
                near_hidden: NearHidden {
                    navmeshes: vec![
                        square("Skyrim.esm", false, [1000, 1000], 110),
                        square("Docks.esp", true, [1000, 1000], 110),
                    ],
                    ground: vec![flat_ground(-200)],
                },
                destinations: Vec::new(),
            },
            ..Facts::default()
        }
    }

    #[test]
    fn hiding_a_pier_with_a_path_on_it_is_reported() {
        let facts = facts(true);
        let findings = HiddenWalkways.run(&Context::new(&facts, &NoModSource));

        assert_eq!(findings.len(), 1);
        let finding = &findings[0];
        assert_eq!(finding.kind, "patch.hides-walkway");
        assert_eq!(finding.severity, Severity::Warning);
        assert_eq!(finding.args["plugin"], ArgValue::from("Docks.esp"));
        assert_eq!(finding.args["count"], ArgValue::Number(1));
        assert_eq!(finding.args["objects"], ArgValue::from("DockPier01"));
        assert_eq!(finding.args["places"], ArgValue::from("Tamriel 0,0"));
        // Only the version the game uses counts.
        assert_eq!(finding.args["triangles"], ArgValue::Number(2));
        assert_eq!(finding.args["navmeshes"], ArgValue::from("Docks.esp"));
        assert_eq!(
            finding.id.as_str(),
            "patch.hides-walkway:override:file:data/modmallet_swap.ini>file:data/docks.esp"
        );
    }

    #[test]
    fn what_characters_reach_only_over_the_hidden_pier_is_named() {
        let mut facts = facts(true);
        assert_eq!(
            HiddenWalkways.run(&Context::new(&facts, &NoModSource))[0].args["unreachable"],
            ArgValue::Number(0)
        );
        // The path on the pier leads on to the shore, and a bed stands on
        // the pier.
        if let Some(edges) = facts.navmeshes.near_hidden.navmeshes[1].edges.get_mut(0) {
            edges[1] = Beyond::Link {
                navmesh: id("Skyrim.esm", 0x6000),
                triangle: 0,
            };
        }
        facts.navmeshes.destinations = vec![mallet_core::facts::navmeshes::PlacedDestination {
            id: id("Docks.esp", 0x901),
            plugin: "Docks.esp".to_owned(),
            kind: mallet_core::reach::DestinationKind::Furniture,
            name: Some("Bed".to_owned()),
            place: Place::Exterior {
                worldspace: world(),
            },
            position: [1000, 1000, 110],
            leads_to: None,
        }];
        let findings = HiddenWalkways.run(&Context::new(&facts, &NoModSource));
        let finding = &findings[0];
        assert_eq!(finding.args["unreachable"], ArgValue::Number(1));
        assert_eq!(finding.args["unreachableNames"], ArgValue::from("Bed"));
        assert_eq!(finding.args["longer"], ArgValue::Number(0));
    }

    #[test]
    fn without_a_patch_nothing_is_reported_or_read() {
        let facts = facts(false);
        assert_eq!(
            HiddenWalkways.run(&Context::new(&facts, &NoModSource)),
            Vec::<Finding>::new()
        );
        assert_eq!(hidden_needs(&facts), HiddenNeeds::default());
    }

    #[test]
    fn the_needs_are_the_model_and_the_cells_the_box_reaches() {
        let mut facts = facts(true);
        // Across the border of cells 0 and 1 in x.
        facts.objects.placed[0].position = [4096, 1000, 0];
        let needs = hidden_needs(&facts);
        assert_eq!(
            needs.models,
            BTreeSet::from(["meshes/docks/pier01.nif".to_owned()])
        );
        assert_eq!(
            needs.worlds,
            BTreeMap::from([(world(), BTreeSet::from([(0, 0), (1, 0)]))])
        );
        assert_eq!(needs.interiors, BTreeSet::new());
    }
}
