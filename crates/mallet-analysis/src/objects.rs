//! Objects of two mods that stand in the same spot.
//!
//! Two mods that change the same town often both put a building, a tree or
//! a market stall where the other already has one. The game shows both:
//! the same house twice, flickering where the two copies meet, or a tree
//! growing through a roof. Neither mod is broken, and no mod manager shows
//! it, because the two mods do not touch the same record.
//!
//! The analysis compares the objects that mods place, in the version the
//! game uses, by the box around each object's model, turned and scaled as
//! placed. It reports two things for each pair of plugins:
//!
//! * the same base object placed twice at almost the same spot,
//! * two objects of a similar size that stand largely inside each other.
//!
//! A box is only an approximation of a model: an arch has empty space under
//! it, a wall stands at an angle. Small objects are left out, and so are
//! objects that only touch or that are far smaller than the one they stand
//! in, such as a chair in a house, and two pieces of landscape, such as two
//! rocks, which are set into each other on purpose. Pairs of plugins where one builds on the
//! other, or where a third plugin builds on both, are left out as well:
//! their authors knew of each other, and a patch has had the chance to
//! sort out what stands where.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use mallet_core::facts::models::{ModelFacts, ModelMesh};
use mallet_core::facts::objects::{BaseObject, Place, PlacedObject};
use mallet_core::facts::{PluginFacts, RecordId};
use mallet_core::{
    Finding, GamePath, MapSpot, ModRef, ObjectRef, Party, SpotKind, SpotLocation, SpotPart,
    Subject, kinds,
};

use crate::contact::{self, Placed};
use crate::{Analysis, Context, examples};

/// How many examples a finding names.
const EXAMPLE_LIMIT: usize = 5;

/// The same base object this close (in game units, about 45 cm) counts as
/// placed twice.
const DUPLICATE_DISTANCE: f64 = 32.0;

/// Objects whose longest side is shorter than this (about 1.5 m) are left
/// out of the overlap check: crockery, books, small rocks.
const MIN_EXTENT: f64 = 100.0;

/// The share of the smaller object that has to lie inside the larger one.
const MIN_SHARE_OF_SMALLER: f64 = 0.5;

/// The share of the larger object that the overlap has to fill, so that a
/// barrel inside a house does not count.
const MIN_SHARE_OF_LARGER: f64 = 0.1;

/// Points sampled along each side of the smaller box to measure how much
/// of it lies inside the larger one.
const SAMPLES: u32 = 5;

/// Where the models of two objects are known, their surfaces have to cross
/// over at least this many game units (about 25 cm) for a touch to count:
/// less is a seam the eye does not notice.
const MIN_TOUCH: f64 = 16.0;

/// Side of the squares the world is divided into for finding neighbours,
/// in game units: a quarter of a cell.
const BUCKET: f64 = 1024.0;

/// Objects that would fall into more squares than this are not compared:
/// they are larger than any building.
const MAX_BUCKETS: i64 = 256;

/// A plugin counts as contained in another when at least this share of its
/// solid objects stands in the other plugin as well, in tenths.
const CONTAINED_SHARE_TENTHS: u64 = 7;

/// Plugins with fewer solid objects than this are never called contained:
/// a handful of shared objects says little.
const CONTAINED_MIN_OBJECTS: u64 = 5;

/// Reports objects of different mods that stand in the same spot.
pub struct PlacedObjects;

impl Analysis for PlacedObjects {
    fn id(&self) -> &'static str {
        "placed-objects"
    }

    fn run(&self, context: &Context<'_>) -> Vec<Finding> {
        context.collisions().findings.clone()
    }
}

/// What the comparison of placed objects found: the findings, and the
/// spot of every collision behind them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Collisions {
    /// One finding for each pair of plugins and kind of collision.
    pub findings: Vec<Finding>,
    /// Where each collision is, with the finding it belongs to.
    pub spots: Vec<MapSpot>,
}

/// One collision, before it is counted into its pair's finding.
struct Found {
    /// The objects and the place, for the examples of a finding.
    example: String,
    /// The objects alone.
    objects: String,
    location: SpotLocation,
    /// The two objects, each with the plugin that placed it.
    parts: [(String, SpotPart); 2],
    /// The point between the two objects.
    position: [i32; 3],
    /// What kind of collision it is.
    kind: SpotKind,
    /// How long the stretch is along which the models cross, where they
    /// were compared.
    depth: Option<u32>,
}

/// Compares the placed objects. Called once per scan through
/// [`Context::collisions`].
pub(crate) fn compute(context: &Context<'_>) -> Collisions {
    let objects = &context.facts.objects;
    if objects.placed.is_empty() {
        return Collisions::default();
    }
    let bases: HashMap<&RecordId, &BaseObject> =
        objects.bases.iter().map(|base| (&base.id, base)).collect();
    let solids = solids_of(context, &bases);
    let mut totals: HashMap<&str, u64> = HashMap::new();
    for solid in &solids {
        *totals.entry(solid.owner()).or_default() += 1;
    }
    let names: HashMap<&RecordId, &str> = objects
        .place_names
        .iter()
        .map(|name| (&name.id, name.editor_id.as_str()))
        .collect();
    let mut owners = Owners::new(context);
    let mut pairs: BTreeMap<(String, String, Clash), Vec<Found>> = BTreeMap::new();

    for (a, b, clash) in candidates(&solids, &mut owners) {
        let crossing = mesh_crossing(a, b, &context.facts.models);
        // Where both models are known, they decide: an overlap of boxes
        // whose models do not cross is an arch over a path, not a clash.
        let keep = match clash {
            Clash::Duplicate => true,
            Clash::Overlap => crossing.is_none_or(|stretch| stretch > 0.0),
            Clash::Touch => {
                crossing.is_some_and(|stretch| stretch >= MIN_TOUCH)
                    && !touched_as_placed_by_the_game(a, b, &context.facts.models)
            }
        };
        if !keep {
            continue;
        }
        let (first, second) = if a.owner() <= b.owner() {
            (a.owner(), b.owner())
        } else {
            (b.owner(), a.owner())
        };
        let mut found = found(a, b, clash, &names);
        found.depth = crossing.map(whole_units);
        for (plugin, part) in &mut found.parts {
            part.object.plugin = owners.file_name(plugin);
        }
        pairs
            .entry((first.to_owned(), second.to_owned(), clash))
            .or_default()
            .push(found);
    }

    let mut collisions = Collisions::default();
    for ((first, second, clash), mut found) in pairs {
        let total = |plugin: &str| totals.get(plugin).copied().unwrap_or_default();
        // Of two copies, the one from the plugin that places fewer objects
        // is the one to hide, if it can be.
        if clash == Clash::Duplicate {
            let smaller = if total(&first) <= total(&second) {
                &first
            } else {
                &second
            };
            suggest_hiding(&mut found, smaller);
        }
        let finding = match clash {
            Clash::Duplicate => {
                let count = u64::try_from(found.len()).unwrap_or(u64::MAX);
                match contained(count, (&first, total(&first)), (&second, total(&second))) {
                    Some((inner, outer, inner_total)) => {
                        contained_finding(&owners, inner, outer, count, inner_total)
                    }
                    None => pair_finding(&owners, &first, &second, clash, &found),
                }
            }
            Clash::Overlap | Clash::Touch => pair_finding(&owners, &first, &second, clash, &found),
        };
        collisions
            .spots
            .extend(found.into_iter().map(|found| MapSpot {
                finding: finding.id.clone(),
                console: console(&found.location),
                location: found.location,
                objects: found.objects,
                parts: found.parts.into_iter().map(|(_, part)| part).collect(),
                position: Some(found.position),
                kind: found.kind,
                depth: found.depth,
            }));
        collisions.findings.push(finding);
    }
    collisions
}

