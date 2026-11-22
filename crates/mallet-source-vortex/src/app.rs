//! Vortex as a running program: whether it runs, where it is installed,
//! and restarting it when the user asks for that, so that it loads a new
//! version of the link.
//!
//! Vortex is asked to close the way the window's close button would; it
//! is never killed. If it does not close in time, nothing else happens.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// The name of Vortex's executable.
const EXECUTABLE: &str = "Vortex.exe";

/// What went wrong while restarting Vortex.
#[derive(Debug, thiserror::Error)]
pub enum RestartError {
    /// Vortex's executable was not found.
    #[error("Vortex's program file was not found")]
    NotInstalled,
    /// Vortex was asked to close but is still running, for example
    /// because it is in the middle of something or asks a question.
    #[error("Vortex did not close within {} seconds", .0.as_secs())]
    StillRunning(Duration),
    /// A program needed for this could not be started.
    #[error("{0}")]
    Io(#[from] std::io::Error),
}

/// Where Vortex's executable is: as its installer recorded it, or in its
/// standard place.
pub fn executable() -> Option<PathBuf> {
    registered_executable()
        .or_else(|| {
            std::env::var_os("ProgramFiles")
                .map(|dir| PathBuf::from(dir).join("Vortex").join(EXECUTABLE))
        })
        .filter(|path| path.is_file())
}

/// The picture Vortex shows for a game, read from Vortex's own
/// installation, or else from the game's extension that Vortex downloaded
/// into the `plugins` folder of `vortex_data_dir` (`%APPDATA%Vortex`):
/// `game_id` is Vortex's identifier, such as `skyrimse`. Mod Mallet ships
/// no pictures of games; without Vortex there are none. Only reads.
pub fn game_art(game_id: &str, vortex_data_dir: Option<&Path>) -> Option<Vec<u8>> {
    let plain = !game_id.is_empty()
        && game_id
            .chars()
            .all(|character| character.is_ascii_lowercase() || character.is_ascii_digit());
    if !plain {
        return None;
    }
    let bundled = executable().and_then(|program| {
        let file = program
            .parent()?
            .join("resources")
            .join("app.asar.unpacked")
            .join("bundledPlugins")
            .join(format!("game-{game_id}"))
            .join("gameart.jpg");
        std::fs::read(file).ok()
    });
    bundled.or_else(|| downloaded_game_art(&vortex_data_dir?.join("plugins"), game_id))
}

/// Extension scripts larger than this are not read: a game's extension is
/// a few dozen kilobytes.
const MAX_EXTENSION_SCRIPT: u64 = 1024 * 1024;

/// The picture of the downloaded extension in `plugins` that adds the game
/// `game_id` to Vortex: the one whose `index.js` names that identifier in
/// quotes and that has a `gameart.jpg` beside it.
fn downloaded_game_art(plugins: &Path, game_id: &str) -> Option<Vec<u8>> {
    let quoted = [format!("\"{game_id}\""), format!("'{game_id}'")];
    let mut folders: Vec<PathBuf> = std::fs::read_dir(plugins)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.join("gameart.jpg").is_file())
        .collect();
    folders.sort();
    folders.into_iter().find_map(|folder| {
        let script = folder.join("index.js");
        let small = std::fs::metadata(&script)
            .is_ok_and(|meta| meta.is_file() && meta.len() <= MAX_EXTENSION_SCRIPT);
        let text = small.then(|| std::fs::read(&script).ok()).flatten()?;
        let text = String::from_utf8_lossy(&text);
        quoted
            .iter()
            .any(|id| text.contains(id.as_str()))
            .then(|| std::fs::read(folder.join("gameart.jpg")).ok())
            .flatten()
    })
}

/// Whether Vortex runs right now.
pub fn is_running() -> bool {
    quiet(Command::new("tasklist").args([
        "/FI",
        &format!("IMAGENAME eq {EXECUTABLE}"),
        "/NH",
        "/FO",
        "CSV",
    ]))
    .output()
    .is_ok_and(|output| {
        String::from_utf8_lossy(&output.stdout)
            .to_ascii_lowercase()
            .contains(&format!("\"{}\"", EXECUTABLE.to_ascii_lowercase()))
    })
}

