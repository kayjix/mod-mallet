//! Reads the list of files in a BSA archive, and single files out of it.
//!
//! The directory at the start of the archive names every file and says
//! where its data lies. Use [`listing_len`] to learn how many bytes to read
//! from disk, then [`parse_directory`] on those bytes. The contents of the
//! packed files, which make up nearly all of an archive's size, are not
//! touched by that.
//!
//! To get one file, read the bytes its [`Entry`] points at and hand them to
//! [`unpack`], which removes the packaging: an optional copy of the file's
//! name, and compression (zlib in the older versions, LZ4 in Skyrim Special
//! Edition).
//!
//! Supported are the archive versions of Oblivion (103), Skyrim (104) and
//! Skyrim Special Edition (105). The layout:
//!
//! ```text
//! header (36 bytes)
//! folder records     one per folder; 16 bytes, or 24 in version 105
//! per folder:        folder name (length byte, text, NUL)
//!                    file records, 16 bytes each
//! file names         NUL-terminated, in the order of the file records
//! file contents      read only on request
//! ```

use std::io::Read;

use flate2::read::ZlibDecoder;
use lz4_flex::frame::FrameDecoder;

use crate::{Bytes, decode_text};

/// Size of the fixed header at the start of an archive.
pub const HEADER_LEN: usize = 36;

const MAGIC: &[u8; 4] = b"BSA\0";
const FILE_RECORD_LEN: u64 = 16;
const FLAG_FOLDER_NAMES: u32 = 0x1;
const FLAG_FILE_NAMES: u32 = 0x2;
const FLAG_COMPRESSED: u32 = 0x4;
const FLAG_EMBEDDED_NAMES: u32 = 0x100;

/// In a file record's size: this file is stored the other way round than
/// the archive's default (compressed in an uncompressed archive, or the
/// reverse).
const SIZE_TOGGLES_COMPRESSION: u32 = 0x4000_0000;

/// The largest file that is unpacked. Far above any real mesh or texture;
/// it keeps a damaged size field from asking for gigabytes of memory.
const MAX_UNPACKED: u64 = 1024 * 1024 * 1024;

/// Why an archive's file list could not be read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BsaError {
    /// The file does not start like a BSA archive.
    #[error("the file is not a BSA archive")]
    NotAnArchive,
    /// The archive has a version this reader does not know.
    #[error("BSA version {0} is not supported")]
    UnsupportedVersion(u32),
    /// The archive does not store the names of its files or folders.
    #[error("the archive does not store file names")]
    NamesMissing,
    /// The data ends before the file list does, or a size in it is wrong.
    #[error("the archive's file list is damaged or cut off")]
    Damaged,
    /// A file in the archive could not be unpacked.
    #[error("a file in the archive is damaged")]
    DamagedFile,
}

/// Where one file lies in an archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The file's path as `folder\file`, in the archive's own spelling.
    pub path: String,
    /// Where the file's data starts, counted from the start of the archive.
    pub offset: u32,
    /// How many bytes the file's data takes up in the archive.
    pub size: u32,
    /// Whether the data is compressed.
    pub compressed: bool,
}

/// The directory of an archive: its files and how they are stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Directory {
    /// The files, in the order of the archive.
    pub entries: Vec<Entry>,
    version: u32,
    embedded_names: bool,
}

struct Header {
    version: u32,
    flags: u32,
    folder_record_len: u64,
    folder_count: u32,
    file_count: u32,
    folder_names_len: u32,
    file_names_len: u32,
}

fn parse_header(data: &[u8]) -> Result<Header, BsaError> {
    let mut bytes = Bytes::new(data);
    if bytes.take(4) != Some(MAGIC.as_slice()) {
        return Err(BsaError::NotAnArchive);
    }
    let mut next = || bytes.u32().ok_or(BsaError::Damaged);
    let version = next()?;
    let _folder_offset = next()?;
    let flags = next()?;
    let folder_count = next()?;
    let file_count = next()?;
    let folder_names_len = next()?;
    let file_names_len = next()?;

    let folder_record_len = match version {
        103 | 104 => 16,
        105 => 24,
        other => return Err(BsaError::UnsupportedVersion(other)),
    };
    if flags & FLAG_FOLDER_NAMES == 0 || flags & FLAG_FILE_NAMES == 0 {
        return Err(BsaError::NamesMissing);
    }
    Ok(Header {
        version,
        flags,
        folder_record_len,
        folder_count,
        file_count,
        folder_names_len,
        file_names_len,
    })
}

