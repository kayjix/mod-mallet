//! Two mods' versions of files other than textures and meshes, for the
//! window to set side by side (ADR 0066): every copy of each file the
//! setup has, in archives, loose in the data directory or kept back in the
//! mod manager's folder, with what can be told about it.
//!
//! Only reads.

use std::path::{Path, PathBuf};

use mallet_core::compare::{
    ComparedVersion, FileComparison, ScriptFunction, ScriptFunctionKind, ScriptSummary,
};
use mallet_core::{GamePath, ModRef, ModSource};
use mallet_game_bethesda::files::{self, FunctionKind, Origin};
use mallet_source_vortex::VortexDeployment;

use crate::ScanOptions;

/// How many files of one duel are compared.
const MAX_FILES: usize = 400;

/// Texts longer than this are cut.
const MAX_TEXT: usize = 256 * 1024;

/// Pictures larger than this are not shown.
const MAX_IMAGE: usize = 4 * 1024 * 1024;

/// Loose copies in a mod's folder larger than this are not read.
const MAX_STAGED: u64 = 64 * 1024 * 1024;

/// The endings of files that are text: settings, script sources, notes.
const TEXT: [&str; 13] = [
    "json", "ini", "toml", "txt", "psc", "xml", "yaml", "yml", "cfg", "csv", "md", "log", "html",
];

/// Every version of each of `paths` (below the data directory), in the
/// same order, compared. `mods` names the mod manager's folders of the
/// mods concerned; their loose copies that are not in the game are read
/// from there. `None` if the setup cannot be read.
pub fn compare_files(
    options: &ScanOptions,
    paths: &[String],
    mods: &[String],
) -> Option<Vec<FileComparison>> {
    let game = options.game().ok()?;
    let (game_dir, _) = options.located_game_dir()?;
    let data_dir = game_dir.join("Data");
    let paths: Vec<&str> = paths.iter().take(MAX_FILES).map(String::as_str).collect();
    let found = files::versions_of_each(game, &game_dir, options.local_dir.as_deref(), &paths);
    let deployment = deployment(options, &game_dir);
    let staging = deployment.as_ref().and_then(|deployment| {
        options
            .staging_dir
            .clone()
            .or_else(|| deployment.staging_path().map(PathBuf::from))
    });
    let provider_of = |path: &str| {
        deployment
            .as_ref()
            .and_then(|deployment| deployment.provider_of(&GamePath::new(&format!("data/{path}"))))
    };

    let comparisons = paths
        .iter()
        .zip(found)
        .map(|(path, versions)| {
            let normal = files::normalize(path);
            let mut read: Vec<(ComparedVersion, Vec<u8>)> = versions
                .into_iter()
                .map(|version| {
                    let (archive, provider, on_disk) = match version.origin {
                        Origin::Loose => (None, provider_of(&normal), data_dir.join(&normal)),
                        Origin::Archive(name) => {
                            let provider = provider_of(&name);
                            let on_disk = data_dir.join(&name);
                            (Some(name), provider, on_disk)
                        }
                    };
                    (
                        describe(&normal, archive, false, provider, &on_disk, &version.bytes),
                        version.bytes,
                    )
                })
                .collect();
            // The loose copies of the mods concerned that the game does
            // not use.
            if let (Some(deployment), Some(staging)) = (&deployment, &staging) {
                for folder in mods {
                    let has_loose = read.iter().any(|(version, _)| {
                        version.archive.is_none()
                            && version
                                .provider
                                .as_ref()
                                .is_some_and(|found| &found.id == folder)
                    });
                    let file = staging.join(folder).join(&normal);
                    if has_loose || !is_inside(staging, &file) {
                        continue;
                    }
                    if let Some(bytes) = read_small(&file) {
                        let provider = Some(deployment.mod_ref(folder));
                        read.push((
                            describe(&normal, None, true, provider, &file, &bytes),
                            bytes,
                        ));
                    }
                }
            }
            let versions = (0..read.len())
                .filter_map(|index| {
                    let (version, bytes) = read.get(index)?;
                    let same_as = read
                        .iter()
                        .take(index)
                        .position(|(_, earlier)| earlier == bytes)
                        .and_then(|position| u32::try_from(position).ok());
                    Some(ComparedVersion {
                        same_as,
                        ..version.clone()
                    })
                })
                .collect();
            FileComparison {
                path: (*path).to_owned(),
                versions,
            }
        })
        .collect();
    Some(comparisons)
}

/// What can be told about one version of the file at `path`.
fn describe(
    path: &str,
    archive: Option<String>,
    staged: bool,
    provider: Option<ModRef>,
    on_disk: &Path,
    bytes: &[u8],
) -> ComparedVersion {
    let extension = path
        .rsplit_once('.')
        .map_or_else(String::new, |(_, extension)| extension.to_ascii_lowercase());
    let (text, text_cut) = if TEXT.contains(&extension.as_str()) {
        text_of(bytes).map_or((None, false), |(text, cut)| (Some(text), cut))
    } else {
        (None, false)
    };
    let image = match extension.as_str() {
        "png" => picture("image/png", bytes),
        "jpg" | "jpeg" => picture("image/jpeg", bytes),
        _ => None,
    };
    let script = (extension == "pex")
        .then(|| files::script(bytes))
        .flatten()
        .map(|script| ScriptSummary {
            name: script.name,
            has_debug_info: script.has_debug_info,
            functions: script
                .functions
                .into_iter()
                .map(|function| ScriptFunction {
                    fingerprint: fingerprint(&function.lines),
                    state: function.state,
                    name: function.name,
                    kind: match function.kind {
                        FunctionKind::Plain => ScriptFunctionKind::Plain,
                        FunctionKind::Getter => ScriptFunctionKind::Getter,
                        FunctionKind::Setter => ScriptFunctionKind::Setter,
                    },
                })
                .collect(),
        });
    ComparedVersion {
        archive,
        staged,
        provider,
        size: bytes.len() as u64,
        modified: modified(on_disk),
        same_as: None,
        text,
        text_cut,
        image,
        script,
    }
}

