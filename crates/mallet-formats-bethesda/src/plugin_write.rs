//! Writes a small plugin that changes records of other plugins.
//!
//! Mod Mallet writes plugins only into its own folder, as patches the mod
//! manager installs (ADR 0060). Such a plugin holds *overrides*: copies of
//! records of its masters with a few fields changed. What a record means
//! is left to the caller; this module only puts records into the nesting a
//! plugin needs, which [`crate::plugin`] describes:
//!
//! ```text
//! TES4                      header: format, masters, flags
//! GRUP "CELL"               interior cells
//!   GRUP block (2)          last decimal digit of the cell's number
//!     GRUP sub-block (3)    the digit before it
//!       CELL                the cell
//!       GRUP children (6)   labelled with the cell's form ID
//!         GRUP persistent (8) / temporary (9)
//!           REFR, NAVM ...  what the caller put there
//! GRUP "WRLD"               worldspaces
//!   WRLD                    the worldspace
//!   GRUP world children (1) labelled with the worldspace's form ID
//!     CELL                  its persistent cell, if any, with children
//!     GRUP block (4)        32 × 32 cells, labelled y, x
//!       GRUP sub-block (5)  8 × 8 cells
//!         CELL + children   as above
//! ```
//!
//! [`PluginBuilder`] collects the records cell by cell: a cell and its
//! worldspace are given with the first records put into it and written once.
//! Records of other types go into top-level groups of their own type.
//! Every form ID is written as given, so the caller has already rewritten
//! them for the new plugin's masters. Records are written uncompressed.
//!
//! The plugin is flagged as light where the game knows light plugins and
//! the plugin's own new records fit into one; overrides do not count.

use crate::plugin::{Record, Signature, Subrecord, group_type};

/// The four bytes that start a group.
const GROUP: &Signature = b"GRUP";
/// The length of a record or group header from Fallout 3 on.
const HEADER_LEN: usize = 24;
/// A field larger than this is announced by an `XXXX` field.
const MAX_FIELD: usize = 0xFFFF;

const FLAG_LIGHT: u32 = 0x0000_0200;
const FLAG_COMPRESSED: u32 = 0x0004_0000;

/// How many masters a plugin may name: the top byte of a form ID indexes
/// them, and the index after the last one means the plugin itself.
const MAX_MASTERS: usize = 0xFE;
/// The highest object number a light plugin's own records may have.
const LIGHT_LAST: u32 = 0xFFF;

/// The version of the plugin format a game reads.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Format {
    /// The version in the header's `HEDR` field.
    pub version: f32,
    /// The form version written for records given without one.
    pub form_version: u16,
    /// Whether the game knows light plugins (`ESL` flag).
    pub light_plugins: bool,
}

impl Format {
    /// Skyrim Special Edition and Anniversary Edition.
    pub const SKYRIM_SE: Self = Self {
        version: 1.71,
        form_version: 44,
        light_plugins: true,
    };
}

/// A record as it is written: its type, form ID (relative to the new
/// plugin's masters), flags, form version and fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrittenRecord {
    /// The record's type, such as `REFR`.
    pub signature: Signature,
    /// The record's form ID, relative to the new plugin's masters.
    pub form_id: u32,
    /// The record's flags. The compression flag is cleared on writing.
    pub flags: u32,
    /// The record format version; 0 writes the format's.
    pub form_version: u16,
    /// The record's fields, in order.
    pub subrecords: Vec<Subrecord>,
}

impl WrittenRecord {
    /// A copy of a record that was read, with its form version, for the
    /// caller to change.
    pub fn from_read(record: &Record, form_version: u16) -> Self {
        Self {
            signature: record.signature,
            form_id: record.form_id,
            flags: record.flags,
            form_version,
            subrecords: record.subrecords.clone(),
        }
    }

    /// The content of the first field of type `signature`.
    pub fn first(&self, signature: &Signature) -> Option<&[u8]> {
        self.subrecords
            .iter()
            .find(|subrecord| &subrecord.signature == signature)
            .map(|subrecord| subrecord.data.as_slice())
    }
}

/// Which children of a cell a record belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Children {
    /// What other places may refer to (group type 8).
    Persistent,
    /// What only the cell needs (group type 9). Navigation meshes and most
    /// placed objects.
    Temporary,
}

