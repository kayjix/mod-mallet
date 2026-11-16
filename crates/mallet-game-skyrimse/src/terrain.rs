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

use mallet_core::facts::RecordId;
use mallet_core::facts::terrain::{SAMPLES_PER_CELL, TerrainCell, TerrainFacts, TerrainWorld};
use mallet_formats_bethesda::plugin::{self, Plugin, Record, group_type};

/// Points per side of the height grid of a cell.
const GRID: usize = 33;

/// The grid points that are kept: the middle of each quarter of a side.
const SAMPLED: [usize; SAMPLES_PER_CELL] = [4, 12, 20, 28];

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

/// What one plugin says about the land: its outside cells, the water of its
/// worldspaces, and the heights of its land. It can be kept as text, so
/// that a large plugin that has not changed need not be read again.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Contribution {
    cells: Vec<(RecordId, CellInfo)>,
    world_water: Vec<(RecordId, f32)>,
    land: Vec<((RecordId, i32, i32), Vec<i32>)>,
}

impl Contribution {
    /// The land of `plugin`, named `plugin_name`.
    pub fn of(plugin_name: &str, plugin: &Plugin) -> Self {
        let masters = &plugin.header.masters;
        let id = |form_id: u32| {
            let (origin, local_id) = plugin::resolve_form_id(form_id, masters, plugin_name);
            RecordId {
                plugin: origin.to_lowercase(),
                local_id,
            }
        };
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
                ["L", plugin, local_id, x, y, heights] => {
                    let heights: Option<Vec<i32>> =
                        heights.split(',').map(|value| value.parse().ok()).collect();
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
    land: HashMap<(RecordId, i32, i32), Vec<i32>>,
}

impl Collector {
    /// Takes in the cells, worldspaces and land of `plugin`, named
    /// `plugin_name`.
    pub fn add(&mut self, plugin_name: &str, plugin: &Plugin) {
        self.merge(Contribution::of(plugin_name, plugin));
    }

    /// Takes in what one plugin says about the land.
    pub fn merge(&mut self, contribution: Contribution) {
        self.cells.extend(contribution.cells);
        self.world_water.extend(contribution.world_water);
        self.land.extend(contribution.land);
    }

    /// The land of every worldspace.
    pub fn finish(self) -> TerrainFacts {
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
        }
    }
}

/// The position and water of an outside cell; `None` for a cell inside.
fn cell_info(record: &Record, world: RecordId) -> Option<CellInfo> {
    let position = record.first(b"XCLC")?;
    let x = i32::from_le_bytes(position.get(0..4)?.try_into().ok()?);
    let y = i32::from_le_bytes(position.get(4..8)?.try_into().ok()?);
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
            *grid.get_mut(index)? = height;
        }
    }
    let mut samples = Vec::with_capacity(SAMPLES_PER_CELL * SAMPLES_PER_CELL);
    for row in SAMPLED {
        for column in SAMPLED {
            samples.push(whole(*grid.get(row * GRID + column)? * 8.0)?);
        }
    }
    Some(samples)
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
        assert_eq!(samples.len(), 16);
        // Row 4 starts at 10 + 2 * 5; column 4 adds 4. In steps of 8.
        assert_eq!(samples[0], (10 + 10 + 4) * 8);
        // Column 28 of the same row.
        assert_eq!(samples[3], (10 + 10 + 28) * 8);
        // Row 28, column 4.
        assert_eq!(samples[12], (10 + 2 * 29 + 4) * 8);
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
        let facts = collector.finish();

        let [world] = facts.worlds.as_slice() else {
            panic!("expected one world, got {facts:?}");
        };
        let [cell] = world.cells.as_slice() else {
            panic!("expected one cell, got {world:?}");
        };
        assert_eq!((cell.x, cell.y), (3, -2));
        assert_eq!(cell.water, Some(-500));
        assert_eq!(cell.heights.len(), 16);
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
            land: vec![((world(), -3, 7), vec![1, -2, 3])],
        };
        let text = contribution.to_text();
        assert_eq!(Contribution::from_text(&text), Some(contribution));
        assert_eq!(Contribution::from_text("nonsense"), None);
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
        ) {
            let _ = heights(&vhgt);
            let _ = cell_info(&record(b"CELL", 1, vec![(b"XCLC", xclc), (b"XCLW", xclw)]), world());
        }
    }
}
