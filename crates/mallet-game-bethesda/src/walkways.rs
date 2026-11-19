//! The navigation meshes and the ground in chosen cells, every plugin's
//! version, for telling whether hidden objects carry paths (ADR 0051).
//!
//! Outside, only the blocks of cells that hold a chosen cell are entered;
//! inside, only the children of the chosen cells. A worldspace that stands
//! on the land of a chosen one, such as a city in its walls, counts as
//! chosen with the same cells. Nothing is written.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use mallet_core::facts::RecordId;
use mallet_core::facts::navmeshes::{Beyond, GroundCell, NavmeshShape, NearHidden};
use mallet_core::facts::objects::Place;
use mallet_formats_bethesda::navmesh;
use mallet_formats_bethesda::plugin::{self, GroupLabel, Plugin, Record, group_type};

use crate::game::Game;
use crate::ingest;
use crate::navmeshes::NAVMESH;
use crate::objects::{FLAG_DELETED, editor_id, record_id};
use crate::terrain;

/// The cells to read.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Around {
    /// Outside: by the internal name of the worldspace (any case), its
    /// cells as column and row.
    pub worlds: BTreeMap<String, BTreeSet<(i32, i32)>>,
    /// Inside: the cells.
    pub interiors: BTreeSet<RecordId>,
}

impl Around {
    /// Whether no cell is chosen.
    pub fn is_empty(&self) -> bool {
        self.worlds.values().all(BTreeSet::is_empty) && self.interiors.is_empty()
    }
}

/// The navigation meshes and the ground of the cells `around` in the
/// setup of `game` in `game_dir`; `local_dir` holds `plugins.txt`, as for
/// [`ingest::read_setup`].
pub fn read(game: Game, game_dir: &Path, local_dir: Option<&Path>, around: &Around) -> NearHidden {
    let data_dir = game_dir.join("Data");
    let plugins: Vec<(String, PathBuf)> = ingest::active_plugin_names(game, game_dir, local_dir)
        .unwrap_or_default()
        .into_iter()
        .map(|name| {
            let path = data_dir.join(&name);
            (name, path)
        })
        .collect();
    read_plugins(&plugins, around)
}

/// The navigation meshes and the ground of the cells `around` in
/// `plugins`, the active plugins in load order by name and file.
pub fn read_plugins(plugins: &[(String, PathBuf)], around: &Around) -> NearHidden {
    let mut found = Found::default();
    if around.worlds.values().any(|cells| !cells.is_empty()) {
        outside(plugins, around, &mut found);
    }
    if !around.interiors.is_empty() {
        inside(plugins, around, &mut found);
    }
    found.finish()
}

/// What the passes found, before the version the game uses is known.
#[derive(Default)]
struct Found {
    /// Every version of every navigation mesh, in load order; `None` for a
    /// version that deletes it or whose geometry cannot be read.
    versions: Vec<(RecordId, Option<NavmeshShape>)>,
    /// The ground by worldspace and cell, the last version.
    ground: HashMap<(RecordId, i32, i32), Option<Vec<i32>>>,
    /// The chosen worldspaces that stand on another one's land, and that
    /// one.
    land_of: HashMap<RecordId, RecordId>,
    /// The cells chosen in each worldspace read.
    cells: HashMap<RecordId, BTreeSet<(i32, i32)>>,
}

impl Found {
    fn finish(self) -> NearHidden {
        let last: HashMap<&RecordId, usize> = self
            .versions
            .iter()
            .enumerate()
            .map(|(index, (id, _))| (id, index))
            .collect();
        let navmeshes = self
            .versions
            .iter()
            .enumerate()
            .filter_map(|(index, (id, shape))| {
                let mut shape = shape.clone()?;
                shape.used = last.get(id) == Some(&index);
                Some(shape)
            })
            .collect();

        let own: BTreeMap<(RecordId, i32, i32), Vec<i32>> = self
            .ground
            .into_iter()
            .filter_map(|(key, heights)| Some((key, heights?)))
            .collect();
        let mut ground: Vec<GroundCell> = Vec::new();
        for ((world, x, y), heights) in &own {
            ground.push(GroundCell {
                worldspace: world.clone(),
                x: *x,
                y: *y,
                heights: heights.clone(),
            });
        }
        // A city stands on the land of the worldspace around it.
        for (child, parent) in &self.land_of {
            for &(x, y) in self.cells.get(child).into_iter().flatten() {
                if own.contains_key(&(child.clone(), x, y)) {
                    continue;
                }
                if let Some(heights) = own.get(&(parent.clone(), x, y)) {
                    ground.push(GroundCell {
                        worldspace: child.clone(),
                        x,
                        y,
                        heights: heights.clone(),
                    });
                }
            }
        }
        ground.sort_by(|a, b| (&a.worldspace, a.x, a.y).cmp(&(&b.worldspace, b.x, b.y)));
        NearHidden { navmeshes, ground }
    }
}

