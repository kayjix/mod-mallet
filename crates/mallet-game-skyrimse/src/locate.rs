//! Finds the game and its files on this machine.
//!
//! Every function here returns `None` when it cannot tell. Callers then ask
//! the user instead of guessing.

use std::path::{Path, PathBuf};

use crate::objects::OFFICIAL_PLUGINS;

/// Where the game keeps its settings, saves and logs, below the user's
/// documents folder.
const MY_GAMES_DIR: [&str; 2] = ["My Games", "Skyrim Special Edition"];

/// The directory the game is installed in.
///
/// On Windows this is read from the registry entry the game's launcher
/// writes. On other systems it is unknown.
pub fn game_dir() -> Option<PathBuf> {
    registry_game_dir()
}

/// The log SKSE wrote the last time the game started.
///
/// The file is not guaranteed to exist: SKSE may not be installed, or the
/// game may never have been started through it.
pub fn extender_log() -> Option<PathBuf> {
    let documents = dirs::document_dir()?;
    let [my_games, game] = MY_GAMES_DIR;
    Some(
        documents
            .join(my_games)
            .join(game)
            .join("SKSE")
            .join("skse64.log"),
    )
}

#[cfg(windows)]
fn registry_game_dir() -> Option<PathBuf> {
    use winreg::RegKey;
    use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY};

    let key = RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey_with_flags(
            r"SOFTWARE\Bethesda Softworks\Skyrim Special Edition",
            KEY_READ | KEY_WOW64_32KEY,
        )
        .ok()?;
    let path: String = key.get_value("installed path").ok()?;
    let path = path.trim();
    if path.is_empty() {
        None
    } else {
        Some(PathBuf::from(path))
    }
}

#[cfg(not(windows))]
fn registry_game_dir() -> Option<PathBuf> {
    None
}

/// The plugins that come with the game, in lower case: its own, and the
/// Creation Club content the game lists in `Skyrim.ccc`. The game's
/// updates replace them, so whatever is changed in them does not last.
///
/// Only reads; a missing or unreadable list counts as empty.
pub fn shipped_plugins(game_dir: &Path) -> Vec<String> {
    let mut plugins: Vec<String> = OFFICIAL_PLUGINS
        .iter()
        .map(|plugin| (*plugin).to_owned())
        .collect();
    if let Ok(bytes) = std::fs::read(game_dir.join("Skyrim.ccc")) {
        plugins.extend(
            String::from_utf8_lossy(&bytes)
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(str::to_lowercase),
        );
    }
    plugins
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_games_plugins_and_its_creation_club_list_are_shipped() {
        let game = tempfile::tempdir().unwrap();
        std::fs::write(
            game.path().join("Skyrim.ccc"),
            "ccBGSSSE001-Fish.esm\r\n\r\nccQDRSSE001-SurvivalMode.esl\r\n",
        )
        .unwrap();

        let plugins = shipped_plugins(game.path());

        assert!(plugins.contains(&"dawnguard.esm".to_owned()));
        assert!(plugins.contains(&"ccbgssse001-fish.esm".to_owned()));
        assert!(plugins.contains(&"ccqdrsse001-survivalmode.esl".to_owned()));
        assert_eq!(plugins.len(), OFFICIAL_PLUGINS.len() + 2);
    }

    #[test]
    fn without_a_creation_club_list_only_the_games_own_plugins_are_shipped() {
        let game = tempfile::tempdir().unwrap();
        assert_eq!(shipped_plugins(game.path()).len(), OFFICIAL_PLUGINS.len());
    }
}
