//! The files of every mod in the mod manager's staging folder (ADR 0049).
//!
//! Vortex keeps each installed mod in a folder of its own and deploys only
//! the files that win. The listing of a mod's folder says which files the
//! mod brings, including those Vortex kept back because another mod's file
//! wins. Only the folder listings are read, never a file, and each mod's
//! listing is kept in the cache until the modification time of its folder
//! changes, which Vortex causes by installing the mod anew. The folders of
//! mods that are installed but switched off are listed too, so that files
//! the game misses can be found in them (ADR 0056).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Instant, UNIX_EPOCH};

use mallet_core::{LocationOrigin, LocationRole, ScanLocation};
use mallet_source_vortex::VortexDeployment;
use mallet_store::Store;

use crate::ScanOptions;

/// The cache namespace of the mods' folder listings.
const CACHE_NAMESPACE: &str = "staged-files-1";

/// The staging folder as the places page shows it before anything is read.
/// It is named in the deployment manifest, which is not opened before the
/// user agrees: unless the user named it, it is listed with where it will
/// be read from and an empty path, as found if the manifest is there.
pub(crate) fn planned(options: &ScanOptions, manifest_found: bool) -> ScanLocation {
    match &options.staging_dir {
        Some(given) => ScanLocation {
            role: LocationRole::ModStaging,
            path: given.display().to_string(),
            origin: LocationOrigin::Given,
            found: given.is_dir(),
        },
        None => ScanLocation {
            role: LocationRole::ModStaging,
            path: String::new(),
            origin: LocationOrigin::Manager,
            found: manifest_found,
        },
    }
}

/// Adds the files of the deployed mods' folders to `deployment`, and the
/// staging folder to `locations`. A staging folder that is neither named
/// nor found leaves the deployment as it is.
pub(crate) fn read(
    options: &ScanOptions,
    deployment: VortexDeployment,
    store: Option<&Store>,
    locations: &mut Vec<ScanLocation>,
) -> VortexDeployment {
    let Some((staging, origin)) = staging_dir(options, &deployment) else {
        return deployment;
    };
    let found = staging.is_dir();
    locations.push(ScanLocation {
        role: LocationRole::ModStaging,
        path: staging.display().to_string(),
        origin,
        found,
    });
    if !found {
        return deployment;
    }
    let staged = list_folders(&staging, deployment.mod_folders(), store);
    deployment.with_staged_files(staged)
}

/// Adds the files of the folders of mods that are installed but switched
/// off, `folders`, so that what the game misses can be found in them. Reads
/// from the same staging folder as [`read`], which has already listed it as
/// a place.
pub(crate) fn read_switched_off(
    options: &ScanOptions,
    deployment: VortexDeployment,
    folders: &[String],
    store: Option<&Store>,
) -> VortexDeployment {
    if folders.is_empty() {
        return deployment;
    }
    let Some((staging, _)) = staging_dir(options, &deployment) else {
        return deployment;
    };
    if !staging.is_dir() {
        return deployment;
    }
    let listed = list_folders(&staging, folders, store);
    deployment.with_switched_off_files(listed)
}

/// The staging folder: the one the user named, else the one the
/// deployment manifest names.
fn staging_dir(
    options: &ScanOptions,
    deployment: &VortexDeployment,
) -> Option<(PathBuf, LocationOrigin)> {
    match &options.staging_dir {
        Some(given) => Some((given.clone(), LocationOrigin::Given)),
        None => deployment
            .staging_path()
            .map(|named| (PathBuf::from(named), LocationOrigin::Manager)),
    }
}

/// The files below each of `folders` in `staging`, from the cache where
/// the folder has not changed. A folder that is missing or cannot be
/// listed is left out.
fn list_folders(
    staging: &Path,
    folders: &[String],
    store: Option<&Store>,
) -> HashMap<String, Vec<String>> {
    let started = Instant::now();
    let mut from_cache = 0usize;
    let mut staged: HashMap<String, Vec<String>> = HashMap::new();
    let mut list = || {
        for folder in folders {
            let path = staging.join(folder);
            let Some(fingerprint) = fingerprint(&path) else {
                continue;
            };
            let cached = store.and_then(|store| {
                store
                    .cached_text(CACHE_NAMESPACE, folder, &fingerprint)
                    .ok()
                    .flatten()
            });
            let files = if let Some(text) = cached {
                from_cache += 1;
                text.lines().map(str::to_owned).collect()
            } else {
                let Ok(files) = mallet_source_vortex::list_mod_folder(&path) else {
                    continue;
                };
                if let Some(store) = store
                    && let Err(error) =
                        store.cache_text(CACHE_NAMESPACE, folder, &fingerprint, &files.join("\n"))
                {
                    tracing::warn!(%error, "mod folder listing could not be cached");
                }
                files
            };
            staged.insert(folder.clone(), files);
        }
    };
    match store {
        Some(store) => {
            if store.batched(&mut list).is_err() {
                list();
            }
        }
        None => list(),
    }
    tracing::info!(
        mods = staged.len(),
        files = staged.values().map(Vec::len).sum::<usize>(),
        from_cache,
        millis = started.elapsed().as_millis(),
        "listed the mods' folders"
    );
    staged
}

