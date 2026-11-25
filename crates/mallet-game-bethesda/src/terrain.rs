//! The shape of the land, read from the plugins.
//!
//! The ground of an outside cell is a `LAND` record in the cell's children.
//! Its `VHGT` field holds the heights of a grid of 33 by 33 points: a
//! starting height, then for each point the difference to its neighbour,
//! the first point of a row relative to the first point of the row before.
//! Heights are in steps of 8 game units.
//!
//! Which cell a `LAND` record belongs to follows from the cell's record
//! before it, whose `XCLC` field holds the cell's grid position. Whether a
//! cell has water, and at which height, is in the cell's `DATA` flags and
//! `XCLW` field, else in the worldspace's `DNAM` field.

use std::collections::{BTreeMap, HashMap};

use mallet_core::WorldOrigin;
use mallet_core::facts::RecordId;
use mallet_core::facts::terrain::{
    SAMPLES_PER_CELL, TerrainCell, TerrainFacts, TerrainWorld, WorldspaceFacts,
};
use mallet_formats_bethesda::plugin::{Plugin, Record, group_type};

use crate::cells::cell_position;
use crate::game::{Family, Game};
use crate::records::{editor_id, full_name, record_id};

/// Points per side of the height grid of a cell.
pub(crate) const GRID: usize = 33;

/// The grid points that are kept: the middle of each eighth of a side.
const SAMPLED: [usize; SAMPLES_PER_CELL] = [2, 6, 10, 14, 18, 22, 26, 30];

/// Cell flag: the cell has water.
const HAS_WATER: u8 = 0x02;

/// Water heights beyond this mean "the worldspace's default".
const MAX_WATER: f32 = 1.0e7;

/// What is known about an outside cell, from its record.
#[derive(Debug, Clone, PartialEq)]
struct CellInfo {
    world: RecordId,
    x: i32,
    y: i32,
    has_water: bool,
    /// The cell's own water height, if it sets one.
    water: Option<f32>,
}

/// What a worldspace's record says, as far as the map needs it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct WorldRecord {
    editor_id: Option<String>,
    /// The name the game shows, already looked up in the string tables.
    title: Option<String>,
    parent: Option<RecordId>,
    /// What the worldspace takes from its parent (`PNAM`).
    parent_flags: u16,
    /// The worldspace's flags (`DATA`).
    flags: u8,
    /// The cells it spans: west, south, east, north.
    bounds: Option<[i32; 4]>,
}

/// Parent flag: the worldspace uses its parent's land.
const USES_PARENT_LAND: u16 = 0x01;

/// World flag: a small world.
const SMALL_WORLD: u8 = 0x01;

/// Coordinates beyond this, in game units, are no real bounds.
const MAX_COORDINATE: f32 = 4096.0 * 1000.0;

/// What one plugin says about the land: its outside cells, its
/// worldspaces, and the heights of its land. It can be kept as text, so
/// that a large plugin that has not changed need not be read again.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Contribution {
    cells: Vec<(RecordId, CellInfo)>,
    world_water: Vec<(RecordId, f32)>,
    worldspaces: Vec<(RecordId, WorldRecord)>,
    land: Vec<((RecordId, i32, i32), Vec<i32>)>,
}

