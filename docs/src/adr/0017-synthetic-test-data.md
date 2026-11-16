# 0017 Test data is synthetic

- Status: accepted
- Date: 2026-11-15

## Context

The natural way to test a parser for mod files is to run it on mod files.
But the game's files belong to its publisher, and mods belong to their
authors, most of whom do not permit redistribution. A repository that
contains them could not be published. Real files are also large and say
nothing about which property a test depends on.

## Decision

- The repository contains **no game files and no mod files**, not even small
  ones.
- Tests build their input themselves: text fixtures written by hand in the
  shape of the real format, and, for binary formats, files produced by small
  writer functions in the test code.
- Each fixture is as small as the property it tests allows.
- The shape of a fixture is taken from real files, and the source is noted
  in a comment ("taken from a log written by SKSE 2.3.1").
- Tests against a **real installation** are welcome, but they are a separate
  group that runs only when an environment variable points at one. They
  never run in CI and may never be required for a change to be accepted.

## Consequences

- The test suite runs anywhere, in seconds, without owning the game.
- The writer functions needed for binary fixtures are the beginning of the
  code that later writes patch plugins.
- A synthetic fixture can miss a quirk of real files. Each such miss, once
  found, becomes a new fixture.
