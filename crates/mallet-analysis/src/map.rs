//! Where mods place objects, cell by cell.
//!
//! The map shows each worldspace with land or places as a grid of cells
//! over its land, which mods place objects in each cell, the places of the
//! game's map and the spots where objects of two mods collide. A
//! worldspace that uses its parent's land, such as a city inside its
//! walls, is shown on its parent's map. It is built from the same facts as the findings and
//! decides nothing.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use mallet_core::facts::RecordId;
use mallet_core::facts::markers::MapMarkerFact;
use mallet_core::facts::objects::{Place, PlacedObject};
use mallet_core::facts::terrain::{SAMPLES_PER_CELL, TerrainWorld, WorldspaceFacts};
use mallet_core::{
    MapBounds, MapCell, MapMarker, MapSpot, MapTerrain, MapWorld, SpotLocation, WorldMap,
    WorldOrigin,
};

use crate::Context;
use crate::objects::Owners;
use crate::places::{self, Origin};

/// The objects of one cell: how many, and which plugins place them.
type CellCount<'a> = (u32, BTreeSet<&'a str>);

/// The map of the setup in `context`.
pub fn world_map<'a>(context: &Context<'a>) -> WorldMap {
    let facts: &'a mallet_core::Facts = context.facts;
    let objects = &facts.objects;
    let terrain_facts = &facts.terrain;
    let names = internal_names(facts);
    let name_of = |worldspace: &RecordId| {
        names
            .get(worldspace)
            .map_or_else(|| worldspace.to_string(), |name| (*name).to_owned())
    };
    let records: HashMap<&'a RecordId, &'a WorldspaceFacts> = terrain_facts
        .worldspaces
        .iter()
        .map(|world| (&world.id, world))
        .collect();
    let lands: HashMap<&RecordId, &TerrainWorld> = terrain_facts
        .worlds
        .iter()
        .map(|land| (&land.worldspace, land))
        .collect();
    let host = |worldspace: &'a RecordId| -> &'a RecordId { host_of(worldspace, &records) };

    let mut worlds = count_cells(&objects.placed, &host);
    // Every worldspace with land or places is on the map, whether or not
    // mods place objects in it.
    let places_by_world = places_by_world(&facts.markers.markers, &host);
    for worldspace in places_by_world.keys() {
        worlds.entry(worldspace).or_default();
    }
    for land in &terrain_facts.worlds {
        worlds.entry(host(&land.worldspace)).or_default();
    }
    let mut children = children_of(&terrain_facts.worldspaces, &host, &name_of);
    let spots = &context.collisions().spots;
    let spot_worlds = spot_worlds(spots);

    // Plugins of the same mod become one owner, and owners are listed by
    // name.
    let owners = Owners::new(context);
    let display: HashMap<&str, String> = worlds
        .values()
        .flat_map(|cells| cells.values())
        .flat_map(|(_, plugins)| plugins.iter().copied())
        .chain(place_plugins(&places_by_world))
        .map(|plugin| (plugin, owners.display(plugin)))
        .collect();
    let names_of_owners: Vec<String> = display
        .values()
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let index_of: HashMap<&str, u32> = names_of_owners
        .iter()
        .zip(0u32..)
        .map(|(name, index)| (name.as_str(), index))
        .collect();
    let owner = |plugin: &str| {
        display
            .get(plugin)
            .and_then(|name| index_of.get(name.as_str()))
            .copied()
            .unwrap_or_default()
    };

    // The worlds the game's first master defines: its main world is one.
    let first_master = facts
        .plugins
        .first()
        .map(|plugin| plugin.name.to_lowercase());
    let mut of_first_master: BTreeSet<String> = BTreeSet::new();
    let mut map_worlds: Vec<MapWorld> = worlds
        .into_iter()
        .map(|(worldspace, cells)| {
            let name = name_of(worldspace);
            if first_master.as_deref() == Some(worldspace.plugin.as_str()) {
                of_first_master.insert(name.clone());
            }
            let children = children.remove(worldspace).unwrap_or_default();
            let has_spot = std::iter::once(&name)
                .chain(&children)
                .any(|world| spot_worlds.contains(world.as_str()));
            let mut world = MapWorld {
                cells: map_cells(cells, &owner),
                markers: world_markers(&places_by_world, worldspace, &name_of, &owner),
                children,
                name,
                ..MapWorld::default()
            };
            let record = records.get(worldspace).copied();
            describe(&mut world, record, lands.get(worldspace).copied(), has_spot);
            world.origin_name =
                (world.origin != WorldOrigin::Game).then(|| owners.display(&worldspace.plugin));
            world
        })
        .collect();
    order(&mut map_worlds, &of_first_master);

    WorldMap {
        owners: names_of_owners,
        worlds: map_worlds,
        spots: spots
            .iter()
            .chain(&context.doors().spots)
            .cloned()
            .collect(),
    }
}

