//! A plugin that moves placed objects a little (ADR 0060).
//!
//! For each object to move, the version the game uses is found in the load
//! order, together with the versions the game uses of the cell it stands
//! in and, outside, of its worldspace. The plugin holds a copy of each,
//! with every reference to another record rewritten for the plugin's own
//! masters. Only the object's position and rotation change; the copies of
//! the cell and the worldspace are the same as the versions that win, so
//! the plugin changes nothing else, as long as it loads after the plugins
//! it copies from. It names them as masters, so it does.
//!
//! Which bytes of a field are references to other records is known per
//! field and record type, after xEdit's definitions. A field this module
//! does not know is not copied blindly: the object is refused instead.
//! Names a localized plugin keeps in its string tables are written as
//! text, since the new plugin is not localized. The offset table of a
//! worldspace (`OFST`) points into the file it came from and is left out.
//!
//! Nothing here writes a file: [`build`] returns the plugin's bytes.

use std::collections::HashMap;
use std::f64::consts::TAU;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use mallet_core::ObjectRef;
use mallet_core::facts::RecordId;
use mallet_core::moves::{DoorTarget, MoveRefusal, ObjectMove, RefusedMove};
use mallet_formats_bethesda::plugin::{self, GroupLabel, Record, Signature, Subrecord, group_type};
use mallet_formats_bethesda::plugin_write::{
    Children, Format, PluginBuilder, WriteError, WrittenRecord,
};

use crate::cells::{CHILDREN, cell_of_unit, holds_near};
use crate::game::Game;
use crate::records::{FLAG_DELETED, FLAG_INITIALLY_DISABLED, editor_id, record_id};

/// The format of the plugins Mod Mallet writes for `game`, if it writes
/// any for it. The fields copied are those of Skyrim Special Edition.
pub fn plugin_format(game: Game) -> Option<Format> {
    game.writes_plugins().then_some(Format::SKYRIM_SE)
}

/// The active plugins of `game` in `game_dir`, name and file in load
/// order, without Mod Mallet's own plugins of moves, which a new one
/// replaces and must not copy from (ADR 0070); `local_dir` holds
/// `plugins.txt`, as for [`crate::ingest::read_setup`].
pub fn active_plugins(
    game: Game,
    game_dir: &Path,
    local_dir: Option<&Path>,
) -> Vec<(String, PathBuf)> {
    let mut plugins = crate::ingest::active_plugin_files(game, game_dir, local_dir);
    plugins.retain(|(name, _)| !crate::swap::is_moves_plugin(name));
    plugins
}

/// The plugin that moves the objects, and what could not be moved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovesPlugin {
    /// The plugin's bytes; `None` if it would move nothing.
    pub bytes: Option<Vec<u8>>,
    /// Its masters, in load order.
    pub masters: Vec<String>,
    /// Whether it is flagged as light.
    pub light: bool,
    /// The objects it moves.
    pub moved: Vec<ObjectRef>,
    /// The objects it does not move, and why.
    pub refused: Vec<RefusedMove>,
}

/// One version of a record, as a plugin of the load order has it.
#[derive(Debug, Clone)]
pub(crate) struct RecordVersion {
    /// The plugin, by its place in the load order.
    pub(crate) plugin: usize,
    pub(crate) record: Record,
    pub(crate) form_version: u16,
}

/// What a plugin says about itself that copying needs.
#[derive(Debug, Clone, Default)]
pub(crate) struct Source {
    masters: Vec<String>,
    localized: bool,
}

/// The versions the game uses of the objects asked for, their cells and
/// worldspaces.
#[derive(Debug, Default)]
pub(crate) struct Found {
    pub(crate) sources: HashMap<usize, Source>,
    objects: HashMap<RecordId, RecordVersion>,
    pub(crate) cells: HashMap<RecordId, RecordVersion>,
    pub(crate) worlds: HashMap<RecordId, RecordVersion>,
}

impl Found {
    /// Keeps `record` of `plugin` as the latest version of what it is.
    fn add(&mut self, plugin: usize, name: &str, record: Record, form_version: u16) {
        let Some(source) = self.sources.get(&plugin) else {
            return;
        };
        let id = record_id(record.form_id, &source.masters, name);
        let version = RecordVersion {
            plugin,
            record,
            form_version,
        };
        let list = match &version.record.signature {
            b"REFR" => &mut self.objects,
            b"CELL" => &mut self.cells,
            b"WRLD" => &mut self.worlds,
            _ => return,
        };
        list.insert(id, version);
    }
}

/// Builds the plugin that applies `moves` to the setup whose active
/// plugins are `plugins` (name and file, in load order). `names` gives the
/// text of a localized plugin's name by plugin and number, where it can.
///
/// # Errors
///
/// If the plugin cannot be written, for example with more masters than a
/// plugin can name.
pub fn build(
    game: Game,
    plugins: &[(String, PathBuf)],
    moves: &[ObjectMove],
    names: &dyn Fn(&str, u32) -> Option<String>,
) -> Result<MovesPlugin, WriteError> {
    let moves: Vec<&ObjectMove> = moves.iter().filter(|each| !each.is_still()).collect();
    let Some(format) = plugin_format(game) else {
        return Ok(MovesPlugin {
            bytes: None,
            masters: Vec::new(),
            light: false,
            moved: Vec::new(),
            refused: moves
                .iter()
                .map(|each| RefusedMove {
                    object: each.object.clone(),
                    reason: MoveRefusal::UnsupportedGame,
                })
                .collect(),
        });
    };
    let found = find(plugins, &moves);
    let name_of = |plugin: usize| plugins.get(plugin).map_or("", |(name, _)| name.as_str());
    let mut refused = Vec::new();
    let (keep, masters) = planned(&found, plugins, &moves, &mut refused);

    let mut builder = PluginBuilder::new(masters.clone())
        .author("Mod Mallet")
        .description(
            "Moves placed objects a little. Made with Mod Mallet; remove it to put them back.",
        );
    let mut done = Vec::new();
    for (each, plan) in keep {
        let copy = |version: &RecordVersion| {
            let source = found
                .sources
                .get(&version.plugin)
                .cloned()
                .unwrap_or_default();
            copied(version, &source, name_of(version.plugin), &masters, names)
        };
        let (mut object, cell, world) = match (
            copy(plan.object),
            copy(plan.cell),
            plan.world.map(copy).transpose(),
        ) {
            (Ok(object), Ok(cell), Ok(world)) => (object, cell, world),
            (Err(reason), _, _) | (_, Err(reason), _) | (_, _, Err(reason)) => {
                refused.push(RefusedMove {
                    object: each.object.clone(),
                    reason,
                });
                continue;
            }
        };
        let placed =
            (each.offset == [0, 0, 0] && each.turn % 360 == 0) || move_data(&mut object, each);
        let led = each
            .teleport
            .as_ref()
            .is_none_or(|target| retarget(&mut object, target, &masters));
        if !placed || !led {
            refused.push(RefusedMove {
                object: each.object.clone(),
                reason: MoveRefusal::NotFound,
            });
            continue;
        }
        match &world {
            Some(world) => builder.add_to_exterior(world, &cell, plan.children, vec![object]),
            None => builder.add_to_interior(&cell, plan.children, vec![object]),
        }
        done.push(each.object.clone());
    }
    if done.is_empty() {
        return Ok(MovesPlugin {
            bytes: None,
            masters: Vec::new(),
            light: false,
            moved: done,
            refused,
        });
    }
    let light = builder.fits_light(&format);
    Ok(MovesPlugin {
        bytes: Some(builder.write(&format)?),
        masters,
        light,
        moved: done,
        refused,
    })
}

