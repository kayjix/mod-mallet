//! Every door of a setup, where it leads, and the door triangles of the
//! navigation meshes (ADR 0067).
//!
//! A door is a reference (`REFR`) whose base object is a `DOOR`. A door
//! that leads somewhere has a teleport field, `XTEL`: the form ID of the
//! door on the other side, then where the player arrives there (three
//! floats) and which way the player faces (three floats), then, in the
//! later games, a 32-bit field of flags. A navigation mesh names the doors
//! characters walk through in its door triangles ([`navmesh::door_triangles`]).
//!
//! Unlike the placed objects of [`crate::objects`], the doors of the game's
//! own plugins are kept: a mod's door at the spot of one of the game's is
//! the case this is for. Reading all doors and navigation meshes of the
//! game's plugins takes a while, so what they hold is kept in the cache
//! ([`Contribution::to_text`]) until the files change.

use std::collections::{HashMap, HashSet};

use mallet_core::facts::RecordId;
use mallet_core::facts::doors::{DoorFacts, DoorLink, EnableParent, PlacedDoor, Teleport};
use mallet_core::facts::objects::{Bounds, Place, PlaceName};
use mallet_formats_bethesda::navmesh;
use mallet_formats_bethesda::plugin::{PluginHeader, Record, Signature, group_type};

use crate::objects::{
    FLAG_DELETED, FLAG_INITIALLY_DISABLED, FLAG_PERSISTENT, bounds, editor_id, floats, model_path,
    place, record_id, whole,
};

/// Under which name the doors of the game's plugins are kept in the cache.
pub const CACHE: &str = "doors-3";

/// The record type of base doors.
pub const DOOR: Signature = *b"DOOR";

/// The records read from the groups of places of the game's own plugins
/// for their doors.
pub const RECORDS: [Signature; 4] = [*b"REFR", *b"NAVM", *b"CELL", *b"WRLD"];

/// The door triangles of a navigation mesh: number and door.
type DoorTriangles = Vec<(u32, RecordId)>;

/// One plugin's version of a door, before it is known whether it wins.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Version {
    base: RecordId,
    place: Place,
    position: [i32; 3],
    rotation: [i32; 3],
    persistent: bool,
    parent: Option<EnableParent>,
    teleport: Option<Teleport>,
}

/// What a collector knows already when it takes in a record.
pub struct Known<'a> {
    /// Whether a base object is a door.
    pub door_base: &'a dyn Fn(&RecordId) -> bool,
    /// Whether a placed object is a door.
    pub door: &'a dyn Fn(&RecordId) -> bool,
    /// Whether a placed object is switched off at the start.
    pub off: &'a dyn Fn(&RecordId) -> bool,
}

/// What one plugin says about doors: its versions of doors (`None` where
/// it deletes or switches one off), its versions of navigation meshes'
/// door triangles (`None` where it deletes a mesh), and the names of its
/// interior cells and worldspaces.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Contribution {
    doors: Vec<(RecordId, Option<Version>)>,
    links: Vec<(RecordId, Option<DoorTriangles>)>,
    names: Vec<(RecordId, String)>,
    /// Whether placed objects that are not doors, and may switch doors on
    /// and off, are switched off at the start.
    off: Vec<(RecordId, bool)>,
}

