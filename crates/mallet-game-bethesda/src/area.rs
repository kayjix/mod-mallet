//! Everything placed around a point of a worldspace, by every plugin, in
//! the version the game uses: the objects of the game itself as well as
//! those of mods. For looking at a place closely (ADR 0038).
//!
//! Only the blocks of cells around the point are read from each plugin;
//! the rest of the worldspace is skipped without being unpacked. The
//! models of the base objects are read once and kept. The objects of an
//! interior cell are read by [`AreaReader::interior`] (ADR 0042). Nothing
//! is written.

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use mallet_core::facts::RecordId;
use mallet_formats_bethesda::plugin::{self, GroupLabel, Record, Signature, group_type};

use crate::game::Game;
use crate::ingest;
use crate::objects::{
    FLAG_DELETED, FLAG_INITIALLY_DISABLED, FLAG_PERSISTENT, FLAG_VISIBLE_WHEN_DISTANT, editor_id,
    floats, model_path, record_id,
};
use crate::terrain;

mod interior;

/// Game units per cell side.
const CELL: f32 = 4096.0;

/// The base objects that can be seen in the world, with a model.
const MODEL_TYPES: [Signature; 19] = [
    *b"STAT", *b"MSTT", *b"SCOL", *b"TREE", *b"FURN", *b"CONT", *b"DOOR", *b"ACTI", *b"FLOR",
    *b"LIGH", *b"MISC", *b"WEAP", *b"ARMO", *b"BOOK", *b"ALCH", *b"INGR", *b"KEYM", *b"SLGM",
    *b"AMMO",
];

/// A base object as far as drawing it goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseModel {
    /// Its model, below the data directory (`meshes/...`), lower case.
    pub model: String,
    /// The name players see, or else the internal one.
    pub name: Option<String>,
}

/// One object placed near the point.
#[derive(Debug, Clone, PartialEq)]
pub struct AreaObject {
    /// The reference; its plugin is the one that placed it first.
    pub id: RecordId,
    /// The plugin whose version the game uses, as the load order spells it.
    pub plugin: String,
    /// What is placed.
    pub base: RecordId,
    /// Where, in game units.
    pub position: [f32; 3],
    /// Turned about the x, y and z axes, in radians.
    pub rotation: [f32; 3],
    /// The size relative to the base object.
    pub scale: f32,
    /// Whether the game relies on it being there: it is persistent (other
    /// things refer to it) or part of the distant view.
    pub anchored: bool,
}

/// The ground of one outside cell near the point, in the version the game
/// uses (ADR 0041).
#[derive(Debug, Clone, PartialEq)]
pub struct CellLand {
    /// The cell's column.
    pub x: i32,
    /// The cell's row.
    pub y: i32,
    /// The heights of 33 rows of 33 points, in game units; the southern
    /// row first, each from west to east. Neighbouring cells share their
    /// edge.
    pub heights: Vec<f32>,
    /// The colours of the same points, three bytes each, if the land has
    /// them.
    pub colours: Option<Vec<u8>>,
    /// The textures of its four quarters, south-west first (ADR 0047).
    pub quarters: Vec<crate::land_textures::Quarter>,
}

/// The cells (west, east, south, north) around a point.
type Cells = (i32, i32, i32, i32);

/// What one pass over the plugins found around a point. Objects and land
/// are asked for one after the other; the second asks takes its part
/// from here instead of reading every plugin again.
#[derive(Debug, Default)]
struct Pass {
    world: String,
    cells: Cells,
    objects: Option<HashMap<RecordId, Option<AreaObject>>>,
    land: Option<Vec<CellLand>>,
}

/// The plugins of a setup, for reading areas of its worldspaces.
#[derive(Debug, Default)]
pub struct AreaReader {
    /// The active plugins in load order: name and file.
    plugins: Vec<(String, PathBuf)>,
    /// The models of base objects, once read.
    models: Option<HashMap<RecordId, BaseModel>>,
    /// The part of the last pass not yet asked for.
    last: Option<Pass>,
    /// The interior cells of every plugin by editor ID, once listed.
    interiors: Option<HashMap<String, interior::Interior>>,
}

