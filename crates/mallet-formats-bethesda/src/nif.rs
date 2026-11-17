//! Reads which textures a mesh (`.nif`) uses.
//!
//! A mesh does not contain its textures. It names them by path, in blocks
//! of the type `BSShaderTextureSet`: one set per surface, with slots for
//! the colour map, the normal map and so on. Which file the game finds
//! under such a path is decided elsewhere, by loose files and archives.
//!
//! A mesh file is a header followed by blocks. The header lists the type
//! and the size of every block, so the reader can jump straight to the
//! texture sets and never has to understand geometry, collision or
//! animation:
//!
//! ```text
//! "Gamebryo File Format, Version 20.2.0.7\n"
//! version, endianness, user version, number of blocks
//! Bethesda version and three export strings
//! block type names
//! for each block: index of its type
//! for each block: its size
//! strings and groups
//! the blocks
//! ```
//!
//! Supported are the meshes of Fallout 3, New Vegas, Skyrim, Skyrim
//! Special Edition and Fallout 4. Oblivion's, of an older version that does
//! not say where its blocks lie, are read by a reader of their own, which
//! reads them block by block. The
//! layout was checked against real files; the reader takes nothing in a
//! file on trust (ADR 0013).

use std::io::{Read, Seek, SeekFrom};

use crate::{Bytes, decode_text};

const MAGIC: &[u8] = b"Gamebryo File Format, Version ";
const VERSION_20_2_0_7: u32 = 0x1402_0007;
const TEXTURE_SET: &str = "BSShaderTextureSet";

/// The newest Bethesda variant of the format this reader knows: Fallout 4
/// (130). Fallout 76 and Starfield changed the header again.
const NEWEST_BETHESDA_VERSION: u32 = 130;

/// From this Bethesda variant on (Fallout 4), the header names one more
/// thing after the export script: the longest file path.
const MAX_FILEPATH_VERSION: u32 = 103;

/// How many short texts follow the Bethesda version in the header: the
/// author, the process script and the export script, and the longest file
/// path in newer variants.
fn header_texts(bethesda_version: u32) -> usize {
    if bethesda_version >= MAX_FILEPATH_VERSION {
        4
    } else {
        3
    }
}

/// The largest Oblivion mesh that is read for its textures. The game's
/// largest is under 4 MB.
const MAX_OBLIVION_MESH: u64 = 64 * 1024 * 1024;

/// How much of a file is read at first to get its header.
const FIRST_READ: usize = 16 * 1024;

/// The largest texture set block that is read. A real one is a few hundred
/// bytes.
const MAX_TEXTURE_SET: usize = 1024 * 1024;

/// Why the textures of a mesh could not be read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum NifError {
    /// The file does not start like a mesh.
    #[error("the file is not a mesh")]
    NotAMesh,
    /// The mesh is in a version of the format this reader does not know.
    #[error("the mesh format version is not supported")]
    UnsupportedVersion,
    /// The data ends too early, or a size in it is wrong.
    #[error("the mesh is damaged or cut off")]
    Damaged,
}

/// The texture paths a mesh names, in the mesh's own spelling, without
/// empty slots and without duplicates.
pub fn texture_paths(data: &[u8]) -> Result<Vec<String>, NifError> {
    if crate::nif_oblivion::has_oblivion_version(data) {
        return crate::nif_oblivion::texture_paths(data);
    }
    let layout = parse_layout(data)?;
    let mut bytes = Bytes::new(data);
    bytes.skip(layout.blocks_start).ok_or(NifError::Damaged)?;

    let mut paths = Vec::new();
    for block in &layout.blocks {
        let data = bytes.take(block.size).ok_or(NifError::Damaged)?;
        if block.is_texture_set {
            add_texture_set(data, &mut paths)?;
        }
    }
    Ok(paths)
}