impl Contribution {
    /// The land of `plugin`, named `plugin_name`. `names` are the strings
    /// of a localized plugin, by their number.
    pub fn of(plugin_name: &str, plugin: &Plugin, names: &HashMap<u32, String>) -> Self {
        let masters = &plugin.header.masters;
        let id = |form_id: u32| record_id(form_id, masters, plugin_name);
        let label = |record: &Record, wanted: u32| {
            record
                .groups
                .iter()
                .find(|group| group.group_type == wanted)
                .map(|group| id(group.label))
        };
        let mut contribution = Self::default();
        // The cells of this plugin, to place its land.
        let mut cells: HashMap<RecordId, (i32, i32)> = HashMap::new();
        for record in &plugin.records {
            match &record.signature {
                b"WRLD" => {
                    if let Some(water) = record
                        .first(b"DNAM")
                        .and_then(|data| float_at(data, 4))
                        .filter(|water| water.abs() < MAX_WATER)
                    {
                        contribution.world_water.push((id(record.form_id), water));
                    }
                    let world = world_record(record, plugin.header.is_localized, names, &id);
                    contribution.worldspaces.push((id(record.form_id), world));
                }
                b"CELL" => {
                    // A cell outside lies in the children of its worldspace.
                    if let Some(world) = label(record, group_type::WORLD_CHILDREN)
                        && let Some(info) = cell_info(record, world)
                    {
                        cells.insert(id(record.form_id), (info.x, info.y));
                        contribution.cells.push((id(record.form_id), info));
                    }
                }
                b"LAND" => {
                    let (Some(world), Some(cell)) = (
                        label(record, group_type::WORLD_CHILDREN),
                        label(record, group_type::CELL_CHILDREN),
                    ) else {
                        continue;
                    };
                    let (Some((x, y)), Some(heights)) = (
                        cells.get(&cell).copied(),
                        record.first(b"VHGT").and_then(heights),
                    ) else {
                        continue;
                    };
                    contribution.land.push(((world, x, y), heights));
                }
                _ => {}
            }
        }
        contribution
    }

    /// The contribution as text, one line per entry, fields separated by
    /// tabs.
    pub fn to_text(&self) -> String {
        let mut lines: Vec<String> = Vec::new();
        for (world, water) in &self.world_water {
            lines.push(format!("W\t{}\t{}\t{water}", world.plugin, world.local_id));
        }
        for (world, record) in &self.worldspaces {
            let text = |value: &Option<String>| {
                value.as_deref().map_or_else(
                    || "-".to_owned(),
                    |value| value.replace(['\t', '\n', '\r'], " "),
                )
            };
            let (parent_plugin, parent_id) = record.parent.as_ref().map_or_else(
                || ("-".to_owned(), "-".to_owned()),
                |parent| (parent.plugin.clone(), parent.local_id.to_string()),
            );
            let bounds = record.bounds.map_or_else(
                || "-".to_owned(),
                |bounds| bounds.map(|edge| edge.to_string()).join(","),
            );
            lines.push(format!(
                "R\t{}\t{}\t{}\t{}\t{parent_plugin}\t{parent_id}\t{}\t{}\t{bounds}",
                world.plugin,
                world.local_id,
                text(&record.editor_id),
                text(&record.title),
                record.parent_flags,
                record.flags,
            ));
        }
        for (cell, info) in &self.cells {
            let water = info
                .water
                .map_or_else(|| "-".to_owned(), |water| water.to_string());
            lines.push(format!(
                "C\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{water}",
                cell.plugin,
                cell.local_id,
                info.world.plugin,
                info.world.local_id,
                info.x,
                info.y,
                u8::from(info.has_water),
            ));
        }
        for ((world, x, y), heights) in &self.land {
            let heights: Vec<String> = heights.iter().map(ToString::to_string).collect();
            lines.push(format!(
                "L\t{}\t{}\t{x}\t{y}\t{}",
                world.plugin,
                world.local_id,
                heights.join(",")
            ));
        }
        lines.join("\n")
    }