impl AreaReader {
    /// The active plugins of `game` in `game_dir`; `local_dir` holds
    /// `plugins.txt`, as for [`ingest::read_setup`].
    pub fn open(game: Game, game_dir: &Path, local_dir: Option<&Path>) -> Self {
        let data_dir = game_dir.join("Data");
        let plugins = ingest::active_plugin_names(game, game_dir, local_dir)
            .unwrap_or_default()
            .into_iter()
            .map(|name| {
                let path = data_dir.join(&name);
                (name, path)
            })
            .collect();
        Self {
            plugins,
            models: None,
            last: None,
            interiors: None,
        }
    }

    /// The model of a base object, if it has one that can be seen. The
    /// first call reads the base objects of every plugin.
    pub fn base(&mut self, id: &RecordId) -> Option<&BaseModel> {
        if self.models.is_none() {
            self.models = Some(read_models(&self.plugins));
        }
        self.models.as_ref()?.get(id)
    }

    /// The objects in the worldspace named `world` (its internal name, such
    /// as `Tamriel`) whose position lies within `radius` game units of
    /// `centre` in both directions.
    pub fn objects(&mut self, world: &str, centre: [f32; 2], radius: f32) -> Vec<AreaObject> {
        let cells = cells_around(centre, radius);
        let found = self.part(world, cells, |pass| pass.objects.take());
        let within = |object: &AreaObject| {
            (object.position[0] - centre[0]).abs() <= radius
                && (object.position[1] - centre[1]).abs() <= radius
        };
        let mut objects: Vec<AreaObject> = found.into_values().flatten().filter(within).collect();
        objects.sort_by(|a, b| a.id.cmp(&b.id));
        objects
    }

    /// The ground of the outside cells of the worldspace named `world`
    /// that reach within `radius` game units of `centre` in both
    /// directions: the same cells [`AreaReader::objects`] reads. Cells
    /// without land are left out.
    pub fn land(&mut self, world: &str, centre: [f32; 2], radius: f32) -> Vec<CellLand> {
        let cells = cells_around(centre, radius);
        let mut land = self.part(world, cells, |pass| pass.land.take());
        land.sort_by_key(|cell| (cell.y, cell.x));
        land
    }

    /// The part `part` of a pass over `cells` of `world`: from the last
    /// pass if it is still there, else from a new one, whose other part is
    /// kept for the next ask.
    fn part<T: Default>(
        &mut self,
        world: &str,
        cells: Cells,
        part: impl Fn(&mut Pass) -> Option<T>,
    ) -> T {
        if let Some(found) = self.take(world, cells, &part) {
            return found;
        }
        let mut pass = self.pass(world, cells);
        let found = part(&mut pass).unwrap_or_default();
        self.last = Some(pass);
        found
    }

    /// The part `part` of the last pass, if it read `cells` of `world` and
    /// the part has not been taken yet.
    fn take<T>(
        &mut self,
        world: &str,
        cells: Cells,
        part: impl FnOnce(&mut Pass) -> Option<T>,
    ) -> Option<T> {
        let pass = self.last.as_mut()?;
        if pass.cells != cells || !pass.world.eq_ignore_ascii_case(world) {
            return None;
        }
        let taken = part(pass);
        if pass.objects.is_none() && pass.land.is_none() {
            self.last = None;
        }
        taken
    }