/// The internal names of places and worldspaces, a worldspace's own record
/// having the last word.
fn internal_names(facts: &mallet_core::Facts) -> HashMap<&RecordId, &str> {
    facts
        .objects
        .place_names
        .iter()
        .map(|name| (&name.id, name.editor_id.as_str()))
        .chain(
            facts
                .terrain
                .worldspaces
                .iter()
                .filter_map(|world| Some((&world.id, world.editor_id.as_deref()?))),
        )
        .collect()
}

/// The cells of a world for the report, each with its owners' positions
/// among the owners of the map.
fn map_cells(
    cells: BTreeMap<(i32, i32), CellCount<'_>>,
    owner: &dyn Fn(&str) -> u32,
) -> Vec<MapCell> {
    cells
        .into_iter()
        .map(|((x, y), (objects, plugins))| {
            let mut owners: Vec<u32> = plugins.into_iter().map(owner).collect();
            owners.sort_unstable();
            owners.dedup();
            MapCell {
                x,
                y,
                objects,
                owners,
            }
        })
        .collect()
}

/// Fills in what the world's own record and land say: its title, origin,
/// bounds, land and whether it is small. `has_spot` keeps it from being
/// folded away.
fn describe(
    world: &mut MapWorld,
    record: Option<&WorldspaceFacts>,
    land: Option<&TerrainWorld>,
    has_spot: bool,
) {
    let land_cells = land.map_or(0, |land| land.cells.len());
    let bounds = record
        .and_then(|record| record.bounds)
        .or_else(|| land.and_then(land_bounds));
    let shown = spanning(bounds, &occupied(&world.cells, &world.markers));
    world.terrain = land.and_then(|land| terrain(land, shown));
    world.title = record
        .and_then(|record| record.title.clone())
        .unwrap_or_else(|| readable(&world.name));
    world.origin = record.map_or(WorldOrigin::Mod, |record| record.origin);
    world.bounds = bounds.map(|[west, south, east, north]| MapBounds {
        west,
        south,
        east,
        north,
    });
    world.land_cells = u32::try_from(land_cells).unwrap_or(u32::MAX);
    world.small = (record.is_some_and(|record| record.small) || land_cells < SMALL) && !has_spot;
}

/// Per worldspace and cell: how many objects mods place, and which
/// plugins. A worldspace drawn on another's map, as `host` says, is
/// counted there.
fn count_cells<'a>(
    placed: &'a [PlacedObject],
    host: &dyn Fn(&'a RecordId) -> &'a RecordId,
) -> BTreeMap<&'a RecordId, BTreeMap<(i32, i32), CellCount<'a>>> {
    let mut worlds: BTreeMap<&RecordId, BTreeMap<(i32, i32), CellCount<'_>>> = BTreeMap::new();
    for object in placed {
        let Place::Exterior { worldspace } = &object.place else {
            continue;
        };
        let [x, y, _] = object.position;
        let cell = worlds
            .entry(host(worldspace))
            .or_default()
            .entry((x.div_euclid(4096), y.div_euclid(4096)))
            .or_default();
        cell.0 = cell.0.saturating_add(1);
        cell.1.insert(object.owner());
    }
    worlds
}

/// The worldspaces drawn on another's map, by that one, named by
/// `name_of`.
fn children_of<'a>(
    worldspaces: &'a [WorldspaceFacts],
    host: &dyn Fn(&'a RecordId) -> &'a RecordId,
    name_of: &dyn Fn(&RecordId) -> String,
) -> BTreeMap<&'a RecordId, Vec<String>> {
    let mut children: BTreeMap<&RecordId, Vec<String>> = BTreeMap::new();
    for world in worldspaces {
        let drawn_on = host(&world.id);
        if drawn_on != &world.id {
            children
                .entry(drawn_on)
                .or_default()
                .push(name_of(&world.id));
        }
    }
    children
}

/// The worlds that hold a collision spot, which are never folded away.
fn spot_worlds(spots: &[MapSpot]) -> BTreeSet<&str> {
    spots
        .iter()
        .filter_map(|spot| match &spot.location {
            SpotLocation::Exterior { world, .. } => Some(world.as_str()),
            SpotLocation::Interior { .. } => None,
        })
        .collect()
}

