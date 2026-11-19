//! Every check that CI runs, in one command.
//!
//! CI calls `cargo xtask ci --require-tools`. Locally the same command skips
//! checks whose tools are not installed and says so, instead of failing.

use std::path::Path;
use std::process::Command;

use anyhow::{Result, bail};

use crate::{architecture, generate, tools, versions};

const DESKTOP_CRATE: &str = "mallet-desktop";

pub(crate) fn run(root: &Path, skip_desktop: bool, require_tools: bool) -> Result<()> {
    let mut skipped = Vec::new();

    step("Architecture rules");
    architecture::check(root)?;

    step("One version");
    versions::check(root)?;

    step("Rust formatting");
    let mut fmt = tools::cargo();
    fmt.current_dir(root)
        .args(["fmt", "--all", "--", "--check"]);
    tools::run(fmt, "cargo fmt")?;

    if tools::node_modules_installed(root) {
        step("Formatting and lints of TypeScript, JSON and CSS");
        let mut biome = tools::npx(root, "biome");
        biome.args(["ci", "."]);
        tools::run(biome, "biome")?;

        step("Vortex extension");
        let mut extension = tools::npm(root);
        extension.args(["test", "--workspace", "integrations/vortex-extension"]);
        tools::run(extension, "extension tests")?;

        if !skip_desktop {
            step("Desktop frontend: type check and build");
            let mut build = tools::npm(root);
            build.args(["run", "build", "--workspace", "apps/desktop"]);
            tools::run(build, "frontend build")?;
        }
    } else {
        skip(&mut skipped, "frontend checks", "run `npm install` first");
    }

    step("Clippy");
    let mut clippy = tools::cargo();
    clippy
        .current_dir(root)
        .args(["clippy", "--workspace", "--all-targets"]);
    exclude_desktop(&mut clippy, skip_desktop);
    clippy.args(["--", "-D", "warnings"]);
    tools::run(clippy, "cargo clippy")?;

    step("Tests");
    let mut test = tools::cargo();
    test.current_dir(root).args(["test", "--workspace"]);
    exclude_desktop(&mut test, skip_desktop);
    tools::run(test, "cargo test")?;

    step("Generated files");
    generate::run(root, true)?;

    let mut deny = tools::cargo();
    deny.arg("deny");
    if tools::is_installed(deny) {
        step("Dependency licenses and advisories");
        let mut deny = tools::cargo();
        deny.current_dir(root).args(["deny", "check"]);
        tools::run(deny, "cargo deny")?;
    } else {
        skip(
            &mut skipped,
            "dependency checks",
            "cargo install --locked cargo-deny",
        );
    }

    if tools::is_installed(Command::new("mdbook")) {
        step("Documentation");
        tools::build_docs(root, false)?;
    } else {
        skip(
            &mut skipped,
            "documentation build",
            "cargo install --locked mdbook mdbook-mermaid",
        );
    }

    println!();
    if skipped.is_empty() {
        println!("All checks passed.");
        Ok(())
    } else if require_tools {
        bail!("checks were skipped: {}", skipped.join(", "));
    } else {
        println!("Checks passed. Skipped: {}.", skipped.join(", "));
        Ok(())
    }
}

fn exclude_desktop(command: &mut Command, skip_desktop: bool) {
    if skip_desktop {
        command.args(["--exclude", DESKTOP_CRATE]);
    }
}

fn step(name: &str) {
    println!("\n== {name} ==");
}

fn skip(skipped: &mut Vec<String>, what: &str, how_to_enable: &str) {
    println!("\n== Skipping {what} ({how_to_enable}) ==");
    skipped.push(what.to_owned());
}
