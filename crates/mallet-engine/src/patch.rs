//! Writing the small mods Mod Mallet hands to the mod manager.
//!
//! A patch is a zip archive of text files, which the mod manager installs
//! like any downloaded mod. The archive is written into Mod Mallet's own
//! folder; nothing is written into the game or the mod manager's data
//! (ADR 0025). It carries a FOMOD installer, so that each part can be left
//! out wherever it is installed (ADR 0053).

use std::io::Write;
use std::path::Path;

use mallet_core::Patch;
use mallet_format_fomod::write::{Choice, Installer};
use mallet_game_bethesda::swap;

/// The folder in the archive that holds the parts' swap files, which the
/// installer copies into the data directory.
const PARTS: &str = "parts";

/// The files of the archive of `patch`, by their paths in the archive:
/// one swap file per part, and the installer that offers them.
pub(crate) fn patch_files(patch: &Patch) -> Vec<(String, Vec<u8>)> {
    let names = swap::part_file_names(patch);
    let mut files = Vec::new();
    let mut options = Vec::new();
    for (part, name) in patch.parts.iter().zip(names) {
        let source = format!("{PARTS}/{name}");
        files.push((source.clone(), swap::part_ini(patch, part).into_bytes()));
        let mut needs = vec![part.plugin.clone()];
        needs.extend(part.against.iter().cloned());
        let count = part.objects.len();
        let objects = if count == 1 { "object" } else { "objects" };
        options.push(Choice {
            name: match &part.against {
                Some(against) => format!("{} (for {against})", part.plugin),
                None => part.plugin.clone(),
            },
            description: match &part.against {
                Some(against) => format!(
                    "Hides {count} {objects} of {} where they collide with {against}.",
                    part.plugin
                ),
                None => format!("Hides {count} {objects} of {}.", part.plugin),
            },
            files: vec![(source, name)],
            needs,
        });
    }
    let installer = Installer {
        name: patch.name.clone(),
        author: patch.author.clone(),
        version: patch.version.clone(),
        description: "Hides placed objects through Base Object Swapper. Made with Mod Mallet."
            .to_owned(),
        page: "Parts".to_owned(),
        group: "Mods the patch changes".to_owned(),
        options,
    };
    files.push((
        "fomod/info.xml".to_owned(),
        mallet_format_fomod::write::info(&installer).into_bytes(),
    ));
    files.push((
        "fomod/ModuleConfig.xml".to_owned(),
        mallet_format_fomod::write::module_config(&installer).into_bytes(),
    ));
    files
}

