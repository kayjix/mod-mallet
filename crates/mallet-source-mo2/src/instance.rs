//! Instances of Mod Organizer and their settings.
//!
//! An instance is one setup that Mod Organizer manages: a game, its mods,
//! profiles and downloads. It is either
//!
//! * **portable**: a folder with `ModOrganizer.exe`, `ModOrganizer.ini` and
//!   an empty `portable.txt`, anywhere the user put it, or
//! * **global**: a folder `%LOCALAPPDATA%\ModOrganizer\<name>` holding
//!   `ModOrganizer.ini`, shared by every Mod Organizer on the machine.
//!
//! Global instances are found by listing that one folder. Portable ones
//! are named by the user: Mod Mallet does not search the disk (ADR 0018).

use std::path::{Path, PathBuf};

use crate::ini::Ini;

/// The settings file of an instance.
pub const INI_FILE: &str = "ModOrganizer.ini";

/// The file that marks a portable instance.
const PORTABLE_MARKER: &str = "portable.txt";

/// The folder below the local application data in which Mod Organizer
/// keeps its global instances.
const GLOBAL_DIR: &str = "ModOrganizer";

/// Placeholder in an instance's paths for its base folder.
const BASE_DIR: &str = "%BASE_DIR%";

/// An instance of Mod Organizer on this machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instance {
    /// The name to show: the folder's name.
    pub name: String,
    /// The folder that holds `ModOrganizer.ini`.
    pub dir: PathBuf,
    /// Whether it is a portable instance.
    pub portable: bool,
}

impl Instance {
    /// The instance at `path`: its folder, its `ModOrganizer.ini`, or the
    /// `ModOrganizer.exe` beside it. `None` if there is no
    /// `ModOrganizer.ini`. Only looks whether files exist.
    pub fn at(path: &Path) -> Option<Self> {
        let dir = if path.is_dir() {
            path.to_path_buf()
        } else {
            path.parent()?.to_path_buf()
        };
        if !dir.join(INI_FILE).is_file() {
            return None;
        }
        Some(Self {
            name: dir
                .file_name()
                .map_or_else(String::new, |name| name.to_string_lossy().into_owned()),
            portable: dir.join(PORTABLE_MARKER).is_file(),
            dir,
        })
    }

    /// The instance's `ModOrganizer.ini`.
    pub fn ini_path(&self) -> PathBuf {
        self.dir.join(INI_FILE)
    }
}

/// The folder in which Mod Organizer keeps its global instances, given the
/// local application data folder (`%LOCALAPPDATA%`).
pub fn global_instances_dir(local_data_dir: &Path) -> PathBuf {
    local_data_dir.join(GLOBAL_DIR)
}

/// The global instances, by name. Only the folder is listed and the
/// existence of each `ModOrganizer.ini` checked; nothing is opened.
pub fn global_instances(local_data_dir: &Path) -> Vec<Instance> {
    let Ok(entries) = std::fs::read_dir(global_instances_dir(local_data_dir)) else {
        return Vec::new();
    };
    let mut instances: Vec<Instance> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .filter_map(|entry| Instance::at(&entry.path()))
        .collect();
    instances.sort_by_key(|instance| instance.name.to_lowercase());
    instances
}

/// What an instance's `ModOrganizer.ini` says, with its paths resolved.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InstanceSettings {
    /// The game, as Mod Organizer names it (`Skyrim Special Edition`).
    pub game_name: Option<String>,
    /// Where the game is installed.
    pub game_path: Option<PathBuf>,
    /// The profile in use.
    pub selected_profile: Option<String>,
    /// The folder the other folders lie in unless they are set apart.
    pub base_dir: PathBuf,
    /// Where each mod has its folder.
    pub mods_dir: PathBuf,
    /// Where each profile has its folder.
    pub profiles_dir: PathBuf,
    /// Where files land that programs started from Mod Organizer write.
    pub overwrite_dir: PathBuf,
    /// Where downloaded archives are kept.
    pub downloads_dir: PathBuf,
}

