//! Which mod manager looks after a game, and reading Mod Organizer 2
//! (ADR 0064).
//!
//! Vortex links the files of the mods into the game; Mod Organizer leaves
//! them in each mod's folder and shows them to the game while it runs.
//! For Mod Organizer, the scan therefore reads the mods' folders as the
//! game would see them, as [`VirtualData`] laid over the data directory.
//!
//! Finding out which managers are on the machine opens no file: it looks
//! whether Vortex's folder exists and lists the folder of Mod Organizer's
//! global instances (ADR 0018). Which instance belongs to which game is
//! only read once the user has agreed.

use std::path::{Path, PathBuf};

use mallet_core::managers::{MOD_ORGANIZER, VORTEX};
use mallet_core::{
    KnownInstance, KnownManager, LocationOrigin, LocationRole, ManagerChoice, ModInfo, ModRef,
    NoModSource, ScanLocation,
};
use mallet_game_bethesda::Game;
use mallet_game_bethesda::ingest::VirtualData;
use mallet_source_mo2::{Instance, Mo2Setup, global_instances, profile};

use crate::ScanOptions;

/// The mod manager whose records a scan reads.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum ModManager {
    /// Vortex, at its standard places or those named in [`ScanOptions`].
    #[default]
    Vortex,
    /// Mod Organizer 2.
    ModOrganizer {
        /// The instance: its folder, its `ModOrganizer.ini` or its
        /// `ModOrganizer.exe`. If `None`, the global instance for the game.
        instance: Option<PathBuf>,
        /// The profile to read. If `None`, the one in use.
        profile: Option<String>,
    },
}

impl ModManager {
    /// The manager with the identifier `id` (`vortex`, `mo2`), with the
    /// instance `instance` where it has instances. `None` for an unknown
    /// identifier.
    pub fn from_id(id: &str, instance: Option<PathBuf>) -> Option<Self> {
        match id {
            VORTEX => Some(Self::Vortex),
            MOD_ORGANIZER => Some(Self::ModOrganizer {
                instance,
                profile: None,
            }),
            _ => None,
        }
    }

    /// The manager's identifier.
    pub fn id(&self) -> &'static str {
        match self {
            Self::Vortex => VORTEX,
            Self::ModOrganizer { .. } => MOD_ORGANIZER,
        }
    }
}

/// The folder of Mod Mallet's user's local application data.
fn local_data_dir() -> Option<PathBuf> {
    dirs::data_local_dir()
}

/// Whether Vortex is on this machine: its data folder exists, or it
/// deployed into the game.
fn vortex_found(game_dir: Option<&Path>) -> bool {
    let data = dirs::config_dir().is_some_and(|dir| dir.join("Vortex").is_dir());
    data || game_dir.is_some_and(|dir| mallet_source_vortex::data_manifest_path(dir).is_file())
}

/// Mod Organizer's instances on this machine: the global ones, and the one
/// at `named` if the user named one. Nothing is opened.
fn mo2_instances(named: Option<&Path>) -> Vec<Instance> {
    let mut instances = local_data_dir().map_or_default(|dir| global_instances(&dir));
    if let Some(instance) = named.and_then(Instance::at)
        && !instances.iter().any(|known| known.dir == instance.dir)
    {
        instances.push(instance);
    }
    instances
}

/// The manager to suggest where the user has not chosen one: Vortex if it
/// deployed into the game or Mod Organizer is not there, else Mod
/// Organizer.
pub(crate) fn suggested(game_dir: Option<&Path>) -> &'static str {
    let deployed =
        game_dir.is_some_and(|dir| mallet_source_vortex::data_manifest_path(dir).is_file());
    if deployed || vortex_found(game_dir) || mo2_instances(None).is_empty() {
        VORTEX
    } else {
        MOD_ORGANIZER
    }
}