/// What each move copies, and the masters the plugin needs: every plugin
/// whose versions it copies and those they refer to, in load order (a
/// plugin outside it, which the game could not load, after the rest). The
/// moves that cannot be planned go into `refused`.
fn planned<'a>(
    found: &'a Found,
    plugins: &[(String, PathBuf)],
    moves: &[&'a ObjectMove],
    refused: &mut Vec<RefusedMove>,
) -> (Vec<(&'a ObjectMove, Plan<'a>)>, Vec<String>) {
    let name_of = |plugin: usize| plugins.get(plugin).map_or("", |(name, _)| name.as_str());
    let mut needed: Vec<String> = Vec::new();
    let mut keep = Vec::new();
    for each in moves {
        let plan = match plan(found, plugins, each) {
            Ok(plan) => plan,
            Err(reason) => {
                refused.push(RefusedMove {
                    object: each.object.clone(),
                    reason,
                });
                continue;
            }
        };
        let mut wanted = Vec::new();
        let mut unknown = None;
        for version in plan.versions() {
            let source = found
                .sources
                .get(&version.plugin)
                .cloned()
                .unwrap_or_default();
            wanted.push(name_of(version.plugin).to_owned());
            match referenced(&version.record, &source, name_of(version.plugin)) {
                Ok(names) => wanted.extend(names),
                Err(field) => unknown = Some(field),
            }
        }
        // A door led elsewhere names the door it is to lead to.
        if let Some(target) = &each.teleport {
            wanted.push(target.door.plugin.clone());
        }
        if let Some(field) = unknown {
            refused.push(RefusedMove {
                object: each.object.clone(),
                reason: MoveRefusal::UnknownField { field },
            });
            continue;
        }
        for name in wanted {
            if !needed.iter().any(|known| known.eq_ignore_ascii_case(&name)) {
                needed.push(name);
            }
        }
        keep.push((*each, plan));
    }
    let place = |name: &str| {
        plugins
            .iter()
            .position(|(known, _)| known.eq_ignore_ascii_case(name))
            .unwrap_or(usize::MAX)
    };
    needed.sort_by_key(|name| place(name));
    (keep, needed)
}

/// What one move copies: the versions the game uses of the object, its
/// cell and, outside, its worldspace, and among which children of the cell
/// the object lies.
struct Plan<'a> {
    object: &'a RecordVersion,
    cell: &'a RecordVersion,
    world: Option<&'a RecordVersion>,
    children: Children,
}

impl Plan<'_> {
    fn versions(&self) -> impl Iterator<Item = &RecordVersion> {
        [Some(self.object), Some(self.cell), self.world]
            .into_iter()
            .flatten()
    }
}

/// What moving `each` copies, or why it cannot be moved.
fn plan<'a>(
    found: &'a Found,
    plugins: &[(String, PathBuf)],
    each: &ObjectMove,
) -> Result<Plan<'a>, MoveRefusal> {
    let id = RecordId {
        plugin: each.object.plugin.to_lowercase(),
        local_id: each.object.local_id,
    };
    let object = found.objects.get(&id).ok_or(MoveRefusal::NotFound)?;
    if object.record.flags & (FLAG_DELETED | FLAG_INITIALLY_DISABLED) != 0 {
        return Err(MoveRefusal::Gone);
    }
    let name = plugins
        .get(object.plugin)
        .map_or("", |(name, _)| name.as_str());
    let masters = found
        .sources
        .get(&object.plugin)
        .map_or_default(|source| source.masters.as_slice());
    let label = |wanted: u32| {
        object
            .record
            .groups
            .iter()
            .find(|group| group.group_type == wanted)
            .map(|group| record_id(group.label, masters, name))
    };
    let cell = label(group_type::CELL_CHILDREN)
        .and_then(|cell| found.cells.get(&cell))
        .ok_or(MoveRefusal::NotFound)?;
    let world = match label(group_type::WORLD_CHILDREN) {
        Some(world) => Some(found.worlds.get(&world).ok_or(MoveRefusal::NotFound)?),
        None => None,
    };
    let persistent = object
        .record
        .groups
        .iter()
        .any(|group| group.group_type == group_type::CELL_PERSISTENT_CHILDREN);
    Ok(Plan {
        object,
        cell,
        world,
        children: if persistent {
            Children::Persistent
        } else {
            Children::Temporary
        },
    })
}

/// Reads the versions of the objects of `moves`, of their cells and
/// worldspaces from every plugin. Outside, only the blocks of cells around
/// where each object was are read; inside, only the cells named.
fn find(plugins: &[(String, PathBuf)], moves: &[&ObjectMove]) -> Found {
    let mut found = Found::default();
    let points: Vec<(i32, i32)> = moves
        .iter()
        .filter(|each| each.cell.is_none())
        .map(|each| (cell_of_unit(each.from[0]), cell_of_unit(each.from[1])))
        .collect();
    let rooms: Vec<String> = moves
        .iter()
        .filter_map(|each| each.cell.as_ref().map(|cell| cell.to_ascii_lowercase()))
        .collect();
    let wanted: Vec<RecordId> = moves
        .iter()
        .map(|each| RecordId {
            plugin: each.object.plugin.to_lowercase(),
            local_id: each.object.local_id,
        })
        .collect();

    // Inside: first the cells with the names asked for, by plugin and the
    // form IDs that plugin writes for them.
    let mut room_ids: HashMap<usize, Vec<u32>> = HashMap::new();
    if !rooms.is_empty() {
        for (plugin, form_id) in rooms_named(plugins, &rooms) {
            room_ids.entry(plugin).or_default().push(form_id);
        }
    }

    for (index, (name, path)) in plugins.iter().enumerate() {
        let near = |group: GroupLabel| holds_near(group, &points);
        if !points.is_empty() {
            read_into(&mut found, index, name, path, &[*b"WRLD"], &near, &wanted);
        }
        if let Some(ids) = room_ids.get(&index) {
            let inside = |group: GroupLabel| {
                !CHILDREN.contains(&group.group_type) || ids.contains(&group.label)
            };
            read_into(&mut found, index, name, path, &[*b"CELL"], &inside, &wanted);
        }
    }
    found
}

