//! Vortex's saved state: the installed mods and what was chosen in their
//! installers.
//!
//! Vortex keeps its state in a database that only Vortex should open. It
//! also writes the whole state as JSON to a backup file, about once an
//! hour; restarting Vortex does not renew it. Mod Mallet reads that backup
//! (ADR 0009). It can be an hour or more behind what Vortex shows.
//!
//! Where the Mod Mallet link is installed in Vortex, it writes the parts
//! below into a file of its own whenever they change (ADR 0021). That file
//! has the same shape and is read by the same code.
//!
//! The parts read here, for one game:
//!
//! ```text
//! settings.downloads.path                      where archives are kept
//! settings.profiles.lastActiveProfile.<game>   the profile in use
//! persistent.profiles.<profile>.modState       which mods are enabled
//! persistent.mods.<game>.<mod>                 folder, archive, attributes
//! persistent.downloads.files.<archive>         the archive's file name
//! ```

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use mallet_core::facts::installer::ChosenOption;
use serde::Deserialize;

use crate::VortexError;

/// Placeholder in Vortex's paths for its own data directory.
const USERDATA: &str = "{USERDATA}";

/// Placeholder in Vortex's paths for the game.
const GAME: &str = "{game}";

/// Where Vortex keeps downloads unless the user chose another place.
const DEFAULT_DOWNLOADS: &str = r"{USERDATA}\downloads";

/// The backup of Vortex's state, given the directory in which Vortex keeps
/// its data (`%APPDATA%\Vortex` on Windows).
pub fn state_backup_path(vortex_data_dir: &Path) -> PathBuf {
    vortex_data_dir
        .join("temp")
        .join("state_backups_full")
        .join("hourly.json")
}

/// The copy of Vortex's state that the Mod Mallet link keeps up to date for
/// `game`, given the folder shared with the link.
pub fn live_state_path(link_dir: &Path, game: &str) -> PathBuf {
    link_dir.join("state").join(format!("{game}.json"))
}

/// A mod as Vortex's state describes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VortexMod {
    /// The mod's folder in Vortex's staging directory. Deployment manifests
    /// refer to mods by this name.
    pub folder: String,
    /// The name Vortex shows for the mod.
    pub name: String,
    /// Whether the mod is enabled in the profile in use.
    pub enabled: bool,
    /// File name of the archive the mod was installed from, if Vortex still
    /// knows it.
    pub archive: Option<String>,
    /// The options chosen in the mod's installer. Empty if the mod has no
    /// installer or nothing was chosen.
    pub chosen: Vec<ChosenOption>,
    /// The installed version, as the author numbers it.
    pub version: Option<String>,
    /// The newest version Vortex knows of, if it differs from the
    /// installed one.
    pub newer_version: Option<String>,
    /// Who made the mod.
    pub author: Option<String>,
    /// The page the mod was downloaded from.
    pub page: Option<String>,
}

/// A program Vortex lists among the tools of a game on its dashboard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VortexTool {
    /// Vortex's identifier of the tool.
    pub id: String,
    /// The name Vortex shows.
    pub name: String,
    /// The program Vortex starts, as Vortex noted it.
    pub path: String,
    /// Whether the user or a collection added it by hand, as opposed to
    /// one a game extension knows how to find.
    pub custom: bool,
    /// Whether Vortex hides it on the dashboard.
    pub hidden: bool,
}

/// The parts of Vortex's state that Mod Mallet uses, for one game.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VortexState {
    /// The installed mods of the game.
    pub mods: Vec<VortexMod>,
    /// The tools Vortex lists for the game, by identifier. `None` when the
    /// state says nothing about tools, as the copy of an older link does.
    pub tools: Option<Vec<VortexTool>>,
    downloads_path: Option<String>,
}

