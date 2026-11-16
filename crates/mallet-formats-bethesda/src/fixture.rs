//! Builds small plugin and archive files for tests.
//!
//! Mod Mallet's tests may not use real game or mod files (ADR 0017), so they
//! build what they need with these functions. The results contain exactly
//! what is passed in and nothing else.
//!
//! The plugins are structurally valid. The archives are listings only: they
//! hold file names but no contents and no name hashes, which is all Mod
//! Mallet reads, but the game would not load them.

use std::io::Write;

use flate2::Compression;
use flate2::write::ZlibEncoder;

use crate::plugin::Signature;

const FLAG_COMPRESSED: u32 = 0x0004_0000;

/// A record to be written by [`plugin`] or [`record`].
#[derive(Debug, Clone)]
pub struct RecordSpec {
    /// The record's type, such as `NPC_`.
    pub signature: Signature,
    /// The record's form ID, relative to the plugin's masters.
    pub form_id: u32,
    /// The record's flags. The compression flag is set by
    /// [`RecordSpec::compressed`].
    pub flags: u32,
    /// The record's fields.
    pub subrecords: Vec<(Signature, Vec<u8>)>,
}

impl RecordSpec {
    /// A record without fields.
    pub fn new(signature: Signature, form_id: u32) -> Self {
        Self {
            signature,
            form_id,
            flags: 0,
            subrecords: Vec::new(),
        }
    }

    /// Adds a field.
    #[must_use]
    pub fn with(mut self, signature: Signature, data: Vec<u8>) -> Self {
        self.subrecords.push((signature, data));
        self
    }

    /// Stores the record's fields compressed, as the game's own files do
    /// for large records.
    #[must_use]
    pub fn compressed(mut self) -> Self {
        self.flags |= FLAG_COMPRESSED;
        self
    }
}

/// Text followed by the NUL byte that ends it in these formats.
pub fn zstring(text: &str) -> Vec<u8> {
    let mut bytes = text.as_bytes().to_vec();
    bytes.push(0);
    bytes
}

/// A plugin with the given masters, header flags and top-level groups.
pub fn plugin(masters: &[&str], flags: u32, groups: &[(Signature, Vec<RecordSpec>)]) -> Vec<u8> {
    let record_count: usize = groups.iter().map(|(_, records)| records.len()).sum();
    let groups: Vec<Vec<u8>> = groups
        .iter()
        .map(|(label, records)| {
            let body: Vec<u8> = records.iter().flat_map(record).collect();
            group(*label, 0, &body)
        })
        .collect();
    plugin_with_groups(masters, flags, record_count, &groups)
}

/// A plugin whose top-level groups are given as they are written, for
/// groups that contain nested groups. See [`interior_cells`] and
/// [`worldspace`].
pub fn plugin_with_raw_groups(masters: &[&str], flags: u32, groups: &[Vec<u8>]) -> Vec<u8> {
    plugin_with_groups(masters, flags, 0, groups)
}

/// The top-level group of interior cells, holding each cell with the
/// objects placed in it.
pub fn interior_cells(cells: &[(RecordSpec, Vec<RecordSpec>)]) -> Vec<u8> {
    let mut sub_block = Vec::new();
    for (cell, placed) in cells {
        sub_block.extend(record(cell));
        sub_block.extend(cell_children(cell.form_id, placed));
    }
    let block = group(0u32.to_le_bytes(), 3, &sub_block);
    group(*b"CELL", 0, &group(0u32.to_le_bytes(), 2, &block))
}

/// The top-level group of worldspaces, holding one worldspace with its
/// exterior cells and the objects placed in them.
pub fn worldspace(world: &RecordSpec, cells: &[(RecordSpec, Vec<RecordSpec>)]) -> Vec<u8> {
    let mut sub_block = Vec::new();
    for (cell, placed) in cells {
        sub_block.extend(record(cell));
        sub_block.extend(cell_children(cell.form_id, placed));
    }
    let block = group(
        0u32.to_le_bytes(),
        4,
        &group(0u32.to_le_bytes(), 5, &sub_block),
    );
    let mut body = record(world);
    body.extend(group(world.form_id.to_le_bytes(), 1, &block));
    group(*b"WRLD", 0, &body)
}

