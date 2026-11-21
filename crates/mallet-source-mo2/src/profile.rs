//! A profile of Mod Organizer: which mods are switched on, in which order,
//! and which plugins the game loads.
//!
//! Each profile is a folder below the instance's profiles folder:
//!
//! ```text
//! modlist.txt    the mods, highest priority first: +on, -off, *not managed
//! plugins.txt    the plugins, in the game's own format
//! loadorder.txt  every plugin, in load order
//! settings.ini   whether the profile keeps its own saves and game INIs
//! ```

use crate::ini::Ini;

/// The list of mods of a profile.
pub const MODLIST_FILE: &str = "modlist.txt";
/// The list of plugins of a profile, as the game reads it.
pub const PLUGINS_FILE: &str = "plugins.txt";
/// The load order of every plugin of a profile.
pub const LOAD_ORDER_FILE: &str = "loadorder.txt";
/// The settings of a profile.
pub const SETTINGS_FILE: &str = "settings.ini";

/// What a mod's folder name ends in if it is not a mod but a separator: a
/// heading the user put into the list of mods.
const SEPARATOR_SUFFIX: &str = "_separator";

/// How a line of `modlist.txt` marks its mod.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryState {
    /// `+`: the mod is switched on.
    Enabled,
    /// `-`: the mod is installed but switched off.
    Disabled,
    /// `*`: a part of the game's own data directory that Mod Organizer
    /// shows but does not manage, such as a DLC.
    Unmanaged,
}

/// One line of `modlist.txt`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModListEntry {
    /// The mod's folder name.
    pub name: String,
    /// How the line marks it.
    pub state: EntryState,
}

impl ModListEntry {
    /// Whether the entry is a separator, not a mod.
    pub fn is_separator(&self) -> bool {
        self.name.ends_with(SEPARATOR_SUFFIX)
    }

    /// The heading a separator shows: its name without the suffix.
    pub fn separator_title(&self) -> Option<&str> {
        self.name.strip_suffix(SEPARATOR_SUFFIX)
    }
}

/// The entries of `modlist.txt` in the order of the file, which is the
/// highest priority first. Comments and lines that mark nothing are left
/// out.
pub fn parse_modlist(text: &str) -> Vec<ModListEntry> {
    text.trim_start_matches('\u{feff}')
        .lines()
        .filter_map(|line| {
            let line = line.trim_end();
            let mut chars = line.chars();
            let state = match chars.next()? {
                '+' => EntryState::Enabled,
                '-' => EntryState::Disabled,
                '*' => EntryState::Unmanaged,
                _ => return None,
            };
            let name = chars.as_str().trim();
            (!name.is_empty()).then(|| ModListEntry {
                name: name.to_owned(),
                state,
            })
        })
        .collect()
}

/// One plugin of `plugins.txt`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginEntry {
    /// The plugin's file name.
    pub name: String,
    /// Whether the game loads it.
    pub active: bool,
}

/// The plugins of `plugins.txt` in the order of the file. The games since
/// Skyrim Special Edition list every plugin and mark the active ones with
/// `*`; the older ones list only the active plugins.
pub fn parse_plugins(text: &str) -> Vec<PluginEntry> {
    let lines: Vec<&str> = plain_lines(text).collect();
    let marked = lines.iter().any(|line| line.starts_with('*'));
    lines
        .into_iter()
        .filter_map(|line| {
            let (name, active) = match line.strip_prefix('*') {
                Some(name) => (name.trim(), true),
                None => (line, !marked),
            };
            (!name.is_empty()).then(|| PluginEntry {
                name: name.to_owned(),
                active,
            })
        })
        .collect()
}

/// The plugins of `loadorder.txt`, in load order.
pub fn parse_load_order(text: &str) -> Vec<String> {
    plain_lines(text).map(str::to_owned).collect()
}

/// The lines of a list that are neither blank nor comments.
fn plain_lines(text: &str) -> impl Iterator<Item = &str> {
    text.trim_start_matches('\u{feff}')
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
}

/// What a profile keeps for itself instead of sharing it with the game.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ProfileSettings {
    /// The profile keeps its own saved games, in its `saves` folder.
    pub local_saves: bool,
    /// The profile keeps its own copies of the game's INI files, in its
    /// folder.
    pub local_settings: bool,
}

impl ProfileSettings {
    /// Reads the text of a profile's `settings.ini`.
    pub fn parse(text: &str) -> Self {
        let ini = Ini::parse(text);
        Self {
            local_saves: ini.flag("General", "LocalSaves"),
            local_settings: ini.flag("General", "LocalSettings"),
        }
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    #[test]
    fn reads_the_mod_list_highest_priority_first() {
        let list = parse_modlist(
            "# This file was automatically generated by Mod Organizer.\r\n+Patch Hub\r\n-Old Textures\r\n+Textures_separator\r\n*DLC: Dawnguard\r\n\r\n+Base Mod\r\n",
        );
        let names: Vec<_> = list.iter().map(|entry| entry.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "Patch Hub",
                "Old Textures",
                "Textures_separator",
                "DLC: Dawnguard",
                "Base Mod"
            ]
        );
        let states: Vec<_> = list.iter().map(|entry| entry.state).collect();
        assert_eq!(
            states,
            [
                EntryState::Enabled,
                EntryState::Disabled,
                EntryState::Enabled,
                EntryState::Unmanaged,
                EntryState::Enabled
            ]
        );
        assert!(list.get(2).is_some_and(ModListEntry::is_separator));
        assert_eq!(
            list.get(2).and_then(ModListEntry::separator_title),
            Some("Textures")
        );
        assert!(!list.first().is_some_and(ModListEntry::is_separator));
    }

    #[test]
    fn plugins_marked_with_a_star_are_active() {
        let plugins = parse_plugins("# comment\n*Patch Hub.esp\nOld Town.esp\n*Base.esm\n");
        assert_eq!(
            plugins,
            [
                PluginEntry {
                    name: "Patch Hub.esp".to_owned(),
                    active: true
                },
                PluginEntry {
                    name: "Old Town.esp".to_owned(),
                    active: false
                },
                PluginEntry {
                    name: "Base.esm".to_owned(),
                    active: true
                },
            ]
        );
    }

    #[test]
    fn in_older_games_every_listed_plugin_is_active() {
        let plugins = parse_plugins("Base.esm\r\nPatch Hub.esp\r\n");
        assert!(plugins.iter().all(|plugin| plugin.active));
        assert_eq!(plugins.len(), 2);
    }

    #[test]
    fn reads_the_load_order() {
        assert_eq!(
            parse_load_order("\u{feff}# comment\r\nSkyrim.esm\r\n\r\nPatch Hub.esp\r\n"),
            ["Skyrim.esm", "Patch Hub.esp"]
        );
    }

    #[test]
    fn reads_what_the_profile_keeps_for_itself() {
        let settings = ProfileSettings::parse(
            "[General]\nLocalSaves=true\nLocalSettings=false\nAutomaticArchiveInvalidation=false\n",
        );
        assert_eq!(
            settings,
            ProfileSettings {
                local_saves: true,
                local_settings: false
            }
        );
        assert_eq!(ProfileSettings::parse(""), ProfileSettings::default());
    }

    proptest! {
        #[test]
        fn never_panics_on_arbitrary_input(text in ".*") {
            let _ = parse_modlist(&text);
            let _ = parse_plugins(&text);
            let _ = parse_load_order(&text);
            let _ = ProfileSettings::parse(&text);
        }
    }
}