/// The models of the objects that may collide, by their paths as the base
/// objects name them. The engine reads these, and only these, into
/// [`ModelFacts`] before the analyses run (ADR 0039).
pub fn models_to_check(context: &Context<'_>) -> BTreeSet<String> {
    let objects = &context.facts.objects;
    let bases: HashMap<&RecordId, &BaseObject> =
        objects.bases.iter().map(|base| (&base.id, base)).collect();
    let solids = solids_of(context, &bases);
    let mut owners = Owners::new(context);
    let mut models = BTreeSet::new();
    for (a, b, clash) in candidates(&solids, &mut owners) {
        if clash != Clash::Duplicate {
            models.extend(a.base.model.iter().cloned());
            models.extend(b.base.model.iter().cloned());
        }
    }
    models
}

/// The solid objects mods place, as boxes in the world.
fn solids_of<'a>(
    context: &'a Context<'_>,
    bases: &HashMap<&RecordId, &'a BaseObject>,
) -> Vec<Solid<'a>> {
    context
        .facts
        .objects
        .placed
        .iter()
        .filter_map(|object| {
            let base = bases.get(&object.base).copied()?;
            if base.solid {
                Solid::new(object, base)
            } else {
                None
            }
        })
        .collect()
}

/// The pairs of solids of different plugins that do not know each other
/// and whose boxes collide, with how.
fn candidates<'s>(
    solids: &'s [Solid<'s>],
    owners: &mut Owners<'_>,
) -> Vec<(&'s Solid<'s>, &'s Solid<'s>, Clash)> {
    let mut found = Vec::new();
    for (first, second) in neighbours(solids) {
        let (Some(a), Some(b)) = (solids.get(first), solids.get(second)) else {
            continue;
        };
        if owners.related(a.owner(), b.owner()) {
            continue;
        }
        let Some(clash) = clash(a, b) else {
            continue;
        };
        if clash != Clash::Touch && !caused_by_moving(a, b) {
            continue;
        }
        found.push((a, b, clash));
    }
    found
}

/// Whether two objects, at least one of them an object of the game that a
/// mod moved, already touch where the game places them: then the touch is
/// not the mods' doing.
fn touched_as_placed_by_the_game(a: &Solid<'_>, b: &Solid<'_>, models: &ModelFacts) -> bool {
    let original = |solid: &Solid<'_>| -> Option<PlacedObject> {
        let from = solid.object.moved.as_ref()?.from?;
        Some(PlacedObject {
            position: from.position,
            rotation: from.rotation,
            scale: from.scale,
            ..solid.object.clone()
        })
    };
    let (first, second) = (original(a), original(b));
    if first.is_none() && second.is_none() {
        return false;
    }
    let (Some(a), Some(b)) = (
        first
            .as_ref()
            .map_or(Some(a.clone()), |object| Solid::new(object, a.base)),
        second
            .as_ref()
            .map_or(Some(b.clone()), |object| Solid::new(object, b.base)),
    ) else {
        return false;
    };
    mesh_crossing(&a, &b, models).is_some_and(|stretch| stretch >= MIN_TOUCH)
}

/// How long the stretch is along which the models of `a` and `b` cross,
/// in game units; `None` where a model is not known or too detailed.
fn mesh_crossing(a: &Solid<'_>, b: &Solid<'_>, models: &ModelFacts) -> Option<f64> {
    let model = |solid: &Solid<'_>| models.meshes.get(solid.base.model.as_deref()?);
    let placed = |solid: &Solid<'_>, mesh: &'_ ModelMesh| -> Vec<contact::Point> {
        mesh.points
            .iter()
            .map(|&point| {
                let world = solid.to_world(Vector::from(point).scaled(solid.scale));
                [world.x, world.y, world.z]
            })
            .collect()
    };
    let (first, second) = (model(a)?, model(b)?);
    contact::crossing(
        &Placed {
            points: placed(a, first),
            triangles: &first.triangles,
        },
        &Placed {
            points: placed(b, second),
            triangles: &second.triangles,
        },
    )
}

/// A length in whole game units.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn whole_units(length: f64) -> u32 {
    // Bounded before converting.
    length.round().clamp(0.0, f64::from(u32::MAX)) as u32
}

/// Marks in each place the copy to hide: the one of `smaller` if it can
/// be hidden, else any that can.
fn suggest_hiding(found: &mut [Found], smaller: &str) {
    for found in found {
        let pick = found
            .parts
            .iter()
            .position(|(plugin, part)| plugin == smaller && part.hideable)
            .or_else(|| found.parts.iter().position(|(_, part)| part.hideable));
        if let Some(part) = pick.and_then(|index| found.parts.get_mut(index)) {
            part.1.suggested = true;
        }
    }
}

/// Whether a clash that involves an object of the game that a mod moved
/// comes from the move: where the game has it, it would not clash. The
/// game's objects as shipped are not compared, so a clash that was there
/// before is not the mod's doing.
fn caused_by_moving(a: &Solid<'_>, b: &Solid<'_>) -> bool {
    [(a, b), (b, a)].into_iter().all(|(moved, other)| {
        let Some(from) = moved.object.moved.as_ref().and_then(|moved| moved.from) else {
            return true;
        };
        let original = PlacedObject {
            position: from.position,
            rotation: from.rotation,
            scale: from.scale,
            ..moved.object.clone()
        };
        Solid::new(&original, moved.base)
            .is_none_or(|solid| clash(&solid, other).is_none_or(|clash| clash == Clash::Touch))
    })
}

