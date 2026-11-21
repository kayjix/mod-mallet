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
            })
        })
        .collect();
    found.sort();
    found.dedup();
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
                },
                SwitchedOffPlugin {
                    plugin: "gone.esm".to_owned(),
                    mod_ref: town(),
                    loaded: false,
                },
            ]
        );
    }
}
