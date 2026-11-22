//! Reading a patch back from its archive (ADR 0070).
//!
//! The user picks an archive: one Mod Mallet wrote, or one somebody shared.
//! Its manifest gives the decisions; an archive without one is read from
//! its swap files. Nothing is installed and nothing is written into the
//! game: what was read goes back to the window, which adds it to the user's
//! patches.

use std::io::Read;

use mallet_core::{LoadedPatch, PATCH_MANIFEST_FILE, PATCH_MANIFEST_FORMAT, Patch, PatchManifest};
use mallet_game_bethesda::swap;

use crate::EngineError;

/// Archives larger than this are not read: a patch is a few kilobytes of
/// text and two small plugins.
const MAX_ARCHIVE_BYTES: usize = 64 * 1024 * 1024;

/// Files inside an archive larger than this are not read.
const MAX_FILE_BYTES: u64 = 8 * 1024 * 1024;

/// Reads the patch in the archive `bytes`, named `archive` as the user
/// picked it. Zip archives are read here; others through 7-Zip, where it
/// is installed.
///
/// # Errors
///
/// If the archive cannot be opened, holds no patch of Mod Mallet, or its
/// manifest is of a format this Mod Mallet does not know.
pub fn load_patch(archive: &str, bytes: &[u8]) -> Result<LoadedPatch, EngineError> {
    if bytes.len() > MAX_ARCHIVE_BYTES {
        return Err(EngineError::ArchiveUnreadable(
            "it is too large for a patch".to_owned(),
        ));
    }
    let files = match zip_entries(bytes) {
        Some(entries) => entries,
        None => with_7zip(archive, bytes)?,
    };
    let loaded = patch_of(archive, &files)?;
    tracing::info!(
        archive,
        manifest = loaded.manifest.is_some(),
        parts = loaded.patch.parts.len(),
        "patch loaded from an archive"
    );
    Ok(loaded)
}

/// The patch the files of an archive describe.
fn patch_of(archive: &str, files: &[(String, Vec<u8>)]) -> Result<LoadedPatch, EngineError> {
    let manifest = files
        .iter()
        .find(|(name, _)| file_name(name).eq_ignore_ascii_case(PATCH_MANIFEST_FILE))
        .map(|(_, bytes)| manifest_of(bytes))
        .transpose()?;
    let mut patch: Option<Patch> = None;
    for (name, bytes) in files {
        if !swap::is_swap_file(file_name(name)) {
            continue;
        }
        let text = String::from_utf8_lossy(bytes);
        let Some(part) = swap::read_part(&text) else {
            continue;
        };
        match &mut patch {
            Some(known) => known.parts.extend(part.parts),
            None => patch = Some(part),
        }
    }
    if manifest.is_none() && patch.is_none() {
        return Err(EngineError::NotAPatch);
    }
    let patch = patch.unwrap_or_else(|| Patch {
        name: manifest
            .as_ref()
            .map_or_else(String::new, |manifest| manifest.name.clone()),
        ..Patch::default()
    });
    Ok(LoadedPatch {
        archive: archive.to_owned(),
        manifest,
        patch,
    })
}

/// The manifest in `bytes`.
fn manifest_of(bytes: &[u8]) -> Result<PatchManifest, EngineError> {
    #[derive(serde::Deserialize)]
    struct Format {
        format: u32,
    }
    let format: Format = serde_json::from_slice(bytes).map_err(|_| EngineError::NotAPatch)?;
    if format.format != PATCH_MANIFEST_FORMAT {
        return Err(EngineError::ManifestFormat(format.format));
    }
    serde_json::from_slice(bytes).map_err(|_| EngineError::NotAPatch)
}