/// A placed object (`REFR`): the base object `base` at `position`, turned
/// by `rotation` (radians about x, y and z), at `scale` if given.
pub fn reference(
    form_id: u32,
    base: u32,
    position: [f32; 3],
    rotation: [f32; 3],
    scale: Option<f32>,
) -> RecordSpec {
    let data: Vec<u8> = position
        .iter()
        .chain(&rotation)
        .flat_map(|value| value.to_le_bytes())
        .collect();
    let spec = RecordSpec::new(*b"REFR", form_id)
        .with(*b"NAME", base.to_le_bytes().to_vec())
        .with(*b"DATA", data);
    match scale {
        Some(scale) => spec.with(*b"XSCL", scale.to_le_bytes().to_vec()),
        None => spec,
    }
}

/// The groups that hold what a cell contains, all of it temporary.
fn cell_children(cell: u32, placed: &[RecordSpec]) -> Vec<u8> {
    let body: Vec<u8> = placed.iter().flat_map(record).collect();
    group(cell.to_le_bytes(), 6, &group(cell.to_le_bytes(), 9, &body))
}

fn plugin_with_groups(
    masters: &[&str],
    flags: u32,
    record_count: usize,
    groups: &[Vec<u8>],
) -> Vec<u8> {
    let mut header_data = Vec::new();
    // HEDR: format version, number of records, next free object id.
    let mut hedr = Vec::new();
    hedr.extend(1.71f32.to_le_bytes());
    hedr.extend(len_u32(record_count).to_le_bytes());
    hedr.extend(0x800u32.to_le_bytes());
    push_subrecord(&mut header_data, *b"HEDR", &hedr);
    for master in masters {
        push_subrecord(&mut header_data, *b"MAST", &zstring(master));
        push_subrecord(&mut header_data, *b"DATA", &[0; 8]);
    }

    let mut file = record_header(*b"TES4", len_u32(header_data.len()), flags, 0);
    file.extend(header_data);
    for group in groups {
        file.extend(group);
    }
    file
}

/// A group with the given label and type around `body`.
pub fn group(label: Signature, group_type: u32, body: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(24 + body.len());
    bytes.extend(b"GRUP");
    bytes.extend(len_u32(24 + body.len()).to_le_bytes());
    bytes.extend(label);
    bytes.extend(group_type.to_le_bytes());
    bytes.extend([0; 8]);
    bytes.extend(body);
    bytes
}

/// One record as it appears inside a group.
pub fn record(spec: &RecordSpec) -> Vec<u8> {
    let mut fields = Vec::new();
    for (signature, data) in &spec.subrecords {
        push_subrecord(&mut fields, *signature, data);
    }
    let data = if spec.flags & FLAG_COMPRESSED == 0 {
        fields
    } else {
        let mut packed = len_u32(fields.len()).to_le_bytes().to_vec();
        packed.extend(zlib(&fields));
        packed
    };
    record_with_raw_data(spec, &data)
}

/// A record whose data is written exactly as given, for building damaged
/// records.
pub fn record_with_raw_data(spec: &RecordSpec, data: &[u8]) -> Vec<u8> {
    let mut bytes = record_header(
        spec.signature,
        len_u32(data.len()),
        spec.flags,
        spec.form_id,
    );
    bytes.extend(data);
    bytes
}

/// An archive of the Skyrim Special Edition version that lists `files`.
///
/// Each entry is `folder\file`; `/` works as a separator too.
pub fn bsa(files: &[&str]) -> Vec<u8> {
    bsa_with_version(files, 105)
}

/// Like [`bsa`], for another archive version.
pub fn bsa_with_version(files: &[&str], version: u32) -> Vec<u8> {
    let empty: Vec<(&str, &[u8])> = files.iter().map(|&file| (file, [].as_slice())).collect();
    bsa_with(
        &empty,
        BsaOptions {
            version,
            compressed: false,
            embedded_names: false,
        },
    )
}

/// A file of an archive: its name and its data as stored in the archive.
type StoredFile = (String, Vec<u8>);