impl InstanceSettings {
    /// Reads the text of `ModOrganizer.ini`. `instance_dir` is the folder
    /// it lies in, against which paths that are not absolute are taken.
    pub fn parse(text: &str, instance_dir: &Path) -> Self {
        let ini = Ini::parse(text);
        let base_dir = ini.text("Settings", "base_directory").map_or_else(
            || instance_dir.to_path_buf(),
            |base| absolute(&base, instance_dir),
        );
        let folder = |key: &str, default: &str| {
            let template = ini
                .text("Settings", key)
                .unwrap_or_else(|| format!("{BASE_DIR}/{default}"));
            resolve(&template, &base_dir, instance_dir)
        };
        Self {
            game_name: ini.text("General", "gameName"),
            game_path: ini
                .text("General", "gamePath")
                .map(|path| absolute(&path, instance_dir)),
            selected_profile: ini.text("General", "selected_profile"),
            mods_dir: folder("mod_directory", "mods"),
            profiles_dir: folder("profiles_directory", "profiles"),
            overwrite_dir: folder("overwrite_directory", "overwrite"),
            downloads_dir: folder("download_directory", "downloads"),
            base_dir,
        }
    }

    /// The profile to read: `wanted` if given, else the one in use, else
    /// `Default`, the profile every instance starts with.
    pub fn profile_name(&self, wanted: Option<&str>) -> String {
        wanted
            .map(str::to_owned)
            .or_else(|| self.selected_profile.clone())
            .unwrap_or_else(|| "Default".to_owned())
    }

    /// The folder of the profile `name`.
    pub fn profile_dir(&self, name: &str) -> PathBuf {
        self.profiles_dir.join(name)
    }

    /// Mod Mallet's identifier of the game the instance manages, if the
    /// game is one Mod Mallet knows.
    pub fn game_id(&self) -> Option<&'static str> {
        game_id(self.game_name.as_deref()?)
    }
}

/// Mod Mallet's identifier of a game, given the name Mod Organizer writes
/// into `ModOrganizer.ini`. `None` for games Mod Mallet does not know,
/// such as Enderal, which runs on Skyrim but is a game of its own.
pub fn game_id(game_name: &str) -> Option<&'static str> {
    const GAMES: &[(&str, &str)] = &[
        ("skyrim special edition", "skyrimse"),
        ("skyrim vr", "skyrimvr"),
        ("skyrim", "skyrim"),
        ("fallout 4", "fallout4"),
        ("fallout 4 vr", "fallout4vr"),
        ("new vegas", "falloutnv"),
        ("fallout nv", "falloutnv"),
        ("ttw", "falloutnv"),
        ("fallout 3", "fallout3"),
        ("oblivion", "oblivion"),
    ];
    let wanted = game_name.trim().to_lowercase();
    GAMES
        .iter()
        .find(|(name, _)| *name == wanted)
        .map(|(_, id)| *id)
}

/// A path of the settings with `%BASE_DIR%` put in.
fn resolve(template: &str, base_dir: &Path, instance_dir: &Path) -> PathBuf {
    let starts_with_base = template
        .get(..BASE_DIR.len())
        .is_some_and(|start| start.eq_ignore_ascii_case(BASE_DIR));
    match template.get(BASE_DIR.len()..) {
        Some(rest) if starts_with_base => {
            let rest = rest.trim_start_matches(['/', '\\']);
            if rest.is_empty() {
                base_dir.to_path_buf()
            } else {
                base_dir.join(rest)
            }
        }
        _ => absolute(template, instance_dir),
    }
}

