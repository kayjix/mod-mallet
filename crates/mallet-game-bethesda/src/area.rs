//! Everything placed around a point of a worldspace, by every plugin, in
//! the version the game uses: the objects of the game itself as well as
//! those of mods. For looking at a place closely (ADR 0038).
//!
//! Only the blocks of cells around the point are read from each plugin;
//! the rest of the worldspace is skipped without being unpacked. The
//! models of the base objects are read once and kept. Nothing is written.

use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use mallet_core::facts::RecordId;
use mallet_formats_bethesda::plugin::{self, GroupLabel, Record, Signature, group_type};

use crate::game::Game;
use crate::ingest;
use crate::objects::{
    FLAG_DELETED, FLAG_INITIALLY_DISABLED, editor_id, floats, model_path, record_id,
};

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
}

/// The plugins of a setup, for reading areas of its worldspaces.
#[derive(Debug, Default)]
pub struct AreaReader {
    /// The active plugins in load order: name and file.
    plugins: Vec<(String, PathBuf)>,
    /// The models of base objects, once read.
    models: Option<HashMap<RecordId, BaseModel>>,
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
    pub fn objects(&self, world: &str, centre: [f32; 2], radius: f32) -> Vec<AreaObject> {
        let cells = (
            cell_of(centre[0] - radius),
            cell_of(centre[0] + radius),
            cell_of(centre[1] - radius),
            cell_of(centre[1] + radius),
        );
        let descend = |group: GroupLabel| wanted(group, cells);
        let request = plugin::Request {
            flat: &[],
            nested: &[*b"WRLD"],
            nested_records: &[*b"WRLD", *b"REFR"],
            nested_overrides: &[],
        };
        let mut world_id: Option<RecordId> = None;
        let mut found: HashMap<RecordId, Option<AreaObject>> = HashMap::new();
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
            for record in &read.records {
                match &record.signature {
                    b"WRLD" => {
                        if world_id.is_none()
                            && editor_id(record).is_some_and(|id| id.eq_ignore_ascii_case(world))
                        {
                            world_id = Some(record_id(record.form_id, masters, name));
                        }
                    }
                    b"REFR" => {
                        let Some(in_world) = world_of(record, masters, name) else {
                            continue;
                        };
                        if world_id.as_ref() != Some(&in_world) {
                            continue;
                        }
                        let id = record_id(record.form_id, masters, name);
                        let object = area_object(record, masters, name, id.clone());
                        found.insert(id, object);
                    }
                    _ => {}
                }
            }
        }
        let within = |object: &AreaObject| {
            (object.position[0] - centre[0]).abs() <= radius
                && (object.position[1] - centre[1]).abs() <= radius
        };
        let mut objects: Vec<AreaObject> = found.into_values().flatten().filter(within).collect();
        objects.sort_by(|a, b| a.id.cmp(&b.id));
        objects
    }
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

    #[test]
    fn cells_are_found_from_game_units() {
        assert_eq!(cell_of(0.0), 0);
        assert_eq!(cell_of(4095.0), 0);
        assert_eq!(cell_of(4096.0), 1);
        assert_eq!(cell_of(-1.0), -1);
        assert_eq!(cell_of(f32::NAN), 0);
    }
}
