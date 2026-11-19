//! Mod Mallet's project automation.
//!
//! Everything a contributor or CI runs beyond plain `cargo build` lives here
//! as Rust, so it behaves the same on Windows, Linux and macOS and needs no
//! shell (ADR 0016). Run `cargo xtask --help` for the list of commands.

mod app;
mod architecture;
mod ci;
mod commit_msg;
mod generate;
mod tools;
mod versions;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};

/// Mod Mallet's project automation.
#[derive(Debug, Parser)]
#[command(name = "cargo xtask")]
struct Xtask {
    #[command(subcommand)]
    task: Task,
}

#[derive(Debug, Subcommand)]
enum Task {
    /// Regenerate the files that are derived from the code: JSON schemas,
    /// TypeScript types and the reference pages of the documentation.
    Gen {
        /// Change nothing; fail if a generated file is out of date.
        #[arg(long)]
        check: bool,
    },
    /// Run every check that CI runs.
    Ci {
        /// Leave out the desktop app, which needs a built frontend and, on
        /// Linux, system libraries.
        #[arg(long)]
        skip_desktop: bool,
        /// Fail instead of skipping a check when its tool is not installed.
        #[arg(long)]
        require_tools: bool,
    },
    /// Build the desktop app, frontend included, and start it.
    App {
        /// Build an optimized app; slower to build.
        #[arg(long)]
        release: bool,
        /// Only build; do not start the app.
        #[arg(long)]
        no_run: bool,
    },
    /// Check that a commit message follows the Conventional Commits format.
    CommitMsg {
        /// The file git wrote the message to. Asked from git if omitted,
        /// which the hook relies on: lefthook splits a path with spaces.
        file: Option<PathBuf>,
    },
    /// Build the documentation site into `docs/book`.
    Docs {
        /// Open the site in a browser afterwards.
        #[arg(long)]
        open: bool,
    },
}

fn main() -> ExitCode {
    let root = workspace_root();
    let result = match Xtask::parse().task {
        Task::Gen { check } => generate::run(&root, check),
        Task::Ci {
            skip_desktop,
            require_tools,
        } => ci::run(&root, skip_desktop, require_tools),
        Task::App { release, no_run } => app::run(&root, release, !no_run),
        Task::CommitMsg { file } => commit_msg::run(file),
        Task::Docs { open } => tools::build_docs(&root, open),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}

/// The directory that holds the workspace's `Cargo.toml`.
fn workspace_root() -> PathBuf {
    let xtask_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    xtask_dir.parent().unwrap_or(xtask_dir).to_path_buf()
}
