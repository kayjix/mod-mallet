# 0006 The store is a cache

- Status: accepted
- Date: 2026-11-15

## Context

Reading thousands of plugin and archive files takes time, so results have to
be kept between runs. A database that holds valuable data needs migrations
for every change of layout, and a failed migration loses that data. Early in
a project the layout changes constantly.

## Decision

The SQLite store (`mallet-store`) is a **cache**. It contains only what a
scan can recreate from the setup.

- The layout has a version number. A store with a different number is
  emptied and rebuilt. There are no migrations.
- Deleting the file is always safe and is the documented way to recover from
  a damaged cache.
- Anything a person decided, such as ignoring a finding or choosing between
  two mods, is **not** stored here. It goes into a separate, human-readable,
  versioned file that is migrated with care.

Adapters that read something slow keep it in the store through the
`TextCache` trait of `mallet-core`: a text under a namespace and a key, with
a fingerprint of the files it came from. An entry whose fingerprint no
longer matches is simply not found. The adapters do not depend on the
store.

## Consequences

- The cache layout can change freely as milestone 1 adds fact tables and
  fingerprints for incremental scans.
- After an update that changes the layout, the first scan is a full scan.
- Two storage mechanisms have to be maintained instead of one. The second
  one, for decisions, is small and does not exist until the first decision
  can be made.