/// The managers Mod Mallet can read, which are on this machine, and which
/// is in use: `chosen` if the user chose one. `instance` is the instance
/// the user named. Only looks whether folders exist.
pub(crate) fn known(
    chosen: Option<&str>,
    game_dir: Option<&Path>,
    instance: Option<&Path>,
) -> ManagerChoice {
    let instances = mo2_instances(instance);
    let active = chosen
        .filter(|id| ModManager::from_id(id, None).is_some())
        .map_or_else(|| suggested(game_dir).to_owned(), str::to_owned);
    ManagerChoice {
        chosen: chosen.is_some_and(|id| id == active),
        active,
        managers: vec![
            KnownManager {
                id: VORTEX.to_owned(),
                name: "Vortex".to_owned(),
                found: vortex_found(game_dir),
                link: true,
                instances: Vec::new(),
            },
            KnownManager {
                id: MOD_ORGANIZER.to_owned(),
                name: "Mod Organizer 2".to_owned(),
                found: !instances.is_empty(),
                link: false,
                instances: instances
                    .into_iter()
                    .map(|instance| KnownInstance {
                        path: instance.dir.display().to_string(),
                        name: instance.name,
                        portable: instance.portable,
                    })
                    .collect(),
            },
        ],
    }
}

/// The instance a scan would read, as the places page shows it before
/// anything is read: the one named, or the only global instance. With
/// several global instances and none named, the user has to pick one.
pub(crate) fn planned_instance(named: Option<&Path>) -> ScanLocation {
    if let Some(named) = named {
        return ScanLocation {
            role: LocationRole::ManagerInstance,
            path: named.display().to_string(),
            origin: LocationOrigin::Given,
            found: Instance::at(named).is_some(),
        };
    }
    let instances = mo2_instances(None);
    match instances.as_slice() {
        [only] => ScanLocation {
            role: LocationRole::ManagerInstance,
            path: only.dir.display().to_string(),
            origin: LocationOrigin::Standard,
            found: true,
        },
        _ => ScanLocation {
            role: LocationRole::ManagerInstance,
            path: String::new(),
            origin: LocationOrigin::Standard,
            found: false,
        },
    }
}

/// The instance to read for `game`: the one named, or else the global
/// instance whose settings say it manages the game, or else the only
/// global instance. Reads the global instances' settings.
fn instance_for(game: Game, named: Option<&Path>) -> Option<Instance> {
    if let Some(named) = named {
        return Instance::at(named);
    }
    let instances = mo2_instances(None);
    let manages_game = |instance: &&Instance| {
        std::fs::read(instance.ini_path()).is_ok_and(|bytes| {
            let text = String::from_utf8_lossy(&bytes);
            mallet_source_mo2::InstanceSettings::parse(&text, &instance.dir).game_id()
                == Some(game.id())
        })
    };
    instances
        .iter()
        .find(manages_game)
        .cloned()
        .or_else(|| match instances.as_slice() {
            [only] => Some(only.clone()),
            _ => None,
        })
}

/// The mod manager of a scan, as far as it is read before the game.
pub(crate) enum ScanManager {
    /// Vortex, which is read after the game.
    Vortex,
    /// Mod Organizer, if its instance could be read.
    ModOrganizer(Option<Mo2Reading>),
}

impl ScanManager {
    /// Reads Mod Organizer if `options` say it manages the game, and notes
    /// the places read in `locations`.
    pub(crate) fn read(
        options: &ScanOptions,
        game: Game,
        locations: &mut Vec<ScanLocation>,
    ) -> Self {
        match &options.manager {
            ModManager::ModOrganizer { instance, profile } => Self::ModOrganizer(read(
                game,
                instance.as_deref(),
                profile.as_deref(),
                locations,
            )),
            ModManager::Vortex => Self::Vortex,
        }
    }

    /// The folders the game sees over its data directory.
    pub(crate) fn virtual_data(&self) -> VirtualData {
        match self {
            Self::ModOrganizer(Some(reading)) => VirtualData {
                folders: reading.setup.data_folders(),
            },
            _ => VirtualData::default(),
        }
    }

    /// What is learned from the manager: which mod each file belongs to,
    /// the mods' installers, and what is known about each mod. Vortex is
    /// read now.
    pub(crate) fn into_parts(
        self,
        options: &ScanOptions,
        game_dir: &Path,
        store: Option<&mallet_store::Store>,
        locations: &mut Vec<ScanLocation>,
    ) -> Result<crate::ManagerParts, crate::EngineError> {
        Ok(match self {
            Self::ModOrganizer(Some(reading)) => {
                let infos = reading.mod_infos();
                (reading.setup, Vec::new(), infos)
            }
            Self::ModOrganizer(None) => (Box::new(NoModSource), Vec::new(), Vec::new()),
            Self::Vortex => crate::load_mod_source(options, game_dir, store, locations)?,
        })
    }