    /// Reads a contribution written by [`Contribution::to_text`]. `None`
    /// if the text is not one.
    pub fn from_text(text: &str) -> Option<Self> {
        let mut contribution = Self::default();
        let id = |plugin: &str, local_id: &str| {
            Some(RecordId {
                plugin: plugin.to_owned(),
                local_id: local_id.parse().ok()?,
            })
        };
        for line in text.lines().filter(|line| !line.is_empty()) {
            let fields: Vec<&str> = line.split('\t').collect();
            match fields.as_slice() {
                ["W", plugin, local_id, water] => contribution
                    .world_water
                    .push((id(plugin, local_id)?, water.parse().ok()?)),
                [
                    "C",
                    plugin,
                    local_id,
                    world,
                    world_id,
                    x,
                    y,
                    has_water,
                    water,
                ] => {
                    contribution.cells.push((
                        id(plugin, local_id)?,
                        CellInfo {
                            world: id(world, world_id)?,
                            x: x.parse().ok()?,
                            y: y.parse().ok()?,
                            has_water: *has_water == "1",
                            water: if *water == "-" {
                                None
                            } else {
                                Some(water.parse().ok()?)
                            },
                        },
                    ));
                }
                [
                    "R",
                    plugin,
                    local_id,
                    editor_id,
                    title,
                    parent_plugin,
                    parent_id,
                    parent_flags,
                    flags,
                    bounds,
                ] => {
                    let text = |value: &str| (value != "-").then(|| value.to_owned());
                    let bounds = if *bounds == "-" {
                        None
                    } else {
                        let edges: Option<Vec<i32>> =
                            bounds.split(',').map(|edge| edge.parse().ok()).collect();
                        Some(<[i32; 4]>::try_from(edges?).ok()?)
                    };
                    contribution.worldspaces.push((
                        id(plugin, local_id)?,
                        WorldRecord {
                            editor_id: text(editor_id),
                            title: text(title),
                            parent: if *parent_plugin == "-" {
                                None
                            } else {
                                Some(id(parent_plugin, parent_id)?)
                            },
                            parent_flags: parent_flags.parse().ok()?,
                            flags: flags.parse().ok()?,
                            bounds,
                        },
                    ));
                }
                ["L", plugin, local_id, x, y, heights] => {
                    let heights: Option<Vec<i32>> =
                        heights.split(',').map(|value| value.parse().ok()).collect();
                    // Kept with another number of samples: read anew.
                    let heights = heights
                        .filter(|heights| heights.len() == SAMPLES_PER_CELL * SAMPLES_PER_CELL);
                    contribution.land.push((
                        (id(plugin, local_id)?, x.parse().ok()?, y.parse().ok()?),
                        heights?,
                    ));
                }
                _ => return None,
            }
        }
        Some(contribution)
    }
}

/// Gathers the land of all plugins, which have to be added in load order
/// so that later versions replace earlier ones.
#[derive(Debug, Default)]
pub struct Collector {
    cells: HashMap<RecordId, CellInfo>,
    /// The default water height of each worldspace.
    world_water: HashMap<RecordId, f32>,
    worldspaces: HashMap<RecordId, WorldRecord>,
    land: HashMap<(RecordId, i32, i32), Vec<i32>>,
}

impl Collector {
    /// Takes in the cells, worldspaces and land of `plugin`, named
    /// `plugin_name`.
    pub fn add(&mut self, plugin_name: &str, plugin: &Plugin) {
        self.merge(Contribution::of(plugin_name, plugin, &HashMap::new()));
    }

    /// Takes in what one plugin says about the land.
    pub fn merge(&mut self, contribution: Contribution) {
        self.cells.extend(contribution.cells);
        self.world_water.extend(contribution.world_water);
        self.worldspaces.extend(contribution.worldspaces);
        self.land.extend(contribution.land);
    }