/// Reads one plugin's top-level groups `top` through the nested groups
/// `descend` accepts, keeping the worldspaces, the cells and the objects
/// in `wanted`.
pub(crate) fn read_into(
    found: &mut Found,
    index: usize,
    name: &str,
    path: &Path,
    top: &[Signature],
    descend: &dyn Fn(GroupLabel) -> bool,
    wanted: &[RecordId],
) {
    let Ok(file) = File::open(path) else {
        return;
    };
    let request = plugin::Request {
        flat: &[],
        nested: top,
        nested_records: &[*b"REFR", *b"CELL", *b"WRLD"],
        nested_overrides: &[],
    };
    let mut kept: Vec<(Record, u16)> = Vec::new();
    let mut header = None;
    let read = plugin::read_request_versions(
        &mut BufReader::new(file),
        &request,
        descend,
        &mut |plugin_header, record, version| {
            if header.is_none() {
                header = Some(plugin_header.clone());
            }
            if &record.signature == b"REFR" {
                let id = record_id(record.form_id, &plugin_header.masters, name);
                if !wanted.contains(&id) {
                    return;
                }
            }
            kept.push((record, version));
        },
    );
    let Ok((plugin_header, _)) = read else {
        return;
    };
    found.sources.insert(
        index,
        Source {
            masters: plugin_header.masters,
            localized: plugin_header.is_localized,
        },
    );
    for (record, version) in kept {
        found.add(index, name, record, version);
    }
}

/// The interior cells whose editor ID is one of `rooms` (lower case), as
/// each plugin writes their form IDs: plugin by place, form ID.
fn rooms_named(plugins: &[(String, PathBuf)], rooms: &[String]) -> Vec<(usize, u32)> {
    let request = plugin::Request {
        flat: &[],
        nested: &[*b"CELL"],
        nested_records: &[*b"CELL"],
        nested_overrides: &[],
    };
    let descend = |group: GroupLabel| !CHILDREN.contains(&group.group_type);
    // Every cell's versions, and whether one of them has a name asked for.
    let mut versions: HashMap<RecordId, (bool, Vec<(usize, u32)>)> = HashMap::new();
    for (index, (name, path)) in plugins.iter().enumerate() {
        let Ok(file) = File::open(path) else {
            continue;
        };
        let Ok(read) = plugin::read_request_where(&mut BufReader::new(file), &request, &descend)
        else {
            continue;
        };
        for record in &read.records {
            let id = record_id(record.form_id, &read.header.masters, name);
            let named = editor_id(record)
                .is_some_and(|editor_id| rooms.contains(&editor_id.to_ascii_lowercase()));
            let entry = versions.entry(id).or_default();
            entry.0 |= named;
            entry.1.push((index, record.form_id));
        }
    }
    versions
        .into_values()
        .filter(|(named, _)| *named)
        .flat_map(|(_, versions)| versions)
        .collect()
}

/// Changes the position and rotation of a copied object as `each` says.
/// `false` if it has no place to change.
fn move_data(object: &mut WrittenRecord, each: &ObjectMove) -> bool {
    let Some(field) = object
        .subrecords
        .iter_mut()
        .find(|field| &field.signature == b"DATA")
    else {
        return false;
    };
    let Some(values) = field.data.get(..24).map(|data| {
        data.as_chunks::<4>()
            .0
            .iter()
            .map(|bytes| f32::from_le_bytes(*bytes))
            .collect::<Vec<f32>>()
    }) else {
        return false;
    };
    let [x, y, z, rx, ry, rz] = values.as_slice() else {
        return false;
    };
    #[allow(clippy::cast_precision_loss)]
    let offset = each.offset.map(|value| value as f32);
    let [ox, oy, oz] = offset;
    let rotation = turned([*rx, *ry, *rz], f64::from(each.turn).to_radians());
    let written: Vec<u8> = [x + ox, y + oy, z + oz]
        .into_iter()
        .chain(rotation)
        .flat_map(f32::to_le_bytes)
        .collect();
    let rest = field.data.get(24..).map_or_default(<[u8]>::to_vec);
    field.data = written;
    field.data.extend(rest);
    true
}

/// Lets a copied door lead to `target` (ADR 0067): its teleport (`XTEL`)
/// names the door, written for `masters`, and where the player arrives
/// and faces where `target` says so. A door without a teleport gets one
/// only where `target` says where the player arrives. `false` if the
/// door cannot be named or the teleport cannot be written.
fn retarget(object: &mut WrittenRecord, target: &DoorTarget, masters: &[String]) -> bool {
    let Some(index) = masters
        .iter()
        .position(|master| master.eq_ignore_ascii_case(&target.door.plugin))
        .and_then(|index| u32::try_from(index).ok())
    else {
        return false;
    };
    let door = (index << 24) | (target.door.local_id & 0x00FF_FFFF);
    let at = object
        .subrecords
        .iter()
        .position(|field| &field.signature == b"XTEL");
    let mut data = match at.and_then(|at| object.subrecords.get(at)) {
        Some(field) => field.data.clone(),
        None if target.arrival.is_some() => vec![0; XTEL_LEN],
        None => return false,
    };
    if data.len() < XTEL_LEN - 4 {
        return false;
    }
    let mut put = |offset: usize, bytes: [u8; 4]| {
        if let Some(slot) = data.get_mut(offset..offset + 4) {
            slot.copy_from_slice(&bytes);
        }
    };
    put(0, door.to_le_bytes());
    #[allow(clippy::cast_precision_loss)]
    if let Some(arrival) = target.arrival {
        for (index, value) in arrival.into_iter().enumerate() {
            put(4 + index * 4, (value as f32).to_le_bytes());
        }
    }
    #[allow(clippy::cast_precision_loss)]
    if let Some(facing) = target.facing {
        for (index, value) in facing.into_iter().enumerate() {
            put(16 + index * 4, (value as f32 / 1000.0).to_le_bytes());
        }
    }
    if let Some(field) = at.and_then(|at| object.subrecords.get_mut(at)) {
        field.data = data;
    } else {
        // Before the position, where the game's own doors have it.
        let before = object
            .subrecords
            .iter()
            .position(|field| &field.signature == b"DATA")
            .unwrap_or(object.subrecords.len());
        object.subrecords.insert(
            before,
            Subrecord {
                signature: *b"XTEL",
                data,
            },
        );
    }
    true
}

/// The length of a teleport field of Skyrim Special Edition: the door, where
/// the player arrives and faces, and flags.
const XTEL_LEN: usize = 32;

