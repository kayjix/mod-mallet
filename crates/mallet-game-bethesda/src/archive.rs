//! The archives of all the games behind one face: the `.bsa` archives of
//! Skyrim, Oblivion and the older Fallouts, and the `.ba2` archives of
//! Fallout 4. Which kind a file is, is read from its first bytes.

use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

use mallet_formats_bethesda::{ba2, bsa};

/// The largest archive file list that is read. Far above any real archive;
/// it keeps a damaged header from asking for gigabytes of memory.
const MAX_LISTING: u64 = 512 * 1024 * 1024;

/// The largest single file that is taken out of an archive.
const MAX_FILE: u64 = 256 * 1024 * 1024;

/// The directory of an archive: the files in it and how they are stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Directory {
    /// A `.bsa` archive.
    Bsa(bsa::Directory),
    /// A `.ba2` archive.
    Ba2(ba2::Directory),
}

impl Directory {
    /// The paths of the files, in the archive's order and spelling.
    pub(crate) fn paths(&self) -> Vec<&str> {
        match self {
            Self::Bsa(directory) => directory
                .entries
                .iter()
                .map(|entry| entry.path.as_str())
                .collect(),
            Self::Ba2(directory) => directory
                .entries
                .iter()
                .map(|entry| entry.path.as_str())
                .collect(),
        }
    }

    /// The content of the file at `index` in [`Self::paths`], from the
    /// archive opened as `file`. `None` if it cannot be read or is too
    /// large to show.
    pub(crate) fn read(&self, file: &mut File, index: usize) -> Option<Vec<u8>> {
        match self {
            Self::Bsa(directory) => {
                let entry = directory.entries.get(index)?;
                if u64::from(entry.size) > MAX_FILE {
                    return None;
                }
                file.seek(SeekFrom::Start(u64::from(entry.offset))).ok()?;
                let mut stored = Vec::new();
                file.take(u64::from(entry.size))
                    .read_to_end(&mut stored)
                    .ok()?;
                bsa::unpack(&stored, entry, directory).ok()
            }
            Self::Ba2(directory) => {
                let entry = directory.entries.get(index)?;
                if entry.size > MAX_FILE {
                    return None;
                }
                ba2::read_file(file, entry, directory).ok()?.ok()
            }
        }
    }
}

/// Reads the directory of the archive at `path`, whichever kind it is.
pub(crate) fn read_directory(path: &Path) -> Result<Directory, String> {
    let read = || -> io::Result<Result<Directory, String>> {
        let mut file = File::open(path)?;
        let mut magic = [0u8; 4];
        file.read_exact(&mut magic)?;
        file.seek(SeekFrom::Start(0))?;
        if &magic == b"BTDX" {
            return Ok(ba2::read_directory(&mut file)?
                .map(Directory::Ba2)
                .map_err(|error| error.to_string()));
        }
        let mut data = vec![0u8; bsa::HEADER_LEN];
        file.read_exact(&mut data)?;
        let listing_len = match bsa::listing_len(&data) {
            Ok(len) => len.min(MAX_LISTING),
            Err(error) => return Ok(Err(error.to_string())),
        };
        let rest = listing_len.saturating_sub(bsa::HEADER_LEN as u64);
        file.take(rest).read_to_end(&mut data)?;
        Ok(bsa::parse_directory(&data)
            .map(Directory::Bsa)
            .map_err(|error| error.to_string()))
    };
    read().unwrap_or_else(|error| Err(error.to_string()))
}

#[cfg(test)]
mod tests {
    use mallet_formats_bethesda::fixture;

    use super::*;

    #[test]
    fn both_kinds_of_archive_are_listed_and_read() {
        let dir = tempfile::tempdir().unwrap();
        let bsa_path = dir.path().join("a.bsa");
        std::fs::write(
            &bsa_path,
            fixture::bsa_with(
                &[("meshes\\a.nif", b"first".as_slice())],
                fixture::BsaOptions {
                    version: 105,
                    compressed: true,
                    embedded_names: false,
                },
            ),
        )
        .unwrap();
        let ba2_path = dir.path().join("b.ba2");
        std::fs::write(
            &ba2_path,
            fixture::ba2_with(
                &[("meshes\\b.nif", b"second".as_slice())],
                fixture::Ba2Options {
                    version: 1,
                    compressed: true,
                },
            ),
        )
        .unwrap();

        for (path, name, content) in [
            (&bsa_path, "meshes\\a.nif", b"first".as_slice()),
            (&ba2_path, "meshes\\b.nif", b"second".as_slice()),
        ] {
            let directory = read_directory(path).unwrap();
            assert_eq!(directory.paths(), [name]);
            let mut file = File::open(path).unwrap();
            assert_eq!(directory.read(&mut file, 0).unwrap(), content);
            assert_eq!(directory.read(&mut file, 1), None);
        }
    }

    #[test]
    fn a_file_that_is_no_archive_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("x.bsa");
        std::fs::write(
            &path,
            b"not an archive at all, but long enough to have a header",
        )
        .unwrap();
        assert!(read_directory(&path).is_err());
        assert!(read_directory(&dir.path().join("missing.ba2")).is_err());
    }
}