/// Reads the chosen outside cells of every plugin.
fn outside(plugins: &[(String, PathBuf)], around: &Around, found: &mut Found) {
    let all_cells: BTreeSet<(i32, i32)> = around.worlds.values().flatten().copied().collect();
    let descend = |group: GroupLabel| holds_any(group, &all_cells);
    let request = plugin::Request {
        flat: &[],
        nested: &[*b"WRLD"],
        nested_records: &[*b"WRLD", *b"CELL", NAVMESH, *b"LAND"],
        nested_overrides: &[],
    };
    let wanted_names: HashMap<String, &BTreeSet<(i32, i32)>> = around
        .worlds
        .iter()
        .map(|(name, cells)| (name.to_lowercase(), cells))
        .collect();
    // The worldspaces read, with their names for people.
    let mut worlds: HashMap<RecordId, String> = HashMap::new();
    for (name, path) in plugins {
        let Some(read) = read_where(path, &request, &descend) else {
            continue;
        };
        let masters = &read.header.masters;
        // This plugin's versions of the chosen cells: where they lie.
        let mut cells: HashMap<RecordId, (RecordId, (i32, i32))> = HashMap::new();
        for record in &read.records {
            match &record.signature {
                b"WRLD" => {
                    let id = record_id(record.form_id, masters, name);
                    if worlds.contains_key(&id) {
                        continue;
                    }
                    let own_name = editor_id(record);
                    if let Some(chosen) = own_name
                        .as_ref()
                        .and_then(|own| wanted_names.get(&own.to_lowercase()))
                    {
                        found.cells.insert(id.clone(), (*chosen).clone());
                    } else if let Some(parent) = land_parent(record, masters, name)
                        && let Some(chosen) = found.cells.get(&parent).cloned()
                    {
                        found.cells.insert(id.clone(), chosen);
                        found.land_of.insert(id.clone(), parent);
                    } else {
                        continue;
                    }
                    worlds.insert(id, own_name.unwrap_or_default());
                }
                b"CELL" => {
                    let Some(world) = in_world(record, masters, name) else {
                        continue;
                    };
                    let Some(position) = terrain::cell_position(record) else {
                        continue;
                    };
                    if found
                        .cells
                        .get(&world)
                        .is_some_and(|chosen| chosen.contains(&position))
                    {
                        cells.insert(record_id(record.form_id, masters, name), (world, position));
                    }
                }
                b"LAND" => {
                    let Some((world, (x, y))) = cell_of(record, masters, name, &cells) else {
                        continue;
                    };
                    let heights = (record.flags & FLAG_DELETED == 0)
                        .then(|| record.first(b"VHGT").and_then(terrain::height_grid))
                        .flatten()
                        .map(|heights| heights.into_iter().map(whole).collect());
                    found.ground.insert((world.clone(), *x, *y), heights);
                }
                _ if record.signature == NAVMESH => {
                    let Some((world, (x, y))) = cell_of(record, masters, name, &cells) else {
                        continue;
                    };
                    let place_name = worlds
                        .get(world)
                        .map(|world_name| format!("{world_name} {x},{y}"));
                    let place = Place::Exterior {
                        worldspace: world.clone(),
                    };
                    found
                        .versions
                        .push(version(record, masters, name, place, place_name));
                }
                _ => {}
            }
        }
    }
}

