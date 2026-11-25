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
use std::sync::{Mutex, PoisonError};

use mallet_core::{
    GameChoice, KnownGame, LocationOrigin, LocationRole, ManagerChoice, ScanLocation,
};
use mallet_game_bethesda::{Game, ingest, locate};
use mallet_unpack::Unpacker;
use serde::{Deserialize, Serialize};

use crate::managers::{self, ModManager};
use crate::{EngineError, ScanOptions, installers, read_if_exists};

/// What the user decided about one game: where Mod Mallet looks, and what
/// they made of the findings.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    /// The game these settings are for, by its identifier (`skyrimse`).
    /// It is the key under which they are kept, not part of them.
    #[serde(skip)]
    pub game: String,
    /// Whether the user has seen the places a scan reads and agreed.
    pub confirmed: bool,
    /// The places the user named. A place that is not listed is found by
    /// its rule.
    pub places: BTreeMap<LocationRole, PathBuf>,
    /// The findings the user has set aside, by their identity. They stay
    /// in the report and are shown apart.
    pub ignored: BTreeSet<String>,
    /// Which side the user picked in a duel between two mods, by the
    /// duel's identity: 0 or 1. Kept here before the picks became
    /// decisions of the patches (ADR 0081); the window moves them there.
    pub decisions: BTreeMap<String, u8>,
    /// The mods in the order the user likes them, the favourite first, by
    /// the mod manager's identity. Duels are decided by it.
    pub ranking: Vec<String>,
    /// What the patch that hides objects is to do, as the user chose it:
    /// by finding, by spot or by object, the plugin whose objects are
    /// hidden, or `None` to keep both. The keys do not depend on the load
    /// order; the desktop app makes them. `None` as a whole while the user
    /// has chosen nothing, so that the app starts from the patch in the
    /// game.
    ///
    /// Since patches are several (ADR 0070), this is only read, to make the
    /// first patch of the [`Settings::patch_book`] from it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub patch_choices: Option<BTreeMap<String, Option<String>>>,
    /// The user's patches, each with its decisions, and the one new
    /// decisions go to (ADR 0070). `None` until the window made it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub patch_book: Option<mallet_core::PatchBook>,
    /// What the user said about preferences between their mods: the ones
    /// they set, their answers to those offered, and whether learning is
    /// on (ADR 0086). `None` until they said anything.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preferences: Option<mallet_core::PatchPreferences>,
    /// Which version of a thing of the catalogue the user had the mod
    /// manager put into the game, by the thing's identity: the version's
    /// identity (`mod:<folder>`, `game`, `archive:<name>`) (ADR 0054).
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub asset_choices: BTreeMap<String, String>,
    /// The mod manager the user chose for this game, by its identifier
    /// (`vortex`, `mo2`). `None` while the user has chosen none; then the
    /// one found on the machine is suggested (ADR 0064).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manager: Option<String>,
}

impl Settings {
    /// Where the mod manager is, for actions on the mods of this game.
    pub fn manager_options(&self) -> crate::ManagerOptions {
        crate::ManagerOptions {
            game: (!self.game.is_empty()).then(|| self.game.clone()),
            setup: Some(self.scan_options()),
            by_hand: self.manager() == mallet_core::managers::MOD_ORGANIZER,
            ..crate::ManagerOptions::default()
        }
    }

    /// The identifier of the mod manager in use: the one the user chose,
    /// or else the one suggested for this machine. Only looks whether a
    /// few folders exist.
    pub fn manager(&self) -> String {
        match &self.manager {
            Some(chosen) if ModManager::from_id(chosen, None).is_some() => chosen.clone(),
            _ => managers::suggested(self.game_dir().as_deref()).to_owned(),
        }
    }

    /// The mod managers Mod Mallet can read, which of them are on this
    /// machine, and which one is in use. Opens no file.
    pub fn managers(&self) -> ManagerChoice {
        managers::known(
            self.manager.as_deref(),
            self.game_dir().as_deref(),
            self.places
                .get(&LocationRole::ManagerInstance)
                .map(PathBuf::as_path),
        )
    }

