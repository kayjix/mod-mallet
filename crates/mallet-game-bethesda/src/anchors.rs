//! What ties placed objects to the game (ADR 0046): which other records
//! point at them, and how, in the versions the load order gives them.
//!
//! Who points at an object can be anywhere in the load order, so the first
//! question reads every placed object, quest, location and AI package of
//! every plugin once and keeps only the pointers between them; later
//! questions are answered from that. Records are looked at one by one as
//! they are read, so even the main game file is never held in memory at
//! once. Nothing is written.

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use mallet_core::anchors::{Anchor, AnchorKind, AnchorOther, OtherState};
use mallet_core::facts::RecordId;
use mallet_formats_bethesda::plugin::{self, PluginHeader, Record, Signature};

use crate::game::Game;
use crate::ingest;
use crate::records::{
    FLAG_DELETED, FLAG_INITIALLY_DISABLED, FLAG_PERSISTENT, FLAG_VISIBLE_WHEN_DISTANT, editor_id,
    floats, spelt_name,
};

/// The kinds of placed objects: things, characters, creatures and
/// projectiles and hazards placed by hand.
const PLACED: [Signature; 11] = [
    *b"REFR", *b"ACHR", *b"ACRE", *b"PGRE", *b"PMIS", *b"PHZD", *b"PARW", *b"PBAR", *b"PBEA",
    *b"PCON", *b"PFLA",
];

/// The records whose name is kept, to name what is placed: base objects,
/// keywords and the roles objects have in locations.
const NAMED: [Signature; 33] = [
    *b"STAT", *b"MSTT", *b"SCOL", *b"TREE", *b"FURN", *b"CONT", *b"DOOR", *b"ACTI", *b"FLOR",
    *b"LIGH", *b"MISC", *b"WEAP", *b"ARMO", *b"BOOK", *b"ALCH", *b"INGR", *b"KEYM", *b"SLGM",
    *b"AMMO", *b"NPC_", *b"CREA", *b"LVLN", *b"LVLC", *b"IDLM", *b"TACT", *b"SOUN", *b"HAZD",
    *b"ADDN", *b"TXST", *b"BNDS", *b"KYWD", *b"LCRT", *b"PROJ",
];

/// The records outside the worldspaces and cells that point at placed
/// objects.
const HOLDERS: [Signature; 3] = [*b"QUST", *b"LCTN", *b"PACK"];

/// The form ID of the player's own placed object in the main game file.
const PLAYER: u32 = 0x14;

/// Objects moved this far below the ground were switched off for good by
/// a cleaning tool.
const BURIED: f32 = -29_000.0;

/// A record, by the plugin that defined it first (an index into the
/// reader's plugin names) and its number there.
type Key = u64;

fn key_of(plugin: u16, local_id: u32) -> Key {
    (u64::from(plugin) << 32) | u64::from(local_id)
}

#[allow(clippy::cast_possible_truncation)]
fn split(key: Key) -> (u16, u32) {
    ((key >> 32) as u16, key as u32)
}

/// A pointer from one record to another.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Out {
    kind: AnchorKind,
    target: Key,
    /// The keyword of a linked reference or the role in a location.
    extra: Option<Key>,
}

/// A placed object, in the version the game uses, as far as ties go.
#[derive(Debug, Clone, Default)]
struct Placed {
    signature: Signature,
    flags: u32,
    base: Option<Key>,
    editor_id: Option<Box<str>>,
    /// Moved far below the ground.
    buried: bool,
    /// Its enable parent, and whether it shows while the parent does not.
    parent: Option<(Key, bool)>,
    /// Its own roles in locations.
    roles: Vec<Key>,
    out: Vec<Out>,
}

/// A quest, location or AI package that points at placed objects.
#[derive(Debug, Clone)]
struct Holder {
    signature: Signature,
    editor_id: Option<Box<str>>,
    deleted: bool,
    out: Vec<Out>,
}

/// A pointer at an object, seen from the object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Pointer {
    kind: AnchorKind,
    from: Key,
    extra: Option<Key>,
}

/// The pointers between the records of a load order.
#[derive(Debug, Default)]
struct Index {
    /// The plugin names keys refer to, in lower case.
    plugins: Vec<String>,
    plugin_keys: HashMap<String, u16>,
    /// The placed objects that point somewhere or that the game keeps
    /// loaded or shows from afar.
    placed: HashMap<Key, Placed>,
    holders: HashMap<Key, Holder>,
    /// Names of base objects, keywords and roles.
    names: HashMap<Key, Box<str>>,
    /// Worldspaces by internal name, in lower case.
    worlds: HashMap<String, Key>,
    /// The large objects of each worldspace.
    large: HashMap<Key, HashSet<Key>>,
    /// Who points at each object.
    pointers: HashMap<Key, Vec<Pointer>>,
}

