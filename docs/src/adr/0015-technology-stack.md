# 0015 Technology stack

- Status: accepted
- Date: 2026-11-15

## Context

The stack should give users a tool that installs as one program, give
contributors something familiar, and fit the tools Mod Mallet integrates
with.

## Decision

| Part | Choice | Reason |
|---|---|---|
| Engine and command line | **Rust**, one Cargo workspace | Single executable without a runtime to install; the LOOT libraries are Rust; strong guarantees when parsing untrusted files. Speed is welcome but was not the deciding factor. |
| Desktop app | **Tauri 2** | Uses the system's web view, so the app stays small. The Rust engine is linked in directly. |
| Frontend | **React + TypeScript + Vite** | The largest pool of contributors and libraries. Vortex's own interface is React, so a future Vortex extension can share components. |
| JavaScript packages | **npm workspaces** | Ships with Node, so there is nothing extra to install. |
| Texts | **Fluent** | Handles plurals and grammar properly; used by large multilingual projects. |
| Cache | **SQLite** via `rusqlite`, bundled | One file, inspectable with standard tools, no service. |
| Documentation | **mdBook** with Mermaid diagrams | Markdown in the repository, the standard in the Rust world. |
| Windows toolchain | **MSVC** | The documented path for Tauri, and required for a future script extender plugin in C++. |

Windows is the primary platform, because that is where the games run. The
engine and the command line also build and are tested on Linux.

## Consequences

- Contributors to the engine need Rust; contributors to the interface need
  Node as well.
- Building on Windows needs the Visual Studio Build Tools, a download of a
  few gigabytes.
- The stack is three ecosystems (Cargo, npm, and later C++ for the in-game
  bridge). `cargo xtask` hides the differences for everyday work.

## Alternatives considered

- **C# with Mutagen.** The most complete library for Bethesda plugins, and
  the language of much of the modding tool scene. It would require the .NET
  runtime and rule out the LOOT libraries as direct dependencies.
- **A pure Rust interface (egui, iced).** No web stack, but fewer
  contributors, weaker text and layout handling, and nothing to share with a
  Vortex extension.
- **pnpm.** Stricter and faster than npm, but one more tool to install, for
  a workspace with two packages.
