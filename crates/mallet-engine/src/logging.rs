//! The log file of the desktop app.
//!
//! The engine reports what it does through `tracing`. The command line
//! prints that on request; the desktop app has no console, so it writes it
//! into a file that a user, a supporter or a coding assistant can read
//! afterwards. One line per event, plain text, times in UTC.

use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// The name of the current log file.
pub const LOG_FILE: &str = "mod-mallet.log";

/// The name the previous log file gets once the current one has grown too
/// large. Only one such file is kept.
pub const PREVIOUS_LOG_FILE: &str = "mod-mallet.previous.log";

/// From this size on, the log file is set aside when the app starts.
const MAX_SIZE: u64 = 2 * 1024 * 1024;

/// The folder the desktop app writes its log into, in its standard place.
pub fn default_log_dir() -> Option<PathBuf> {
    Some(dirs::data_local_dir()?.join("mod-mallet").join("logs"))
}

/// Sends everything the engine reports from now on into the log file in
/// `dir`, and returns that file. A log file that has grown too large is
/// first set aside, replacing the one set aside before.
///
/// # Errors
///
/// If the folder cannot be created or the file not opened, or if logging
/// was already set up in this process.
pub fn start_logging(dir: &Path) -> std::io::Result<PathBuf> {
    fs::create_dir_all(dir)?;
    let path = dir.join(LOG_FILE);
    if fs::metadata(&path).is_ok_and(|metadata| metadata.len() > MAX_SIZE) {
        fs::rename(&path, dir.join(PREVIOUS_LOG_FILE))?;
    }
    let file = OpenOptions::new().create(true).append(true).open(&path)?;
    tracing_subscriber::fmt()
        .with_writer(Mutex::new(file))
        .with_ansi(false)
        .with_target(false)
        .with_max_level(tracing::Level::INFO)
        .try_init()
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        commit = option_env!("MALLET_COMMIT").unwrap_or("none"),
        log = %path.display(),
        "Mod Mallet started"
    );
    Ok(path)
}