impl Index {
    fn plugin_key(&mut self, name: &str) -> u16 {
        let lower = name.to_lowercase();
        if let Some(&key) = self.plugin_keys.get(&lower) {
            return key;
        }
        let key = u16::try_from(self.plugins.len()).unwrap_or(u16::MAX);
        self.plugins.push(lower.clone());
        self.plugin_keys.insert(lower, key);
        key
    }

    fn find(&self, id: &RecordId) -> Option<Key> {
        let plugin = self.plugin_keys.get(&id.plugin.to_lowercase())?;
        Some(key_of(*plugin, id.local_id))
    }

    /// Reads one plugin's records into the index, replacing earlier
    /// versions.
    fn add(&mut self, name: &str, path: &Path) {
        let Ok(file) = File::open(path) else {
            return;
        };
        let mut flat: Vec<Signature> = NAMED.to_vec();
        flat.extend(HOLDERS);
        let mut nested_records: Vec<Signature> = PLACED.to_vec();
        nested_records.push(*b"WRLD");
        let request = plugin::Request {
            flat: &flat,
            nested: &[*b"CELL", *b"WRLD"],
            nested_records: &nested_records,
            nested_overrides: &[],
        };
        let mut table: Option<Vec<u16>> = None;
        let _ = plugin::read_request_each(
            &mut BufReader::new(file),
            &request,
            &|_| true,
            &mut |header, record| {
                if table.is_none() {
                    table = Some(self.key_table(header, name));
                }
                if let Some(keys) = &table {
                    self.record(&Keys(keys), header, &record);
                }
            },
        );
    }

    /// The plugin keys a plugin's form IDs refer to: its masters, then
    /// itself.
    fn key_table(&mut self, header: &PluginHeader, name: &str) -> Vec<u16> {
        let mut keys: Vec<u16> = header
            .masters
            .iter()
            .map(|master| self.plugin_key(master))
            .collect();
        keys.push(self.plugin_key(name));
        keys
    }

    fn record(&mut self, keys: &Keys<'_>, header: &PluginHeader, record: &Record) {
        let Some(key) = keys.key(record.form_id) else {
            return;
        };
        let signature = &record.signature;
        if PLACED.contains(signature) {
            self.add_placed(keys, key, record);
        } else if HOLDERS.contains(signature) {
            self.add_holder(keys, key, record);
        } else if signature == b"WRLD" {
            self.add_world(keys, key, record);
        } else if let Some(name) = name_of(record, header.is_localized) {
            self.names.insert(key, name.into_boxed_str());
        }
    }

    fn add_placed(&mut self, keys: &Keys<'_>, key: Key, record: &Record) {
        if record.flags & FLAG_DELETED != 0 {
            // A deleted version keeps what the last one pointed at, so that
            // the pointers can be shown as gone.
            if let Some(placed) = self.placed.get_mut(&key) {
                placed.flags |= FLAG_DELETED;
            }
            return;
        }
        let placed = placed_of(keys, record);
        let kept = placed.flags & (FLAG_PERSISTENT | FLAG_VISIBLE_WHEN_DISTANT) != 0
            || !placed.out.is_empty()
            || !placed.roles.is_empty();
        if kept {
            self.placed.insert(key, placed);
        } else {
            self.placed.remove(&key);
        }
    }

    fn add_holder(&mut self, keys: &Keys<'_>, key: Key, record: &Record) {
        if record.flags & FLAG_DELETED != 0 {
            if let Some(holder) = self.holders.get_mut(&key) {
                holder.deleted = true;
            }
            return;
        }
        let out = holder_pointers(keys, record);
        if out.is_empty() {
            self.holders.remove(&key);
        } else {
            self.holders.insert(
                key,
                Holder {
                    signature: record.signature,
                    editor_id: editor_id(record).map(String::into_boxed_str),
                    deleted: false,
                    out,
                },
            );
        }
    }

    fn add_world(&mut self, keys: &Keys<'_>, key: Key, record: &Record) {
        if let Some(name) = editor_id(record) {
            self.worlds.entry(name.to_lowercase()).or_insert(key);
        }
        // Each plugin lists the large objects it places or moves; a version
        // of the worldspace without the list takes none away.
        let large = record
            .subrecords
            .iter()
            .filter(|field| &field.signature == b"RNAM")
            .flat_map(|field| large_references(&field.data))
            .filter_map(|form_id| keys.key(form_id));
        self.large.entry(key).or_default().extend(large);
    }

    /// Turns the pointers kept with each record round, to the records
    /// pointed at.
    fn finish(&mut self) {
        let mut pointers: HashMap<Key, Vec<Pointer>> = HashMap::new();
        let from_placed = self.placed.iter().map(|(key, placed)| (key, &placed.out));
        let from_holders = self.holders.iter().map(|(key, holder)| (key, &holder.out));
        for (from, out) in from_placed.chain(from_holders) {
            for pointer in out {
                pointers.entry(pointer.target).or_default().push(Pointer {
                    kind: pointer.kind,
                    from: *from,
                    extra: pointer.extra,
                });
            }
        }
        for list in pointers.values_mut() {
            list.sort_by_key(|pointer| (pointer.kind, pointer.from, pointer.extra));
            list.dedup();
        }
        self.pointers = pointers;
    }