/// `path`, taken against `instance_dir` if it is not absolute.
fn absolute(path: &str, instance_dir: &Path) -> PathBuf {
    let path = PathBuf::from(path);
    // A Windows path with a drive letter counts as absolute everywhere, so
    // that settings written on Windows read the same in tests elsewhere.
    let has_drive = path
        .to_str()
        .is_some_and(|text| text.as_bytes().get(1) == Some(&b':'));
    if path.is_absolute() || has_drive {
        path
    } else {
        instance_dir.join(path)
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    /// The shape of `ModOrganizer.ini` of a global instance; made up.
    const GLOBAL_INI: &str = "[General]\r\ngameName=Skyrim Special Edition\r\nselected_profile=@ByteArray(Survival)\r\ngamePath=@ByteArray(D:\\\\Games\\\\Skyrim Special Edition)\r\nversion=2.5.2\r\nfirst_start=false\r\n\r\n[Settings]\r\nbase_directory=D:/Modding/Skyrim\r\ndownload_directory=E:/Downloads\r\nmod_directory=%BASE_DIR%/mods\r\n";

    #[test]
    fn reads_the_game_the_profile_and_the_folders() {
        let instance_dir = Path::new("/local/ModOrganizer/Skyrim SE");
        let settings = InstanceSettings::parse(GLOBAL_INI, instance_dir);
        assert_eq!(
            settings.game_name.as_deref(),
            Some("Skyrim Special Edition")
        );
        assert_eq!(settings.game_id(), Some("skyrimse"));
        assert_eq!(
            settings.game_path,
            Some(PathBuf::from(r"D:\Games\Skyrim Special Edition"))
        );
        assert_eq!(settings.selected_profile.as_deref(), Some("Survival"));
        let base = PathBuf::from("D:/Modding/Skyrim");
        assert_eq!(settings.base_dir, base);
        assert_eq!(settings.mods_dir, base.join("mods"));
        assert_eq!(settings.profiles_dir, base.join("profiles"));
        assert_eq!(settings.overwrite_dir, base.join("overwrite"));
        assert_eq!(settings.downloads_dir, PathBuf::from("E:/Downloads"));
        assert_eq!(
            settings.profile_dir("Survival"),
            base.join("profiles").join("Survival")
        );
    }

    #[test]
    fn a_portable_instance_keeps_everything_beside_its_settings() {
        let instance_dir = Path::new("/tools/MO2");
        let settings = InstanceSettings::parse("[General]\ngameName=Fallout 4\n", instance_dir);
        assert_eq!(settings.base_dir, instance_dir);
        assert_eq!(settings.mods_dir, instance_dir.join("mods"));
        assert_eq!(settings.profile_name(None), "Default");
        assert_eq!(settings.profile_name(Some("Other")), "Other");
        assert_eq!(settings.game_id(), Some("fallout4"));
    }

    #[test]
    fn relative_folders_lie_in_the_instance() {
        let instance_dir = Path::new("/tools/MO2");
        let settings =
            InstanceSettings::parse("[Settings]\nmod_directory=../shared/mods\n", instance_dir);
        assert_eq!(settings.mods_dir, instance_dir.join("../shared/mods"));
    }

    #[test]
    fn games_mod_mallet_does_not_know_have_no_identifier() {
        assert_eq!(game_id("Enderal Special Edition"), None);
        assert_eq!(game_id("  oblivion "), Some("oblivion"));
    }

    #[test]
    fn instances_are_found_by_their_settings_file() {
        let local = tempfile::tempdir().unwrap();
        let root = global_instances_dir(local.path());
        for name in ["Skyrim SE", "Fallout 4"] {
            std::fs::create_dir_all(root.join(name)).unwrap();
            std::fs::write(root.join(name).join(INI_FILE), "[General]\n").unwrap();
        }
        // A folder without settings is not an instance.
        std::fs::create_dir_all(root.join("cache")).unwrap();

        let instances = global_instances(local.path());
        let names: Vec<_> = instances.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(names, ["Fallout 4", "Skyrim SE"]);
        assert!(instances.iter().all(|instance| !instance.portable));
        assert_eq!(global_instances(&local.path().join("missing")), Vec::new());
    }

    #[test]
    fn a_portable_instance_is_found_by_its_folder_or_its_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(INI_FILE), "").unwrap();
        std::fs::write(dir.path().join("portable.txt"), "").unwrap();
        let by_folder = Instance::at(dir.path()).unwrap();
        assert!(by_folder.portable);
        assert_eq!(
            Instance::at(&dir.path().join(INI_FILE)),
            Some(by_folder.clone())
        );
        assert_eq!(
            Instance::at(&dir.path().join("ModOrganizer.exe")),
            Some(by_folder)
        );
        assert_eq!(Instance::at(&dir.path().join("nothing").join("here")), None);
    }

    proptest! {
        #[test]
        fn never_panics_on_arbitrary_input(text in ".*") {
            let settings = InstanceSettings::parse(&text, Path::new("instance"));
            let _ = settings.game_id();
            let _ = settings.profile_dir(&settings.profile_name(None));
        }
    }
}
