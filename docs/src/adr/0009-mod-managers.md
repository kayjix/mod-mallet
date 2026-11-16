# 0009 Mod managers behind one trait

- Status: accepted
- Date: 2026-11-15

## Context

Mod Mallet needs to know which mod a file belongs to, which mods are
enabled, and how they were installed. That knowledge lives in the mod
manager. The first target is Vortex, but a large part of the community uses
Mod Organizer 2, which stores the same information very differently. If
Vortex's data model leaks into the analyses, supporting a second manager
means rewriting them.

## Decision

Analyses see a mod manager only through the `ModSource` trait in
`mallet-core`. The trait grows one question at a time, as analyses need
answers. Each manager gets its own adapter crate that implements it.

**Vortex** is read from files it already writes, without Vortex running and
without changing anything:

- the deployment manifest `vortex.deployment.json`, which lists every
  deployed file and the mod it came from,
- the state backup Vortex writes as JSON, which holds categories, versions
  and the installer options chosen for each mod. Its LevelDB database is not
  read directly.

**Writing back** to Vortex (changing a file priority, reinstalling a mod with
other options) will go through a Vortex extension that uses Vortex's own
API. Mod Mallet does not edit Vortex's data behind its back.

**Mod Organizer 2** is the second adapter. It is built early enough to prove
that the trait is not shaped like Vortex.

## Consequences

- A setup without any known manager still scans; findings then lack the
  name of the mod.
- The trait's design is validated by a second implementation instead of by
  guessing.
- Reading a backup means the data can be slightly older than Vortex's live
  state. The age of the data is shown where it matters.
