//! Builds small plugin and archive files for tests.
//!
//! Mod Mallet's tests may not use real game or mod files (ADR 0017), so they
//! build what they need with these functions. The results contain exactly
//! what is passed in and nothing else.
//!
//! The plugins are structurally valid. The archives hold file names and
//! the contents passed in, but no name hashes, which Mod Mallet does not
//! read; the game would not load them.

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

/// How long the header of each record and group is in a plugin.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Headers {
    /// 24 bytes, as from Fallout 3 on.
    #[default]
    Long,
    /// 20 bytes, as in Oblivion: without the last four bytes.
    Short,
}

impl Headers {
    fn len(self) -> usize {
        match self {
            Self::Long => 24,
            Self::Short => 20,
        }
    }
}

/// A plugin with the given masters, header flags and top-level groups.
pub fn plugin(masters: &[&str], flags: u32, groups: &[(Signature, Vec<RecordSpec>)]) -> Vec<u8> {
    plugin_in(Headers::Long, masters, flags, groups)
}

fn plugin_in(
    headers: Headers,
    masters: &[&str],
    flags: u32,
    groups: &[(Signature, Vec<RecordSpec>)],
) -> Vec<u8> {
    let record_count: usize = groups.iter().map(|(_, records)| records.len()).sum();
    let groups: Vec<Vec<u8>> = groups
        .iter()
        .map(|(label, records)| {
            let body: Vec<u8> = records
                .iter()
                .flat_map(|spec| record_in(headers, spec))
                .collect();
            group_in(headers, *label, 0, &body)
        })
        .collect();
    plugin_with_groups(headers, masters, flags, record_count, &groups)
}

/// A plugin whose top-level groups are given as they are written, for
/// groups that contain nested groups. See [`interior_cells`] and
/// [`worldspace`].
pub fn plugin_with_raw_groups(masters: &[&str], flags: u32, groups: &[Vec<u8>]) -> Vec<u8> {
    plugin_with_groups(Headers::Long, masters, flags, 0, groups)
}

/// The top-level group of interior cells, holding each cell with the
/// objects placed in it.
pub fn interior_cells(cells: &[(RecordSpec, Vec<RecordSpec>)]) -> Vec<u8> {
    interior_cells_in(Headers::Long, cells)
}

fn interior_cells_in(headers: Headers, cells: &[(RecordSpec, Vec<RecordSpec>)]) -> Vec<u8> {
    let mut sub_block = Vec::new();
    for (cell, placed) in cells {
        sub_block.extend(record_in(headers, cell));
        sub_block.extend(cell_children(headers, cell.form_id, placed));
    }
    let block = group_in(headers, 0u32.to_le_bytes(), 3, &sub_block);
    let blocks = group_in(headers, 0u32.to_le_bytes(), 2, &block);
    group_in(headers, *b"CELL", 0, &blocks)
}

/// The top-level group of worldspaces, holding one worldspace with its
/// exterior cells and the objects placed in them.
pub fn worldspace(world: &RecordSpec, cells: &[(RecordSpec, Vec<RecordSpec>)]) -> Vec<u8> {
    worldspace_in(Headers::Long, world, cells)
}