    /// The land and the worldspaces of `game`. `origin` tells whether a
    /// plugin comes with the game, the Creation Club or a mod.
    pub fn finish(self, game: Game, origin: &dyn Fn(&str) -> WorldOrigin) -> TerrainFacts {
        // Oblivion and the older Fallouts keep this flag in another bit.
        let no_lod_water = match game.family() {
            Family::Oblivion | Family::Fallout3 => 0x10,
            Family::Skyrim | Family::Fallout4 => 0x08,
        };
        let mut worldspaces: Vec<WorldspaceFacts> = self
            .worldspaces
            .iter()
            .map(|(id, record)| WorldspaceFacts {
                id: id.clone(),
                editor_id: record.editor_id.clone(),
                title: record.title.clone(),
                parent: record.parent.clone(),
                uses_parent_land: record.parent.is_some()
                    && record.parent_flags & USES_PARENT_LAND != 0,
                small: record.flags & SMALL_WORLD != 0,
                no_lod_water: record.flags & no_lod_water != 0,
                bounds: record.bounds,
                water: self.world_water.get(id).copied().and_then(whole),
                origin: origin(&id.plugin),
            })
            .collect();
        worldspaces.sort_by(|a, b| a.id.cmp(&b.id));
        let mut worlds: BTreeMap<RecordId, Vec<TerrainCell>> = BTreeMap::new();
        let cells_by_place: HashMap<(&RecordId, i32, i32), &CellInfo> = self
            .cells
            .values()
            .map(|info| ((&info.world, info.x, info.y), info))
            .collect();
        for ((world, x, y), heights) in &self.land {
            let (x, y) = (*x, *y);
            let default_water = self.world_water.get(world).copied();
            let cell_water = cells_by_place.get(&(world, x, y)).and_then(|info| {
                if info.has_water {
                    info.water.or(default_water)
                } else {
                    None
                }
            });
            worlds.entry(world.clone()).or_default().push(TerrainCell {
                x,
                y,
                heights: heights.clone(),
                water: cell_water.and_then(whole),
            });
        }
        TerrainFacts {
            worlds: worlds
                .into_iter()
                .map(|(worldspace, mut cells)| {
                    cells.sort_by_key(|cell| (cell.y, cell.x));
                    TerrainWorld { worldspace, cells }
                })
                .collect(),
            worldspaces,
        }
    }
}

/// What the record of a worldspace says. `id` turns a form ID of its
/// plugin into a record.
fn world_record(
    record: &Record,
    localized: bool,
    names: &HashMap<u32, String>,
    id: &dyn Fn(u32) -> RecordId,
) -> WorldRecord {
    let editor_id = editor_id(record);
    let title = full_name(record, localized, names)
        .map(|title| title.trim().to_owned())
        .filter(|title| !title.is_empty());
    let parent = record
        .first(b"WNAM")
        .and_then(|data| Some(u32::from_le_bytes(data.get(..4)?.try_into().ok()?)))
        .filter(|form_id| *form_id != 0)
        .map(id);
    let parent_flags = record
        .first(b"PNAM")
        .and_then(|data| Some(u16::from_le_bytes(data.get(..2)?.try_into().ok()?)))
        .unwrap_or_default();
    let flags = record
        .first(b"DATA")
        .and_then(|data| data.first().copied())
        .unwrap_or_default();
    WorldRecord {
        editor_id,
        title,
        parent,
        parent_flags,
        flags,
        // The corners hold the land; the map's cells (`MNAM`) are the part
        // its world map shows, which can be much smaller.
        bounds: corner_bounds(record).or_else(|| map_bounds(record)),
    }
}

/// The cells of a worldspace's map (`MNAM`): west, south, east, north.
fn map_bounds(record: &Record) -> Option<[i32; 4]> {
    let data = record.first(b"MNAM")?;
    let edge = |at: usize| -> Option<i32> {
        Some(i32::from(i16::from_le_bytes(
            data.get(at..at + 2)?.try_into().ok()?,
        )))
    };
    let (west, north, east, south) = (edge(8)?, edge(10)?, edge(12)?, edge(14)?);
    (west < east && south < north).then_some([west, south, east, north])
}

