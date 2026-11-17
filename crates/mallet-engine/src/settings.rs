//! The places Mod Mallet reads, as the user confirmed or chose them.
//!
//! Mod Mallet does not search a disk. It reads a small set of places, each
//! named by the user or found by a rule (ADR 0018). [`planned_locations`]
//! says which places a scan would read, before anything is read, so that
//! the desktop app can show them and ask. What the user answers is kept as
//! [`Settings`] in a small readable file. The settings are not part of the
//! cache, which may be deleted at will.

use std::collections::{BTreeMap, BTreeSet};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use mallet_core::{LocationOrigin, LocationRole, ScanLocation};
use mallet_game_bethesda::{ingest, locate};
use mallet_unpack::Unpacker;
use serde::{Deserialize, Serialize};

use crate::{EngineError, ScanOptions, installers, read_if_exists};

/// What the user decided about where Mod Mallet looks.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    /// Whether the user has seen the places a scan reads and agreed.
    pub confirmed: bool,
    /// The places the user named. A place that is not listed is found by
    /// its rule.
    pub places: BTreeMap<LocationRole, PathBuf>,
    /// The findings the user has set aside, by their identity. They stay
    /// in the report and are shown apart.
    pub ignored: BTreeSet<String>,
    /// Which side the user picked in a duel between two mods, by the
    /// duel's identity: 0 or 1.
    pub decisions: BTreeMap<String, u8>,
    /// The mods in the order the user likes them, the favourite first, by
    /// the mod manager's identity. Duels are decided by it.
    pub ranking: Vec<String>,
}

impl Settings {
    /// The options for a scan that looks where these settings say.
    pub fn scan_options(&self) -> ScanOptions {
        let place = |role: LocationRole| self.places.get(&role).cloned();
        ScanOptions {
            game_dir: place(LocationRole::GameDir),
            local_dir: place(LocationRole::PluginList).map(|path| plugin_list_dir(&path)),
            extender_log: place(LocationRole::ExtenderLog),
            deployment_manifest: place(LocationRole::DeploymentManifest),
            manager_state: place(LocationRole::ManagerState),
            manager_live_state: None,
            downloads_dir: place(LocationRole::Downloads),
            unpacker: place(LocationRole::Unpacker),
            masterlist: place(LocationRole::Masterlist),
        }
    }
}

/// The directory of the list of active plugins. The user is shown the
/// file, so that is what comes back; a directory is accepted as well.
fn plugin_list_dir(path: &Path) -> PathBuf {
    let names_the_list = path
        .file_name()
        .is_some_and(|name| name.to_string_lossy().eq_ignore_ascii_case("plugins.txt"));
    match path.parent() {
        Some(directory) if names_the_list => directory.to_path_buf(),
        _ => path.to_path_buf(),
    }
}

/// Where the settings are kept unless the caller chooses another place.
///
/// `None` if the system does not tell where applications keep settings.
pub fn default_settings_path() -> Option<PathBuf> {
    Some(dirs::config_dir()?.join("mod-mallet").join("settings.json"))
}

/// Reads the settings. A file that does not exist yet means that nothing
/// has been decided.
pub fn load_settings(path: &Path) -> Result<Settings, EngineError> {
    match read_if_exists(path)? {
        None => Ok(Settings::default()),
        Some(text) => serde_json::from_str(&text).map_err(|error| EngineError::Settings {
            path: path.to_path_buf(),
            reason: error.to_string(),
        }),
    }
}

/// Writes the settings, creating their directory if needed.
pub fn save_settings(path: &Path, settings: &Settings) -> Result<(), EngineError> {
    let failed = |source: std::io::Error| EngineError::WriteSettings {
        path: path.to_path_buf(),
        source,
    };
    if let Some(directory) = path.parent().filter(|dir| !dir.as_os_str().is_empty()) {
        std::fs::create_dir_all(directory).map_err(failed)?;
    }
    let text = serde_json::to_string_pretty(settings).map_err(|error| failed(error.into()))?;
    std::fs::write(path, text).map_err(failed)
}

