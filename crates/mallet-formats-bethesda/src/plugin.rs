//! Reads records from a plugin (`.esp`, `.esm`, `.esl`).
//!
//! A plugin is a header record followed by groups of records, one top-level
//! group per record type:
//!
//! ```text
//! TES4 record          header: flags and the list of masters
//! GRUP "NPC_"          all characters
//!   NPC_ record
//!     subrecord EDID   the fields of the record
//!     subrecord ...
//! GRUP "WEAP"          all weapons
//! ...
//! ```
//!
//! [`read`] takes the record types the caller is interested in and skips
//! every other group without reading it, so even the game's 250 MB master
//! file is read in a moment.
//!
//! The places of the world and what stands in them are not flat. Their
//! top-level groups, `CELL` and `WRLD`, hold groups within groups:
//!
//! ```text
//! GRUP "CELL"                      interior cells
//!   GRUP block / sub-block         (types 2 and 3)
//!     CELL record                  one room
//!     GRUP cell children (6)       labelled with the cell's form ID
//!       GRUP persistent (8) or temporary (9) children
//!         REFR record              an object placed in the cell
//! GRUP "WRLD"                      worldspaces
//!   WRLD record                    one worldspace
//!   GRUP world children (1)        labelled with the worldspace's form ID
//!     ... cells and their children, as above (types 4, 5, 6, 8, 9)
//! ```
//!
//! [`read_request`] can descend into such groups. Each record it returns
//! then names the groups it was found in.
//!
//! A record's identifier (its *form ID*) is written relative to the plugin's
//! masters. [`resolve_form_id`] turns it into the plugin that defines the
//! record and the record's number there, which is the same in every plugin.
//!
//! Every record and group starts with a header of 24 bytes, except in
//! Oblivion, where the last four bytes are missing. The reader tells the two
//! apart by the plugin's header record: in an Oblivion plugin its first
//! field, `HEDR`, starts where the longer header would still go on.

use std::io::{self, Read, Seek, SeekFrom};

use flate2::read::ZlibDecoder;

use crate::{Bytes, decode_text};

/// The four-character type of a record, group or subrecord, such as `NPC_`.
pub type Signature = [u8; 4];

/// The length of a record or group header from Fallout 3 on.
const HEADER_LEN: usize = 24;
/// The length of a record or group header in Oblivion.
const SHORT_HEADER_LEN: usize = 20;
/// The first field of the plugin header record.
const HEADER_FIELD: &Signature = b"HEDR";
const GROUP: &Signature = b"GRUP";
const PLUGIN_HEADER: &Signature = b"TES4";
const OVERSIZED: &Signature = b"XXXX";

const FLAG_MASTER: u32 = 0x0000_0001;
const FLAG_LOCALIZED: u32 = 0x0000_0080;
const FLAG_LIGHT: u32 = 0x0000_0200;
const FLAG_COMPRESSED: u32 = 0x0004_0000;

/// Top-level groups have type 0. Other types nest inside them.
const GROUP_TYPE_TOP: u32 = 0;

/// The types of groups below the top level that hold places and what is in
/// them. The label of each is given in brackets.
pub mod group_type {
    /// The cells of a worldspace \[the worldspace's form ID\].
    pub const WORLD_CHILDREN: u32 = 1;
    /// A block of interior cells \[block number\].
    pub const INTERIOR_BLOCK: u32 = 2;
    /// A sub-block of interior cells \[sub-block number\].
    pub const INTERIOR_SUB_BLOCK: u32 = 3;
    /// A block of exterior cells \[grid coordinates\].
    pub const EXTERIOR_BLOCK: u32 = 4;
    /// A sub-block of exterior cells \[grid coordinates\].
    pub const EXTERIOR_SUB_BLOCK: u32 = 5;
    /// What a cell contains \[the cell's form ID\].
    pub const CELL_CHILDREN: u32 = 6;
    /// What a cell contains and other places refer to \[the cell's form ID\].
    pub const CELL_PERSISTENT_CHILDREN: u32 = 8;
    /// What a cell contains and only it needs \[the cell's form ID\].
    pub const CELL_TEMPORARY_CHILDREN: u32 = 9;
}

/// How deep groups may nest. The game's files nest five levels deep; the
/// limit keeps a damaged file from recursing without end.
const MAX_GROUP_DEPTH: usize = 12;

/// Upper bounds for sizes read from a file, far above anything real. They
/// keep a damaged size field from asking for gigabytes of memory.
const MAX_HEADER_RECORD: usize = 16 * 1024 * 1024;
const MAX_GROUP: u64 = 1024 * 1024 * 1024;
const MAX_DECOMPRESSED: u64 = 256 * 1024 * 1024;

