# 0030 Rules from the LOOT masterlist, evaluated by libloot

- Status: accepted
- Date: 2026-11-16

## Context

ADR 0008 puts community-maintained structured data third in the order of
trust, first of all the LOOT masterlist. It says for thousands of plugins
which files they need (`req`), which they cannot be used with (`inc`), and
which released versions contain records that a cleaning tool removes
(`dirty`, recognised by the file's CRC-32). Milestone 8 is about this list.

The list is YAML with anchors and merge keys, and most rules carry a
condition in LOOT's own small language: `active("x.esp")`,
`file("SKSE/Plugins/x.dll")`, `checksum(...)`, `version(...)` and more.
Getting these semantics right is most of the work, and LOOT already does
it: its library, `libloot`, has been rewritten in Rust and is published on
crates.io under GPL-3.0-or-later, like Mod Mallet (ADR 0002).

Mod managers keep a current copy of the masterlist on the machine. Vortex
updates `%APPDATA%\Vortex\skyrimse\masterlist\masterlist.yaml` while it
runs; LOOT keeps one in its local data folder.

## Decision

1. **A new adapter crate, `mallet-knowledge`**, reads the masterlist with
   `libloot` and depends on `mallet-core` only. It evaluates every rule's
   condition against the game as it is and hands on only the rules that
   apply (`KnowledgeFacts`), with whether each named file is there. It
   only reads: files are looked up and checksums computed.
2. **The masterlist is not downloaded.** Mod Mallet reads the newest of the
   copies Vortex and LOOT keep (ADR 0014). It is a place the scan reads,
   shown and confirmed like the others (ADR 0018), and, like Vortex's live
   state, found automatically only if Vortex's state is too.
3. **The analysis decides** (`mallet-analysis::knowledge`):
   - a required file that is missing, or a required plugin that is not
     active: `plugins.requirement-missing`;
   - an incompatible file that is there, or an incompatible plugin that is
     active: `plugins.incompatible`, one finding per pair;
   - a plugin version known to delete objects or navigation meshes:
     `plugins.needs-cleaning` (warning); one with nothing but unchanged
     copies of records: `plugins.identical-records` (note).
4. **The game's own plugins are not reported for cleaning**: the five main
   files and the Creation Club content listed in `Skyrim.ccc`. The game's
   updates replace them, and whether they should be cleaned at all is
   disputed. Requirements and incompatibilities are still checked for
   them.
5. **Messages, tags, groups and load-after rules are not used yet.** LOOT's
   messages are English prose and would break the rule that findings carry
   no prose; sorting stays with the mod manager.

## Consequences

- Requirements of thousands of mods, such as SKSE plugins and libraries,
  are checked without Mod Mallet keeping a list of its own.
- A scan of a setup with about 400 plugins takes about half a second longer;
  most of it is computing the checksums of plugins the list knows dirty
  versions of.
- Findings are only as good as the list. A rule that is wrong or out of
  date gives a wrong finding; its confidence says so (*likely*).
- `libloot` brings `esplugin`, `loot-condition-interpreter`, `saphyr`,
  `petgraph` and `rayon` into the tree. `libloadorder` was there already.
- Without Vortex or LOOT on the machine there is no masterlist and these
  checks do not run. Updating the list from Mod Mallet itself would be an
  explicit, visible action (ADR 0014) and is left for later.

## Alternatives considered

- **Our own YAML reader and condition interpreter.** Smaller, but every
  function of LOOT's condition language would have to be rebuilt and kept
  in step with LOOT; a mistake would quietly report rules that do not
  apply.
- **Running LOOT's program.** It has no command line that reports
  metadata, and it is not always installed; Vortex users have the list
  without it.
- **Reporting dirty official plugins.** LOOT does, but on a current setup
  that is about forty findings about files the user cannot keep cleaned
  and should not need to touch.