/// The cells within a worldspace's corners (`NAM0`, `NAM9`): west, south,
/// east, north.
fn corner_bounds(record: &Record) -> Option<[i32; 4]> {
    let corner = |signature: &[u8; 4]| -> Option<(f32, f32)> {
        let data = record.first(signature)?;
        let (x, y) = (float_at(data, 0)?, float_at(data, 4)?);
        (x.abs() < MAX_COORDINATE && y.abs() < MAX_COORDINATE).then_some((x, y))
    };
    let (low, high) = (corner(b"NAM0")?, corner(b"NAM9")?);
    let cell = |coordinate: f32| whole((coordinate / 4096.0).floor());
    let bounds = [cell(low.0)?, cell(low.1)?, cell(high.0)?, cell(high.1)?];
    let [west, south, east, north] = bounds;
    (west < east && south < north).then_some(bounds)
}

/// Whether a cell has water, and the height it sets for it, if any
/// (ADR 0058).
pub(crate) fn cell_water(record: &Record) -> (bool, Option<f32>) {
    let flags = record
        .first(b"DATA")
        .and_then(|data| data.first().copied())
        .unwrap_or_default();
    let water = record
        .first(b"XCLW")
        .and_then(|data| float_at(data, 0))
        .filter(|water| water.abs() < MAX_WATER);
    (flags & HAS_WATER != 0, water)
}

/// The height of a worldspace's water where its cells set none.
pub(crate) fn world_water(record: &Record) -> Option<f32> {
    record
        .first(b"DNAM")
        .and_then(|data| float_at(data, 4))
        .filter(|water| water.abs() < MAX_WATER)
}

/// The position and water of an outside cell; `None` for a cell inside.
fn cell_info(record: &Record, world: RecordId) -> Option<CellInfo> {
    let (x, y) = cell_position(record)?;
    let flags = record
        .first(b"DATA")
        .and_then(|data| data.first().copied())
        .unwrap_or_default();
    let water = record
        .first(b"XCLW")
        .and_then(|data| float_at(data, 0))
        .filter(|water| water.abs() < MAX_WATER);
    Some(CellInfo {
        world,
        x,
        y,
        has_water: flags & HAS_WATER != 0,
        water,
    })
}

/// The sampled heights of a `VHGT` field, in game units.
fn heights(data: &[u8]) -> Option<Vec<i32>> {
    let grid = height_grid(data)?;
    let mut samples = Vec::with_capacity(SAMPLES_PER_CELL * SAMPLES_PER_CELL);
    for row in SAMPLED {
        for column in SAMPLED {
            samples.push(whole(*grid.get(row * GRID + column)?)?);
        }
    }
    Some(samples)
}

/// Every height of a `VHGT` field, in game units: 33 rows of 33 points,
/// the southern row first, each from west to east.
pub(crate) fn height_grid(data: &[u8]) -> Option<Vec<f32>> {
    let offset = float_at(data, 0)?;
    let deltas = data.get(4..4 + GRID * GRID)?;
    if !offset.is_finite() {
        return None;
    }
    // Rebuild the whole grid, in steps of 8 game units.
    let mut grid = vec![0f32; GRID * GRID];
    let mut row_start = offset;
    for row in 0..GRID {
        let mut height = row_start;
        for column in 0..GRID {
            let index = row * GRID + column;
            let delta = f32::from(i8::from_le_bytes([*deltas.get(index)?]));
            if column == 0 {
                row_start += delta;
                height = row_start;
            } else {
                height += delta;
            }
            *grid.get_mut(index)? = height * 8.0;
        }
    }
    Some(grid)
}

fn float_at(data: &[u8], offset: usize) -> Option<f32> {
    Some(f32::from_le_bytes(
        data.get(offset..offset + 4)?.try_into().ok()?,
    ))
}

/// A height rounded to whole game units, or `None` if it is not one the
/// world could have.
#[allow(clippy::cast_possible_truncation)]
fn whole(value: f32) -> Option<i32> {
    // The range check makes the conversion exact up to rounding.
    (value.is_finite() && value.abs() <= MAX_WATER).then(|| value.round() as i32)
}