/// Marks the game's main world and orders the worlds: the game first, its
/// main world leading, then the Creation Club, then mods; small worlds
/// last. The main world is the largest of those the game's first master
/// defines (`of_first_master`, by name), else of all the game's: not
/// Solstheim, whose grid of distant land is larger than Skyrim's.
fn order(worlds: &mut [MapWorld], of_first_master: &BTreeSet<String>) {
    if let Some(main) = worlds
        .iter_mut()
        .filter(|world| world.origin == WorldOrigin::Game)
        .max_by_key(|world| (of_first_master.contains(&world.name), world.land_cells))
    {
        main.main = true;
        main.small = false;
    }
    worlds.sort_by(|a, b| {
        (a.small, a.origin, !a.main, a.title.to_lowercase(), &a.name).cmp(&(
            b.small,
            b.origin,
            !b.main,
            b.title.to_lowercase(),
            &b.name,
        ))
    });
}

/// The most height samples a world's picture of its land holds.
const MAX_SAMPLES: usize = 1_000_000;

/// Worlds with fewer cells of land than this are small.
const SMALL: usize = 64;

/// The worldspace on whose map `worldspace` is drawn: its parent, if it
/// uses the parent's land, and so on up; else itself.
fn host_of<'a>(
    worldspace: &'a RecordId,
    records: &HashMap<&RecordId, &'a WorldspaceFacts>,
) -> &'a RecordId {
    let mut current = worldspace;
    // A chain of parents is short; a loop in it is not followed for ever.
    for _ in 0..8 {
        match records
            .get(current)
            .and_then(|record| record.parent.as_ref().filter(|_| record.uses_parent_land))
        {
            Some(parent) if parent != current => current = parent,
            _ => break,
        }
    }
    current
}

/// The cells with land of a worldspace, as west, south, east, north.
fn land_bounds(land: &TerrainWorld) -> Option<[i32; 4]> {
    Some([
        land.cells.iter().map(|cell| cell.x).min()?,
        land.cells.iter().map(|cell| cell.y).min()?,
        land.cells.iter().map(|cell| cell.x).max()?,
        land.cells.iter().map(|cell| cell.y).max()?,
    ])
}

/// The cells whose land is shown: the world's bounds, widened to the
/// cells where mods place objects or places lie, as west, east, south,
/// north. `None` for all the land.
fn spanning(bounds: Option<[i32; 4]>, occupied: &[(i32, i32)]) -> Option<(i32, i32, i32, i32)> {
    let [west, south, east, north] = bounds?;
    Some((
        occupied.iter().map(|(x, _)| *x).fold(west, i32::min),
        occupied.iter().map(|(x, _)| *x).fold(east, i32::max),
        occupied.iter().map(|(_, y)| *y).fold(south, i32::min),
        occupied.iter().map(|(_, y)| *y).fold(north, i32::max),
    ))
}

/// A worldspace's internal name made readable, for a world the game gives
/// no name: known prefixes dropped, words split
/// (`ccBGSSSE031BlackboneIsle` becomes `Blackbone Isle`).
fn readable(name: &str) -> String {
    let mut rest = name;
    // Creation Club content: `cc`, its author's letters, its number.
    if let Some(after) = rest.strip_prefix("cc") {
        let letters = after.trim_start_matches(|c: char| c.is_ascii_uppercase());
        let digits = letters.trim_start_matches(|c: char| c.is_ascii_digit());
        if letters.len() < after.len() && digits.len() < letters.len() && !digits.is_empty() {
            rest = digits;
        }
    }
    // The game's DLC: `DLC1`, `DLC01`, `DLC2`.
    if let Some(after) = rest.strip_prefix("DLC") {
        let digits = after.trim_start_matches(|c: char| c.is_ascii_digit());
        if digits.len() < after.len() && !digits.is_empty() {
            rest = digits;
        }
    }
    for suffix in ["Worldspace", "World"] {
        if let Some(before) = rest.strip_suffix(suffix)
            && !before.is_empty()
        {
            rest = before;
            break;
        }
    }
    let mut words = String::new();
    let mut previous: Option<char> = None;
    for character in rest.chars() {
        let character = if character == '_' { ' ' } else { character };
        if let Some(before) = previous {
            let new_word = (character.is_ascii_uppercase() && before.is_ascii_lowercase())
                || (character.is_ascii_digit() && before.is_ascii_alphabetic())
                || (character.is_ascii_alphabetic() && before.is_ascii_digit());
            if new_word && before != ' ' {
                words.push(' ');
            }
        }
        words.push(character);
        previous = Some(character);
    }
    let words = words.split_whitespace().collect::<Vec<_>>().join(" ");
    if words.is_empty() {
        name.to_owned()
    } else {
        words
    }
}

