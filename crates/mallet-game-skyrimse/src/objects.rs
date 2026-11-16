//! What the plugins place in the world.
//!
//! A placed object is a reference record (`REFR`) inside the groups of a
//! cell. Its `NAME` field names the base object, `DATA` holds the position
//! and the rotation as six floating-point numbers, and `XSCL` the size
//! if it is not the base object's own. A base object brings the box around
//! its model in its `OBND` field: two corners, three 16-bit numbers each.
//!
//! Whether a base object is solid follows from its model (`MODL`): trigger
//! zones have none, and markers, effects and small plants have theirs in
//! folders of their own. Pieces of landscape have theirs below a
//! `Landscape` folder.
//!
//! [`Collector`] goes through the plugins in load order, keeps the version
//! of each reference and base object that the game will use, and leaves
//! out the objects of the game itself: what the game places as shipped is
//! not a mod's doing. An object of the game that a mod moves, turns or
//! resizes is the exception: it then stands where the mod put it, and
//! counts as that mod's. To tell, the game's own versions of those objects
//! are read afterwards ([`Collector::add_game_versions`]).

use std::collections::{HashMap, HashSet};

use mallet_core::facts::RecordId;
use mallet_core::facts::objects::{
    BaseKind, BaseObject, Bounds, Moved, ObjectFacts, Place, PlaceName, PlacedObject, Pose,
};
use mallet_formats_bethesda::plugin::{self, Plugin, Record, Signature, group_type};

/// The game's own plugins, in lower case.
pub const OFFICIAL_PLUGINS: [&str; 5] = [
    "skyrim.esm",
    "update.esm",
    "dawnguard.esm",
    "hearthfires.esm",
    "dragonborn.esm",
];

/// The record types of base objects whose placement is compared.
pub const BASE_TYPES: [(Signature, BaseKind); 8] = [
    (*b"STAT", BaseKind::Static),
    (*b"MSTT", BaseKind::MovableStatic),
    (*b"SCOL", BaseKind::StaticCollection),
    (*b"TREE", BaseKind::Tree),
    (*b"FURN", BaseKind::Furniture),
    (*b"CONT", BaseKind::Container),
    (*b"DOOR", BaseKind::Door),
    (*b"ACTI", BaseKind::Activator),
];

/// The top-level groups that hold places and what is placed in them.
pub const PLACE_GROUPS: [Signature; 2] = [*b"CELL", *b"WRLD"];

/// The top-level group of the outside world, the one that holds the land.
pub const WORLD_GROUP: [Signature; 1] = [*b"WRLD"];

/// The records read from the place groups: placed objects, cells and
/// worldspaces for their names and positions, and the land.
pub const PLACE_RECORDS: [Signature; 4] = [*b"REFR", *b"CELL", *b"WRLD", *b"LAND"];

/// The records read from the world group of the game's own plugins: the
/// land, and the cells and worldspaces it belongs to.
pub const LAND_RECORDS: [Signature; 3] = [*b"CELL", *b"WRLD", *b"LAND"];

/// Record flag: the record is deleted.
const FLAG_DELETED: u32 = 0x0000_0020;

/// Record flag of references: the object is switched off when the game
/// starts and only appears if a script switches it on.
const FLAG_INITIALLY_DISABLED: u32 = 0x0000_0800;

/// Record flag of references: the object is persistent.
const FLAG_PERSISTENT: u32 = 0x0000_0400;

/// An object of the game counts as moved by a mod if the mod puts it at
/// least this far, in game units, from where the game has it.
const MOVED_DISTANCE: i64 = 32;

/// ... or turns it by at least this much about one axis, in thousandths of
/// a radian (about six degrees) ...
const MOVED_TURN: i32 = 100;

/// ... or changes its size by at least this much, in thousandths.
const MOVED_SCALE: i32 = 100;

/// A full turn, in thousandths of a radian.
const FULL_TURN: i32 = 6283;