    /// Reads the objects and the land of `cells` of `world` from every
    /// plugin, later plugins replacing earlier ones.
    fn pass(&self, world: &str, cells: Cells) -> Pass {
        let descend = |group: GroupLabel| wanted(group, cells);
        let request = plugin::Request {
            flat: &[],
            nested: &[*b"WRLD"],
            nested_records: &[*b"WRLD", *b"REFR", *b"CELL", *b"LAND"],
            nested_overrides: &[],
        };
        let (west, east, south, north) = cells;
        let inside = |(x, y): (i32, i32)| x >= west && x <= east && y >= south && y <= north;
        let mut world_id: Option<RecordId> = None;
        // The worldspaces that use this one's land, such as its cities.
        let mut children: HashSet<RecordId> = HashSet::new();
        let mut found: HashMap<RecordId, Option<AreaObject>> = HashMap::new();
        let mut land: HashMap<(i32, i32), CellLand> = HashMap::new();
        for (name, path) in &self.plugins {
            let Ok(file) = File::open(path) else {
                continue;
            };
            let Ok(read) =
                plugin::read_request_where(&mut BufReader::new(file), &request, &descend)
            else {
                continue;
            };
            let masters = &read.header.masters;
            // The cells of this plugin in the world, to place its land.
            let mut positions: HashMap<RecordId, (i32, i32)> = HashMap::new();
            for record in &read.records {
                match &record.signature {
                    b"WRLD" => {
                        if world_id.is_none()
                            && editor_id(record).is_some_and(|id| id.eq_ignore_ascii_case(world))
                        {
                            world_id = Some(record_id(record.form_id, masters, name));
                        } else if world_id.is_some()
                            && land_parent(record, masters, name) == world_id
                        {
                            // A city inside its walls stands on this world's
                            // land; its objects are part of the place.
                            children.insert(record_id(record.form_id, masters, name));
                        }
                    }
                    b"REFR" => {
                        let Some(in_world) = world_of(record, masters, name) else {
                            continue;
                        };
                        if world_id.as_ref() != Some(&in_world) && !children.contains(&in_world) {
                            continue;
                        }
                        let id = record_id(record.form_id, masters, name);
                        let object = area_object(record, masters, name, id.clone());
                        found.insert(id, object);
                    }
                    b"CELL" => {
                        if world_id.is_some()
                            && world_of(record, masters, name) == world_id
                            && let Some(position) = terrain::cell_position(record)
                        {
                            positions.insert(record_id(record.form_id, masters, name), position);
                        }
                    }
                    b"LAND" => {
                        if world_id.is_none() || world_of(record, masters, name) != world_id {
                            continue;
                        }
                        let Some(position) = record
                            .groups
                            .iter()
                            .find(|group| group.group_type == group_type::CELL_CHILDREN)
                            .and_then(|group| positions.get(&record_id(group.label, masters, name)))
                            .copied()
                            .filter(|position| inside(*position))
                        else {
                            continue;
                        };
                        match cell_land(record, position) {
                            Some(mut cell) => {
                                cell.quarters =
                                    crate::land_textures::quarters(record, masters, name);
                                land.insert(position, cell);
                            }
                            None if record.flags & FLAG_DELETED != 0 => {
                                land.remove(&position);
                            }
                            None => {}
                        }
                    }
                    _ => {}
                }
            }
        }
        Pass {
            world: world.to_owned(),
            cells,
            objects: Some(found),
            land: Some(land.into_values().collect()),
        }
    }
}

/// The cells (west, east, south, north) within `radius` game units of
/// `centre` in both directions.
fn cells_around(centre: [f32; 2], radius: f32) -> Cells {
    (
        cell_of(centre[0] - radius),
        cell_of(centre[0] + radius),
        cell_of(centre[1] - radius),
        cell_of(centre[1] + radius),
    )
}

/// The ground a `LAND` record gives the cell at `position`; `None` if it
/// has no heights or is deleted.
fn cell_land(record: &Record, (x, y): (i32, i32)) -> Option<CellLand> {
    if record.flags & FLAG_DELETED != 0 {
        return None;
    }
    let heights = terrain::height_grid(record.first(b"VHGT")?)?;
    let points = terrain::GRID * terrain::GRID;
    let colours = record
        .first(b"VCLR")
        .and_then(|data| data.get(..points * 3))
        .map(<[u8]>::to_vec);
    Some(CellLand {
        x,
        y,
        heights,
        colours,
        quarters: Vec::new(),
    })
}