/// Reads the chosen interior cells of every plugin.
fn inside(plugins: &[(String, PathBuf)], around: &Around, found: &mut Found) {
    // A plugin labels a cell's children by its form ID as it writes it,
    // which depends on its masters: the number within the plugin decides
    // which children are entered, the masters which are kept.
    let numbers: BTreeSet<u32> = around.interiors.iter().map(|cell| cell.local_id).collect();
    let descend = |group: GroupLabel| {
        !CHILDREN.contains(&group.group_type) || numbers.contains(&(group.label & 0x00FF_FFFF))
    };
    let request = plugin::Request {
        flat: &[],
        nested: &[*b"CELL"],
        nested_records: &[*b"CELL", NAVMESH],
        nested_overrides: &[],
    };
    let mut names: HashMap<RecordId, String> = HashMap::new();
    for (name, path) in plugins {
        let Some(read) = read_where(path, &request, &descend) else {
            continue;
        };
        let masters = &read.header.masters;
        for record in &read.records {
            if &record.signature == b"CELL" {
                let id = record_id(record.form_id, masters, name);
                if around.interiors.contains(&id)
                    && let Some(cell_name) = editor_id(record)
                {
                    names.insert(id, cell_name);
                }
                continue;
            }
            let Some(cell) = record
                .groups
                .iter()
                .find(|group| group.group_type == group_type::CELL_CHILDREN)
                .map(|group| record_id(group.label, masters, name))
                .filter(|cell| around.interiors.contains(cell))
            else {
                continue;
            };
            let place_name = names.get(&cell).cloned();
            let place = Place::Interior { cell };
            found
                .versions
                .push(version(record, masters, name, place, place_name));
        }
    }
}

/// The groups that hold what is placed in a cell.
const CHILDREN: [u32; 3] = [
    group_type::CELL_CHILDREN,
    group_type::CELL_PERSISTENT_CHILDREN,
    group_type::CELL_TEMPORARY_CHILDREN,
];

/// One plugin's version of a navigation mesh; `None` in the second place
/// if it deletes the mesh or its geometry cannot be read.
fn version(
    record: &Record,
    masters: &[String],
    plugin_name: &str,
    place: Place,
    place_name: Option<String>,
) -> (RecordId, Option<NavmeshShape>) {
    let id = record_id(record.form_id, masters, plugin_name);
    let shape = (record.flags & FLAG_DELETED == 0)
        .then(|| record.first(b"NVNM").and_then(navmesh::geometry))
        .flatten()
        .map(|geometry| NavmeshShape {
            id: id.clone(),
            plugin: plugin_name.to_owned(),
            used: false,
            place,
            place_name,
            vertices: geometry
                .vertices
                .iter()
                .map(|vertex| vertex.map(whole))
                .collect(),
            triangles: geometry
                .triangles
                .iter()
                .map(|triangle| triangle.map(u32::from))
                .collect(),
            edges: geometry
                .edges
                .iter()
                .map(|edges| {
                    edges.map(|beyond| match beyond {
                        navmesh::Beyond::Nothing => Beyond::Nothing,
                        navmesh::Beyond::Triangle(triangle) => Beyond::Triangle(triangle.into()),
                        navmesh::Beyond::Link { navmesh, triangle } => Beyond::Link {
                            navmesh: record_id(navmesh, masters, plugin_name),
                            triangle: triangle.into(),
                        },
                    })
                })
                .collect(),
            doors: geometry
                .doors
                .iter()
                .map(|(triangle, door)| {
                    (u32::from(*triangle), record_id(*door, masters, plugin_name))
                })
                .collect(),
        });
    (id, shape)
}

/// The chosen cell a record is placed in, by its worldspace and position.
fn cell_of<'a>(
    record: &Record,
    masters: &[String],
    plugin_name: &str,
    cells: &'a HashMap<RecordId, (RecordId, (i32, i32))>,
) -> Option<&'a (RecordId, (i32, i32))> {
    let cell = record
        .groups
        .iter()
        .find(|group| group.group_type == group_type::CELL_CHILDREN)?;
    cells.get(&record_id(cell.label, masters, plugin_name))
}

/// The worldspace a record lies in, if it is outside.
fn in_world(record: &Record, masters: &[String], plugin_name: &str) -> Option<RecordId> {
    record
        .groups
        .iter()
        .find(|group| group.group_type == group_type::WORLD_CHILDREN)
        .map(|group| record_id(group.label, masters, plugin_name))
}