/// How many bytes from the start of the archive hold its file list.
///
/// `header` must contain at least the first [`HEADER_LEN`] bytes.
pub fn listing_len(header: &[u8]) -> Result<u64, BsaError> {
    let header = parse_header(header)?;
    let folders = u64::from(header.folder_count);
    // A stored folder name is preceded by a length byte, which the total in
    // the header does not count.
    Ok(HEADER_LEN as u64
        + folders * header.folder_record_len
        + u64::from(header.folder_names_len)
        + folders
        + u64::from(header.file_count) * FILE_RECORD_LEN
        + u64::from(header.file_names_len))
}

/// Lists the files in an archive, as `folder\file` in the archive's own
/// spelling.
///
/// `data` must contain at least the first [`listing_len`] bytes of the
/// archive.
pub fn parse_listing(data: &[u8]) -> Result<Vec<String>, BsaError> {
    let directory = parse_directory(data)?;
    Ok(directory
        .entries
        .into_iter()
        .map(|entry| entry.path)
        .collect())
}

/// Reads the directory of an archive.
///
/// `data` must contain at least the first [`listing_len`] bytes of the
/// archive.
pub fn parse_directory(data: &[u8]) -> Result<Directory, BsaError> {
    let header = parse_header(data)?;
    let archive_compressed = header.flags & FLAG_COMPRESSED != 0;
    let mut bytes = Bytes::new(data);
    bytes.skip(HEADER_LEN).ok_or(BsaError::Damaged)?;

    // Folder records: only the number of files in each folder matters.
    let mut files_per_folder = Vec::new();
    for _ in 0..header.folder_count {
        let record = bytes
            .take(usize::try_from(header.folder_record_len).map_err(|_| BsaError::Damaged)?)
            .ok_or(BsaError::Damaged)?;
        let count = record
            .get(8..12)
            .and_then(|count| count.try_into().ok())
            .map(u32::from_le_bytes)
            .ok_or(BsaError::Damaged)?;
        files_per_folder.push(count);
    }

    // Folder names, each followed by that folder's file records: a name
    // hash, the size and the offset of the data.
    let mut folders = Vec::with_capacity(files_per_folder.len());
    let mut records = Vec::new();
    for &count in &files_per_folder {
        let name_len = usize::from(bytes.u8().ok_or(BsaError::Damaged)?);
        let name = bytes.take(name_len).ok_or(BsaError::Damaged)?;
        let name = name.strip_suffix(&[0]).unwrap_or(name);
        for _ in 0..count {
            bytes.skip(8).ok_or(BsaError::Damaged)?;
            let size = bytes.u32().ok_or(BsaError::Damaged)?;
            let offset = bytes.u32().ok_or(BsaError::Damaged)?;
            records.push((size, offset));
        }
        folders.push((decode_text(name), count));
    }

    // File names, in the order of the file records.
    let names = bytes
        .take(usize::try_from(header.file_names_len).map_err(|_| BsaError::Damaged)?)
        .ok_or(BsaError::Damaged)?;
    let mut names = names.split(|&byte| byte == 0);

    let mut entries = Vec::with_capacity(records.len());
    let mut records = records.into_iter();
    for (folder, count) in folders {
        for _ in 0..count {
            let name = names.next().filter(|name| !name.is_empty());
            let name = name.ok_or(BsaError::Damaged)?;
            let (size, offset) = records.next().ok_or(BsaError::Damaged)?;
            entries.push(Entry {
                path: format!("{folder}\\{}", decode_text(name)),
                offset,
                size: size & !SIZE_TOGGLES_COMPRESSION,
                compressed: archive_compressed ^ (size & SIZE_TOGGLES_COMPRESSION != 0),
            });
        }
    }
    if u64::try_from(entries.len()).ok() != Some(u64::from(header.file_count)) {
        return Err(BsaError::Damaged);
    }
    Ok(Directory {
        entries,
        version: header.version,
        // Only the Skyrim versions of the format know this flag.
        embedded_names: header.version >= 104 && header.flags & FLAG_EMBEDDED_NAMES != 0,
    })
}