/// Like [`texture_paths`], for a mesh in a file: reads the header and the
/// texture blocks and leaves the rest of the file, which is nearly all of
/// it, on disk.
pub fn read_texture_paths<R: Read + Seek>(reader: &mut R) -> Result<Vec<String>, NifError> {
    let io = |_: std::io::Error| NifError::Damaged;

    // The header is usually a few kilobytes. If it turns out to be cut
    // off, more of the file is read until it is complete or the file ends.
    let mut start = Vec::new();
    let mut wanted = FIRST_READ;
    let layout = loop {
        let missing = wanted.saturating_sub(start.len());
        let read = reader
            .by_ref()
            .take(u64::try_from(missing).unwrap_or(u64::MAX))
            .read_to_end(&mut start)
            .map_err(io)?;
        match parse_layout(&start) {
            Err(NifError::Damaged) if read == missing => wanted = wanted.saturating_mul(4),
            // Oblivion's meshes do not say where their blocks lie; they
            // are read whole.
            Err(NifError::UnsupportedVersion)
                if crate::nif_oblivion::has_oblivion_version(&start) =>
            {
                reader
                    .by_ref()
                    .take(MAX_OBLIVION_MESH)
                    .read_to_end(&mut start)
                    .map_err(io)?;
                return crate::nif_oblivion::texture_paths(&start);
            }
            result => break result?,
        }
    };

    let mut paths = Vec::new();
    let mut offset = layout.blocks_start;
    for block in &layout.blocks {
        let end = offset.checked_add(block.size).ok_or(NifError::Damaged)?;
        if block.is_texture_set {
            if let Some(data) = start.get(offset..end) {
                add_texture_set(data, &mut paths)?;
            } else {
                if block.size > MAX_TEXTURE_SET {
                    return Err(NifError::Damaged);
                }
                let position = u64::try_from(offset).map_err(|_| NifError::Damaged)?;
                reader.seek(SeekFrom::Start(position)).map_err(io)?;
                let mut data = vec![0; block.size];
                reader.read_exact(&mut data).map_err(io)?;
                add_texture_set(&data, &mut paths)?;
            }
        }
        offset = end;
    }
    Ok(paths)
}

/// Where the blocks of a mesh lie.
struct Layout {
    /// Where the first block starts.
    blocks_start: usize,
    /// The blocks in file order.
    blocks: Vec<Block>,
}

struct Block {
    is_texture_set: bool,
    size: usize,
}

/// Reads the header of a mesh. Fails with [`NifError::Damaged`] if `data`
/// ends before the header does.
fn parse_layout(data: &[u8]) -> Result<Layout, NifError> {
    let mut bytes = Bytes::new(data);
    let damaged = || NifError::Damaged;

    // The header line ends with a line feed.
    let line_len = data
        .iter()
        .take(64)
        .position(|&byte| byte == b'\n')
        .ok_or(NifError::NotAMesh)?;
    let line = bytes.take(line_len + 1).ok_or(NifError::NotAMesh)?;
    if !line.starts_with(MAGIC) {
        return Err(NifError::NotAMesh);
    }

    let version = bytes.u32().ok_or_else(damaged)?;
    let little_endian = bytes.u8().ok_or_else(damaged)? == 1;
    let _user_version = bytes.u32().ok_or_else(damaged)?;
    let block_count = bytes.u32().ok_or_else(damaged)?;
    let bethesda_version = bytes.u32().ok_or_else(damaged)?;
    if version != VERSION_20_2_0_7 || !little_endian || bethesda_version > NEWEST_BETHESDA_VERSION {
        return Err(NifError::UnsupportedVersion);
    }
    // Author, process script, export script and perhaps the longest file
    // path: a length byte and text each.
    for _ in 0..header_texts(bethesda_version) {
        let len = usize::from(bytes.u8().ok_or_else(damaged)?);
        bytes.skip(len).ok_or_else(damaged)?;
    }

    let type_count = bytes.u16().ok_or_else(damaged)?;
    let mut texture_set_type = None;
    for index in 0..type_count {
        if sized_string(&mut bytes).ok_or_else(damaged)? == TEXTURE_SET.as_bytes() {
            texture_set_type = Some(index);
        }
    }

    // Reading stops at the first failure, so a block count that promises
    // too much cannot make these vectors grow beyond the data.
    let mut types = Vec::new();
    for _ in 0..block_count {
        types.push(bytes.u16().ok_or_else(damaged)?);
    }
    let mut blocks = Vec::new();
    for block_type in types {
        let size = bytes.u32().ok_or_else(damaged)?;
        blocks.push(Block {
            is_texture_set: Some(block_type) == texture_set_type,
            size: usize::try_from(size).map_err(|_| NifError::Damaged)?,
        });
    }

    let string_count = bytes.u32().ok_or_else(damaged)?;
    let _longest_string = bytes.u32().ok_or_else(damaged)?;
    for _ in 0..string_count {
        sized_string(&mut bytes).ok_or_else(damaged)?;
    }
    let group_count = bytes.u32().ok_or_else(damaged)?;
    for _ in 0..group_count {
        bytes.u32().ok_or_else(damaged)?;
    }

    Ok(Layout {
        blocks_start: data.len() - bytes.remaining(),
        blocks,
    })
}

/// One block of a mesh: its type and where its bytes lie in the file.
pub(crate) struct BlockInfo {
    /// The block type, such as `BSTriShape`.
    pub(crate) kind: String,
    /// Where the block starts.
    pub(crate) start: usize,
    /// How many bytes it has.
    pub(crate) size: usize,
}