impl VortexState {
    /// Reads the text of a state backup. `game` is Vortex's identifier of
    /// the game, for example `skyrimse`.
    pub fn parse(state_json: &str, game: &str) -> Result<Self, VortexError> {
        let raw: RawState = serde_json::from_str(state_json)?;

        let profile = raw
            .settings
            .profiles
            .last_active_profile
            .get(game)
            .and_then(|id| raw.persistent.profiles.get(id));

        let mut mods: Vec<VortexMod> = raw
            .persistent
            .mods
            .get(game)
            .into_iter()
            .flatten()
            .map(|(id, raw_mod)| {
                let folder = raw_mod
                    .installation_path
                    .clone()
                    .unwrap_or_else(|| id.clone());
                let attributes = &raw_mod.attributes;
                let name = [
                    &attributes.custom_file_name,
                    &attributes.logical_file_name,
                    &attributes.mod_name,
                    &attributes.name,
                ]
                .into_iter()
                .flatten()
                .find(|name| !name.trim().is_empty())
                .cloned()
                .unwrap_or_else(|| folder.clone());

                VortexMod {
                    name,
                    // Without a profile there is no record of what is
                    // enabled; everything installed counts.
                    enabled: profile.is_none_or(|profile| {
                        profile.mod_state.get(id).is_some_and(|state| state.enabled)
                    }),
                    archive: raw_mod
                        .archive_id
                        .as_ref()
                        .and_then(|archive| raw.persistent.downloads.files.get(archive))
                        .and_then(|download| download.local_path.clone()),
                    chosen: attributes
                        .installer_choices
                        .as_ref()
                        .map_or_default(chosen_options),
                    version: non_empty(attributes.version.as_ref()),
                    newer_version: non_empty(attributes.newest_version.as_ref())
                        .filter(|newest| Some(newest) != attributes.version.as_ref()),
                    author: non_empty(attributes.author.as_ref()),
                    page: page(attributes),
                    folder,
                }
            })
            .collect();
        mods.sort_by(|a, b| a.folder.cmp(&b.folder));

        let tools = raw.settings.game_mode.discovered.get(game).map(|found| {
            let mut tools: Vec<VortexTool> = found
                .tools
                .iter()
                .filter_map(|(id, tool)| {
                    Some(VortexTool {
                        id: id.clone(),
                        name: non_empty(tool.name.as_ref())?,
                        path: non_empty(tool.path.as_ref())?,
                        custom: tool.custom,
                        hidden: tool.hidden,
                    })
                })
                .collect();
            tools.sort_by(|a, b| a.id.cmp(&b.id));
            tools
        });

        Ok(Self {
            mods,
            tools,
            downloads_path: raw.settings.downloads.path,
        })
    }

    /// The directory in which Vortex keeps the downloaded archives of
    /// `game`, given Vortex's own data directory.
    pub fn downloads_dir(&self, vortex_data_dir: &Path, game: &str) -> PathBuf {
        let template = self.downloads_path.as_deref().unwrap_or(DEFAULT_DOWNLOADS);
        let mut directory = PathBuf::new();
        for (index, part) in template.split(['\\', '/']).enumerate() {
            match part {
                USERDATA if index == 0 => directory.push(vortex_data_dir),
                GAME => directory.push(game),
                // A leading empty part is the root of an absolute path.
                "" if index == 0 => directory.push(std::path::MAIN_SEPARATOR_STR),
                "" => {}
                // A drive letter needs its separator to be absolute.
                drive if index == 0 && drive.ends_with(':') => {
                    directory.push(format!("{drive}{}", std::path::MAIN_SEPARATOR));
                }
                other => directory.push(other),
            }
        }
        if template.contains(GAME) {
            directory
        } else {
            directory.join(game)
        }
    }

    /// The name Vortex shows for each mod, by the mod's staging folder.
    pub fn names(&self) -> HashMap<String, String> {
        self.mods
            .iter()
            .map(|vortex_mod| (vortex_mod.folder.clone(), vortex_mod.name.clone()))
            .collect()
    }
}

fn non_empty(text: Option<&String>) -> Option<String> {
    text.map(|text| text.trim())
        .filter(|text| !text.is_empty())
        .map(str::to_owned)
}