/// Why a plugin could not be written.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WriteError {
    /// More masters than form IDs can name.
    #[error("a plugin can name at most 254 masters, not {0}")]
    TooManyMasters(usize),
    /// A form ID names a master the plugin does not have.
    #[error("the form ID {0:08X} names a master the plugin does not have")]
    UnknownMaster(u32),
    /// A record or group is larger than the format can hold.
    #[error("a record or group is too large for a plugin")]
    TooLarge,
    /// An outside cell lies beyond the grid the format can label.
    #[error("the cell {0:08X} lies outside the grid of a worldspace")]
    OutsideTheGrid(u32),
}

/// What a cell holds in the new plugin.
#[derive(Debug, Clone)]
struct CellEntry {
    cell: WrittenRecord,
    persistent: Vec<WrittenRecord>,
    temporary: Vec<WrittenRecord>,
}

impl CellEntry {
    fn new(cell: WrittenRecord) -> Self {
        Self {
            cell,
            persistent: Vec::new(),
            temporary: Vec::new(),
        }
    }

    /// Puts `records` among its children; a record with the form ID of one
    /// already there replaces it.
    fn add(&mut self, children: Children, records: Vec<WrittenRecord>) {
        let list = match children {
            Children::Persistent => &mut self.persistent,
            Children::Temporary => &mut self.temporary,
        };
        for record in records {
            match list
                .iter_mut()
                .find(|known| known.form_id == record.form_id)
            {
                Some(known) => *known = record,
                None => list.push(record),
            }
        }
    }

    /// The cell's place in its worldspace's grid, from its `XCLC` field;
    /// `None` for a worldspace's persistent cell, which has none.
    fn grid(&self) -> Option<(i32, i32)> {
        let data = self.cell.first(b"XCLC")?;
        let x = i32::from_le_bytes(data.get(0..4)?.try_into().ok()?);
        let y = i32::from_le_bytes(data.get(4..8)?.try_into().ok()?);
        Some((x, y))
    }
}

/// What a worldspace holds in the new plugin.
#[derive(Debug, Clone)]
struct WorldEntry {
    world: WrittenRecord,
    cells: Vec<CellEntry>,
}

/// Collects the records of a new plugin and writes it.
#[derive(Debug, Clone, Default)]
pub struct PluginBuilder {
    masters: Vec<String>,
    author: Option<String>,
    description: Option<String>,
    interiors: Vec<CellEntry>,
    worlds: Vec<WorldEntry>,
    top: Vec<WrittenRecord>,
}

impl PluginBuilder {
    /// A plugin with these masters, in this order, and nothing in it yet.
    pub fn new(masters: Vec<String>) -> Self {
        Self {
            masters,
            ..Self::default()
        }
    }

    /// Who made it, written into its header.
    #[must_use]
    pub fn author(mut self, author: &str) -> Self {
        self.author = Some(author.to_owned());
        self
    }

    /// What it is for, written into its header.
    #[must_use]
    pub fn description(mut self, description: &str) -> Self {
        self.description = Some(description.to_owned());
        self
    }

    /// The masters it names.
    pub fn masters(&self) -> &[String] {
        &self.masters
    }

    /// Whether nothing has been put into it.
    pub fn is_empty(&self) -> bool {
        self.interiors.is_empty() && self.worlds.is_empty() && self.top.is_empty()
    }

    /// Puts `records` among the children of the interior cell `cell`. The
    /// cell is written as given the first time it is named.
    pub fn add_to_interior(
        &mut self,
        cell: &WrittenRecord,
        children: Children,
        records: Vec<WrittenRecord>,
    ) {
        add_to_cell(&mut self.interiors, cell, children, records);
    }

    /// Puts `records` among the children of the outside cell `cell` of the
    /// worldspace `world`; a cell without a place in the grid (`XCLC`) is
    /// the worldspace's persistent cell. The worldspace and the cell are
    /// written as given the first time each is named. For overrides of
    /// placed objects, and of navigation meshes (ADR 0052).
    pub fn add_to_exterior(
        &mut self,
        world: &WrittenRecord,
        cell: &WrittenRecord,
        children: Children,
        records: Vec<WrittenRecord>,
    ) {
        let index = self
            .worlds
            .iter()
            .position(|known| known.world.form_id == world.form_id)
            .unwrap_or(self.worlds.len());
        if index == self.worlds.len() {
            self.worlds.push(WorldEntry {
                world: world.clone(),
                cells: Vec::new(),
            });
        }
        if let Some(entry) = self.worlds.get_mut(index) {
            add_to_cell(&mut entry.cells, cell, children, records);
        }
    }