/// Positions farther out than this are not real places. The world is a
/// few hundred thousand units wide.
const MAX_COORDINATE: f32 = 1.0e7;

/// Whether `plugin` is one of the game's own plugins.
pub fn is_official(plugin: &str) -> bool {
    OFFICIAL_PLUGINS
        .iter()
        .any(|official| official.eq_ignore_ascii_case(plugin))
}

/// The record types a plugin is read for, besides characters: the base
/// objects always, the places only for plugins that are not the game's.
pub fn wanted_base_types() -> impl Iterator<Item = Signature> {
    BASE_TYPES.iter().map(|(signature, _)| *signature)
}

/// Gathers the objects of all plugins, which have to be added in load
/// order so that later versions replace earlier ones.
#[derive(Debug, Default)]
pub struct Collector {
    bases: HashMap<RecordId, BaseObject>,
    /// `None` where the winning version deletes or switches off the object.
    placed: HashMap<RecordId, Option<PlacedObject>>,
    names: HashMap<RecordId, String>,
    /// The large references the worldspaces list.
    large: HashSet<RecordId>,
    /// Objects of the game that mods change, in the version the game uses;
    /// `None` where it deletes or switches them off.
    game_objects: HashMap<RecordId, Option<PlacedObject>>,
    /// The game's own versions of those objects.
    game_versions: HashMap<RecordId, Option<PlacedObject>>,
}