/// Turns the stored bytes of a file into its content.
///
/// `stored` are the [`Entry::size`] bytes at [`Entry::offset`] of the
/// archive that `directory` was read from.
pub fn unpack(stored: &[u8], entry: &Entry, directory: &Directory) -> Result<Vec<u8>, BsaError> {
    let mut bytes = Bytes::new(stored);
    if directory.embedded_names {
        // The file's path again, preceded by its length.
        let len = usize::from(bytes.u8().ok_or(BsaError::DamagedFile)?);
        bytes.skip(len).ok_or(BsaError::DamagedFile)?;
    }
    if !entry.compressed {
        return bytes
            .take(bytes.remaining())
            .map(<[u8]>::to_vec)
            .ok_or(BsaError::DamagedFile);
    }

    let unpacked_len = u64::from(bytes.u32().ok_or(BsaError::DamagedFile)?);
    if unpacked_len > MAX_UNPACKED {
        return Err(BsaError::DamagedFile);
    }
    let packed = bytes.take(bytes.remaining()).ok_or(BsaError::DamagedFile)?;
    let mut unpacked = Vec::new();
    let result = if directory.version >= 105 {
        FrameDecoder::new(packed)
            .take(unpacked_len)
            .read_to_end(&mut unpacked)
    } else {
        ZlibDecoder::new(packed)
            .take(unpacked_len)
            .read_to_end(&mut unpacked)
    };
    match result {
        Ok(_) if u64::try_from(unpacked.len()).ok() == Some(unpacked_len) => Ok(unpacked),
        _ => Err(BsaError::DamagedFile),
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::fixture;

    const FILES: &[&str] = &[
        r"textures\landscape\grass\fern01.dds",
        r"textures\landscape\grass\fern02.dds",
        r"meshes\landscape\grass\fern01.nif",
    ];

    fn sorted(mut names: Vec<String>) -> Vec<String> {
        names.sort();
        names
    }

    #[test]
    fn lists_the_files_of_an_archive() {
        let archive = fixture::bsa(FILES);
        let expected: Vec<String> = FILES.iter().map(|&file| file.to_owned()).collect();
        assert_eq!(sorted(parse_listing(&archive).unwrap()), sorted(expected));
    }

    #[test]
    fn the_listing_ends_where_listing_len_says() {
        let mut archive = fixture::bsa(FILES);
        let len = usize::try_from(listing_len(&archive).unwrap()).unwrap();
        assert_eq!(len, archive.len(), "the fixture stores no file contents");

        // File contents after the listing are not needed.
        archive.extend_from_slice(&[0xAB; 64]);
        assert_eq!(parse_listing(&archive[..len]).unwrap().len(), FILES.len());

        // One byte less than the listing is not enough.
        assert_eq!(parse_listing(&archive[..len - 1]), Err(BsaError::Damaged));
    }

    #[test]
    fn older_versions_use_shorter_folder_records() {
        let archive = fixture::bsa_with_version(FILES, 104);
        assert_eq!(parse_listing(&archive).unwrap().len(), FILES.len());
    }

    fn options(version: u32, compressed: bool, embedded_names: bool) -> fixture::BsaOptions {
        fixture::BsaOptions {
            version,
            compressed,
            embedded_names,
        }
    }

    /// Reads a file back out of an archive the way a caller with a file on
    /// disk does: directory first, then the bytes the entry points at.
    fn read_back(archive: &[u8], path: &str) -> Result<Vec<u8>, BsaError> {
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
    fn files_are_read_back_from_every_kind_of_archive() {
        let mesh = b"a mesh, repeated: a mesh, repeated: a mesh, repeated".as_slice();
        let files = [(r"meshes\wall.nif", mesh), (r"textures\wall.dds", b"tex")];

        for version in [104, 105] {
            for compressed in [false, true] {
                for embedded_names in [false, true] {
                    let archive =
                        fixture::bsa_with(&files, options(version, compressed, embedded_names));
                    assert_eq!(
                        read_back(&archive, r"meshes\wall.nif").unwrap(),
                        mesh,
                        "version {version}, compressed {compressed}, names {embedded_names}"
                    );
                    assert_eq!(read_back(&archive, r"textures\wall.dds").unwrap(), b"tex");
                }
            }
        }
    }

    #[test]
    fn damaged_file_data_is_an_error_not_a_crash() {
        let files = [(r"meshes\wall.nif", b"some mesh data".as_slice())];
        let archive = fixture::bsa_with(&files, options(105, true, false));
        let directory = parse_directory(&archive).unwrap();
        let entry = &directory.entries[0];

        assert_eq!(unpack(&[], entry, &directory), Err(BsaError::DamagedFile));
        assert_eq!(
            unpack(&[9, 0, 0, 0, 1, 2, 3], entry, &directory),
            Err(BsaError::DamagedFile)
        );
        // A size that claims four gigabytes.
        assert_eq!(
            unpack(&[0xFF; 12], entry, &directory),
            Err(BsaError::DamagedFile)
        );
    }

    #[test]
    fn an_empty_archive_lists_nothing() {
        assert_eq!(
            parse_listing(&fixture::bsa(&[])).unwrap(),
            Vec::<String>::new()
        );
    }

    #[test]
    fn other_files_are_rejected() {
        assert_eq!(
            parse_listing(b"TES4 and so on"),
            Err(BsaError::NotAnArchive)
        );
        assert_eq!(parse_listing(b""), Err(BsaError::NotAnArchive));
        assert_eq!(listing_len(b"BSA\0"), Err(BsaError::Damaged));
    }

    #[test]
    fn unknown_versions_are_rejected() {
        let archive = fixture::bsa_with_version(FILES, 1);
        assert_eq!(
            parse_listing(&archive),
            Err(BsaError::UnsupportedVersion(1))
        );
    }

    #[test]
    fn an_archive_without_names_is_rejected() {
        let mut archive = fixture::bsa(FILES);
        // Clear the flags that say names are stored.
        archive[12..16].copy_from_slice(&0u32.to_le_bytes());
        assert_eq!(parse_listing(&archive), Err(BsaError::NamesMissing));
    }

    #[test]
    fn a_header_that_promises_too_much_is_damaged_not_fatal() {
        let mut archive = fixture::bsa(FILES);
        // Claim four billion folders.
        archive[16..20].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(parse_listing(&archive), Err(BsaError::Damaged));
    }

    proptest! {
        #[test]
        fn never_panics_on_arbitrary_bytes(data in proptest::collection::vec(any::<u8>(), 0..512)) {
            let _ = listing_len(&data);
            let _ = parse_listing(&data);
        }

        #[test]
        fn unpacking_never_panics(
            stored in proptest::collection::vec(any::<u8>(), 0..200),
            version in 103u32..106,
            compressed in any::<bool>(),
            embedded_names in any::<bool>(),
        ) {
            let files = [(r"meshes\wall.nif", b"x".as_slice())];
            let archive = fixture::bsa_with(&files, options(version.max(104), compressed, embedded_names));
            let directory = parse_directory(&archive).unwrap();
            let _ = unpack(&stored, &directory.entries[0], &directory);
        }

        #[test]
        fn never_panics_on_a_damaged_archive(
            position in 0usize..200,
            value in any::<u8>(),
            cut in 0usize..200,
        ) {
            let mut archive = fixture::bsa(FILES);
            if let Some(byte) = archive.get_mut(position) {
                *byte = value;
            }
            archive.truncate(archive.len().saturating_sub(cut));
            let _ = listing_len(&archive);
            let _ = parse_listing(&archive);
        }
    }
}