impl Contribution {
    /// Takes in one record of the plugin `plugin_name`, whose header is
    /// `header`, with what is `known` from earlier plugins.
    pub fn record(
        &mut self,
        plugin_name: &str,
        header: &PluginHeader,
        record: &Record,
        known: &Known<'_>,
    ) {
        let masters = &header.masters;
        let id = || record_id(record.form_id, masters, plugin_name);
        match &record.signature {
            b"REFR" => {
                let id = id();
                let base = record
                    .first(b"NAME")
                    .and_then(|data| data.get(..4)?.try_into().ok())
                    .map(|bytes| record_id(u32::from_le_bytes(bytes), masters, plugin_name));
                let door = record.first(b"XTEL").is_some()
                    || base.as_ref().is_some_and(|base| (known.door_base)(base))
                    || (known.door)(&id);
                let switched_off = record.flags & (FLAG_DELETED | FLAG_INITIALLY_DISABLED) != 0;
                if !door {
                    // It may switch doors on and off.
                    if switched_off || (known.off)(&id) {
                        self.off.push((id, switched_off));
                    }
                    return;
                }
                let version = (!switched_off)
                    .then(|| version(plugin_name, masters, record, base?))
                    .flatten();
                self.doors.push((id, version));
            }
            b"NAVM" => {
                let links = (record.flags & FLAG_DELETED == 0).then(|| {
                    record
                        .first(b"NVNM")
                        .and_then(navmesh::door_triangles)
                        .unwrap_or_default()
                        .into_iter()
                        .map(|(triangle, door)| {
                            (u32::from(triangle), record_id(door, masters, plugin_name))
                        })
                        .collect()
                });
                self.links.push((id(), links));
            }
            b"WRLD" => {
                if let Some(name) = editor_id(record) {
                    self.names.push((id(), name));
                }
            }
            b"CELL" => {
                let outside = record
                    .groups
                    .iter()
                    .any(|group| group.group_type == group_type::WORLD_CHILDREN);
                if !outside && let Some(name) = editor_id(record) {
                    self.names.push((id(), name));
                }
            }
            _ => {}
        }
    }

    /// The contribution as text, for the cache: one line per entry, its
    /// fields apart by tabs.
    pub fn to_text(&self) -> String {
        let mut rows: Vec<Vec<String>> = Vec::new();
        for (id, version) in &self.doors {
            let mut fields = vec!["d".to_owned(), id.to_string()];
            match version {
                None => fields.push("-".to_owned()),
                Some(version) => {
                    let (kind, place) = match &version.place {
                        Place::Interior { cell } => ("i", cell),
                        Place::Exterior { worldspace } => ("w", worldspace),
                    };
                    fields.extend([
                        version.base.to_string(),
                        kind.to_owned(),
                        place.to_string(),
                        triple(version.position),
                        triple(version.rotation),
                        u8::from(version.persistent).to_string(),
                    ]);
                    match &version.parent {
                        Some(parent) => fields.extend([
                            parent.parent.to_string(),
                            u8::from(parent.opposite).to_string(),
                        ]),
                        None => fields.extend(["-".to_owned(), "0".to_owned()]),
                    }
                    if let Some(teleport) = &version.teleport {
                        fields.extend([
                            teleport.door.to_string(),
                            triple(teleport.position),
                            triple(teleport.rotation),
                            teleport.flags.to_string(),
                        ]);
                    }
                }
            }
            rows.push(fields);
        }
        for (navmesh, links) in &self.links {
            let mut fields = vec!["l".to_owned(), navmesh.to_string()];
            match links {
                None => fields.push("-".to_owned()),
                Some(links) => {
                    for (triangle, door) in links {
                        fields.extend([triangle.to_string(), door.to_string()]);
                    }
                }
            }
            rows.push(fields);
        }
        for (id, name) in &self.names {
            rows.push(vec!["n".to_owned(), id.to_string(), name.clone()]);
        }
        for (id, off) in &self.off {
            rows.push(vec![
                "o".to_owned(),
                id.to_string(),
                u8::from(*off).to_string(),
            ]);
        }
        rows.into_iter()
            .map(|fields| fields.join("\t") + "\n")
            .collect()
    }

