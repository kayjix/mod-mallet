//! Reads the list of files in a BA2 archive, and single files out of it.
//!
//! BA2 is the archive format of Fallout 4 (and Fallout 4 VR) and Starfield.
//! It comes in two kinds: *general* archives (`GNRL`), which hold any file
//! whole, and *texture* archives (`DX10`), which hold DDS textures split
//! into chunks of mipmap levels and without their DDS header.
//!
//! Unlike a BSA, the names of the files are not at the start but in a table
//! at the end of the archive:
//!
//! ```text
//! header             24 bytes; 32 in version 2 and 36 in version 3
//! file records       GNRL: 36 bytes each
//!                    DX10: 24 bytes each, then 24 bytes per chunk
//! file contents      read only on request
//! name table         per file: length (u16), then the path
//! ```
//!
//! So one stretch of bytes from the start is not enough to list an archive.
//! [`read_directory`] reads the header and the records, then jumps to the
//! name table; it reads nothing of the file contents. [`parse_directory`]
//! does the same on an archive that is already in memory.
//!
//! To get one file, read the bytes its [`Entry`] points at and hand them to
//! [`unpack`], or let [`read_file`] do both. A texture comes back as a
//! complete DDS file, with a header rebuilt from what the archive records
//! about it.
//!
//! Supported are the versions of Fallout 4 (1, and 7 and 8 from its
//! next-gen update) and of Starfield (2 and 3).

use std::io::{self, BufReader, Cursor, Read, Seek, SeekFrom};

use flate2::read::ZlibDecoder;

use crate::{Bytes, decode_text};

/// Size of the part of the header that every version has. Versions 2 and
/// 3 add some bytes; [`MAX_HEADER_LEN`] covers them all.
pub const HEADER_LEN: usize = 24;
/// Size of the longest header, that of version 3.
pub const MAX_HEADER_LEN: usize = 36;

const MAGIC: &[u8; 4] = b"BTDX";
const GENERAL_RECORD_LEN: usize = 36;
const TEXTURE_RECORD_LEN: usize = 24;
const CHUNK_LEN: usize = 24;

/// Compression methods named in the header of version 3. Older versions
/// always use zlib.
const COMPRESSION_ZLIB: u32 = 0;
const COMPRESSION_LZ4: u32 = 3;

/// The largest file that is unpacked. Far above any real mesh or texture;
/// it keeps a damaged size field from asking for gigabytes of memory.
const MAX_UNPACKED: u64 = 1024 * 1024 * 1024;
/// How many times larger than its packed data an LZ4 block can unpack to,
/// at most. A larger claim is damage.
const LZ4_MAX_RATIO: u64 = 255;
/// More files than any real archive holds; keeps a damaged count from
/// running for long.
const MAX_FILES: u32 = 4 * 1024 * 1024;

/// Why an archive's file list, or a file in it, could not be read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Ba2Error {
    /// The file does not start like a BA2 archive.
    #[error("the file is not a BA2 archive")]
    NotAnArchive,
    /// The archive has a version this reader does not know.
    #[error("BA2 version {0} is not supported")]
    UnsupportedVersion(u32),
    /// The archive is of a kind this reader does not know, such as the
    /// console texture archives (`GNMF`).
    #[error("BA2 archives of type `{0}` are not supported")]
    UnsupportedType(String),
    /// The archive does not store the names of its files.
    #[error("the archive does not store file names")]
    NamesMissing,
    /// The data ends before the file list does, or a size in it is wrong.
    #[error("the archive's file list is damaged or cut off")]
    Damaged,
    /// A file in the archive could not be unpacked.
    #[error("a file in the archive is damaged")]
    DamagedFile,
    /// The archive's files are compressed in a way this reader does not
    /// know.
    #[error("compression method {0} of BA2 archives is not supported")]
    UnsupportedCompression(u32),
}

/// What an archive holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Any kind of file, each stored whole (`GNRL`).
    General,
    /// DDS textures, stored in chunks without their header (`DX10`).
    Textures,
}

/// Where one file lies in an archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The file's path as `folder\file`, in the archive's own spelling.
    pub path: String,
    /// Where the file's data starts, counted from the start of the archive.
    pub offset: u64,
    /// How many bytes the file's data takes up in the archive. For a
    /// texture, from the start of its first chunk to the end of its last.
    pub size: u64,
    /// Whether the data is compressed, for a texture any of its chunks.
    pub compressed: bool,
    content: Content,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Content {
    /// A file of a general archive. Its stored data is [`Entry::size`]
    /// bytes long.
    General { unpacked: u32 },
    /// A texture.
    Texture(Texture),
}