/// How two objects collide.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Clash {
    /// The same base object at almost the same spot.
    Duplicate,
    /// Two objects largely inside each other.
    Overlap,
    /// Two objects whose boxes meet without one standing largely in the
    /// other: whether they touch, only their models can tell.
    Touch,
}

/// Which pairs of solids are close enough to be compared: each pair once,
/// for solids in the same place whose boxes reach into the same square.
fn neighbours(solids: &[Solid<'_>]) -> Vec<(usize, usize)> {
    let mut squares: HashMap<(&Place, i64, i64), Vec<usize>> = HashMap::new();
    for (index, solid) in solids.iter().enumerate() {
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

    let mut pairs = Vec::new();
    for ((_, x, y), members) in &squares {
        for (position, &first) in members.iter().enumerate() {
            for &second in members.iter().skip(position + 1) {
                let (Some(a), Some(b)) = (solids.get(first), solids.get(second)) else {
                    continue;
                };
                if a.owner() == b.owner() || !a.boxes_meet(b) {
                    continue;
                }
                // A pair whose boxes share several squares is compared in
                // the square where their common part begins, and only there.
                let start = a.common_start(b);
                if start == (*x, *y) {
                    pairs.push((first, second));
                }
            }
        }
    }
    pairs.sort_unstable();
    pairs
}

/// Whether and how two solids of different plugins collide.
fn clash(a: &Solid<'_>, b: &Solid<'_>) -> Option<Clash> {
    if a.object.base == b.object.base && a.distance(b) <= DUPLICATE_DISTANCE {
        return Some(Clash::Duplicate);
    }
    if a.base.natural && b.base.natural {
        return None;
    }
    let (smaller, larger) = if a.volume() <= b.volume() {
        (a, b)
    } else {
        (b, a)
    };
    if smaller.longest_side() < MIN_EXTENT {
        return None;
    }
    let share = smaller.share_inside(larger);
    let share_of_larger = share * smaller.volume() / larger.volume();
    if share >= MIN_SHARE_OF_SMALLER && share_of_larger >= MIN_SHARE_OF_LARGER {
        Some(Clash::Overlap)
    } else {
        // Landscape and snow are set into other things on purpose; only
        // standing largely inside something counts for them.
        (!a.base.natural && !b.base.natural).then_some(Clash::Touch)
    }
}

/// What is said about one collision: the objects and where they are.
fn found(a: &Solid<'_>, b: &Solid<'_>, clash: Clash, names: &HashMap<&RecordId, &str>) -> Found {
    let name_of = |id: &RecordId| {
        names
            .get(id)
            .map_or_else(|| id.to_string(), |name| (*name).to_owned())
    };
    let (location, place) = match &a.object.place {
        Place::Interior { cell } => {
            let cell = name_of(cell);
            (SpotLocation::Interior { cell: cell.clone() }, cell)
        }
        Place::Exterior { worldspace } => {
            let world = name_of(worldspace);
            let [x, y, _] = a.object.position;
            let (x, y) = (cell_of(x), cell_of(y));
            let place = format!("{world} {x}, {y}");
            (SpotLocation::Exterior { world, x, y }, place)
        }
    };
    let objects = match clash {
        Clash::Duplicate => a.name(),
        Clash::Overlap | Clash::Touch => {
            let mut pair = [a.name(), b.name()];
            pair.sort_unstable();
            let [first, second] = pair;
            format!("{first} / {second}")
        }
    };
    let part = |solid: &Solid<'_>| {
        (
            solid.owner().to_owned(),
            SpotPart {
                object: ObjectRef {
                    plugin: solid.owner().to_owned(),
                    local_id: solid.object.id.local_id,
                },
                name: solid.name(),
                hideable: !solid.object.persistent && !solid.object.large,
                suggested: false,
            },
        )
    };
    Found {
        example: format!("{objects} ({place})"),
        objects,
        location,
        parts: [part(a), part(b)],
        position: middle(a.object.position, b.object.position),
        kind: match clash {
            Clash::Duplicate => SpotKind::Duplicate,
            Clash::Overlap => SpotKind::Overlap,
            Clash::Touch => SpotKind::Touch,
        },
        depth: None,
    }
}

/// The point halfway between two positions.
fn middle([ax, ay, az]: [i32; 3], [bx, by, bz]: [i32; 3]) -> [i32; 3] {
    let half = |a: i32, b: i32| {
        // Summed wide, so that no position can overflow.
        i32::try_from((i64::from(a) + i64::from(b)).div_euclid(2)).unwrap_or(a)
    };
    [half(ax, bx), half(ay, by), half(az, bz)]
}

/// The cell a coordinate lies in. The world is laid out in cells of 4096
/// units.
fn cell_of(coordinate: i32) -> i32 {
    coordinate.div_euclid(4096)
}

/// The command of the game's console that takes the player to a spot.
fn console(location: &SpotLocation) -> String {
    match location {
        SpotLocation::Exterior { world, x, y } => format!("cow {world} {x} {y}"),
        SpotLocation::Interior { cell } => format!("coc {cell}"),
    }
}

/// Whether one plugin of a pair is contained in the other: most of its
/// solid objects stand in the other as well. Returns the contained plugin,
/// the other, and how many solid objects the contained one has.
fn contained<'p>(
    duplicates: u64,
    first: (&'p str, u64),
    second: (&'p str, u64),
) -> Option<(&'p str, &'p str, u64)> {
    let share_is_enough = |total: u64| {
        total >= CONTAINED_MIN_OBJECTS && duplicates * 10 >= total * CONTAINED_SHARE_TENTHS
    };
    // Where both qualify, the smaller one is the one inside.
    let (smaller, larger) = if first.1 <= second.1 {
        (first, second)
    } else {
        (second, first)
    };
    [(smaller, larger), (larger, smaller)]
        .into_iter()
        .find(|((_, total), _)| share_is_enough(*total))
        .map(|((inner, total), (outer, _))| (inner, outer, total))
}

fn pair_finding(
    owners: &Owners<'_>,
    first: &str,
    second: &str,
    clash: Clash,
    found: &[Found],
) -> Finding {
    let kind = match clash {
        Clash::Duplicate => &kinds::OBJECTS_DUPLICATED,
        Clash::Overlap => &kinds::OBJECTS_OVERLAPPING,
        Clash::Touch => &kinds::OBJECTS_TOUCHING,
    };
    Finding::new(kind, owners.pair(first, second))
        .with_arg("first", owners.display(first))
        .with_arg("second", owners.display(second))
        .with_arg("count", i64::try_from(found.len()).unwrap_or(i64::MAX))
        .with_arg(
            "examples",
            examples(
                found.iter().map(|found| found.example.as_str()),
                EXAMPLE_LIMIT,
            ),
        )
}

fn contained_finding(
    owners: &Owners<'_>,
    inner: &str,
    outer: &str,
    duplicates: u64,
    total: u64,
) -> Finding {
    let dependents = owners.dependents(inner);
    let number = |value: u64| i64::try_from(value).unwrap_or(i64::MAX);
    Finding::new(&kinds::PLUGINS_CONTAINED, owners.pair(inner, outer))
        .with_arg("plugin", owners.display(inner))
        .with_arg("pluginFile", owners.file_name(inner))
        .with_arg("container", owners.display(outer))
        .with_arg("count", number(duplicates))
        .with_arg("total", number(total))
        .with_arg(
            "dependentCount",
            i64::try_from(dependents.len()).unwrap_or(i64::MAX),
        )
        .with_arg(
            "dependents",
            examples(dependents.iter().map(String::as_str), EXAMPLE_LIMIT),
        )
        .with_provider(owners.provider(inner))
}

/// The plugins that place objects, and what is known about how they
/// belong together.
pub(crate) struct Owners<'a> {
    context: &'a Context<'a>,
    /// The active plugins by their name in lower case.
    plugins: HashMap<String, &'a PluginFacts>,
    /// The masters of each active plugin, in lower case.
    masters: HashMap<String, HashSet<String>>,
    /// Answers already given by [`Owners::related`].
    known: HashMap<(String, String), bool>,
}

impl<'a> Owners<'a> {
    pub(crate) fn new(context: &'a Context<'a>) -> Self {
        let plugins = context
            .facts
            .plugins
            .iter()
            .map(|plugin| (plugin.name.to_lowercase(), plugin))
            .collect();
        let masters = context
            .facts
            .plugins
            .iter()
            .map(|plugin| {
                let masters = plugin.masters.iter().map(|master| master.to_lowercase());
                (plugin.name.to_lowercase(), masters.collect())
            })
            .collect();
        Self {
            context,
            plugins,
            masters,
            known: HashMap::new(),
        }
    }

    /// Whether the authors of two plugins knew of each other: one builds on
    /// the other, a third plugin builds on both, or both come with the same
    /// mod.
    fn related(&mut self, a: &str, b: &str) -> bool {
        let key = if a <= b {
            (a.to_owned(), b.to_owned())
        } else {
            (b.to_owned(), a.to_owned())
        };
        if let Some(known) = self.known.get(&key) {
            return *known;
        }
        let builds_on = |plugin: &str, master: &str| {
            self.masters
                .get(plugin)
                .is_some_and(|masters| masters.contains(master))
        };
        let same_mod = match (self.plugins.get(a), self.plugins.get(b)) {
            (Some(a), Some(b)) => {
                let provider = |plugin: &PluginFacts| self.context.mods.provider_of(&plugin.path);
                provider(a).is_some_and(|mod_a| Some(mod_a) == provider(b))
            }
            _ => false,
        };
        let related = same_mod
            || builds_on(a, b)
            || builds_on(b, a)
            || self
                .masters
                .values()
                .any(|masters| masters.contains(a) && masters.contains(b));
        self.known.insert(key, related);
        related
    }

    fn party(&self, plugin: &str) -> Party {
        Party::File {
            path: self.plugins.get(plugin).map_or_else(
                || GamePath::new("data").join(plugin),
                |facts| facts.path.clone(),
            ),
        }
    }

    /// The subject of a finding about two plugins, in an order that does
    /// not depend on the load order.
    fn pair(&self, a: &str, b: &str) -> Subject {
        let (first, second) = if a <= b { (a, b) } else { (b, a) };
        Subject::Pair {
            first: self.party(first),
            second: self.party(second),
        }
    }

    /// The name to show: the mod a plugin belongs to, else the plugin.
    pub(crate) fn display(&self, plugin: &str) -> String {
        self.provider(plugin)
            .map_or_else(|| self.file_name(plugin), |provider| provider.name)
    }

    /// The plugin's file name in the spelling of the load order.
    fn file_name(&self, plugin: &str) -> String {
        self.plugins
            .get(plugin)
            .map_or_else(|| plugin.to_owned(), |facts| facts.name.clone())
    }

    /// The mod a plugin belongs to.
    fn provider(&self, plugin: &str) -> Option<ModRef> {
        let facts = self.plugins.get(plugin)?;
        self.context.mods.provider_of(&facts.path)
    }

    /// The active plugins that build on `plugin`, by file name.
    fn dependents(&self, plugin: &str) -> Vec<String> {
        let mut dependents: Vec<String> = self
            .masters
            .iter()
            .filter(|(_, masters)| masters.contains(plugin))
            .map(|(dependent, _)| self.file_name(dependent))
            .collect();
        dependents.sort_unstable();
        dependents
    }
}

/// A placed object as a box in the world.
#[derive(Clone)]
struct Solid<'a> {
    object: &'a PlacedObject,
    base: &'a BaseObject,
    position: Vector,
    /// The size relative to the base object.
    scale: f64,
    /// Turns a direction of the object into a direction of the world.
    rotation: Matrix,
    /// The object's own box, scaled.
    min: Vector,
    max: Vector,
    /// The box that encloses the turned object, in the world.
    world_min: Vector,
    world_max: Vector,
}