/// The last part of a path in an archive.
fn file_name(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

/// Whether a file of an archive is one a patch is read from.
fn wanted(name: &str) -> bool {
    let name = file_name(name);
    swap::is_swap_file(name) || name.eq_ignore_ascii_case(PATCH_MANIFEST_FILE)
}

/// The files of a zip archive that a patch is read from, by their paths;
/// `None` if `bytes` is not a zip archive. Entries that cannot be read are
/// left out.
fn zip_entries(bytes: &[u8]) -> Option<Vec<(String, Vec<u8>)>> {
    let end = end_of_directory(bytes)?;
    let count = usize::from(u16_at(bytes, end + 10)?);
    let mut at = usize::try_from(u32_at(bytes, end + 16)?).ok()?;
    let mut files = Vec::new();
    for _ in 0..count {
        if u32_at(bytes, at)? != 0x0201_4b50 {
            break;
        }
        let method = u16_at(bytes, at + 10)?;
        let packed = u64::from(u32_at(bytes, at + 20)?);
        let size = u64::from(u32_at(bytes, at + 24)?);
        let name_length = usize::from(u16_at(bytes, at + 28)?);
        let extra_length = usize::from(u16_at(bytes, at + 30)?);
        let comment_length = usize::from(u16_at(bytes, at + 32)?);
        let header = usize::try_from(u32_at(bytes, at + 42)?).ok()?;
        let name = String::from_utf8_lossy(bytes.get(at + 46..at + 46 + name_length)?).into_owned();
        at += 46 + name_length + extra_length + comment_length;
        if !wanted(&name) || size > MAX_FILE_BYTES {
            continue;
        }
        if let Some(content) = entry(bytes, header, method, packed, size) {
            files.push((name, content));
        }
    }
    Some(files)
}

/// The content of the entry whose local header starts at `header`.
fn entry(bytes: &[u8], header: usize, method: u16, packed: u64, size: u64) -> Option<Vec<u8>> {
    if u32_at(bytes, header)? != 0x0403_4b50 {
        return None;
    }
    let name_length = usize::from(u16_at(bytes, header + 26)?);
    let extra_length = usize::from(u16_at(bytes, header + 28)?);
    let start = header + 30 + name_length + extra_length;
    let data = bytes.get(start..start + usize::try_from(packed).ok()?)?;
    match method {
        0 => Some(data.to_vec()),
        8 => {
            let mut content = Vec::new();
            flate2::read::DeflateDecoder::new(data)
                .take(size)
                .read_to_end(&mut content)
                .ok()?;
            Some(content)
        }
        _ => None,
    }
}

/// Where the end of the central directory starts, looked for from the end.
fn end_of_directory(bytes: &[u8]) -> Option<usize> {
    let last = bytes.len().checked_sub(22)?;
    let first = last.saturating_sub(0xFFFF);
    (first..=last)
        .rev()
        .find(|&at| u32_at(bytes, at) == Some(0x0605_4b50))
}

fn u16_at(bytes: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(bytes.get(at..at + 2)?.try_into().ok()?))
}

fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

/// The files of a patch in an archive of another kind, such as 7z, read
/// through 7-Zip from a copy in the system's folder for temporary files.
fn with_7zip(archive: &str, bytes: &[u8]) -> Result<Vec<(String, Vec<u8>)>, EngineError> {
    let unpacker = mallet_unpack::Unpacker::locate().ok_or_else(|| {
        EngineError::ArchiveUnreadable("it is no zip archive, and 7-Zip was not found".to_owned())
    })?;
    let extension = archive
        .rsplit_once('.')
        .map(|(_, extension)| extension)
        .filter(|extension| extension.chars().all(|c| c.is_ascii_alphanumeric()))
        .unwrap_or("7z");
    let copy = std::env::temp_dir().join(format!(
        "mod-mallet-patch-{}.{extension}",
        std::process::id()
    ));
    std::fs::write(&copy, bytes)
        .map_err(|error| EngineError::ArchiveUnreadable(error.to_string()))?;
    let read = || -> Result<Vec<(String, Vec<u8>)>, EngineError> {
        let names = unpacker
            .list(&copy)
            .map_err(|error| EngineError::ArchiveUnreadable(error.to_string()))?;
        Ok(names
            .into_iter()
            .filter(|name| wanted(name))
            .filter_map(|name| {
                let content = unpacker.read(&copy, &name).ok()?;
                Some((name, content))
            })
            .collect())
    };
    let files = read();
    let _ = std::fs::remove_file(&copy);
    files
}

#[cfg(test)]
mod tests {
    use mallet_core::{ObjectRef, PatchPart};
    use proptest::prelude::*;

    use super::*;

    fn patch() -> Patch {
        Patch {
            name: "Two Towns".to_owned(),
            author: Some("Someone".to_owned()),
            version: Some("1.2".to_owned()),
            parts: vec![PatchPart {
                plugin: "Roads.esp".to_owned(),
                against: Some("Town.esp".to_owned()),
                objects: vec![ObjectRef {
                    plugin: "Roads.esp".to_owned(),
                    local_id: 0x812,
                }],
            }],
            manifest: Some(PatchManifest {
                format: 0,
                name: String::new(),
                decisions: [("spot:a|b".to_owned(), Some("Roads.esp".to_owned()))]
                    .into_iter()
                    .collect(),
                plugins: vec!["Roads.esp".to_owned(), "Town.esp".to_owned()],
                ..PatchManifest::default()
            }),
            ..Patch::default()
        }
    }

    fn archive(patch: &Patch) -> Vec<u8> {
        let folder = tempfile::tempdir().unwrap();
        let file = folder.path().join("patch.zip");
        crate::patch::write_zip(
            &file,
            &crate::patch::patch_files(
                patch,
                &[("P - Mod Mallet Moves.esp".to_owned(), b"TES4".to_vec())],
            ),
        )
        .unwrap();
        std::fs::read(file).unwrap()
    }