impl Collector {
    /// Takes in what `plugin`, named `plugin_name`, says about objects.
    pub fn add(&mut self, plugin_name: &str, plugin: &Plugin) {
        let masters = &plugin.header.masters;
        let official = is_official(plugin_name);
        for record in &plugin.records {
            let id = || record_id(record.form_id, masters, plugin_name);
            if let Some(kind) = base_kind(record.signature) {
                if let Some(bounds) = record.first(b"OBND").and_then(bounds) {
                    self.bases.insert(
                        id(),
                        BaseObject {
                            id: id(),
                            editor_id: editor_id(record),
                            name: (!plugin.header.is_localized)
                                .then(|| record.first(b"FULL").map(plugin::zstring))
                                .flatten()
                                .filter(|name| !name.is_empty()),
                            model: model_path(record),
                            kind,
                            solid: is_solid(record),
                            natural: is_natural(record),
                            bounds,
                        },
                    );
                }
                continue;
            }
            match &record.signature {
                b"REFR" if !official => {
                    let id = id();
                    let placed = placed(plugin_name, masters, record, id.clone());
                    // Changes to the game's own objects are kept apart:
                    // they count only if they move the object.
                    if is_official(&id.plugin) {
                        self.game_objects.insert(id, placed);
                    } else {
                        self.placed.insert(id, placed);
                    }
                }
                b"CELL" | b"WRLD" => {
                    if let Some(name) = editor_id(record) {
                        self.names.insert(id(), name);
                    }
                    if &record.signature == b"WRLD" {
                        for field in record
                            .subrecords
                            .iter()
                            .filter(|field| &field.signature == b"RNAM")
                        {
                            self.large.extend(
                                large_references(&field.data)
                                    .into_iter()
                                    .map(|form_id| record_id(form_id, masters, plugin_name)),
                            );
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// Whether mods change objects of the game, so that the game's own
    /// versions of them have to be read with [`Self::add_game_versions`].
    pub fn wants_game_versions(&self) -> bool {
        !self.game_objects.is_empty()
    }

    /// Takes in the game's own versions of the objects that mods change,
    /// from one of the game's plugins. The game's plugins have to be added
    /// in load order, after all plugins went through [`Self::add`], and
    /// with their placed objects read.
    pub fn add_game_versions(&mut self, plugin_name: &str, plugin: &Plugin) {
        let masters = &plugin.header.masters;
        for record in plugin
            .records
            .iter()
            .filter(|record| &record.signature == b"REFR")
        {
            let id = record_id(record.form_id, masters, plugin_name);
            if self.game_objects.contains_key(&id) {
                let placed = placed(plugin_name, masters, record, id.clone());
                self.game_versions.insert(id, placed);
            }
        }
    }

    /// The objects as the game will show them, with the base objects and
    /// places they refer to.
    pub fn finish(self) -> ObjectFacts {
        let moved: Vec<PlacedObject> = self
            .game_objects
            .into_iter()
            .filter_map(|(id, object)| {
                let mut object = object?;
                let from = match self.game_versions.get(&id)? {
                    Some(original) if moves(original, &object) => Some(Pose {
                        position: original.position,
                        rotation: original.rotation,
                        scale: original.scale,
                    }),
                    Some(_) => return None,
                    // The game switches it off; the mod brings it in.
                    None => None,
                };
                object.moved = Some(Moved {
                    by: object.plugin.to_lowercase(),
                    from,
                });
                Some(object)
            })
            .collect();
        let mut placed: Vec<PlacedObject> = self
            .placed
            .into_values()
            .flatten()
            .chain(moved)
            .filter(|object| self.bases.contains_key(&object.base))
            .map(|mut object| {
                object.large = self.large.contains(&object.id);
                object
            })
            .collect();
        placed.sort_by(|a, b| a.id.cmp(&b.id));

        let used: HashSet<&RecordId> = placed.iter().map(|object| &object.base).collect();
        let mut bases: Vec<BaseObject> = self
            .bases
            .into_values()
            .filter(|base| used.contains(&base.id))
            .collect();
        bases.sort_by(|a, b| a.id.cmp(&b.id));

        let mut place_names: Vec<PlaceName> = self
            .names
            .into_iter()
            .map(|(id, editor_id)| PlaceName { id, editor_id })
            .collect();
        place_names.sort_by(|a, b| a.id.cmp(&b.id));

        ObjectFacts {
            placed,
            bases,
            place_names,
        }
    }
}

/// Whether `changed` stands noticeably elsewhere than `original`: moved,
/// turned or resized.
fn moves(original: &PlacedObject, changed: &PlacedObject) -> bool {
    let distance_squared: i64 = original
        .position
        .iter()
        .zip(changed.position)
        .map(|(a, b)| (i64::from(*a) - i64::from(b)).pow(2))
        .sum();
    let turned = original
        .rotation
        .iter()
        .zip(changed.rotation)
        .any(|(a, b)| {
            let difference = (a - b).rem_euclid(FULL_TURN);
            difference.min(FULL_TURN - difference) >= MOVED_TURN
        });
    distance_squared >= MOVED_DISTANCE * MOVED_DISTANCE
        || turned
        || (original.scale - changed.scale).abs() >= MOVED_SCALE
        || original.place != changed.place
}

fn base_kind(signature: Signature) -> Option<BaseKind> {
    BASE_TYPES
        .iter()
        .find(|(base, _)| *base == signature)
        .map(|(_, kind)| *kind)
}

fn record_id(form_id: u32, masters: &[String], plugin_name: &str) -> RecordId {
    let (origin, local_id) = plugin::resolve_form_id(form_id, masters, plugin_name);
    RecordId {
        plugin: origin.to_lowercase(),
        local_id,
    }
}

fn editor_id(record: &Record) -> Option<String> {
    record
        .first(b"EDID")
        .map(plugin::zstring)
        .filter(|name| !name.is_empty())
}

/// The model of a record, as a path below the data directory.
fn model_path(record: &Record) -> Option<String> {
    let model = record.first(b"MODL").map(plugin::zstring)?;
    let model = model.replace('\\', "/").to_ascii_lowercase();
    if model.is_empty() {
        return None;
    }
    Some(if model.starts_with("meshes/") {
        model
    } else {
        format!("meshes/{model}")
    })
}

/// Whether the model of a base object is one that can be seen and bumped
/// into.
fn is_solid(record: &Record) -> bool {
    let Some(model) = record.first(b"MODL").map(plugin::zstring) else {
        return false;
    };
    let model = model.to_ascii_lowercase().replace('/', "\\");
    let file = model.rsplit('\\').next().unwrap_or_default();
    !(model.is_empty()
        || model.starts_with("markers\\")
        || model.contains("effects\\")
        || model.contains("plants\\")
        || file.contains("marker"))
}

/// Whether the model of a base object is a piece of landscape.
fn is_natural(record: &Record) -> bool {
    record
        .first(b"MODL")
        .map(plugin::zstring)
        .is_some_and(|model| {
            model
                .to_ascii_lowercase()
                .replace('/', "\\")
                .contains("landscape\\")
        })
}

/// The box in an `OBND` field.
fn bounds(data: &[u8]) -> Option<Bounds> {
    let values: Vec<i32> = data
        .get(..12)?
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| i32::from(i16::from_le_bytes(*pair)))
        .collect();
    let [x1, y1, z1, x2, y2, z2] = values.as_slice() else {
        return None;
    };
    Some(Bounds {
        min: [*x1.min(x2), *y1.min(y2), *z1.min(z2)],
        max: [*x1.max(x2), *y1.max(y2), *z1.max(z2)],
    })
}

/// The object a reference places, or `None` if it is deleted, switched
/// off, or does not say what or where.
fn placed(
    plugin_name: &str,
    masters: &[String],
    record: &Record,
    id: RecordId,
) -> Option<PlacedObject> {
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
    Some(PlacedObject {
        id,
        plugin: plugin_name.to_owned(),
        base: record_id(base, masters, plugin_name),
        place: place(plugin_name, masters, record)?,
        position: [whole(*x)?, whole(*y)?, whole(*z)?],
        rotation: [
            whole(rx * 1000.0)?,
            whole(ry * 1000.0)?,
            whole(rz * 1000.0)?,
        ],
        scale: whole(scale * 1000.0)?,
        persistent: record.flags & FLAG_PERSISTENT != 0
            || record
                .groups
                .iter()
                .any(|group| group.group_type == group_type::CELL_PERSISTENT_CHILDREN),
        large: false,
        moved: None,
    })
}

/// The references listed in an `RNAM` field of a worldspace: a cell
/// position, a count, then for each reference its form ID and position.
fn large_references(data: &[u8]) -> Vec<u32> {
    let Some(count) = data
        .get(4..8)
        .and_then(|bytes| bytes.try_into().ok())
        .map(u32::from_le_bytes)
    else {
        return Vec::new();
    };
    data.get(8..)
        .unwrap_or_default()
        .as_chunks::<8>()
        .0
        .iter()
        .take(usize::try_from(count).unwrap_or(usize::MAX))
        .filter_map(|entry| entry.get(..4)?.try_into().ok().map(u32::from_le_bytes))
        .collect()
}

/// The cell or worldspace a reference lies in, from the groups around it.
fn place(plugin_name: &str, masters: &[String], record: &Record) -> Option<Place> {
    let label = |wanted: u32| {
        record
            .groups
            .iter()
            .find(|group| group.group_type == wanted)
            .map(|group| record_id(group.label, masters, plugin_name))
    };
    match label(group_type::WORLD_CHILDREN) {
        Some(worldspace) => Some(Place::Exterior { worldspace }),
        None => label(group_type::CELL_CHILDREN).map(|cell| Place::Interior { cell }),
    }
}

fn floats(data: &[u8]) -> Vec<f32> {
    data.as_chunks::<4>()
        .0
        .iter()
        .map(|bytes| f32::from_le_bytes(*bytes))
        .collect()
}

/// A coordinate rounded to a whole number, or `None` if it is not one a
/// real place could have.
#[allow(clippy::cast_possible_truncation)]
fn whole(value: f32) -> Option<i32> {
    // The range check makes the conversion exact up to rounding.
    (value.is_finite() && value.abs() <= MAX_COORDINATE).then(|| value.round() as i32)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use mallet_formats_bethesda::fixture::{self, RecordSpec};
    use mallet_formats_bethesda::plugin::Request;
    use proptest::prelude::*;

    use super::*;

    const HOUSE: u32 = 0x0001_0001;

    fn house() -> RecordSpec {
        let mut obnd = Vec::new();
        for value in [-100i16, -200, 0, 100, 200, 300] {
            obnd.extend(value.to_le_bytes());
        }
        RecordSpec::new(*b"STAT", HOUSE)
            .with(*b"EDID", fixture::zstring("House"))
            .with(*b"OBND", obnd)
            .with(*b"MODL", fixture::zstring(r"Architecture\House.nif"))
    }

    fn read(data: &[u8]) -> Plugin {
        let flat: Vec<Signature> = wanted_base_types().collect();
        plugin::read_request(
            &mut Cursor::new(data),
            &Request {
                flat: &flat,
                nested: &PLACE_GROUPS,
                nested_records: &PLACE_RECORDS,
            },
        )
        .unwrap()
    }

    /// The game with a house, and a mod that places it twice: once in a
    /// room and once outside, turned and enlarged.
    fn setup() -> Collector {
        let game = fixture::plugin(&[], 1, &[(*b"STAT", vec![house()])]);
        let room = RecordSpec::new(*b"CELL", 0x0100_0010).with(*b"EDID", fixture::zstring("Room"));
        let world = RecordSpec::new(*b"WRLD", 0x0000_003C).with(*b"EDID", fixture::zstring("Land"));
        let outside = RecordSpec::new(*b"CELL", 0x0100_0011);
        let a_mod = fixture::plugin_with_raw_groups(
            &["Skyrim.esm"],
            0,
            &[
                fixture::interior_cells(&[(
                    room,
                    vec![fixture::reference(
                        0x0100_0020,
                        HOUSE,
                        [10.4, 20.6, -5.0],
                        [0.0; 3],
                        None,
                    )],
                )]),
                fixture::worldspace(
                    &world,
                    &[(
                        outside,
                        vec![fixture::reference(
                            0x0100_0021,
                            HOUSE,
                            [4096.0, -8192.0, 100.0],
                            [0.0, 0.0, std::f32::consts::FRAC_PI_2],
                            Some(2.0),
                        )],
                    )],
                ),
            ],
        );
        let mut collector = Collector::default();
        collector.add("Skyrim.esm", &read(&game));
        collector.add("Village.esp", &read(&a_mod));
        collector
    }

    fn id(plugin: &str, local_id: u32) -> RecordId {
        RecordId {
            plugin: plugin.to_owned(),
            local_id,
        }
    }

    #[test]
    fn placed_objects_are_read_with_their_place_and_base() {
        let facts = setup().finish();

        let [room, outside] = facts.placed.as_slice() else {
            panic!("expected two objects, got {:?}", facts.placed);
        };
        assert_eq!(room.id, id("village.esp", 0x20));
        assert_eq!(room.plugin, "Village.esp");
        assert_eq!(room.base, id("skyrim.esm", 0x01_0001));
        assert_eq!(
            room.place,
            Place::Interior {
                cell: id("village.esp", 0x10)
            }
        );
        assert_eq!(room.position, [10, 21, -5]);
        assert_eq!(room.scale, 1000);

        assert_eq!(
            outside.place,
            Place::Exterior {
                worldspace: id("skyrim.esm", 0x3C)
            }
        );
        assert_eq!(outside.position, [4096, -8192, 100]);
        assert_eq!(outside.rotation, [0, 0, 1571]);
        assert_eq!(outside.scale, 2000);
    }

    #[test]
    fn base_objects_bring_their_bounds_and_places_their_names() {
        let facts = setup().finish();
        let [base] = facts.bases.as_slice() else {
            panic!("expected one base object, got {:?}", facts.bases);
        };
        assert_eq!(base.editor_id.as_deref(), Some("House"));
        assert_eq!(base.kind, BaseKind::Static);
        assert!(base.solid);
        assert!(!base.natural);
        assert_eq!(
            base.bounds,
            Bounds {
                min: [-100, -200, 0],
                max: [100, 200, 300]
            }
        );
        let names: Vec<_> = facts
            .place_names
            .iter()
            .map(|name| name.editor_id.as_str())
            .collect();
        assert_eq!(names, ["Land", "Room"]);
    }

    #[test]
    fn a_later_plugin_that_switches_an_object_off_removes_it() {
        let mut collector = setup();
        let mut off = fixture::reference(0x0100_0020, HOUSE, [0.0; 3], [0.0; 3], None);
        off.flags |= FLAG_INITIALLY_DISABLED;
        let patch = fixture::plugin_with_raw_groups(
            &["Skyrim.esm", "Village.esp"],
            0,
            &[fixture::interior_cells(&[(
                RecordSpec::new(*b"CELL", 0x0100_0010),
                vec![off],
            )])],
        );
        collector.add("Patch.esp", &read(&patch));

        let facts = collector.finish();
        let ids: Vec<_> = facts
            .placed
            .iter()
            .map(|object| object.id.local_id)
            .collect();
        assert_eq!(ids, [0x21]);
    }

    /// The game with a house in a room, and a mod that puts the house at
    /// `position`, turned by `rotation`. With `game_versions`, the game's
    /// own version is read afterwards, as the ingest does.
    fn game_and_mover(position: [f32; 3], rotation: [f32; 3], game_versions: bool) -> Collector {
        let game = fixture::plugin_with_raw_groups(
            &[],
            1,
            &[
                fixture::group(*b"STAT", 0, &fixture::record(&house())),
                fixture::interior_cells(&[(
                    RecordSpec::new(*b"CELL", 0x10),
                    vec![fixture::reference(0x20, HOUSE, [0.0; 3], [0.0; 3], None)],
                )]),
            ],
        );
        let a_mod = fixture::plugin_with_raw_groups(
            &["Skyrim.esm"],
            0,
            &[fixture::interior_cells(&[(
                RecordSpec::new(*b"CELL", 0x10),
                vec![fixture::reference(0x20, HOUSE, position, rotation, None)],
            )])],
        );
        let mut collector = Collector::default();
        collector.add("Skyrim.esm", &read(&game));
        collector.add("Mover.esp", &read(&a_mod));
        if game_versions {
            assert!(collector.wants_game_versions());
            collector.add_game_versions("Skyrim.esm", &read(&game));
        }
        collector
    }

    #[test]
    fn an_object_of_the_game_that_a_mod_moves_counts_as_the_mods() {
        let placed = game_and_mover([500.0; 3], [0.0; 3], true).finish().placed;

        let [object] = placed.as_slice() else {
            panic!("expected one object, got {placed:?}");
        };
        assert_eq!(object.id, id("skyrim.esm", 0x20));
        assert_eq!(object.position, [500; 3]);
        let moved = object.moved.as_ref().unwrap();
        assert_eq!(moved.by, "mover.esp");
        assert_eq!(moved.from.map(|pose| pose.position), Some([0; 3]));
        assert_eq!(object.owner(), "mover.esp");
    }

    #[test]
    fn an_object_of_the_game_that_a_mod_turns_counts_as_the_mods() {
        let turned = [0.0, 0.0, std::f32::consts::FRAC_PI_2];
        let placed = game_and_mover([0.0; 3], turned, true).finish().placed;
        assert_eq!(placed.len(), 1);
    }

    #[test]
    fn an_object_of_the_game_that_a_mod_only_nudges_is_left_out() {
        let placed = game_and_mover([10.0, -10.0, 5.0], [0.0, 0.0, 0.05], true)
            .finish()
            .placed;
        assert_eq!(placed, Vec::new());
    }

    #[test]
    fn without_the_games_own_versions_changes_to_its_objects_are_left_out() {
        let placed = game_and_mover([500.0; 3], [0.0; 3], false).finish().placed;
        assert_eq!(placed, Vec::new());
    }

    #[test]
    fn a_full_turn_is_no_turn() {
        let object = |rotation| PlacedObject {
            id: id("skyrim.esm", 1),
            plugin: "Skyrim.esm".to_owned(),
            base: id("skyrim.esm", HOUSE),
            place: Place::Interior {
                cell: id("skyrim.esm", 0x10),
            },
            position: [0; 3],
            rotation,
            scale: 1000,
            persistent: false,
            large: false,
            moved: None,
        };
        assert!(!moves(&object([0, 0, 10]), &object([0, 0, 6280])));
        assert!(moves(&object([0, 0, 0]), &object([0, 0, 3141])));
    }

    #[test]
    fn large_references_are_read_from_their_list() {
        let mut data = Vec::new();
        data.extend(3i16.to_le_bytes());
        data.extend((-2i16).to_le_bytes());
        data.extend(2u32.to_le_bytes());
        for form_id in [0x0100_0810u32, 0x0100_0811] {
            data.extend(form_id.to_le_bytes());
            data.extend([0; 4]);
        }
        assert_eq!(large_references(&data), [0x0100_0810, 0x0100_0811]);
        assert_eq!(large_references(&[1, 2]), Vec::<u32>::new());
    }

    #[test]
    fn markers_triggers_effects_and_plants_are_not_solid() {
        let with_model = |model: Option<&str>| {
            let mut record = Record {
                signature: *b"STAT",
                form_id: 1,
                flags: 0,
                subrecords: Vec::new(),
                groups: Vec::new(),
            };
            if let Some(model) = model {
                record.subrecords.push(plugin::Subrecord {
                    signature: *b"MODL",
                    data: fixture::zstring(model),
                });
            }
            is_solid(&record)
        };
        assert!(with_model(Some(r"Architecture\Whiterun\WRHouse01.nif")));
        assert!(with_model(Some("Landscape/Trees/TreeAspen01.nif")));
        assert!(!with_model(None));
        assert!(!with_model(Some(r"Markers\Marker_DragonLanding.nif")));
        assert!(!with_model(Some(r"Furniture\SmelterMarker.nif")));
        assert!(!with_model(Some(r"Effects\FXfireWithEmbers01.nif")));
        assert!(!with_model(Some(r"CreationClub\Effects\Fishing.nif")));
        assert!(!with_model(Some(r"Landscape\Plants\Thicket01.nif")));
    }

    #[test]
    fn a_reference_without_position_or_with_nonsense_is_skipped() {
        let record = Record {
            signature: *b"REFR",
            form_id: 0x0100_0001,
            flags: 0,
            subrecords: Vec::new(),
            groups: Vec::new(),
        };
        assert_eq!(placed("A.esp", &[], &record, id("a.esp", 1)), None);
        assert_eq!(whole(f32::NAN), None);
        assert_eq!(whole(1.0e9), None);
        assert_eq!(whole(-2.5), Some(-3));
    }

    proptest! {
        #[test]
        fn never_panics_on_arbitrary_fields(
            name in proptest::collection::vec(any::<u8>(), 0..8),
            data in proptest::collection::vec(any::<u8>(), 0..32),
            scale in proptest::collection::vec(any::<u8>(), 0..6),
            obnd in proptest::collection::vec(any::<u8>(), 0..14),
        ) {
            let subrecord = |signature: Signature, data: Vec<u8>| {
                mallet_formats_bethesda::plugin::Subrecord { signature, data }
            };
            let record = Record {
                signature: *b"REFR",
                form_id: 0x0100_0001,
                flags: 0,
                subrecords: vec![
                    subrecord(*b"NAME", name),
                    subrecord(*b"DATA", data),
                    subrecord(*b"XSCL", scale),
                ],
                groups: Vec::new(),
            };
            let _ = placed("A.esp", &[], &record, id("a.esp", 1));
            let _ = bounds(&obnd);
            let _ = large_references(&obnd);
        }
    }
}
