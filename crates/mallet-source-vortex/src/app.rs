//! Vortex as a running program: whether it runs, where it is installed,
//! and restarting it when the user asks for that, so that it loads a new
//! version of the link.
//!
//! Vortex is asked to close the way the window's close button would; it
//! is never killed. If it does not close in time, nothing else happens.

use std::path::PathBuf;
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
/// installation: `game_id` is Vortex's identifier, such as `skyrimse`.
/// Mod Mallet ships no pictures of games; without Vortex there are none.
pub fn game_art(game_id: &str) -> Option<Vec<u8>> {
    let plain = !game_id.is_empty()
        && game_id
            .chars()
            .all(|character| character.is_ascii_lowercase() || character.is_ascii_digit());
    if !plain {
        return None;
    }
    let file = executable()?
        .parent()?
        .join("resources")
        .join("app.asar.unpacked")
        .join("bundledPlugins")
        .join(format!("game-{game_id}"))
        .join("gameart.jpg");
    std::fs::read(file).ok()
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