/// What a texture archive records about a texture: enough to rebuild its
/// DDS header.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Texture {
    width: u16,
    height: u16,
    mip_count: u8,
    /// The `DXGI_FORMAT` of its pixels.
    format: u8,
    cubemap: bool,
    chunks: Vec<Chunk>,
}

/// One chunk of a texture, usually a range of mipmap levels.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Chunk {
    /// Where its data starts, counted from [`Entry::offset`].
    start: u64,
    /// The size of its data in the archive; 0 if stored uncompressed.
    packed: u32,
    /// The size of its data unpacked.
    unpacked: u32,
}

impl Chunk {
    fn stored_len(&self) -> u32 {
        if self.packed == 0 {
            self.unpacked
        } else {
            self.packed
        }
    }
}

/// The directory of an archive: its files and how they are stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Directory {
    /// The files, in the order of the archive.
    pub entries: Vec<Entry>,
    /// What the archive holds.
    pub kind: Kind,
    version: u32,
    compression: u32,
}

struct Header {
    version: u32,
    kind: Kind,
    file_count: u32,
    name_table_offset: u64,
    len: usize,
    compression: u32,
}

fn parse_header(data: &[u8]) -> Result<Header, Ba2Error> {
    let mut bytes = Bytes::new(data);
    if bytes.take(4) != Some(MAGIC.as_slice()) {
        return Err(Ba2Error::NotAnArchive);
    }
    let version = bytes.u32().ok_or(Ba2Error::Damaged)?;
    let kind = bytes.signature().ok_or(Ba2Error::Damaged)?;
    let file_count = bytes.u32().ok_or(Ba2Error::Damaged)?;
    let name_table_offset = bytes.u64().ok_or(Ba2Error::Damaged)?;
    let (len, compression) = match version {
        1 | 7 | 8 => (HEADER_LEN, COMPRESSION_ZLIB),
        2 => {
            bytes.skip(8).ok_or(Ba2Error::Damaged)?;
            (HEADER_LEN + 8, COMPRESSION_ZLIB)
        }
        3 => {
            bytes.skip(8).ok_or(Ba2Error::Damaged)?;
            let compression = bytes.u32().ok_or(Ba2Error::Damaged)?;
            (MAX_HEADER_LEN, compression)
        }
        other => return Err(Ba2Error::UnsupportedVersion(other)),
    };
    let kind = match &kind {
        b"GNRL" => Kind::General,
        b"DX10" => Kind::Textures,
        other => return Err(Ba2Error::UnsupportedType(decode_text(other))),
    };
    Ok(Header {
        version,
        kind,
        file_count,
        name_table_offset,
        len,
        compression,
    })
}

/// Why reading failed: the medium, or the archive itself.
enum Failure {
    Io(io::Error),
    Archive(Ba2Error),
}