    /// The ties of the object `key`, in the worldspace `world` if it is
    /// outside.
    fn anchors(&self, key: Key, world: Option<Key>) -> Vec<Anchor> {
        let mut found: Vec<Anchor> = Vec::new();
        let own = self.placed.get(&key);
        let mut seen: HashSet<(AnchorKind, Key)> = HashSet::new();
        let mut roles_listed: HashSet<Key> = HashSet::new();
        for pointer in self.pointers.get(&key).into_iter().flatten() {
            if !seen.insert((pointer.kind, pointer.from))
                && pointer.kind != AnchorKind::LocationRole
            {
                continue;
            }
            if pointer.kind == AnchorKind::LocationRole
                && let Some(role) = pointer.extra
            {
                roles_listed.insert(role);
            }
            found.push(Anchor {
                kind: pointer.kind,
                other: self.other(pointer.from),
                keyword: pointer.extra.and_then(|extra| self.name(extra)),
            });
        }
        if let Some(own) = own {
            // A door's partner is found from either side.
            for out in &own.out {
                if out.kind == AnchorKind::TeleportDoor
                    && seen.insert((AnchorKind::TeleportDoor, out.target))
                {
                    found.push(Anchor {
                        kind: AnchorKind::TeleportDoor,
                        other: self.other(out.target),
                        keyword: None,
                    });
                }
            }
            for role in own.roles.iter().filter(|role| !roles_listed.contains(role)) {
                found.push(Anchor {
                    kind: AnchorKind::LocationRole,
                    other: None,
                    keyword: self.name(*role),
                });
            }
        }
        if world
            .and_then(|world| self.large.get(&world))
            .is_some_and(|large| large.contains(&key))
        {
            found.push(Anchor {
                kind: AnchorKind::LargeReference,
                other: None,
                keyword: None,
            });
        }
        let flags = own.map_or(0, |own| own.flags);
        for (flag, kind) in [
            (FLAG_VISIBLE_WHEN_DISTANT, AnchorKind::VisibleWhenDistant),
            (FLAG_PERSISTENT, AnchorKind::Persistent),
        ] {
            if flags & flag != 0 {
                found.push(Anchor {
                    kind,
                    other: None,
                    keyword: None,
                });
            }
        }
        // The most binding first; within a kind, what is there before what
        // is gone.
        found.sort_by_key(|anchor| {
            (
                std::cmp::Reverse(anchor.kind.verdict()),
                anchor.kind,
                anchor
                    .other
                    .as_ref()
                    .map(|other| other.state == OtherState::Gone),
            )
        });
        found
    }

    fn name(&self, key: Key) -> Option<String> {
        self.names.get(&key).map(ToString::to_string)
    }

    /// The other side of a tie.
    fn other(&self, key: Key) -> Option<AnchorOther> {
        let (plugin, local_id) = split(key);
        let plugin = self.plugins.get(usize::from(plugin))?.clone();
        let (record_type, name, state) = if let Some(placed) = self.placed.get(&key) {
            let name = placed
                .editor_id
                .as_deref()
                .map(str::to_owned)
                .or_else(|| placed.base.and_then(|base| self.name(base)));
            (placed.signature, name, state_of(placed))
        } else if let Some(holder) = self.holders.get(&key) {
            let state = if holder.deleted {
                OtherState::Gone
            } else {
                OtherState::Present
            };
            (
                holder.signature,
                holder.editor_id.as_deref().map(str::to_owned),
                state,
            )
        } else {
            // Only a door's partner can be pointed at without being kept:
            // a door that is not there.
            (*b"REFR", None, OtherState::Gone)
        };
        Some(AnchorOther {
            plugin,
            local_id,
            record_type: String::from_utf8_lossy(&record_type).into_owned(),
            name,
            state,
        })
    }
}

/// Whether a placed object is still in the game. Switched off for good
/// are objects buried by a cleaning tool, objects shown only while the
/// player is not, and objects nothing can switch on: without an enable
/// parent and not persistent, so no script can reach them.
fn state_of(placed: &Placed) -> OtherState {
    if placed.flags & FLAG_DELETED != 0 {
        return OtherState::Gone;
    }
    if placed.flags & FLAG_INITIALLY_DISABLED == 0 {
        return OtherState::Present;
    }
    // The main game file is read first and so has plugin key 0.
    let player = key_of(0, PLAYER);
    let off_for_good = placed.buried
        || placed
            .parent
            .is_some_and(|(parent, opposite)| opposite && parent == player)
        || (placed.parent.is_none() && placed.flags & FLAG_PERSISTENT == 0);
    if off_for_good {
        OtherState::Gone
    } else {
        OtherState::Off
    }
}