    /// `options` with the profile's folder as the folder of `plugins.txt`,
    /// unless another was named. `None` where nothing changes.
    pub(crate) fn scan_options(&self, options: &ScanOptions) -> Option<ScanOptions> {
        match self {
            Self::ModOrganizer(Some(reading)) if options.local_dir.is_none() => Some(ScanOptions {
                local_dir: Some(reading.setup.profile().dir.clone()),
                ..options.clone()
            }),
            _ => None,
        }
    }
}

/// What a scan learned from Mod Organizer.
pub(crate) struct Mo2Reading {
    /// The mods and their files.
    setup: Box<Mo2Setup>,
}

impl Mo2Reading {
    /// What is known about each mod, for the report.
    fn mod_infos(&self) -> Vec<ModInfo> {
        self.setup
            .mods()
            .iter()
            .map(|found| ModInfo {
                mod_ref: ModRef {
                    source: mallet_source_mo2::SOURCE_ID.to_owned(),
                    id: found.folder.clone(),
                    name: found.folder.clone(),
                },
                enabled: found.enabled,
                version: found.meta.version.clone(),
                newer_version: found.meta.newer_version(),
                author: None,
                page: found.meta.page(),
            })
            .collect()
    }
}

/// Reads the instance and profile `instance` and `profile` name, for a
/// scan of `game`, and notes the places read in `locations`. `None` if
/// there is no instance or it cannot be read, or it manages another game;
/// the scan then goes on without a mod manager.
fn read(
    game: Game,
    instance: Option<&Path>,
    profile: Option<&str>,
    locations: &mut Vec<ScanLocation>,
) -> Option<Mo2Reading> {
    let origin = if instance.is_some() {
        LocationOrigin::Given
    } else {
        LocationOrigin::Standard
    };
    let Some(found) = instance_for(game, instance) else {
        tracing::info!("no Mod Organizer instance found; files will not be attributed to mods");
        locations.push(ScanLocation {
            role: LocationRole::ManagerInstance,
            path: instance.map_or_default(|path| path.display().to_string()),
            origin,
            found: false,
        });
        return None;
    };
    let setup = Mo2Setup::read(&found, profile);
    locations.push(ScanLocation {
        role: LocationRole::ManagerInstance,
        path: found.ini_path().display().to_string(),
        origin,
        found: setup.is_ok(),
    });
    let setup = match setup {
        Ok(setup) => setup,
        Err(error) => {
            tracing::warn!(%error, "the Mod Organizer instance could not be read");
            return None;
        }
    };
    if let Some(other) = setup.settings().game_id()
        && other != game.id()
    {
        tracing::warn!(
            instance = %found.dir.display(),
            manages = other,
            "the Mod Organizer instance manages another game; it is not used"
        );
        return None;
    }
    // The profile's list of plugins is noted where the load order is read.
    locations.push(ScanLocation {
        role: LocationRole::ManagerMods,
        path: setup.settings().mods_dir.display().to_string(),
        origin: LocationOrigin::ManagerSettings,
        found: setup.settings().mods_dir.is_dir(),
    });
    tracing::info!(
        instance = %found.dir.display(),
        profile = setup.profile().name,
        mods = setup.mods().len(),
        enabled = setup.enabled_count(),
        files = setup.file_count(),
        plugins_active = setup.profile().plugins.iter().filter(|p| p.active).count(),
        local_saves = setup.profile().settings.local_saves,
        local_settings = setup.profile().settings.local_settings,
        "read Mod Organizer"
    );
    Some(Mo2Reading {
        setup: Box::new(setup),
    })
}

/// The files of the profile whose changes call for a new scan: the list of
/// mods and the list of plugins. Reads the instance's settings to find the
/// profile.
pub(crate) fn watched_files(options: &ScanOptions) -> Vec<PathBuf> {
    let ModManager::ModOrganizer { instance, profile } = &options.manager else {
        return Vec::new();
    };
    let Ok(game) = options.game() else {
        return Vec::new();
    };
    let Some(found) = instance_for(game, instance.as_deref()) else {
        return Vec::new();
    };
    let Ok(bytes) = std::fs::read(found.ini_path()) else {
        return Vec::new();
    };
    let settings =
        mallet_source_mo2::InstanceSettings::parse(&String::from_utf8_lossy(&bytes), &found.dir);
    let dir = settings.profile_dir(&settings.profile_name(profile.as_deref()));
    vec![
        found.ini_path(),
        dir.join(profile::MODLIST_FILE),
        dir.join(profile::PLUGINS_FILE),
        settings.overwrite_dir,
    ]
}
