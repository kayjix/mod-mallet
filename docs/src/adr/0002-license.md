# 0002 License: GPL-3.0-or-later

- Status: accepted
- Date: 2026-11-15

## Context

The best existing building blocks for reading load orders, plugin headers
and the LOOT masterlist are the LOOT project's Rust libraries
(`libloadorder`, `esplugin`, `libloot`), all under GPL-3.0. Linking them
makes the result a work under the GPL. Vortex itself is GPL-3.0 as well. A
license is very hard to change once several people have contributed.

## Decision

Mod Mallet is licensed under **GPL-3.0-or-later**. Every crate and package in
the repository declares it, and `cargo deny` rejects any dependency whose
license is not compatible.

Parts that run as separate programs and only exchange data with Mod Mallet,
such as a future script extender plugin, may choose a different license if
their own dependencies require it. That needs its own decision record.

## Consequences

- The LOOT libraries can be used directly.
- Anyone may use, study, change and share Mod Mallet. Anyone who distributes
  a changed version must share the changes under the same terms.
- Code under licenses that are incompatible with the GPL cannot be included.
  The RAR decompression library is a known example; RAR archives have to be
  read through an external program.
- The LOOT team asks authors of derived works to get in touch. We do so
  before the first release that uses their libraries.

## Alternatives considered

- **MIT or Apache-2.0.** Would rule out the LOOT libraries and mean
  reimplementing their work, including the masterlist condition language.