    /// Puts a record into the top-level group of its type, replacing one
    /// with the same form ID. Not for cells, worldspaces or what they
    /// hold.
    pub fn add_top_level(&mut self, record: WrittenRecord) {
        match self
            .top
            .iter_mut()
            .find(|known| known.signature == record.signature && known.form_id == record.form_id)
        {
            Some(known) => *known = record,
            None => self.top.push(record),
        }
    }

    /// Every record put into it, cells and worldspaces included.
    fn records(&self) -> impl Iterator<Item = &WrittenRecord> {
        fn in_cell(cell: &CellEntry) -> impl Iterator<Item = &WrittenRecord> {
            std::iter::once(&cell.cell)
                .chain(&cell.persistent)
                .chain(&cell.temporary)
        }
        self.top
            .iter()
            .chain(self.interiors.iter().flat_map(in_cell))
            .chain(self.worlds.iter().flat_map(|world| {
                std::iter::once(&world.world).chain(world.cells.iter().flat_map(in_cell))
            }))
    }

    /// Whether the plugin can be flagged as light in `format`: the game
    /// knows light plugins, and its own new records (not overrides) are
    /// few enough and numbered within the range of one.
    pub fn fits_light(&self, format: &Format) -> bool {
        if !format.light_plugins {
            return false;
        }
        let own = u32::try_from(self.masters.len()).unwrap_or(u32::MAX);
        // From format 1.71 on, light plugins may number from 1.
        let first = if format.version >= 1.71 { 0x001 } else { 0x800 };
        self.records()
            .filter(|record| record.form_id >> 24 >= own)
            .all(|record| (first..=LIGHT_LAST).contains(&(record.form_id & 0x00FF_FFFF)))
    }

    /// The plugin's bytes.
    ///
    /// # Errors
    ///
    /// If it names too many masters, a form ID names a master it does not
    /// have, a cell lies beyond the grid, or something is too large.
    pub fn write(&self, format: &Format) -> Result<Vec<u8>, WriteError> {
        if self.masters.len() > MAX_MASTERS {
            return Err(WriteError::TooManyMasters(self.masters.len()));
        }
        let own = u32::try_from(self.masters.len()).unwrap_or(u32::MAX);
        if let Some(record) = self.records().find(|record| record.form_id >> 24 > own) {
            return Err(WriteError::UnknownMaster(record.form_id));
        }

        let mut writer = Writer {
            format: *format,
            count: 0,
        };
        let mut body = Vec::new();
        let mut signatures: Vec<Signature> = Vec::new();
        for record in &self.top {
            if !signatures.contains(&record.signature) {
                signatures.push(record.signature);
            }
        }
        for signature in signatures {
            let mut records = Vec::new();
            for record in self
                .top
                .iter()
                .filter(|record| record.signature == signature)
            {
                records.extend(writer.record(record)?);
            }
            body.extend(writer.group(signature, 0, &records)?);
        }
        if !self.interiors.is_empty() {
            body.extend(writer.interiors(&self.interiors)?);
        }
        if !self.worlds.is_empty() {
            let mut worlds = Vec::new();
            let mut sorted: Vec<&WorldEntry> = self.worlds.iter().collect();
            sorted.sort_by_key(|world| world.world.form_id);
            for world in sorted {
                worlds.extend(writer.world(world)?);
            }
            body.extend(writer.group(*b"WRLD", 0, &worlds)?);
        }

        let light = self.fits_light(format);
        let mut header = Vec::new();
        let mut hedr = Vec::new();
        hedr.extend(format.version.to_le_bytes());
        hedr.extend(writer.count.to_le_bytes());
        hedr.extend(0x800u32.to_le_bytes());
        push_field(&mut header, *b"HEDR", &hedr)?;
        push_field(
            &mut header,
            *b"CNAM",
            &zstring(self.author.as_deref().unwrap_or("Mod Mallet")),
        )?;
        if let Some(description) = &self.description {
            push_field(&mut header, *b"SNAM", &zstring(description))?;
        }
        for master in &self.masters {
            push_field(&mut header, *b"MAST", &zstring(master))?;
            push_field(&mut header, *b"DATA", &[0; 8])?;
        }
        let plugin_header = WrittenRecord {
            signature: *b"TES4",
            form_id: 0,
            flags: if light { FLAG_LIGHT } else { 0 },
            // The header record carries no form version of its own.
            form_version: 0,
            subrecords: Vec::new(),
        };
        let mut file = record_header(&plugin_header, len_u32(header.len())?, 0);
        file.extend(header);
        file.extend(body);
        Ok(file)
    }
}

