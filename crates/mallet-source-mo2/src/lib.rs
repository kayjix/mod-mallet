//! Mod Mallet's adapter for Mod Organizer 2.
//!
//! Mod Organizer keeps each mod in a folder of its own and never copies or
//! links the files into the game. While a program started from Mod
//! Organizer runs, a virtual file system shows the game the files of every
//! switched-on mod in its data directory, merged by the mods' priority.
//! The game directory itself holds only the game's own files.
//!
//! [`Mo2Setup`] reads what that virtual data directory would show: from an
//! [`Instance`], its `ModOrganizer.ini`, the profile in use (`modlist.txt`,
//! `plugins.txt`, `loadorder.txt`, `settings.ini`), each mod's `meta.ini`
//! and the names of the files in each mod's folder and in the overwrite
//! folder. It answers the same questions as the Vortex adapter (ADR 0009):
//! which mod a file of the game comes from, which files each mod brings,
//! and which switched-off mods have a file.
//!
//! Nothing is ever written: not into Mod Organizer's files, not into a
//! mod. Mod Organizer has no outside entry point for actions, so changes
//! are described to the user instead (ADR 0064).

pub mod ini;
pub mod instance;
pub mod meta;
pub mod profile;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use mallet_core::{GamePath, ModRef, ModSource};

pub use instance::{Instance, InstanceSettings, global_instances};
pub use meta::ModMeta;
pub use profile::{EntryState, ModListEntry, PluginEntry, ProfileSettings};

/// The value of [`ModRef::source`] for mods managed by Mod Organizer.
pub const SOURCE_ID: &str = "mo2";

/// The identifier of the overwrite folder, which Mod Organizer shows as a
/// mod with the highest priority. No mod folder can have this name.
pub const OVERWRITE_ID: &str = "Overwrite";

/// The ending Mod Organizer gives a file it hides in a mod's folder.
const HIDDEN_SUFFIX: &str = ".mohidden";

/// Why an instance could not be read.
#[derive(Debug, thiserror::Error)]
pub enum Mo2Error {
    /// A file the instance needs could not be read.
    #[error("`{}` could not be read: {source}", .path.display())]
    Read {
        /// The file.
        path: PathBuf,
        /// What the system reported.
        source: std::io::Error,
    },
    /// The profile does not exist.
    #[error("Mod Organizer has no profile `{name}` in `{}`", .dir.display())]
    NoProfile {
        /// The profile's name.
        name: String,
        /// The folder it was looked for in.
        dir: PathBuf,
    },
}

/// A mod as Mod Organizer's profile and its `meta.ini` describe it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mo2Mod {
    /// The mod's folder name, which is also the name Mod Organizer shows.
    pub folder: String,
    /// Whether the mod is switched on in the profile.
    pub enabled: bool,
    /// The mod's priority: 0 is the lowest. Of two mods with the same file,
    /// the game sees the one with the higher priority.
    pub priority: usize,
    /// The title of the separator the mod is listed under, if any.
    pub separator: Option<String>,
    /// What `meta.ini` says about the mod.
    pub meta: ModMeta,
}

/// What a profile of Mod Organizer says, besides its mods.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Mo2Profile {
    /// The profile's name.
    pub name: String,
    /// The profile's folder. It holds `plugins.txt`, so it is what the
    /// game's load order is read from.
    pub dir: PathBuf,
    /// The plugins of `plugins.txt`.
    pub plugins: Vec<PluginEntry>,
    /// The plugins of `loadorder.txt`, in load order.
    pub load_order: Vec<String>,
    /// What the profile keeps for itself.
    pub settings: ProfileSettings,
}

/// The mods of one profile of a Mod Organizer instance, the files each
/// brings, and which of them the game sees.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Mo2Setup {
    settings: InstanceSettings,
    profile: Mo2Profile,
    /// The mods, lowest priority first. Separators and parts of the game
    /// that Mod Organizer does not manage are not among them.
    mods: Vec<Mo2Mod>,
    /// Every file of each mod, by the mod's position in `mods`.
    files: Vec<Vec<GamePath>>,
    /// Every file of the overwrite folder.
    overwrite: Vec<GamePath>,
    /// The mod whose copy the game sees, by file: a position in `mods`, or
    /// `None` for the overwrite folder.
    winners: HashMap<GamePath, Option<usize>>,
    /// The switched-off mods that have each file, by position in `mods`.
    switched_off: HashMap<GamePath, Vec<usize>>,
}