    /// A contribution read back from [`Self::to_text`]; `None` if the text
    /// is not one.
    pub fn from_text(text: &str) -> Option<Self> {
        let mut contribution = Self::default();
        for line in text.lines() {
            let fields: Vec<&str> = line.split('\t').collect();
            match fields.as_slice() {
                ["d", id, "-"] => contribution.doors.push((parse_id(id)?, None)),
                [
                    "d",
                    id,
                    base,
                    kind,
                    place,
                    position,
                    rotation,
                    persistent,
                    parent,
                    opposite,
                    rest @ ..,
                ] => {
                    let place = parse_id(place)?;
                    let teleport = match rest {
                        [] => None,
                        [door, position, rotation, flags] => Some(Teleport {
                            door: parse_id(door)?,
                            position: parse_triple(position)?,
                            rotation: parse_triple(rotation)?,
                            flags: flags.parse().ok()?,
                        }),
                        _ => return None,
                    };
                    contribution.doors.push((
                        parse_id(id)?,
                        Some(Version {
                            base: parse_id(base)?,
                            place: match *kind {
                                "i" => Place::Interior { cell: place },
                                "w" => Place::Exterior { worldspace: place },
                                _ => return None,
                            },
                            position: parse_triple(position)?,
                            rotation: parse_triple(rotation)?,
                            persistent: *persistent == "1",
                            parent: match *parent {
                                "-" => None,
                                parent => Some(EnableParent {
                                    parent: parse_id(parent)?,
                                    opposite: *opposite == "1",
                                }),
                            },
                            teleport,
                        }),
                    ));
                }
                ["l", navmesh, "-"] => contribution.links.push((parse_id(navmesh)?, None)),
                ["l", navmesh, rest @ ..] => {
                    let links = rest
                        .chunks(2)
                        .map(|pair| match pair {
                            [triangle, door] => Some((triangle.parse().ok()?, parse_id(door)?)),
                            _ => None,
                        })
                        .collect::<Option<Vec<_>>>()?;
                    contribution.links.push((parse_id(navmesh)?, Some(links)));
                }
                ["n", id, name] => contribution.names.push((parse_id(id)?, (*name).to_owned())),
                ["o", id, off] => contribution.off.push((parse_id(id)?, *off == "1")),
                [""] => {}
                _ => return None,
            }
        }
        Some(contribution)
    }
}

/// One plugin's version of a door that is shown, or `None` if it does not
/// say where it stands.
fn version(
    plugin_name: &str,
    masters: &[String],
    record: &Record,
    base: RecordId,
) -> Option<Version> {
    let data = floats(record.first(b"DATA")?.get(..24)?);
    let [x, y, z, rx, ry, rz] = data.as_slice() else {
        return None;
    };
    let teleport = record.first(b"XTEL").and_then(|data| {
        let door = u32::from_le_bytes(data.get(..4)?.try_into().ok()?);
        let values = floats(data.get(4..28)?);
        let [x, y, z, rx, ry, rz] = values.as_slice() else {
            return None;
        };
        let flags = data
            .get(28..32)
            .and_then(|bytes| bytes.try_into().ok())
            .map_or(0, u32::from_le_bytes);
        Some(Teleport {
            door: record_id(door, masters, plugin_name),
            position: [whole(*x)?, whole(*y)?, whole(*z)?],
            rotation: [
                whole(rx * 1000.0)?,
                whole(ry * 1000.0)?,
                whole(rz * 1000.0)?,
            ],
            flags,
        })
    });
    Some(Version {
        base,
        place: place(plugin_name, masters, record)?,
        position: [whole(*x)?, whole(*y)?, whole(*z)?],
        rotation: [
            whole(rx * 1000.0)?,
            whole(ry * 1000.0)?,
            whole(rz * 1000.0)?,
        ],
        persistent: record.flags & FLAG_PERSISTENT != 0
            || record
                .groups
                .iter()
                .any(|group| group.group_type == group_type::CELL_PERSISTENT_CHILDREN),
        parent: record.first(b"XESP").and_then(|data| {
            let parent = u32::from_le_bytes(data.get(..4)?.try_into().ok()?);
            Some(EnableParent {
                parent: record_id(parent, masters, plugin_name),
                opposite: data.get(4).is_some_and(|flags| flags & 1 != 0),
            })
        }),
        teleport,
    })
}

fn triple([a, b, c]: [i32; 3]) -> String {
    format!("{a},{b},{c}")
}

fn parse_triple(text: &str) -> Option<[i32; 3]> {
    let mut parts = text.split(',').map(str::parse::<i32>);
    let triple = [
        parts.next()?.ok()?,
        parts.next()?.ok()?,
        parts.next()?.ok()?,
    ];
    parts.next().is_none().then_some(triple)
}

fn parse_id(text: &str) -> Option<RecordId> {
    let (plugin, local_id) = text.rsplit_once(':')?;
    Some(RecordId {
        plugin: plugin.to_owned(),
        local_id: u32::from_str_radix(local_id, 16).ok()?,
    })
}

/// A door as far as the plugins went: its version the game uses so far,
/// by which plugin, and which plugins had a version of it.
#[derive(Debug, Clone)]
struct Door {
    version: Option<Version>,
    plugin: String,
    official: bool,
    plugins: Vec<String>,
}