/// Puts `records` among the children of `cell` in `cells`, adding the cell
/// if it is not there yet.
fn add_to_cell(
    cells: &mut Vec<CellEntry>,
    cell: &WrittenRecord,
    children: Children,
    records: Vec<WrittenRecord>,
) {
    let index = cells
        .iter()
        .position(|known| known.cell.form_id == cell.form_id)
        .unwrap_or(cells.len());
    if index == cells.len() {
        cells.push(CellEntry::new(cell.clone()));
    }
    if let Some(entry) = cells.get_mut(index) {
        entry.add(children, records);
    }
}

/// Writes records and groups and counts them for the header.
struct Writer {
    format: Format,
    /// How many records and groups were written, for `HEDR`.
    count: u32,
}

impl Writer {
    /// One record with its header.
    fn record(&mut self, record: &WrittenRecord) -> Result<Vec<u8>, WriteError> {
        let mut data = Vec::new();
        for field in &record.subrecords {
            push_field(&mut data, field.signature, &field.data)?;
        }
        let version = if record.form_version == 0 {
            self.format.form_version
        } else {
            record.form_version
        };
        let written = WrittenRecord {
            flags: record.flags & !FLAG_COMPRESSED,
            ..record.clone()
        };
        let mut bytes = record_header(&written, len_u32(data.len())?, version);
        bytes.extend(data);
        self.count = self.count.saturating_add(1);
        Ok(bytes)
    }

    /// A group around `body`.
    fn group(
        &mut self,
        label: [u8; 4],
        group_type: u32,
        body: &[u8],
    ) -> Result<Vec<u8>, WriteError> {
        let size = len_u32(
            HEADER_LEN
                .checked_add(body.len())
                .ok_or(WriteError::TooLarge)?,
        )?;
        let mut bytes = Vec::with_capacity(HEADER_LEN + body.len());
        bytes.extend(GROUP);
        bytes.extend(size.to_le_bytes());
        bytes.extend(label);
        bytes.extend(group_type.to_le_bytes());
        // Time stamp, version control and unknown values.
        bytes.extend([0; 8]);
        bytes.extend(body);
        self.count = self.count.saturating_add(1);
        Ok(bytes)
    }

    /// A cell and the group of its children.
    fn cell(&mut self, cell: &CellEntry) -> Result<Vec<u8>, WriteError> {
        let label = cell.cell.form_id.to_le_bytes();
        let mut children = Vec::new();
        for (records, group_type) in [
            (&cell.persistent, group_type::CELL_PERSISTENT_CHILDREN),
            (&cell.temporary, group_type::CELL_TEMPORARY_CHILDREN),
        ] {
            if records.is_empty() {
                continue;
            }
            let mut body = Vec::new();
            for record in records {
                body.extend(self.record(record)?);
            }
            children.extend(self.group(label, group_type, &body)?);
        }
        let mut bytes = self.record(&cell.cell)?;
        if !children.is_empty() {
            bytes.extend(self.group(label, group_type::CELL_CHILDREN, &children)?);
        }
        Ok(bytes)
    }

    /// The top-level group of interior cells, in blocks and sub-blocks by
    /// the last two decimal digits of each cell's object number.
    fn interiors(&mut self, cells: &[CellEntry]) -> Result<Vec<u8>, WriteError> {
        let mut sorted: Vec<&CellEntry> = cells.iter().collect();
        let place = |cell: &CellEntry| {
            let number = cell.cell.form_id & 0x00FF_FFFF;
            (number % 10, (number / 10) % 10)
        };
        sorted.sort_by_key(|cell| (place(cell), cell.cell.form_id));
        let mut blocks = Vec::new();
        let mut index = 0;
        while let Some(first) = sorted.get(index) {
            let (block, _) = place(first);
            let mut sub_blocks = Vec::new();
            while let Some(next) = sorted.get(index).filter(|cell| place(cell).0 == block) {
                let (_, sub_block) = place(next);
                let mut body = Vec::new();
                while let Some(cell) = sorted.get(index).filter(|cell| place(cell) == place(next)) {
                    body.extend(self.cell(cell)?);
                    index += 1;
                }
                sub_blocks.extend(self.group(
                    sub_block.to_le_bytes(),
                    group_type::INTERIOR_SUB_BLOCK,
                    &body,
                )?);
            }
            blocks.extend(self.group(
                block.to_le_bytes(),
                group_type::INTERIOR_BLOCK,
                &sub_blocks,
            )?);
        }
        self.group(*b"CELL", 0, &blocks)
    }