impl<'a> Solid<'a> {
    fn new(object: &'a PlacedObject, base: &'a BaseObject) -> Option<Self> {
        let scale = f64::from(object.scale) / 1000.0;
        if scale <= 0.0 {
            return None;
        }
        let [x, y, z] = object.position;
        let [rx, ry, rz] = object.rotation.map(|value| f64::from(value) / 1000.0);
        let min = Vector::from(base.bounds.min).scaled(scale);
        let max = Vector::from(base.bounds.max).scaled(scale);
        let mut solid = Self {
            object,
            base,
            position: Vector::new(f64::from(x), f64::from(y), f64::from(z)),
            scale,
            rotation: Matrix::placed(rx, ry, rz),
            min,
            max,
            world_min: Vector::new(f64::MAX, f64::MAX, f64::MAX),
            world_max: Vector::new(f64::MIN, f64::MIN, f64::MIN),
        };
        for corner in corners(min, max) {
            let world = solid.to_world(corner);
            solid.world_min = solid.world_min.min(world);
            solid.world_max = solid.world_max.max(world);
        }
        Some(solid)
    }

    /// The plugin the object counts as: the one that placed it first, or
    /// the one that moved it, if it is an object of the game.
    fn owner(&self) -> &str {
        self.object.owner()
    }

