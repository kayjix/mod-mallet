//! Entry point of the Mod Mallet desktop app.

// Without this, the app opens a console window next to itself on Windows.
// Only the development build that loads its frontend from the development
// server keeps the console, for its log output.
#![cfg_attr(
    any(not(debug_assertions), feature = "custom-protocol"),
    windows_subsystem = "windows"
)]

use std::process::ExitCode;

fn main() -> ExitCode {
    match mallet_desktop_lib::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}
