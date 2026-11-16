//! Where mods place objects, cell by cell.
//!
//! The map shows each worldspace as a grid of cells, how many objects mods
//! place in each and which mods they are, and the spots where objects of
//! two mods collide. It is built from the same facts as the findings and
//! decides nothing.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use mallet_core::facts::RecordId;
use mallet_core::facts::objects::Place;
use mallet_core::facts::terrain::{SAMPLES_PER_CELL, TerrainWorld};
use mallet_core::{MapCell, MapTerrain, MapWorld, WorldMap};

use crate::Context;
use crate::objects::Owners;

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
        cell.1.insert(object.id.plugin.as_str());
    }

    // Plugins of the same mod become one owner, and owners are listed by
    // name.
    let owners = Owners::new(context);
    let display: HashMap<&str, String> = worlds
        .values()
        .flat_map(|cells| cells.values())
        .flat_map(|(_, plugins)| plugins.iter().copied())
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

    // The worldspace where most mods meet comes first, then the busiest.
    let mut map_worlds: Vec<((usize, u64), MapWorld)> = worlds
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
            let terrain = context
                .facts
                .terrain
                .worlds
                .iter()
                .find(|land| &land.worldspace == worldspace)
                .and_then(|land| terrain(land, around(&cells)));
            (
                (shared, total),
                MapWorld {
                    name,
                    cells,
                    terrain,
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

/// The cells around the cells of the map, as west, east, south, north.
fn around(cells: &[MapCell]) -> Option<(i32, i32, i32, i32)> {
    Some((
        cells.iter().map(|cell| cell.x).min()? - MARGIN,
        cells.iter().map(|cell| cell.x).max()? + MARGIN,
        cells.iter().map(|cell| cell.y).min()? - MARGIN,
        cells.iter().map(|cell| cell.y).max()? + MARGIN,
    ))
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

    // Land is what lies above its cell's water.
    let above = |height: i32, water: Option<i32>| water.is_none_or(|water| height > water);
    let land_heights = cells.iter().flat_map(|cell| {
        cell.heights
            .iter()
            .copied()
            .filter(move |height| above(*height, cell.water))
    });
    let lowest = land_heights.clone().min().unwrap_or_default();
    let highest = land_heights.max().unwrap_or_default().max(lowest + 1);

    let mut image = vec![0u8; width * rows * samples];
    for cell in &cells {
        let column = usize::try_from(cell.x - west).ok()?;
        let row = usize::try_from(north - cell.y).ok()?;
        for (index, height) in cell.heights.iter().enumerate() {
            // Within a cell, samples go from south to north; the image
            // goes from north to south.
            let (sample_row, sample_column) = (samples - 1 - index / samples, index % samples);
            let at = (row * samples + sample_row) * width + column * samples + sample_column;
            let value = if above(*height, cell.water) {
                let share = f64::from(height - lowest) / f64::from(highest - lowest);
                level(share)
            } else {
                1
            };
            if let Some(pixel) = image.get_mut(at) {
                *pixel = value;
            }
        }
    }
    Some(MapTerrain {
        west,
        south,
        columns: u32::try_from(columns).ok()?,
        rows: u32::try_from(rows).ok()?,
        samples: u32::try_from(samples).ok()?,
        heights: base64(&image),
        lowest,
        highest,
    })
}

/// A share from 0 to 1 as a land value from 2 to 255.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn level(share: f64) -> u8 {
    // Clamped to the range of the result first.
    (2.0 + share.clamp(0.0, 1.0) * 253.0).round() as u8
}

/// Bytes in Base64, the standard alphabet with padding.
fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let symbol =
        |value: u32| char::from(ALPHABET.get((value & 63) as usize).copied().unwrap_or(b'='));
    let mut text = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let [a, b, c] = [0, 1, 2].map(|index| u32::from(chunk.get(index).copied().unwrap_or(0)));
        let group = (a << 16) | (b << 8) | c;
        text.push(symbol(group >> 18));
        text.push(symbol(group >> 12));
        text.push(if chunk.len() > 1 {
            symbol(group >> 6)
        } else {
            '='
        });
        text.push(if chunk.len() > 2 { symbol(group) } else { '=' });
    }
    text
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
        }
    }

    fn terrain_columns(land: &TerrainWorld, within: Option<(i32, i32, i32, i32)>) -> u32 {
        terrain(land, within).unwrap().columns
    }

    #[test]
    fn base64_is_the_standard_one() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"M"), "TQ==");
        assert_eq!(base64(b"Ma"), "TWE=");
        assert_eq!(base64(b"Man"), "TWFu");
        assert_eq!(base64(&[0xFF, 0xEF]), "/+8=");
    }

    #[test]
    fn land_becomes_a_height_image_with_water_apart() {
        use mallet_core::facts::terrain::TerrainCell;
        let flat = |x: i32, height: i32, water: Option<i32>| TerrainCell {
            x,
            y: 0,
            heights: vec![height; 16],
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
        assert_eq!((terrain.columns, terrain.rows, terrain.samples), (4, 1, 4));
        assert_eq!((terrain.lowest, terrain.highest), (100, 500));
        // 16 samples per row: cell 0 low land, cell 1 high land, cell 2
        // unknown, cell 3 water.
        let expected: Vec<u8> = [2u8, 255, 0, 1]
            .iter()
            .flat_map(|value| [*value; 4])
            .collect();
        let row: Vec<u8> = expected.iter().copied().cycle().take(16).collect();
        let image: Vec<u8> = row.iter().copied().cycle().take(64).collect();
        assert_eq!(terrain.heights, base64(&image));
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
}