/// Why a plugin could not be read at all.
///
/// Damage further inside a plugin is not an error: [`read`] returns what it
/// could read and counts the rest in [`Plugin::damaged`].
#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    /// The file could not be read from disk.
    #[error("the plugin could not be read: {0}")]
    Io(#[from] io::Error),
    /// The file does not start with a plugin header.
    #[error("the file is not a plugin")]
    NotAPlugin,
}

/// What a plugin says about itself.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PluginHeader {
    /// The plugins this one builds on, in the order its form IDs refer to
    /// them.
    pub masters: Vec<String>,
    /// Whether the plugin is flagged as a master and therefore loads before
    /// ordinary plugins.
    pub is_master: bool,
    /// Whether the plugin is flagged as light and does not take up a full
    /// slot in the load order.
    pub is_light: bool,
    /// Whether names and descriptions are kept in separate string files.
    pub is_localized: bool,
}

/// One field of a record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subrecord {
    /// The field's type, such as `EDID`.
    pub signature: Signature,
    /// The field's content.
    pub data: Vec<u8>,
}

/// One record with its fields, decompressed if necessary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    /// The record's type, such as `NPC_`.
    pub signature: Signature,
    /// The record's identifier relative to the plugin's masters. See
    /// [`resolve_form_id`].
    pub form_id: u32,
    /// The record's flags.
    pub flags: u32,
    /// The record's fields, in file order.
    pub subrecords: Vec<Subrecord>,
    /// The groups the record was found in below its top-level group,
    /// outermost first. Empty for a record directly in a top-level group.
    pub groups: Vec<GroupLabel>,
}

/// A group below the top level that encloses a record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GroupLabel {
    /// What kind of group it is; see [`group_type`].
    pub group_type: u32,
    /// The group's label. For most nested groups a form ID, relative to the
    /// plugin's masters like any other.
    pub label: u32,
}

/// Which parts of a plugin to read.
#[derive(Debug, Clone, Copy, Default)]
pub struct Request<'a> {
    /// Top-level groups whose records are read. Groups nested inside them
    /// are skipped.
    pub flat: &'a [Signature],
    /// Top-level groups that are read through all their nested groups.
    pub nested: &'a [Signature],
    /// Of the records inside the `nested` groups, the types that are kept.
    /// Records of other types are skipped without being unpacked.
    pub nested_records: &'a [Signature],
    /// Of the records inside the `nested` groups, the types that are kept
    /// only where they change a record of one of the plugin's masters. The
    /// plugin's own records of these types are skipped without being
    /// unpacked. For large records of which only the changes matter.
    pub nested_overrides: &'a [Signature],
}

impl Record {
    /// The content of the first field of type `signature`.
    pub fn first(&self, signature: &Signature) -> Option<&[u8]> {
        self.subrecords
            .iter()
            .find(|subrecord| &subrecord.signature == signature)
            .map(|subrecord| subrecord.data.as_slice())
    }
}

/// The parts of a plugin that were asked for.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Plugin {
    /// What the plugin says about itself.
    pub header: PluginHeader,
    /// The records of the requested types, in file order.
    pub records: Vec<Record>,
    /// How many records or groups were skipped because they are damaged.
    /// Zero for a healthy plugin.
    pub damaged: u32,
}

/// Reads the header of a plugin and all records of the types in `wanted`.
pub fn read<R: Read + Seek>(reader: &mut R, wanted: &[Signature]) -> Result<Plugin, PluginError> {
    read_request(
        reader,
        &Request {
            flat: wanted,
            ..Request::default()
        },
    )
}

/// Reads the header of a plugin and the records `request` asks for.
pub fn read_request<R: Read + Seek>(
    reader: &mut R,
    request: &Request<'_>,
) -> Result<Plugin, PluginError> {
    read_request_where(reader, request, &|_| true)
}

/// Reads the header of a plugin and the records `request` asks for, but
/// only from the nested groups that `descend` accepts. A group that is
/// refused is skipped with everything in it, without being unpacked.
pub fn read_request_where<R: Read + Seek>(
    reader: &mut R,
    request: &Request<'_>,
    descend: &dyn Fn(GroupLabel) -> bool,
) -> Result<Plugin, PluginError> {
    let (header, header_len) = read_plugin_header(reader)?;
    let mut plugin = Plugin {
        header,
        ..Plugin::default()
    };

    while let Some(group_header) = read_header(reader, header_len)? {
        let mut bytes = Bytes::new(&group_header);
        let (Some(signature), Some(size), Some(label), Some(group_type)) = (
            bytes.signature(),
            bytes.u32(),
            bytes.signature(),
            bytes.u32(),
        ) else {
            break;
        };
        let body_len = u64::from(size).checked_sub(header_len as u64);
        let (true, Some(body_len)) = (&signature == GROUP, body_len) else {
            // Anything but a well-formed group at the top level means the
            // rest of the file cannot be trusted.
            plugin.damaged += 1;
            break;
        };

        let flat = request.flat.contains(&label);
        let nested = request.nested.contains(&label);
        if group_type == GROUP_TYPE_TOP && (flat || nested) && body_len <= MAX_GROUP {
            let mut body = Vec::new();
            reader.by_ref().take(body_len).read_to_end(&mut body)?;
            if u64::try_from(body.len()).ok() != Some(body_len) {
                plugin.damaged += 1;
            }
            let scope = if nested {
                Scope::Nested {
                    records: request.nested_records,
                    overrides: request.nested_overrides,
                }
            } else {
                Scope::Flat
            };
            let reading = Reading {
                header_len,
                masters: plugin.header.masters.len(),
                scope,
                descend,
            };
            read_group(&body, &reading, &mut Vec::new(), &mut plugin);
        } else {
            let Ok(offset) = i64::try_from(body_len) else {
                plugin.damaged += 1;
                break;
            };
            reader.seek(SeekFrom::Current(offset))?;
        }
    }
    Ok(plugin)
}