    fn name(&self) -> String {
        self.base
            .editor_id
            .clone()
            .unwrap_or_else(|| self.base.id.to_string())
    }

    fn to_world(&self, local: Vector) -> Vector {
        self.position.plus(self.rotation.apply(local))
    }

    fn to_local(&self, world: Vector) -> Vector {
        self.rotation.apply_inverse(world.minus(self.position))
    }

    fn contains(&self, world: Vector) -> bool {
        let local = self.to_local(world);
        local.at_least(self.min) && self.max.at_least(local)
    }

    fn size(&self) -> Vector {
        self.max.minus(self.min).map(|side| side.max(1.0))
    }

    fn volume(&self) -> f64 {
        let size = self.size();
        size.x * size.y * size.z
    }

    fn longest_side(&self) -> f64 {
        let size = self.size();
        size.x.max(size.y).max(size.z)
    }

    fn distance(&self, other: &Self) -> f64 {
        let apart = self.position.minus(other.position);
        apart.dot(apart).sqrt()
    }

    /// Whether the boxes around the two turned objects meet.
    fn boxes_meet(&self, other: &Self) -> bool {
        self.world_min.at_most(other.world_max) && other.world_min.at_most(self.world_max)
    }

    /// The squares that the box around the turned object reaches into.
    fn squares(&self) -> (i64, i64, i64, i64) {
        (
            square(self.world_min.x),
            square(self.world_min.y),
            square(self.world_max.x),
            square(self.world_max.y),
        )
    }

    /// The square in which the common part of two boxes begins.
    fn common_start(&self, other: &Self) -> (i64, i64) {
        (
            square(self.world_min.x.max(other.world_min.x)),
            square(self.world_min.y.max(other.world_min.y)),
        )
    }

    /// How much of this object lies inside `other`, from 0 to 1, measured
    /// at points spread evenly through this object's box.
    fn share_inside(&self, other: &Self) -> f64 {
        let size = self.max.minus(self.min);
        let step = |index: u32| (f64::from(index) + 0.5) / f64::from(SAMPLES);
        let mut inside = 0u32;
        for i in 0..SAMPLES {
            for j in 0..SAMPLES {
                for k in 0..SAMPLES {
                    let local = Vector::new(
                        self.min.x + size.x * step(i),
                        self.min.y + size.y * step(j),
                        self.min.z + size.z * step(k),
                    );
                    if other.contains(self.to_world(local)) {
                        inside += 1;
                    }
                }
            }
        }
        f64::from(inside) / f64::from(SAMPLES.pow(3))
    }
}

/// The square a coordinate lies in.
#[allow(clippy::cast_possible_truncation)]
fn square(coordinate: f64) -> i64 {
    // Coordinates are bounded by the size of the world, far inside i64.
    (coordinate / BUCKET).floor().clamp(-1.0e12, 1.0e12) as i64
}