fn worldspace_in(
    headers: Headers,
    world: &RecordSpec,
    cells: &[(RecordSpec, Vec<RecordSpec>)],
) -> Vec<u8> {
    let mut sub_block = Vec::new();
    for (cell, placed) in cells {
        sub_block.extend(record_in(headers, cell));
        sub_block.extend(cell_children(headers, cell.form_id, placed));
    }
    let block = group_in(
        headers,
        0u32.to_le_bytes(),
        4,
        &group_in(headers, 0u32.to_le_bytes(), 5, &sub_block),
    );
    let mut body = record_in(headers, world);
    body.extend(group_in(headers, world.form_id.to_le_bytes(), 1, &block));
    group_in(headers, *b"WRLD", 0, &body)
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
fn cell_children(headers: Headers, cell: u32, placed: &[RecordSpec]) -> Vec<u8> {
    let body: Vec<u8> = placed
        .iter()
        .flat_map(|spec| record_in(headers, spec))
        .collect();
    let temporary = group_in(headers, cell.to_le_bytes(), 9, &body);
    group_in(headers, cell.to_le_bytes(), 6, &temporary)
}

fn plugin_with_groups(
    headers: Headers,
    masters: &[&str],
    flags: u32,
    record_count: usize,
    groups: &[Vec<u8>],
) -> Vec<u8> {
    let mut header_data = Vec::new();
    // HEDR: format version, number of records, next free object id.
    let version: f32 = match headers {
        Headers::Long => 1.71,
        Headers::Short => 1.0,
    };
    let mut hedr = Vec::new();
    hedr.extend(version.to_le_bytes());
    hedr.extend(len_u32(record_count).to_le_bytes());
    hedr.extend(0x800u32.to_le_bytes());
    push_subrecord(&mut header_data, *b"HEDR", &hedr);
    for master in masters {
        push_subrecord(&mut header_data, *b"MAST", &zstring(master));
        push_subrecord(&mut header_data, *b"DATA", &[0; 8]);
    }

    let mut file = record_header(headers, *b"TES4", len_u32(header_data.len()), flags, 0);
    file.extend(header_data);
    for group in groups {
        file.extend(group);
    }
    file
}

/// A group with the given label and type around `body`.
pub fn group(label: Signature, group_type: u32, body: &[u8]) -> Vec<u8> {
    group_in(Headers::Long, label, group_type, body)
}

fn group_in(headers: Headers, label: Signature, group_type: u32, body: &[u8]) -> Vec<u8> {
    let header_len = headers.len();
    let mut bytes = Vec::with_capacity(header_len + body.len());
    bytes.extend(b"GRUP");
    bytes.extend(len_u32(header_len + body.len()).to_le_bytes());
    bytes.extend(label);
    bytes.extend(group_type.to_le_bytes());
    bytes.extend(std::iter::repeat_n(0u8, header_len.saturating_sub(16))); // stamp and version
    bytes.extend(body);
    bytes
}

/// One record as it appears inside a group.
pub fn record(spec: &RecordSpec) -> Vec<u8> {
    record_in(Headers::Long, spec)
}

fn record_in(headers: Headers, spec: &RecordSpec) -> Vec<u8> {
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
    record_with_raw_data_in(headers, spec, &data)
}

/// A record whose data is written exactly as given, for building damaged
/// records.
pub fn record_with_raw_data(spec: &RecordSpec, data: &[u8]) -> Vec<u8> {
    record_with_raw_data_in(Headers::Long, spec, data)
}

fn record_with_raw_data_in(headers: Headers, spec: &RecordSpec, data: &[u8]) -> Vec<u8> {
    let mut bytes = record_header(
        headers,
        spec.signature,
        len_u32(data.len()),
        spec.flags,
        spec.form_id,
    );
    bytes.extend(data);
    bytes
}

/// The plugin builders again, for Oblivion, whose records and groups have
/// 20-byte headers.
pub mod oblivion {
    use super::{Headers, RecordSpec};
    use crate::plugin::Signature;

    /// Like [`super::plugin`].
    pub fn plugin(
        masters: &[&str],
        flags: u32,
        groups: &[(Signature, Vec<RecordSpec>)],
    ) -> Vec<u8> {
        super::plugin_in(Headers::Short, masters, flags, groups)
    }

    /// Like [`super::plugin_with_raw_groups`].
    pub fn plugin_with_raw_groups(masters: &[&str], flags: u32, groups: &[Vec<u8>]) -> Vec<u8> {
        super::plugin_with_groups(Headers::Short, masters, flags, 0, groups)
    }

    /// Like [`super::interior_cells`].
    pub fn interior_cells(cells: &[(RecordSpec, Vec<RecordSpec>)]) -> Vec<u8> {
        super::interior_cells_in(Headers::Short, cells)
    }

    /// Like [`super::worldspace`].
    pub fn worldspace(world: &RecordSpec, cells: &[(RecordSpec, Vec<RecordSpec>)]) -> Vec<u8> {
        super::worldspace_in(Headers::Short, world, cells)
    }

    /// Like [`super::group`].
    pub fn group(label: Signature, group_type: u32, body: &[u8]) -> Vec<u8> {
        super::group_in(Headers::Short, label, group_type, body)
    }

    /// Like [`super::record`].
    pub fn record(spec: &RecordSpec) -> Vec<u8> {
        super::record_in(Headers::Short, spec)
    }

    /// Like [`super::record_with_raw_data`].
    pub fn record_with_raw_data(spec: &RecordSpec, data: &[u8]) -> Vec<u8> {
        super::record_with_raw_data_in(Headers::Short, spec, data)
    }
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

fn record_header(
    headers: Headers,
    signature: Signature,
    data_len: u32,
    flags: u32,
    form_id: u32,
) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(headers.len());
    bytes.extend(signature);
    bytes.extend(data_len.to_le_bytes());
    bytes.extend(flags.to_le_bytes());
    bytes.extend(form_id.to_le_bytes());
    bytes.extend([0; 4]); // version control
    if headers == Headers::Long {
        bytes.extend(44u16.to_le_bytes()); // record format version
        bytes.extend([0; 2]);
    }
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

/// How [`ba2_with`] and [`ba2_textures`] store the files.
#[derive(Debug, Clone, Copy)]
pub struct Ba2Options {
    /// The archive version: 1 for Fallout 4, 7 or 8 after its next-gen
    /// update, 2 or 3 for Starfield.
    pub version: u32,
    /// Whether the files are compressed: with LZ4 in version 3, with zlib
    /// in the others.
    pub compressed: bool,
}

/// A texture to be written by [`ba2_textures`].
#[derive(Debug, Clone)]
pub struct Ba2Texture<'a> {
    /// The texture's path, `folder\file`.
    pub path: &'a str,
    /// Width of the largest level in pixels.
    pub width: u16,
    /// Height of the largest level in pixels.
    pub height: u16,
    /// Number of mipmap levels.
    pub mip_count: u8,
    /// The `DXGI_FORMAT` of the pixels, such as 71 for BC1.
    pub format: u8,
    /// Whether the texture is a cube of six faces.
    pub cubemap: bool,
    /// The pixel data, unpacked, split into the chunks the archive stores.
    pub chunks: Vec<Vec<u8>>,
}

const BA2_SENTINEL: u32 = 0xBAAD_F00D;

/// A general archive of the Fallout 4 version that lists `files`, each
/// empty.
pub fn ba2(files: &[&str]) -> Vec<u8> {
    let empty: Vec<(&str, &[u8])> = files.iter().map(|&file| (file, [].as_slice())).collect();
    ba2_with(
        &empty,
        Ba2Options {
            version: 1,
            compressed: false,
        },
    )
}

/// A general archive that contains `files`, each given as path and
/// content.
pub fn ba2_with(files: &[(&str, &[u8])], options: Ba2Options) -> Vec<u8> {
    let header_len = ba2_header_len(options.version);
    let mut data_offset = header_len + files.len() * 36;
    let mut records = Vec::new();
    let mut contents = Vec::new();
    for (path, content) in files {
        let (packed, stored) = match ba2_pack(content, options) {
            Some(packed) => (len_u32(packed.len()), packed),
            None => (0, content.to_vec()),
        };
        records.extend([0; 4]); // name hash, not read by Mod Mallet
        records.extend(extension(path));
        records.extend([0; 4]); // folder hash
        records.extend(0u32.to_le_bytes()); // flags
        records.extend((data_offset as u64).to_le_bytes());
        records.extend(packed.to_le_bytes());
        records.extend(len_u32(content.len()).to_le_bytes());
        records.extend(BA2_SENTINEL.to_le_bytes());
        data_offset += stored.len();
        contents.extend(stored);
    }
    let paths: Vec<&str> = files.iter().map(|(path, _)| *path).collect();
    let mut bytes = ba2_header(options.version, *b"GNRL", files.len(), data_offset);
    bytes.extend(records);
    bytes.extend(contents);
    bytes.extend(ba2_names(&paths));
    bytes
}

/// A texture archive that contains `textures`.
pub fn ba2_textures(textures: &[Ba2Texture<'_>], options: Ba2Options) -> Vec<u8> {
    let header_len = ba2_header_len(options.version);
    let records_len: usize = textures
        .iter()
        .map(|texture| 24 + texture.chunks.len() * 24)
        .sum();
    let mut data_offset = header_len + records_len;
    let mut records = Vec::new();
    let mut contents = Vec::new();
    for texture in textures {
        records.extend([0; 4]); // name hash
        records.extend(*b"dds\0");
        records.extend([0; 4]); // folder hash
        records.push(0); // unknown
        records.push(u8::try_from(texture.chunks.len()).unwrap_or(u8::MAX));
        records.extend(24u16.to_le_bytes()); // size of a chunk record
        records.extend(texture.height.to_le_bytes());
        records.extend(texture.width.to_le_bytes());
        records.push(texture.mip_count);
        records.push(texture.format);
        records.push(u8::from(texture.cubemap));
        records.push(8); // tile mode
        for (index, chunk) in texture.chunks.iter().enumerate() {
            let (packed, stored) = match ba2_pack(chunk, options) {
                Some(packed) => (len_u32(packed.len()), packed),
                None => (0, chunk.clone()),
            };
            let mip = u16::try_from(index).unwrap_or(u16::MAX);
            records.extend((data_offset as u64).to_le_bytes());
            records.extend(packed.to_le_bytes());
            records.extend(len_u32(chunk.len()).to_le_bytes());
            records.extend(mip.to_le_bytes()); // first mipmap level
            records.extend(mip.to_le_bytes()); // last mipmap level
            records.extend(BA2_SENTINEL.to_le_bytes());
            data_offset += stored.len();
            contents.extend(stored);
        }
    }
    let paths: Vec<&str> = textures.iter().map(|texture| texture.path).collect();
    let mut bytes = ba2_header(options.version, *b"DX10", textures.len(), data_offset);
    bytes.extend(records);
    bytes.extend(contents);
    bytes.extend(ba2_names(&paths));
    bytes
}

fn ba2_header_len(version: u32) -> usize {
    match version {
        2 => 32,
        3 => 36,
        _ => 24,
    }
}

fn ba2_header(version: u32, kind: [u8; 4], file_count: usize, name_table: usize) -> Vec<u8> {
    let mut bytes = b"BTDX".to_vec();
    bytes.extend(version.to_le_bytes());
    bytes.extend(kind);
    bytes.extend(len_u32(file_count).to_le_bytes());
    bytes.extend((name_table as u64).to_le_bytes());
    if version == 2 || version == 3 {
        bytes.extend(1u32.to_le_bytes());
        bytes.extend(0u32.to_le_bytes());
    }
    if version == 3 {
        bytes.extend(3u32.to_le_bytes()); // LZ4
    }
    bytes
}

/// The stored form of `data` if the archive compresses it.
fn ba2_pack(data: &[u8], options: Ba2Options) -> Option<Vec<u8>> {
    match (options.compressed, options.version) {
        (false, _) => None,
        (true, 3) => Some(lz4_flex::block::compress(data)),
        (true, _) => Some(zlib(data)),
    }
}

/// The extension of a path as a BA2 record stores it: up to four bytes,
/// padded with NUL.
fn extension(path: &str) -> [u8; 4] {
    let mut bytes = [0; 4];
    let extension = path.rsplit_once('.').map_or("", |(_, extension)| extension);
    for (slot, byte) in bytes.iter_mut().zip(extension.bytes()) {
        *slot = byte;
    }
    bytes
}

fn ba2_names(paths: &[&str]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for path in paths {
        bytes.extend(u16::try_from(path.len()).unwrap_or(u16::MAX).to_le_bytes());
        bytes.extend(path.as_bytes());
    }
    bytes
}