/// Map markers grouped by their worldspace, each with whose it is.
type PlacesByWorld<'a> = BTreeMap<&'a RecordId, Vec<(&'a MapMarkerFact, Origin<'a>)>>;

/// The places of the world map by the worldspace whose map shows them, and
/// whose they are. `host` gives that worldspace.
fn places_by_world<'a>(
    markers: &'a [MapMarkerFact],
    host: &dyn Fn(&'a RecordId) -> &'a RecordId,
) -> PlacesByWorld<'a> {
    let mut by_world: PlacesByWorld<'a> = BTreeMap::new();
    for marker in markers {
        by_world
            .entry(host(&marker.worldspace))
            .or_default()
            .push((marker, places::origin(marker)));
    }
    by_world
}

/// The plugins that added, changed or removed a place.
fn place_plugins<'a>(places: &'a PlacesByWorld<'a>) -> impl Iterator<Item = &'a str> {
    places
        .values()
        .flatten()
        .filter_map(|(_, origin)| origin.plugin)
}

/// The places shown on the map of `worldspace`, for the report. `name_of`
/// names the worldspace each lies in; `owner` gives the position of a
/// plugin's mod among the owners of the map.
fn world_markers(
    places: &PlacesByWorld<'_>,
    worldspace: &RecordId,
    name_of: &dyn Fn(&RecordId) -> String,
    owner: &impl Fn(&str) -> u32,
) -> Vec<MapMarker> {
    places
        .get(worldspace)
        .into_iter()
        .flatten()
        .map(|(marker, origin)| map_marker(marker, origin, &name_of(&marker.worldspace), owner))
        .collect()
}

/// The cells with objects of mods or a place, whose land is shown.
fn occupied(cells: &[MapCell], markers: &[MapMarker]) -> Vec<(i32, i32)> {
    cells
        .iter()
        .map(|cell| (cell.x, cell.y))
        .chain(markers.iter().map(|marker| {
            let [x, y, _] = marker.position;
            (x.div_euclid(4096), y.div_euclid(4096))
        }))
        .collect()
}

/// A place of the world map in `world`, for the report. `owner` gives the
/// position of a plugin's mod among the owners of the map.
fn map_marker(
    marker: &MapMarkerFact,
    origin: &Origin<'_>,
    world: &str,
    owner: impl Fn(&str) -> u32,
) -> MapMarker {
    let version = &marker.last;
    let [x, y, _] = version.position;
    MapMarker {
        name: version
            .name
            .clone()
            .or_else(|| marker.first.name.clone())
            .unwrap_or_default(),
        kind: version.kind,
        position: version.position,
        origin: origin.whose,
        owner: origin.plugin.map(owner),
        moved_from: origin.moved_from,
        renamed_from: origin.renamed_from.map(str::to_owned),
        console: format!("cow {world} {} {}", x.div_euclid(4096), y.div_euclid(4096)),
    }
}

