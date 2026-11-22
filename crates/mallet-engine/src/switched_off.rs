//! The plugins of mods the mod manager keeps switched off (ADR 0061).
//!
//! Switching a mod off in Vortex takes the files Vortex put into the game
//! out again when Vortex deploys. Until then, and for good where a copy was
//! in the data directory before (Vortex puts it back when it takes its own
//! away, and keeps such a plugin active), the game loads a mod the user
//! switched off. The engine names these plugins, so that the analyses can
//! say why the plugin is still there; and it names the plugins the game no
//! longer loads, so that the window can tell the decisions about them from
//! the others.

use std::path::Path;

use mallet_core::facts::PluginFacts;
use mallet_core::{GamePath, ModSource, SwitchedOffPlugin};
use mallet_game_bethesda::Game;

/// The file endings of plugins.
const PLUGIN_ENDINGS: [&str; 3] = [".esp", ".esm", ".esl"];

/// The plugins that the switched-off mods bring in the data directory,
/// each with the mod and whether the game loads it all the same. Plugins
/// another mod that is on deployed, and the game's own plugins, are left
/// out. Only reads.
pub(crate) fn plugins(
    game: Game,
    game_dir: &Path,
    active: &[PluginFacts],
    mods: &dyn ModSource,
) -> Vec<SwitchedOffPlugin> {
    let creation_club = creation_club(game, game_dir);
    let data = GamePath::new("data");
    let mut found: Vec<SwitchedOffPlugin> = mods
        .switched_off_files()
        .into_iter()
        .filter_map(|(path, mod_ref)| {
            let name = path
                .as_str()
                .strip_prefix(data.as_str())?
                .strip_prefix('/')?;
            let is_plugin =
                !name.contains('/') && PLUGIN_ENDINGS.iter().any(|ending| name.ends_with(ending));
            if !is_plugin
                || mods
                    .provider_of(&path)
                    .is_some_and(|provider| provider != mod_ref)
                || game.is_official(name)
                || creation_club
                    .iter()
                    .any(|own| own.eq_ignore_ascii_case(name))
            {
                return None;
            }
            let loaded = active
                .iter()
                .find(|plugin| plugin.name.eq_ignore_ascii_case(name));
            Some(SwitchedOffPlugin {
                plugin: loaded.map_or_else(|| name.to_owned(), |plugin| plugin.name.clone()),
                mod_ref,
                loaded: loaded.is_some(),
                left_behind: Vec::new(),
            })
        })
        .collect();
    found.sort();
    found.dedup();
    if !found.is_empty() {
        let data_files = data_files(&game_dir.join("Data"));
        for plugin in &mut found {
            plugin.left_behind = left_behind(&plugin.plugin, &data_files, mods);
        }
    }
    if found.iter().any(|plugin| plugin.loaded) {
        tracing::info!(
            plugins = ?found
                .iter()
                .filter(|plugin| plugin.loaded)
                .map(|plugin| plugin.plugin.as_str())
                .collect::<Vec<_>>(),
            "the game loads plugins of switched-off mods"
        );
    }
    found
}

/// Where a plugin or an archive named `name` lies directly in the game's
/// data directory, for the window to show it in the file explorer (ADR
/// 0073). `None` for any other name, for a name with a folder in it, and
/// for a file that is not there, so that nothing else can be shown this way.
pub fn data_file(options: &crate::ScanOptions, name: &str) -> Option<std::path::PathBuf> {
    let lower = name.to_lowercase();
    let ending = lower.rsplit_once('.').map(|(_, ending)| ending)?;
    let plain = !name.is_empty()
        && name.len() <= 260
        && !name.contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|'])
        && name != "."
        && name != "..";
    let known = PLUGIN_ENDINGS
        .iter()
        .any(|plugin| plugin.strip_prefix('.') == Some(ending))
        || ARCHIVE_ENDINGS.contains(&ending);
    if !plain || !known {
        return None;
    }
    let (game_dir, _) = options.located_game_dir()?;
    let file = game_dir.join("Data").join(name);
    file.is_file().then_some(file)
}

/// The names of the files directly in the data directory. Only reads;
/// nothing where the directory cannot be read.
fn data_files(data_dir: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(data_dir) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect()
}

/// The files among `data_files` that belong to `plugin` by name, the plugin
/// itself and the archives named after it, which no mod manager put there
/// (ADR 0073). In the spelling of the data directory, sorted.
fn left_behind(plugin: &str, data_files: &[String], mods: &dyn ModSource) -> Vec<String> {
    let plugin = plugin.to_lowercase();
    let stem = plugin
        .rsplit_once('.')
        .map_or(plugin.as_str(), |(stem, _)| stem);
    let data = GamePath::new("data");
    let mut found: Vec<String> = data_files
        .iter()
        .filter(|name| {
            let lower = name.to_lowercase();
            if lower == plugin {
                return true;
            }
            let Some((name_stem, ending)) = lower.rsplit_once('.') else {
                return false;
            };
            ARCHIVE_ENDINGS.contains(&ending)
                && (name_stem == stem
                    || name_stem
                        .strip_prefix(stem)
                        .is_some_and(|rest| rest.starts_with(" - ")))
        })
        .filter(|name| mods.provider_of(&data.join(name)).is_none())
        .cloned()
        .collect();
    found.sort_by_key(|name| name.to_lowercase());
    found
}