fn corners(min: Vector, max: Vector) -> [Vector; 8] {
    let pick = |x: bool, y: bool, z: bool| {
        Vector::new(
            if x { max.x } else { min.x },
            if y { max.y } else { min.y },
            if z { max.z } else { min.z },
        )
    };
    [
        pick(false, false, false),
        pick(true, false, false),
        pick(false, true, false),
        pick(true, true, false),
        pick(false, false, true),
        pick(true, false, true),
        pick(false, true, true),
        pick(true, true, true),
    ]
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Vector {
    x: f64,
    y: f64,
    z: f64,
}

impl Vector {
    fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    fn map(self, change: impl Fn(f64) -> f64) -> Self {
        Self::new(change(self.x), change(self.y), change(self.z))
    }

    fn scaled(self, factor: f64) -> Self {
        self.map(|value| value * factor)
    }

    fn plus(self, other: Self) -> Self {
        Self::new(self.x + other.x, self.y + other.y, self.z + other.z)
    }

    fn minus(self, other: Self) -> Self {
        Self::new(self.x - other.x, self.y - other.y, self.z - other.z)
    }

    fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    fn min(self, other: Self) -> Self {
        Self::new(
            self.x.min(other.x),
            self.y.min(other.y),
            self.z.min(other.z),
        )
    }

    fn max(self, other: Self) -> Self {
        Self::new(
            self.x.max(other.x),
            self.y.max(other.y),
            self.z.max(other.z),
        )
    }

    /// Whether every coordinate is at least the other's.
    fn at_least(self, other: Self) -> bool {
        self.x >= other.x && self.y >= other.y && self.z >= other.z
    }

    /// Whether every coordinate is at most the other's.
    fn at_most(self, other: Self) -> bool {
        other.at_least(self)
    }
}

impl From<[i32; 3]> for Vector {
    fn from([x, y, z]: [i32; 3]) -> Self {
        Self::new(f64::from(x), f64::from(y), f64::from(z))
    }
}

/// A rotation, as three rows.
#[derive(Debug, Clone, Copy)]
struct Matrix {
    rows: [Vector; 3],
}

impl Matrix {
    /// The rotation of a placed object turned by `x`, `y` and `z` radians.
    ///
    /// The game turns objects clockwise about each axis, first about x,
    /// then y, then z: an object turned a quarter about z faces east
    /// instead of north.
    fn placed(x: f64, y: f64, z: f64) -> Self {
        let about_x = |angle: f64| {
            let (sin, cos) = angle.sin_cos();
            Self::from_rows([1.0, 0.0, 0.0], [0.0, cos, -sin], [0.0, sin, cos])
        };
        let about_y = |angle: f64| {
            let (sin, cos) = angle.sin_cos();
            Self::from_rows([cos, 0.0, sin], [0.0, 1.0, 0.0], [-sin, 0.0, cos])
        };
        let about_z = |angle: f64| {
            let (sin, cos) = angle.sin_cos();
            Self::from_rows([cos, -sin, 0.0], [sin, cos, 0.0], [0.0, 0.0, 1.0])
        };
        about_z(-z).times(about_y(-y)).times(about_x(-x))
    }

    fn from_rows(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> Self {
        let row = |[x, y, z]: [f64; 3]| Vector::new(x, y, z);
        Self {
            rows: [row(a), row(b), row(c)],
        }
    }

    fn columns(&self) -> [Vector; 3] {
        let [a, b, c] = self.rows;
        [
            Vector::new(a.x, b.x, c.x),
            Vector::new(a.y, b.y, c.y),
            Vector::new(a.z, b.z, c.z),
        ]
    }

    fn apply(&self, vector: Vector) -> Vector {
        let [a, b, c] = self.rows;
        Vector::new(a.dot(vector), b.dot(vector), c.dot(vector))
    }

    /// The inverse rotation, which for a rotation is the transposed one.
    fn apply_inverse(&self, vector: Vector) -> Vector {
        let [a, b, c] = self.columns();
        Vector::new(a.dot(vector), b.dot(vector), c.dot(vector))
    }

    fn times(&self, other: Self) -> Self {
        let columns = other.columns();
        let row = |row: Vector| {
            let [a, b, c] = columns;
            Vector::new(row.dot(a), row.dot(b), row.dot(c))
        };
        let [a, b, c] = self.rows;
        Self {
            rows: [row(a), row(b), row(c)],
        }
    }
}

#[cfg(test)]
mod tests {
    use mallet_core::facts::PluginFacts;
    use mallet_core::facts::objects::{BaseKind, Bounds, Moved, ObjectFacts, PlaceName, Pose};
    use mallet_core::{ArgValue, Confidence, Facts, NoModSource, Severity};

    use super::*;

    const HOUSE: u32 = 0x10;
    const TOWER: u32 = 0x11;
    const BARREL: u32 = 0x12;

    fn id(plugin: &str, local_id: u32) -> RecordId {
        RecordId {
            plugin: plugin.to_lowercase(),
            local_id,
        }
    }

    fn base(local_id: u32, name: &str, min: [i32; 3], max: [i32; 3]) -> BaseObject {
        BaseObject {
            name: None,
            model: None,
            id: id("Skyrim.esm", local_id),
            editor_id: Some(name.to_owned()),
            kind: BaseKind::Static,
            solid: true,
            natural: false,
            bounds: Bounds { min, max },
        }
    }

    fn bases() -> Vec<BaseObject> {
        vec![
            base(HOUSE, "House", [-400, -300, 0], [400, 300, 600]),
            base(TOWER, "Tower", [-200, -200, 0], [200, 200, 500]),
            base(BARREL, "Barrel", [-30, -30, 0], [30, 30, 80]),
        ]
    }

    fn town() -> RecordId {
        id("Skyrim.esm", 0x3C)
    }

    fn placed(
        plugin: &str,
        local_id: u32,
        base: u32,
        position: [i32; 3],
        turn: i32,
    ) -> PlacedObject {
        PlacedObject {
            id: id(plugin, local_id),
            plugin: plugin.to_owned(),
            base: id("Skyrim.esm", base),
            place: Place::Exterior { worldspace: town() },
            position,
            rotation: [0, 0, turn],
            scale: 1000,
            persistent: false,
            large: false,
            moved: None,
        }
    }

    fn plugin(name: &str, masters: &[&str]) -> PluginFacts {
        PluginFacts {
            name: name.to_owned(),
            path: GamePath::new("data").join(name),
            masters: masters.iter().map(|master| (*master).to_owned()).collect(),
            npcs: Vec::new(),
        }
    }

    fn facts(plugins: Vec<PluginFacts>, placed: Vec<PlacedObject>) -> Facts {
        Facts {
            plugins,
            objects: ObjectFacts {
                placed,
                bases: bases(),
                place_names: vec![PlaceName {
                    id: town(),
                    editor_id: "Town".to_owned(),
                }],
            },
            ..Facts::default()
        }
    }

    fn two_mods() -> Vec<PluginFacts> {
        vec![
            plugin("Skyrim.esm", &[]),
            plugin("Village.esp", &["Skyrim.esm"]),
            plugin("Hamlet.esp", &["Skyrim.esm"]),
        ]
    }

    fn findings(facts: &Facts) -> Vec<Finding> {
        PlacedObjects.run(&Context::new(facts, &NoModSource))
    }

    #[test]
    fn the_same_house_placed_by_two_mods_is_reported_once_for_the_pair() {
        let facts = facts(
            two_mods(),
            vec![
                placed("Village.esp", 1, HOUSE, [5000, 9000, 0], 0),
                placed("Hamlet.esp", 1, HOUSE, [5010, 9005, 2], 0),
                placed("Village.esp", 2, HOUSE, [-5000, -100, 0], 0),
                placed("Hamlet.esp", 2, HOUSE, [-5000, -100, 0], 0),
            ],
        );
        let findings = findings(&facts);

        let [finding] = findings.as_slice() else {
            panic!("expected one finding, got {findings:?}");
        };
        assert_eq!(finding.kind, "objects.duplicated");
        assert_eq!(finding.severity, Severity::Warning);
        assert_eq!(finding.confidence, Confidence::Likely);
        assert_eq!(
            finding.id.as_str(),
            "objects.duplicated:pair:file:data/hamlet.esp+file:data/village.esp"
        );
        assert_eq!(finding.args["first"], ArgValue::from("Hamlet.esp"));
        assert_eq!(finding.args["second"], ArgValue::from("Village.esp"));
        assert_eq!(finding.args["count"], ArgValue::Number(2));
        assert_eq!(
            finding.args["examples"],
            ArgValue::from("House (Town -2, -1), House (Town 1, 2)")
        );
    }

    #[test]
    fn a_house_of_the_game_that_one_mod_moves_onto_another_mods_house_is_reported() {
        let moved = PlacedObject {
            moved: Some(Moved {
                by: "village.esp".to_owned(),
                from: Some(Pose {
                    position: [-5000, -5000, 0],
                    rotation: [0; 3],
                    scale: 1000,
                }),
            }),
            plugin: "Village.esp".to_owned(),
            ..placed("Skyrim.esm", 0x99, HOUSE, [5000, 9000, 0], 0)
        };
        let facts = facts(
            two_mods(),
            vec![moved, placed("Hamlet.esp", 1, HOUSE, [5010, 9005, 2], 0)],
        );
        let findings = findings(&facts);

        let [finding] = findings.as_slice() else {
            panic!("expected one finding, got {findings:?}");
        };
        assert_eq!(
            finding.id.as_str(),
            "objects.duplicated:pair:file:data/hamlet.esp+file:data/village.esp"
        );
    }

    #[test]
    fn a_clash_that_was_there_before_the_move_is_not_the_movers() {
        let nudged = PlacedObject {
            moved: Some(Moved {
                by: "village.esp".to_owned(),
                from: Some(Pose {
                    position: [5000, 9040, 0],
                    rotation: [0; 3],
                    scale: 1000,
                }),
            }),
            plugin: "Village.esp".to_owned(),
            ..placed("Skyrim.esm", 0x99, HOUSE, [5000, 9000, 0], 0)
        };
        let facts = facts(
            two_mods(),
            vec![nudged, placed("Hamlet.esp", 1, HOUSE, [5010, 9005, 2], 0)],
        );
        assert_eq!(findings(&facts), Vec::new());
    }

    #[test]
    fn a_plugin_whose_objects_all_stand_in_another_is_contained_in_it() {
        let mut plugins = two_mods();
        plugins.push(plugin("Hamlet Patch.esp", &["Hamlet.esp"]));
        let mut objects = Vec::new();
        for index in 0..6u32 {
            let position = [i32::try_from(index).unwrap() * 5000, 0, 0];
            objects.push(placed("Village.esp", index, HOUSE, position, 0));
            objects.push(placed("Hamlet.esp", index, HOUSE, position, 0));
        }
        // The village has more of its own.
        objects.push(placed("Village.esp", 99, TOWER, [90_000, 0, 0], 0));
        let facts = facts(plugins, objects);

        let findings = findings(&facts);
        let [finding] = findings.as_slice() else {
            panic!("expected one finding, got {findings:?}");
        };
        assert_eq!(finding.kind, "plugins.contained");
        assert_eq!(finding.args["plugin"], ArgValue::from("Hamlet.esp"));
        assert_eq!(finding.args["container"], ArgValue::from("Village.esp"));
        assert_eq!(finding.args["count"], ArgValue::Number(6));
        assert_eq!(finding.args["total"], ArgValue::Number(6));
        assert_eq!(finding.args["dependentCount"], ArgValue::Number(1));
        assert_eq!(
            finding.args["dependents"],
            ArgValue::from("Hamlet Patch.esp")
        );

        // Every collision is a spot on the map, with the way there.
        let context = Context::new(&facts, &NoModSource);
        let spots = &context.collisions().spots;
        assert_eq!(spots.len(), 6);
        assert!(spots.iter().all(|spot| spot.finding == finding.id));
        assert!(spots.iter().any(|spot| spot.console == "cow Town 0 0"));
        // Each spot names both copies; the one of the smaller plugin is the
        // one suggested to hide.
        let spot = &spots[0];
        let plugins: Vec<_> = spot
            .parts
            .iter()
            .map(|part| (part.object.plugin.as_str(), part.suggested))
            .collect();
        assert!(plugins.contains(&("Hamlet.esp", true)), "{plugins:?}");
        assert!(plugins.contains(&("Village.esp", false)), "{plugins:?}");
    }

    #[test]
    fn two_buildings_standing_inside_each_other_are_reported() {
        let facts = facts(
            two_mods(),
            vec![
                placed("Village.esp", 1, HOUSE, [0, 0, 0], 0),
                placed("Hamlet.esp", 1, TOWER, [100, 0, 0], 0),
            ],
        );
        let findings = findings(&facts);

        let [finding] = findings.as_slice() else {
            panic!("expected one finding, got {findings:?}");
        };
        assert_eq!(finding.kind, "objects.overlapping");
        assert_eq!(finding.confidence, Confidence::Hint);
        assert_eq!(
            finding.args["examples"],
            ArgValue::from("House / Tower (Town 0, 0)")
        );
    }

    #[test]
    fn objects_that_only_touch_or_stand_apart_are_not_reported() {
        let facts = facts(
            two_mods(),
            vec![
                placed("Village.esp", 1, HOUSE, [0, 0, 0], 0),
                // Side by side, sharing a strip of wall.
                placed("Hamlet.esp", 1, TOWER, [580, 0, 0], 0),
                // Far away.
                placed("Hamlet.esp", 2, HOUSE, [20_000, 0, 0], 0),
            ],
        );
        assert_eq!(findings(&facts), Vec::new());
    }

    #[test]
    fn a_turned_house_is_compared_by_its_turned_box() {
        // A long house, turned a quarter, reaches 400 units north and
        // south, and only 300 east and west.
        let tower_east = |turn| {
            facts(
                two_mods(),
                vec![
                    placed("Village.esp", 1, HOUSE, [0, 0, 0], turn),
                    placed("Hamlet.esp", 1, TOWER, [380, 0, 0], 0),
                ],
            )
        };
        // Not turned, the house reaches 400 east: more than half the tower
        // is in it.
        assert_eq!(findings(&tower_east(0)).len(), 1);
        // Turned, it reaches 300 east: the tower stands next to it.
        assert_eq!(findings(&tower_east(1571)), Vec::new());
    }

    #[test]
    fn a_barrel_inside_a_house_and_a_mods_own_objects_are_left_alone() {
        let facts = facts(
            two_mods(),
            vec![
                placed("Village.esp", 1, HOUSE, [0, 0, 0], 0),
                placed("Hamlet.esp", 1, BARREL, [0, 0, 100], 0),
                // The same mod's own objects may stand in each other.
                placed("Village.esp", 2, TOWER, [0, 0, 0], 0),
            ],
        );
        assert_eq!(findings(&facts), Vec::new());
    }

    #[test]
    fn plugins_that_know_each_other_are_left_alone() {
        let objects = vec![
            placed("Village.esp", 1, HOUSE, [0, 0, 0], 0),
            placed("Hamlet.esp", 1, HOUSE, [0, 0, 0], 0),
        ];
        // The hamlet builds on the village.
        let addon = facts(
            vec![
                plugin("Village.esp", &["Skyrim.esm"]),
                plugin("Hamlet.esp", &["Skyrim.esm", "Village.esp"]),
            ],
            objects.clone(),
        );
        assert_eq!(findings(&addon), Vec::new());

        // A patch builds on both.
        let mut patched = two_mods();
        patched.push(plugin(
            "Village - Hamlet Patch.esp",
            &["Village.esp", "Hamlet.esp"],
        ));
        assert_eq!(findings(&facts(patched, objects)), Vec::new());
    }

    #[test]
    fn two_rocks_set_into_each_other_are_left_alone_but_not_twice_the_same() {
        let mut facts = facts(
            two_mods(),
            vec![
                placed("Village.esp", 1, HOUSE, [0, 0, 0], 0),
                placed("Hamlet.esp", 1, TOWER, [100, 0, 0], 0),
                placed("Village.esp", 2, HOUSE, [9000, 0, 0], 0),
                placed("Hamlet.esp", 2, HOUSE, [9000, 0, 0], 0),
            ],
        );
        for base in &mut facts.objects.bases {
            base.natural = true;
        }
        let kinds: Vec<_> = findings(&facts).into_iter().map(|f| f.kind).collect();
        assert_eq!(kinds, ["objects.duplicated"]);
    }

    #[test]
    fn a_persistent_copy_is_never_suggested_for_hiding() {
        let mut village = placed("Village.esp", 1, HOUSE, [0, 0, 0], 0);
        let mut hamlet = placed("Hamlet.esp", 1, HOUSE, [0, 0, 0], 0);
        village.persistent = false;
        hamlet.persistent = true;
        let facts = facts(two_mods(), vec![village, hamlet]);
        let context = Context::new(&facts, &NoModSource);
        let [spot] = context.collisions().spots.as_slice() else {
            panic!("expected one spot");
        };
        let suggested: Vec<_> = spot
            .parts
            .iter()
            .filter(|part| part.suggested)
            .map(|part| part.object.plugin.as_str())
            .collect();
        assert_eq!(suggested, ["Village.esp"]);
        assert!(spot.parts.iter().any(|part| !part.hideable));
    }

    #[test]
    fn what_is_not_solid_is_left_alone() {
        let mut facts = facts(
            two_mods(),
            vec![
                placed("Village.esp", 1, HOUSE, [0, 0, 0], 0),
                placed("Hamlet.esp", 1, HOUSE, [0, 0, 0], 0),
            ],
        );
        for base in &mut facts.objects.bases {
            base.solid = false;
        }
        assert_eq!(findings(&facts), Vec::new());
    }

    #[test]
    fn objects_in_different_places_never_meet() {
        let mut inside = placed("Hamlet.esp", 1, HOUSE, [0, 0, 0], 0);
        inside.place = Place::Interior {
            cell: id("Hamlet.esp", 0x800),
        };
        let facts = facts(
            two_mods(),
            vec![placed("Village.esp", 1, HOUSE, [0, 0, 0], 0), inside],
        );
        assert_eq!(findings(&facts), Vec::new());
    }

    #[test]
    fn a_quarter_turn_about_z_turns_north_into_east() {
        let turned = Matrix::placed(0.0, 0.0, std::f64::consts::FRAC_PI_2);
        let east = turned.apply(Vector::new(0.0, 1.0, 0.0));
        assert!(
            (east.x - 1.0).abs() < 1e-9 && east.y.abs() < 1e-9,
            "{east:?}"
        );
        let back = turned.apply_inverse(east);
        assert!((back.y - 1.0).abs() < 1e-9, "{back:?}");
    }

    /// A box-shaped model from `min` to `max`, as twelve triangles.
    fn box_model(min: [i32; 3], max: [i32; 3]) -> ModelMesh {
        let mut points = Vec::new();
        for corner in 0..8u8 {
            let pick = |axis: usize| {
                if corner >> axis & 1 == 1 {
                    max[axis]
                } else {
                    min[axis]
                }
            };
            points.push([pick(0), pick(1), pick(2)]);
        }
        let faces = [
            [0, 1, 3, 2],
            [4, 6, 7, 5],
            [0, 4, 5, 1],
            [2, 3, 7, 6],
            [0, 2, 6, 4],
            [1, 5, 7, 3],
        ];
        ModelMesh {
            points,
            triangles: faces
                .iter()
                .flat_map(|[a, b, c, d]| [[*a, *b, *c], [*a, *c, *d]])
                .collect(),
        }
    }

    /// Facts in which the house and the tower have models: the house's
    /// fills only the western end of its box, as an arch leaves its middle
    /// empty; the tower's fills its box.
    fn facts_with_models(placed: Vec<PlacedObject>) -> Facts {
        let mut facts = facts(two_mods(), placed);
        for base in &mut facts.objects.bases {
            base.model = Some(format!("meshes/{}.nif", base.id.local_id));
        }
        facts.models.meshes.insert(
            format!("meshes/{HOUSE}.nif"),
            box_model([-400, -300, 0], [-300, 300, 600]),
        );
        facts.models.meshes.insert(
            format!("meshes/{TOWER}.nif"),
            box_model([-200, -200, 0], [200, 200, 500]),
        );
        facts
    }

    #[test]
    fn boxes_that_overlap_but_whose_models_do_not_meet_are_not_a_clash() {
        // The tower stands in the empty middle of the house's box.
        let placed = vec![
            placed("Village.esp", 1, HOUSE, [0, 0, 0], 0),
            placed("Hamlet.esp", 1, TOWER, [50, 0, 0], 0),
        ];
        assert!(
            findings(&facts(two_mods(), placed.clone()))
                .iter()
                .any(|finding| finding.kind == "objects.overlapping")
        );
        assert_eq!(findings(&facts_with_models(placed)), Vec::new());
    }

    #[test]
    fn models_that_cut_into_each_other_are_reported_as_touching() {
        // The tower reaches 100 units into the house's solid western end.
        let facts = facts_with_models(vec![
            placed("Village.esp", 1, HOUSE, [0, 0, 0], 0),
            placed("Hamlet.esp", 1, TOWER, [-500, 0, 0], 0),
        ]);
        let context = Context::new(&facts, &NoModSource);
        let collisions = context.collisions();
        let [finding] = collisions.findings.as_slice() else {
            panic!("expected one finding, got {:?}", collisions.findings);
        };
        assert_eq!(finding.kind, "objects.touching");
        assert_eq!(finding.severity, Severity::Info);
        let [spot] = collisions.spots.as_slice() else {
            panic!("expected one spot");
        };
        assert_eq!(spot.kind, SpotKind::Touch);
        assert!(spot.depth.is_some_and(|depth| depth >= 100), "{spot:?}");
    }

    #[test]
    fn touches_are_only_reported_where_the_models_tell() {
        let facts = facts(
            two_mods(),
            vec![
                placed("Village.esp", 1, HOUSE, [0, 0, 0], 0),
                placed("Hamlet.esp", 1, TOWER, [-500, 0, 0], 0),
            ],
        );
        assert_eq!(findings(&facts), Vec::new());
        let context = Context::new(&facts, &NoModSource);
        assert_eq!(
            models_to_check(&context),
            BTreeSet::new(),
            "without model paths nothing can be read"
        );
    }

    #[test]
    fn the_models_of_possible_clashes_are_asked_for() {
        let mut facts = facts_with_models(vec![
            placed("Village.esp", 1, HOUSE, [0, 0, 0], 0),
            placed("Hamlet.esp", 1, TOWER, [-500, 0, 0], 0),
            placed("Hamlet.esp", 2, BARREL, [9000, 9000, 0], 0),
        ]);
        facts.models = ModelFacts::default();
        let context = Context::new(&facts, &NoModSource);
        assert_eq!(
            models_to_check(&context),
            BTreeSet::from([format!("meshes/{HOUSE}.nif"), format!("meshes/{TOWER}.nif")])
        );
    }
}