/// The land of a worldspace as a height image, limited to `within` (west,
/// east, south, north) where given.
fn terrain(land: &TerrainWorld, within: Option<(i32, i32, i32, i32)>) -> Option<MapTerrain> {
    let (min_x, max_x, min_y, max_y) = within.unwrap_or((i32::MIN, i32::MAX, i32::MIN, i32::MAX));
    let cells: Vec<_> = land
        .cells
        .iter()
        .filter(|cell| (min_x..=max_x).contains(&cell.x) && (min_y..=max_y).contains(&cell.y))
        .collect();
    let west = cells.iter().map(|cell| cell.x).min()?;
    let east = cells.iter().map(|cell| cell.x).max()?;
    let south = cells.iter().map(|cell| cell.y).min()?;
    let north = cells.iter().map(|cell| cell.y).max()?;
    let columns = usize::try_from(east - west + 1).ok()?;
    let rows = usize::try_from(north - south + 1).ok()?;
    // A large world keeps every second or fourth sample, so that its
    // picture stays small enough to send.
    let mut step = 1;
    while step < SAMPLES_PER_CELL && columns * rows * (SAMPLES_PER_CELL / step).pow(2) > MAX_SAMPLES
    {
        step *= 2;
    }
    let samples = SAMPLES_PER_CELL / step;
    let width = columns * samples;

    // Ground below its cell's water shows the water surface instead, so
    // that a lake lies at its own height and not at the sea's.
    let surface = |height: i32, water: Option<i32>| match water {
        Some(water) if height <= water => (water, true),
        _ => (height, false),
    };
    let all_heights = cells.iter().flat_map(|cell| {
        cell.heights
            .iter()
            .map(move |height| surface(*height, cell.water).0)
    });
    let lowest = all_heights.clone().min().unwrap_or_default();
    let highest = all_heights.max().unwrap_or_default().max(lowest + 1);

    let mut image = vec![0u8; width * rows * samples * 2];
    let mut water_mask = vec![0u8; width * rows * samples];
    for cell in &cells {
        let column = usize::try_from(cell.x - west).ok()?;
        let row = usize::try_from(north - cell.y).ok()?;
        for (index, height) in cell.heights.iter().enumerate() {
            let (from_south, from_west) = (index / SAMPLES_PER_CELL, index % SAMPLES_PER_CELL);
            if from_south % step != 0 || from_west % step != 0 {
                continue;
            }
            // Within a cell, samples go from south to north; the image
            // goes from north to south.
            let (sample_row, sample_column) = (samples - 1 - from_south / step, from_west / step);
            let at = (row * samples + sample_row) * width + column * samples + sample_column;
            let (height, under_water) = surface(*height, cell.water);
            let share = f64::from(height - lowest) / f64::from(highest - lowest);
            if let Some(pixel) = image.get_mut(at * 2..at * 2 + 2) {
                pixel.copy_from_slice(&level(share).to_le_bytes());
            }
            if let Some(pixel) = water_mask.get_mut(at) {
                *pixel = u8::from(under_water);
            }
        }
    }
    Some(MapTerrain {
        west,
        south,
        columns: u32::try_from(columns).ok()?,
        rows: u32::try_from(rows).ok()?,
        samples: u32::try_from(samples).ok()?,
        heights: mallet_core::encoding::base64(&image),
        water: mallet_core::encoding::base64(&water_mask),
        lowest,
        highest,
    })
}

/// A share from 0 to 1 as a height value from 1 to 65535.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn level(share: f64) -> u16 {
    // Clamped to the range of the result first.
    (1.0 + share.clamp(0.0, 1.0) * 65534.0).round() as u16
}

#[cfg(test)]
mod tests {
    use mallet_core::facts::PluginFacts;
    use mallet_core::facts::objects::{ObjectFacts, PlaceName, PlacedObject};
    use mallet_core::facts::terrain::TerrainWorld;
    use mallet_core::{Facts, GamePath, NoModSource};

    use super::*;

    fn id(plugin: &str, local_id: u32) -> RecordId {
        RecordId {
            plugin: plugin.to_lowercase(),
            local_id,
        }
    }

    fn placed(plugin: &str, local_id: u32, position: [i32; 3]) -> PlacedObject {
        PlacedObject {
            id: id(plugin, local_id),
            plugin: plugin.to_owned(),
            base: id("skyrim.esm", 1),
            place: Place::Exterior {
                worldspace: id("skyrim.esm", 0x3C),
            },
            position,
            rotation: [0; 3],
            scale: 1000,
            persistent: false,
            large: false,
            moved: None,
        }
    }

    fn terrain_columns(land: &TerrainWorld, within: Option<(i32, i32, i32, i32)>) -> u32 {
        terrain(land, within).unwrap().columns
    }

    #[test]
    fn land_becomes_a_height_image_with_water_apart() {
        use mallet_core::facts::terrain::TerrainCell;
        let flat = |x: i32, height: i32, water: Option<i32>| TerrainCell {
            x,
            y: 0,
            heights: vec![height; 64],
            water,
        };
        let land = TerrainWorld {
            worldspace: id("skyrim.esm", 0x3C),
            cells: vec![
                flat(0, 100, Some(0)),
                flat(1, 500, Some(0)),
                flat(3, -50, Some(0)),
            ],
        };
        let terrain = terrain(&land, None).unwrap();
        // Limited to the first two cells.
        assert_eq!(terrain_columns(&land, Some((0, 1, 0, 0))), 2);
        assert_eq!((terrain.west, terrain.south), (0, 0));
        assert_eq!((terrain.columns, terrain.rows, terrain.samples), (4, 1, 8));
        // The water of cell 3 lies at 0, below the land.
        assert_eq!((terrain.lowest, terrain.highest), (0, 500));
        // 32 samples per row, 8 rows: cell 0 low land (100 of 0 to 500),
        // cell 1 high land, cell 2 unknown, cell 3 water at its surface.
        let row: Vec<u16> = [13_108u16, 65_535, 0, 1]
            .iter()
            .flat_map(|value| [*value; 8])
            .collect();
        let image: Vec<u8> = row
            .iter()
            .flat_map(|value| value.to_le_bytes())
            .cycle()
            .take(32 * 8 * 2)
            .collect();
        assert_eq!(terrain.heights, mallet_core::encoding::base64(&image));
        let water: Vec<u8> = [0u8, 0, 0, 1]
            .iter()
            .flat_map(|value| [*value; 8])
            .cycle()
            .take(32 * 8)
            .collect();
        assert_eq!(terrain.water, mallet_core::encoding::base64(&water));
    }