impl From<io::Error> for Failure {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<Ba2Error> for Failure {
    fn from(error: Ba2Error) -> Self {
        Self::Archive(error)
    }
}

/// Reads the directory of the archive that `reader` holds.
///
/// Reads the header and the file records from the start, then the name
/// table from the end; the file contents in between are skipped. The outer
/// error is a failure of `reader`, the inner one a problem of the archive.
/// Where `reader` stands afterwards is unspecified.
pub fn read_directory<R: Read + Seek>(reader: &mut R) -> io::Result<Result<Directory, Ba2Error>> {
    match directory_from(&mut BufReader::new(reader)) {
        Ok(directory) => Ok(Ok(directory)),
        Err(Failure::Archive(error)) => Ok(Err(error)),
        Err(Failure::Io(error)) => Err(error),
    }
}

/// Reads the directory of an archive that is in memory as a whole.
pub fn parse_directory(archive: &[u8]) -> Result<Directory, Ba2Error> {
    match directory_from(&mut Cursor::new(archive)) {
        Ok(directory) => Ok(directory),
        Err(Failure::Archive(error)) => Err(error),
        // Reading from memory does not fail; should it, the data is at
        // fault.
        Err(Failure::Io(_)) => Err(Ba2Error::Damaged),
    }
}

/// Lists the files in an archive that is in memory as a whole, as
/// `folder\file` in the archive's own spelling.
pub fn parse_listing(archive: &[u8]) -> Result<Vec<String>, Ba2Error> {
    let directory = parse_directory(archive)?;
    Ok(directory
        .entries
        .into_iter()
        .map(|entry| entry.path)
        .collect())
}

/// Reads exactly `len` bytes, or reports the archive as damaged.
fn read_exactly<R: Read>(reader: &mut R, len: usize) -> Result<Vec<u8>, Failure> {
    let mut data = Vec::with_capacity(len.min(64 * 1024));
    reader
        .by_ref()
        .take(u64::try_from(len).map_err(|_| Ba2Error::Damaged)?)
        .read_to_end(&mut data)?;
    if data.len() == len {
        Ok(data)
    } else {
        Err(Ba2Error::Damaged.into())
    }
}

fn directory_from<R: Read + Seek>(reader: &mut R) -> Result<Directory, Failure> {
    let archive_len = reader.seek(SeekFrom::End(0))?;
    reader.seek(SeekFrom::Start(0))?;
    let mut head = Vec::new();
    reader
        .by_ref()
        .take(MAX_HEADER_LEN as u64)
        .read_to_end(&mut head)?;
    let header = parse_header(&head)?;
    reader.seek(SeekFrom::Start(header.len as u64))?;

    // The records alone must fit into the archive.
    let min_record_len = match header.kind {
        Kind::General => GENERAL_RECORD_LEN,
        Kind::Textures => TEXTURE_RECORD_LEN,
    };
    let records_len = u64::from(header.file_count) * min_record_len as u64;
    if header.file_count > MAX_FILES || header.len as u64 + records_len > archive_len {
        return Err(Ba2Error::Damaged.into());
    }

    let mut entries = Vec::new();
    for _ in 0..header.file_count {
        let entry = match header.kind {
            Kind::General => read_general_record(reader)?,
            Kind::Textures => read_texture_record(reader)?,
        };
        // The stored data must lie inside the archive.
        if entry
            .offset
            .checked_add(entry.size)
            .is_none_or(|end| end > archive_len)
        {
            return Err(Ba2Error::Damaged.into());
        }
        entries.push(entry);
    }

    if !entries.is_empty() {
        if header.name_table_offset == 0 {
            return Err(Ba2Error::NamesMissing.into());
        }
        if header.name_table_offset > archive_len {
            return Err(Ba2Error::Damaged.into());
        }
        reader.seek(SeekFrom::Start(header.name_table_offset))?;
        for entry in &mut entries {
            let len = read_exactly(reader, 2)?;
            let len = u16::from_le_bytes(len.try_into().map_err(|_| Ba2Error::Damaged)?);
            let name = read_exactly(reader, usize::from(len))?;
            if name.is_empty() {
                return Err(Ba2Error::Damaged.into());
            }
            entry.path = decode_text(&name).replace('/', "\\");
        }
    }

    Ok(Directory {
        entries,
        kind: header.kind,
        version: header.version,
        compression: header.compression,
    })
}

/// Reads one record of a general archive. The path is filled in later from
/// the name table.
fn read_general_record<R: Read>(reader: &mut R) -> Result<Entry, Failure> {
    let record = read_exactly(reader, GENERAL_RECORD_LEN)?;
    let mut bytes = Bytes::new(&record);
    // Name hash, extension, folder hash, flags: not needed.
    bytes.skip(16).ok_or(Ba2Error::Damaged)?;
    let offset = bytes.u64().ok_or(Ba2Error::Damaged)?;
    let packed = bytes.u32().ok_or(Ba2Error::Damaged)?;
    let unpacked = bytes.u32().ok_or(Ba2Error::Damaged)?;
    // The rest is a fixed marker, 0xBAADF00D, which is not checked so that
    // archives from other packing tools still list.
    Ok(Entry {
        path: String::new(),
        offset,
        size: u64::from(if packed == 0 { unpacked } else { packed }),
        compressed: packed != 0,
        content: Content::General { unpacked },
    })
}

/// Reads one record of a texture archive with its chunks.
fn read_texture_record<R: Read>(reader: &mut R) -> Result<Entry, Failure> {
    let record = read_exactly(reader, TEXTURE_RECORD_LEN)?;
    let mut bytes = Bytes::new(&record);
    // Name hash, extension, folder hash, one unknown byte.
    bytes.skip(13).ok_or(Ba2Error::Damaged)?;
    let chunk_count = bytes.u8().ok_or(Ba2Error::Damaged)?;
    let chunk_header_len = bytes.u16().ok_or(Ba2Error::Damaged)?;
    let height = bytes.u16().ok_or(Ba2Error::Damaged)?;
    let width = bytes.u16().ok_or(Ba2Error::Damaged)?;
    let mip_count = bytes.u8().ok_or(Ba2Error::Damaged)?;
    let format = bytes.u8().ok_or(Ba2Error::Damaged)?;
    let cubemap = bytes.u8().ok_or(Ba2Error::Damaged)? & 1 != 0;
    // The last byte is the tile mode, which only matters to consoles.
    if usize::from(chunk_header_len) != CHUNK_LEN {
        return Err(Ba2Error::Damaged.into());
    }

    let chunk_data = read_exactly(reader, usize::from(chunk_count) * CHUNK_LEN)?;
    let mut bytes = Bytes::new(&chunk_data);
    let mut located = Vec::with_capacity(usize::from(chunk_count));
    for _ in 0..chunk_count {
        let offset = bytes.u64().ok_or(Ba2Error::Damaged)?;
        let packed = bytes.u32().ok_or(Ba2Error::Damaged)?;
        let unpacked = bytes.u32().ok_or(Ba2Error::Damaged)?;
        // First and last mipmap level, and the fixed marker.
        bytes.skip(8).ok_or(Ba2Error::Damaged)?;
        located.push((offset, packed, unpacked));
    }

    // The entry spans all chunks; each chunk is placed relative to it.
    let start = located
        .iter()
        .map(|&(offset, _, _)| offset)
        .min()
        .unwrap_or(0);
    let mut end = start;
    let mut chunks = Vec::with_capacity(located.len());
    for (offset, packed, unpacked) in located {
        let chunk = Chunk {
            start: offset.saturating_sub(start),
            packed,
            unpacked,
        };
        let chunk_end = offset
            .checked_add(u64::from(chunk.stored_len()))
            .ok_or(Ba2Error::Damaged)?;
        end = end.max(chunk_end);
        chunks.push(chunk);
    }
    let compressed = chunks.iter().any(|chunk| chunk.packed != 0);
    Ok(Entry {
        path: String::new(),
        offset: start,
        size: end.saturating_sub(start),
        compressed,
        content: Content::Texture(Texture {
            width,
            height,
            mip_count,
            format,
            cubemap,
            chunks,
        }),
    })
}

/// Turns the stored bytes of a file into its content. A texture comes back
/// as a complete DDS file.
///
/// `stored` are the [`Entry::size`] bytes at [`Entry::offset`] of the
/// archive that `directory` was read from.
pub fn unpack(stored: &[u8], entry: &Entry, directory: &Directory) -> Result<Vec<u8>, Ba2Error> {
    match &entry.content {
        Content::General { unpacked } => {
            let size = usize::try_from(entry.size).map_err(|_| Ba2Error::DamagedFile)?;
            let data = stored.get(..size).ok_or(Ba2Error::DamagedFile)?;
            if entry.compressed {
                decompress(data, *unpacked, directory)
            } else {
                Ok(data.to_vec())
            }
        }
        Content::Texture(texture) => unpack_texture(stored, texture, directory),
    }
}

/// Reads one file of the archive that `reader` holds and unpacks it. The
/// outer error is a failure of `reader`, the inner one a problem of the
/// archive.
pub fn read_file<R: Read + Seek>(
    reader: &mut R,
    entry: &Entry,
    directory: &Directory,
) -> io::Result<Result<Vec<u8>, Ba2Error>> {
    if entry.size > MAX_UNPACKED {
        return Ok(Err(Ba2Error::DamagedFile));
    }
    reader.seek(SeekFrom::Start(entry.offset))?;
    let mut stored = Vec::new();
    reader.by_ref().take(entry.size).read_to_end(&mut stored)?;
    Ok(unpack(&stored, entry, directory))
}

fn unpack_texture(
    stored: &[u8],
    texture: &Texture,
    directory: &Directory,
) -> Result<Vec<u8>, Ba2Error> {
    if texture.chunks.is_empty() {
        return Err(Ba2Error::DamagedFile);
    }
    let total: u64 = texture
        .chunks
        .iter()
        .map(|chunk| u64::from(chunk.unpacked))
        .sum();
    if total > MAX_UNPACKED {
        return Err(Ba2Error::DamagedFile);
    }

    let mut dds = dds_header(texture);
    for chunk in &texture.chunks {
        let start = usize::try_from(chunk.start).map_err(|_| Ba2Error::DamagedFile)?;
        let len = usize::try_from(chunk.stored_len()).map_err(|_| Ba2Error::DamagedFile)?;
        let end = start.checked_add(len).ok_or(Ba2Error::DamagedFile)?;
        let data = stored.get(start..end).ok_or(Ba2Error::DamagedFile)?;
        if chunk.packed == 0 {
            dds.extend_from_slice(data);
        } else {
            dds.extend(decompress(data, chunk.unpacked, directory)?);
        }
    }
    Ok(dds)
}

/// Unpacks compressed data that must come to `unpacked` bytes.
fn decompress(packed: &[u8], unpacked: u32, directory: &Directory) -> Result<Vec<u8>, Ba2Error> {
    let expected = u64::from(unpacked);
    if expected > MAX_UNPACKED {
        return Err(Ba2Error::DamagedFile);
    }
    // Only version 3 names its compression; the older ones use zlib.
    let method = if directory.version == 3 {
        directory.compression
    } else {
        COMPRESSION_ZLIB
    };
    match method {
        COMPRESSION_ZLIB => {
            let mut data = Vec::new();
            match ZlibDecoder::new(packed)
                .take(expected)
                .read_to_end(&mut data)
            {
                Ok(_) if u64::try_from(data.len()).ok() == Some(expected) => Ok(data),
                _ => Err(Ba2Error::DamagedFile),
            }
        }
        COMPRESSION_LZ4 => {
            let packed_len = u64::try_from(packed.len()).map_err(|_| Ba2Error::DamagedFile)?;
            if expected > packed_len.saturating_mul(LZ4_MAX_RATIO).saturating_add(16) {
                return Err(Ba2Error::DamagedFile);
            }
            let mut data = vec![0; usize::try_from(expected).map_err(|_| Ba2Error::DamagedFile)?];
            match lz4_flex::block::decompress_into(packed, &mut data) {
                Ok(len) if len == data.len() => Ok(data),
                _ => Err(Ba2Error::DamagedFile),
            }
        }
        other => Err(Ba2Error::UnsupportedCompression(other)),
    }
}

/// Bytes per 4 × 4 block of the block-compressed `DXGI_FORMAT`s; `None`
/// for other formats.
fn block_bytes(format: u8) -> Option<u32> {
    match format {
        // BC1, BC4
        70..=72 | 79..=81 => Some(8),
        // BC2, BC3, BC5, BC6H, BC7
        73..=78 | 82..=84 | 94..=99 => Some(16),
        _ => None,
    }
}

/// The DDS header of a texture: the classic header, pointing to the
/// extended `DX10` header that follows and names the pixel format as the
/// archive records it.
fn dds_header(texture: &Texture) -> Vec<u8> {
    const CAPS: u32 = 0x1;
    const HEIGHT: u32 = 0x2;
    const WIDTH: u32 = 0x4;
    const PIXEL_FORMAT: u32 = 0x1000;
    const MIPMAP_COUNT: u32 = 0x2_0000;
    const LINEAR_SIZE: u32 = 0x8_0000;
    const FOUR_CC: u32 = 0x4;
    const CAPS_COMPLEX: u32 = 0x8;
    const CAPS_TEXTURE: u32 = 0x1000;
    const CAPS_MIPMAP: u32 = 0x40_0000;
    const CAPS2_CUBEMAP_ALL_FACES: u32 = 0x200 | 0xFC00;
    const DIMENSION_TEXTURE_2D: u32 = 3;
    const MISC_TEXTURE_CUBE: u32 = 0x4;

    let width = u32::from(texture.width);
    let height = u32::from(texture.height);
    let mip_count = u32::from(texture.mip_count).max(1);
    // The size of the largest level; left out if it does not fit the field.
    let linear_size = block_bytes(texture.format).and_then(|block| {
        width
            .div_ceil(4)
            .max(1)
            .checked_mul(height.div_ceil(4).max(1))?
            .checked_mul(block)
    });

    let mut flags = CAPS | HEIGHT | WIDTH | PIXEL_FORMAT | MIPMAP_COUNT;
    if linear_size.is_some() {
        flags |= LINEAR_SIZE;
    }
    let mut caps = CAPS_TEXTURE;
    if mip_count > 1 {
        caps |= CAPS_COMPLEX | CAPS_MIPMAP;
    }
    let (caps2, misc) = if texture.cubemap {
        caps |= CAPS_COMPLEX;
        (CAPS2_CUBEMAP_ALL_FACES, MISC_TEXTURE_CUBE)
    } else {
        (0, 0)
    };

    let mut header = Vec::with_capacity(148);
    header.extend(b"DDS ");
    let mut push = |value: u32| header.extend(value.to_le_bytes());
    push(124); // size of the header after the magic number
    push(flags);
    push(height);
    push(width);
    push(linear_size.unwrap_or(0));
    push(0); // depth
    push(mip_count);
    for _ in 0..11 {
        push(0); // reserved
    }
    // Pixel format: its size, then "the format is named by a code".
    push(32);
    push(FOUR_CC);
    push(u32::from_le_bytes(*b"DX10"));
    for _ in 0..5 {
        push(0); // bit count and masks
    }
    push(caps);
    push(caps2);
    for _ in 0..3 {
        push(0); // caps 3 and 4, reserved
    }
    // The extended header: format, dimension, flags, array size, alpha.
    push(u32::from(texture.format));
    push(DIMENSION_TEXTURE_2D);
    push(misc);
    push(1);
    push(0);
    header
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::dds;
    use crate::fixture::{self, Ba2Options, Ba2Texture};

    const FILES: &[&str] = &[
        r"Meshes\Landscape\Grass\Fern01.nif",
        r"Meshes\Landscape\Grass\Fern02.nif",
        r"Sound\FX\step.wav",
    ];

    fn options(version: u32, compressed: bool) -> Ba2Options {
        Ba2Options {
            version,
            compressed,
        }
    }

    fn read_back(archive: &[u8], path: &str) -> Result<Vec<u8>, Ba2Error> {
        let directory = parse_directory(archive)?;
        let entry = directory
            .entries
            .iter()
            .find(|entry| entry.path == path)
            .unwrap();
        let start = usize::try_from(entry.offset).unwrap();
        let end = start + usize::try_from(entry.size).unwrap();
        unpack(&archive[start..end], entry, &directory)
    }

    #[test]
    fn lists_the_files_of_an_archive() {
        let archive = fixture::ba2(FILES);
        assert_eq!(parse_listing(&archive).unwrap(), FILES);
        assert_eq!(parse_directory(&archive).unwrap().kind, Kind::General);
    }

    #[test]
    fn forward_slashes_are_listed_as_backslashes() {
        let archive = fixture::ba2(&["Meshes/Rock.nif"]);
        assert_eq!(parse_listing(&archive).unwrap(), [r"Meshes\Rock.nif"]);
    }

    #[test]
    fn reading_from_a_reader_matches_reading_from_memory() {
        let files = [(r"Meshes\Wall.nif", b"wall wall wall wall".as_slice())];
        let archive = fixture::ba2_with(&files, options(1, true));
        let mut reader = Cursor::new(&archive);
        let directory = read_directory(&mut reader).unwrap().unwrap();
        assert_eq!(directory, parse_directory(&archive).unwrap());

        let entry = &directory.entries[0];
        let content = read_file(&mut reader, entry, &directory).unwrap().unwrap();
        assert_eq!(content, b"wall wall wall wall");
    }

    #[test]
    fn files_are_read_back_from_every_version() {
        let mesh = b"a mesh, repeated: a mesh, repeated: a mesh, repeated".as_slice();
        let files = [(r"Meshes\Wall.nif", mesh), (r"Textures\Wall.txt", b"tex")];
        for version in [1, 2, 3, 7, 8] {
            for compressed in [false, true] {
                let archive = fixture::ba2_with(&files, options(version, compressed));
                assert_eq!(
                    read_back(&archive, r"Meshes\Wall.nif").unwrap(),
                    mesh,
                    "version {version}, compressed {compressed}"
                );
                assert_eq!(read_back(&archive, r"Textures\Wall.txt").unwrap(), b"tex");
                let directory = parse_directory(&archive).unwrap();
                assert!(
                    directory
                        .entries
                        .iter()
                        .all(|entry| entry.compressed == compressed)
                );
            }
        }
    }

    fn texture(path: &str, cubemap: bool) -> Ba2Texture<'_> {
        // BC1, 8 × 8 pixels with two levels: 4 blocks, then 1 block.
        Ba2Texture {
            path,
            width: 8,
            height: 8,
            mip_count: 2,
            format: 71,
            cubemap,
            chunks: vec![vec![0x11; 32], vec![0x22; 8]],
        }
    }