/// How [`bsa_with`] stores the files.
#[derive(Debug, Clone, Copy)]
pub struct BsaOptions {
    /// The archive version: 104 for Skyrim, 105 for Skyrim Special Edition.
    pub version: u32,
    /// Whether the files are compressed.
    pub compressed: bool,
    /// Whether each file's data starts with a copy of its path, as the
    /// game's own texture archives have it.
    pub embedded_names: bool,
}

/// An archive that contains `files`, each given as path and content.
pub fn bsa_with(files: &[(&str, &[u8])], options: BsaOptions) -> Vec<u8> {
    let BsaOptions {
        version,
        compressed,
        embedded_names,
    } = options;
    let folder_record_len = if version == 105 { 24 } else { 16 };

    // Group the files by folder, keeping the order of first appearance.
    // Each file becomes its name and its data as stored in the archive.
    let mut folders: Vec<(String, Vec<StoredFile>)> = Vec::new();
    for (path, content) in files {
        let normalized = path.replace('/', "\\");
        let (folder, name) = normalized.rsplit_once('\\').unwrap_or(("", &normalized));

        let mut stored = Vec::new();
        if embedded_names {
            stored.push(u8::try_from(normalized.len()).unwrap_or(u8::MAX));
            stored.extend(normalized.as_bytes());
        }
        if compressed {
            stored.extend(len_u32(content.len()).to_le_bytes());
            stored.extend(if version == 105 {
                lz4_frame(content)
            } else {
                zlib(content)
            });
        } else {
            stored.extend(*content);
        }

        let file = (name.to_owned(), stored);
        match folders.iter_mut().find(|(existing, _)| existing == folder) {
            Some((_, folder_files)) => folder_files.push(file),
            None => folders.push((folder.to_owned(), vec![file])),
        }
    }

    let folder_names_len: usize = folders.iter().map(|(folder, _)| folder.len() + 1).sum();
    let file_names_len: usize = folders
        .iter()
        .flat_map(|(_, folder_files)| folder_files)
        .map(|(name, _)| name.len() + 1)
        .sum();
    let directory_len = 36
        + folders.len() * folder_record_len
        + folder_names_len
        + folders.len()
        + files.len() * 16
        + file_names_len;

    let mut flags = 0x3u32; // folder names and file names are stored
    if compressed {
        flags |= 0x4;
    }
    if embedded_names {
        flags |= 0x100;
    }

    let mut bytes = Vec::new();
    bytes.extend(b"BSA\0");
    bytes.extend(version.to_le_bytes());
    bytes.extend(36u32.to_le_bytes());
    bytes.extend(flags.to_le_bytes());
    bytes.extend(len_u32(folders.len()).to_le_bytes());
    bytes.extend(len_u32(files.len()).to_le_bytes());
    bytes.extend(len_u32(folder_names_len).to_le_bytes());
    bytes.extend(len_u32(file_names_len).to_le_bytes());
    bytes.extend(0u32.to_le_bytes());

    for (_, folder_files) in &folders {
        bytes.extend([0; 8]); // name hash, not read by Mod Mallet
        bytes.extend(len_u32(folder_files.len()).to_le_bytes());
        bytes.extend(std::iter::repeat_n(0u8, folder_record_len - 12)); // offset
    }
    let mut data_offset = directory_len;
    for (folder, folder_files) in &folders {
        bytes.push(u8::try_from(folder.len() + 1).unwrap_or(u8::MAX));
        bytes.extend(zstring(folder));
        for (_, stored) in folder_files {
            bytes.extend([0; 8]); // name hash
            bytes.extend(len_u32(stored.len()).to_le_bytes());
            bytes.extend(len_u32(data_offset).to_le_bytes());
            data_offset += stored.len();
        }
    }
    for (name, _) in folders.iter().flat_map(|(_, folder_files)| folder_files) {
        bytes.extend(zstring(name));
    }
    for (_, stored) in folders.iter().flat_map(|(_, folder_files)| folder_files) {
        bytes.extend(stored);
    }
    bytes
}

