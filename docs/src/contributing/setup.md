# Development setup

## What you need

| Tool | Version | Needed for |
|---|---|---|
| [Rust](https://rustup.rs) | pinned in `rust-toolchain.toml`, installed automatically by rustup | everything |
| Visual Studio Build Tools (Windows) | 2022, components "MSVC C++ x64/x86 build tools" and a Windows SDK | everything on Windows |
| [Node.js](https://nodejs.org) | 22 or newer | the desktop app, formatting, git hooks |
| `cargo-deny` | current | dependency checks |
| `mdbook`, `mdbook-mermaid` | current | building this documentation |

On Windows the first two can be installed with:

```text
winget install Rustlang.Rustup
winget install Microsoft.VisualStudio.2022.BuildTools --override "--passive --wait --add Microsoft.VisualStudio.Component.VC.Tools.x86.x64 --add Microsoft.VisualStudio.Component.Windows11SDK.22621"
```

On Linux the desktop app additionally needs the
[system libraries Tauri lists](https://tauri.app/start/prerequisites/). The
engine and the command line need nothing beyond Rust.

You do **not** need the game, a mod manager or any mod to build and test Mod
Mallet ([ADR 0017](../adr/0017-synthetic-test-data.md)).

A few tests read a real archive and need a 7-Zip
([ADR 0019](../adr/0019-archives-through-7zip.md)). Where none is found they
are skipped and say so.

## First build

```text
git clone <repository url>
cd mod-mallet
npm install
cargo install --locked cargo-deny mdbook mdbook-mermaid
cargo xtask ci
```

`npm install` also installs the git hooks. `cargo xtask ci` runs every check
that CI runs; when it passes, your setup is complete.

## Everyday commands

| Command | What it does |
|---|---|
| `cargo test` | Tests of the engine and the command line |
| `cargo run -p mallet-cli -- scan` | Scan the setup on this machine |
| `cargo run -p mallet-cli -- --lang de scan` | The same in German |
| `cargo xtask app` | Build the desktop app, frontend included, and start it. With `--release` it also puts the finished app at `dist/Mod Mallet.exe`, a single file that can be started from there or copied anywhere |
| `npm run desktop -- dev` | Start the desktop app with live reload |
| `npm run dev --workspace apps/desktop` | The frontend alone in a browser, with a made-up report |
| `cargo xtask gen` | Regenerate schemas, TypeScript types and reference pages |
| `cargo xtask ci` | Every check CI runs |
| `npm test --workspace integrations/vortex-extension` | Tests of the Vortex extension, without Vortex |
| `cargo xtask docs --open` | Build and open this documentation |
| `npm run format` | Format TypeScript, JSON and CSS |
| `cargo fmt --all` | Format Rust |
| `npm run desktop -- icon src-tauri/icons/icon.svg` | Regenerate the app icons after changing `icon.svg` |

Plain `cargo build` and `cargo test` leave the desktop app out, because
compiling it needs the built web frontend. `cargo xtask ci` covers it.

## Snapshot tests

The text output of the command line is tested with
[insta](https://insta.rs) snapshots under `crates/mallet-cli/tests/snapshots`.
When you change wording on purpose, a test fails and writes a `.snap.new`
file next to the old snapshot. Read the difference, and if it is what you
intended, accept it:

```text
cargo install cargo-insta
cargo insta review
```

## Where things are

```text
crates/              the Rust engine, one crate per responsibility
apps/desktop/        the desktop app: React frontend and Tauri shell
integrations/        what runs inside other tools: the Vortex extension
xtask/               project automation (cargo xtask)
docs/                this book
schemas/             generated JSON Schemas of the data contracts
```

[Crates and their rules](../architecture/crates.md) explains the crates.