/// Turns a form ID from a plugin into the plugin that defines the record
/// and the record's number in that plugin.
///
/// The top byte of a form ID is an index into the masters of the plugin it
/// was read from. An index past the end of the list means the record is the
/// plugin's own.
pub fn resolve_form_id<'a>(
    form_id: u32,
    masters: &'a [String],
    own_name: &'a str,
) -> (&'a str, u32) {
    let index = usize::try_from(form_id >> 24).unwrap_or(usize::MAX);
    let origin = masters.get(index).map_or(own_name, String::as_str);
    (origin, form_id & 0x00FF_FFFF)
}

/// Reads up to one header of `len` bytes, at most [`HEADER_LEN`]. `None` at
/// a clean end of the file, and also when the file ends in the middle of a
/// header.
fn read_header<R: Read>(reader: &mut R, len: usize) -> io::Result<Option<Vec<u8>>> {
    let mut header = vec![0u8; len.min(HEADER_LEN)];
    let mut filled = 0;
    while let Some(rest) = header.get_mut(filled..).filter(|rest| !rest.is_empty()) {
        match reader.read(rest) {
            Ok(0) => return Ok(None),
            Ok(count) => filled += count,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error),
        }
    }
    Ok(Some(header))
}

/// Reads the plugin header record. Returns it with the length of every
/// record and group header in this plugin.
fn read_plugin_header<R: Read>(reader: &mut R) -> Result<(PluginHeader, usize), PluginError> {
    let header = read_header(reader, HEADER_LEN)?.ok_or(PluginError::NotAPlugin)?;
    let mut bytes = Bytes::new(&header);
    let (Some(signature), Some(size), Some(flags)) = (bytes.signature(), bytes.u32(), bytes.u32())
    else {
        return Err(PluginError::NotAPlugin);
    };
    let size = usize::try_from(size).unwrap_or(usize::MAX);
    if &signature != PLUGIN_HEADER || size > MAX_HEADER_RECORD {
        return Err(PluginError::NotAPlugin);
    }

    // In Oblivion the header is four bytes shorter, so what was read as its
    // last four bytes is already the record's first field.
    let first_field = header.get(SHORT_HEADER_LEN..HEADER_LEN);
    let (header_len, mut data) = match first_field {
        Some(field) if field == HEADER_FIELD.as_slice() => (SHORT_HEADER_LEN, field.to_vec()),
        _ => (HEADER_LEN, Vec::new()),
    };
    data.truncate(size);
    let rest = size.saturating_sub(data.len());
    reader
        .by_ref()
        .take(u64::try_from(rest).unwrap_or(u64::MAX))
        .read_to_end(&mut data)?;

    let masters = read_subrecords(&data)
        .into_iter()
        .filter(|subrecord| &subrecord.signature == b"MAST")
        .map(|subrecord| decode_text(until_nul(&subrecord.data)))
        .collect();
    let header = PluginHeader {
        masters,
        is_master: flags & FLAG_MASTER != 0,
        is_light: flags & FLAG_LIGHT != 0,
        is_localized: flags & FLAG_LOCALIZED != 0,
    };
    Ok((header, header_len))
}

/// What to read from the body of a group.
#[derive(Clone, Copy)]
enum Scope<'a> {
    /// Every record, and no nested group.
    Flat,
    /// The records of these types, in this group and all nested ones:
    /// `records` always, `overrides` where they change a record of a
    /// master.
    Nested {
        records: &'a [Signature],
        overrides: &'a [Signature],
    },
}

/// What stays the same while the groups of a plugin are read.
struct Reading<'a> {
    /// The length of every record and group header in the plugin.
    header_len: usize,
    /// How many masters the plugin has.
    masters: usize,
    /// What to read from the groups.
    scope: Scope<'a>,
    /// Which nested groups to descend into.
    descend: &'a dyn Fn(GroupLabel) -> bool,
}