/// The web page of a mod: the one Vortex noted, else the Nexus Mods page
/// of a mod downloaded from there for this game. Only `https` addresses
/// are taken, so that what is shown can be opened without a second
/// thought.
fn page(attributes: &RawAttributes) -> Option<String> {
    if let Some(homepage) = non_empty(attributes.homepage.as_ref())
        && homepage.starts_with("https://")
    {
        return Some(homepage);
    }
    let from_nexus = attributes.source.as_deref() == Some("nexus");
    let domain = match attributes.download_game.as_deref() {
        Some("skyrimse") => "skyrimspecialedition",
        _ => return None,
    };
    let mod_id = match attributes.mod_id.as_ref()? {
        serde_json::Value::Number(number) => number.as_u64()?,
        serde_json::Value::String(text) => text.trim().parse().ok()?,
        _ => return None,
    };
    from_nexus.then(|| format!("https://www.nexusmods.com/{domain}/mods/{mod_id}"))
}

fn chosen_options(choices: &RawChoices) -> Vec<ChosenOption> {
    choices
        .options
        .iter()
        .flat_map(|step| {
            step.groups.iter().flat_map(move |group| {
                group.choices.iter().map(move |choice| ChosenOption {
                    step: step.name.clone(),
                    group: group.name.clone(),
                    option: choice.name.clone(),
                })
            })
        })
        .collect()
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct RawState {
    settings: RawSettings,
    persistent: RawPersistent,
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct RawSettings {
    downloads: RawDownloadSettings,
    profiles: RawProfileSettings,
    game_mode: RawGameMode,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct RawGameMode {
    discovered: HashMap<String, RawDiscovered>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct RawDiscovered {
    tools: HashMap<String, RawTool>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct RawTool {
    name: Option<String>,
    path: Option<String>,
    custom: bool,
    hidden: bool,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct RawDownloadSettings {
    path: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct RawProfileSettings {
    last_active_profile: HashMap<String, String>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct RawPersistent {
    profiles: HashMap<String, RawProfile>,
    mods: HashMap<String, HashMap<String, RawMod>>,
    downloads: RawDownloads,
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct RawProfile {
    mod_state: HashMap<String, RawModState>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct RawModState {
    enabled: bool,
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct RawMod {
    archive_id: Option<String>,
    installation_path: Option<String>,
    attributes: RawAttributes,
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct RawAttributes {
    custom_file_name: Option<String>,
    logical_file_name: Option<String>,
    mod_name: Option<String>,
    name: Option<String>,
    installer_choices: Option<RawChoices>,
    version: Option<String>,
    newest_version: Option<String>,
    author: Option<String>,
    homepage: Option<String>,
    source: Option<String>,
    download_game: Option<String>,
    mod_id: Option<serde_json::Value>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct RawChoices {
    options: Vec<RawStep>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct RawStep {
    name: String,
    groups: Vec<RawGroup>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct RawGroup {
    name: String,
    choices: Vec<RawChoice>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct RawChoice {
    name: String,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct RawDownloads {
    files: HashMap<String, RawDownload>,
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct RawDownload {
    local_path: Option<String>,
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    /// A state backup reduced to the parts that are read. Names are made
    /// up; the shape is that of a backup written by Vortex 2.10.
    const STATE: &str = r#"{
      "settings": {
        "downloads": { "path": "{USERDATA}\\downloads", "maxChunks": 10 },
        "profiles": { "activeProfileId": "p1", "lastActiveProfile": { "skyrimse": "p1", "othergame": "p9" } }
      },
      "persistent": {
        "profiles": {
          "p1": { "gameId": "skyrimse", "name": "Default", "modState": {
            "Patch Hub 1.0": { "enabled": true, "enabledTime": 1 },
            "Disabled Mod": { "enabled": false }
          } },
          "p9": { "gameId": "othergame", "modState": { "Elsewhere": { "enabled": true } } }
        },
        "mods": {
          "skyrimse": {
            "Patch Hub 1.0": {
              "id": "Patch Hub 1.0",
              "archiveId": "a1",
              "installationPath": "Patch Hub 1.0",
              "state": "installed",
              "attributes": {
                "name": "Patch Hub 1.0",
                "modName": "Patch Hub - The Mod",
                "logicalFileName": "Patch Hub",
                "version": "1.0", "newestVersion": "1.2", "author": "Someone",
                "source": "nexus", "downloadGame": "skyrimse", "modId": 1234,
                "installerChoices": { "type": "fomod", "options": [
                  { "name": "Patches", "groups": [
                    { "name": "Cities", "choices": [ { "name": "City Overhaul", "idx": 3 } ] },
                    { "name": "Nothing chosen", "choices": [] }
                  ] }
                ] }
              }
            },
            "Disabled Mod": { "installationPath": "Disabled Mod", "attributes": { "name": "Disabled Mod" } },
            "Never Enabled": { "attributes": {} }
          },
          "othergame": { "Elsewhere": { "attributes": {} } }
        },
        "downloads": { "files": { "a1": { "localPath": "Patch Hub 1.0.rar", "size": 5, "game": ["skyrimse"] } } }
      }
    }"#;

    fn find<'a>(state: &'a VortexState, folder: &str) -> &'a VortexMod {
        state
            .mods
            .iter()
            .find(|vortex_mod| vortex_mod.folder == folder)
            .unwrap()
    }

    #[test]
    fn reads_the_mods_of_one_game() {
        let state = VortexState::parse(STATE, "skyrimse").unwrap();
        let folders: Vec<_> = state.mods.iter().map(|m| m.folder.as_str()).collect();
        assert_eq!(folders, ["Disabled Mod", "Never Enabled", "Patch Hub 1.0"]);
    }

    #[test]
    fn reads_name_archive_and_choices() {
        let state = VortexState::parse(STATE, "skyrimse").unwrap();
        let hub = find(&state, "Patch Hub 1.0");
        assert_eq!(hub.name, "Patch Hub");
        assert!(hub.enabled);
        assert_eq!(hub.archive.as_deref(), Some("Patch Hub 1.0.rar"));
        assert_eq!(
            hub.chosen,
            [ChosenOption {
                step: "Patches".to_owned(),
                group: "Cities".to_owned(),
                option: "City Overhaul".to_owned(),
            }]
        );
    }

    #[test]
    fn reads_version_author_and_the_page_the_mod_came_from() {
        let state = VortexState::parse(STATE, "skyrimse").unwrap();
        let hub = find(&state, "Patch Hub 1.0");
        assert_eq!(hub.version.as_deref(), Some("1.0"));
        assert_eq!(hub.newer_version.as_deref(), Some("1.2"));
        assert_eq!(hub.author.as_deref(), Some("Someone"));
        assert_eq!(
            hub.page.as_deref(),
            Some("https://www.nexusmods.com/skyrimspecialedition/mods/1234")
        );
        // Without such details there is nothing to show.
        let bare = find(&state, "Never Enabled");
        assert_eq!((bare.version.clone(), bare.page.clone()), (None, None));
    }

    #[test]
    fn a_page_that_is_not_https_is_not_taken() {
        let attributes = RawAttributes {
            homepage: Some("javascript:alert(1)".to_owned()),
            ..RawAttributes::default()
        };
        assert_eq!(page(&attributes), None);
    }

    #[test]
    fn mods_that_are_off_or_missing_from_the_profile_are_not_enabled() {
        let state = VortexState::parse(STATE, "skyrimse").unwrap();
        assert!(!find(&state, "Disabled Mod").enabled);
        assert!(!find(&state, "Never Enabled").enabled);
    }

    #[test]
    fn a_mod_without_names_is_called_by_its_folder() {
        let state = VortexState::parse(STATE, "skyrimse").unwrap();
        let bare = find(&state, "Never Enabled");
        assert_eq!(bare.name, "Never Enabled");
        assert_eq!(bare.archive, None);
        assert_eq!(bare.chosen, Vec::new());
    }

    #[test]
    fn without_a_profile_every_installed_mod_counts_as_enabled() {
        let state = VortexState::parse(
            r#"{ "persistent": { "mods": { "skyrimse": { "Some Mod": {} } } } }"#,
            "skyrimse",
        )
        .unwrap();
        assert!(find(&state, "Some Mod").enabled);
    }

    #[test]
    fn a_game_vortex_does_not_manage_has_no_mods() {
        let state = VortexState::parse(STATE, "fallout4").unwrap();
        assert_eq!(state.mods, Vec::new());
    }

    #[test]
    fn downloads_lie_below_vortex_data_in_a_folder_per_game() {
        let state = VortexState::parse(STATE, "skyrimse").unwrap();
        let data = Path::new("appdata").join("Vortex");
        assert_eq!(
            state.downloads_dir(&data, "skyrimse"),
            data.join("downloads").join("skyrimse")
        );
        // The default is the same place.
        assert_eq!(
            VortexState::default().downloads_dir(&data, "skyrimse"),
            data.join("downloads").join("skyrimse")
        );
    }

    #[test]
    fn a_download_path_with_a_game_placeholder_is_used_as_it_is() {
        let state = VortexState {
            downloads_path: Some("downloads/{game}/archives".to_owned()),
            ..VortexState::default()
        };
        assert_eq!(
            state.downloads_dir(Path::new("unused"), "skyrimse"),
            Path::new("downloads").join("skyrimse").join("archives")
        );
    }

    #[test]
    fn names_are_keyed_by_staging_folder() {
        let state = VortexState::parse(STATE, "skyrimse").unwrap();
        assert_eq!(
            state.names().get("Patch Hub 1.0").map(String::as_str),
            Some("Patch Hub")
        );
    }

    #[test]
    fn the_links_copy_lies_in_a_folder_per_game() {
        let path = live_state_path(Path::new("bridge"), "skyrimse");
        assert!(path.ends_with(Path::new("bridge/state/skyrimse.json")));
    }

    #[test]
    fn the_backup_lies_in_vortex_temp_directory() {
        let path = state_backup_path(Path::new("Vortex"));
        assert!(path.ends_with(Path::new("temp/state_backups_full/hourly.json")));
    }

    #[test]
    fn broken_json_is_an_error() {
        assert!(VortexState::parse("{ nope", "skyrimse").is_err());
        // Valid JSON of the wrong shape is read as an empty state.
        assert_eq!(
            VortexState::parse("{}", "skyrimse").unwrap(),
            VortexState::default()
        );
    }

    #[test]
    fn the_tools_of_a_game_are_read_with_their_paths() {
        let json = r#"{ "settings": { "gameMode": { "discovered": {
          "skyrimse": { "path": "C:\\Games\\Made Up", "tools": {
            "zz": { "id": "zz", "name": "Made Up Editor (Patch)", "path": "C:\\Tools\\Editor.exe", "custom": true, "hidden": false },
            "aa": { "id": "aa", "name": "Made Up Idles", "path": "C:\\Tools\\Idles.exe", "custom": false },
            "nopath": { "id": "nopath", "name": "Never Found" },
            "noname": { "path": "C:\\Tools\\x.exe" }
          } },
          "othergame": { "tools": { "o": { "name": "Elsewhere", "path": "C:\\o.exe" } } }
        } } } }"#;
        let tools = VortexState::parse(json, "skyrimse").unwrap().tools.unwrap();
        let ids: Vec<&str> = tools.iter().map(|tool| tool.id.as_str()).collect();
        assert_eq!(ids, ["aa", "zz"]);
        assert!(tools.iter().any(|tool| tool.custom && !tool.hidden));
        assert_eq!(tools[0].path, r"C:\Tools\Idles.exe");
        // A game Vortex has no entry for, and a state without the part.
        assert_eq!(VortexState::parse(json, "fallout4").unwrap().tools, None);
        assert_eq!(VortexState::parse("{}", "skyrimse").unwrap().tools, None);
    }

    proptest! {
        #[test]
        fn never_panics_on_arbitrary_input(json in ".*", game in ".*") {
            let _ = VortexState::parse(&json, &game);
        }
    }
}