    #[test]
    fn lists_the_textures_of_a_texture_archive() {
        let textures = [
            texture(r"Textures\Rock_d.dds", false),
            texture(r"Textures\Sky_e.dds", true),
        ];
        let archive = fixture::ba2_textures(&textures, options(1, false));
        let directory = parse_directory(&archive).unwrap();
        assert_eq!(directory.kind, Kind::Textures);
        assert_eq!(
            parse_listing(&archive).unwrap(),
            [r"Textures\Rock_d.dds", r"Textures\Sky_e.dds"]
        );
    }

    #[test]
    fn textures_come_back_as_dds_files() {
        for version in [1, 3, 8] {
            for compressed in [false, true] {
                let textures = [texture(r"Textures\Rock_d.dds", false)];
                let archive = fixture::ba2_textures(&textures, options(version, compressed));
                let dds_file = read_back(&archive, r"Textures\Rock_d.dds").unwrap();

                assert_eq!(
                    dds_file.len(),
                    dds::HEADER_SIZE + dds::DX10_HEADER_SIZE + 40
                );
                let mut expected_pixels = vec![0x11; 32];
                expected_pixels.extend([0x22; 8]);
                assert_eq!(&dds_file[148..], expected_pixels.as_slice());

                // The texture reader understands the rebuilt header.
                assert_eq!(
                    dds::size(&dds_file).unwrap(),
                    dds::Size {
                        width: 8,
                        height: 8,
                        levels: 2
                    }
                );
                let level = dds::locate_level(&dds_file, 1).unwrap();
                assert_eq!((level.offset, level.length), (148 + 32, 8));
                assert!(dds::decode_level(&dds_file, 0).is_ok());
            }
        }
    }