#[cfg(test)]
mod tests {
    use mallet_formats_bethesda::plugin::{GroupLabel, Subrecord};
    use proptest::prelude::*;

    use super::*;

    #[allow(clippy::trivially_copy_pass_by_ref)]
    fn record(signature: &[u8; 4], form_id: u32, fields: Vec<(&[u8; 4], Vec<u8>)>) -> Record {
        Record {
            signature: *signature,
            form_id,
            flags: 0,
            subrecords: fields
                .into_iter()
                .map(|(signature, data)| Subrecord {
                    signature: *signature,
                    data,
                })
                .collect(),
            groups: Vec::new(),
        }
    }

    fn world() -> RecordId {
        RecordId {
            plugin: "skyrim.esm".to_owned(),
            local_id: 0x3C,
        }
    }

    /// A height field that rises by one step per point towards the east
    /// and by two per row towards the north.
    fn slope(offset: f32) -> Vec<u8> {
        let mut data = offset.to_le_bytes().to_vec();
        for _row in 0..GRID {
            for column in 0..GRID {
                // The first point of a row is relative to the row before.
                data.push(if column == 0 { 2 } else { 1 });
            }
        }
        data.extend([0; 3]);
        data
    }

    #[test]
    fn heights_are_rebuilt_from_their_differences() {
        let samples = heights(&slope(10.0)).unwrap();
        assert_eq!(samples.len(), 64);
        // Row 2 starts at 10 + 2 * 3; column 2 adds 2. In steps of 8.
        assert_eq!(samples[0], (10 + 6 + 2) * 8);
        // Column 30 of the same row.
        assert_eq!(samples[7], (10 + 6 + 30) * 8);
        // Row 30, column 2.
        assert_eq!(samples[56], (10 + 2 * 31 + 2) * 8);
    }

    #[test]
    fn land_is_placed_by_its_cell_and_given_the_water_of_its_world() {
        let mut position = 3i32.to_le_bytes().to_vec();
        position.extend((-2i32).to_le_bytes());
        let mut world_heights = 0f32.to_le_bytes().to_vec();
        world_heights.extend((-500f32).to_le_bytes());
        let world_group = GroupLabel {
            group_type: group_type::WORLD_CHILDREN,
            label: 0x3C,
        };
        let mut cell = record(
            b"CELL",
            0x10,
            vec![(b"DATA", vec![HAS_WATER]), (b"XCLC", position)],
        );
        cell.groups = vec![world_group];
        let mut land = record(b"LAND", 0x20, vec![(b"VHGT", slope(0.0))]);
        land.groups = vec![
            GroupLabel {
                group_type: group_type::WORLD_CHILDREN,
                label: 0x3C,
            },
            GroupLabel {
                group_type: group_type::CELL_CHILDREN,
                label: 0x10,
            },
        ];
        let plugin = Plugin {
            records: vec![
                record(b"WRLD", 0x3C, vec![(b"DNAM", world_heights)]),
                cell,
                land,
            ],
            ..Plugin::default()
        };
        let mut collector = Collector::default();
        collector.add("Skyrim.esm", &plugin);
        let facts = collector.finish(Game::SkyrimSe, &|_| WorldOrigin::Game);

        let [world] = facts.worlds.as_slice() else {
            panic!("expected one world, got {facts:?}");
        };
        let [cell] = world.cells.as_slice() else {
            panic!("expected one cell, got {world:?}");
        };
        assert_eq!((cell.x, cell.y), (3, -2));
        assert_eq!(cell.water, Some(-500));
        assert_eq!(cell.heights.len(), 64);
    }