    /// A worldspace and the group of its cells: the persistent cell first,
    /// then the others in blocks of 32 and sub-blocks of 8 cells.
    fn world(&mut self, world: &WorldEntry) -> Result<Vec<u8>, WriteError> {
        let mut children = Vec::new();
        let mut placed = Vec::new();
        for cell in &world.cells {
            match cell.grid() {
                None => children.extend(self.cell(cell)?),
                Some((x, y)) => {
                    let block = grid_label(x.div_euclid(32), y.div_euclid(32))
                        .ok_or(WriteError::OutsideTheGrid(cell.cell.form_id))?;
                    let sub_block = grid_label(x.div_euclid(8), y.div_euclid(8))
                        .ok_or(WriteError::OutsideTheGrid(cell.cell.form_id))?;
                    placed.push((block, sub_block, cell));
                }
            }
        }
        placed.sort_by_key(|(block, sub_block, cell)| (*block, *sub_block, cell.cell.form_id));
        let mut index = 0;
        while let Some(&(block, _, _)) = placed.get(index) {
            let mut sub_blocks = Vec::new();
            while let Some(&(_, sub_block, _)) = placed.get(index).filter(|entry| entry.0 == block)
            {
                let mut body = Vec::new();
                while let Some((_, _, cell)) = placed
                    .get(index)
                    .filter(|entry| entry.0 == block && entry.1 == sub_block)
                {
                    body.extend(self.cell(cell)?);
                    index += 1;
                }
                sub_blocks.extend(self.group(
                    sub_block.to_le_bytes(),
                    group_type::EXTERIOR_SUB_BLOCK,
                    &body,
                )?);
            }
            children.extend(self.group(
                block.to_le_bytes(),
                group_type::EXTERIOR_BLOCK,
                &sub_blocks,
            )?);
        }
        let mut bytes = self.record(&world.world)?;
        if !children.is_empty() {
            bytes.extend(self.group(
                world.world.form_id.to_le_bytes(),
                group_type::WORLD_CHILDREN,
                &children,
            )?);
        }
        Ok(bytes)
    }
}

/// The label of an outside block or sub-block at `x`, `y`: y first, then
/// x, sixteen bits each. `None` beyond sixteen bits.
fn grid_label(x: i32, y: i32) -> Option<u32> {
    let [y0, y1] = i16::try_from(y).ok()?.to_le_bytes();
    let [x0, x1] = i16::try_from(x).ok()?.to_le_bytes();
    Some(u32::from_le_bytes([y0, y1, x0, x1]))
}

/// The header of a record whose data is `size` bytes long.
fn record_header(record: &WrittenRecord, size: u32, version: u16) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(HEADER_LEN);
    bytes.extend(record.signature);
    bytes.extend(size.to_le_bytes());
    bytes.extend(record.flags.to_le_bytes());
    bytes.extend(record.form_id.to_le_bytes());
    // Version control data.
    bytes.extend([0; 4]);
    bytes.extend(version.to_le_bytes());
    bytes.extend([0; 2]);
    bytes
}

/// Appends one field; a field larger than 64 KiB is announced by an `XXXX`
/// field with its size.
fn push_field(data: &mut Vec<u8>, signature: Signature, content: &[u8]) -> Result<(), WriteError> {
    let size = match u16::try_from(content.len()) {
        Ok(size) if content.len() <= MAX_FIELD => size,
        _ => {
            data.extend(b"XXXX");
            data.extend(4u16.to_le_bytes());
            data.extend(len_u32(content.len())?.to_le_bytes());
            0
        }
    };
    data.extend(signature);
    data.extend(size.to_le_bytes());
    data.extend(content);
    Ok(())
}

fn len_u32(len: usize) -> Result<u32, WriteError> {
    u32::try_from(len).map_err(|_| WriteError::TooLarge)
}