    #[test]
    fn the_rebuilt_header_describes_the_texture() {
        let textures = [texture(r"Textures\Sky_e.dds", true)];
        let archive = fixture::ba2_textures(&textures, options(1, false));
        let dds_file = read_back(&archive, r"Textures\Sky_e.dds").unwrap();
        let u32_at =
            |offset: usize| u32::from_le_bytes(dds_file[offset..offset + 4].try_into().unwrap());
        assert_eq!(&dds_file[..4], b"DDS ");
        assert_eq!(u32_at(4), 124);
        assert_eq!(u32_at(20), 32, "linear size of the largest level");
        assert_eq!(&dds_file[84..88], b"DX10");
        assert_eq!(u32_at(112), 0xFE00, "all six faces of a cube");
        assert_eq!(u32_at(128), 71, "DXGI format");
        assert_eq!(u32_at(136), 4, "cube texture");
    }

    #[test]
    fn a_texture_entry_spans_all_its_chunks() {
        let textures = [
            texture(r"Textures\Rock_d.dds", false),
            texture(r"Textures\Sky_e.dds", false),
        ];
        let archive = fixture::ba2_textures(&textures, options(1, false));
        let directory = parse_directory(&archive).unwrap();
        let [first, second] = directory.entries.as_slice() else {
            panic!("two entries expected");
        };
        assert_eq!(first.size, 40);
        assert_eq!(second.offset, first.offset + 40);
        assert!(!first.compressed);
    }

