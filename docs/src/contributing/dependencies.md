# Dependencies

Every dependency is code we ship but did not write. The rules come from
[ADR 0016](../adr/0016-automation-and-dependencies.md).

## Before adding one

1. **Is it needed now?** A dependency arrives with the milestone that uses
   it. "We will need this later" is not a reason.
2. **Would a few lines do?** A small, stable function is cheaper to own than
   a dependency to track.
3. **Is it maintained?** Look at the date of the last release, open issues,
   and who else depends on it.
4. **What does it pull in?** `cargo tree -p <crate>` shows the whole
   subtree.
5. **Is the license compatible** with GPL-3.0-or-later? `cargo deny check`
   answers that, including for the subtree.

## Adding one

1. Add it to `[workspace.dependencies]` in the root `Cargo.toml`, keeping
   the list alphabetical.
2. Reference it from the crate with `name.workspace = true`.
3. Run `cargo deny check`. If a new license appears, add it to `deny.toml`
   only after confirming it is compatible, and say so in the pull request.
4. Explain in the pull request why the dependency is needed.

JavaScript dependencies follow the same thinking. Tools used by the whole
repository go into the root `package.json` with an exact version; the
desktop app's own dependencies go into `apps/desktop/package.json`.

## Current choices

| Purpose | Dependency | Note |
|---|---|---|
| Serialization | `serde`, `serde_json` | |
| JSON Schema | `schemars` | Source of the generated contracts |
| Texts | `fluent-bundle`, `unic-langid` | |
| Cache | `rusqlite` (bundled SQLite) | Compiles SQLite from source; needs a C compiler |
| Load order | `libloadorder` | GPL-3.0, by the LOOT team. Brings `esplugin` with it |
| Masterlist metadata | `libloot` | GPL-3.0, by the LOOT team, who ask to be contacted about derived works. Added with milestone 8 (see [ADR 0030](../adr/0030-rules-from-the-loot-masterlist.md)) |
| Compressed records | `flate2` | With its pure Rust backend |
| Files in Skyrim SE archives | `lz4_flex` | Pure Rust, no `unsafe` by default |
| Installer descriptions | `roxmltree` | Read-only XML tree, no dependencies of its own. The text is decoded from UTF-8 or UTF-16 before parsing |
| Command line | `clap` | |
| Errors | `thiserror`, `anyhow` | `anyhow` only in binaries |
| Logging | `tracing`, `tracing-subscriber` | |
| System locations | `dirs`, `winreg` (Windows only) | |
| Desktop shell | `tauri` | |
| Tests | `proptest`, `insta`, `tempfile` | |

## Written ourselves, and why

| Purpose | Where | Why not a dependency |
|---|---|---|
| Plugin records | `mallet-formats-bethesda::plugin` | `esplugin` reads headers and record IDs for LOOT's needs, not the fields of records. Reading only the requested groups keeps a scan fast. |
| Archive file lists and single files | `mallet-formats-bethesda::bsa` | Only the directory and, on request, one file are needed. The `ba2` crate reads whole archives and pulls in C compression libraries. Fallout 4's BA2 format will need a decision of its own. |
| Mesh texture paths | `mallet-formats-bethesda::nif` | The header of a mesh says where each block lies, so the texture paths can be read without understanding geometry. The existing `nif` crate targets an older format version than Skyrim uses. |

## Programs that are run, not linked

| Purpose | Program | Note |
|---|---|---|
| Reading zip, 7z and rar archives | a 7-Zip that is on the machine | See [ADR 0019](../adr/0019-archives-through-7zip.md). The RAR library's license is not compatible with the GPL |

## Still to be checked

`libloot` is added (see above), but the license of the LOOT masterlist
itself, the data it reads, has still not been checked. It must be before
Mod Mallet ships a copy of the masterlist: today the app only reads the copy
that Vortex or LOOT keep on the machine and quotes its messages
([ADR 0030](../adr/0030-rules-from-the-loot-masterlist.md), [ADR 0035](../adr/0035-loots-messages-are-quoted.md)).