/// The places a scan with `options` would read, how each was found and
/// whether something is there. Nothing is opened to find this out.
///
/// The directory of downloaded archives is listed only if the user named
/// it: otherwise it follows from the mod manager's state, which is not
/// read here.
pub fn planned_locations(options: &ScanOptions) -> Vec<ScanLocation> {
    let mut locations = Vec::new();

    let game_dir = match &options.game_dir {
        Some(given) => Some((given.clone(), LocationOrigin::Given)),
        None => locate::game_dir().map(|path| (path, LocationOrigin::Registry)),
    };
    locations.push(match &game_dir {
        Some((path, origin)) => planned(LocationRole::GameDir, path, *origin, path.is_dir()),
        None => missing(LocationRole::GameDir, LocationOrigin::Registry),
    });
    let game_dir = game_dir.map(|(path, _)| path);

    if let Some(list) = game_dir
        .as_deref()
        .and_then(|game_dir| ingest::plugin_list_path(game_dir, options.local_dir.as_deref()))
    {
        let origin = given_or_standard(options.local_dir.is_some());
        locations.push(planned(
            LocationRole::PluginList,
            &list,
            origin,
            list.is_file(),
        ));
    }

    if let Some(log) = options.extender_log.clone().or_else(locate::extender_log) {
        let origin = given_or_standard(options.extender_log.is_some());
        locations.push(planned(
            LocationRole::ExtenderLog,
            &log,
            origin,
            log.is_file(),
        ));
    }

    if let Some(manifest) = options.deployment_manifest.clone().or_else(|| {
        game_dir
            .as_deref()
            .map(mallet_source_vortex::data_manifest_path)
    }) {
        let origin = given_or_standard(options.deployment_manifest.is_some());
        locations.push(planned(
            LocationRole::DeploymentManifest,
            &manifest,
            origin,
            manifest.is_file(),
        ));
    }

    if let Some((state, origin)) = installers::planned_state(options) {
        locations.push(planned(
            LocationRole::ManagerState,
            &state,
            origin,
            state.is_file(),
        ));
    }

    if let Some(downloads) = &options.downloads_dir {
        locations.push(planned(
            LocationRole::Downloads,
            downloads,
            LocationOrigin::Given,
            downloads.is_dir(),
        ));
    }

    if let Some((masterlist, origin)) = crate::knowledge::planned_masterlist(options) {
        locations.push(planned(
            LocationRole::Masterlist,
            &masterlist,
            origin,
            masterlist.is_file(),
        ));
    }

    locations.push(match &options.unpacker {
        Some(given) => planned(
            LocationRole::Unpacker,
            given,
            LocationOrigin::Given,
            given.is_file(),
        ),
        None => match Unpacker::locate() {
            Some(unpacker) => planned(
                LocationRole::Unpacker,
                unpacker.program(),
                LocationOrigin::Standard,
                true,
            ),
            None => missing(LocationRole::Unpacker, LocationOrigin::Standard),
        },
    });

    locations
}

/// A short text that changes whenever one of the files a scan starts from
/// changes: the list of active plugins, the mod manager's records, the
/// script extender's log, the game's data directory. Comparing it costs a
/// few file lookups, so it can be asked for every few seconds to notice
/// that a new scan is due.
pub fn setup_fingerprint(options: &ScanOptions) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    let mut add = |path: &Path| {
        path.hash(&mut hasher);
        if let Ok(metadata) = std::fs::metadata(path) {
            metadata.len().hash(&mut hasher);
            if let Ok(modified) = metadata.modified() {
                modified.hash(&mut hasher);
            }
        }
    };
    for location in planned_locations(options) {
        if location.role == LocationRole::GameDir {
            // Installing or removing a plugin changes the data directory.
            add(&Path::new(&location.path).join("Data"));
        } else {
            add(Path::new(&location.path));
        }
    }
    format!("{:016x}", hasher.finish())
}

fn given_or_standard(given: bool) -> LocationOrigin {
    if given {
        LocationOrigin::Given
    } else {
        LocationOrigin::Standard
    }
}

fn planned(role: LocationRole, path: &Path, origin: LocationOrigin, found: bool) -> ScanLocation {
    ScanLocation {
        role,
        path: path.display().to_string(),
        origin,
        found,
    }
}