/// The file endings of the game's archives, in lower case, without the dot.
const ARCHIVE_ENDINGS: [&str; 2] = ["bsa", "ba2"];

/// The Creation Club plugins the game counts as its own.
fn creation_club(game: Game, game_dir: &Path) -> Vec<String> {
    let Some(list) = game.creation_club_list() else {
        return Vec::new();
    };
    let Ok(bytes) = std::fs::read(game_dir.join(list)) else {
        return Vec::new();
    };
    String::from_utf8_lossy(&bytes)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use mallet_core::ModRef;

    use super::*;

    struct Manager;

    fn town() -> ModRef {
        ModRef {
            source: "vortex".to_owned(),
            id: "Town 1".to_owned(),
            name: "Town".to_owned(),
        }
    }

    impl ModSource for Manager {
        fn provider_of(&self, path: &GamePath) -> Option<ModRef> {
            // Another mod that is on brings this one.
            (path.as_str() == "data/shared.esp").then(|| ModRef {
                source: "vortex".to_owned(),
                id: "Other 2".to_owned(),
                name: "Other".to_owned(),
            })
        }

        fn switched_off_files(&self) -> Vec<(GamePath, ModRef)> {
            [
                "data/Town.esp",
                "data/Gone.esm",
                "data/shared.esp",
                "data/Skyrim.esm",
                "data/meshes/town.esp",
                "data/textures/town.dds",
            ]
            .into_iter()
            .map(|path| (GamePath::new(path), town()))
            .collect()
        }
    }

    fn active(name: &str) -> PluginFacts {
        PluginFacts {
            name: name.to_owned(),
            path: GamePath::new("data").join(name),
            masters: Vec::new(),
            npcs: Vec::new(),
        }
    }

    #[test]
    fn plugins_of_switched_off_mods_are_named_with_whether_the_game_loads_them() {
        let found = plugins(
            Game::SkyrimSe,
            Path::new("Z:/no such game"),
            &[active("Town.esp"), active("Skyrim.esm")],
            &Manager,
        );
        assert_eq!(
            found,
            vec![
                SwitchedOffPlugin {
                    plugin: "Town.esp".to_owned(),
                    mod_ref: town(),
                    loaded: true,
                    left_behind: Vec::new(),
                },
                SwitchedOffPlugin {
                    plugin: "gone.esm".to_owned(),
                    mod_ref: town(),
                    loaded: false,
                    left_behind: Vec::new(),
                },
            ]
        );
    }

    #[test]
    fn the_files_no_mod_manager_put_there_are_named_as_left_behind() {
        let game = tempfile::tempdir().unwrap();
        let data = game.path().join("Data");
        std::fs::create_dir_all(data.join("Town.bsa folder")).unwrap();
        for name in [
            "Town.esp",
            "Town.bsa",
            "Town - Textures.bsa",
            "Townhall.bsa",
            "Town.ini",
            "shared.esp",
        ] {
            std::fs::write(data.join(name), b"").unwrap();
        }
        let found = plugins(Game::SkyrimSe, game.path(), &[active("Town.esp")], &Manager);
        let town = found
            .iter()
            .find(|plugin| plugin.plugin == "Town.esp")
            .unwrap();
        assert_eq!(
            town.left_behind,
            ["Town - Textures.bsa", "Town.bsa", "Town.esp"]
        );
        // Nothing of the other plugin is left in the folder.
        let gone = found
            .iter()
            .find(|plugin| plugin.plugin == "gone.esm")
            .unwrap();
        assert_eq!(gone.left_behind, Vec::<String>::new());
    }

    #[test]
    fn files_a_mod_manager_put_there_are_not_left_behind() {
        let files = ["shared.esp".to_owned(), "Shared.bsa".to_owned()];
        assert_eq!(left_behind("shared.esp", &files, &Manager), ["Shared.bsa"]);
    }

    #[test]
    fn only_plugins_and_archives_in_the_data_directory_are_shown() {
        let game = tempfile::tempdir().unwrap();
        let data = game.path().join("Data");
        std::fs::create_dir_all(data.join("meshes")).unwrap();
        for name in ["Town.esp", "Town.bsa", "Town.ini"] {
            std::fs::write(data.join(name), b"").unwrap();
        }
        std::fs::write(game.path().join("Outside.esp"), b"").unwrap();
        let options = crate::ScanOptions {
            game_dir: Some(game.path().to_path_buf()),
            ..crate::ScanOptions::default()
        };
        assert_eq!(data_file(&options, "Town.esp"), Some(data.join("Town.esp")));
        assert_eq!(data_file(&options, "Town.bsa"), Some(data.join("Town.bsa")));
        for name in [
            "Town.ini",
            "Gone.esp",
            "../Outside.esp",
            r"..\Outside.esp",
            "meshes",
            r"C:\Windows\notepad.esp",
            "",
        ] {
            assert_eq!(data_file(&options, name), None, "{name}");
        }
    }
}
