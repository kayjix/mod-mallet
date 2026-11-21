//! Finds the games and their files on this machine.
//!
//! A game is looked for where its launchers say they put it: the entry
//! Bethesda's launcher writes into the registry, Steam's game libraries,
//! and the entry GOG Galaxy writes. A directory counts only if the game's
//! program is in it, so an entry left behind by an uninstalled game is not
//! taken for the game.
//!
//! Every function here returns `None` when it cannot tell. Callers then ask
//! the user instead of guessing.

use std::path::{Path, PathBuf};

use mallet_core::LocationOrigin;

use crate::game::{Game, LogPlace};

/// The directory `game` is installed in, and how it was found.
pub fn game_dir(game: Game) -> Option<(PathBuf, LocationOrigin)> {
    let is_game = |dir: &Path| dir.join(game.executable()).is_file();
    if let Some(dir) = registry::bethesda(game).filter(|dir| is_game(dir)) {
        return Some((dir, LocationOrigin::Registry));
    }
    if let Some(dir) = steam_libraries()
        .into_iter()
        .flat_map(|library| {
            game.steam_folders()
                .iter()
                .map(move |folder| library.join("steamapps").join("common").join(folder))
        })
        .find(|dir| is_game(dir))
    {
        return Some((dir, LocationOrigin::Steam));
    }
    game.gog_ids()
        .iter()
        .filter_map(|id| registry::gog(id))
        .find(|dir| is_game(dir))
        .map(|dir| (dir, LocationOrigin::Gog))
}

/// The games that are installed on this machine, with their directories
/// and how each was found.
pub fn installed_games() -> Vec<(Game, PathBuf, LocationOrigin)> {
    Game::ALL
        .into_iter()
        .filter_map(|game| game_dir(game).map(|(dir, origin)| (game, dir, origin)))
        .collect()
}

/// The log the script extender of `game`, installed in `game_dir`, wrote
/// the last time the game started.
///
/// The file is not guaranteed to exist: the extender may not be installed,
/// or the game may never have been started through it.
pub fn extender_log(game: Game, game_dir: Option<&Path>) -> Option<PathBuf> {
    match game.script_extender().log? {
        LogPlace::MyGames { folder, file } => Some(
            dirs::document_dir()?
                .join("My Games")
                .join(game.my_games_folder())
                .join(folder)
                .join(file),
        ),
        LogPlace::GameDir { file } => game_dir.map(|dir| dir.join(file)),
    }
}

/// The folder whose `Data` folder holds the plugins and archives of `game`
/// installed in `dir`: `dir` itself for every game but Oblivion
/// Remastered, whose old game data lies four folders further down
/// (ADR 0065). A `dir` that already is that folder, or that does not have
/// it, is taken as it is, so that the user may name either.
///
/// Every check of plugins, archives and loose files reads below this
/// folder, so the paths the game sees start with `data/` in every game.
pub fn data_root(game: Game, dir: &Path) -> PathBuf {
    let below = game.data_root();
    if below.is_empty() {
        return dir.to_path_buf();
    }
    let root = below
        .iter()
        .fold(dir.to_path_buf(), |path, folder| path.join(folder));
    if root.is_dir() {
        root
    } else {
        dir.to_path_buf()
    }
}

/// The directory `game` is installed in, from the folder [`data_root`]
/// gives: `root` itself for every game but Oblivion Remastered, where it
/// lies four folders above. A `root` that does not end in those folders is
/// taken as the installation itself. Only compares names.
pub fn install_dir(game: Game, root: &Path) -> PathBuf {
    let below = game.data_root();
    let mut install = root;
    for folder in below.iter().rev() {
        let matches = install
            .file_name()
            .is_some_and(|name| name.eq_ignore_ascii_case(folder));
        match install.parent() {
            Some(parent) if matches => install = parent,
            _ => return root.to_path_buf(),
        }
    }
    install.to_path_buf()
}

