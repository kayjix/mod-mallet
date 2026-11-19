# 0057 One version for the app

- Status: accepted
- Date: 2026-11-19

## Context

Every crate and package said `0.0.0`. Builds of the desktop app are handed
over to be tried on a real setup several times a day, and nobody could
tell which build was running: whether a fix was in it, or which build a
log came from. Only the Vortex extension had a version that changed.

## Decision

1. **One version for the app**, in `[workspace.package]` of the
   workspace's `Cargo.toml`. All crates inherit it. Tauri's config names no
   version, so the app takes the crate's. The npm packages repeat it
   because npm wants one.
2. **Semantic Versioning, starting at 0.1.0**: the minor number for a
   build that brings something new, the patch number for one that only
   fixes. Every build handed over for trying gets its own version.
3. **A change log** in `CHANGELOG.md`, in the style of
   [Keep a Changelog](https://keepachangelog.com), written for people
   who install mods.
4. **The build carries its commit.** `cargo xtask app` passes the short
   commit hash (marked `-modified` with uncommitted changes) and its day
   to the frontend build and the Rust build. The app shows them under
   Settings → About and writes them into its log at start.
5. **CI checks that the places agree**: the step "One version" of
   `cargo xtask ci` fails when a package names another version, Tauri's
   config names one, or the change log has no section for the version.
6. A version is tagged `vX.Y.Z` on its commit.

The Vortex extension keeps its own version, which the toolbox compares
with the one Vortex runs to offer an update (ADR 0026).

## Consequences

- A screenshot of Settings → About or the first line of the log says
  exactly which build someone runs.
- Raising the version touches three files and the change log; the CI step
  catches a forgotten one. The steps are in
  [Releases](../contributing/releases.md).
- A build made outside `cargo xtask app`, such as `npm run desktop -- dev`,
  shows "development build" instead of a commit.
