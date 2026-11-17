//! Which archives the game loads, and in which order.
//!
//! The game loads archives from two sources:
//!
//! 1. Its own archives, such as `Skyrim - Meshes0.bsa`, which the game's
//!    settings list. They come first.
//! 2. For every active plugin, the archives named after it, in load order.
//!    Skyrim loads `<plugin>.bsa` and `<plugin> - Textures.bsa`; Fallout 4
//!    loads `<plugin> - Main.ba2`, `<plugin> - Textures.ba2` and the like;
//!    Oblivion, Fallout 3 and New Vegas load every archive whose name
//!    starts with the plugin's.
//!
//! When two archives contain the same file, the one loaded later wins. A
//! loose file wins over every archive.
//!
//! The game's own archives are recognised here by their name instead of by
//! reading the settings files. That is exact for an unmodified archive list,
//! which is the normal case.

use crate::game::{ArchiveNaming, Game};

/// The archives `game` loads, from lowest to highest precedence, in the
/// spelling they have on disk.
///
/// `archives_on_disk` are the names of the game's archives in the data
/// directory. `active_plugins` are the plugin file names in load order.
pub fn load_order(
    game: Game,
    archives_on_disk: &[String],
    active_plugins: &[String],
) -> Vec<String> {
    let prefixes = game.base_archive_prefixes();
    let last = game.base_archives_last();
    let mut base: Vec<&String> = archives_on_disk
        .iter()
        .filter(|name| {
            let lower = name.to_ascii_lowercase();
            prefixes.iter().any(|prefix| lower.starts_with(prefix))
                || last.contains(&lower.as_str())
        })
        .collect();
    base.sort_by_key(|name| {
        let lower = name.to_ascii_lowercase();
        (last.contains(&lower.as_str()), lower)
    });

    let mut order: Vec<String> = base.into_iter().cloned().collect();
    let extension = game.archive_extension();
    for plugin in active_plugins {
        let stem = plugin
            .rsplit_once('.')
            .map_or(plugin.as_str(), |(stem, _)| stem)
            .to_ascii_lowercase();
        let mut found: Vec<&String> = archives_on_disk
            .iter()
            .filter(|name| belongs_to(game.archive_naming(), &stem, extension, name))
            .filter(|name| !order.contains(name))
            .collect();
        // Within one plugin, the archive named exactly like it first.
        found.sort_by_key(|name| {
            let lower = name.to_ascii_lowercase();
            (!lower.starts_with(&format!("{stem}.")), lower)
        });
        order.extend(found.into_iter().cloned());
    }
    order
}

/// Whether the archive `name` belongs to the plugin whose name without
/// its extension is `stem`, in lower case.
fn belongs_to(naming: ArchiveNaming, stem: &str, extension: &str, name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    let Some(base) = lower
        .strip_suffix(extension)
        .and_then(|base| base.strip_suffix('.'))
    else {
        return false;
    };
    match naming {
        ArchiveNaming::Skyrim => base == stem || base == format!("{stem} - textures"),
        ArchiveNaming::Suffixed => base
            .strip_prefix(stem)
            .is_some_and(|rest| rest.starts_with(" - ")),
        ArchiveNaming::Prefixed => base.starts_with(stem),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(names: &[&str]) -> Vec<String> {
        names.iter().map(|&name| name.to_owned()).collect()
    }

    #[test]
    fn the_games_archives_come_first_with_the_patch_last() {
        let on_disk = names(&[
            "Skyrim - Patch.bsa",
            "Skyrim - Textures1.bsa",
            "Skyrim - Meshes0.bsa",
            "Grass.bsa",
        ]);
        let order = load_order(
            Game::SkyrimSe,
            &on_disk,
            &names(&["Skyrim.esm", "Grass.esp"]),
        );
        assert_eq!(
            order,
            [
                "Skyrim - Meshes0.bsa",
                "Skyrim - Textures1.bsa",
                "Skyrim - Patch.bsa",
                "Grass.bsa",
            ]
        );
    }

    #[test]
    fn plugin_archives_follow_the_load_order() {
        let on_disk = names(&["B.bsa", "A.bsa", "A - Textures.bsa"]);
        let order = load_order(Game::SkyrimSe, &on_disk, &names(&["B.esp", "A.esm"]));
        assert_eq!(order, ["B.bsa", "A.bsa", "A - Textures.bsa"]);
    }

    #[test]
    fn archives_are_matched_regardless_of_case() {
        let on_disk = names(&["grass - textures.BSA"]);
        let order = load_order(Game::SkyrimSe, &on_disk, &names(&["Grass.esp"]));
        assert_eq!(order, ["grass - textures.BSA"]);
    }

    #[test]
    fn archives_without_an_active_plugin_are_not_loaded() {
        let on_disk = names(&["Orphan.bsa", "Used.bsa"]);
        let order = load_order(Game::SkyrimSe, &on_disk, &names(&["Used.esp"]));
        assert_eq!(order, ["Used.bsa"]);
    }

    #[test]
    fn fallout_4_loads_every_ba2_archive_named_after_a_plugin() {
        let on_disk = names(&[
            "Fallout4 - Textures1.ba2",
            "Fallout4 - Meshes.ba2",
            "Town - Textures.ba2",
            "Town - Main.ba2",
            "Townhall - Main.ba2",
            "Town.bsa",
        ]);
        let order = load_order(
            Game::Fallout4,
            &on_disk,
            &names(&["Fallout4.esm", "Town.esp"]),
        );
        assert_eq!(
            order,
            [
                "Fallout4 - Meshes.ba2",
                "Fallout4 - Textures1.ba2",
                "Town - Main.ba2",
                "Town - Textures.ba2",
            ]
        );
    }

    #[test]
    fn the_older_games_load_every_archive_that_starts_with_a_plugins_name() {
        let on_disk = names(&[
            "Update.bsa",
            "Fallout - Meshes.bsa",
            "DeadMoney - Main.bsa",
            "DeadMoney - Sounds.bsa",
            "Other.bsa",
        ]);
        let order = load_order(
            Game::FalloutNv,
            &on_disk,
            &names(&["FalloutNV.esm", "DeadMoney.esm"]),
        );
        assert_eq!(
            order,
            [
                "Fallout - Meshes.bsa",
                "Update.bsa",
                "DeadMoney - Main.bsa",
                "DeadMoney - Sounds.bsa",
            ]
        );
    }

    #[test]
    fn plugin_names_with_several_dots_keep_their_stem() {
        let on_disk = names(&["My.Mod.v2.bsa"]);
        let order = load_order(Game::SkyrimSe, &on_disk, &names(&["My.Mod.v2.esp"]));
        assert_eq!(order, ["My.Mod.v2.bsa"]);
    }
}