/// A place whose rule yields nothing on this machine: the user has to name
/// it.
fn missing(role: LocationRole, origin: LocationOrigin) -> ScanLocation {
    ScanLocation {
        role,
        path: String::new(),
        origin,
        found: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn role_of(locations: &[ScanLocation], role: LocationRole) -> &ScanLocation {
        locations
            .iter()
            .find(|location| location.role == role)
            .unwrap()
    }

    #[test]
    fn nothing_is_decided_before_the_settings_exist() {
        let directory = tempfile::tempdir().unwrap();
        let settings = load_settings(&directory.path().join("settings.json")).unwrap();
        assert_eq!(settings, Settings::default());
        assert!(!settings.confirmed);
    }

    #[test]
    fn settings_come_back_as_they_were_saved() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("nested").join("settings.json");
        let settings = Settings {
            confirmed: true,
            places: BTreeMap::from([
                (LocationRole::GameDir, PathBuf::from("games/skyrim")),
                (LocationRole::Unpacker, PathBuf::from("tools/7z.exe")),
            ]),
            ignored: BTreeSet::from(["objects.overlapping:pair:a+b".to_owned()]),
            decisions: BTreeMap::from([("duel:a|b".to_owned(), 1)]),
            ranking: vec!["faces".to_owned(), "patch".to_owned()],
        };

        save_settings(&path, &settings).unwrap();

        assert_eq!(load_settings(&path).unwrap(), settings);
        // The file is meant to be read by people.
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("\"game-dir\""), "{text}");
        assert!(text.contains('\n'));
    }

    #[test]
    fn the_fingerprint_changes_when_a_file_the_scan_reads_changes() {
        let directory = tempfile::tempdir().unwrap();
        let log = directory.path().join("skse64.log");
        std::fs::write(&log, "first").unwrap();
        let options = ScanOptions {
            game_dir: Some(directory.path().join("game")),
            extender_log: Some(log.clone()),
            ..ScanOptions::default()
        };
        let before = setup_fingerprint(&options);
        assert_eq!(setup_fingerprint(&options), before);

        std::fs::write(&log, "second, longer").unwrap();
        assert_ne!(setup_fingerprint(&options), before);
    }

    #[test]
    fn a_damaged_settings_file_is_an_error_and_not_a_fresh_start() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.json");
        std::fs::write(&path, "{ confirmed").unwrap();
        assert!(matches!(
            load_settings(&path),
            Err(EngineError::Settings { .. })
        ));
    }

    #[test]
    fn named_places_become_the_options_of_a_scan() {
        let settings = Settings {
            confirmed: true,
            ignored: BTreeSet::new(),
            decisions: BTreeMap::new(),
            ranking: Vec::new(),
            places: BTreeMap::from([
                (LocationRole::GameDir, PathBuf::from("games/skyrim")),
                (
                    LocationRole::PluginList,
                    Path::new("local").join("Plugins.txt"),
                ),
                (LocationRole::Downloads, PathBuf::from("downloads")),
            ]),
        };
        let options = settings.scan_options();
        assert_eq!(options.game_dir, Some(PathBuf::from("games/skyrim")));
        // The list is shown as a file; the scan wants its directory.
        assert_eq!(options.local_dir, Some(PathBuf::from("local")));
        assert_eq!(options.downloads_dir, Some(PathBuf::from("downloads")));
        assert_eq!(options.extender_log, None);
        assert_eq!(options.manager_state, None);
    }

    #[test]
    fn a_plugin_list_given_as_a_directory_is_taken_as_it_is() {
        assert_eq!(plugin_list_dir(Path::new("local")), Path::new("local"));
        assert_eq!(
            plugin_list_dir(&Path::new("local").join("plugins.TXT")),
            Path::new("local")
        );
    }

    #[test]
    fn named_places_are_planned_as_given_and_checked_for_being_there() {
        let directory = tempfile::tempdir().unwrap();
        let game_dir = directory.path().join("game");
        std::fs::create_dir_all(game_dir.join("Data")).unwrap();
        let log = directory.path().join("skse64.log");
        std::fs::write(&log, "log").unwrap();
        let options = ScanOptions {
            game_dir: Some(game_dir.clone()),
            local_dir: Some(directory.path().join("local")),
            extender_log: Some(log.clone()),
            manager_state: Some(directory.path().join("no-state.json")),
            downloads_dir: Some(directory.path().join("no-downloads")),
            unpacker: Some(directory.path().join("no-7z.exe")),
            ..ScanOptions::default()
        };

        let locations = planned_locations(&options);

        let game = role_of(&locations, LocationRole::GameDir);
        assert_eq!(game.path, game_dir.display().to_string());
        assert_eq!(game.origin, LocationOrigin::Given);
        assert!(game.found);
        assert!(role_of(&locations, LocationRole::ExtenderLog).found);
        let list = role_of(&locations, LocationRole::PluginList);
        assert_eq!(list.origin, LocationOrigin::Given);
        assert!(!list.found);
        // The manifest follows from the game directory.
        let manifest = role_of(&locations, LocationRole::DeploymentManifest);
        assert_eq!(manifest.origin, LocationOrigin::Standard);
        assert!(manifest.path.starts_with(&game.path));
        for role in [
            LocationRole::ManagerState,
            LocationRole::Downloads,
            LocationRole::Unpacker,
        ] {
            let location = role_of(&locations, role);
            assert_eq!(location.origin, LocationOrigin::Given);
            assert!(!location.found, "{role:?}");
        }
    }
}