    #[test]
    fn a_contribution_survives_being_kept_as_text() {
        let contribution = Contribution {
            cells: vec![(
                RecordId {
                    plugin: "a mod.esp".to_owned(),
                    local_id: 0x10,
                },
                CellInfo {
                    world: world(),
                    x: -3,
                    y: 7,
                    has_water: true,
                    water: Some(-120.5),
                },
            )],
            world_water: vec![(world(), -500.0)],
            worldspaces: vec![
                (
                    world(),
                    WorldRecord {
                        editor_id: Some("Tamriel".to_owned()),
                        title: Some("Skyrim\twith a tab".to_owned()),
                        parent: None,
                        parent_flags: 0,
                        flags: 0x40,
                        bounds: Some([-57, -43, 61, 50]),
                    },
                ),
                (
                    RecordId {
                        plugin: "skyrim.esm".to_owned(),
                        local_id: 0x1A26F,
                    },
                    WorldRecord {
                        editor_id: Some("WhiterunWorld".to_owned()),
                        title: None,
                        parent: Some(world()),
                        parent_flags: 0x5F,
                        flags: 0x01,
                        bounds: None,
                    },
                ),
            ],
            land: vec![((world(), -3, 7), (0..64).collect())],
        };
        let text = contribution.to_text();
        // A tab in a name would split its line; it is kept as a space.
        let mut expected = contribution.clone();
        if let Some((_, record)) = expected.worldspaces.first_mut() {
            record.title = Some("Skyrim with a tab".to_owned());
        }
        assert_eq!(Contribution::from_text(&text), Some(expected));
        let contribution = Contribution {
            worldspaces: Vec::new(),
            ..contribution
        };
        let text = contribution.to_text();
        assert_eq!(Contribution::from_text(&text), Some(contribution));
        assert_eq!(Contribution::from_text("nonsense"), None);
        // Land kept with another number of samples is read anew.
        let old = "L	skyrim.esm	60	-3	7	1,-2,3";
        assert_eq!(Contribution::from_text(old), None);
    }

    fn map_data(west: i16, north: i16, east: i16, south: i16) -> Vec<u8> {
        let mut data = 400i32.to_le_bytes().to_vec();
        data.extend(300i32.to_le_bytes());
        for edge in [west, north, east, south] {
            data.extend(edge.to_le_bytes());
        }
        data.extend([0; 12]);
        data
    }

    fn corner(x: f32, y: f32) -> Vec<u8> {
        let mut data = x.to_le_bytes().to_vec();
        data.extend(y.to_le_bytes());
        data
    }

    #[test]
    fn a_worldspace_tells_its_name_parent_flags_and_bounds() {
        let id = |form_id: u32| RecordId {
            plugin: "skyrim.esm".to_owned(),
            local_id: form_id,
        };
        let whiterun = record(
            b"WRLD",
            0x1A26F,
            vec![
                (b"EDID", b"WhiterunWorld\0".to_vec()),
                (b"FULL", b"Whiterun\0".to_vec()),
                (b"WNAM", 0x3Cu32.to_le_bytes().to_vec()),
                (b"PNAM", 0x5Fu16.to_le_bytes().to_vec()),
                (b"DATA", vec![0x01]),
                (b"MNAM", map_data(-2, 3, 4, -5)),
            ],
        );
        let read = world_record(&whiterun, false, &HashMap::new(), &id);
        assert_eq!(read.editor_id.as_deref(), Some("WhiterunWorld"));
        assert_eq!(read.title.as_deref(), Some("Whiterun"));
        assert_eq!(read.parent, Some(id(0x3C)));
        assert_eq!(read.bounds, Some([-2, -5, 4, 3]));

        // A localized plugin names it by a string's number; empty map
        // bounds give way to the corners.
        let mut names = HashMap::new();
        names.insert(7u32, "Blackreach".to_owned());
        let blackreach = record(
            b"WRLD",
            0x1,
            vec![
                (b"FULL", 7u32.to_le_bytes().to_vec()),
                (b"MNAM", map_data(0, 0, 0, 0)),
                (b"NAM0", corner(-8192.0, -4096.5)),
                (b"NAM9", corner(12288.0, 4095.0)),
            ],
        );
        let read = world_record(&blackreach, true, &names, &id);
        assert_eq!(read.title.as_deref(), Some("Blackreach"));
        assert_eq!(read.bounds, Some([-2, -2, 3, 0]));
        assert_eq!(read.parent, None);

        // Corners the game leaves unset are no bounds.
        let unset = record(
            b"WRLD",
            0x2,
            vec![
                (b"NAM0", corner(f32::MAX, f32::MAX)),
                (b"NAM9", corner(-f32::MAX, -f32::MAX)),
            ],
        );
        assert_eq!(
            world_record(&unset, false, &HashMap::new(), &id).bounds,
            None
        );
    }