    #[test]
    fn a_written_patch_is_read_back_with_its_decisions() {
        let loaded = load_patch("Two Towns.zip", &archive(&patch())).unwrap();
        let manifest = loaded.manifest.unwrap();
        assert_eq!(manifest.format, PATCH_MANIFEST_FORMAT);
        assert_eq!(manifest.name, "Two Towns");
        assert_eq!(manifest.author.as_deref(), Some("Someone"));
        assert_eq!(
            manifest.decisions.get("spot:a|b"),
            Some(&Some("Roads.esp".to_owned()))
        );
        assert_eq!(loaded.patch.parts.len(), 1);
        assert_eq!(loaded.patch.objects().len(), 1);
    }

    #[test]
    fn an_archive_without_a_manifest_is_read_from_its_swap_files() {
        let mut old = patch();
        old.manifest = None;
        let loaded = load_patch("Two Towns.zip", &archive(&old)).unwrap();
        assert!(loaded.manifest.is_none());
        assert_eq!(loaded.patch.name, "Two Towns");
        assert_eq!(loaded.patch.objects().len(), 1);
    }

    #[test]
    fn a_compressed_entry_is_read() {
        use std::io::Write;
        let text = br#"{"format":1,"name":"Packed","decisions":{"object:A.esp|1":"A.esp"}}"#;
        let mut packed =
            flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
        packed.write_all(text).unwrap();
        let packed = packed.finish().unwrap();
        // A hand-made archive with one deflated entry.
        let name = PATCH_MANIFEST_FILE.as_bytes();
        let mut zip = Vec::new();
        zip.extend(0x0403_4b50u32.to_le_bytes());
        zip.extend([20, 0, 0, 0, 8, 0, 0, 0, 0x21, 0]);
        zip.extend(0u32.to_le_bytes());
        zip.extend(u32::try_from(packed.len()).unwrap().to_le_bytes());
        zip.extend(u32::try_from(text.len()).unwrap().to_le_bytes());
        zip.extend(u16::try_from(name.len()).unwrap().to_le_bytes());
        zip.extend(0u16.to_le_bytes());
        zip.extend(name);
        zip.extend(&packed);
        let directory = zip.len();
        zip.extend(0x0201_4b50u32.to_le_bytes());
        zip.extend([20, 0, 20, 0, 0, 0, 8, 0, 0, 0, 0x21, 0]);
        zip.extend(0u32.to_le_bytes());
        zip.extend(u32::try_from(packed.len()).unwrap().to_le_bytes());
        zip.extend(u32::try_from(text.len()).unwrap().to_le_bytes());
        zip.extend(u16::try_from(name.len()).unwrap().to_le_bytes());
        zip.extend([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        zip.extend(0u32.to_le_bytes());
        zip.extend(name);
        let size = zip.len() - directory;
        zip.extend(0x0605_4b50u32.to_le_bytes());
        zip.extend([0, 0, 0, 0, 1, 0, 1, 0]);
        zip.extend(u32::try_from(size).unwrap().to_le_bytes());
        zip.extend(u32::try_from(directory).unwrap().to_le_bytes());
        zip.extend(0u16.to_le_bytes());
        let loaded = load_patch("packed.zip", &zip).unwrap();
        assert_eq!(loaded.manifest.unwrap().name, "Packed");
    }

    #[test]
    fn a_newer_manifest_is_refused() {
        let files = vec![(
            PATCH_MANIFEST_FILE.to_owned(),
            br#"{"format":99,"name":"x","decisions":{}}"#.to_vec(),
        )];
        assert!(matches!(
            patch_of("x.zip", &files),
            Err(EngineError::ManifestFormat(99))
        ));
    }

    #[test]
    fn an_archive_without_a_patch_is_refused() {
        let files = vec![("readme.txt".to_owned(), b"hello".to_vec())];
        assert!(matches!(
            patch_of("x.zip", &files),
            Err(EngineError::NotAPatch)
        ));
    }

    proptest! {
        #[test]
        fn reading_any_bytes_never_panics(bytes in proptest::collection::vec(any::<u8>(), 0..512)) {
            let _ = zip_entries(&bytes);
        }

        #[test]
        fn reading_a_damaged_archive_never_panics(cut in 0usize..2000, flip in 0usize..2000) {
            let mut bytes = archive(&patch());
            if let Some(byte) = bytes.get_mut(flip) {
                *byte ^= 0x5A;
            }
            bytes.truncate(bytes.len().saturating_sub(cut % 64));
            let _ = zip_entries(&bytes);
        }
    }
}