/// Writes a zip archive at `path` that holds `files`, each by its path in
/// the archive and its content. The files are stored without compression:
/// they are a few kilobytes of text each.
pub(crate) fn write_zip(path: &Path, files: &[(String, Vec<u8>)]) -> std::io::Result<()> {
    let too_large = || std::io::Error::other("the files are too large for a zip archive");
    let mut archive = Vec::new();
    let mut directory = Vec::new();
    for (name, content) in files {
        let size = u32::try_from(content.len()).map_err(|_| too_large())?;
        let name_length = u16::try_from(name.len()).map_err(|_| too_large())?;
        let header_start = u32::try_from(archive.len()).map_err(|_| too_large())?;
        let mut crc = flate2::Crc::new();
        crc.update(content);
        let checksum = crc.sum();

        // The fields both headers share: version, flags, method, time,
        // date, checksum, sizes, name length.
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

        archive.extend(0x0403_4b50u32.to_le_bytes());
        archive.extend(&shared);
        archive.extend(0u16.to_le_bytes());
        archive.extend(name.as_bytes());
        archive.extend(content);

        directory.extend(0x0201_4b50u32.to_le_bytes());
        directory.extend(20u16.to_le_bytes());
        directory.extend(&shared);
        // Extra field, comment, disk, internal and external attributes,
        // and where the file's header starts.
        directory.extend(0u16.to_le_bytes());
        directory.extend(0u16.to_le_bytes());
        directory.extend(0u16.to_le_bytes());
        directory.extend(0u16.to_le_bytes());
        directory.extend(0u32.to_le_bytes());
        directory.extend(header_start.to_le_bytes());
        directory.extend(name.as_bytes());
    }

    let count = u16::try_from(files.len()).map_err(|_| too_large())?;
    let directory_start = u32::try_from(archive.len()).map_err(|_| too_large())?;
    let directory_size = u32::try_from(directory.len()).map_err(|_| too_large())?;
    archive.extend(directory);
    archive.extend(0x0605_4b50u32.to_le_bytes());
    archive.extend(0u16.to_le_bytes());
    archive.extend(0u16.to_le_bytes());
    archive.extend(count.to_le_bytes());
    archive.extend(count.to_le_bytes());
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
    use mallet_core::{ObjectRef, PatchPart};

    use super::*;

    fn files(names: &[&str]) -> Vec<(String, Vec<u8>)> {
        names
            .iter()
            .map(|name| ((*name).to_owned(), format!("[{name}]\n").into_bytes()))
            .collect()
    }

    #[test]
    fn a_written_archive_can_be_read_by_7zip() {
        let Some(unpacker) = mallet_unpack::Unpacker::locate() else {
            eprintln!("no 7-Zip found, skipping");
            return;
        };
        let directory = tempfile::tempdir().unwrap();
        let archive = directory.path().join("patch.zip");
        write_zip(&archive, &files(&["a_SWAP.ini", "fomod/info.xml"])).unwrap();

        // 7-Zip lists folders with the separator of the system.
        let mut listed: Vec<String> = unpacker
            .list(&archive)
            .unwrap()
            .into_iter()
            .map(|name| name.replace('\\', "/"))
            .collect();
        listed.sort();
        assert_eq!(listed, ["a_SWAP.ini", "fomod/info.xml"]);
        assert_eq!(
            unpacker.read(&archive, "a_SWAP.ini").unwrap(),
            b"[a_SWAP.ini]\n"
        );
    }

    #[test]
    fn the_archive_ends_with_its_directory() {
        let directory = tempfile::tempdir().unwrap();
        let archive = directory.path().join("nested").join("patch.zip");
        write_zip(&archive, &files(&["a.ini", "b.ini"])).unwrap();
        let bytes = std::fs::read(&archive).unwrap();
        assert_eq!(&bytes[..4], b"PK\x03\x04");
        let end = &bytes[bytes.len() - 22..];
        assert_eq!(&end[..4], b"PK\x05\x06");
        assert_eq!(end[8..10], 2u16.to_le_bytes());
    }

    #[test]
    fn a_patch_has_a_swap_file_per_part_and_an_installer() {
        let object = |plugin: &str, local_id| ObjectRef {
            plugin: plugin.to_owned(),
            local_id,
        };
        let patch = Patch {
            name: "Mod Mallet Patch".to_owned(),
            author: None,
            version: None,
            parts: vec![
                PatchPart {
                    plugin: "Roads.esp".to_owned(),
                    against: Some("Town.esp".to_owned()),
                    objects: vec![object("Roads.esp", 1), object("Roads.esp", 2)],
                },
                PatchPart {
                    plugin: "Roads.esp".to_owned(),
                    against: None,
                    objects: vec![object("Roads.esp", 3)],
                },
            ],
        };
        let files = patch_files(&patch);
        let names: Vec<&str> = files.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(
            names,
            [
                "parts/Mod Mallet Patch - Roads - Town_SWAP.ini",
                "parts/Mod Mallet Patch - Roads_SWAP.ini",
                "fomod/info.xml",
                "fomod/ModuleConfig.xml",
            ]
        );
        let config = String::from_utf8(files[3].1.clone()).unwrap();
        let steps = mallet_format_fomod::parse(&config).unwrap();
        let options = &steps[0].groups[0].options;
        assert_eq!(options[0].name, "Roads.esp (for Town.esp)");
        assert_eq!(options[1].name, "Roads.esp");
        assert!(config.contains("destination=\"Mod Mallet Patch - Roads_SWAP.ini\""));
        let part = String::from_utf8(files[0].1.clone()).unwrap();
        assert_eq!(swap::read_part(&part).unwrap().parts[0].objects.len(), 2);
    }
}