    #[test]
    fn worldspaces_are_finished_with_their_flags_and_origin() {
        let mut collector = Collector::default();
        let mut world_heights = 0f32.to_le_bytes().to_vec();
        world_heights.extend((-14000f32).to_le_bytes());
        let plugin = Plugin {
            records: vec![
                record(
                    b"WRLD",
                    0x3C,
                    vec![(b"DATA", vec![0x08]), (b"DNAM", world_heights)],
                ),
                record(
                    b"WRLD",
                    0x1A26F,
                    vec![
                        (b"WNAM", 0x3Cu32.to_le_bytes().to_vec()),
                        (b"PNAM", 0x01u16.to_le_bytes().to_vec()),
                        (b"DATA", vec![0x01]),
                    ],
                ),
            ],
            ..Plugin::default()
        };
        collector.add("Skyrim.esm", &plugin);
        let facts = collector.finish(Game::SkyrimSe, &|plugin| {
            if plugin == "skyrim.esm" {
                WorldOrigin::Game
            } else {
                WorldOrigin::Mod
            }
        });
        let [tamriel, whiterun] = facts.worldspaces.as_slice() else {
            panic!("expected two worldspaces, got {facts:?}");
        };
        assert!(tamriel.no_lod_water && !tamriel.small && !tamriel.uses_parent_land);
        assert_eq!(tamriel.water, Some(-14000));
        assert_eq!(tamriel.origin, WorldOrigin::Game);
        assert!(whiterun.small && whiterun.uses_parent_land && !whiterun.no_lod_water);
        assert_eq!(whiterun.parent, Some(world()));
    }

    #[test]
    fn a_cell_inside_has_no_position() {
        assert!(cell_info(&record(b"CELL", 1, Vec::new()), world()).is_none());
    }

    proptest! {
        #[test]
        fn never_panics_on_arbitrary_fields(
            vhgt in proptest::collection::vec(any::<u8>(), 0..1200),
            xclc in proptest::collection::vec(any::<u8>(), 0..12),
            xclw in proptest::collection::vec(any::<u8>(), 0..6),
            full in proptest::collection::vec(any::<u8>(), 0..8),
            wnam in proptest::collection::vec(any::<u8>(), 0..6),
            pnam in proptest::collection::vec(any::<u8>(), 0..3),
            mnam in proptest::collection::vec(any::<u8>(), 0..20),
            nam0 in proptest::collection::vec(any::<u8>(), 0..9),
            nam9 in proptest::collection::vec(any::<u8>(), 0..9),
            localized in any::<bool>(),
        ) {
            let worldspace = record(b"WRLD", 1, vec![
                (b"FULL", full), (b"WNAM", wnam), (b"PNAM", pnam),
                (b"MNAM", mnam), (b"NAM0", nam0), (b"NAM9", nam9),
            ]);
            let id = |form_id: u32| RecordId { plugin: "a.esp".to_owned(), local_id: form_id };
            let _ = world_record(&worldspace, localized, &HashMap::new(), &id);
            let _ = heights(&vhgt);
            let _ = height_grid(&vhgt);
            let _ = cell_info(&record(b"CELL", 1, vec![(b"XCLC", xclc), (b"XCLW", xclw)]), world());
        }
    }
}