/// Reads the records in the body of a group. `groups` are the nested
/// groups the body lies in.
fn read_group(
    body: &[u8],
    reading: &Reading<'_>,
    groups: &mut Vec<GroupLabel>,
    plugin: &mut Plugin,
) {
    let Reading {
        header_len,
        masters,
        scope,
        descend,
    } = *reading;
    let mut bytes = Bytes::new(body);
    while bytes.remaining() > 0 {
        let (Some(signature), Some(size), Some(third), Some(form_id)) =
            (bytes.signature(), bytes.u32(), bytes.u32(), bytes.u32())
        else {
            plugin.damaged += 1;
            return;
        };
        // The rest of the header: version control data.
        let rest_of_header = header_len.saturating_sub(16);
        let size = usize::try_from(size).unwrap_or(usize::MAX);

        if &signature == GROUP {
            // For a group, the size includes the header just read, and its
            // third field is the type that the label belongs to.
            let (label, group_type) = (third, form_id);
            let Some(nested) = size.checked_sub(header_len).and_then(|body_len| {
                bytes
                    .skip(rest_of_header)
                    .and_then(|()| bytes.take(body_len))
            }) else {
                plugin.damaged += 1;
                return;
            };
            let group = GroupLabel { group_type, label };
            if let Scope::Nested { .. } = scope
                && descend(group)
            {
                if groups.len() < MAX_GROUP_DEPTH {
                    groups.push(group);
                    read_group(nested, reading, groups, plugin);
                    groups.pop();
                } else {
                    plugin.damaged += 1;
                }
            }
            continue;
        }

        let flags = third;
        let Some(data) = bytes.skip(rest_of_header).and_then(|()| bytes.take(size)) else {
            plugin.damaged += 1;
            return;
        };
        if let Scope::Nested { records, overrides } = scope
            && !records.contains(&signature)
            && !(overrides.contains(&signature) && changes_master(form_id, masters))
        {
            continue;
        }
        let subrecords = if flags & FLAG_COMPRESSED == 0 {
            Some(read_subrecords(data))
        } else {
            decompress(data).map(|data| read_subrecords(&data))
        };
        match subrecords {
            Some(subrecords) => plugin.records.push(Record {
                signature,
                form_id,
                flags,
                subrecords,
                groups: groups.clone(),
            }),
            None => plugin.damaged += 1,
        }
    }
}

/// Whether a record with this form ID, in a plugin with `masters` masters,
/// changes a record of one of them rather than being the plugin's own.
fn changes_master(form_id: u32, masters: usize) -> bool {
    usize::try_from(form_id >> 24).is_ok_and(|index| index < masters)
}

/// Unpacks the data of a compressed record: the unpacked size, then a zlib
/// stream.
fn decompress(data: &[u8]) -> Option<Vec<u8>> {
    let mut bytes = Bytes::new(data);
    let expected = u64::from(bytes.u32()?);
    if expected > MAX_DECOMPRESSED {
        return None;
    }
    let compressed = bytes.take(bytes.remaining())?;
    let mut unpacked = Vec::new();
    ZlibDecoder::new(compressed)
        .take(expected)
        .read_to_end(&mut unpacked)
        .ok()?;
    Some(unpacked)
}

/// Splits the data of a record into its fields. Stops at the first field
/// that does not fit and keeps the ones before it.
fn read_subrecords(data: &[u8]) -> Vec<Subrecord> {
    let mut bytes = Bytes::new(data);
    let mut subrecords = Vec::new();
    let mut oversized: Option<usize> = None;

    while bytes.remaining() > 0 {
        let (Some(signature), Some(size)) = (bytes.signature(), bytes.u16()) else {
            break;
        };
        // A field larger than 64 KiB is announced by an `XXXX` field that
        // holds its real size; its own size field is then zero.
        let size = oversized.take().unwrap_or_else(|| usize::from(size));
        let Some(content) = bytes.take(size) else {
            break;
        };
        if &signature == OVERSIZED {
            oversized = content
                .try_into()
                .ok()
                .map(u32::from_le_bytes)
                .and_then(|size| usize::try_from(size).ok());
        } else {
            subrecords.push(Subrecord {
                signature,
                data: content.to_vec(),
            });
        }
    }
    subrecords
}

/// The bytes of a NUL-terminated text without the terminator.
pub fn until_nul(data: &[u8]) -> &[u8] {
    data.split(|&byte| byte == 0).next().unwrap_or(data)
}

