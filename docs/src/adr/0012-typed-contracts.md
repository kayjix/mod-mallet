# 0012 Contracts are generated from Rust

- Status: accepted
- Date: 2026-11-15

## Context

The same data crosses several boundaries: from the engine to the desktop
app's TypeScript, to the JSON output of the command line, and later to a
Vortex extension and other programs. If each side declares the shape by
hand, they drift apart, and the mistake shows up at run time on someone's
machine.

## Decision

The Rust types are the single source of truth.

1. Types that cross a boundary derive `serde` and `schemars` traits.
2. `cargo xtask gen` writes their **JSON Schemas** to `schemas/`.
3. The same command converts the schema to **TypeScript types** in
   `apps/desktop/src/generated/`.
4. Generated files are committed, and `cargo xtask gen --check` runs in CI,
   so a change to a Rust type that is not followed by regeneration fails the
   build.
5. Each contract carries a **version number**
   (`ScanReport::schema_version`) that is raised when a field is removed or
   changes meaning.

JSON Schema was chosen as the intermediate form because it is not tied to
one consumer. It also validates the data files of ADR 0008.

## Consequences

- The frontend fails its type check when the backend changes shape.
- Other programs can consume `mallet scan --format json` against a published
  schema.
- Doc comments on Rust types become the documentation of the TypeScript
  types.
- The build has one more generated artifact to keep fresh. The check in CI
  makes forgetting impossible.

## Alternatives considered

- **`tauri-specta`.** Generates TypeScript bindings for Tauri commands
  directly, but was still a release candidate for Tauri 2 and does nothing
  for consumers outside the desktop app.
