//! The app's one version (ADR 0057).
//!
//! The version lives in `[workspace.package]` of the workspace's
//! `Cargo.toml`. The npm packages repeat it because npm wants one, and the
//! change log has a section for it. This module checks that they agree and
//! says which commit a build was made from.

use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail, ensure};

/// The files besides `Cargo.toml` that name the version.
const PACKAGE_FILES: [&str; 2] = ["package.json", "apps/desktop/package.json"];

/// Fails unless every place that names the app's version names the same.
pub(crate) fn check(root: &Path) -> Result<()> {
    let version = workspace_version(root)?;

    for file in PACKAGE_FILES {
        let path = root.join(file);
        let text =
            std::fs::read_to_string(&path).with_context(|| format!("{file} could not be read"))?;
        let json: serde_json::Value =
            serde_json::from_str(&text).with_context(|| format!("{file} is not JSON"))?;
        let named = json.get("version").and_then(serde_json::Value::as_str);
        ensure!(
            named == Some(version.as_str()),
            "{file} names version {}, Cargo.toml {version}: change both together",
            named.unwrap_or("none")
        );
    }

    // Tauri takes the version from the crate when its config names none,
    // which leaves one place fewer to forget.
    let tauri = "apps/desktop/src-tauri/tauri.conf.json";
    let text = std::fs::read_to_string(root.join(tauri))
        .with_context(|| format!("{tauri} could not be read"))?;
    let json: serde_json::Value =
        serde_json::from_str(&text).with_context(|| format!("{tauri} is not JSON"))?;
    ensure!(
        json.get("version").is_none(),
        "{tauri} names a version: remove it, the app takes the one of Cargo.toml"
    );

    let changelog = std::fs::read_to_string(root.join("CHANGELOG.md"))
        .context("CHANGELOG.md could not be read")?;
    let heading = format!("## [{version}]");
    ensure!(
        changelog.lines().any(|line| line.starts_with(&heading)),
        "CHANGELOG.md has no section `{heading}`: say what changed in this version"
    );
    Ok(())
}

/// The version in `[workspace.package]` of the workspace's `Cargo.toml`.
fn workspace_version(root: &Path) -> Result<String> {
    let text =
        std::fs::read_to_string(root.join("Cargo.toml")).context("Cargo.toml could not be read")?;
    let mut in_section = false;
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            in_section = line == "[workspace.package]";
            continue;
        }
        if in_section
            && let Some(value) = line
                .strip_prefix("version")
                .map(str::trim_start)
                .and_then(|rest| rest.strip_prefix('='))
        {
            return Ok(value.trim().trim_matches('"').to_owned());
        }
    }
    bail!("Cargo.toml has no version in [workspace.package]")
}

/// Which commit a build is made from: its short hash, with `-modified` when
/// the working tree has changes that are not committed, and the day it was
/// committed. `None` outside a git checkout.
pub(crate) fn build_stamp(root: &Path) -> Option<(String, String)> {
    let git = |args: &[&str]| -> Option<String> {
        let output = Command::new("git")
            .current_dir(root)
            .args(args)
            .output()
            .ok()?;
        output
            .status
            .success()
            .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
    };
    let hash = git(&["rev-parse", "--short", "HEAD"])?;
    let date = git(&["show", "-s", "--format=%cs", "HEAD"])?;
    let modified = git(&["status", "--porcelain", "--untracked-files=no"])
        .is_some_and(|status| !status.is_empty());
    let commit = if modified {
        format!("{hash}-modified")
    } else {
        hash
    };
    Some((commit, date))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_repository_names_one_version() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .map_or_else(std::path::PathBuf::new, Path::to_path_buf);
        assert!(check(&root).is_ok(), "{:?}", check(&root).err());
    }
}
