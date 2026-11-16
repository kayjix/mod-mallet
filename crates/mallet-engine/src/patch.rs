//! Writing the small mods Mod Mallet hands to the mod manager.
//!
//! A patch is one text file in a zip archive, which the mod manager
//! installs like any downloaded mod. The archive is written into Mod
//! Mallet's own folder; nothing is written into the game or the mod
//! manager's data (ADR 0025).

use std::io::Write;
use std::path::Path;

/// Writes a zip archive at `path` that holds one file, `name`, with
/// `content`. The file is stored without compression: it is a few
/// kilobytes of text.
pub(crate) fn write_zip(path: &Path, name: &str, content: &[u8]) -> std::io::Result<()> {
    let too_large = || std::io::Error::other("the file is too large for a zip archive");
    let size = u32::try_from(content.len()).map_err(|_| too_large())?;
    let name_length = u16::try_from(name.len()).map_err(|_| too_large())?;
    let mut crc = flate2::Crc::new();
    crc.update(content);
    let checksum = crc.sum();

    // The fields both headers share: version, flags, method, time, date,
    // checksum, sizes, name length.
    let mut shared = Vec::new();
    shared.extend(20u16.to_le_bytes());
    shared.extend(0u16.to_le_bytes());
    shared.extend(0u16.to_le_bytes());
    shared.extend(0u16.to_le_bytes());
    // 1 January 1980, the earliest date the format knows.
    shared.extend(0x21u16.to_le_bytes());
    shared.extend(checksum.to_le_bytes());
    shared.extend(size.to_le_bytes());
    shared.extend(size.to_le_bytes());
    shared.extend(name_length.to_le_bytes());

    let mut archive = Vec::new();
    archive.extend(0x0403_4b50u32.to_le_bytes());
    archive.extend(&shared);
    archive.extend(0u16.to_le_bytes());
    archive.extend(name.as_bytes());
    archive.extend(content);

    let directory_start = u32::try_from(archive.len()).map_err(|_| too_large())?;
    let mut directory = Vec::new();
    directory.extend(0x0201_4b50u32.to_le_bytes());
    directory.extend(20u16.to_le_bytes());
    directory.extend(&shared);
    // Extra field, comment, disk, internal and external attributes, and
    // where the file's header starts.
    directory.extend(0u16.to_le_bytes());
    directory.extend(0u16.to_le_bytes());
    directory.extend(0u16.to_le_bytes());
    directory.extend(0u16.to_le_bytes());
    directory.extend(0u32.to_le_bytes());
    directory.extend(0u32.to_le_bytes());
    directory.extend(name.as_bytes());
    let directory_size = u32::try_from(directory.len()).map_err(|_| too_large())?;
    archive.extend(directory);

    archive.extend(0x0605_4b50u32.to_le_bytes());
    archive.extend(0u16.to_le_bytes());
    archive.extend(0u16.to_le_bytes());
    archive.extend(1u16.to_le_bytes());
    archive.extend(1u16.to_le_bytes());
    archive.extend(directory_size.to_le_bytes());
    archive.extend(directory_start.to_le_bytes());
    archive.extend(0u16.to_le_bytes());

    if let Some(directory) = path.parent() {
        std::fs::create_dir_all(directory)?;
    }
    // Written under another name first, so that nobody reads half an
    // archive.
    let temporary = path.with_extension("tmp");
    std::fs::File::create(&temporary)?.write_all(&archive)?;
    std::fs::rename(&temporary, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_written_archive_can_be_read_by_7zip() {
        let Some(unpacker) = mallet_unpack::Unpacker::locate() else {
            eprintln!("no 7-Zip found, skipping");
            return;
        };
        let directory = tempfile::tempdir().unwrap();
        let archive = directory.path().join("patch.zip");
        write_zip(&archive, "ModMallet_SWAP.ini", b"[References]\n").unwrap();

        assert_eq!(unpacker.list(&archive).unwrap(), ["ModMallet_SWAP.ini"]);
        assert_eq!(
            unpacker.read(&archive, "ModMallet_SWAP.ini").unwrap(),
            b"[References]\n"
        );
    }

    #[test]
    fn the_archive_ends_with_its_directory() {
        let directory = tempfile::tempdir().unwrap();
        let archive = directory.path().join("nested").join("patch.zip");
        write_zip(&archive, "a.ini", b"abc").unwrap();
        let bytes = std::fs::read(&archive).unwrap();
        assert_eq!(&bytes[..4], b"PK\x03\x04");
        assert_eq!(&bytes[bytes.len() - 22..bytes.len() - 18], b"PK\x05\x06");
    }
}