/// The text in `bytes`, and whether it was cut; `None` if it is not text.
fn text_of(bytes: &[u8]) -> Option<(String, bool)> {
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    let cut = bytes.len() > MAX_TEXT;
    let shown = bytes.get(..MAX_TEXT.min(bytes.len()))?;
    if shown.contains(&0) {
        return None;
    }
    let text = match std::str::from_utf8(shown) {
        Ok(text) => text.to_owned(),
        // Cut in the middle of a character: keep what is whole.
        Err(error) if cut && error.error_len().is_none() => {
            String::from_utf8_lossy(shown.get(..error.valid_up_to())?).into_owned()
        }
        // Older files are in a Windows code page; byte by byte keeps them
        // readable.
        Err(_) => shown.iter().map(|&byte| char::from(byte)).collect(),
    };
    Some((text, cut))
}

fn picture(kind: &str, bytes: &[u8]) -> Option<String> {
    (bytes.len() <= MAX_IMAGE).then(|| {
        format!(
            "data:{kind};base64,{}",
            mallet_core::encoding::base64(bytes)
        )
    })
}

/// Stands for a function's compiled code: how many instructions it has
/// and which source line each comes from, as FNV-1a.
fn fingerprint(lines: &[u16]) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in lines.iter().flat_map(|line| line.to_le_bytes()) {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{}:{hash:016x}", lines.len())
}

fn modified(path: &Path) -> Option<u64> {
    let time = std::fs::metadata(path).ok()?.modified().ok()?;
    Some(time.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs())
}

/// Whether `file` lies below `folder`, so that no path from a report
/// leads out of the mod manager's folder.
fn is_inside(folder: &Path, file: &Path) -> bool {
    file.strip_prefix(folder).is_ok_and(|rest| {
        rest.components()
            .all(|part| matches!(part, std::path::Component::Normal(_)))
    })
}

fn read_small(file: &Path) -> Option<Vec<u8>> {
    let small =
        std::fs::metadata(file).is_ok_and(|meta| meta.is_file() && meta.len() <= MAX_STAGED);
    small.then(|| std::fs::read(file).ok()).flatten()
}

/// Vortex's deployment, or `None` without Vortex.
fn deployment(options: &ScanOptions, game_dir: &Path) -> Option<VortexDeployment> {
    let manifest_path = options
        .deployment_manifest
        .clone()
        .unwrap_or_else(|| mallet_source_vortex::data_manifest_path(game_dir));
    let text = std::fs::read_to_string(manifest_path).ok()?;
    VortexDeployment::parse(&text, &game_dir.to_string_lossy()).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_is_read_and_cut_where_long() {
        assert_eq!(text_of(b"a = 1\n"), Some(("a = 1\n".to_owned(), false)));
        assert_eq!(
            text_of(b"\xEF\xBB\xBF{}"),
            Some(("{}".to_owned(), false)),
            "the byte order mark is left out"
        );
        assert_eq!(text_of(b"\x00binary"), None);
        assert_eq!(
            text_of(b"caf\xE9"),
            Some(("caf\u{e9}".to_owned(), false)),
            "a Windows code page is read byte by byte"
        );
        let long = vec![b'x'; MAX_TEXT + 10];
        let (text, cut) = text_of(&long).unwrap();
        assert!(cut);
        assert_eq!(text.len(), MAX_TEXT);
    }

    #[test]
    fn fingerprints_tell_different_code_apart() {
        assert_eq!(fingerprint(&[0, 1, 2]), fingerprint(&[0, 1, 2]));
        assert_ne!(fingerprint(&[0, 1, 2]), fingerprint(&[0, 2, 1]));
        assert_ne!(fingerprint(&[0, 1]), fingerprint(&[0, 1, 1]));
    }

    #[test]
    fn paths_out_of_the_folder_are_refused() {
        let folder = Path::new("staging");
        assert!(is_inside(folder, &folder.join("mod").join("scripts/a.pex")));
        assert!(!is_inside(folder, &folder.join("mod").join("../../secret")));
    }

    #[test]
    fn the_versions_of_a_loose_file_are_compared() {
        let game = tempfile::tempdir().unwrap();
        let scripts = game.path().join("Data/scripts");
        std::fs::create_dir_all(&scripts).unwrap();
        std::fs::write(scripts.join("a.psc"), b"Scriptname A\n").unwrap();
        let local = tempfile::tempdir().unwrap();
        std::fs::write(local.path().join("plugins.txt"), b"").unwrap();
        let options = ScanOptions {
            game_dir: Some(game.path().to_path_buf()),
            local_dir: Some(local.path().to_path_buf()),
            deployment_manifest: Some(game.path().join("none.json")),
            ..ScanOptions::default()
        };
        let compared = compare_files(&options, &["scripts/a.psc".to_owned()], &[]).unwrap();
        assert_eq!(compared.len(), 1);
        let version = &compared[0].versions[0];
        assert_eq!(version.text.as_deref(), Some("Scriptname A\n"));
        assert_eq!(version.size, 13);
        assert!(version.modified.is_some());
        assert_eq!(version.same_as, None);
    }
}