    #[test]
    fn a_huge_texture_gets_a_header_without_a_crash() {
        let texture = Texture {
            width: u16::MAX,
            height: u16::MAX,
            mip_count: 0,
            format: 98,
            cubemap: false,
            chunks: Vec::new(),
        };
        assert_eq!(dds_header(&texture).len(), 148);
    }

    #[test]
    fn the_listing_needs_the_name_table() {
        let archive = fixture::ba2(FILES);
        // Cutting into the name table at the end loses names.
        assert_eq!(
            parse_listing(&archive[..archive.len() - 1]),
            Err(Ba2Error::Damaged)
        );
        // Cutting into the header or the records, too.
        assert_eq!(parse_listing(&archive[..20]), Err(Ba2Error::Damaged));
        assert_eq!(parse_listing(&archive[..40]), Err(Ba2Error::Damaged));
    }

    #[test]
    fn an_empty_archive_lists_nothing() {
        assert_eq!(
            parse_listing(&fixture::ba2(&[])).unwrap(),
            Vec::<String>::new()
        );
    }

    #[test]
    fn other_files_are_rejected() {
        assert_eq!(
            parse_listing(b"BSA\0 and so on"),
            Err(Ba2Error::NotAnArchive)
        );
        assert_eq!(parse_listing(b""), Err(Ba2Error::NotAnArchive));
        assert_eq!(parse_listing(b"BTDX"), Err(Ba2Error::Damaged));
    }

