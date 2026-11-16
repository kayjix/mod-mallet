//! Building and starting the desktop app the way a user gets it.
//!
//! The app's executable carries the web frontend inside it. Building the
//! Rust crate alone would embed whatever frontend was built last, so this
//! command builds both, in order.

use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, ensure};

use crate::tools;

const DESKTOP_CRATE: &str = "mallet-desktop";

pub(crate) fn run(root: &Path, release: bool, start: bool) -> Result<()> {
    ensure!(
        tools::node_modules_installed(root),
        "the frontend's dependencies are missing: run `npm install` first"
    );

    let mut frontend = tools::npm(root);
    frontend.args(["run", "build", "--workspace", "apps/desktop"]);
    tools::run(frontend, "frontend build")?;

    let mut build = tools::cargo();
    build.current_dir(root).args(["build", "-p", DESKTOP_CRATE]);
    if release {
        build.arg("--release");
    }
    tools::run(build, "cargo build")
        .context("if the app is still running, close it: its file cannot be replaced meanwhile")?;

    let profile = if release { "release" } else { "debug" };
    let executable = root
        .join("target")
        .join(profile)
        .join(format!("{DESKTOP_CRATE}{}", std::env::consts::EXE_SUFFIX));
    println!("Built {}", executable.display());

    // The release build is one file with the frontend inside. A copy with
    // a recognisable name in `dist/` is what the user starts by hand.
    if release {
        let dist = root.join("dist");
        let copy = dist.join(format!("Mod Mallet{}", std::env::consts::EXE_SUFFIX));
        match std::fs::create_dir_all(&dist).and_then(|()| std::fs::copy(&executable, &copy)) {
            Ok(_) => println!("Copied to {}", copy.display()),
            // Most likely that copy is running. The build itself is fine.
            Err(error) => eprintln!("warning: {} not replaced: {error}", copy.display()),
        }
    }

    if start {
        // Started, not waited for: the app outlives this command. It gets
        // no share of this command's output, or whoever reads that output
        // would wait until the app is closed.
        launcher(&executable, root)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .with_context(|| format!("{} could not be started", executable.display()))?;
    }
    Ok(())
}

/// The command that starts the app as if the user had started it.
///
/// On Windows, a program started by a packaged app (a coding assistant's
/// desktop app, for example) shares that app's private copy of `AppData`:
/// the folders it creates there are invisible to Vortex. Windows' own
/// management service starts the app instead, outside any package.
#[cfg(windows)]
fn launcher(executable: &Path, root: &Path) -> Command {
    let mut command = Command::new("powershell");
    command
        .env("MALLET_APP", executable)
        .env("MALLET_ROOT", root)
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Invoke-CimMethod -ClassName Win32_Process -MethodName Create -Arguments \
             @{ CommandLine = '\"' + $env:MALLET_APP + '\"'; CurrentDirectory = $env:MALLET_ROOT } \
             | Out-Null",
        ]);
    command
}

#[cfg(not(windows))]
fn launcher(executable: &Path, root: &Path) -> Command {
    let mut command = Command::new(executable);
    command.current_dir(root);
    command
}