/// Turns a plugin's form IDs into keys.
struct Keys<'a>(&'a [u16]);

impl Keys<'_> {
    /// The key of `form_id`; `None` for the null form ID.
    fn key(&self, form_id: u32) -> Option<Key> {
        if form_id == 0 {
            return None;
        }
        let index = usize::try_from(form_id >> 24).unwrap_or(usize::MAX);
        let plugin = self.0.get(index).or_else(|| self.0.last())?;
        Some(key_of(*plugin, form_id & 0x00FF_FFFF))
    }
}

/// The form ID at the start of `data`.
fn form_id_at(data: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        data.get(offset..offset + 4)?.try_into().ok()?,
    ))
}

/// The name of a base object, keyword or role: what players see, or else
/// the internal name.
fn name_of(record: &Record, localized: bool) -> Option<String> {
    spelt_name(record, localized).or_else(|| editor_id(record))
}

/// What a placed object points at, and what the game makes of it.
fn placed_of(keys: &Keys<'_>, record: &Record) -> Placed {
    let mut placed = Placed {
        signature: record.signature,
        flags: record.flags,
        base: record
            .first(b"NAME")
            .and_then(|data| form_id_at(data, 0))
            .and_then(|id| keys.key(id)),
        editor_id: editor_id(record).map(String::into_boxed_str),
        buried: record
            .first(b"DATA")
            .and_then(|data| data.get(8..12))
            .and_then(|z| floats(z).first().copied())
            .is_some_and(|z| z < BURIED),
        ..Placed::default()
    };
    for field in &record.subrecords {
        let data = field.data.as_slice();
        match &field.signature {
            b"XESP" => {
                if let Some(parent) = form_id_at(data, 0).and_then(|id| keys.key(id)) {
                    let opposite = data.get(4).is_some_and(|flags| flags & 1 != 0);
                    placed.parent = Some((parent, opposite));
                    placed.out.push(Out {
                        kind: AnchorKind::EnableParent,
                        target: parent,
                        extra: None,
                    });
                }
            }
            b"XLKR" => {
                // Since Skyrim a keyword, then the object; before, the
                // object alone.
                let (keyword, target) = if data.len() >= 8 {
                    (form_id_at(data, 0), form_id_at(data, 4))
                } else {
                    (None, form_id_at(data, 0))
                };
                if let Some(target) = target.and_then(|id| keys.key(id)) {
                    placed.out.push(Out {
                        kind: AnchorKind::LinkedRef,
                        target,
                        extra: keyword.and_then(|id| keys.key(id)),
                    });
                }
            }
            b"XAPR" => {
                // The objects whose use activates this one, each with a
                // delay.
                for entry in data.as_chunks::<8>().0 {
                    if let Some(parent) = form_id_at(entry, 0).and_then(|id| keys.key(id)) {
                        placed.out.push(Out {
                            kind: AnchorKind::ActivateParent,
                            target: parent,
                            extra: None,
                        });
                    }
                }
            }
            b"XTEL" => {
                if let Some(door) = form_id_at(data, 0).and_then(|id| keys.key(id)) {
                    placed.out.push(Out {
                        kind: AnchorKind::TeleportDoor,
                        target: door,
                        extra: None,
                    });
                }
            }
            b"XLRT" => {
                placed.roles.extend(
                    data.as_chunks::<4>()
                        .0
                        .iter()
                        .filter_map(|entry| form_id_at(entry, 0))
                        .filter_map(|id| keys.key(id)),
                );
            }
            b"VMAD" => {
                placed.out.extend(
                    script_objects(data, false)
                        .into_iter()
                        .filter_map(|id| keys.key(id))
                        .map(|target| Out {
                            kind: AnchorKind::Script,
                            target,
                            extra: None,
                        }),
                );
            }
            _ => {}
        }
    }
    placed
}

/// What a quest, location or AI package points at.
fn holder_pointers(keys: &Keys<'_>, record: &Record) -> Vec<Out> {
    let mut out = Vec::new();
    let mut push = |kind, target: Option<u32>, extra: Option<u32>| {
        if let Some(target) = target.and_then(|id| keys.key(id)) {
            out.push(Out {
                kind,
                target,
                extra: extra.and_then(|id| keys.key(id)),
            });
        }
    };
    for field in &record.subrecords {
        let data = field.data.as_slice();
        match (&record.signature, &field.signature) {
            (b"QUST", b"ALFR") => push(AnchorKind::QuestAlias, form_id_at(data, 0), None),
            (b"QUST", b"VMAD") => {
                for id in script_objects(data, true) {
                    push(AnchorKind::Script, Some(id), None);
                }
            }
            // A location's special objects: the role, the object, its
            // cell or worldspace and the cell's place.
            (b"LCTN", b"LCSR" | b"ACSR") => {
                for entry in data.as_chunks::<16>().0 {
                    push(
                        AnchorKind::LocationRole,
                        form_id_at(entry, 4),
                        form_id_at(entry, 0),
                    );
                }
            }
            // A package's places and targets: a type, then, for type 0, a
            // placed object.
            (b"PACK", b"PLDT" | b"PLD2" | b"PTDA") if form_id_at(data, 0) == Some(0) => {
                push(AnchorKind::Package, form_id_at(data, 4), None);
            }
            _ => {}
        }
    }
    out
}