    #[test]
    fn cells_count_objects_and_name_the_mods_that_place_them() {
        let plugin = |name: &str| PluginFacts {
            name: name.to_owned(),
            path: GamePath::new("data").join(name),
            masters: Vec::new(),
            npcs: Vec::new(),
        };
        let facts = Facts {
            plugins: vec![plugin("A.esp"), plugin("B.esp")],
            objects: ObjectFacts {
                placed: vec![
                    placed("A.esp", 1, [100, 100, 0]),
                    placed("A.esp", 2, [200, 100, 0]),
                    placed("B.esp", 1, [300, 100, 0]),
                    placed("B.esp", 2, [-100, 5000, 0]),
                ],
                bases: Vec::new(),
                place_names: vec![PlaceName {
                    id: id("skyrim.esm", 0x3C),
                    editor_id: "Tamriel".to_owned(),
                }],
            },
            ..Facts::default()
        };
        let map = world_map(&Context::new(&facts, &NoModSource));

        assert_eq!(map.owners, ["A.esp", "B.esp"]);
        let [world] = map.worlds.as_slice() else {
            panic!("expected one world, got {:?}", map.worlds);
        };
        assert_eq!(world.name, "Tamriel");
        let cells: Vec<_> = world
            .cells
            .iter()
            .map(|cell| (cell.x, cell.y, cell.objects, cell.owners.clone()))
            .collect();
        assert_eq!(cells, [(-1, 1, 1, vec![1]), (0, 0, 3, vec![0, 1])]);
    }

    #[test]
    fn places_are_on_the_map_with_their_origin_and_mod() {
        use mallet_core::facts::markers::{MapMarkerFact, MarkerFacts, MarkerVersion};
        use mallet_core::{PlaceKind, PlaceOrigin};

        let version = |plugin: &str, kind: PlaceKind, name: &str, x: i32| MarkerVersion {
            plugin: plugin.to_owned(),
            official: plugin == "Skyrim.esm",
            kind,
            name: Some(name.to_owned()),
            position: [x, 0, 0],
            removed: false,
        };
        let marker = |local_id: u32, first: MarkerVersion, last: MarkerVersion| MapMarkerFact {
            id: id(&first.plugin, local_id),
            worldspace: id("skyrim.esm", 0x3C),
            first,
            last,
        };
        let facts = Facts {
            markers: MarkerFacts {
                markers: vec![
                    marker(
                        1,
                        version("Skyrim.esm", PlaceKind::Town, "Millbrook", 0),
                        version("Skyrim.esm", PlaceKind::Town, "Millbrook", 0),
                    ),
                    marker(
                        2,
                        version("Skyrim.esm", PlaceKind::Cave, "Echo Hollow", 0),
                        version("Caves.esp", PlaceKind::Cave, "Echo Hollow", 9000),
                    ),
                    marker(
                        3,
                        version("Village.esp", PlaceKind::Settlement, "New Village", -5000),
                        version("Village.esp", PlaceKind::Settlement, "New Village", -5000),
                    ),
                ],
            },
            objects: ObjectFacts {
                place_names: vec![PlaceName {
                    id: id("skyrim.esm", 0x3C),
                    editor_id: "Tamriel".to_owned(),
                }],
                ..ObjectFacts::default()
            },
            ..Facts::default()
        };
        let map = world_map(&Context::new(&facts, &NoModSource));

        assert_eq!(map.owners, ["Caves.esp", "Village.esp"]);
        let [world] = map.worlds.as_slice() else {
            panic!("expected one world, got {:?}", map.worlds);
        };
        let found: Vec<_> = world
            .markers
            .iter()
            .map(|marker| {
                (
                    marker.name.as_str(),
                    marker.kind,
                    marker.origin,
                    marker.owner,
                )
            })
            .collect();
        assert_eq!(
            found,
            [
                ("Millbrook", PlaceKind::Town, PlaceOrigin::Game, None),
                (
                    "Echo Hollow",
                    PlaceKind::Cave,
                    PlaceOrigin::Changed,
                    Some(0)
                ),
                (
                    "New Village",
                    PlaceKind::Settlement,
                    PlaceOrigin::Added,
                    Some(1)
                ),
            ]
        );
        let cave = &world.markers[1];
        assert_eq!(cave.moved_from, Some([0, 0, 0]));
        assert_eq!(cave.console, "cow Tamriel 2 0");
    }