    #[test]
    fn unknown_versions_and_types_are_rejected() {
        let mut archive = fixture::ba2(FILES);
        archive[4..8].copy_from_slice(&9u32.to_le_bytes());
        assert_eq!(
            parse_listing(&archive),
            Err(Ba2Error::UnsupportedVersion(9))
        );

        let mut archive = fixture::ba2(FILES);
        archive[8..12].copy_from_slice(b"GNMF");
        assert_eq!(
            parse_listing(&archive),
            Err(Ba2Error::UnsupportedType("GNMF".to_owned()))
        );
    }

    #[test]
    fn an_archive_without_names_is_rejected() {
        let mut archive = fixture::ba2(FILES);
        archive[16..24].copy_from_slice(&0u64.to_le_bytes());
        assert_eq!(parse_listing(&archive), Err(Ba2Error::NamesMissing));
    }

    #[test]
    fn a_header_that_promises_too_much_is_damaged_not_fatal() {
        let mut archive = fixture::ba2(FILES);
        // Claim four billion files.
        archive[12..16].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(parse_listing(&archive), Err(Ba2Error::Damaged));

        // A name table beyond the end.
        let mut archive = fixture::ba2(FILES);
        archive[16..24].copy_from_slice(&u64::MAX.to_le_bytes());
        assert_eq!(parse_listing(&archive), Err(Ba2Error::Damaged));
    }

    #[test]
    fn a_file_that_points_past_the_end_is_damage() {
        let files = [(r"Meshes\Wall.nif", b"wall".as_slice())];
        let mut archive = fixture::ba2_with(&files, options(1, false));
        // The data offset of the first record.
        archive[40..48].copy_from_slice(&u64::MAX.to_le_bytes());
        assert_eq!(parse_listing(&archive), Err(Ba2Error::Damaged));
    }

