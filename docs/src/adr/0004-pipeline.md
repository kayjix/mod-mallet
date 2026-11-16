# 0004 A pipeline with I/O at the edges

- Status: accepted
- Date: 2026-11-15

## Context

The hard part of Mod Mallet is judgement: deciding from many small
observations that something is wrong. Judgement is easy to get wrong and
needs many tests. Tests are only cheap if the code under test needs no game
installation, no mod manager and no disk.

## Decision

A scan is a pipeline of three stages:

1. **Ingest.** Adapters read files and produce `Facts`, plain data defined
   in `mallet-core`.
2. **Analyse.** Analyses are pure functions from facts to findings.
3. **Present.** Findings are rendered into text and shown.

File, process and network access happen only in adapters and in
`mallet-engine`, which wires the stages together. Within an adapter, parsing
is a pure function from text or bytes to facts; finding and reading the file
is separate code.

`mallet-core` and `mallet-analysis` perform no I/O at all. The dependency
rules that enforce this are in
[Crates and their rules](../architecture/crates.md).

## Consequences

- An analysis is tested by writing facts by hand. No fixtures on disk.
- Parsers can be fuzzed and property-tested directly.
- The core could later run somewhere without a file system, such as a web
  page, without being rewritten.
- Facts need a home in `mallet-core` before an adapter can produce them.
  That is a little ceremony for each new kind of observation.
- Facts are held in memory today. When ingest becomes expensive
  (milestone 1), they move into the store so that unchanged inputs are not
  read again. The stage boundaries stay the same.