/// Text followed by the NUL byte that ends it.
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

    use proptest::prelude::*;

    use super::*;
    use crate::plugin::{GroupLabel, Request, read_request, read_request_versions};

    fn field(signature: [u8; 4], data: Vec<u8>) -> Subrecord {
        Subrecord { signature, data }
    }

    fn record(signature: [u8; 4], form_id: u32, fields: Vec<Subrecord>) -> WrittenRecord {
        WrittenRecord {
            signature,
            form_id,
            flags: 0,
            form_version: 0,
            subrecords: fields,
        }
    }

    fn grid(x: i32, y: i32) -> Vec<u8> {
        let mut data = x.to_le_bytes().to_vec();
        data.extend(y.to_le_bytes());
        data.extend(0u32.to_le_bytes());
        data
    }

    fn placed(form_id: u32, x: f32) -> WrittenRecord {
        let mut data = x.to_le_bytes().to_vec();
        data.extend([0; 20]);
        record(
            *b"REFR",
            form_id,
            vec![
                field(*b"NAME", 0x0001_0000u32.to_le_bytes().to_vec()),
                field(*b"DATA", data),
            ],
        )
    }

    const EVERYTHING: Request<'static> = Request {
        flat: &[*b"KYWD"],
        nested: &[*b"CELL", *b"WRLD"],
        nested_records: &[*b"REFR", *b"CELL", *b"WRLD", *b"NAVM"],
        nested_overrides: &[],
    };

    fn sample() -> PluginBuilder {
        let mut builder = PluginBuilder::new(vec!["Skyrim.esm".to_owned(), "Town.esp".to_owned()])
            .description("moves");
        let world = record(
            *b"WRLD",
            0x0000_003C,
            vec![field(*b"EDID", b"Tamriel\0".to_vec())],
        );
        // A cell west and south of the origin: blocks are floored.
        let outside = record(*b"CELL", 0x0000_9001, vec![field(*b"XCLC", grid(-1, -9))]);
        let persistent = record(*b"CELL", 0x0000_0D74, vec![]);
        let room = record(
            *b"CELL",
            0x0001_2347,
            vec![field(*b"EDID", b"Room\0".to_vec())],
        );
        builder.add_to_exterior(
            &world,
            &outside,
            Children::Temporary,
            vec![placed(0x0100_0812, 1.0)],
        );
        builder.add_to_exterior(
            &world,
            &outside,
            Children::Temporary,
            vec![record(
                *b"NAVM",
                0x0000_7777,
                vec![field(*b"NVNM", vec![1, 2, 3])],
            )],
        );
        builder.add_to_exterior(
            &world,
            &persistent,
            Children::Persistent,
            vec![placed(0x0000_0900, 2.0)],
        );
        builder.add_to_interior(&room, Children::Temporary, vec![placed(0x0100_0813, 3.0)]);
        // The same object again replaces the first copy.
        builder.add_to_interior(&room, Children::Temporary, vec![placed(0x0100_0813, 4.0)]);
        builder
    }

    fn groups_of(records: &[crate::plugin::Record], form_id: u32) -> Vec<(u32, u32)> {
        records
            .iter()
            .find(|record| record.form_id == form_id)
            .unwrap()
            .groups
            .iter()
            .map(|GroupLabel { group_type, label }| (*group_type, *label))
            .collect()
    }

    #[test]
    fn a_written_plugin_reads_back_with_its_nesting() {
        let bytes = sample().write(&Format::SKYRIM_SE).unwrap();
        let plugin = read_request(&mut Cursor::new(&bytes), &EVERYTHING).unwrap();
        assert_eq!(plugin.damaged, 0);
        assert_eq!(plugin.header.masters, ["Skyrim.esm", "Town.esp"]);
        assert!(plugin.header.is_light);
        assert!(!plugin.header.is_master);

        let found: Vec<_> = plugin
            .records
            .iter()
            .map(|record| (record.signature, record.form_id))
            .collect();
        assert_eq!(
            found,
            [
                (*b"CELL", 0x0001_2347),
                (*b"REFR", 0x0100_0813),
                (*b"WRLD", 0x0000_003C),
                (*b"CELL", 0x0000_0D74),
                (*b"REFR", 0x0000_0900),
                (*b"CELL", 0x0000_9001),
                (*b"REFR", 0x0100_0812),
                (*b"NAVM", 0x0000_7777),
            ]
        );

        // 0x12347 is 74567: block 7, sub-block 6.
        assert_eq!(
            groups_of(&plugin.records, 0x0100_0813),
            [
                (group_type::INTERIOR_BLOCK, 7),
                (group_type::INTERIOR_SUB_BLOCK, 6),
                (group_type::CELL_CHILDREN, 0x0001_2347),
                (group_type::CELL_TEMPORARY_CHILDREN, 0x0001_2347),
            ]
        );
        // The persistent cell lies in the worldspace's children directly.
        assert_eq!(
            groups_of(&plugin.records, 0x0000_0900),
            [
                (group_type::WORLD_CHILDREN, 0x3C),
                (group_type::CELL_CHILDREN, 0x0D74),
                (group_type::CELL_PERSISTENT_CHILDREN, 0x0D74),
            ]
        );
        // Cell (-1, -9): block (-1, -1), sub-block (-1, -2), labelled y, x.
        let label = |x: i16, y: i16| {
            let [y0, y1] = y.to_le_bytes();
            let [x0, x1] = x.to_le_bytes();
            u32::from_le_bytes([y0, y1, x0, x1])
        };
        let navmesh_groups = [
            (group_type::WORLD_CHILDREN, 0x3C),
            (group_type::EXTERIOR_BLOCK, label(-1, -1)),
            (group_type::EXTERIOR_SUB_BLOCK, label(-1, -2)),
            (group_type::CELL_CHILDREN, 0x9001),
            (group_type::CELL_TEMPORARY_CHILDREN, 0x9001),
        ];
        assert_eq!(groups_of(&plugin.records, 0x0100_0812), navmesh_groups);
        assert_eq!(groups_of(&plugin.records, 0x0000_7777), navmesh_groups);

        let moved = plugin
            .records
            .iter()
            .find(|record| record.form_id == 0x0100_0813)
            .unwrap();
        assert_eq!(moved.first(b"DATA").unwrap()[..4], 4.0f32.to_le_bytes());
    }

    #[test]
    fn the_header_counts_records_and_groups() {
        let bytes = sample().write(&Format::SKYRIM_SE).unwrap();
        // HEDR follows the 24-byte header and its own 6-byte field header.
        let count = u32::from_le_bytes(bytes[34..38].try_into().unwrap());
        // 8 records (3 cells, 1 world, 4 children) and 13 groups:
        // CELL, block, sub-block, children, temporary (5); WRLD, world
        // children, persistent cell's children and persistent (4); block,
        // sub-block, children, temporary (4).
        assert_eq!(count, 8 + 13);
        assert_eq!(f32::from_le_bytes(bytes[30..34].try_into().unwrap()), 1.71);
    }

    #[test]
    fn form_versions_are_kept_or_the_formats_written() {
        let mut builder = PluginBuilder::new(vec!["Skyrim.esm".to_owned()]);
        let mut old = placed(0x0000_0900, 1.0);
        old.form_version = 43;
        let room = record(*b"CELL", 0x0000_1234, vec![]);
        builder.add_to_interior(&room, Children::Persistent, vec![old]);
        let bytes = builder.write(&Format::SKYRIM_SE).unwrap();
        let mut versions = Vec::new();
        read_request_versions(
            &mut Cursor::new(&bytes),
            &EVERYTHING,
            &|_| true,
            &mut |_, record, version| versions.push((record.signature, version)),
        )
        .unwrap();
        assert_eq!(versions, [(*b"CELL", 44), (*b"REFR", 43)]);
    }

    #[test]
    fn records_outside_the_places_go_into_groups_of_their_own() {
        let mut builder = PluginBuilder::new(vec![]);
        builder.add_top_level(record(
            *b"KYWD",
            0x801,
            vec![field(*b"EDID", b"A\0".to_vec())],
        ));
        builder.add_top_level(record(*b"KYWD", 0x802, vec![]));
        let bytes = builder.write(&Format::SKYRIM_SE).unwrap();
        let plugin = read_request(&mut Cursor::new(&bytes), &EVERYTHING).unwrap();
        let ids: Vec<_> = plugin.records.iter().map(|record| record.form_id).collect();
        assert_eq!(ids, [0x801, 0x802]);
        // New records numbered within the light range still fit one.
        assert!(plugin.header.is_light);
    }

    #[test]
    fn too_many_own_records_or_numbers_beyond_the_range_are_not_light() {
        let mut builder = PluginBuilder::new(vec![]);
        builder.add_top_level(record(*b"KYWD", 0x1000, vec![]));
        assert!(!builder.fits_light(&Format::SKYRIM_SE));
        let bytes = builder.write(&Format::SKYRIM_SE).unwrap();
        let plugin = read_request(&mut Cursor::new(&bytes), &EVERYTHING).unwrap();
        assert!(!plugin.header.is_light);
        // A game without light plugins never gets the flag.
        let mut small = PluginBuilder::new(vec![]);
        small.add_top_level(record(*b"KYWD", 0x801, vec![]));
        let without = Format {
            light_plugins: false,
            ..Format::SKYRIM_SE
        };
        assert!(!small.fits_light(&without));
    }

    #[test]
    fn a_form_id_beyond_the_masters_is_refused() {
        let mut builder = PluginBuilder::new(vec!["Skyrim.esm".to_owned()]);
        let room = record(*b"CELL", 0x0000_1234, vec![]);
        builder.add_to_interior(&room, Children::Temporary, vec![placed(0x0500_0001, 1.0)]);
        assert_eq!(
            builder.write(&Format::SKYRIM_SE),
            Err(WriteError::UnknownMaster(0x0500_0001))
        );
    }

    #[test]
    fn large_fields_are_announced_and_read_back_whole() {
        let big = vec![9u8; 70_000];
        let mut builder = PluginBuilder::new(vec![]);
        builder.add_top_level(record(*b"KYWD", 0x801, vec![field(*b"DATA", big.clone())]));
        let bytes = builder.write(&Format::SKYRIM_SE).unwrap();
        let plugin = read_request(&mut Cursor::new(&bytes), &EVERYTHING).unwrap();
        assert_eq!(plugin.records[0].first(b"DATA"), Some(big.as_slice()));
    }

    #[test]
    fn compressed_records_are_written_plain() {
        let mut builder = PluginBuilder::new(vec![]);
        let mut packed = record(*b"KYWD", 0x801, vec![field(*b"EDID", b"A\0".to_vec())]);
        packed.flags = FLAG_COMPRESSED | 0x20;
        builder.add_top_level(packed);
        let bytes = builder.write(&Format::SKYRIM_SE).unwrap();
        let plugin = read_request(&mut Cursor::new(&bytes), &EVERYTHING).unwrap();
        assert_eq!(plugin.records[0].flags, 0x20);
        assert_eq!(plugin.records[0].first(b"EDID"), Some(b"A\0".as_slice()));
    }

    fn any_record(signature: [u8; 4]) -> impl Strategy<Value = WrittenRecord> {
        (
            0u32..0x0400_0000,
            any::<u32>(),
            proptest::collection::vec(proptest::collection::vec(any::<u8>(), 0..40), 0..4),
        )
            .prop_map(move |(form_id, flags, fields)| WrittenRecord {
                signature,
                form_id,
                flags,
                form_version: 0,
                subrecords: fields
                    .into_iter()
                    .map(|data| Subrecord {
                        signature: *b"DATA",
                        data,
                    })
                    .collect(),
            })
    }

    proptest! {
        #[test]
        fn anything_written_reads_back_undamaged(
            masters in 0usize..6,
            cells in proptest::collection::vec(
                (any_record(*b"CELL"), any::<Option<(i16, i16)>>(), any::<bool>(),
                 proptest::collection::vec(any_record(*b"REFR"), 0..4)),
                0..5,
            ),
        ) {
            let masters: Vec<String> = (0..masters).map(|index| format!("M{index}.esm")).collect();
            let mut builder = PluginBuilder::new(masters);
            let world = record(*b"WRLD", 0x3C, vec![]);
            for (mut cell, at, inside, children) in cells {
                if let Some((x, y)) = at {
                    cell.subrecords.push(field(*b"XCLC", grid(i32::from(x), i32::from(y))));
                }
                if inside {
                    builder.add_to_interior(&cell, Children::Temporary, children);
                } else {
                    builder.add_to_exterior(&world, &cell, Children::Persistent, children);
                }
            }
            match builder.write(&Format::SKYRIM_SE) {
                Ok(bytes) => {
                    let plugin = read_request(&mut Cursor::new(&bytes), &EVERYTHING).unwrap();
                    prop_assert_eq!(plugin.damaged, 0);
                }
                Err(WriteError::UnknownMaster(_)) => {}
                Err(other) => prop_assert!(false, "{other}"),
            }
        }
    }
}