/// A 3 × 3 matrix, row by row.
type Matrix = [[f64; 3]; 3];

fn multiply(a: &Matrix, b: &Matrix) -> Matrix {
    let mut product = [[0.0; 3]; 3];
    for (row, out) in a.iter().zip(product.iter_mut()) {
        for (column, cell) in out.iter_mut().enumerate() {
            *cell = row
                .iter()
                .zip(b.iter())
                .map(|(value, other)| value * other.get(column).copied().unwrap_or(0.0))
                .sum();
        }
    }
    product
}

fn about_x(angle: f64) -> Matrix {
    let (s, c) = angle.sin_cos();
    [[1.0, 0.0, 0.0], [0.0, c, -s], [0.0, s, c]]
}

fn about_y(angle: f64) -> Matrix {
    let (s, c) = angle.sin_cos();
    [[c, 0.0, s], [0.0, 1.0, 0.0], [-s, 0.0, c]]
}

fn about_z(angle: f64) -> Matrix {
    let (s, c) = angle.sin_cos();
    [[c, -s, 0.0], [s, c, 0.0], [0.0, 0.0, 1.0]]
}

/// The turn of an object as the game places it: about the world's z, then
/// y, then x, each clockwise (`Rx(−x)·Ry(−y)·Rz(−z)`, as the desktop's
/// `placement` and the analyses' matrix have it).
fn placement(rotation: [f64; 3]) -> Matrix {
    let [x, y, z] = rotation;
    multiply(&multiply(&about_x(-x), &about_y(-y)), &about_z(-z))
}

/// The angles of a placement, the inverse of [`placement`].
fn angles(m: &Matrix) -> [f64; 3] {
    let at = |row: usize, column: usize| {
        m.get(row)
            .and_then(|r| r.get(column))
            .copied()
            .unwrap_or(0.0)
    };
    let b = at(0, 2).clamp(-1.0, 1.0).asin();
    let (a, c) = if b.cos().abs() > 1e-9 {
        (
            f64::atan2(-at(1, 2), at(2, 2)),
            f64::atan2(-at(0, 1), at(0, 0)),
        )
    } else {
        // Upright x: only the sum of the turns about x and z counts.
        (0.0, f64::atan2(at(1, 0), at(1, 1)))
    };
    [-a, -b, -c].map(|angle| angle.rem_euclid(TAU))
}

/// The rotation of an object (radians about x, y and z, as the game keeps
/// it) after turning it by `turn` radians about the world's upright axis,
/// clockwise seen from above as the game counts.
pub fn turned(rotation: [f32; 3], turn: f64) -> [f32; 3] {
    let [x, y, z] = rotation.map(f64::from);
    #[allow(clippy::cast_possible_truncation)]
    let narrow = |angle: f64| angle as f32;
    if x.abs() < 1e-7 && y.abs() < 1e-7 {
        // Upright: the turn adds to the angle about z, exactly.
        let [rx, ry, _] = rotation;
        return [rx, ry, narrow((z + turn).rem_euclid(TAU))];
    }
    let m = multiply(&about_z(-turn), &placement([x, y, z]));
    angles(&m).map(narrow)
}