/// Closes Vortex, waits up to `timeout` for it to be gone, and starts it
/// again. Only to be called because the user asked for it.
///
/// # Errors
///
/// If Vortex is not installed, does not close in time, or cannot be
/// started.
pub fn restart(timeout: Duration) -> Result<(), RestartError> {
    let program = executable().ok_or(RestartError::NotInstalled)?;
    if is_running() {
        // Without /F: Vortex receives the same request to close as from
        // its window, and saves its state as usual.
        quiet(Command::new("taskkill").args(["/IM", EXECUTABLE])).status()?;
        let started = Instant::now();
        while is_running() {
            if started.elapsed() > timeout {
                return Err(RestartError::StillRunning(timeout));
            }
            std::thread::sleep(Duration::from_millis(500));
        }
    }
    Command::new(&program)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    Ok(())
}

/// A command that opens no console window and prints nothing.
fn quiet(command: &mut Command) -> &mut Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        /// Windows' `CREATE_NO_WINDOW`.
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command.stdin(Stdio::null()).stderr(Stdio::null())
}

#[cfg(windows)]
fn registered_executable() -> Option<PathBuf> {
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ};

    const UNINSTALL: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall";
    for root in [HKEY_LOCAL_MACHINE, HKEY_CURRENT_USER] {
        let Ok(uninstall) = RegKey::predef(root).open_subkey_with_flags(UNINSTALL, KEY_READ) else {
            continue;
        };
        for name in uninstall.enum_keys().filter_map(Result::ok) {
            let Ok(key) = uninstall.open_subkey_with_flags(&name, KEY_READ) else {
                continue;
            };
            let display: String = key.get_value("DisplayName").unwrap_or_default();
            if display != "Vortex" {
                continue;
            }
            // "C:\Program Files\Vortex\Vortex.exe,0"
            let icon: String = key.get_value("DisplayIcon").unwrap_or_default();
            let path = icon.split(',').next().unwrap_or_default().trim_matches('"');
            if !path.is_empty() {
                return Some(PathBuf::from(path));
            }
        }
    }
    None
}

#[cfg(not(windows))]
fn registered_executable() -> Option<PathBuf> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Writes a downloaded extension named `folder` whose script says
    /// `script`, with a picture if `picture` is given.
    fn extension(plugins: &Path, folder: &str, script: &str, picture: Option<&[u8]>) {
        let dir = plugins.join(folder);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("index.js"), script).unwrap();
        if let Some(picture) = picture {
            std::fs::write(dir.join("gameart.jpg"), picture).unwrap();
        }
    }

    #[test]
    fn the_picture_of_a_downloaded_extension_is_found_by_its_game() {
        let plugins = tempfile::tempdir().unwrap();
        let plugins = plugins.path();
        extension(
            plugins,
            "Game Other",
            "const GAME_ID = 'other';",
            Some(b"other"),
        );
        extension(
            plugins,
            "Game Enderal SE",
            "const GAME_ID = \"enderalspecialedition\";",
            Some(b"enderal"),
        );
        extension(plugins, "No Picture", "id: 'nopicture'", None);
        assert_eq!(
            downloaded_game_art(plugins, "enderalspecialedition"),
            Some(b"enderal".to_vec())
        );
        assert_eq!(
            downloaded_game_art(plugins, "other"),
            Some(b"other".to_vec())
        );
        // A name merely contained in another is not the game.
        assert_eq!(downloaded_game_art(plugins, "enderal"), None);
        assert_eq!(downloaded_game_art(plugins, "nopicture"), None);
        assert_eq!(downloaded_game_art(&plugins.join("missing"), "other"), None);
    }

    #[test]
    fn identifiers_that_are_not_plain_are_refused() {
        assert_eq!(game_art("../skyrimse", None), None);
        assert_eq!(game_art("", None), None);
    }
}
