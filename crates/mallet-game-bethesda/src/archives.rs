//! Which archives the game loads, and in which order.
//!
//! The game loads archives from two sources:
//!
//! 1. Its own archives, named `Skyrim - *.bsa`, which the game's settings
//!    list. They come first.
//! 2. For every active plugin, the archives named after it:
//!    `<plugin>.bsa` and `<plugin> - Textures.bsa`. They follow in load
//!    order.
//!
//! When two archives contain the same file, the one loaded later wins. A
//! loose file wins over every archive.
//!
//! The game's own archives are recognised here by their name instead of by
//! reading the settings files. That is exact for an unmodified archive list,
//! which is the normal case.

/// Prefix of the archives that belong to the game itself.
const BASE_PREFIX: &str = "skyrim - ";

/// The game's patch archive, which is loaded after its other archives.
const BASE_PATCH: &str = "skyrim - patch.bsa";

/// The archives the game loads, from lowest to highest precedence, in the
/// spelling they have on disk.
///
/// `archives_on_disk` are the names of the `.bsa` files in the data
/// directory. `active_plugins` are the plugin file names in load order.
pub fn load_order(archives_on_disk: &[String], active_plugins: &[String]) -> Vec<String> {
    let find = |wanted: &str| {
        archives_on_disk
            .iter()
            .find(|name| name.eq_ignore_ascii_case(wanted))
    };

    let mut base: Vec<&String> = archives_on_disk
        .iter()
        .filter(|name| name.to_ascii_lowercase().starts_with(BASE_PREFIX))
        .collect();
    base.sort_by_key(|name| {
        let lower = name.to_ascii_lowercase();
        (lower == BASE_PATCH, lower)
    });

    let mut order: Vec<String> = base.into_iter().cloned().collect();
    for plugin in active_plugins {
        let stem = plugin
            .rsplit_once('.')
            .map_or(plugin.as_str(), |(stem, _)| stem);
        for candidate in [format!("{stem}.bsa"), format!("{stem} - Textures.bsa")] {
            if let Some(found) = find(&candidate)
                && !order.contains(found)
            {
                order.push(found.clone());
            }
        }
    }
    order
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
        let order = load_order(&on_disk, &names(&["Skyrim.esm", "Grass.esp"]));
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
        let order = load_order(&on_disk, &names(&["B.esp", "A.esm"]));
        assert_eq!(order, ["B.bsa", "A.bsa", "A - Textures.bsa"]);
    }

    #[test]
    fn archives_are_matched_regardless_of_case() {
        let on_disk = names(&["grass - textures.BSA"]);
        let order = load_order(&on_disk, &names(&["Grass.esp"]));
        assert_eq!(order, ["grass - textures.BSA"]);
    }

    #[test]
    fn archives_without_an_active_plugin_are_not_loaded() {
        let on_disk = names(&["Orphan.bsa", "Used.bsa"]);
        let order = load_order(&on_disk, &names(&["Used.esp"]));
        assert_eq!(order, ["Used.bsa"]);
    }

    #[test]
    fn plugin_names_with_several_dots_keep_their_stem() {
        let on_disk = names(&["My.Mod.v2.bsa"]);
        let order = load_order(&on_disk, &names(&["My.Mod.v2.esp"]));
        assert_eq!(order, ["My.Mod.v2.bsa"]);
    }
}