    /// These settings with the mod manager `manager` (`vortex`, `mo2`)
    /// chosen and its instance at `instance`, or found by its rule if
    /// `None`: for showing the places that choice would read before it is
    /// kept. An unknown manager leaves the choice as it was.
    #[must_use]
    pub fn with_manager(&self, manager: &str, instance: Option<PathBuf>) -> Self {
        let mut settings = self.clone();
        if ModManager::from_id(manager, None).is_some() {
            settings.manager = Some(manager.to_owned());
        }
        match instance {
            Some(instance) => {
                settings
                    .places
                    .insert(LocationRole::ManagerInstance, instance);
            }
            None => {
                settings.places.remove(&LocationRole::ManagerInstance);
            }
        }
        settings
    }

    /// The game's directory: where the user said it is, or where it is
    /// found.
    fn game_dir(&self) -> Option<PathBuf> {
        if let Some(dir) = self.places.get(&LocationRole::GameDir) {
            return Some(dir.clone());
        }
        locate::game_dir(Game::from_id(&self.game)?).map(|(dir, _)| dir)
    }

    /// The options for a scan that looks where these settings say.
    pub fn scan_options(&self) -> ScanOptions {
        let place = |role: LocationRole| self.places.get(&role).cloned();
        let manager = ModManager::from_id(&self.manager(), place(LocationRole::ManagerInstance))
            .unwrap_or_default();
        ScanOptions {
            manager,
            game: (!self.game.is_empty()).then(|| self.game.clone()),
            game_dir: place(LocationRole::GameDir),
            local_dir: place(LocationRole::PluginList).map(|path| plugin_list_dir(&path)),
            extender_log: place(LocationRole::ExtenderLog),
            deployment_manifest: place(LocationRole::DeploymentManifest),
            manager_state: place(LocationRole::ManagerState),
            manager_live_state: None,
            downloads_dir: place(LocationRole::Downloads),
            staging_dir: place(LocationRole::ModStaging),
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

/// The settings file: the settings of each game, and which game the user
/// looked at last.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct SettingsFile {
    /// The game looked at last, by its identifier.
    game: Option<String>,
    /// The settings of each game, by the game's identifier.
    games: BTreeMap<String, Settings>,
}

impl SettingsFile {
    /// Reads the file at `path`. A file that does not exist yet means that
    /// nothing has been decided.
    ///
    /// Before there were settings per game, the file held those of Skyrim
    /// Special Edition at its top level. Such a file is read as theirs.
    fn load(path: &Path) -> Result<Self, EngineError> {
        let Some(text) = read_if_exists(path)? else {
            return Ok(Self::default());
        };
        let invalid = |error: serde_json::Error| EngineError::Settings {
            path: path.to_path_buf(),
            reason: error.to_string(),
        };
        let value: serde_json::Value = serde_json::from_str(&text).map_err(invalid)?;
        let per_game = value
            .as_object()
            .is_some_and(|object| object.contains_key("games") || object.contains_key("game"));
        if per_game {
            serde_json::from_value(value).map_err(invalid)
        } else {
            let skyrim: Settings = serde_json::from_value(value).map_err(invalid)?;
            let id = Game::SkyrimSe.id().to_owned();
            Ok(Self {
                game: Some(id.clone()),
                games: BTreeMap::from([(id, skyrim)]),
            })
        }
    }

    /// Writes the file, creating its directory if needed.
    fn save(&self, path: &Path) -> Result<(), EngineError> {
        let failed = |source: std::io::Error| EngineError::WriteSettings {
            path: path.to_path_buf(),
            source,
        };
        if let Some(directory) = path.parent().filter(|dir| !dir.as_os_str().is_empty()) {
            std::fs::create_dir_all(directory).map_err(failed)?;
        }
        let text = serde_json::to_string_pretty(self).map_err(|error| failed(error.into()))?;
        std::fs::write(path, text).map_err(failed)
    }

    /// The game to look at: the one looked at last, or else the first
    /// game that is installed, or else Skyrim Special Edition.
    fn active_game(&self) -> Game {
        self.game
            .as_deref()
            .and_then(Game::from_id)
            .or_else(|| {
                locate::installed_games()
                    .into_iter()
                    .map(|(game, _, _)| game)
                    .next()
            })
            .unwrap_or_default()
    }

    /// The settings of `game`.
    fn of(&self, game: Game) -> Settings {
        Settings {
            game: game.id().to_owned(),
            ..self.games.get(game.id()).cloned().unwrap_or_default()
        }
    }
}

/// Reads the settings of the game the user looked at last. A file that
/// does not exist yet means that nothing has been decided.
pub fn load_settings(path: &Path) -> Result<Settings, EngineError> {
    let file = SettingsFile::load(path)?;
    Ok(file.of(file.active_game()))
}

/// Writes the settings of the game they are for, creating their directory
/// if needed, and remembers that game as the one looked at last. The
/// settings of the other games stay as they were.
pub fn save_settings(path: &Path, settings: &Settings) -> Result<(), EngineError> {
    let _one_at_a_time = CHANGING.lock().unwrap_or_else(PoisonError::into_inner);
    write_settings(path, settings)
}

/// The settings file is changed one change at a time, so that two changes
/// made at the same moment do not undo each other.
static CHANGING: Mutex<()> = Mutex::new(());

/// Changes the settings of the game the user looked at last with `change`
/// and returns them as written. Reading and writing are one step: of two
/// changes made at the same moment, from different parts of the app, both
/// arrive.
///
/// # Errors
///
/// If the settings cannot be read or written.
pub fn update_settings(
    path: &Path,
    change: impl FnOnce(&mut Settings),
) -> Result<Settings, EngineError> {
    let _one_at_a_time = CHANGING.lock().unwrap_or_else(PoisonError::into_inner);
    let mut settings = load_settings(path)?;
    change(&mut settings);
    write_settings(path, &settings)?;
    Ok(settings)
}

/// [`save_settings`], for a caller that already changes the file alone.
fn write_settings(path: &Path, settings: &Settings) -> Result<(), EngineError> {
    let game = Game::from_id(&settings.game).unwrap_or_default();
    let mut file = SettingsFile::load(path)?;
    let mut kept = settings.clone();
    kept.game = String::new();
    file.games.insert(game.id().to_owned(), kept);
    file.game = Some(game.id().to_owned());
    file.save(path)
}

/// Makes the game with the identifier `game` the one to look at, and
/// returns its settings. Only ever called because the user chose the game.
///
/// # Errors
///
/// If Mod Mallet does not know the game, or the settings cannot be read
/// or written.
pub fn select_game(path: &Path, game: &str) -> Result<Settings, EngineError> {
    let game = Game::from_id(game).ok_or_else(|| EngineError::UnknownGame(game.to_owned()))?;
    let _one_at_a_time = CHANGING.lock().unwrap_or_else(PoisonError::into_inner);
    let mut file = SettingsFile::load(path)?;
    file.game = Some(game.id().to_owned());
    file.save(path)?;
    tracing::info!(game = game.id(), "game chosen");
    Ok(file.of(game))
}

/// The games Mod Mallet knows, with where each is on this machine (where
/// the user said it is, or else where it was found), and which of them the
/// user looks at. Only looks up a few registry entries and folders.
pub fn known_games(path: &Path) -> Result<GameChoice, EngineError> {
    let file = SettingsFile::load(path)?;
    let games = Game::ALL
        .into_iter()
        .map(|game| {
            let named = file
                .of(game)
                .places
                .get(&LocationRole::GameDir)
                .map(|dir| (dir.clone(), LocationOrigin::Given));
            let found = named.or_else(|| locate::game_dir(game));
            KnownGame {
                id: game.id().to_owned(),
                name: game.name().to_owned(),
                support: game.support(),
                dir: found.as_ref().map(|(dir, _)| dir.display().to_string()),
                origin: found.map(|(_, origin)| origin),
                script_extender: game.script_extender().short_name.to_owned(),
            }
        })
        .collect();
    Ok(GameChoice {
        active: file.active_game().id().to_owned(),
        games,
    })
}

/// The places a scan with `options` would read, how each was found and
/// whether something is there. Nothing is opened to find this out.
///
/// The directory of downloaded archives is listed only if the user named
/// it: otherwise it follows from the mod manager's state, which is not
/// read here.
pub fn planned_locations(options: &ScanOptions) -> Vec<ScanLocation> {
    let mut locations = Vec::new();

    let game = options.game().unwrap_or_default();
    let game_dir = options.located_game_dir();
    locations.push(match &game_dir {
        Some((path, origin)) => planned(LocationRole::GameDir, path, *origin, path.is_dir()),
        None => missing(LocationRole::GameDir, LocationOrigin::Registry),
    });
    let game_dir = game_dir.map(|(path, _)| path);

    // Mod Organizer keeps the list of plugins in its profile, named in its
    // settings, which are not opened before the user agrees (ADR 0064).
    let mo2_instance = match &options.manager {
        ModManager::ModOrganizer { instance, .. } => Some(instance.as_deref()),
        ModManager::Vortex => None,
    };
    if let Some(instance) = mo2_instance {
        locations.push(managers::planned_instance(instance));
    }
    if let Some(list) = game_dir
        .as_deref()
        .filter(|_| mo2_instance.is_none() || options.local_dir.is_some())
        .and_then(|game_dir| ingest::plugin_list_path(game, game_dir, options.local_dir.as_deref()))
    {
        let origin = given_or_standard(options.local_dir.is_some());
        locations.push(planned(
            LocationRole::PluginList,
            &list,
            origin,
            list.is_file(),
        ));
    }

    if let Some(log) = options
        .extender_log
        .clone()
        .or_else(|| locate::extender_log(game, game_dir.as_deref()))
    {
        let origin = given_or_standard(options.extender_log.is_some());
        locations.push(planned(
            LocationRole::ExtenderLog,
            &log,
            origin,
            log.is_file(),
        ));
    }

    if mo2_instance.is_none() {
        planned_vortex(options, game_dir.as_deref(), &mut locations);
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

/// The places of Vortex a scan with `options` would read: its list of
/// deployed files, its folder of installed mods, its saved state and the
/// folder of downloads if the user named it.
fn planned_vortex(
    options: &ScanOptions,
    game_dir: Option<&Path>,
    locations: &mut Vec<ScanLocation>,
) {
    if let Some(manifest) = options
        .deployment_manifest
        .clone()
        .or_else(|| game_dir.map(mallet_source_vortex::data_manifest_path))
    {
        let origin = given_or_standard(options.deployment_manifest.is_some());
        locations.push(planned(
            LocationRole::DeploymentManifest,
            &manifest,
            origin,
            manifest.is_file(),
        ));
        locations.push(crate::staging::planned(options, manifest.is_file()));
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
            let root = Path::new(&location.path);
            add(&root.join("Data"));
            // And an Unreal mod its folder (ADR 0065).
            if let Ok(game) = options.game() {
                let install = locate::install_dir(game, root);
                for (folder, _) in game.unreal_pak_folders() {
                    add(&install.join(folder));
                }
            }
        } else {
            add(Path::new(&location.path));
        }
    }
    // Mod Organizer's profile changes when a mod is switched or moved.
    for file in managers::watched_files(options) {
        add(&file);
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
        // The game is the first one installed on the machine running the
        // test, or Skyrim Special Edition.
        assert!(Game::from_id(&settings.game).is_some());
        assert_eq!(
            settings,
            Settings {
                game: settings.game.clone(),
                ..Settings::default()
            }
        );
        assert!(!settings.confirmed);
    }

    #[test]
    fn settings_written_before_there_were_games_are_skyrim_special_editions() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.json");
        std::fs::write(
            &path,
            r#"{ "confirmed": true, "places": { "game-dir": "games/skyrim" }, "ranking": ["faces"] }"#,
        )
        .unwrap();

        let settings = load_settings(&path).unwrap();
        assert_eq!(settings.game, "skyrimse");
        assert!(settings.confirmed);
        assert_eq!(settings.ranking, ["faces"]);

        // Saving writes the new form and keeps what was there.
        save_settings(&path, &settings).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("\"games\""), "{text}");
        assert_eq!(load_settings(&path).unwrap(), settings);
    }

    #[test]
    fn each_game_keeps_settings_of_its_own() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.json");
        let skyrim = Settings {
            game: "skyrimse".to_owned(),
            confirmed: true,
            ignored: BTreeSet::from(["a finding".to_owned()]),
            ..Settings::default()
        };
        save_settings(&path, &skyrim).unwrap();

        let fallout = select_game(&path, "fallout4").unwrap();
        assert_eq!(fallout.game, "fallout4");
        assert!(!fallout.confirmed);
        assert!(fallout.ignored.is_empty());
        assert_eq!(load_settings(&path).unwrap(), fallout);

        let fallout = Settings {
            confirmed: true,
            ..fallout
        };
        save_settings(&path, &fallout).unwrap();
        assert_eq!(select_game(&path, "skyrimse").unwrap(), skyrim);
        assert_eq!(select_game(&path, "fallout4").unwrap(), fallout);
        assert_eq!(fallout.scan_options().game.as_deref(), Some("fallout4"));
        assert_eq!(fallout.manager_options().game.as_deref(), Some("fallout4"));
    }