/// The worldspace whose land the worldspace `record` uses, if any.
fn land_parent(record: &Record, masters: &[String], plugin_name: &str) -> Option<RecordId> {
    let flags = u16::from_le_bytes(record.first(b"PNAM")?.get(..2)?.try_into().ok()?);
    let parent = u32::from_le_bytes(record.first(b"WNAM")?.get(..4)?.try_into().ok()?);
    (flags & 0x01 != 0 && parent != 0).then(|| record_id(parent, masters, plugin_name))
}

/// Whether a block of outside cells holds one of `cells`. Groups that are
/// not blocks are entered.
fn holds_any(group: GroupLabel, cells: &BTreeSet<(i32, i32)>) -> bool {
    let size = match group.group_type {
        group_type::EXTERIOR_BLOCK => 32,
        group_type::EXTERIOR_SUB_BLOCK => 8,
        _ => return true,
    };
    let [y0, y1, x0, x1] = group.label.to_le_bytes();
    let x = i32::from(i16::from_le_bytes([x0, x1]));
    let y = i32::from(i16::from_le_bytes([y0, y1]));
    cells
        .iter()
        .any(|(cx, cy)| cx.div_euclid(size) == x && cy.div_euclid(size) == y)
}

fn read_where(
    path: &Path,
    request: &plugin::Request<'_>,
    descend: &dyn Fn(GroupLabel) -> bool,
) -> Option<Plugin> {
    let file = File::open(path).ok()?;
    plugin::read_request_where(&mut BufReader::new(file), request, descend).ok()
}

