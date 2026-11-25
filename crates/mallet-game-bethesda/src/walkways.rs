//! The navigation meshes and the ground in chosen cells, every plugin's
//! version, for telling whether hidden objects carry paths (ADR 0051).
//!
//! Outside, only the blocks of cells that hold a chosen cell are entered;
//! inside, only the children of the chosen cells. A worldspace that stands
//! on the land of a chosen one, such as a city in its walls, counts as
//! chosen with the same cells. Nothing is written.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use mallet_core::facts::RecordId;
use mallet_core::facts::navmeshes::{
    Beyond, GroundCell, NavmeshShape, NearHidden, PlacedDestination,
};
use mallet_core::facts::objects::Place;
use mallet_formats_bethesda::navmesh;
use mallet_formats_bethesda::plugin::{self, GroupLabel, Record, group_type};

use crate::cells::{
    CHILDREN, cell_at, cell_position, holds_any, land_parent, read_where, world_of,
};
use crate::destinations::{self, Collector};
use crate::game::Game;
use crate::ingest;
use crate::navmeshes::NAVMESH;
use crate::records::{FLAG_DELETED, editor_id, record_id};
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
    let plugins = ingest::active_plugin_files(game, game_dir, local_dir);
    read_plugins(&plugins, around)
}

/// The navigation meshes and the ground of the cells `around` in
/// `plugins`, the active plugins in load order by name and file.
pub fn read_plugins(plugins: &[(String, PathBuf)], around: &Around) -> NearHidden {
    read_plugins_with_destinations(plugins, around).0
}

/// The navigation meshes and the ground of the cells `around` in the
/// setup of `game` in `game_dir`, as [`read`], and the places characters
/// walk to there (ADR 0059).
pub fn read_with_destinations(
    game: Game,
    game_dir: &Path,
    local_dir: Option<&Path>,
    around: &Around,
) -> (NearHidden, Vec<PlacedDestination>) {
    let plugins = ingest::active_plugin_files(game, game_dir, local_dir);
    read_plugins_with_destinations(&plugins, around)
}