/// How the bytes of a field hold references to other records.
enum Layout {
    /// None.
    Plain,
    /// A form ID at each of these offsets, where the field is long enough.
    At(&'static [usize]),
    /// Nothing but form IDs.
    Array,
    /// A name, kept in a string table if the plugin is localized.
    Name,
    /// Large references of a worldspace: a cell, a count, and per
    /// reference a form ID and a cell.
    LargeReferences,
    /// Scripts attached, with their properties.
    Scripts,
    /// A topic of a patrol: a form ID only if its type is 0.
    Topic,
    /// Left out of the copy.
    Dropped,
}

/// How the field `field` of a record of type `record` is laid out; `None`
/// for a field not known.
fn layout(record: Signature, field: Signature) -> Option<Layout> {
    /// Per record type: the fields without references, those that are one
    /// form ID at their start, and those that are nothing but form IDs.
    type Fields = (
        &'static [Signature],
        &'static [Signature],
        &'static [Signature],
    );
    const REFR: Fields = (
        &[
            *b"DATA", *b"XMBO", *b"XPRM", *b"XOCP", *b"XPTL", *b"XRMR", *b"XMBP", *b"XRGD",
            *b"XRGB", *b"XRDS", *b"XLIG", *b"XALP", *b"XWCN", *b"XWCS", *b"XWCU", *b"XSCL",
            *b"XAPD", *b"XLCM", *b"XTRI", *b"XIS2", *b"XRNK", *b"XCNT", *b"XCHG", *b"XACT",
            *b"ONAM", *b"XHTW", *b"XFVC", *b"XMRK", *b"FNAM", *b"TNAM", *b"XPPA", *b"XPRD",
            *b"XLOD", *b"XCZA",
        ],
        &[
            *b"NAME", *b"LNAM", *b"INAM", *b"XLRM", *b"XEMI", *b"XTNM", *b"XMBR", *b"XSPC",
            *b"XLIB", *b"XLCN", *b"XEZN", *b"XNDP", *b"XOWN", *b"XLRL", *b"XESP", *b"XATR",
            *b"XLTW", *b"XPWR", *b"XCZR", *b"XCZC", *b"XAPR",
        ],
        &[*b"XORD", *b"XPOD", *b"XLRT"],
    );
    const CELL: Fields = (
        &[
            *b"DATA", *b"XCLC", *b"XCLL", *b"TVDT", *b"MHDT", *b"LNAM", *b"XCLW", *b"XNAM",
            *b"XWCN", *b"XWCS", *b"XWCU", *b"XRNK", *b"XWEM",
        ],
        &[
            *b"LTMP", *b"XLCN", *b"XCWT", *b"XOWN", *b"XILL", *b"XCCM", *b"XCAS", *b"XEZN",
            *b"XCMO", *b"XCIM",
        ],
        &[*b"XCLR"],
    );
    const WRLD: Fields = (
        &[
            *b"MHDT", *b"WCTR", *b"PNAM", *b"NAM4", *b"DNAM", *b"MNAM", *b"ONAM", *b"NAMA",
            *b"DATA", *b"NAM0", *b"NAM9", *b"NNAM", *b"XNAM", *b"TNAM", *b"UNAM", *b"XWEM",
        ],
        &[
            *b"LTMP", *b"XEZN", *b"XLCN", *b"WNAM", *b"CNAM", *b"NAM2", *b"NAM3", *b"ZNAM",
        ],
        &[],
    );
    let (plain, ids, arrays) = match &record {
        b"REFR" => REFR,
        b"CELL" => CELL,
        b"WRLD" => WRLD,
        _ => return None,
    };
    let layout = if &field == b"EDID" || plain.contains(&field) {
        Layout::Plain
    } else if &field == b"FULL" {
        Layout::Name
    } else if ids.contains(&field) {
        Layout::At(&[0])
    } else if arrays.contains(&field) {
        Layout::Array
    } else {
        match (&record, &field) {
            (b"REFR", b"VMAD") => Layout::Scripts,
            (b"REFR", b"PDTO") => Layout::Topic,
            (b"REFR", b"XLKR") => Layout::At(&[0, 4]),
            (b"REFR", b"XTEL") => Layout::At(&[0, 32]),
            (b"REFR", b"XLOC") => Layout::At(&[4]),
            (b"WRLD", b"RNAM") => Layout::LargeReferences,
            (b"WRLD", b"OFST") => Layout::Dropped,
            _ => return None,
        }
    };
    Some(layout)
}

/// Where the form IDs of a field lie; `None` if the field does not have
/// the shape its layout says.
fn form_id_offsets(layout: &Layout, data: &[u8]) -> Option<Vec<usize>> {
    match layout {
        Layout::Plain | Layout::Name | Layout::Dropped => Some(Vec::new()),
        Layout::At(offsets) => Some(
            offsets
                .iter()
                .copied()
                .filter(|offset| offset + 4 <= data.len())
                .collect(),
        ),
        Layout::Array => data
            .len()
            .is_multiple_of(4)
            .then(|| (0..data.len()).step_by(4).collect()),
        Layout::LargeReferences => {
            let count = u32::from_le_bytes(data.get(4..8)?.try_into().ok()?);
            let count = usize::try_from(count).ok()?;
            (count.checked_mul(8)?.checked_add(8)? == data.len())
                .then(|| (0..count).map(|index| 8 + index * 8).collect())
        }
        Layout::Topic => {
            let kind = u32::from_le_bytes(data.get(0..4)?.try_into().ok()?);
            Some(if kind == 0 && data.len() >= 8 {
                vec![4]
            } else {
                Vec::new()
            })
        }
        Layout::Scripts => script_form_ids(data),
    }
}

/// Where the form IDs of the objects in a placed object's scripts lie.
/// `None` if the field is not laid out as a placed object's scripts are.
fn script_form_ids(data: &[u8]) -> Option<Vec<usize>> {
    struct Cursor<'a> {
        data: &'a [u8],
        at: usize,
    }
    impl Cursor<'_> {
        fn take(&mut self, count: usize) -> Option<&[u8]> {
            let end = self.at.checked_add(count)?;
            let slice = self.data.get(self.at..end)?;
            self.at = end;
            Some(slice)
        }
        fn u8(&mut self) -> Option<u8> {
            self.take(1)?.first().copied()
        }
        fn u16(&mut self) -> Option<u16> {
            Some(u16::from_le_bytes(self.take(2)?.try_into().ok()?))
        }
        fn u32(&mut self) -> Option<usize> {
            usize::try_from(u32::from_le_bytes(self.take(4)?.try_into().ok()?)).ok()
        }
        fn text(&mut self) -> Option<()> {
            let length = usize::from(self.u16()?);
            self.take(length).map(|_| ())
        }
    }
    let mut cursor = Cursor { data, at: 0 };
    let version = cursor.u16()?;
    let object_format = cursor.u16()?;
    let scripts = cursor.u16()?;
    // Where in an object property its form ID lies.
    let object_id = match object_format {
        1 => 0,
        2 => 4,
        _ => return None,
    };
    let mut offsets = Vec::new();
    for _ in 0..scripts {
        cursor.text()?;
        if version >= 4 {
            cursor.u8()?;
        }
        let properties = cursor.u16()?;
        for _ in 0..properties {
            cursor.text()?;
            let kind = cursor.u8()?;
            if version >= 4 {
                cursor.u8()?;
            }
            match kind {
                1 => {
                    offsets.push(cursor.at + object_id);
                    cursor.take(8)?;
                }
                2 => cursor.text()?,
                3 | 4 => {
                    cursor.take(4)?;
                }
                5 => {
                    cursor.take(1)?;
                }
                11 => {
                    for _ in 0..cursor.u32()? {
                        offsets.push(cursor.at + object_id);
                        cursor.take(8)?;
                    }
                }
                12 => {
                    for _ in 0..cursor.u32()? {
                        cursor.text()?;
                    }
                }
                13 | 14 => {
                    let count = cursor.u32()?;
                    cursor.take(count.checked_mul(4)?)?;
                }
                15 => {
                    let count = cursor.u32()?;
                    cursor.take(count)?;
                }
                _ => return None,
            }
        }
    }
    // A placed object's scripts are followed by nothing.
    (cursor.at == data.len()).then_some(offsets)
}

/// The record type and field, for saying which one is not known.
fn field_name(record: Signature, field: Signature) -> String {
    format!(
        "{} {}",
        String::from_utf8_lossy(&record),
        String::from_utf8_lossy(&field)
    )
}

/// The plugins whose records `record` refers to, by name as `source`
/// spells them; `Err` with the field that is not known.
pub(crate) fn referenced(
    record: &Record,
    source: &Source,
    plugin_name: &str,
) -> Result<Vec<String>, String> {
    let mut names: Vec<String> = Vec::new();
    for field in &record.subrecords {
        let unknown = || field_name(record.signature, field.signature);
        let layout = layout(record.signature, field.signature).ok_or_else(unknown)?;
        let offsets = form_id_offsets(&layout, &field.data).ok_or_else(unknown)?;
        for offset in offsets {
            let Some(form_id) = read_u32(&field.data, offset).filter(|id| *id != 0) else {
                continue;
            };
            let (origin, _) = plugin::resolve_form_id(form_id, &source.masters, plugin_name);
            if !names.iter().any(|known| known.eq_ignore_ascii_case(origin)) {
                names.push(origin.to_owned());
            }
        }
    }
    Ok(names)
}

fn read_u32(data: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        data.get(offset..offset.checked_add(4)?)?.try_into().ok()?,
    ))
}