/// The large objects listed in one `RNAM` field of a worldspace: a cell's
/// place, a count, then for each object its form ID and its cell's place.
fn large_references(data: &[u8]) -> Vec<u32> {
    let count = form_id_at(data, 4).map_or(0, |count| usize::try_from(count).unwrap_or(0));
    data.get(8..)
        .unwrap_or_default()
        .as_chunks::<8>()
        .0
        .iter()
        .take(count)
        .filter_map(|entry| form_id_at(entry, 0))
        .collect()
}

/// The objects the scripts of a record refer to through their
/// properties (`VMAD`). With `quest`, also those of the scripts of the
/// quest's aliases. Stops where the data cannot be read and keeps what was
/// found before.
fn script_objects(data: &[u8], quest: bool) -> Vec<u32> {
    let mut reader = Cursor { data, at: 0 };
    let mut found = Vec::new();
    let _ = read_scripts_of(&mut reader, quest, &mut found);
    found
}

/// Reads a `VMAD` field; `None` where it cannot be read on.
fn read_scripts_of(reader: &mut Cursor<'_>, quest: bool, found: &mut Vec<u32>) -> Option<()> {
    let version = reader.u16()?;
    let format = reader.u16()?;
    read_scripts(reader, version, format, found)?;
    // Quest fragments and alias scripts are laid out like this up to
    // Skyrim; later versions are left there.
    if !quest || version > 5 || reader.at >= reader.data.len() {
        return Some(());
    }
    reader.skip(1)?;
    let fragments = reader.u16()?;
    reader.string()?;
    for _ in 0..fragments {
        reader.skip(2 + 2 + 4 + 1)?;
        reader.string()?;
        reader.string()?;
    }
    let aliases = reader.u16()?;
    for _ in 0..aliases {
        reader.skip(8)?;
        let version = reader.u16()?;
        let format = reader.u16()?;
        read_scripts(reader, version, format, found)?;
    }
    Some(())
}

/// Reads a list of scripts with their properties.
fn read_scripts(
    reader: &mut Cursor<'_>,
    version: u16,
    format: u16,
    found: &mut Vec<u32>,
) -> Option<()> {
    let scripts = reader.u16()?;
    for _ in 0..scripts {
        reader.string()?;
        if version >= 4 {
            reader.skip(1)?;
        }
        let properties = reader.u16()?;
        for _ in 0..properties {
            reader.string()?;
            let kind = reader.u8()?;
            if version >= 4 {
                reader.skip(1)?;
            }
            read_value(reader, kind, format, found)?;
        }
    }
    Some(())
}

/// Reads one property's value, keeping the objects it names.
fn read_value(reader: &mut Cursor<'_>, kind: u8, format: u16, found: &mut Vec<u32>) -> Option<()> {
    match kind {
        0 => {}
        1 => read_object(reader, format, found)?,
        2 => reader.string()?,
        3 | 4 => reader.skip(4)?,
        5 => reader.skip(1)?,
        11 => {
            let count = reader.u32()?;
            for _ in 0..count {
                read_object(reader, format, found)?;
            }
        }
        12 => {
            let count = reader.u32()?;
            for _ in 0..count {
                reader.string()?;
            }
        }
        13 | 14 => {
            let count = reader.u32()?;
            reader.skip(usize::try_from(count).ok()?.checked_mul(4)?)?;
        }
        15 => {
            let count = reader.u32()?;
            reader.skip(usize::try_from(count).ok()?)?;
        }
        // Later games' structures and variables are not read.
        _ => return None,
    }
    Some(())
}

/// Reads an object value: a form ID and an alias, in the order `format`
/// gives. Keeps the form ID where it names an object, not an alias.
fn read_object(reader: &mut Cursor<'_>, format: u16, found: &mut Vec<u32>) -> Option<()> {
    let (form_id, alias) = if format == 1 {
        let form_id = reader.u32()?;
        let alias = reader.u16()?;
        reader.skip(2)?;
        (form_id, alias)
    } else {
        reader.skip(2)?;
        let alias = reader.u16()?;
        (reader.u32()?, alias)
    };
    if alias == u16::MAX {
        found.push(form_id);
    }
    Some(())
}

/// Reads little-endian values from a field.
struct Cursor<'a> {
    data: &'a [u8],
    at: usize,
}