/// The navigation meshes and the ground of the cells `around` in
/// `plugins`, as [`read_plugins`], and the places characters walk to
/// there: doors, furniture, idle markers and map markers (ADR 0059).
pub fn read_plugins_with_destinations(
    plugins: &[(String, PathBuf)],
    around: &Around,
) -> (NearHidden, Vec<PlacedDestination>) {
    let mut found = Found::default();
    if around.worlds.values().any(|cells| !cells.is_empty()) {
        outside(plugins, around, &mut found);
    }
    if !around.interiors.is_empty() {
        inside(plugins, around, &mut found);
    }
    let destinations = std::mem::take(&mut found.destinations).finish();
    (found.finish(), destinations)
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
    /// The places characters walk to (ADR 0059).
    destinations: Collector,
    /// By worldspace and cell, whether the cell has water and the height
    /// it sets, the last version.
    cell_water: HashMap<(RecordId, i32, i32), (bool, Option<f32>)>,
    /// The height of each worldspace's water where cells set none.
    world_water: HashMap<RecordId, f32>,
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
        let water = |world: &RecordId, x: i32, y: i32| -> Option<i32> {
            let (has_water, own) = self.cell_water.get(&(world.clone(), x, y))?;
            let default = self.world_water.get(world).copied().or_else(|| {
                let parent = self.land_of.get(world)?;
                self.world_water.get(parent).copied()
            });
            has_water.then(|| own.or(default)).flatten().map(whole)
        };
        let mut ground: Vec<GroundCell> = Vec::new();
        for ((world, x, y), heights) in &own {
            ground.push(GroundCell {
                worldspace: world.clone(),
                x: *x,
                y: *y,
                heights: heights.clone(),
                water: water(world, *x, *y),
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
                        water: water(child, x, y).or_else(|| water(parent, x, y)),
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
        flat: &destinations::BASES,
        nested: &[*b"WRLD"],
        nested_records: &[*b"WRLD", *b"CELL", NAVMESH, *b"LAND", *b"REFR"],
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
        let localized = read.header.is_localized;
        // This plugin's versions of the chosen cells: where they lie.
        let mut cells: HashMap<RecordId, (RecordId, (i32, i32))> = HashMap::new();
        for record in &read.records {
            match &record.signature {
                b"WRLD" => {
                    let id = record_id(record.form_id, masters, name);
                    if let Some(water) = terrain::world_water(record) {
                        found.world_water.insert(id.clone(), water);
                    }
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
                    let Some(world) = world_of(record, masters, name) else {
                        continue;
                    };
                    let Some(position) = cell_position(record) else {
                        continue;
                    };
                    if found
                        .cells
                        .get(&world)
                        .is_some_and(|chosen| chosen.contains(&position))
                    {
                        found.cell_water.insert(
                            (world.clone(), position.0, position.1),
                            terrain::cell_water(record),
                        );
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
                b"REFR" => placed_outside(found, record, masters, name, localized),
                signature if destinations::BASES.contains(signature) => {
                    found.destinations.base(record, masters, name, localized);
                }
                _ => {}
            }
        }
    }
}

/// Takes in a placed object outside, if it stands in a chosen cell. Placed
/// objects are kept by where they stand: those the game keeps loaded lie
/// in a cell of their own.
fn placed_outside(
    found: &mut Found,
    record: &Record,
    masters: &[String],
    plugin_name: &str,
    localized: bool,
) {
    let Some(world) = world_of(record, masters, plugin_name) else {
        return;
    };
    let Some(chosen) = found.cells.get(&world) else {
        return;
    };
    let place = Place::Exterior { worldspace: world };
    found.destinations.placed(
        record,
        masters,
        plugin_name,
        localized,
        place,
        |[x, y, _]| chosen.contains(&(cell_at(x), cell_at(y))),
    );
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
        flat: &destinations::BASES,
        nested: &[*b"CELL"],
        nested_records: &[*b"CELL", NAVMESH, *b"REFR"],
        nested_overrides: &[],
    };
    let mut names: HashMap<RecordId, String> = HashMap::new();
    for (name, path) in plugins {
        let Some(read) = read_where(path, &request, &descend) else {
            continue;
        };
        let masters = &read.header.masters;
        let localized = read.header.is_localized;
        for record in &read.records {
            if destinations::BASES.contains(&record.signature) {
                found.destinations.base(record, masters, name, localized);
                continue;
            }
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
            let place = Place::Interior { cell };
            if &record.signature == b"REFR" {
                found
                    .destinations
                    .placed(record, masters, name, localized, place, |_| true);
                continue;
            }
            let place_name = match &place {
                Place::Interior { cell } => names.get(cell).cloned(),
                Place::Exterior { .. } => None,
            };
            found
                .versions
                .push(version(record, masters, name, place, place_name));
        }
    }
}

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

/// A length in whole game units.
#[allow(clippy::cast_possible_truncation)]
fn whole(value: f32) -> i32 {
    // Bounded before converting; coordinates of the games are far inside.
    value.round().clamp(-1.0e9, 1.0e9) as i32
}

#[cfg(test)]
mod tests {
    use std::fs::File;
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
    fn the_doors_and_furniture_of_the_chosen_cells_are_read() {
        let directory = tempfile::tempdir().unwrap();
        let door = RecordSpec::new(*b"DOOR", 0x0000_0050).with(*b"EDID", fixture::zstring("Door"));
        let bed = RecordSpec::new(*b"FURN", 0x0000_0051).with(*b"EDID", fixture::zstring("Bed"));
        let bases = fixture::group(
            *b"DOOR",
            0,
            &[fixture::record(&door), fixture::record(&bed)].concat(),
        );
        let inn = RecordSpec::new(*b"CELL", 0x0000_0100).with(*b"EDID", fixture::zstring("Inn"));
        let field = RecordSpec::new(*b"CELL", 0x0000_0200).with(*b"XCLC", grid(5, 3));
        let mut xtel = 0x0000_0211u32.to_le_bytes().to_vec();
        xtel.extend([0; 28]);
        let inner_door = fixture::reference(0x0000_0111, 0x50, [1.0, 2.0, 3.0], [0.0; 3], None)
            .with(*b"XTEL", xtel);
        let mut xtel = 0x0000_0111u32.to_le_bytes().to_vec();
        xtel.extend([0; 28]);
        let outer_door = fixture::reference(
            0x0000_0211,
            0x50,
            [5.0 * 4096.0 + 10.0, 3.0 * 4096.0 + 10.0, 0.0],
            [0.0; 3],
            None,
        )
        .with(*b"XTEL", xtel);
        // A bed in a cell that was not chosen.
        let far_bed = fixture::reference(0x0000_0212, 0x51, [0.0, 0.0, 0.0], [0.0; 3], None);
        let game = fixture::plugin_with_raw_groups(
            &[],
            0,
            &[
                bases,
                fixture::interior_cells(&[(inn, vec![inner_door])]),
                fixture::worldspace(&world(), &[(field, vec![outer_door, far_bed])]),
            ],
        );
        let path = directory.path().join("Skyrim.esm");
        File::create(&path).unwrap().write_all(&game).unwrap();
        let around = Around {
            worlds: BTreeMap::from([("Tamriel".to_owned(), BTreeSet::from([(5, 3)]))]),
            interiors: BTreeSet::from([id("skyrim.esm", 0x100)]),
        };
        let (_, found) =
            read_plugins_with_destinations(&[("Skyrim.esm".to_owned(), path)], &around);
        let summary: Vec<_> = found
            .iter()
            .map(|found| (found.id.local_id, found.kind, found.leads_to.clone()))
            .collect();
        assert_eq!(
            summary,
            vec![
                (
                    0x111,
                    mallet_core::reach::DestinationKind::Door,
                    Some(id("skyrim.esm", 0x211))
                ),
                (
                    0x211,
                    mallet_core::reach::DestinationKind::Door,
                    Some(id("skyrim.esm", 0x111))
                ),
            ]
        );
        assert_eq!(
            found.first().map(|found| found.place.clone()),
            Some(Place::Interior {
                cell: id("skyrim.esm", 0x100)
            })
        );
    }

    #[test]
    fn the_water_of_a_cell_is_its_own_or_its_worldspace_s() {
        let mut world_heights = (-27_000f32).to_le_bytes().to_vec();
        world_heights.extend((-14_000f32).to_le_bytes());
        let world = world().with(*b"DNAM", world_heights);
        let cell = |form_id: u32, x: i32, own: Option<f32>| {
            let mut spec = RecordSpec::new(*b"CELL", form_id)
                .with(*b"XCLC", grid(x, 3))
                .with(*b"DATA", vec![0x02, 0]);
            if let Some(height) = own {
                spec = spec.with(*b"XCLW", height.to_le_bytes().to_vec());
            }
            spec
        };
        let land = |form_id: u32| RecordSpec::new(*b"LAND", form_id).with(*b"VHGT", heights(0.0));
        let game = fixture::plugin_with_raw_groups(
            &[],
            0,
            &[fixture::worldspace(
                &world,
                &[
                    (cell(0x200, 5, None), vec![land(0x201)]),
                    (cell(0x300, 6, Some(-13_500.0)), vec![land(0x301)]),
                ],
            )],
        );
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("Skyrim.esm");
        File::create(&path).unwrap().write_all(&game).unwrap();
        let around = Around {
            worlds: BTreeMap::from([("Tamriel".to_owned(), BTreeSet::from([(5, 3), (6, 3)]))]),
            interiors: BTreeSet::new(),
        };
        let near = read_plugins(&[("Skyrim.esm".to_owned(), path)], &around);
        let water: Vec<(i32, Option<i32>)> = near
            .ground
            .iter()
            .map(|cell| (cell.x, cell.water))
            .collect();
        assert_eq!(water, vec![(5, Some(-14_000)), (6, Some(-13_500))]);
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
