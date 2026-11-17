//! Where mods place objects, cell by cell.
//!
//! The map shows each worldspace as a grid of cells, how many objects mods
//! place in each and which mods they are, and the spots where objects of
//! two mods collide. It is built from the same facts as the findings and
//! decides nothing.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use mallet_core::facts::RecordId;
use mallet_core::facts::markers::MapMarkerFact;
use mallet_core::facts::objects::Place;
use mallet_core::facts::terrain::{SAMPLES_PER_CELL, TerrainWorld};
use mallet_core::{MapCell, MapMarker, MapTerrain, MapWorld, WorldMap};

use crate::Context;
use crate::objects::Owners;
use crate::places::{self, Origin};

/// The objects of one cell: how many, and which plugins place them.
type CellCount<'a> = (u32, BTreeSet<&'a str>);

/// The map of the setup in `context`.
pub fn world_map(context: &Context<'_>) -> WorldMap {
    let objects = &context.facts.objects;
    let names: HashMap<&RecordId, &str> = objects
        .place_names
        .iter()
        .map(|name| (&name.id, name.editor_id.as_str()))
        .collect();

    // Per worldspace and cell: how many objects, and which plugins.
    let mut worlds: BTreeMap<&RecordId, BTreeMap<(i32, i32), CellCount<'_>>> = BTreeMap::new();
    for object in &objects.placed {
        let Place::Exterior { worldspace } = &object.place else {
            continue;
        };
        let [x, y, _] = object.position;
        let cell = worlds
            .entry(worldspace)
            .or_default()
            .entry((x.div_euclid(4096), y.div_euclid(4096)))
            .or_default();
        cell.0 = cell.0.saturating_add(1);
        cell.1.insert(object.owner());
    }

    // A worldspace with places is on the map even if no mod places objects
    // in it.
    let places_by_world = places_by_world(&context.facts.markers.markers);
    for worldspace in places_by_world.keys() {
        worlds.entry(worldspace).or_default();
    }

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

    // The worldspace where most mods meet comes first, then the busiest,
    // then the one with the most places.
    let mut map_worlds: Vec<((usize, u64, usize), MapWorld)> = worlds
        .into_iter()
        .map(|(worldspace, cells)| {
            let total: u64 = cells.values().map(|(count, _)| u64::from(*count)).sum();
            let cells: Vec<MapCell> = cells
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
                .collect();
            let name = names
                .get(worldspace)
                .map_or_else(|| worldspace.to_string(), |name| (*name).to_owned());
            let shared = cells.iter().filter(|cell| cell.owners.len() > 1).count();
            let markers = world_markers(&places_by_world, worldspace, &name, &owner);
            let terrain = context
                .facts
                .terrain
                .worlds
                .iter()
                .find(|land| &land.worldspace == worldspace)
                .and_then(|land| terrain(land, around(&occupied(&cells, &markers))));
            (
                (shared, total, markers.len()),
                MapWorld {
                    name,
                    cells,
                    terrain,
                    markers,
                },
            )
        })
        .collect();
    map_worlds.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.name.cmp(&b.1.name)));

    WorldMap {
        owners: names_of_owners,
        worlds: map_worlds.into_iter().map(|(_, world)| world).collect(),
        spots: context.collisions().spots.clone(),
    }
}

/// How many cells of land are shown around the cells where mods place
/// objects.
const MARGIN: i32 = 8;

/// Map markers grouped by their worldspace, each with whose it is.
type PlacesByWorld<'a> = BTreeMap<&'a RecordId, Vec<(&'a MapMarkerFact, Origin<'a>)>>;

/// The places of the world map by worldspace, and whose they are.
fn places_by_world(markers: &[MapMarkerFact]) -> PlacesByWorld<'_> {
    let mut by_world: PlacesByWorld<'_> = BTreeMap::new();
    for marker in markers {
        by_world
            .entry(&marker.worldspace)
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

/// The places of `worldspace`, named `world`, for the report. `owner` gives
/// the position of a plugin's mod among the owners of the map.
fn world_markers(
    places: &PlacesByWorld<'_>,
    worldspace: &RecordId,
    world: &str,
    owner: &impl Fn(&str) -> u32,
) -> Vec<MapMarker> {
    places
        .get(worldspace)
        .into_iter()
        .flatten()
        .map(|(marker, origin)| map_marker(marker, origin, world, owner))
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

/// The cells around the cells of the map, as west, east, south, north.
fn around(cells: &[(i32, i32)]) -> Option<(i32, i32, i32, i32)> {
    Some((
        cells.iter().map(|(x, _)| *x).min()?.saturating_sub(MARGIN),
        cells.iter().map(|(x, _)| *x).max()?.saturating_add(MARGIN),
        cells.iter().map(|(_, y)| *y).min()?.saturating_sub(MARGIN),
        cells.iter().map(|(_, y)| *y).max()?.saturating_add(MARGIN),
    ))
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
        kind: places::kind(version.kind),
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
    let samples = SAMPLES_PER_CELL;
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
            // Within a cell, samples go from south to north; the image
            // goes from north to south.
            let (sample_row, sample_column) = (samples - 1 - index / samples, index % samples);
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

        let version = |plugin: &str, kind: u8, name: &str, x: i32| MarkerVersion {
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
                        version("Skyrim.esm", 2, "Millbrook", 0),
                        version("Skyrim.esm", 2, "Millbrook", 0),
                    ),
                    marker(
                        2,
                        version("Skyrim.esm", 4, "Echo Hollow", 0),
                        version("Caves.esp", 4, "Echo Hollow", 9000),
                    ),
                    marker(
                        3,
                        version("Village.esp", 3, "New Village", -5000),
                        version("Village.esp", 3, "New Village", -5000),
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
}
