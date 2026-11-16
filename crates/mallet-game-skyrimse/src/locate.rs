//! Finds the game and its files on this machine.
//!
//! Every function here returns `None` when it cannot tell. Callers then ask
//! the user instead of guessing.

use std::path::PathBuf;

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