    fn worldspace(
        local_id: u32,
        plugin: &str,
        editor_id: &str,
        origin: WorldOrigin,
    ) -> WorldspaceFacts {
        WorldspaceFacts {
            id: id(plugin, local_id),
            editor_id: Some(editor_id.to_owned()),
            title: None,
            parent: None,
            uses_parent_land: false,
            small: false,
            no_lod_water: false,
            bounds: None,
            water: None,
            origin,
        }
    }

    fn land(plugin: &str, local_id: u32, side: i32) -> TerrainWorld {
        use mallet_core::facts::terrain::TerrainCell;
        TerrainWorld {
            worldspace: id(plugin, local_id),
            cells: (0..side)
                .flat_map(|x| (0..side).map(move |y| (x, y)))
                .map(|(x, y)| TerrainCell {
                    x,
                    y,
                    heights: vec![0; 64],
                    water: None,
                })
                .collect(),
        }
    }

    /// The game's main world with a city in it, a world of a DLC, one of the
    /// Creation Club and two of a mod.
    fn many_worlds() -> Facts {
        use mallet_core::facts::terrain::TerrainFacts;
        let tamriel = WorldspaceFacts {
            title: Some("Skyrim".to_owned()),
            bounds: Some([-2, -2, 12, 12]),
            ..worldspace(0x3C, "Skyrim.esm", "Tamriel", WorldOrigin::Game)
        };
        let whiterun = WorldspaceFacts {
            parent: Some(id("Skyrim.esm", 0x3C)),
            uses_parent_land: true,
            small: true,
            ..worldspace(0x1A26F, "Skyrim.esm", "WhiterunWorld", WorldOrigin::Game)
        };
        Facts {
            plugins: vec![PluginFacts {
                name: "Falskaar.esm".to_owned(),
                path: GamePath::new("data").join("Falskaar.esm"),
                masters: Vec::new(),
                npcs: Vec::new(),
            }],
            terrain: TerrainFacts {
                worlds: vec![
                    land("Skyrim.esm", 0x3C, 10),
                    land("Skyrim.esm", 0x1A26F, 2),
                    land("Dawnguard.esm", 0x800, 9),
                    land("ccBGSSSE031-Blackbone.esl", 0x801, 9),
                    land("Falskaar.esm", 0x802, 9),
                    land("Falskaar.esm", 0x803, 3),
                ],
                worldspaces: vec![
                    tamriel,
                    whiterun,
                    worldspace(0x800, "Dawnguard.esm", "DLC01SoulCairn", WorldOrigin::Game),
                    worldspace(
                        0x801,
                        "ccBGSSSE031-Blackbone.esl",
                        "ccBGSSSE031BlackboneIsle",
                        WorldOrigin::CreationClub,
                    ),
                    WorldspaceFacts {
                        title: Some("Falskaar".to_owned()),
                        ..worldspace(0x802, "Falskaar.esm", "Falskaar", WorldOrigin::Mod)
                    },
                    worldspace(0x803, "Falskaar.esm", "FalskaarCaveWorld", WorldOrigin::Mod),
                ],
            },
            ..Facts::default()
        }
    }

    #[test]
    fn every_world_with_land_is_listed_named_grouped_and_ordered() {
        let facts = many_worlds();
        let map = world_map(&Context::new(&facts, &NoModSource));
        let listed: Vec<(&str, &str, WorldOrigin, bool, bool)> = map
            .worlds
            .iter()
            .map(|world| {
                (
                    world.name.as_str(),
                    world.title.as_str(),
                    world.origin,
                    world.main,
                    world.small,
                )
            })
            .collect();
        assert_eq!(
            listed,
            [
                ("Tamriel", "Skyrim", WorldOrigin::Game, true, false),
                (
                    "DLC01SoulCairn",
                    "Soul Cairn",
                    WorldOrigin::Game,
                    false,
                    false
                ),
                (
                    "ccBGSSSE031BlackboneIsle",
                    "Blackbone Isle",
                    WorldOrigin::CreationClub,
                    false,
                    false
                ),
                ("Falskaar", "Falskaar", WorldOrigin::Mod, false, false),
                (
                    "FalskaarCaveWorld",
                    "Falskaar Cave",
                    WorldOrigin::Mod,
                    false,
                    true
                ),
            ]
        );
        let skyrim = &map.worlds[0];
        // The city is part of Skyrim, not a world of its own.
        assert_eq!(skyrim.children, ["WhiterunWorld"]);
        assert_eq!(skyrim.land_cells, 100);
        assert_eq!(
            skyrim.bounds,
            Some(MapBounds {
                west: -2,
                south: -2,
                east: 12,
                north: 12
            })
        );
        // The land is shown over the bounds, not only where mods place.
        let terrain = skyrim.terrain.as_ref().unwrap();
        assert_eq!((terrain.west, terrain.columns, terrain.rows), (0, 10, 10));
        assert_eq!(map.worlds[3].origin_name.as_deref(), Some("Falskaar.esm"));
        assert_eq!(map.worlds[0].origin_name, None);
    }