impl Mo2Setup {
    /// Puts together what was read: the instance's settings, the profile,
    /// the lines of `modlist.txt` (highest priority first, as in the file),
    /// each mod's `meta.ini` and the files below its folder, and the files
    /// of the overwrite folder. Files are named relative to their folder,
    /// as [`list_mod_folder`] lists them. Mods without an entry in `metas`
    /// or `files` have no details or no files.
    pub fn new(
        settings: InstanceSettings,
        profile: Mo2Profile,
        modlist: &[ModListEntry],
        metas: &HashMap<String, ModMeta>,
        files: &HashMap<String, Vec<String>>,
        overwrite: &[String],
    ) -> Self {
        let data = GamePath::new("data");
        let in_data = |names: &[String]| -> Vec<GamePath> {
            names.iter().map(|name| data.join(name)).collect()
        };

        let mut mods = Vec::new();
        let mut mod_files = Vec::new();
        let mut separator: Option<String> = None;
        // A mod listed twice keeps its higher priority, as in Mod
        // Organizer, which reads the file from the top.
        let mut seen = std::collections::HashSet::new();
        let listed: Vec<&ModListEntry> = modlist
            .iter()
            .filter(|entry| entry.state != EntryState::Unmanaged)
            .filter(|entry| seen.insert(entry.name.as_str()))
            .collect();
        // The file lists the highest priority first.
        for entry in listed.into_iter().rev() {
            if let Some(title) = entry.separator_title() {
                separator = Some(title.to_owned());
                continue;
            }
            mods.push(Mo2Mod {
                folder: entry.name.clone(),
                enabled: entry.state == EntryState::Enabled,
                priority: 0,
                separator: separator.clone(),
                meta: metas.get(&entry.name).cloned().unwrap_or_default(),
            });
            mod_files.push(
                files
                    .get(&entry.name)
                    .map_or_else(Vec::new, |names| in_data(names)),
            );
        }
        for (priority, found) in mods.iter_mut().enumerate() {
            found.priority = priority;
        }

        let mut winners: HashMap<GamePath, Option<usize>> = HashMap::new();
        let mut switched_off: HashMap<GamePath, Vec<usize>> = HashMap::new();
        for (index, (found, paths)) in mods.iter().zip(&mod_files).enumerate() {
            for path in paths {
                if found.enabled {
                    winners.insert(path.clone(), Some(index));
                } else {
                    switched_off.entry(path.clone()).or_default().push(index);
                }
            }
        }
        let overwrite = in_data(overwrite);
        for path in &overwrite {
            winners.insert(path.clone(), None);
        }

        Self {
            settings,
            profile,
            mods,
            files: mod_files,
            overwrite,
            winners,
            switched_off,
        }
    }

    /// Reads the profile `profile` of `instance`, or the profile in use if
    /// `None`. Only reads: the instance's settings, the profile's lists,
    /// each listed mod's `meta.ini` and the names of the files in the mod
    /// folders. A mod whose folder cannot be listed counts as having no
    /// files.
    ///
    /// # Errors
    ///
    /// If `ModOrganizer.ini` or the profile's `modlist.txt` cannot be read,
    /// or the profile does not exist.
    pub fn read(instance: &Instance, profile: Option<&str>) -> Result<Self, Mo2Error> {
        let ini_path = instance.ini_path();
        let settings = InstanceSettings::parse(&read_text(&ini_path)?, &instance.dir);
        let name = settings.profile_name(profile);
        let profile_dir = settings.profile_dir(&name);
        if !profile_dir.is_dir() {
            return Err(Mo2Error::NoProfile {
                name,
                dir: settings.profiles_dir.clone(),
            });
        }
        let modlist = profile::parse_modlist(&read_text(&profile_dir.join(profile::MODLIST_FILE))?);
        let optional = |file: &str| read_text(&profile_dir.join(file)).unwrap_or_default();
        let profile_facts = Mo2Profile {
            plugins: profile::parse_plugins(&optional(profile::PLUGINS_FILE)),
            load_order: profile::parse_load_order(&optional(profile::LOAD_ORDER_FILE)),
            settings: ProfileSettings::parse(&optional(profile::SETTINGS_FILE)),
            dir: profile_dir.clone(),
            name,
        };

        let mut metas = HashMap::new();
        let mut files = HashMap::new();
        for entry in &modlist {
            if entry.state == EntryState::Unmanaged || entry.is_separator() {
                continue;
            }
            let folder = settings.mods_dir.join(&entry.name);
            if let Ok(text) = read_text(&folder.join(meta::META_FILE)) {
                metas.insert(entry.name.clone(), ModMeta::parse(&text));
            }
            files.insert(
                entry.name.clone(),
                list_mod_folder(&folder).unwrap_or_default(),
            );
        }
        let overwrite = list_mod_folder(&settings.overwrite_dir).unwrap_or_default();
        Ok(Self::new(
            settings,
            profile_facts,
            &modlist,
            &metas,
            &files,
            &overwrite,
        ))
    }