/// A length in whole game units.
#[allow(clippy::cast_possible_truncation)]
fn whole(value: f32) -> i32 {
    // Bounded before converting; coordinates of the games are far inside.
    value.round().clamp(-1.0e9, 1.0e9) as i32
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use mallet_formats_bethesda::fixture::{self, RecordSpec};
    use proptest::prelude::*;

    use super::*;

    /// A navigation mesh field with one triangle at height `z`.
    fn surface(z: f32) -> Vec<u8> {
        let mut data = 12u32.to_le_bytes().to_vec();
        data.extend([0; 12]);
        data.extend(3u32.to_le_bytes());
        for vertex in [[0.0, 0.0, z], [100.0, 0.0, z], [0.0, 100.0, z]] {
            for value in vertex {
                data.extend(f32::to_le_bytes(value));
            }
        }
        data.extend(1u32.to_le_bytes());
        for vertex in [0u16, 1, 2] {
            data.extend(vertex.to_le_bytes());
        }
        data.extend([0; 10]);
        data.extend([0; 8]);
        data
    }

    fn grid(x: i32, y: i32) -> Vec<u8> {
        let mut data = x.to_le_bytes().to_vec();
        data.extend(y.to_le_bytes());
        data.extend(0u32.to_le_bytes());
        data
    }

    fn heights(offset: f32) -> Vec<u8> {
        let mut data = offset.to_le_bytes().to_vec();
        data.extend(vec![0u8; 33 * 33]);
        data.extend([0; 3]);
        data
    }

    fn world() -> RecordSpec {
        RecordSpec::new(*b"WRLD", 0x0000_003C).with(*b"EDID", fixture::zstring("Tamriel"))
    }

    /// The game: a navigation mesh and land outside in cell (5, 3), and
    /// one inside the inn.
    fn game() -> Vec<u8> {
        let inn = RecordSpec::new(*b"CELL", 0x0000_0100).with(*b"EDID", fixture::zstring("Inn"));
        let field = RecordSpec::new(*b"CELL", 0x0000_0200).with(*b"XCLC", grid(5, 3));
        let far = RecordSpec::new(*b"CELL", 0x0000_0300).with(*b"XCLC", grid(40, 40));
        fixture::plugin_with_raw_groups(
            &[],
            0,
            &[
                fixture::interior_cells(&[(
                    inn,
                    vec![RecordSpec::new(NAVMESH, 0x0000_0110).with(*b"NVNM", surface(10.0))],
                )]),
                fixture::worldspace(
                    &world(),
                    &[
                        (
                            field,
                            vec![
                                RecordSpec::new(NAVMESH, 0x0000_0210).with(*b"NVNM", surface(64.0)),
                                RecordSpec::new(*b"LAND", 0x0000_0211).with(*b"VHGT", heights(1.0)),
                            ],
                        ),
                        (
                            far,
                            vec![
                                RecordSpec::new(NAVMESH, 0x0000_0310).with(*b"NVNM", surface(0.0)),
                            ],
                        ),
                    ],
                ),
            ],
        )
    }

    /// A mod that changes the outside mesh of the game.
    fn a_mod() -> Vec<u8> {
        let field = RecordSpec::new(*b"CELL", 0x0000_0200).with(*b"XCLC", grid(5, 3));
        fixture::plugin_with_raw_groups(
            &["Skyrim.esm"],
            0,
            &[fixture::worldspace(
                &world(),
                &[(
                    field,
                    vec![
                        RecordSpec::new(NAVMESH, 0x0000_0210)
                            .with(*b"NVNM", surface(80.0))
                            .compressed(),
                    ],
                )],
            )],
        )
    }

    fn setup() -> (tempfile::TempDir, Vec<(String, PathBuf)>) {
        let directory = tempfile::tempdir().unwrap();
        let mut plugins = Vec::new();
        for (name, data) in [("Skyrim.esm", game()), ("Docks.esp", a_mod())] {
            let path = directory.path().join(name);
            File::create(&path).unwrap().write_all(&data).unwrap();
            plugins.push((name.to_owned(), path));
        }
        (directory, plugins)
    }

    /// A version as compared: the mesh, the plugin of the version, whether
    /// it is used, where it lies and the height of its first corner.
    type Version<'a> = (&'a str, u32, &'a str, bool, Option<&'a str>, i32);

    fn id(plugin: &str, local_id: u32) -> RecordId {
        RecordId {
            plugin: plugin.to_owned(),
            local_id,
        }
    }

    #[test]
    fn every_version_of_the_meshes_in_the_chosen_cells_is_read() {
        let (_directory, plugins) = setup();
        let around = Around {
            worlds: BTreeMap::from([("tamriel".to_owned(), BTreeSet::from([(5, 3)]))]),
            interiors: BTreeSet::from([id("skyrim.esm", 0x100)]),
        };
        let near = read_plugins(&plugins, &around);

        let versions: Vec<Version<'_>> = near
            .navmeshes
            .iter()
            .map(|shape| {
                (
                    shape.id.plugin.as_str(),
                    shape.id.local_id,
                    shape.plugin.as_str(),
                    shape.used,
                    shape.place_name.as_deref(),
                    shape.vertices[0][2],
                )
            })
            .collect();
        assert_eq!(
            versions,
            vec![
                (
                    "skyrim.esm",
                    0x210,
                    "Skyrim.esm",
                    false,
                    Some("Tamriel 5,3"),
                    64
                ),
                (
                    "skyrim.esm",
                    0x210,
                    "Docks.esp",
                    true,
                    Some("Tamriel 5,3"),
                    80
                ),
                ("skyrim.esm", 0x110, "Skyrim.esm", true, Some("Inn"), 10),
            ]
        );
        assert_eq!(near.navmeshes[2].triangles, vec![[0, 1, 2]]);
        assert_eq!(
            near.navmeshes[0].place,
            Place::Exterior {
                worldspace: id("skyrim.esm", 0x3C)
            }
        );
        // The ground: 8 units per step of the heights' offset.
        assert_eq!(near.ground.len(), 1);
        assert_eq!((near.ground[0].x, near.ground[0].y), (5, 3));
        assert_eq!(near.ground[0].heights.len(), 33 * 33);
        assert_eq!(near.ground[0].heights[0], 8);
    }

    #[test]
    fn nothing_chosen_reads_nothing() {
        let (_directory, plugins) = setup();
        assert_eq!(
            read_plugins(&plugins, &Around::default()),
            NearHidden::default()
        );
    }

    proptest! {
        #[test]
        fn never_panics_on_arbitrary_blocks(
            group_type in 0u32..10,
            label in any::<u32>(),
            x in -100i32..100,
            y in -100i32..100,
        ) {
            let _ = holds_any(GroupLabel { group_type, label }, &BTreeSet::from([(x, y)]));
        }
    }
}
