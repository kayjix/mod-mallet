# Notes for AI assistants working in this repository

Mod Mallet diagnoses a modded game setup, explains what is wrong and
proposes reversible fixes. Rust engine, Tauri + React desktop app. Read
`docs/src/product/vision.md` (especially the non-goals) and
`docs/src/architecture/crates.md` before changing structure.

## Commands

- `cargo xtask ci` runs every check CI runs. Run it before declaring work
  done.
- `cargo xtask gen` regenerates schemas, TypeScript types and reference
  docs. Run it after changing a finding kind, a type that derives
  `JsonSchema`, or the command line. Never edit generated files by hand
  (`schemas/`, `apps/desktop/src/generated/`, `docs/src/reference/findings.md`,
  `docs/src/reference/cli.md`).
- `cargo test` covers the engine. The desktop crate is not a default member;
  `cargo xtask ci` builds it.
- `INSTA_UPDATE=always cargo test -p mallet-cli` rewrites snapshots after an
  intended change of wording. Read the diff before keeping it.

## Logs

When the app or the Vortex link misbehaved on a real machine, read the logs
before guessing: `%LOCALAPPDATA%\mod-mallet\logs\mod-mallet.log` (the app)
and `%LOCALAPPDATA%\mod-mallet\vortex-bridge\extension.log` (the Vortex
extension). See `docs/src/reference/logs.md`. A Mod Mallet started from a
packaged app's shell sees a private copy of `AppData`; the same page says
how to start it so that it does not.

## Rules that are easy to break

- **Crate dependencies are restricted** and checked by
  `xtask/src/architecture.rs`. `mallet-core` depends on no workspace crate;
  analyses, messages, store and adapters depend on `mallet-core` only. Only
  `mallet-engine` wires them together. The shells (`mallet-cli`,
  `mallet-desktop`) contain no logic.
- **No panics outside tests.** `unwrap`, `expect`, `panic!`, indexing and
  `todo!` are denied by lint. Code that reads external data returns an error
  or skips what it cannot read. Every parser gets a property test that it
  never panics.
- **No `unsafe`.**
- **A scan only reads.** Nothing in this code base may write into a game
  directory, a mod, or a mod manager's data. The one exception is
  installing the Vortex extension into its own folder, which only happens
  when the user asks. Changes to a setup are requested from the mod manager
  as actions (ADR 0020); never start an action or install the extension on
  a user's machine without their say-so.
- **Findings carry no prose.** Add the kind in
  `crates/mallet-core/src/kinds.rs`, the texts in every language under
  `crates/mallet-messages/locales/`. See `docs/src/architecture/findings.md`.
- **Finding identities must not depend on the load order.** Never put a full
  FormID into a subject.
- **Test data is synthetic.** Never add game or mod files to the repository,
  and never copy text from a user's real logs into a fixture beyond the
  shape of a line.
- **Dependencies are added when needed**, in `[workspace.dependencies]`, and
  must pass `cargo deny check`. See `docs/src/contributing/dependencies.md`.
- **Architectural changes need a decision record** in `docs/src/adr/`.

## Style

- English for code, comments, commits and docs. User-facing texts are
  translated (English and German so far).
- Conventional Commits (`feat(scope): subject`).
- Public Rust items are documented: what it is for, not how it works.
- User-facing texts address someone who installs mods but has never opened
  a modding tool.