/// Gathers the doors of all plugins, which have to be added in load order
/// so that later versions replace earlier ones.
#[derive(Debug, Default)]
pub struct Collector {
    /// The base doors: internal name, box, and whether it has a model.
    bases: HashMap<RecordId, (Option<String>, Option<Bounds>, bool)>,
    doors: HashMap<RecordId, Door>,
    /// The door triangles of each navigation mesh, with the plugin of the
    /// version; `None` where the last version deletes the mesh.
    links: HashMap<RecordId, Option<(String, DoorTriangles)>>,
    names: HashMap<RecordId, String>,
    /// The placed objects that are not doors and are switched off at the
    /// start.
    off: HashSet<RecordId>,
}

impl Collector {
    /// Takes in the base doors of a plugin, from its records of the type
    /// [`DOOR`]. Called for every plugin, before its placed doors.
    pub fn add_bases<'r>(
        &mut self,
        plugin_name: &str,
        header: &PluginHeader,
        records: impl IntoIterator<Item = &'r Record>,
    ) {
        for record in records
            .into_iter()
            .filter(|record| record.signature == DOOR)
        {
            let id = record_id(record.form_id, &header.masters, plugin_name);
            let bounds = record.first(b"OBND").and_then(bounds);
            let visible = model_path(record).is_some();
            self.bases.insert(id, (editor_id(record), bounds, visible));
        }
    }

    /// The contribution of a plugin whose records are all at hand.
    pub fn contribution<'r>(
        &self,
        plugin_name: &str,
        header: &PluginHeader,
        records: impl IntoIterator<Item = &'r Record>,
    ) -> Contribution {
        let mut contribution = Contribution::default();
        for record in records {
            self.take(&mut contribution, plugin_name, header, record);
        }
        contribution
    }

    /// Takes one record into `contribution`, with what the collector knows
    /// of base doors and doors.
    pub fn take(
        &self,
        contribution: &mut Contribution,
        plugin_name: &str,
        header: &PluginHeader,
        record: &Record,
    ) {
        contribution.record(
            plugin_name,
            header,
            record,
            &Known {
                door_base: &|base| self.bases.contains_key(base),
                door: &|id| self.doors.contains_key(id),
                off: &|id| self.off.contains(id),
            },
        );
    }

    /// Takes in what the plugin `plugin_name` says about doors.
    pub fn merge(&mut self, plugin_name: &str, official: bool, contribution: Contribution) {
        let plugin = plugin_name.to_lowercase();
        for (id, version) in contribution.doors {
            let door = self.doors.entry(id).or_insert_with(|| Door {
                version: None,
                plugin: plugin.clone(),
                official,
                plugins: Vec::new(),
            });
            door.version = version;
            door.plugin.clone_from(&plugin);
            if !door.plugins.contains(&plugin) {
                door.plugins.push(plugin.clone());
            }
        }
        for (navmesh, links) in contribution.links {
            self.links
                .insert(navmesh, links.map(|links| (plugin.clone(), links)));
        }
        self.names.extend(contribution.names);
        for (id, off) in contribution.off {
            if off {
                self.off.insert(id);
            } else {
                self.off.remove(&id);
            }
        }
    }

    /// The doors as the game will show them.
    pub fn finish(self) -> DoorFacts {
        // A door whose enable parent is switched off at the start, or on
        // where it shows only while its parent is off, is not shown.
        let parent_off = |parent: &RecordId| {
            self.off.contains(parent)
                || self
                    .doors
                    .get(parent)
                    .is_some_and(|door| door.version.is_none())
        };
        let is_shown = |door: &Door| {
            door.version.as_ref().is_some_and(|version| {
                version
                    .parent
                    .as_ref()
                    .is_none_or(|parent| parent_off(&parent.parent) == parent.opposite)
            })
        };
        let hidden: HashSet<RecordId> = self
            .doors
            .iter()
            .filter(|(_, door)| !is_shown(door))
            .map(|(id, _)| id.clone())
            .collect();
        let shown: HashSet<&RecordId> = self
            .doors
            .keys()
            .filter(|id| !hidden.contains(*id))
            .collect();
        let mut links: Vec<DoorLink> = self
            .links
            .iter()
            .filter_map(|(navmesh, links)| Some((navmesh, links.as_ref()?)))
            .flat_map(|(navmesh, (plugin, links))| {
                links
                    .iter()
                    .filter(|(_, door)| shown.contains(door))
                    .map(|(triangle, door)| DoorLink {
                        navmesh: navmesh.clone(),
                        plugin: plugin.clone(),
                        triangle: *triangle,
                        door: door.clone(),
                    })
            })
            .collect();
        links.sort_by(|a, b| {
            (&a.door, &a.navmesh, a.triangle).cmp(&(&b.door, &b.navmesh, b.triangle))
        });
        let mut doors: Vec<PlacedDoor> = self
            .doors
            .into_iter()
            .filter(|(id, _)| !hidden.contains(id))
            .filter_map(|(id, door)| {
                let version = door.version?;
                let (base_name, bounds, visible) = self
                    .bases
                    .get(&version.base)
                    .cloned()
                    .unwrap_or((None, None, true));
                Some(PlacedDoor {
                    changed_by: door
                        .plugins
                        .into_iter()
                        .filter(|plugin| *plugin != door.plugin)
                        .collect(),
                    plugin: door.plugin,
                    official: door.official,
                    base: version.base,
                    base_name,
                    visible,
                    bounds: bounds.map(|bounds| [bounds.min, bounds.max]),
                    place: version.place,
                    position: version.position,
                    rotation: version.rotation,
                    persistent: version.persistent,
                    enable_parent: version.parent,
                    teleport: version.teleport,
                    id,
                })
            })
            .collect();
        doors.sort_by(|a, b| a.id.cmp(&b.id));
        let mut place_names: Vec<PlaceName> = self
            .names
            .into_iter()
            .map(|(id, editor_id)| PlaceName { id, editor_id })
            .collect();
        place_names.sort_by(|a, b| a.id.cmp(&b.id));
        DoorFacts {
            doors,
            links,
            place_names,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use mallet_formats_bethesda::fixture::{self, RecordSpec};
    use mallet_formats_bethesda::plugin::{self, Plugin, Request};
    use proptest::prelude::*;

    use super::*;

    const DOOR_BASE: u32 = 0x0000_0D00;

    fn door_base() -> RecordSpec {
        let mut obnd = Vec::new();
        for value in [-60i16, -10, 0, 60, 10, 200] {
            obnd.extend(value.to_le_bytes());
        }
        RecordSpec::new(DOOR, DOOR_BASE)
            .with(*b"EDID", fixture::zstring("HouseDoor"))
            .with(*b"MODL", fixture::zstring(r"Architecture\HouseDoor.nif"))
            .with(*b"OBND", obnd)
    }

    fn xtel(door: u32, at: [f32; 3]) -> Vec<u8> {
        let mut data = door.to_le_bytes().to_vec();
        for value in at.iter().chain(&[0.0, 0.0, 1.5]) {
            data.extend(value.to_le_bytes());
        }
        data.extend(0u32.to_le_bytes());
        data
    }

    fn navmesh(form_id: u32, doors: &[(u16, u32)]) -> RecordSpec {
        // Version 12, one triangle, no links, then the door triangles.
        let mut nvnm = Vec::new();
        nvnm.extend(12u32.to_le_bytes());
        nvnm.extend([0; 12]);
        nvnm.extend(3u32.to_le_bytes());
        nvnm.extend([0; 36]);
        nvnm.extend(1u32.to_le_bytes());
        nvnm.extend([0; 16]);
        nvnm.extend(0u32.to_le_bytes());
        nvnm.extend(u32::try_from(doors.len()).unwrap().to_le_bytes());
        for (triangle, door) in doors {
            nvnm.extend(triangle.to_le_bytes());
            nvnm.extend([0; 4]);
            nvnm.extend(door.to_le_bytes());
        }
        RecordSpec::new(*b"NAVM", form_id).with(*b"NVNM", nvnm)
    }

    fn read(data: &[u8]) -> Plugin {
        plugin::read_request(
            &mut Cursor::new(data),
            &Request {
                flat: &[DOOR],
                nested: &[*b"CELL", *b"WRLD"],
                nested_records: &RECORDS,
                nested_overrides: &[],
            },
        )
        .unwrap()
    }

    fn add(collector: &mut Collector, name: &str, data: &[u8]) {
        let plugin = read(data);
        collector.add_bases(name, &plugin.header, &plugin.records);
        let contribution = collector.contribution(name, &plugin.header, &plugin.records);
        // What is kept in the cache reads back the same.
        assert_eq!(
            Contribution::from_text(&contribution.to_text()).as_ref(),
            Some(&contribution)
        );
        collector.merge(name, name == "Game.esm", contribution);
    }

    /// The game: a room with a door out and a door in from outside; a mod
    /// that puts a door of its own in front of the game's outside door and
    /// connects it in a navigation mesh of its own.
    fn setup() -> DoorFacts {
        let room = RecordSpec::new(*b"CELL", 0x0000_0E00).with(*b"EDID", fixture::zstring("Room"));
        let world = RecordSpec::new(*b"WRLD", 0x0000_003C).with(*b"EDID", fixture::zstring("Land"));
        let outside = RecordSpec::new(*b"CELL", 0x0000_0E01);
        let inner = fixture::reference(0x0000_0A01, DOOR_BASE, [0.0, 0.0, 0.0], [0.0; 3], None)
            .with(*b"XTEL", xtel(0x0000_0A02, [100.0, 50.0, 0.0]));
        let outer = fixture::reference(0x0000_0A02, DOOR_BASE, [100.0, 0.0, 0.0], [0.0; 3], None)
            .with(*b"XTEL", xtel(0x0000_0A01, [0.0, 50.0, 0.0]));
        let game = fixture::plugin_with_raw_groups(
            &[],
            1,
            &[
                fixture::group(DOOR, 0, &fixture::record(&door_base())),
                fixture::interior_cells(&[(room, vec![inner])]),
                fixture::worldspace(&world, &[(outside.clone(), vec![outer])]),
            ],
        );
        let own = fixture::reference(0x0100_0800, DOOR_BASE, [104.0, 6.0, 0.0], [0.0; 3], None)
            .with(*b"XTEL", xtel(0x0000_0A01, [0.0, 60.0, 0.0]));
        let a_mod = fixture::plugin_with_raw_groups(
            &["Game.esm"],
            0,
            &[fixture::worldspace(
                &world,
                &[(
                    outside,
                    vec![own, navmesh(0x0100_0801, &[(0, 0x0100_0800)])],
                )],
            )],
        );
        let mut collector = Collector::default();
        add(&mut collector, "Game.esm", &game);
        add(&mut collector, "Mod.esp", &a_mod);
        collector.finish()
    }

    fn id(plugin: &str, local_id: u32) -> RecordId {
        RecordId {
            plugin: plugin.to_owned(),
            local_id,
        }
    }

    #[test]
    fn the_doors_of_the_game_and_of_mods_are_kept_with_where_they_lead() {
        let facts = setup();
        assert_eq!(facts.doors.len(), 3);
        let own = facts
            .doors
            .iter()
            .find(|door| door.id == id("mod.esp", 0x800))
            .unwrap();
        assert_eq!(own.plugin, "mod.esp");
        assert_eq!(own.position, [104, 6, 0]);
        assert_eq!(own.base_name.as_deref(), Some("HouseDoor"));
        assert!(own.visible);
        assert_eq!(own.bounds, Some([[-60, -10, 0], [60, 10, 200]]));
        let teleport = own.teleport.as_ref().unwrap();
        assert_eq!(teleport.door, id("game.esm", 0xA01));
        assert_eq!(teleport.position, [0, 60, 0]);
        assert_eq!(teleport.rotation, [0, 0, 1500]);
        assert!(matches!(own.place, Place::Exterior { .. }));
        let inner = facts
            .doors
            .iter()
            .find(|door| door.id == id("game.esm", 0xA01))
            .unwrap();
        assert_eq!(
            inner.place,
            Place::Interior {
                cell: id("game.esm", 0xE00)
            }
        );
    }

    #[test]
    fn the_door_triangles_of_navigation_meshes_are_kept() {
        let facts = setup();
        assert_eq!(
            facts.links,
            vec![DoorLink {
                navmesh: id("mod.esp", 0x801),
                plugin: "mod.esp".to_owned(),
                triangle: 0,
                door: id("mod.esp", 0x800),
            }]
        );
        assert!(
            facts
                .place_names
                .iter()
                .any(|name| name.editor_id == "Room")
        );
    }

    #[test]
    fn a_door_a_later_plugin_switches_off_is_gone() {
        let world = RecordSpec::new(*b"WRLD", 0x0000_003C);
        let outside = RecordSpec::new(*b"CELL", 0x0000_0E01);
        let mut off = fixture::reference(0x0000_0A02, DOOR_BASE, [0.0; 3], [0.0; 3], None);
        off.flags |= FLAG_INITIALLY_DISABLED;
        let game = fixture::plugin_with_raw_groups(
            &[],
            1,
            &[
                fixture::group(DOOR, 0, &fixture::record(&door_base())),
                fixture::worldspace(
                    &world,
                    &[(
                        outside.clone(),
                        vec![fixture::reference(
                            0x0000_0A02,
                            DOOR_BASE,
                            [0.0; 3],
                            [0.0; 3],
                            None,
                        )],
                    )],
                ),
            ],
        );
        let a_mod = fixture::plugin_with_raw_groups(
            &["Game.esm"],
            0,
            &[fixture::worldspace(&world, &[(outside, vec![off])])],
        );
        let mut collector = Collector::default();
        add(&mut collector, "Game.esm", &game);
        add(&mut collector, "Mod.esp", &a_mod);
        assert_eq!(collector.finish().doors.len(), 0);
    }

    #[test]
    fn a_door_switched_off_with_its_enable_parent_is_gone() {
        let world = RecordSpec::new(*b"WRLD", 0x0000_003C);
        let outside = RecordSpec::new(*b"CELL", 0x0000_0E01);
        let mut marker = fixture::reference(0x0000_0B00, 0x0000_0005, [0.0; 3], [0.0; 3], None);
        marker.flags |= FLAG_INITIALLY_DISABLED;
        let xesp = |opposite: u8| {
            let mut data = 0x0000_0B00u32.to_le_bytes().to_vec();
            data.extend([opposite, 0, 0, 0]);
            data
        };
        let with_parent = fixture::reference(0x0000_0A02, DOOR_BASE, [0.0; 3], [0.0; 3], None)
            .with(*b"XESP", xesp(0));
        let instead = fixture::reference(0x0000_0A03, DOOR_BASE, [0.0; 3], [0.0; 3], None)
            .with(*b"XESP", xesp(1));
        let game = fixture::plugin_with_raw_groups(
            &[],
            1,
            &[
                fixture::group(DOOR, 0, &fixture::record(&door_base())),
                fixture::worldspace(&world, &[(outside, vec![marker, with_parent, instead])]),
            ],
        );
        let mut collector = Collector::default();
        add(&mut collector, "Game.esm", &game);
        let facts = collector.finish();
        let ids: Vec<u32> = facts.doors.iter().map(|door| door.id.local_id).collect();
        assert_eq!(ids, vec![0xA03]);
        assert_eq!(
            facts
                .doors
                .first()
                .and_then(|door| door.enable_parent.clone()),
            Some(EnableParent {
                parent: id("game.esm", 0xB00),
                opposite: true
            })
        );
    }

    proptest! {
        #[test]
        fn reading_cached_text_never_panics(text in "\\PC*") {
            let _ = Contribution::from_text(&text);
        }

        #[test]
        fn reading_damaged_doors_never_panics(xtel in proptest::collection::vec(any::<u8>(), 0..40),
                                              data in proptest::collection::vec(any::<u8>(), 0..30)) {
            let spec = RecordSpec::new(*b"REFR", 0x800)
                .with(*b"NAME", DOOR_BASE.to_le_bytes().to_vec())
                .with(*b"DATA", data)
                .with(*b"XTEL", xtel);
            let world = RecordSpec::new(*b"WRLD", 0x0000_003C);
            let outside = RecordSpec::new(*b"CELL", 0x0000_0E01);
            let bytes = fixture::plugin_with_raw_groups(&[], 0, &[fixture::worldspace(&world, &[(outside, vec![spec])])]);
            let mut collector = Collector::default();
            let plugin = read(&bytes);
            let contribution = collector.contribution("Mod.esp", &plugin.header, &plugin.records);
            collector.merge("Mod.esp", false, contribution);
            let _ = collector.finish();
        }
    }
}