impl Cursor<'_> {
    fn take(&mut self, count: usize) -> Option<&[u8]> {
        let end = self.at.checked_add(count)?;
        let taken = self.data.get(self.at..end)?;
        self.at = end;
        Some(taken)
    }

    fn skip(&mut self, count: usize) -> Option<()> {
        self.take(count).map(|_| ())
    }

    fn u8(&mut self) -> Option<u8> {
        self.take(1)?.first().copied()
    }

    fn u16(&mut self) -> Option<u16> {
        Some(u16::from_le_bytes(self.take(2)?.try_into().ok()?))
    }

    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }

    /// Skips a text with a two-byte length.
    fn string(&mut self) -> Option<()> {
        let len = self.u16()?;
        self.skip(usize::from(len))
    }
}

/// The plugins of a setup, for finding out what ties placed objects to
/// the game.
#[derive(Debug, Default)]
pub struct AnchorReader {
    /// The active plugins in load order: name and file.
    plugins: Vec<(String, PathBuf)>,
    /// What points at what, once read.
    index: Option<Index>,
}

impl AnchorReader {
    /// The active plugins of `game` in `game_dir`; `local_dir` holds
    /// `plugins.txt`, as for [`ingest::read_setup`].
    pub fn open(game: Game, game_dir: &Path, local_dir: Option<&Path>) -> Self {
        let plugins = ingest::active_plugin_files(game, game_dir, local_dir);
        Self {
            plugins,
            index: None,
        }
    }

    /// The plugins `plugins`, in load order, by name and file.
    pub fn with_plugins(plugins: Vec<(String, PathBuf)>) -> Self {
        Self {
            plugins,
            index: None,
        }
    }

    /// Reads every plugin, unless that has been done already. The first
    /// call takes a few seconds on a large setup.
    pub fn prepare(&mut self) {
        if self.index.is_some() {
            return;
        }
        let mut index = Index::default();
        for (name, path) in &self.plugins {
            index.add(name, path);
        }
        index.finish();
        self.index = Some(index);
    }