/// A copy of `version` with its references rewritten for `masters`, names
/// of a localized plugin written as text, and the worldspace's offset
/// table left out.
pub(crate) fn copied(
    version: &RecordVersion,
    source: &Source,
    plugin_name: &str,
    masters: &[String],
    names: &dyn Fn(&str, u32) -> Option<String>,
) -> Result<WrittenRecord, MoveRefusal> {
    let record = &version.record;
    let remap = |form_id: u32| -> Option<u32> {
        if form_id == 0 {
            return Some(0);
        }
        let (origin, local) = plugin::resolve_form_id(form_id, &source.masters, plugin_name);
        let index = masters
            .iter()
            .position(|master| master.eq_ignore_ascii_case(origin))?;
        let index = u32::try_from(index).ok()?;
        Some((index << 24) | local)
    };
    let form_id = remap(record.form_id).ok_or(MoveRefusal::NotFound)?;
    let mut fields = Vec::new();
    for field in &record.subrecords {
        let unknown = || MoveRefusal::UnknownField {
            field: field_name(record.signature, field.signature),
        };
        let layout = layout(record.signature, field.signature).ok_or_else(unknown)?;
        match layout {
            Layout::Dropped => continue,
            Layout::Name if source.localized => {
                let id = read_u32(&field.data, 0).ok_or(MoveRefusal::NameNotFound)?;
                let text = if id == 0 {
                    String::new()
                } else {
                    names(plugin_name, id).ok_or(MoveRefusal::NameNotFound)?
                };
                fields.push(Subrecord {
                    signature: field.signature,
                    data: zstring(&text),
                });
                continue;
            }
            _ => {}
        }
        let offsets = form_id_offsets(&layout, &field.data).ok_or_else(unknown)?;
        let mut data = field.data.clone();
        for offset in offsets {
            let Some(old) = read_u32(&data, offset) else {
                continue;
            };
            let new = remap(old).ok_or(MoveRefusal::NotFound)?;
            if let Some(slot) = data.get_mut(offset..offset + 4) {
                slot.copy_from_slice(&new.to_le_bytes());
            }
        }
        fields.push(Subrecord {
            signature: field.signature,
            data,
        });
    }
    Ok(WrittenRecord {
        signature: record.signature,
        form_id,
        flags: record.flags,
        form_version: version.form_version,
        subrecords: fields,
    })
}