    /// What `ModOrganizer.ini` says.
    pub fn settings(&self) -> &InstanceSettings {
        &self.settings
    }

    /// The profile that was read.
    pub fn profile(&self) -> &Mo2Profile {
        &self.profile
    }

    /// The mods, lowest priority first.
    pub fn mods(&self) -> &[Mo2Mod] {
        &self.mods
    }

    /// The folders whose files the game sees in its data directory, the one
    /// that wins first: the overwrite folder, then the switched-on mods
    /// from the highest priority down. Behind all of them lies the game's
    /// own data directory.
    pub fn data_folders(&self) -> Vec<PathBuf> {
        let mut folders = vec![self.settings.overwrite_dir.clone()];
        folders.extend(
            self.mods
                .iter()
                .rev()
                .filter(|found| found.enabled)
                .map(|found| self.settings.mods_dir.join(&found.folder)),
        );
        folders
    }

    /// The mod with the folder name `folder`, or the overwrite folder.
    pub fn mod_ref(&self, folder: &str) -> ModRef {
        ModRef {
            source: SOURCE_ID.to_owned(),
            id: folder.to_owned(),
            name: folder.to_owned(),
        }
    }

    /// How many files the game sees from the mods and the overwrite folder.
    pub fn file_count(&self) -> usize {
        self.winners.len()
    }

    /// How many mods are switched on.
    pub fn enabled_count(&self) -> usize {
        self.mods.iter().filter(|found| found.enabled).count()
    }

    fn mod_at(&self, index: Option<usize>) -> Option<ModRef> {
        match index {
            None => Some(self.mod_ref(OVERWRITE_ID)),
            Some(index) => Some(self.mod_ref(&self.mods.get(index)?.folder)),
        }
    }
}

impl ModSource for Mo2Setup {
    fn provider_of(&self, path: &GamePath) -> Option<ModRef> {
        self.mod_at(*self.winners.get(path)?)
    }

    fn staged_files(&self, mod_ref: &ModRef) -> Option<Vec<GamePath>> {
        if mod_ref.source != SOURCE_ID {
            return None;
        }
        if mod_ref.id == OVERWRITE_ID {
            return Some(self.overwrite.clone());
        }
        let index = self
            .mods
            .iter()
            .position(|found| found.folder == mod_ref.id)?;
        self.files.get(index).cloned()
    }

    fn switched_off_with(&self, path: &GamePath) -> Vec<ModRef> {
        self.switched_off
            .get(path)
            .into_iter()
            .flatten()
            .filter_map(|index| self.mod_at(Some(*index)))
            .collect()
    }
}

/// The files below one mod's folder, as paths relative to it with
/// backslashes. Mod Organizer's own `meta.ini` at the top and the files it
/// hid (`.mohidden`) are left out: the game never sees them. Only folder
/// listings are read. Entries that cannot be read are left out.
///
/// # Errors
///
/// If the folder itself cannot be listed.
pub fn list_mod_folder(folder: &Path) -> std::io::Result<Vec<String>> {
    let mut files = Vec::new();
    let mut pending = vec![(std::fs::read_dir(folder)?, String::new())];
    while let Some((entries, prefix)) = pending.pop() {
        for entry in entries.flatten() {
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.to_lowercase().ends_with(HIDDEN_SUFFIX) {
                continue;
            }
            if prefix.is_empty() && kind.is_file() && name.eq_ignore_ascii_case(meta::META_FILE) {
                continue;
            }
            let path = if prefix.is_empty() {
                name
            } else {
                format!(r"{prefix}\{name}")
            };
            if kind.is_dir() {
                if let Ok(inner) = std::fs::read_dir(entry.path()) {
                    pending.push((inner, path));
                }
            } else {
                files.push(path);
            }
        }
    }
    files.sort_unstable();
    Ok(files)
}

/// Reads a text file. Bytes that are not UTF-8 are replaced.
fn read_text(path: &Path) -> Result<String, Mo2Error> {
    std::fs::read(path)
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .map_err(|source| Mo2Error::Read {
            path: path.to_path_buf(),
            source,
        })
}

#[cfg(test)]
mod tests;