/// A mesh of the Skyrim Special Edition version with one textured surface
/// per entry of `texture_sets`. Each entry lists the texture paths of a
/// surface; an empty path is an unused slot.
pub fn nif(texture_sets: &[&[&str]]) -> Vec<u8> {
    nif_with_padding(texture_sets, 16)
}

/// Like [`nif`], with `padding` bytes of other data in front of the texture
/// sets, the way geometry precedes them in a real mesh.
pub fn nif_with_padding(texture_sets: &[&[&str]], padding: usize) -> Vec<u8> {
    let sized = |text: &str| {
        let mut bytes = len_u32(text.len()).to_le_bytes().to_vec();
        bytes.extend(text.as_bytes());
        bytes
    };

    // One block that is not a texture set, as every real mesh has.
    let mut blocks: Vec<(u16, Vec<u8>)> = vec![(0, vec![0xAA; padding])];
    for set in texture_sets {
        let mut block = len_u32(set.len()).to_le_bytes().to_vec();
        for path in *set {
            block.extend(sized(path));
        }
        blocks.push((1, block));
    }
    let block_types: &[&str] = if texture_sets.is_empty() {
        &["BSFadeNode"]
    } else {
        &["BSFadeNode", "BSShaderTextureSet"]
    };

    let mut bytes = b"Gamebryo File Format, Version 20.2.0.7\n".to_vec();
    bytes.extend(0x1402_0007u32.to_le_bytes());
    bytes.push(1); // little endian
    bytes.extend(12u32.to_le_bytes()); // user version
    bytes.extend(len_u32(blocks.len()).to_le_bytes());
    bytes.extend(100u32.to_le_bytes()); // Bethesda version: Skyrim Special Edition
    for _ in 0..3 {
        bytes.extend([1, 0]); // author and two scripts: empty, NUL-terminated
    }
    bytes.extend(
        u16::try_from(block_types.len())
            .unwrap_or(u16::MAX)
            .to_le_bytes(),
    );
    for block_type in block_types {
        bytes.extend(sized(block_type));
    }
    for (block_type, _) in &blocks {
        bytes.extend(block_type.to_le_bytes());
    }
    for (_, block) in &blocks {
        bytes.extend(len_u32(block.len()).to_le_bytes());
    }
    bytes.extend(0u32.to_le_bytes()); // number of strings
    bytes.extend(0u32.to_le_bytes()); // longest string
    bytes.extend(0u32.to_le_bytes()); // number of groups
    for (_, block) in &blocks {
        bytes.extend(block);
    }
    bytes
}

fn lz4_frame(data: &[u8]) -> Vec<u8> {
    let mut encoder = lz4_flex::frame::FrameEncoder::new(Vec::new());
    // Writing into a `Vec` cannot fail.
    let _ = encoder.write_all(data);
    encoder.finish().unwrap_or_default()
}

fn record_header(signature: Signature, data_len: u32, flags: u32, form_id: u32) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(24);
    bytes.extend(signature);
    bytes.extend(data_len.to_le_bytes());
    bytes.extend(flags.to_le_bytes());
    bytes.extend(form_id.to_le_bytes());
    bytes.extend([0; 4]); // version control
    bytes.extend(44u16.to_le_bytes()); // record format version
    bytes.extend([0; 2]);
    bytes
}

fn push_subrecord(target: &mut Vec<u8>, signature: Signature, data: &[u8]) {
    if let Ok(len) = u16::try_from(data.len()) {
        target.extend(signature);
        target.extend(len.to_le_bytes());
    } else {
        // Too large for the 16-bit size: announce the real size first.
        target.extend(b"XXXX");
        target.extend(4u16.to_le_bytes());
        target.extend(len_u32(data.len()).to_le_bytes());
        target.extend(signature);
        target.extend(0u16.to_le_bytes());
    }
    target.extend(data);
}

fn zlib(data: &[u8]) -> Vec<u8> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    // Writing into a `Vec` cannot fail.
    let _ = encoder.write_all(data);
    encoder.finish().unwrap_or_default()
}

/// Fixtures are tiny; a length that does not fit is clamped instead of
/// failing.
fn len_u32(len: usize) -> u32 {
    u32::try_from(len).unwrap_or(u32::MAX)
}