/// The text in a field that holds a NUL-terminated string, such as `EDID`.
pub fn zstring(data: &[u8]) -> String {
    decode_text(until_nul(data))
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use proptest::prelude::*;

    use super::*;
    use crate::fixture::{self, RecordSpec};

    const NPC: Signature = *b"NPC_";
    const WEAPON: Signature = *b"WEAP";

    fn npc(form_id: u32, editor_id: &str) -> RecordSpec {
        RecordSpec::new(NPC, form_id).with(*b"EDID", fixture::zstring(editor_id))
    }

    fn sample() -> Vec<u8> {
        fixture::plugin(
            &["Skyrim.esm", "Update.esm"],
            FLAG_MASTER,
            &[
                (WEAPON, vec![RecordSpec::new(WEAPON, 0x0001_2EB7)]),
                (
                    NPC,
                    vec![
                        npc(0x0001_3BA1, "Uthgerd"),
                        npc(0x0200_0D62, "NewGuard").compressed(),
                    ],
                ),
            ],
        )
    }

    fn read_bytes(data: &[u8], wanted: &[Signature]) -> Plugin {
        read(&mut Cursor::new(data), wanted).unwrap()
    }

    #[test]
    fn reads_the_header() {
        let plugin = read_bytes(&sample(), &[]);
        assert_eq!(plugin.header.masters, ["Skyrim.esm", "Update.esm"]);
        assert!(plugin.header.is_master);
        assert!(!plugin.header.is_light);
        assert!(!plugin.header.is_localized);
        assert_eq!(plugin.records, Vec::new());
    }

    #[test]
    fn reads_only_the_requested_record_types() {
        let plugin = read_bytes(&sample(), &[NPC]);
        let ids: Vec<_> = plugin.records.iter().map(|record| record.form_id).collect();
        assert_eq!(ids, [0x0001_3BA1, 0x0200_0D62]);
        assert!(plugin.records.iter().all(|record| record.signature == NPC));
        assert_eq!(plugin.damaged, 0);
    }

    #[test]
    fn compressed_records_are_unpacked() {
        let plugin = read_bytes(&sample(), &[NPC]);
        let compressed = &plugin.records[1];
        assert_ne!(compressed.flags & FLAG_COMPRESSED, 0);
        assert_eq!(compressed.first(b"EDID"), Some(b"NewGuard\0".as_slice()));
    }

    #[test]
    fn fields_larger_than_64k_are_read_whole() {
        let big = vec![7u8; 70_000];
        let data = fixture::plugin(
            &[],
            0,
            &[(
                NPC,
                vec![
                    RecordSpec::new(NPC, 0x800)
                        .with(*b"EDID", fixture::zstring("Big"))
                        .with(*b"NAM9", big.clone())
                        .with(*b"FULL", vec![1, 2, 3, 4]),
                ],
            )],
        );
        let plugin = read_bytes(&data, &[NPC]);
        let record = &plugin.records[0];
        assert_eq!(record.first(b"NAM9"), Some(big.as_slice()));
        assert_eq!(record.first(b"FULL"), Some([1, 2, 3, 4].as_slice()));
    }

    #[test]
    fn nested_groups_are_skipped() {
        let mut data = fixture::plugin(&[], 0, &[]);
        let nested = fixture::group(*b"\x01\0\0\0", 6, &fixture::record(&npc(0x999, "Inner")));
        let mut body = fixture::record(&npc(0x801, "Before"));
        body.extend(nested);
        body.extend(fixture::record(&npc(0x802, "After")));
        data.extend(fixture::group(NPC, 0, &body));

        let plugin = read_bytes(&data, &[NPC]);
        let ids: Vec<_> = plugin.records.iter().map(|record| record.form_id).collect();
        assert_eq!(ids, [0x801, 0x802]);
    }

    /// A plugin with a room and a worldspace, each with objects in it.
    fn places() -> Vec<u8> {
        let cell = RecordSpec::new(*b"CELL", 0x0100_0801).with(*b"EDID", fixture::zstring("Room"));
        let world = RecordSpec::new(*b"WRLD", 0x0000_003C);
        let outside = RecordSpec::new(*b"CELL", 0x0100_0802);
        let placed = |form_id| fixture::reference(form_id, 0x0001_0000, [1.0; 3], [0.0; 3], None);
        let land = RecordSpec::new(*b"LAND", 0x0100_0803)
            .with(*b"VHGT", vec![0; 64])
            .compressed();
        fixture::plugin_with_raw_groups(
            &["Skyrim.esm"],
            0,
            &[
                fixture::group(NPC, 0, &fixture::record(&npc(0x0100_0900, "Somebody"))),
                fixture::interior_cells(&[(cell, vec![placed(0x0100_0811)])]),
                fixture::worldspace(&world, &[(outside, vec![land, placed(0x0100_0812)])]),
            ],
        )
    }

    fn read_places(data: &[u8]) -> Plugin {
        read_request(
            &mut Cursor::new(data),
            &Request {
                flat: &[NPC],
                nested: &[*b"CELL", *b"WRLD"],
                nested_records: &[*b"REFR", *b"CELL", *b"WRLD"],
                nested_overrides: &[],
            },
        )
        .unwrap()
    }

    #[test]
    fn records_asked_for_as_overrides_are_kept_only_where_they_change_a_master() {
        let plugin = read_request(
            &mut Cursor::new(places()),
            &Request {
                flat: &[],
                nested: &[*b"CELL", *b"WRLD"],
                nested_records: &[],
                nested_overrides: &[*b"WRLD", *b"CELL", *b"LAND"],
            },
        )
        .unwrap();
        assert_eq!(plugin.damaged, 0);

        // The worldspace belongs to the master; the cells and the land are
        // the plugin's own.
        let found: Vec<_> = plugin
            .records
            .iter()
            .map(|record| (record.signature, record.form_id))
            .collect();
        assert_eq!(found, [(*b"WRLD", 0x0000_003C)]);
    }

    #[test]
    fn nested_groups_are_read_on_request_with_the_groups_around_each_record() {
        let plugin = read_places(&places());
        assert_eq!(plugin.damaged, 0);

        let found: Vec<_> = plugin
            .records
            .iter()
            .map(|record| (record.signature, record.form_id))
            .collect();
        assert_eq!(
            found,
            [
                (NPC, 0x0100_0900),
                (*b"CELL", 0x0100_0801),
                (*b"REFR", 0x0100_0811),
                (*b"WRLD", 0x0000_003C),
                (*b"CELL", 0x0100_0802),
                (*b"REFR", 0x0100_0812),
            ]
        );

        let groups = |form_id: u32| {
            plugin
                .records
                .iter()
                .find(|record| record.form_id == form_id)
                .unwrap()
                .groups
                .iter()
                .map(|group| (group.group_type, group.label))
                .collect::<Vec<_>>()
        };
        // Records of flat groups and top-level records have no nesting.
        assert_eq!(groups(0x0100_0900), []);
        assert_eq!(groups(0x0000_003C), []);
        assert_eq!(
            groups(0x0100_0811),
            [
                (group_type::INTERIOR_BLOCK, 0),
                (group_type::INTERIOR_SUB_BLOCK, 0),
                (group_type::CELL_CHILDREN, 0x0100_0801),
                (group_type::CELL_TEMPORARY_CHILDREN, 0x0100_0801),
            ]
        );
        assert_eq!(
            groups(0x0100_0812),
            [
                (group_type::WORLD_CHILDREN, 0x0000_003C),
                (group_type::EXTERIOR_BLOCK, 0),
                (group_type::EXTERIOR_SUB_BLOCK, 0),
                (group_type::CELL_CHILDREN, 0x0100_0802),
                (group_type::CELL_TEMPORARY_CHILDREN, 0x0100_0802),
            ]
        );
    }

    #[test]
    fn refused_groups_are_skipped_with_everything_in_them() {
        let plugin = read_request_where(
            &mut Cursor::new(places()),
            &Request {
                flat: &[NPC],
                nested: &[*b"CELL", *b"WRLD"],
                nested_records: &[*b"REFR", *b"CELL", *b"WRLD"],
                nested_overrides: &[],
            },
            &|group| group.group_type != group_type::CELL_TEMPORARY_CHILDREN,
        )
        .unwrap();
        let found: Vec<_> = plugin.records.iter().map(|record| record.form_id).collect();
        // Both objects lie in temporary children; the cells are still read.
        assert_eq!(found, [0x0100_0900, 0x0100_0801, 0x0000_003C, 0x0100_0802]);
    }

    #[test]
    fn without_a_request_for_nesting_the_places_are_skipped() {
        let plugin = read_bytes(&places(), &[NPC, *b"CELL"]);
        let found: Vec<_> = plugin.records.iter().map(|record| record.form_id).collect();
        // The interior group holds no record directly, only blocks.
        assert_eq!(found, [0x0100_0900]);
    }

    #[test]
    fn form_ids_resolve_to_the_defining_plugin() {
        let masters = ["Skyrim.esm".to_owned(), "Update.esm".to_owned()];
        assert_eq!(
            resolve_form_id(0x0001_3BA1, &masters, "Mine.esp"),
            ("Skyrim.esm", 0x01_3BA1)
        );
        assert_eq!(
            resolve_form_id(0x0100_0800, &masters, "Mine.esp"),
            ("Update.esm", 0x800)
        );
        assert_eq!(
            resolve_form_id(0x0200_0D62, &masters, "Mine.esp"),
            ("Mine.esp", 0xD62)
        );
        // An index far past the masters still means "this plugin".
        assert_eq!(
            resolve_form_id(0xFE00_0001, &masters, "Mine.esp"),
            ("Mine.esp", 1)
        );
    }

    #[test]
    fn a_file_that_is_not_a_plugin_is_rejected() {
        for data in [b"".as_slice(), b"BSA\0", &[0u8; 64]] {
            assert!(matches!(
                read(&mut Cursor::new(data), &[NPC]),
                Err(PluginError::NotAPlugin)
            ));
        }
    }

    #[test]
    fn a_cut_off_plugin_yields_what_was_complete() {
        let data = sample();
        let cut = &data[..data.len() - 10];
        let plugin = read_bytes(cut, &[NPC]);
        assert_eq!(plugin.records.len(), 1);
        assert!(plugin.damaged > 0);
    }

    #[test]
    fn garbage_in_a_compressed_record_is_counted_not_fatal() {
        let mut data = fixture::plugin(&[], 0, &[]);
        let mut broken = RecordSpec::new(NPC, 0x800);
        broken.flags = FLAG_COMPRESSED;
        let mut body = fixture::record_with_raw_data(&broken, &[9, 0, 0, 0, 1, 2, 3]);
        body.extend(fixture::record(&npc(0x801, "Fine")));
        data.extend(fixture::group(NPC, 0, &body));

        let plugin = read_bytes(&data, &[NPC]);
        assert_eq!(plugin.records.len(), 1);
        assert_eq!(plugin.damaged, 1);
    }

    /// Like [`sample`], with Oblivion's 20-byte headers.
    fn oblivion_sample() -> Vec<u8> {
        fixture::oblivion::plugin(
            &["Oblivion.esm"],
            FLAG_MASTER,
            &[
                (WEAPON, vec![RecordSpec::new(WEAPON, 0x0001_2EB7)]),
                (
                    NPC,
                    vec![
                        npc(0x0001_3BA1, "Umbacano"),
                        npc(0x0100_0D62, "NewGuard").compressed(),
                    ],
                ),
            ],
        )
    }

    /// Like [`places`], with Oblivion's 20-byte headers.
    fn oblivion_places() -> Vec<u8> {
        let cell = RecordSpec::new(*b"CELL", 0x0000_0801).with(*b"EDID", fixture::zstring("Room"));
        let world = RecordSpec::new(*b"WRLD", 0x0000_003C);
        let outside = RecordSpec::new(*b"CELL", 0x0000_0802);
        let placed = |form_id| fixture::reference(form_id, 0x0001_0000, [1.0; 3], [0.0; 3], None);
        let land = RecordSpec::new(*b"LAND", 0x0000_0803)
            .with(*b"VHGT", vec![0; 64])
            .compressed();
        fixture::oblivion::plugin_with_raw_groups(
            &["Oblivion.esm"],
            0,
            &[
                fixture::oblivion::group(
                    NPC,
                    0,
                    &fixture::oblivion::record(&npc(0x0000_0900, "Somebody")),
                ),
                fixture::oblivion::interior_cells(&[(cell, vec![placed(0x0000_0811)])]),
                fixture::oblivion::worldspace(
                    &world,
                    &[(outside, vec![land, placed(0x0000_0812)])],
                ),
            ],
        )
    }

    #[test]
    fn oblivion_fixtures_have_short_headers() {
        let data = oblivion_sample();
        assert_eq!(&data[20..24], b"HEDR");
        let data = sample();
        assert_ne!(&data[20..24], b"HEDR");
    }

    #[test]
    fn reads_the_header_of_an_oblivion_plugin() {
        let plugin = read_bytes(&oblivion_sample(), &[]);
        assert_eq!(plugin.header.masters, ["Oblivion.esm"]);
        assert!(plugin.header.is_master);
        assert_eq!(plugin.records, Vec::new());
        assert_eq!(plugin.damaged, 0);
    }

    #[test]
    fn reads_the_records_of_an_oblivion_plugin() {
        let plugin = read_bytes(&oblivion_sample(), &[NPC, WEAPON]);
        assert_eq!(plugin.damaged, 0);
        let found: Vec<_> = plugin
            .records
            .iter()
            .map(|record| (record.signature, record.form_id))
            .collect();
        assert_eq!(
            found,
            [
                (WEAPON, 0x0001_2EB7),
                (NPC, 0x0001_3BA1),
                (NPC, 0x0100_0D62)
            ]
        );
        assert_eq!(
            plugin.records[1].first(b"EDID"),
            Some(b"Umbacano\0".as_slice())
        );
    }

    #[test]
    fn compressed_records_of_an_oblivion_plugin_are_unpacked() {
        let plugin = read_bytes(&oblivion_sample(), &[NPC]);
        let compressed = &plugin.records[1];
        assert_ne!(compressed.flags & FLAG_COMPRESSED, 0);
        assert_eq!(compressed.first(b"EDID"), Some(b"NewGuard\0".as_slice()));
    }

    #[test]
    fn nested_groups_of_an_oblivion_plugin_are_read_on_request() {
        let plugin = read_places(&oblivion_places());
        assert_eq!(plugin.damaged, 0);
        let found: Vec<_> = plugin
            .records
            .iter()
            .map(|record| (record.signature, record.form_id))
            .collect();
        assert_eq!(
            found,
            [
                (NPC, 0x0000_0900),
                (*b"CELL", 0x0000_0801),
                (*b"REFR", 0x0000_0811),
                (*b"WRLD", 0x0000_003C),
                (*b"CELL", 0x0000_0802),
                (*b"REFR", 0x0000_0812),
            ]
        );
        let outside = plugin
            .records
            .iter()
            .find(|record| record.form_id == 0x0000_0812)
            .unwrap();
        let groups: Vec<_> = outside
            .groups
            .iter()
            .map(|group| (group.group_type, group.label))
            .collect();
        assert_eq!(
            groups,
            [
                (group_type::WORLD_CHILDREN, 0x0000_003C),
                (group_type::EXTERIOR_BLOCK, 0),
                (group_type::EXTERIOR_SUB_BLOCK, 0),
                (group_type::CELL_CHILDREN, 0x0000_0802),
                (group_type::CELL_TEMPORARY_CHILDREN, 0x0000_0802),
            ]
        );
    }

    #[test]
    fn nested_groups_of_an_oblivion_plugin_are_skipped_in_flat_reading() {
        let mut data = fixture::oblivion::plugin(&[], 0, &[]);
        let nested = fixture::oblivion::group(
            *b"\x01\0\0\0",
            6,
            &fixture::oblivion::record(&npc(0x999, "Inner")),
        );
        let mut body = fixture::oblivion::record(&npc(0x801, "Before"));
        body.extend(nested);
        body.extend(fixture::oblivion::record(&npc(0x802, "After")));
        data.extend(fixture::oblivion::group(NPC, 0, &body));

        let plugin = read_bytes(&data, &[NPC]);
        let ids: Vec<_> = plugin.records.iter().map(|record| record.form_id).collect();
        assert_eq!(ids, [0x801, 0x802]);
        assert_eq!(plugin.damaged, 0);
    }

    #[test]
    fn a_cut_off_oblivion_plugin_yields_what_was_complete() {
        let data = oblivion_sample();
        let plugin = read_bytes(&data[..data.len() - 10], &[NPC]);
        assert_eq!(plugin.records.len(), 1);
        assert!(plugin.damaged > 0);
    }

    #[test]
    fn an_oblivion_header_record_cut_short_is_not_a_crash() {
        // A plugin header whose size is smaller than the field already
        // read ahead.
        let mut data = oblivion_sample();
        data[4..8].copy_from_slice(&2u32.to_le_bytes());
        let plugin = read_bytes(&data, &[NPC]);
        assert_eq!(plugin.header.masters, Vec::<String>::new());
    }

    proptest! {
        #[test]
        fn never_panics_on_arbitrary_bytes(data in proptest::collection::vec(any::<u8>(), 0..512)) {
            let _ = read(&mut Cursor::new(&data), &[NPC]);
        }

        #[test]
        fn never_panics_on_arbitrary_bytes_after_an_oblivion_header(
            size in any::<u32>(),
            rest in proptest::collection::vec(any::<u8>(), 0..512),
        ) {
            let mut data = b"TES4".to_vec();
            data.extend(size.to_le_bytes());
            data.extend([0; 12]);
            data.extend(b"HEDR");
            data.extend(rest);
            let _ = read_request(
                &mut Cursor::new(&data),
                &Request {
                    flat: &[NPC],
                    nested: &[*b"CELL", *b"WRLD"],
                    nested_records: &[*b"REFR", *b"CELL", *b"WRLD"],
                    nested_overrides: &[],
                },
            );
        }

        #[test]
        fn never_panics_on_a_damaged_plugin(
            oblivion in any::<bool>(),
            position in 0usize..400,
            value in any::<u8>(),
            cut in 0usize..400,
        ) {
            let mut data = if oblivion { oblivion_sample() } else { sample() };
            if let Some(byte) = data.get_mut(position) {
                *byte = value;
            }
            data.truncate(data.len().saturating_sub(cut));
            let _ = read(&mut Cursor::new(&data), &[NPC, WEAPON]);
        }

        #[test]
        fn never_panics_on_damaged_places(
            oblivion in any::<bool>(),
            position in 0usize..700,
            value in any::<u8>(),
            cut in 0usize..700,
        ) {
            let mut data = if oblivion { oblivion_places() } else { places() };
            if let Some(byte) = data.get_mut(position) {
                *byte = value;
            }
            data.truncate(data.len().saturating_sub(cut));
            let _ = read_request(
                &mut Cursor::new(&data),
                &Request {
                    flat: &[NPC],
                    nested: &[*b"CELL", *b"WRLD"],
                    nested_records: &[*b"REFR", *b"CELL", *b"WRLD"],
                    nested_overrides: &[],
                },
            );
        }
    }
}