    /// What ties each of `objects` to the game, in the same order. `place`
    /// is the worldspace the objects are in, by internal name, or an
    /// interior cell, whose objects are never large ones.
    pub fn anchors(&mut self, place: &str, objects: &[RecordId]) -> Vec<Vec<Anchor>> {
        self.prepare();
        let Some(index) = self.index.as_ref() else {
            return Vec::new();
        };
        let world = index.worlds.get(&place.to_lowercase()).copied();
        objects
            .iter()
            .map(|object| {
                index
                    .find(object)
                    .map_or_default(|key| index.anchors(key, world))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use mallet_core::anchors::AnchorVerdict;
    use mallet_formats_bethesda::fixture::{self, RecordSpec};

    use super::*;

    const MAIN: &str = "Main.esm";
    const MOD: &str = "Mod.esp";

    fn id(plugin: &str, local_id: u32) -> RecordId {
        RecordId {
            plugin: plugin.to_lowercase(),
            local_id,
        }
    }

    fn placed(form_id: u32, base: u32, z: f32) -> RecordSpec {
        fixture::reference(form_id, base, [0.0, 0.0, z], [0.0; 3], None)
    }

    fn with_flags(mut spec: RecordSpec, flags: u32) -> RecordSpec {
        spec.flags |= flags;
        spec
    }

    fn le(values: &[u32]) -> Vec<u8> {
        values
            .iter()
            .flat_map(|value| value.to_le_bytes())
            .collect()
    }

    fn world() -> RecordSpec {
        // One large object, 0x900, in the cell at (0, 0).
        let mut rnam = vec![0, 0, 0, 0];
        rnam.extend(1u32.to_le_bytes());
        rnam.extend(0x900u32.to_le_bytes());
        rnam.extend([0, 0, 0, 0]);
        RecordSpec::new(*b"WRLD", 0x3C)
            .with(*b"EDID", fixture::zstring("Tamriel"))
            .with(*b"RNAM", rnam)
    }

    fn write(dir: &Path, name: &str, bytes: &[u8]) -> (String, PathBuf) {
        let path = dir.join(name);
        std::fs::write(&path, bytes).unwrap();
        (name.to_owned(), path)
    }

    /// A main file with a gate, the stone it stands on, a lever and two
    /// doors; a mod that deletes the gate and places a well.
    fn setup(dir: &Path) -> AnchorReader {
        let stone = with_flags(placed(0x800, 0x10, 0.0), FLAG_PERSISTENT);
        let gate = placed(0x801, 0x11, 0.0)
            .with(*b"XLKR", le(&[0x20, 0x800]))
            .with(*b"XESP", le(&[0x802, 0]))
            .with(*b"XAPR", le(&[0x802, 0]));
        let lever = with_flags(placed(0x802, 0x12, 0.0), FLAG_PERSISTENT);
        let door_a = with_flags(placed(0x803, 0x13, 0.0), FLAG_PERSISTENT)
            .with(*b"XTEL", le(&[0x804, 0, 0, 0, 0, 0, 0, 0]));
        let door_b = with_flags(placed(0x804, 0x13, 0.0), FLAG_PERSISTENT)
            .with(*b"XTEL", le(&[0x803, 0, 0, 0, 0, 0, 0, 0]));
        let tower = with_flags(placed(0x900, 0x10, 0.0), FLAG_VISIBLE_WHEN_DISTANT);
        let cell = RecordSpec::new(*b"CELL", 0x700);
        let main = fixture::plugin_with_raw_groups(
            &[],
            1,
            &[
                fixture::group(
                    *b"STAT",
                    0,
                    &fixture::record(
                        &RecordSpec::new(*b"STAT", 0x11).with(*b"EDID", fixture::zstring("Gate")),
                    ),
                ),
                fixture::group(
                    *b"KYWD",
                    0,
                    &fixture::record(
                        &RecordSpec::new(*b"KYWD", 0x20)
                            .with(*b"EDID", fixture::zstring("GateBase")),
                    ),
                ),
                fixture::group(
                    *b"QUST",
                    0,
                    &fixture::record(
                        &RecordSpec::new(*b"QUST", 0x30)
                            .with(*b"EDID", fixture::zstring("GateQuest"))
                            .with(*b"ALFR", le(&[0x802])),
                    ),
                ),
                fixture::worldspace(
                    &world(),
                    &[(cell, vec![stone, gate, lever, door_a, door_b, tower])],
                ),
            ],
        );
        let deleted_gate = with_flags(RecordSpec::new(*b"REFR", 0x801), FLAG_DELETED);
        let well = placed(0x0100_0800, 0x10, 0.0);
        let plugin = fixture::plugin_with_raw_groups(
            &[MAIN],
            0,
            &[fixture::worldspace(
                // A version of the worldspace without large objects keeps
                // those of the main file.
                &RecordSpec::new(*b"WRLD", 0x3C).with(*b"EDID", fixture::zstring("Tamriel")),
                &[(RecordSpec::new(*b"CELL", 0x700), vec![deleted_gate, well])],
            )],
        );
        AnchorReader::with_plugins(vec![write(dir, MAIN, &main), write(dir, MOD, &plugin)])
    }

    fn kinds(anchors: &[Anchor]) -> Vec<(AnchorKind, Option<OtherState>)> {
        anchors
            .iter()
            .map(|anchor| (anchor.kind, anchor.other.as_ref().map(|other| other.state)))
            .collect()
    }

    #[test]
    fn what_points_at_objects_is_found_in_the_winning_versions() {
        let dir = tempfile::tempdir().unwrap();
        let mut reader = setup(dir.path());
        let found = reader.anchors(
            "tamriel",
            &[
                id(MAIN, 0x800),
                id(MAIN, 0x802),
                id(MAIN, 0x803),
                id(MAIN, 0x900),
                id(MOD, 0x800),
            ],
        );
        let [stone, lever, door, tower, well] = found.as_slice() else {
            panic!("one answer per object: {found:?}");
        };

        // The gate that linked to the stone is gone: the stone may go.
        assert_eq!(
            kinds(stone),
            vec![
                (AnchorKind::LinkedRef, Some(OtherState::Gone)),
                (AnchorKind::Persistent, None),
            ]
        );
        let linked = stone[0].other.as_ref().unwrap();
        assert_eq!(linked.name.as_deref(), Some("Gate"));
        assert_eq!(
            (linked.plugin.as_str(), linked.local_id),
            ("main.esm", 0x801)
        );
        assert_eq!(stone[0].keyword.as_deref(), Some("GateBase"));
        assert_eq!(AnchorVerdict::of(stone), AnchorVerdict::Careful);

        // The lever: a quest names it; it switched and activated the gate.
        assert_eq!(
            kinds(lever),
            vec![
                (AnchorKind::QuestAlias, Some(OtherState::Present)),
                (AnchorKind::ActivateParent, Some(OtherState::Gone)),
                (AnchorKind::EnableParent, Some(OtherState::Gone)),
                (AnchorKind::Persistent, None),
            ]
        );
        assert_eq!(
            lever[0].other.as_ref().unwrap().name.as_deref(),
            Some("GateQuest")
        );
        assert_eq!(AnchorVerdict::of(lever), AnchorVerdict::Locked);

        // A door is found from either side, once.
        assert_eq!(
            kinds(door),
            vec![
                (AnchorKind::TeleportDoor, Some(OtherState::Present)),
                (AnchorKind::Persistent, None),
            ]
        );

        assert_eq!(
            kinds(tower),
            vec![
                (AnchorKind::LargeReference, None),
                (AnchorKind::VisibleWhenDistant, None),
            ]
        );
        assert_eq!(well.as_slice(), []);
        assert_eq!(AnchorVerdict::of(well), AnchorVerdict::Free);
    }

    #[test]
    fn objects_switched_off_for_good_are_gone() {
        let state = |flags, parent, buried| {
            state_of(&Placed {
                flags,
                buried,
                parent,
                ..Placed::default()
            })
        };
        assert_eq!(state(0, None, false), OtherState::Present);
        assert_eq!(state(FLAG_DELETED, None, false), OtherState::Gone);
        let disabled = FLAG_INITIALLY_DISABLED;
        assert_eq!(state(disabled, None, false), OtherState::Gone);
        assert_eq!(state(disabled, None, true), OtherState::Gone);
        assert_eq!(
            state(disabled | FLAG_PERSISTENT, None, false),
            OtherState::Off
        );
        let parent = key_of(0, 0x900);
        assert_eq!(
            state(disabled, Some((parent, false)), false),
            OtherState::Off
        );
        let player = key_of(0, PLAYER);
        assert_eq!(
            state(disabled, Some((player, true)), false),
            OtherState::Gone
        );
    }

    /// A `VMAD` field of version 5, object format 2, with one script whose
    /// properties name an object, an alias, and an array of two objects.
    fn vmad(quest_tail: &[u8]) -> Vec<u8> {
        let mut data = le16(&[5, 2, 1]);
        data.extend(text("GateScript"));
        data.push(0);
        data.extend(le16(&[3]));
        data.extend(text("Lever"));
        data.extend([1, 1]);
        data.extend(object(0x802, u16::MAX));
        data.extend(text("Alias"));
        data.extend([1, 1]);
        data.extend(object(0x30, 2));
        data.extend(text("Parts"));
        data.extend([11, 1]);
        data.extend(2u32.to_le_bytes());
        data.extend(object(0x803, u16::MAX));
        data.extend(object(0x804, u16::MAX));
        data.extend(quest_tail);
        data
    }

    fn le16(values: &[u16]) -> Vec<u8> {
        values
            .iter()
            .flat_map(|value| value.to_le_bytes())
            .collect()
    }

    fn text(text: &str) -> Vec<u8> {
        let mut data = le16(&[u16::try_from(text.len()).unwrap()]);
        data.extend(text.as_bytes());
        data
    }

    fn object(form_id: u32, alias: u16) -> Vec<u8> {
        let mut data = le16(&[0, alias]);
        data.extend(form_id.to_le_bytes());
        data
    }

    #[test]
    fn script_properties_name_objects() {
        assert_eq!(script_objects(&vmad(&[]), false), vec![0x802, 0x803, 0x804]);
        // A quest's fragments, then an alias with a script of its own.
        let mut tail = vec![2];
        tail.extend(le16(&[1]));
        tail.extend(text("QF_Gate"));
        tail.extend([0; 9]);
        tail.extend(text("QF_Gate"));
        tail.extend(text("Fragment_0"));
        tail.extend(le16(&[1]));
        tail.extend(object(0x30, 0));
        tail.extend(le16(&[5, 2, 1]));
        tail.extend(text("AliasScript"));
        tail.push(0);
        tail.extend(le16(&[1]));
        tail.extend(text("Door"));
        tail.extend([1, 1]);
        tail.extend(object(0x805, u16::MAX));
        assert_eq!(
            script_objects(&vmad(&tail), true),
            vec![0x802, 0x803, 0x804, 0x805]
        );
        // What is cut short keeps what came before.
        let full = vmad(&[]);
        let cut = full.get(..full.len() - 3).unwrap();
        assert_eq!(script_objects(cut, false), vec![0x802, 0x803]);
    }

    #[test]
    fn large_references_are_read_by_count() {
        let mut rnam = vec![0, 0, 0, 0];
        rnam.extend(2u32.to_le_bytes());
        rnam.extend(le(&[0x900, 0, 0x901, 0, 0x902, 0]));
        assert_eq!(large_references(&rnam), vec![0x900, 0x901]);
        assert_eq!(large_references(&[1, 2]), [] as [u32; 0]);
    }

    proptest::proptest! {
        #[test]
        fn fields_never_panic(
            data in proptest::collection::vec(proptest::prelude::any::<u8>(), 0..300),
        ) {
            let _ = script_objects(&data, true);
            let _ = large_references(&data);
            let keys = Keys(&[0, 1]);
            let mut record = Record {
                signature: *b"REFR",
                form_id: 0x800,
                flags: 0,
                subrecords: [b"XESP", b"XLKR", b"XAPR", b"XTEL", b"XLRT", b"VMAD", b"DATA", b"NAME"]
                    .into_iter()
                    .map(|signature| plugin::Subrecord { signature: *signature, data: data.clone() })
                    .collect(),
                groups: Vec::new(),
            };
            let _ = placed_of(&keys, &record);
            for signature in [b"QUST", b"LCTN", b"PACK"] {
                record.signature = *signature;
                record.subrecords = [b"ALFR", b"VMAD", b"LCSR", b"PLDT", b"PTDA"]
                    .into_iter()
                    .map(|field| plugin::Subrecord { signature: *field, data: data.clone() })
                    .collect();
                let _ = holder_pointers(&keys, &record);
            }
        }
    }
}