/// The cell a coordinate in game units lies in.
#[allow(clippy::cast_possible_truncation)]
fn cell_of(coordinate: f32) -> i32 {
    // Worldspaces lie within a few hundred cells of their origin.
    (coordinate / CELL).floor().clamp(-32_768.0, 32_767.0) as i32
}

/// Whether a nested group may hold objects of the cells `cells` (west,
/// east, south, north). Blocks of 32 × 32 and 8 × 8 cells are labelled with
/// their place; everything else is entered.
fn wanted(group: GroupLabel, cells: (i32, i32, i32, i32)) -> bool {
    let size = match group.group_type {
        group_type::EXTERIOR_BLOCK => 32,
        group_type::EXTERIOR_SUB_BLOCK => 8,
        _ => return true,
    };
    let [y0, y1, x0, x1] = group.label.to_le_bytes();
    let x = i32::from(i16::from_le_bytes([x0, x1]));
    let y = i32::from(i16::from_le_bytes([y0, y1]));
    let (west, east, south, north) = cells;
    x <= east.div_euclid(size)
        && x >= west.div_euclid(size)
        && y <= north.div_euclid(size)
        && y >= south.div_euclid(size)
}

/// The worldspace whose land the worldspace `record` uses (its parent,
/// `WNAM`, if its parent flags, `PNAM`, say so), if any.
fn land_parent(record: &Record, masters: &[String], plugin_name: &str) -> Option<RecordId> {
    let flags = u16::from_le_bytes(record.first(b"PNAM")?.get(..2)?.try_into().ok()?);
    let parent = u32::from_le_bytes(record.first(b"WNAM")?.get(..4)?.try_into().ok()?);
    (flags & 0x01 != 0 && parent != 0).then(|| record_id(parent, masters, plugin_name))
}

/// The worldspace a placed object is in, if it is outside.
fn world_of(record: &Record, masters: &[String], plugin_name: &str) -> Option<RecordId> {
    record
        .groups
        .iter()
        .find(|group| group.group_type == group_type::WORLD_CHILDREN)
        .map(|group| record_id(group.label, masters, plugin_name))
}

/// A placed object as the game shows it; `None` where this version
/// deletes or switches it off.
fn area_object(
    record: &Record,
    masters: &[String],
    plugin_name: &str,
    id: RecordId,
) -> Option<AreaObject> {
    if record.flags & (FLAG_DELETED | FLAG_INITIALLY_DISABLED) != 0 {
        return None;
    }
    let base = u32::from_le_bytes(record.first(b"NAME")?.get(..4)?.try_into().ok()?);
    let data = floats(record.first(b"DATA")?.get(..24)?);
    let [x, y, z, rx, ry, rz] = data.as_slice() else {
        return None;
    };
    let scale = match record.first(b"XSCL") {
        Some(field) => *floats(field.get(..4)?).first()?,
        None => 1.0,
    };
    let values = [*x, *y, *z, *rx, *ry, *rz, scale];
    if values.iter().any(|value| !value.is_finite()) || scale <= 0.0 {
        return None;
    }
    Some(AreaObject {
        id,
        plugin: plugin_name.to_owned(),
        base: record_id(base, masters, plugin_name),
        position: [*x, *y, *z],
        rotation: [*rx, *ry, *rz],
        scale,
        anchored: record.flags & (FLAG_PERSISTENT | FLAG_VISIBLE_WHEN_DISTANT) != 0,
    })
}