/// The modification time of a mod's folder, or `None` if it is not a
/// folder.
fn fingerprint(folder: &Path) -> Option<String> {
    let metadata = std::fs::metadata(folder)
        .ok()
        .filter(std::fs::Metadata::is_dir)?;
    let modified = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |since| since.as_nanos());
    Some(modified.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str = r#"{
        "stagingPath": "STAGING",
        "targetPath": "GAME\\Data",
        "files": [
            { "relPath": "textures\\sky.dds", "source": "Sky 1", "target": "", "time": 1 }
        ]
    }"#;

    fn deployment(staging: &Path, game: &Path) -> VortexDeployment {
        let text = MANIFEST
            .replace(
                "STAGING",
                &staging.display().to_string().replace('\\', "\\\\"),
            )
            .replace("GAME", &game.display().to_string().replace('\\', "\\\\"));
        VortexDeployment::parse(&text, &game.display().to_string()).unwrap()
    }

    #[test]
    fn every_file_of_a_mods_folder_is_known_and_kept_in_the_cache() {
        let directory = tempfile::tempdir().unwrap();
        let staging = directory.path().join("staging");
        let game = directory.path().join("game");
        let textures = staging.join("Sky 1").join("textures");
        std::fs::create_dir_all(&textures).unwrap();
        std::fs::write(textures.join("sky.dds"), b"").unwrap();
        std::fs::write(textures.join("moon.dds"), b"").unwrap();
        let store = Store::open_in_memory().unwrap();

        for _ in 0..2 {
            let mut locations = Vec::new();
            let deployment = read(
                &ScanOptions::default(),
                deployment(&staging, &game),
                Some(&store),
                &mut locations,
            );
            let sky = mallet_core::ModSource::provider_of(
                &deployment,
                &mallet_core::GamePath::new("data/textures/sky.dds"),
            )
            .unwrap();
            let files = mallet_core::ModSource::staged_files(&deployment, &sky).unwrap();
            assert_eq!(
                files,
                [
                    mallet_core::GamePath::new("data/textures/moon.dds"),
                    mallet_core::GamePath::new("data/textures/sky.dds"),
                ]
            );
            assert_eq!(locations.len(), 1);
            assert_eq!(locations[0].role, LocationRole::ModStaging);
            assert_eq!(locations[0].origin, LocationOrigin::Manager);
            assert!(locations[0].found);
        }
        // The second time, the listing came from the cache.
        let fingerprint = fingerprint(&staging.join("Sky 1")).unwrap();
        assert!(
            store
                .cached_text(CACHE_NAMESPACE, "Sky 1", &fingerprint)
                .unwrap()
                .is_some()
        );
    }

    #[test]
    fn without_a_staging_folder_the_deployment_stays_as_it_is() {
        let directory = tempfile::tempdir().unwrap();
        let game = directory.path().join("game");
        let mut locations = Vec::new();
        let deployment = read(
            &ScanOptions::default(),
            deployment(&directory.path().join("missing"), &game),
            None,
            &mut locations,
        );
        assert!(!locations[0].found);
        let sky = mallet_core::ModSource::provider_of(
            &deployment,
            &mallet_core::GamePath::new("data/textures/sky.dds"),
        )
        .unwrap();
        assert_eq!(
            mallet_core::ModSource::staged_files(&deployment, &sky),
            None
        );
    }

    #[test]
    fn files_of_switched_off_mods_are_listed_from_the_same_folder() {
        let directory = tempfile::tempdir().unwrap();
        let staging = directory.path().join("staging");
        let game = directory.path().join("game");
        let textures = staging.join("Town 2").join("textures");
        std::fs::create_dir_all(&textures).unwrap();
        std::fs::write(textures.join("wall.dds"), b"").unwrap();

        let deployment = read_switched_off(
            &ScanOptions::default(),
            deployment(&staging, &game),
            &["Town 2".to_owned(), "Gone 3".to_owned()],
            None,
        );
        let found = mallet_core::ModSource::switched_off_with(
            &deployment,
            &mallet_core::GamePath::new("data/textures/wall.dds"),
        );
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, "Town 2");
    }
}
