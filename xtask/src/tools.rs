//! Running the external tools the automation relies on.

use std::ffi::OsString;
use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{Context, Result, ensure};

/// The cargo that started this xtask, so nested calls use the same
/// toolchain.
pub(crate) fn cargo() -> Command {
    Command::new(std::env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo")))
}

/// A command from the project's `node_modules`, started through `npx`.
///
/// `--no-install` makes a missing tool an error instead of a download.
pub(crate) fn npx(root: &Path, tool: &str) -> Command {
    let mut command = node_launcher("npx");
    command.current_dir(root).args(["--no-install", tool]);
    command
}

/// `npm` in the workspace root.
pub(crate) fn npm(root: &Path) -> Command {
    let mut command = node_launcher("npm");
    command.current_dir(root);
    command
}

/// On Windows `npm` and `npx` are batch files, which cannot be started
/// directly as a process.
fn node_launcher(program: &str) -> Command {
    if cfg!(windows) {
        let mut command = Command::new("cmd");
        command.args(["/C", program]);
        command
    } else {
        Command::new(program)
    }
}

/// Whether the project's Node dependencies have been installed.
pub(crate) fn node_modules_installed(root: &Path) -> bool {
    root.join("node_modules").is_dir()
}

/// Whether `program --version` runs successfully.
pub(crate) fn is_installed(mut command: Command) -> bool {
    command
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

/// Runs `command` with its output going to the terminal and fails if it
/// does.
pub(crate) fn run(mut command: Command, what: &str) -> Result<()> {
    let status = command
        .status()
        .with_context(|| format!("{what}: could not be started"))?;
    ensure!(status.success(), "{what} failed");
    Ok(())
}

/// Runs `command` and returns what it printed.
pub(crate) fn capture(mut command: Command, what: &str) -> Result<String> {
    let output = command
        .stderr(Stdio::inherit())
        .output()
        .with_context(|| format!("{what}: could not be started"))?;
    ensure!(output.status.success(), "{what} failed");
    String::from_utf8(output.stdout).with_context(|| format!("{what}: printed invalid UTF-8"))
}

/// Builds the documentation site with mdBook.
pub(crate) fn build_docs(root: &Path, open: bool) -> Result<()> {
    // The diagrams need two script files next to book.toml. They are large
    // and derived, so they are installed here instead of being committed.
    let mut mermaid = Command::new("mdbook-mermaid");
    mermaid
        .arg("install")
        .arg(root.join("docs"))
        .stdout(Stdio::null());
    run(mermaid, "mdbook-mermaid install")
        .context("the diagrams need mdbook-mermaid: cargo install --locked mdbook-mermaid")?;

    let mut command = Command::new("mdbook");
    command.arg("build").arg(root.join("docs"));
    if open {
        command.arg("--open");
    }
    run(command, "mdbook build").context(
        "building the documentation needs mdBook: cargo install --locked mdbook mdbook-mermaid",
    )
}