/// Text in the game's code page, ended by a NUL byte; characters beyond
/// it become `?`.
fn zstring(text: &str) -> Vec<u8> {
    let mut bytes: Vec<u8> = text
        .chars()
        .map(|character| u8::try_from(u32::from(character)).unwrap_or(b'?'))
        .collect();
    bytes.push(0);
    bytes
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use mallet_formats_bethesda::fixture::{self, RecordSpec};
    use proptest::prelude::*;

    use super::*;

    const MASTER: &str = "Skyrim.esm";

    fn grid(x: i32, y: i32) -> Vec<u8> {
        let mut data = x.to_le_bytes().to_vec();
        data.extend(y.to_le_bytes());
        data.extend(0u32.to_le_bytes());
        data
    }

    fn ids(values: &[u32]) -> Vec<u8> {
        values
            .iter()
            .flat_map(|value| value.to_le_bytes())
            .collect()
    }

    /// The game's master: a worldspace with one cell and two objects, and
    /// a room with one.
    fn master() -> Vec<u8> {
        let world = RecordSpec::new(*b"WRLD", 0x3C)
            .with(*b"EDID", fixture::zstring("Tamriel"))
            .with(*b"FULL", 7u32.to_le_bytes().to_vec())
            .with(*b"CNAM", ids(&[0x0000_0AAA]))
            .with(*b"RNAM", {
                let mut data = vec![0, 0, 0, 0];
                data.extend(1u32.to_le_bytes());
                data.extend(ids(&[0x0000_0812]));
                data.extend([0, 0, 0, 0]);
                data
            })
            .with(*b"OFST", vec![1, 2, 3, 4]);
        let cell = RecordSpec::new(*b"CELL", 0x9001)
            .with(*b"XCLC", grid(0, 0))
            .with(*b"XCLR", ids(&[0x0000_0B01, 0x0000_0B02]));
        let room = RecordSpec::new(*b"CELL", 0x5555).with(*b"EDID", fixture::zstring("Room"));
        fixture::plugin_with_raw_groups(
            &[],
            0x81,
            &[
                fixture::interior_cells(&[(
                    room,
                    vec![fixture::reference(
                        0x0000_0820,
                        0x10,
                        [5.0, 5.0, 0.0],
                        [0.0; 3],
                        None,
                    )],
                )]),
                fixture::worldspace(
                    &world,
                    &[(
                        cell,
                        vec![
                            fixture::reference(
                                0x0000_0812,
                                0x10,
                                [100.0, 200.0, 0.0],
                                [0.0; 3],
                                None,
                            ),
                            fixture::reference(
                                0x0000_0813,
                                0x10,
                                [300.0, 200.0, 0.0],
                                [0.0; 3],
                                None,
                            ),
                        ],
                    )],
                ),
            ],
        )
    }

    /// A mod that changes the first object: its own version, linked to a
    /// keyword of the master and to an object of its own.
    fn town() -> Vec<u8> {
        let world = RecordSpec::new(*b"WRLD", 0x3C).with(*b"EDID", fixture::zstring("Tamriel"));
        let cell = RecordSpec::new(*b"CELL", 0x9001).with(*b"XCLC", grid(0, 0));
        let changed = fixture::reference(
            0x0000_0812,
            0x0100_0900,
            [110.0, 200.0, 0.0],
            [0.0; 3],
            None,
        )
        .with(*b"XLKR", ids(&[0x0000_0C00, 0x0100_0901]))
        .with(*b"XESP", {
            let mut data = ids(&[0x0100_0901]);
            data.extend([1, 0, 0, 0]);
            data
        });
        fixture::plugin_with_raw_groups(
            &[MASTER],
            0,
            &[fixture::worldspace(&world, &[(cell, vec![changed])])],
        )
    }

    fn setup(plugins: &[(&str, Vec<u8>)]) -> (tempfile::TempDir, Vec<(String, PathBuf)>) {
        let dir = tempfile::tempdir().unwrap();
        let list = plugins
            .iter()
            .map(|(name, bytes)| {
                let path = dir.path().join(name);
                std::fs::write(&path, bytes).unwrap();
                ((*name).to_owned(), path)
            })
            .collect();
        (dir, list)
    }

    fn movement(
        plugin: &str,
        local_id: u32,
        cell: Option<&str>,
        offset: [i32; 3],
        turn: i32,
    ) -> ObjectMove {
        ObjectMove {
            object: ObjectRef {
                plugin: plugin.to_owned(),
                local_id,
            },
            cell: cell.map(str::to_owned),
            from: [100, 200, 0],
            offset,
            turn,
            teleport: None,
        }
    }

    fn names(plugin: &str, id: u32) -> Option<String> {
        (plugin == MASTER && id == 7).then(|| "Skyrim".to_owned())
    }

    fn everything(bytes: &[u8]) -> plugin::Plugin {
        plugin::read_request(
            &mut Cursor::new(bytes),
            &plugin::Request {
                flat: &[],
                nested: &[*b"CELL", *b"WRLD"],
                nested_records: &[*b"REFR", *b"CELL", *b"WRLD"],
                nested_overrides: &[],
            },
        )
        .unwrap()
    }

    #[test]
    fn a_door_is_led_to_another_plugins_door_and_its_arrival_moves() {
        // The game's door in the room leads to its door outside (0x812);
        // a mod's door outside (0x800) is kept instead.
        let world = RecordSpec::new(*b"WRLD", 0x3C).with(*b"EDID", fixture::zstring("Tamriel"));
        let cell = RecordSpec::new(*b"CELL", 0x9001).with(*b"XCLC", grid(0, 0));
        let room = RecordSpec::new(*b"CELL", 0x5555).with(*b"EDID", fixture::zstring("Room"));
        let mut xtel = ids(&[0x0000_0812]);
        for value in [100.0f32, 150.0, 0.0, 0.0, 0.0, 1.0] {
            xtel.extend(value.to_le_bytes());
        }
        xtel.extend(0u32.to_le_bytes());
        let inside = fixture::reference(0x0000_0820, 0x10, [5.0, 5.0, 0.0], [0.0; 3], None)
            .with(*b"XTEL", xtel);
        let game = fixture::plugin_with_raw_groups(
            &[],
            0x81,
            &[
                fixture::interior_cells(&[(room, vec![inside])]),
                fixture::worldspace(
                    &world,
                    &[(
                        cell.clone(),
                        vec![fixture::reference(
                            0x0000_0812,
                            0x10,
                            [100.0, 200.0, 0.0],
                            [0.0; 3],
                            None,
                        )],
                    )],
                ),
            ],
        );
        let doors = fixture::plugin_with_raw_groups(
            &[MASTER],
            0,
            &[fixture::worldspace(
                &world,
                &[(
                    cell,
                    vec![fixture::reference(
                        0x0100_0800,
                        0x10,
                        [100.0, 206.0, 0.0],
                        [0.0; 3],
                        None,
                    )],
                )],
            )],
        );
        let (_dir, plugins) = setup(&[(MASTER, game), ("Doors.esp", doors)]);
        let mut led = movement(MASTER, 0x820, Some("Room"), [0, 0, 0], 0);
        led.from = [5, 5, 0];
        led.teleport = Some(DoorTarget {
            door: ObjectRef {
                plugin: "Doors.esp".to_owned(),
                local_id: 0x800,
            },
            arrival: Some([100, 156, 0]),
            facing: None,
        });
        assert!(!led.is_still());
        let built = build(Game::SkyrimSe, &plugins, &[led], &names).unwrap();
        assert_eq!(built.refused, []);
        assert_eq!(built.masters, [MASTER, "Doors.esp"]);
        let plugin = everything(&built.bytes.unwrap());
        let door = plugin
            .records
            .iter()
            .find(|record| &record.signature == b"REFR")
            .unwrap();
        let xtel = door.first(b"XTEL").unwrap();
        // The second master's record, where the player now arrives, and
        // the facing kept.
        assert_eq!(read_u32(xtel, 0), Some(0x0100_0800));
        let values = crate::records::floats(xtel.get(4..28).unwrap());
        assert_eq!(values, [100.0, 156.0, 0.0, 0.0, 0.0, 1.0]);
        // Nothing else moved.
        let data = crate::records::floats(door.first(b"DATA").unwrap());
        assert_eq!(&data[..3], &[5.0, 5.0, 0.0]);
    }

    #[test]
    fn the_winning_version_is_copied_moved_and_its_references_rewritten() {
        let (_dir, plugins) = setup(&[(MASTER, master()), ("Town.esp", town())]);
        let moves = [movement(MASTER, 0x812, None, [10, -20, 5], 90)];
        let built = build(Game::SkyrimSe, &plugins, &moves, &names).unwrap();
        assert_eq!(built.refused, []);
        assert_eq!(built.masters, [MASTER, "Town.esp"]);
        assert!(built.light);
        let plugin = everything(&built.bytes.unwrap());
        assert_eq!(plugin.damaged, 0);
        assert_eq!(plugin.header.masters, [MASTER, "Town.esp"]);
        assert!(!plugin.header.is_localized);

        let object = plugin
            .records
            .iter()
            .find(|record| &record.signature == b"REFR")
            .unwrap();
        assert_eq!(object.form_id, 0x0000_0812);
        // The mod's version, moved: 110 + 10, 200 − 20, 0 + 5, turned 90°.
        let data: Vec<f32> = crate::records::floats(object.first(b"DATA").unwrap());
        assert_eq!(&data[..3], &[120.0, 180.0, 5.0]);
        assert!((f64::from(data[5]) - std::f64::consts::FRAC_PI_2).abs() < 1e-6);
        // Its base and links pointed at the mod's own records: master 1 now.
        assert_eq!(object.first(b"NAME").unwrap(), ids(&[0x0100_0900]));
        assert_eq!(
            object.first(b"XLKR").unwrap(),
            ids(&[0x0000_0C00, 0x0100_0901])
        );

        // The cell and the worldspace are the versions that win, the mod's:
        // the cell without regions, the worldspace without a climate.
        let world = plugin
            .records
            .iter()
            .find(|record| &record.signature == b"WRLD")
            .unwrap();
        assert_eq!(world.first(b"EDID"), Some(b"Tamriel\0".as_slice()));
        assert_eq!(world.first(b"CNAM"), None);
        let cell = plugin
            .records
            .iter()
            .find(|record| &record.signature == b"CELL")
            .unwrap();
        assert_eq!(cell.first(b"XCLR"), None);
    }

    #[test]
    fn a_master_only_object_copies_the_masters_cell_and_worldspace() {
        let (_dir, plugins) = setup(&[(MASTER, master()), ("Town.esp", town())]);
        let moves = [movement(MASTER, 0x813, None, [0, 0, 10], 0)];
        let built = build(Game::SkyrimSe, &plugins, &moves, &names).unwrap();
        assert_eq!(built.refused, []);
        // The object and the cell's regions come from the master, but the
        // cell and worldspace that win are the mod's.
        assert_eq!(built.masters, [MASTER, "Town.esp"]);
        let plugin = everything(&built.bytes.unwrap());
        let object = plugin
            .records
            .iter()
            .find(|record| &record.signature == b"REFR")
            .unwrap();
        assert_eq!(object.form_id, 0x813);
        assert_eq!(
            crate::records::floats(object.first(b"DATA").unwrap())[2],
            10.0
        );
    }

    #[test]
    fn names_of_a_localized_plugin_become_text_and_the_offsets_go() {
        let (_dir, plugins) = setup(&[(MASTER, master())]);
        let moves = [movement(MASTER, 0x812, None, [1, 0, 0], 0)];
        let built = build(Game::SkyrimSe, &plugins, &moves, &names).unwrap();
        assert_eq!(built.refused, []);
        assert_eq!(built.masters, [MASTER]);
        let plugin = everything(&built.bytes.unwrap());
        let world = plugin
            .records
            .iter()
            .find(|record| &record.signature == b"WRLD")
            .unwrap();
        assert_eq!(world.first(b"FULL"), Some(b"Skyrim\0".as_slice()));
        assert_eq!(world.first(b"OFST"), None);
        assert_eq!(world.first(b"CNAM").unwrap(), ids(&[0x0000_0AAA]));
        assert_eq!(world.first(b"RNAM").unwrap()[8..12], ids(&[0x0000_0812]));

        // Without the string table, the move is refused.
        let built = build(Game::SkyrimSe, &plugins, &moves, &|_, _| None).unwrap();
        assert_eq!(built.bytes, None);
        assert_eq!(built.refused[0].reason, MoveRefusal::NameNotFound);
    }

    #[test]
    fn an_object_in_a_room_is_found_by_the_rooms_name() {
        let (_dir, plugins) = setup(&[(MASTER, master())]);
        let mut moves = [movement(MASTER, 0x820, Some("room"), [0, 3, 0], 0)];
        moves[0].from = [0, 0, 0];
        let built = build(Game::SkyrimSe, &plugins, &moves, &names).unwrap();
        assert_eq!(built.refused, []);
        let plugin = everything(&built.bytes.unwrap());
        let object = plugin
            .records
            .iter()
            .find(|record| &record.signature == b"REFR")
            .unwrap();
        assert_eq!(
            object.groups.first().map(|group| group.group_type),
            Some(group_type::INTERIOR_BLOCK)
        );
        assert_eq!(
            crate::records::floats(object.first(b"DATA").unwrap())[1],
            8.0
        );
    }

    #[test]
    fn unknown_fields_missing_objects_and_other_games_are_refused() {
        let world = RecordSpec::new(*b"WRLD", 0x3C);
        let cell = RecordSpec::new(*b"CELL", 0x9001).with(*b"XCLC", grid(0, 0));
        let odd = fixture::reference(0x812, 0x10, [100.0, 200.0, 0.0], [0.0; 3], None)
            .with(*b"XCVL", vec![0; 12]);
        let plugin = fixture::plugin_with_raw_groups(
            &[],
            0,
            &[fixture::worldspace(&world, &[(cell, vec![odd])])],
        );
        let (_dir, plugins) = setup(&[(MASTER, plugin)]);
        let moves = [
            movement(MASTER, 0x812, None, [1, 0, 0], 0),
            movement(MASTER, 0x999, None, [1, 0, 0], 0),
        ];
        let built = build(Game::SkyrimSe, &plugins, &moves, &names).unwrap();
        assert_eq!(built.bytes, None);
        assert_eq!(
            built
                .refused
                .iter()
                .map(|refused| refused.reason.clone())
                .collect::<Vec<_>>(),
            // In the order of the moves.
            [
                MoveRefusal::UnknownField {
                    field: "REFR XCVL".to_owned()
                },
                MoveRefusal::NotFound,
            ]
        );
        let built = build(Game::Fallout4, &plugins, &moves, &names).unwrap();
        assert!(
            built
                .refused
                .iter()
                .all(|refused| refused.reason == MoveRefusal::UnsupportedGame)
        );
    }

    #[test]
    fn scripts_name_their_objects_at_the_right_place() {
        // RecordVersion 5, object format 2, one script with an object property
        // and an array of two objects.
        let mut data = Vec::new();
        data.extend(5u16.to_le_bytes());
        data.extend(2u16.to_le_bytes());
        data.extend(1u16.to_le_bytes());
        data.extend(3u16.to_le_bytes());
        data.extend(b"Foo");
        data.push(0);
        data.extend(2u16.to_le_bytes());
        data.extend(1u16.to_le_bytes());
        data.extend(b"A");
        data.extend([1, 1]);
        let first = data.len();
        data.extend([0, 0, 0xFF, 0xFF]);
        data.extend(ids(&[0x0100_0001]));
        data.extend(1u16.to_le_bytes());
        data.extend(b"B");
        data.extend([11, 1]);
        data.extend(2u32.to_le_bytes());
        let second = data.len();
        data.extend([0; 4]);
        data.extend(ids(&[0x0100_0002]));
        data.extend([0; 4]);
        data.extend(ids(&[0x0100_0003]));
        assert_eq!(
            script_form_ids(&data),
            Some(vec![first + 4, second + 4, second + 12])
        );
        // Anything left over is not a placed object's scripts.
        data.push(0);
        assert_eq!(script_form_ids(&data), None);
    }

    #[test]
    fn turning_an_upright_object_adds_to_its_angle_about_z() {
        let [x, y, z] = turned([0.0, 0.0, 1.0], 0.5);
        assert_eq!((x, y), (0.0, 0.0));
        assert!((z - 1.5).abs() < 1e-6);
        // Past a full turn it wraps.
        let [_, _, z] = turned([0.0, 0.0, 6.0], 1.0);
        assert!((f64::from(z) - (7.0 - TAU)).abs() < 1e-5);
    }

    #[test]
    fn turning_a_tilted_object_turns_it_about_the_worlds_upright_axis() {
        let rotation = [0.3_f32, -0.2, 1.1];
        let turn = 0.7;
        let result = turned(rotation, turn);
        let expected = multiply(&about_z(-turn), &placement(rotation.map(f64::from)));
        let got = placement(result.map(f64::from));
        for (row, wanted) in got.iter().zip(expected.iter()) {
            for (value, other) in row.iter().zip(wanted.iter()) {
                assert!((value - other).abs() < 1e-5, "{got:?} != {expected:?}");
            }
        }
    }

    proptest! {
        #[test]
        fn reading_scripts_never_panics(data in proptest::collection::vec(any::<u8>(), 0..200)) {
            let _ = script_form_ids(&data);
        }

        #[test]
        fn field_layouts_never_panic(
            field in proptest::sample::select(vec![*b"RNAM", *b"PDTO", *b"XLKR", *b"XTEL", *b"XCLR", *b"VMAD"]),
            data in proptest::collection::vec(any::<u8>(), 0..64),
        ) {
            for record in [*b"REFR", *b"CELL", *b"WRLD"] {
                if let Some(layout) = layout(record, field)
                    && let Some(offsets) = form_id_offsets(&layout, &data)
                {
                    prop_assert!(offsets.iter().all(|offset| offset + 4 <= data.len()));
                }
            }
        }

        #[test]
        fn any_turn_keeps_the_placement_a_turn_about_z(
            x in -3.0f32..3.0, y in -1.5f32..1.5, z in -3.0f32..3.0, turn in -7.0f64..7.0,
        ) {
            let result = turned([x, y, z], turn);
            let expected = multiply(&about_z(-turn), &placement([x, y, z].map(f64::from)));
            let got = placement(result.map(f64::from));
            for (row, wanted) in got.iter().zip(expected.iter()) {
                for (value, other) in row.iter().zip(wanted.iter()) {
                    prop_assert!((value - other).abs() < 1e-4);
                }
            }
        }
    }
}
