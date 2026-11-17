//! Checks that the crates depend on each other only as the architecture
//! allows.
//!
//! The rules are explained in `docs/src/architecture/crates.md`. Keeping
//! them here as data means a pull request that bends the architecture has to
//! change this file, where a reviewer will see it.

use std::path::Path;

use anyhow::{Context, Result, bail};
use serde_json::Value;

use crate::tools;

/// Prefix of the crates that belong to Mod Mallet itself.
const OWN_PREFIX: &str = "mallet-";

/// For each restricted crate, the workspace crates it may depend on.
///
/// Crates that are not listed (the engine, the shells and `xtask`) wire the
/// others together and may depend on anything.
const RULES: &[(&str, &[&str])] = &[
    ("mallet-core", &[]),
    ("mallet-analysis", &["mallet-core"]),
    ("mallet-messages", &["mallet-core"]),
    ("mallet-store", &["mallet-core"]),
    ("mallet-formats-bethesda", &[]),
    ("mallet-format-fomod", &["mallet-core"]),
    ("mallet-unpack", &[]),
    (
        "mallet-game-bethesda",
        &["mallet-core", "mallet-formats-bethesda"],
    ),
    ("mallet-source-vortex", &["mallet-core"]),
    ("mallet-knowledge", &["mallet-core"]),
];

/// Crates that may depend on anything in the workspace.
const UNRESTRICTED: &[&str] = &["mallet-engine", "mallet-cli", "mallet-desktop", "xtask"];

pub(crate) fn check(root: &Path) -> Result<()> {
    let mut metadata = tools::cargo();
    metadata
        .current_dir(root)
        .args(["metadata", "--format-version", "1", "--no-deps"]);
    let json = tools::capture(metadata, "cargo metadata")?;
    let metadata: Value =
        serde_json::from_str(&json).context("cargo metadata printed invalid JSON")?;
    let packages = metadata
        .get("packages")
        .and_then(Value::as_array)
        .context("cargo metadata lists no packages")?;

    let mut violations = Vec::new();
    for package in packages {
        let Some(name) = package.get("name").and_then(Value::as_str) else {
            continue;
        };
        let Some((_, allowed)) = RULES.iter().find(|(restricted, _)| *restricted == name) else {
            if !UNRESTRICTED.contains(&name) {
                violations.push(format!(
                    "`{name}` is not covered by the architecture rules; add it to xtask/src/architecture.rs"
                ));
            }
            continue;
        };
        for dependency in own_dependencies(package) {
            if !allowed.contains(&dependency) {
                violations.push(format!("`{name}` must not depend on `{dependency}`"));
            }
        }
    }

    if violations.is_empty() {
        println!("{} crates respect the architecture rules", packages.len());
        Ok(())
    } else {
        bail!(
            "architecture rules violated (see docs/src/architecture/crates.md):\n  {}",
            violations.join("\n  ")
        );
    }
}

/// The workspace crates `package` depends on outside its tests.
fn own_dependencies(package: &Value) -> Vec<&str> {
    package
        .get("dependencies")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|dependency| dependency.get("kind").and_then(Value::as_str) != Some("dev"))
        .filter_map(|dependency| dependency.get("name").and_then(Value::as_str))
        .filter(|name| name.starts_with(OWN_PREFIX))
        .collect()
}