/// The models of the base objects of every plugin, in the versions the
/// game uses. Markers, which the game does not show, are left out.
fn read_models(plugins: &[(String, PathBuf)]) -> HashMap<RecordId, BaseModel> {
    let mut models = HashMap::new();
    for (name, path) in plugins {
        let Ok(file) = File::open(path) else {
            continue;
        };
        let Ok(read) = plugin::read(&mut BufReader::new(file), &MODEL_TYPES) else {
            continue;
        };
        let masters = &read.header.masters;
        for record in &read.records {
            let id = record_id(record.form_id, masters, name);
            match model_path(record).filter(|model| {
                Path::new(model)
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("nif"))
                    && !model.contains("marker")
            }) {
                Some(model) => {
                    let shown = (!read.header.is_localized)
                        .then(|| record.first(b"FULL").map(plugin::zstring))
                        .flatten()
                        .filter(|name| !name.is_empty());
                    models.insert(
                        id,
                        BaseModel {
                            model,
                            name: shown.or_else(|| editor_id(record)),
                        },
                    );
                }
                None => {
                    models.remove(&id);
                }
            }
        }
    }
    models
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(group_type: u32, x: i16, y: i16) -> GroupLabel {
        let [y0, y1] = y.to_le_bytes();
        let [x0, x1] = x.to_le_bytes();
        GroupLabel {
            group_type,
            label: u32::from_le_bytes([y0, y1, x0, x1]),
        }
    }

    #[test]
    fn only_blocks_around_the_point_are_entered() {
        // Cells 4 to 6 east, -2 to 0 north: block (0, -1) and (0, 0).
        let cells = (4, 6, -2, 0);
        assert!(wanted(block(group_type::EXTERIOR_BLOCK, 0, -1), cells));
        assert!(wanted(block(group_type::EXTERIOR_BLOCK, 0, 0), cells));
        assert!(!wanted(block(group_type::EXTERIOR_BLOCK, 1, 0), cells));
        assert!(!wanted(block(group_type::EXTERIOR_BLOCK, -1, -1), cells));
        // Sub-blocks of 8 cells: x 0 (cells 0 to 7), y -1 (cells -8 to -1).
        assert!(wanted(block(group_type::EXTERIOR_SUB_BLOCK, 0, -1), cells));
        assert!(!wanted(block(group_type::EXTERIOR_SUB_BLOCK, 1, -1), cells));
        // Cells and their children are always entered.
        assert!(wanted(block(group_type::CELL_CHILDREN, 99, 99), cells));
    }

    fn land(fields: Vec<(&[u8; 4], Vec<u8>)>, flags: u32) -> Record {
        Record {
            signature: *b"LAND",
            form_id: 0x20,
            flags,
            subrecords: fields
                .into_iter()
                .map(|(signature, data)| plugin::Subrecord {
                    signature: *signature,
                    data,
                })
                .collect(),
            groups: Vec::new(),
        }
    }

    /// A flat height field at `offset` steps, with a rise of one step
    /// towards the north in each row.
    fn rising(offset: f32) -> Vec<u8> {
        let mut data = offset.to_le_bytes().to_vec();
        for _row in 0..33 {
            for column in 0..33 {
                data.push(u8::from(column == 0));
            }
        }
        data.extend([0; 3]);
        data
    }

    #[test]
    fn the_land_of_a_cell_keeps_every_point() {
        let colours = vec![7u8; 33 * 33 * 3];
        let cell = cell_land(
            &land(vec![(b"VHGT", rising(2.0)), (b"VCLR", colours.clone())], 0),
            (3, -2),
        )
        .unwrap();
        assert_eq!((cell.x, cell.y), (3, -2));
        assert_eq!(cell.heights.len(), 33 * 33);
        // The southern row: offset plus the first row's step, in units.
        assert_eq!(cell.heights[0], 3.0 * 8.0);
        assert_eq!(cell.heights[32], 3.0 * 8.0);
        // The northern row has risen one step per row.
        assert_eq!(cell.heights[32 * 33], 35.0 * 8.0);
        assert_eq!(cell.colours, Some(colours));
    }

    #[test]
    fn land_without_heights_or_deleted_is_none() {
        assert!(cell_land(&land(Vec::new(), 0), (0, 0)).is_none());
        assert!(cell_land(&land(vec![(b"VHGT", rising(0.0))], FLAG_DELETED), (0, 0)).is_none());
        // Colours cut short are left out, the heights stay.
        let short = land(vec![(b"VHGT", rising(0.0)), (b"VCLR", vec![1, 2, 3])], 0);
        assert_eq!(cell_land(&short, (0, 0)).unwrap().colours, None);
    }

    #[test]
    fn a_pass_is_taken_part_by_part_and_only_for_its_cells() {
        let mut reader = AreaReader::default();
        let cells = (0, 1, 0, 1);
        reader.last = Some(Pass {
            world: "Tamriel".to_owned(),
            cells,
            objects: Some(HashMap::new()),
            land: Some(Vec::new()),
        });
        // Other cells or another world: read anew.
        assert!(
            reader
                .take("Tamriel", (0, 2, 0, 1), |pass| pass.land.take())
                .is_none()
        );
        assert!(
            reader
                .take("Sovngarde", cells, |pass| pass.land.take())
                .is_none()
        );
        assert!(
            reader
                .take("tamriel", cells, |pass| pass.land.take())
                .is_some()
        );
        // Each part only once; the pass is gone once both are taken.
        assert!(
            reader
                .take("Tamriel", cells, |pass| pass.land.take())
                .is_none()
        );
        assert!(
            reader
                .take("Tamriel", cells, |pass| pass.objects.take())
                .is_some()
        );
        assert!(reader.last.is_none());
    }

    proptest::proptest! {
        #[test]
        fn land_never_panics_on_arbitrary_fields(
            vhgt in proptest::collection::vec(proptest::prelude::any::<u8>(), 0..1200),
            vclr in proptest::collection::vec(proptest::prelude::any::<u8>(), 0..3400),
            flags in proptest::prelude::any::<u32>(),
        ) {
            let _ = cell_land(&land(vec![(b"VHGT", vhgt), (b"VCLR", vclr)], flags), (0, 0));
        }
    }

    #[test]
    fn cells_are_found_from_game_units() {
        assert_eq!(cell_of(0.0), 0);
        assert_eq!(cell_of(4095.0), 0);
        assert_eq!(cell_of(4096.0), 1);
        assert_eq!(cell_of(-1.0), -1);
        assert_eq!(cell_of(f32::NAN), 0);
    }

    #[test]
    fn a_city_uses_its_parents_land_only_if_its_flags_say_so() {
        let world = |fields: Vec<(&[u8; 4], Vec<u8>)>| Record {
            signature: *b"WRLD",
            form_id: 0x1A26F,
            flags: 0,
            subrecords: fields
                .into_iter()
                .map(|(signature, data)| plugin::Subrecord {
                    signature: *signature,
                    data,
                })
                .collect(),
            groups: Vec::new(),
        };
        let masters = vec!["Skyrim.esm".to_owned()];
        let tamriel = RecordId {
            plugin: "skyrim.esm".to_owned(),
            local_id: 0x3C,
        };
        let city = world(vec![
            (b"WNAM", 0x3Cu32.to_le_bytes().to_vec()),
            (b"PNAM", 0x5Fu16.to_le_bytes().to_vec()),
        ]);
        assert_eq!(land_parent(&city, &masters, "Mod.esp"), Some(tamriel));
        let own_land = world(vec![
            (b"WNAM", 0x3Cu32.to_le_bytes().to_vec()),
            (b"PNAM", 0x04u16.to_le_bytes().to_vec()),
        ]);
        assert_eq!(land_parent(&own_land, &masters, "Mod.esp"), None);
        assert_eq!(land_parent(&world(Vec::new()), &masters, "Mod.esp"), None);
    }

    proptest::proptest! {
        #[test]
        fn a_parent_never_panics_on_arbitrary_fields(
            wnam in proptest::collection::vec(proptest::prelude::any::<u8>(), 0..6),
            pnam in proptest::collection::vec(proptest::prelude::any::<u8>(), 0..3),
        ) {
            let record = Record {
                signature: *b"WRLD",
                form_id: 1,
                flags: 0,
                subrecords: vec![
                    plugin::Subrecord { signature: *b"WNAM", data: wnam },
                    plugin::Subrecord { signature: *b"PNAM", data: pnam },
                ],
                groups: Vec::new(),
            };
            let _ = land_parent(&record, &[], "Mod.esp");
        }
    }
}