/// The Bethesda version of the mesh in `data` (83 for Skyrim, 100 for
/// Skyrim Special Edition, 130 for Fallout 4), all its blocks, and the
/// texts its blocks name by number, such as the names of shaders.
pub(crate) fn blocks(data: &[u8]) -> Result<(u32, Vec<BlockInfo>, Vec<String>), NifError> {
    let mut bytes = Bytes::new(data);
    let damaged = || NifError::Damaged;
    let line_len = data
        .iter()
        .take(64)
        .position(|&byte| byte == b'\n')
        .ok_or(NifError::NotAMesh)?;
    let line = bytes.take(line_len + 1).ok_or(NifError::NotAMesh)?;
    if !line.starts_with(MAGIC) {
        return Err(NifError::NotAMesh);
    }
    let version = bytes.u32().ok_or_else(damaged)?;
    let little_endian = bytes.u8().ok_or_else(damaged)? == 1;
    let _user_version = bytes.u32().ok_or_else(damaged)?;
    let block_count = bytes.u32().ok_or_else(damaged)?;
    let bethesda_version = bytes.u32().ok_or_else(damaged)?;
    if version != VERSION_20_2_0_7 || !little_endian || bethesda_version > NEWEST_BETHESDA_VERSION {
        return Err(NifError::UnsupportedVersion);
    }
    for _ in 0..header_texts(bethesda_version) {
        let len = usize::from(bytes.u8().ok_or_else(damaged)?);
        bytes.skip(len).ok_or_else(damaged)?;
    }
    let type_count = bytes.u16().ok_or_else(damaged)?;
    let mut type_names = Vec::new();
    for _ in 0..type_count {
        type_names.push(decode_text(sized_string(&mut bytes).ok_or_else(damaged)?));
    }
    let mut types = Vec::new();
    for _ in 0..block_count {
        types.push(bytes.u16().ok_or_else(damaged)?);
    }
    let mut sizes = Vec::new();
    for _ in 0..block_count {
        sizes.push(usize::try_from(bytes.u32().ok_or_else(damaged)?).map_err(|_| damaged())?);
    }
    let string_count = bytes.u32().ok_or_else(damaged)?;
    let _longest_string = bytes.u32().ok_or_else(damaged)?;
    let mut strings = Vec::new();
    for _ in 0..string_count {
        strings.push(decode_text(sized_string(&mut bytes).ok_or_else(damaged)?));
    }
    let group_count = bytes.u32().ok_or_else(damaged)?;
    for _ in 0..group_count {
        bytes.u32().ok_or_else(damaged)?;
    }
    let mut start = data.len() - bytes.remaining();
    let mut found = Vec::new();
    for (kind, size) in types.into_iter().zip(sizes) {
        found.push(BlockInfo {
            kind: type_names
                .get(usize::from(kind))
                .cloned()
                .unwrap_or_default(),
            start,
            size,
        });
        start = start.checked_add(size).ok_or_else(damaged)?;
    }
    Ok((bethesda_version, found, strings))
}

/// Adds the paths of one texture set block to `paths`.
fn add_texture_set(block: &[u8], paths: &mut Vec<String>) -> Result<(), NifError> {
    let mut block = Bytes::new(block);
    let slot_count = block.u32().ok_or(NifError::Damaged)?;
    for _ in 0..slot_count {
        let path = decode_text(sized_string(&mut block).ok_or(NifError::Damaged)?);
        if !path.is_empty() && !paths.contains(&path) {
            paths.push(path);
        }
    }
    Ok(())
}