    #[test]
    fn objects_in_a_city_are_counted_on_its_parents_map() {
        use mallet_core::facts::terrain::TerrainFacts;
        let mut object = placed("A.esp", 1, [100, 100, 0]);
        object.place = Place::Exterior {
            worldspace: id("Skyrim.esm", 0x1A26F),
        };
        let facts = Facts {
            plugins: vec![PluginFacts {
                name: "A.esp".to_owned(),
                path: GamePath::new("data").join("A.esp"),
                masters: Vec::new(),
                npcs: Vec::new(),
            }],
            objects: ObjectFacts {
                placed: vec![object],
                ..ObjectFacts::default()
            },
            terrain: TerrainFacts {
                worlds: vec![land("Skyrim.esm", 0x3C, 2)],
                worldspaces: vec![
                    worldspace(0x3C, "Skyrim.esm", "Tamriel", WorldOrigin::Game),
                    WorldspaceFacts {
                        parent: Some(id("Skyrim.esm", 0x3C)),
                        uses_parent_land: true,
                        ..worldspace(0x1A26F, "Skyrim.esm", "WhiterunWorld", WorldOrigin::Game)
                    },
                ],
            },
            ..Facts::default()
        };
        let map = world_map(&Context::new(&facts, &NoModSource));
        let [world] = map.worlds.as_slice() else {
            panic!("expected one world, got {:?}", map.worlds);
        };
        assert_eq!(world.name, "Tamriel");
        assert_eq!(world.cells.len(), 1);
    }

    #[test]
    fn internal_names_are_made_readable() {
        assert_eq!(readable("ccBGSSSE031BlackboneIsle"), "Blackbone Isle");
        assert_eq!(readable("ccKRTSSE001QNWorld"), "QN");
        assert_eq!(readable("DLC2SolstheimWorld"), "Solstheim");
        assert_eq!(readable("DLC01FalmerValley"), "Falmer Valley");
        assert_eq!(readable("WindhelmPitWorldspace"), "Windhelm Pit");
        assert_eq!(readable("tos_Arena03Orcs"), "tos Arena 03 Orcs");
        assert_eq!(readable("Tamriel"), "Tamriel");
        assert_eq!(readable("World"), "World");
    }

    #[test]
    fn a_large_world_keeps_fewer_samples_per_cell() {
        // 160 by 160 cells: 1.6 million samples at eight a side, 409 600 at four.
        let big = land("Skyrim.esm", 0x3C, 160);
        let terrain = terrain(&big, None).unwrap();
        assert_eq!((terrain.columns, terrain.samples), (160, 4));
        let small = land("Skyrim.esm", 0x3C, 10);
        assert_eq!(terrain_columns(&small, None), 10);
        assert_eq!(super::terrain(&small, None).unwrap().samples, 8);
    }

    #[test]
    fn the_main_world_is_the_first_masters_not_the_largest() {
        use mallet_core::facts::terrain::TerrainFacts;
        let plugin = |name: &str| PluginFacts {
            name: name.to_owned(),
            path: GamePath::new("data").join(name),
            masters: Vec::new(),
            npcs: Vec::new(),
        };
        let facts = Facts {
            plugins: vec![plugin("Skyrim.esm"), plugin("Dragonborn.esm")],
            terrain: TerrainFacts {
                // Solstheim's grid of distant land is the larger.
                worlds: vec![
                    land("Skyrim.esm", 0x3C, 8),
                    land("Dragonborn.esm", 0x800, 12),
                ],
                worldspaces: vec![
                    worldspace(0x3C, "Skyrim.esm", "Tamriel", WorldOrigin::Game),
                    worldspace(
                        0x800,
                        "Dragonborn.esm",
                        "DLC2SolstheimWorld",
                        WorldOrigin::Game,
                    ),
                ],
            },
            ..Facts::default()
        };
        let map = world_map(&Context::new(&facts, &NoModSource));
        let first = &map.worlds[0];
        assert_eq!((first.name.as_str(), first.main), ("Tamriel", true));
        assert!(!map.worlds[1].main);
    }
}