/// The plugins that come with `game`, in lower case: its own, and the
/// Creation Club content the game lists in its `.ccc` file. The game's
/// updates replace them, so whatever is changed in them does not last.
///
/// Only reads; a missing or unreadable list counts as empty.
pub fn shipped_plugins(game: Game, game_dir: &Path) -> Vec<String> {
    let mut plugins: Vec<String> = game
        .official_plugins()
        .iter()
        .map(|plugin| (*plugin).to_owned())
        .collect();
    if let Some(list) = game.creation_club_list()
        && let Ok(bytes) = std::fs::read(game_dir.join(list))
    {
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

/// The folders of Steam's game libraries on this machine: Steam's own
/// folder and those listed in its `libraryfolders.vdf`.
pub fn steam_libraries() -> Vec<PathBuf> {
    let Some(steam) = registry::steam() else {
        return Vec::new();
    };
    let mut libraries = vec![steam.clone()];
    if let Ok(bytes) = std::fs::read(steam.join("steamapps").join("libraryfolders.vdf")) {
        for path in library_paths(&String::from_utf8_lossy(&bytes)) {
            if !libraries.contains(&path) {
                libraries.push(path);
            }
        }
    }
    libraries
}

/// The library folders named in the text of a `libraryfolders.vdf`.
///
/// The file is Valve's key-value format; each library is a block with a
/// line `"path"  "D:\\SteamLibrary"`. Only those lines are read. Anything
/// else in the file is skipped, whatever it is.
pub fn library_paths(text: &str) -> Vec<PathBuf> {
    text.lines()
        .filter_map(|line| {
            let rest = line.trim().strip_prefix("\"path\"")?.trim();
            let value = rest.strip_prefix('"')?.strip_suffix('"')?;
            let value = value.replace("\\\\", "\\");
            (!value.is_empty()).then(|| PathBuf::from(value))
        })
        .collect()
}

/// Reading the entries launchers write into the Windows registry.
#[cfg(windows)]
mod registry {
    use std::path::PathBuf;

    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY};

    use crate::game::Game;

    /// A text value of a key, as a path; `None` if it is missing or empty.
    fn value(root: winreg::HKEY, key: &str, name: &str) -> Option<PathBuf> {
        let key = RegKey::predef(root)
            .open_subkey_with_flags(key, KEY_READ | KEY_WOW64_32KEY)
            .ok()?;
        let path: String = key.get_value(name).ok()?;
        let path = path.trim();
        (!path.is_empty()).then(|| PathBuf::from(path))
    }

    /// Where Bethesda's launcher, or Steam on its behalf, says the game is.
    pub(super) fn bethesda(game: Game) -> Option<PathBuf> {
        value(
            HKEY_LOCAL_MACHINE,
            &format!(r"SOFTWARE\Bethesda Softworks\{}", game.registry_key()),
            "installed path",
        )
    }

    /// Where GOG Galaxy installed the game with GOG's identifier `id`.
    pub(super) fn gog(id: &str) -> Option<PathBuf> {
        value(
            HKEY_LOCAL_MACHINE,
            &format!(r"SOFTWARE\GOG.com\Games\{id}"),
            "path",
        )
    }

    /// Steam's own folder.
    pub(super) fn steam() -> Option<PathBuf> {
        value(HKEY_CURRENT_USER, r"Software\Valve\Steam", "SteamPath")
            .or_else(|| value(HKEY_LOCAL_MACHINE, r"SOFTWARE\Valve\Steam", "InstallPath"))
    }
}

/// Other systems have no registry; nothing is found there.
#[cfg(not(windows))]
mod registry {
    use std::path::PathBuf;

    use crate::game::Game;

    pub(super) fn bethesda(_game: Game) -> Option<PathBuf> {
        None
    }

    pub(super) fn gog(_id: &str) -> Option<PathBuf> {
        None
    }

    pub(super) fn steam() -> Option<PathBuf> {
        None
    }
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

        let plugins = shipped_plugins(Game::SkyrimSe, game.path());

        assert!(plugins.contains(&"dawnguard.esm".to_owned()));
        assert!(plugins.contains(&"ccbgssse001-fish.esm".to_owned()));
        assert!(plugins.contains(&"ccqdrsse001-survivalmode.esl".to_owned()));
        assert_eq!(plugins.len(), Game::SkyrimSe.official_plugins().len() + 2);
    }

    #[test]
    fn without_a_creation_club_list_only_the_games_own_plugins_are_shipped() {
        let game = tempfile::tempdir().unwrap();
        assert_eq!(
            shipped_plugins(Game::SkyrimSe, game.path()).len(),
            Game::SkyrimSe.official_plugins().len()
        );
    }

    #[test]
    fn games_without_creation_club_ignore_a_stray_list() {
        let game = tempfile::tempdir().unwrap();
        std::fs::write(game.path().join("Skyrim.ccc"), "ccBGSSSE001-Fish.esm\r\n").unwrap();
        assert_eq!(
            shipped_plugins(Game::FalloutNv, game.path()),
            Game::FalloutNv.official_plugins()
        );
    }

    #[test]
    fn extenders_of_the_older_games_log_into_the_game_directory() {
        let dir = Path::new("game");
        assert_eq!(
            extender_log(Game::FalloutNv, Some(dir)),
            Some(dir.join("nvse.log"))
        );
        assert_eq!(extender_log(Game::Oblivion, None), None);
    }

    #[test]
    fn oblivion_remastered_keeps_its_data_four_folders_down() {
        let install = tempfile::tempdir().unwrap();
        let root = install
            .path()
            .join("OblivionRemastered/Content/Dev/ObvData");
        // Before the folder exists, the directory is taken as it is.
        assert_eq!(
            data_root(Game::OblivionRemastered, install.path()),
            install.path()
        );
        std::fs::create_dir_all(root.join("Data")).unwrap();

        assert_eq!(data_root(Game::OblivionRemastered, install.path()), root);
        // Naming the folder itself works as well.
        assert_eq!(data_root(Game::OblivionRemastered, &root), root);
        assert_eq!(install_dir(Game::OblivionRemastered, &root), install.path());
        let lower = install
            .path()
            .join("oblivionremastered/content/dev/obvdata");
        assert_eq!(
            install_dir(Game::OblivionRemastered, &lower),
            install.path()
        );
        assert_eq!(
            install_dir(Game::OblivionRemastered, install.path()),
            install.path()
        );
        // The other games read below their installation.
        assert_eq!(data_root(Game::Oblivion, install.path()), install.path());
        assert_eq!(install_dir(Game::Oblivion, &root), root);
        assert_eq!(extender_log(Game::OblivionRemastered, Some(&root)), None);
    }

    #[test]
    fn library_paths_are_read_from_the_library_list() {
        let text = r#""libraryfolders"
{
	"0"
	{
		"path"		"C:\\Program Files (x86)\\Steam"
		"label"		""
		"apps"
		{
			"489830"		"14773925436"
		}
	}
	"1"
	{
		"path"		"D:\\SteamLibrary"
	}
	"2"
	{
		"path"		""
	}
}
"#;
        assert_eq!(
            library_paths(text),
            [
                PathBuf::from(r"C:\Program Files (x86)\Steam"),
                PathBuf::from(r"D:\SteamLibrary")
            ]
        );
    }

    #[test]
    fn a_game_directory_without_the_games_program_is_not_the_game() {
        // Nothing on a test machine is installed under these names; the
        // check only must not mistake an empty answer for a game.
        for game in Game::ALL {
            if let Some((dir, _)) = game_dir(game) {
                assert!(dir.join(game.executable()).is_file());
            }
        }
    }

    proptest::proptest! {
        #[test]
        fn library_lists_never_panic(text in ".*") {
            let _ = library_paths(&text);
        }
    }
}