/// A text preceded by its length as a 32-bit number.
fn sized_string<'a>(bytes: &mut Bytes<'a>) -> Option<&'a [u8]> {
    let len = usize::try_from(bytes.u32()?).ok()?;
    bytes.take(len)
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::fixture;

    const WALL: &[&str] = &[
        r"textures\architecture\whiterun\wrwall01.dds",
        r"textures\architecture\whiterun\wrwall01_n.dds",
        "",
    ];
    const ROOF: &[&str] = &[
        r"textures\mymod\roof.dds",
        r"textures\architecture\whiterun\wrwall01_n.dds",
    ];

    #[test]
    fn lists_the_textures_of_every_surface() {
        let mesh = fixture::nif(&[WALL, ROOF]);
        assert_eq!(
            texture_paths(&mesh).unwrap(),
            [
                r"textures\architecture\whiterun\wrwall01.dds",
                r"textures\architecture\whiterun\wrwall01_n.dds",
                r"textures\mymod\roof.dds",
            ]
        );
    }

    #[test]
    fn reading_from_a_file_gives_the_same_as_reading_from_memory() {
        // A mesh whose texture sets lie far behind what is read at first.
        let mut mesh = fixture::nif_with_padding(&[WALL, ROOF], 3 * FIRST_READ);
        let expected = texture_paths(&mesh).unwrap();
        assert_eq!(expected.len(), 3);
        assert_eq!(
            read_texture_paths(&mut std::io::Cursor::new(&mesh)).unwrap(),
            expected
        );

        // A small mesh that fits into the first read.
        let small = fixture::nif(&[WALL]);
        assert_eq!(
            read_texture_paths(&mut std::io::Cursor::new(&small)).unwrap(),
            texture_paths(&small).unwrap()
        );

        // A file that ends inside a texture set.
        mesh.truncate(mesh.len() - 5);
        assert_eq!(
            read_texture_paths(&mut std::io::Cursor::new(&mesh)),
            Err(NifError::Damaged)
        );
        assert_eq!(
            read_texture_paths(&mut std::io::Cursor::new(b"not a mesh")),
            Err(NifError::NotAMesh)
        );
    }

    #[test]
    fn a_mesh_without_textured_surfaces_has_no_textures() {
        assert_eq!(
            texture_paths(&fixture::nif(&[])).unwrap(),
            Vec::<String>::new()
        );
    }

    #[test]
    fn other_files_are_rejected() {
        assert_eq!(texture_paths(b""), Err(NifError::NotAMesh));
        assert_eq!(texture_paths(b"BSA\0 and more"), Err(NifError::NotAMesh));
        assert_eq!(
            texture_paths(b"NetImmerse File Format, Version 4.0.0.2\n"),
            Err(NifError::NotAMesh)
        );
    }

    #[test]
    fn a_newer_variant_of_the_format_is_reported_as_unsupported() {
        let mut mesh = fixture::nif(&[WALL]);
        let after_line = mesh.iter().position(|&byte| byte == b'\n').unwrap() + 1;
        // The Bethesda version follows version, endianness, user version
        // and block count. 155 is Fallout 76.
        let bethesda_version = after_line + 4 + 1 + 4 + 4;
        mesh[bethesda_version..bethesda_version + 4].copy_from_slice(&155u32.to_le_bytes());
        assert_eq!(texture_paths(&mesh), Err(NifError::UnsupportedVersion));
    }

    #[test]
    fn fallout_4_meshes_have_one_more_text_in_their_header() {
        let mut mesh = fixture::nif(&[WALL]);
        let after_line = mesh.iter().position(|&byte| byte == b'\n').unwrap() + 1;
        let bethesda_version = after_line + 4 + 1 + 4 + 4;
        mesh[bethesda_version..bethesda_version + 4].copy_from_slice(&130u32.to_le_bytes());
        // An empty longest file path after the three texts of two bytes.
        let after_texts = bethesda_version + 4 + 6;
        mesh.splice(after_texts..after_texts, [1, 0]);
        let skyrim = texture_paths(&fixture::nif(&[WALL])).unwrap();
        assert_eq!(texture_paths(&mesh).unwrap(), skyrim);
        assert_eq!(skyrim.len(), 2);
    }

    #[test]
    fn a_cut_off_mesh_is_damaged_not_fatal() {
        let mesh = fixture::nif(&[WALL, ROOF]);
        assert_eq!(
            texture_paths(&mesh[..mesh.len() - 5]),
            Err(NifError::Damaged)
        );
        assert_eq!(texture_paths(&mesh[..60]), Err(NifError::Damaged));
    }

    #[test]
    fn a_header_that_promises_too_much_is_damaged_not_fatal() {
        let mut mesh = fixture::nif(&[WALL]);
        let after_line = mesh.iter().position(|&byte| byte == b'\n').unwrap() + 1;
        let block_count = after_line + 4 + 1 + 4;
        mesh[block_count..block_count + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(texture_paths(&mesh), Err(NifError::Damaged));
    }

    proptest! {
        #[test]
        fn never_panics_on_arbitrary_bytes(data in proptest::collection::vec(any::<u8>(), 0..512)) {
            let _ = texture_paths(&data);
        }

        #[test]
        fn never_panics_on_a_damaged_mesh(
            position in 0usize..400,
            value in any::<u8>(),
            cut in 0usize..400,
        ) {
            let mut mesh = fixture::nif(&[WALL, ROOF]);
            if let Some(byte) = mesh.get_mut(position) {
                *byte = value;
            }
            mesh.truncate(mesh.len().saturating_sub(cut));
            let _ = texture_paths(&mesh);
            let _ = read_texture_paths(&mut std::io::Cursor::new(&mesh));
        }
    }
}