    #[test]
    fn damaged_file_data_is_an_error_not_a_crash() {
        for version in [1, 3] {
            let files = [(r"Meshes\Wall.nif", b"some mesh data".as_slice())];
            let archive = fixture::ba2_with(&files, options(version, true));
            let directory = parse_directory(&archive).unwrap();
            let entry = &directory.entries[0];
            assert_eq!(unpack(&[], entry, &directory), Err(Ba2Error::DamagedFile));
            let garbage = vec![0xAB; usize::try_from(entry.size).unwrap()];
            assert_eq!(
                unpack(&garbage, entry, &directory),
                Err(Ba2Error::DamagedFile)
            );
        }

        let textures = [texture(r"Textures\Rock_d.dds", false)];
        let archive = fixture::ba2_textures(&textures, options(1, true));
        let directory = parse_directory(&archive).unwrap();
        let entry = &directory.entries[0];
        assert_eq!(
            unpack(&[1, 2, 3], entry, &directory),
            Err(Ba2Error::DamagedFile)
        );
    }

    #[test]
    fn an_unknown_compression_is_named() {
        let files = [(r"Meshes\Wall.nif", b"some mesh data".as_slice())];
        let mut archive = fixture::ba2_with(&files, options(3, true));
        archive[32..36].copy_from_slice(&7u32.to_le_bytes());
        // The listing does not depend on it.
        let directory = parse_directory(&archive).unwrap();
        let entry = &directory.entries[0];
        let start = usize::try_from(entry.offset).unwrap();
        let stored = &archive[start..start + usize::try_from(entry.size).unwrap()];
        assert_eq!(
            unpack(stored, entry, &directory),
            Err(Ba2Error::UnsupportedCompression(7))
        );
    }

    proptest! {
        #[test]
        fn never_panics_on_arbitrary_bytes(data in proptest::collection::vec(any::<u8>(), 0..512)) {
            let _ = parse_listing(&data);
        }

        #[test]
        fn never_panics_on_arbitrary_bytes_after_a_header(
            version in prop_oneof![Just(1u32), Just(2), Just(3), Just(8)],
            textures in any::<bool>(),
            file_count in 0u32..8,
            name_table_offset in 0u64..600,
            rest in proptest::collection::vec(any::<u8>(), 0..512),
        ) {
            let mut data = b"BTDX".to_vec();
            data.extend(version.to_le_bytes());
            data.extend(if textures { b"DX10" } else { b"GNRL" });
            data.extend(file_count.to_le_bytes());
            data.extend(name_table_offset.to_le_bytes());
            data.extend(rest);
            if let Ok(directory) = parse_directory(&data) {
                for entry in &directory.entries {
                    let start = usize::try_from(entry.offset).unwrap_or(usize::MAX).min(data.len());
                    let _ = unpack(&data[start..], entry, &directory);
                }
            }
        }

        #[test]
        fn never_panics_on_a_damaged_archive(
            textures in any::<bool>(),
            version in prop_oneof![Just(1u32), Just(2), Just(3)],
            compressed in any::<bool>(),
            position in 0usize..400,
            value in any::<u8>(),
            cut in 0usize..400,
        ) {
            let mut archive = if textures {
                fixture::ba2_textures(
                    &[texture(r"Textures\Rock_d.dds", false), texture(r"Textures\Sky.dds", true)],
                    options(version, compressed),
                )
            } else {
                let files = [(r"Meshes\Wall.nif", b"wall wall wall".as_slice()), (r"a.txt", b"a")];
                fixture::ba2_with(&files, options(version, compressed))
            };
            if let Some(byte) = archive.get_mut(position) {
                *byte = value;
            }
            archive.truncate(archive.len().saturating_sub(cut));
            if let Ok(directory) = parse_directory(&archive) {
                for entry in &directory.entries {
                    let mut reader = Cursor::new(&archive);
                    let _ = read_file(&mut reader, entry, &directory);
                }
            }
        }

        #[test]
        fn unpacking_never_panics(
            stored in proptest::collection::vec(any::<u8>(), 0..200),
            textures in any::<bool>(),
            version in prop_oneof![Just(1u32), Just(3)],
            compressed in any::<bool>(),
        ) {
            let archive = if textures {
                fixture::ba2_textures(&[texture(r"Textures\Rock_d.dds", false)], options(version, compressed))
            } else {
                fixture::ba2_with(&[(r"Meshes\Wall.nif", b"x".as_slice())], options(version, compressed))
            };
            let directory = parse_directory(&archive).unwrap();
            let _ = unpack(&stored, &directory.entries[0], &directory);
        }
    }
}