    #[test]
    fn an_unknown_game_cannot_be_chosen() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.json");
        assert!(matches!(
            select_game(&path, "morrowind"),
            Err(EngineError::UnknownGame(_))
        ));
        assert!(!path.exists());
    }

    #[test]
    fn every_game_is_known_with_where_the_user_put_it() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.json");
        let settings = Settings {
            game: "falloutnv".to_owned(),
            places: BTreeMap::from([(LocationRole::GameDir, PathBuf::from("games/nv"))]),
            ..Settings::default()
        };
        save_settings(&path, &settings).unwrap();

        let choice = known_games(&path).unwrap();
        assert_eq!(choice.active, "falloutnv");
        let games = choice.games;
        assert_eq!(games.len(), Game::ALL.len());
        let nv = games.iter().find(|game| game.id == "falloutnv").unwrap();
        assert_eq!(
            nv.dir.as_deref(),
            Some(PathBuf::from("games/nv").display().to_string().as_str())
        );
        assert_eq!(nv.origin, Some(LocationOrigin::Given));
        assert_eq!(nv.script_extender, "xNVSE");
    }

    #[test]
    fn settings_come_back_as_they_were_saved() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("nested").join("settings.json");
        let settings = Settings {
            game: "skyrimse".to_owned(),
            confirmed: true,
            places: BTreeMap::from([
                (LocationRole::GameDir, PathBuf::from("games/skyrim")),
                (LocationRole::Unpacker, PathBuf::from("tools/7z.exe")),
            ]),
            ignored: BTreeSet::from(["objects.overlapping:pair:a+b".to_owned()]),
            decisions: BTreeMap::from([("duel:a|b".to_owned(), 1)]),
            ranking: vec!["faces".to_owned(), "patch".to_owned()],
            patch_choices: Some(BTreeMap::from([
                (
                    "objects.overlapping:pair:a+b".to_owned(),
                    Some("b.esp".to_owned()),
                ),
                ("spot:objects.overlapping:pair:a+b|a.esp:1".to_owned(), None),
            ])),
            asset_choices: BTreeMap::from([(
                "skyrim.esm/00013989".to_owned(),
                "mod:Better Blades".to_owned(),
            )]),
            patch_book: Some(mallet_core::PatchBook {
                patches: vec![mallet_core::BookPatch {
                    id: "own".to_owned(),
                    name: "Two Towns".to_owned(),
                    decisions: BTreeMap::from([(
                        "object:a.esp|1".to_owned(),
                        Some("a.esp".to_owned()),
                    )]),
                    unticked: vec!["object:a.esp|1".to_owned()],
                    ..mallet_core::BookPatch::default()
                }],
                active: "own".to_owned(),
                pins: vec!["pin:spot:x".to_owned()],
            }),
            preferences: Some(mallet_core::PatchPreferences {
                rules: vec![mallet_core::PreferenceRule {
                    mod_name: "Better Blades".to_owned(),
                    over: None,
                    topic: Some("meshes".to_owned()),
                }],
                ..mallet_core::PatchPreferences::default()
            }),
            manager: Some("mo2".to_owned()),
        };

        save_settings(&path, &settings).unwrap();

        assert_eq!(load_settings(&path).unwrap(), settings);
        // The file is meant to be read by people.
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("\"game-dir\""), "{text}");
        assert!(text.contains('\n'));
    }

    #[test]
    fn choices_for_the_patch_are_kept_and_told_from_none_made() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.json");
        assert_eq!(load_settings(&path).unwrap().patch_choices, None);

        // Choosing, then taking every choice back, is not the same as never
        // having chosen.
        update_settings(&path, |settings| {
            settings.patch_choices = Some(BTreeMap::new());
        })
        .unwrap();
        assert_eq!(
            load_settings(&path).unwrap().patch_choices,
            Some(BTreeMap::new())
        );
    }

    #[test]
    fn changes_made_at_the_same_moment_all_arrive() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.json");
        std::thread::scope(|scope| {
            for number in 0..8 {
                let path = &path;
                scope.spawn(move || {
                    update_settings(path, |settings| {
                        settings.ignored.insert(format!("finding {number}"));
                    })
                    .unwrap();
                });
            }
        });
        assert_eq!(load_settings(&path).unwrap().ignored.len(), 8);
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
            game: "skyrimse".to_owned(),
            confirmed: true,
            places: BTreeMap::from([
                (LocationRole::GameDir, PathBuf::from("games/skyrim")),
                (
                    LocationRole::PluginList,
                    Path::new("local").join("Plugins.txt"),
                ),
                (LocationRole::Downloads, PathBuf::from("downloads")),
            ]),
            ..Settings::default()
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
