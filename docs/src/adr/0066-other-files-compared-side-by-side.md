# 0066 Other files are compared side by side

- Status: accepted
- Date: 2026-11-21

## Context

The assets page (ADR 0048) shows two mods' textures and meshes on one
stage, so that a person can pick the one they like. Duels about other files
had nothing to show. Findings about archives offered only their textures and
meshes for looking at, so a duel such as two house mods sharing a script, or
a menu mod over SkyUI, listed no files at all: the page said there was
nothing to show, counted zero things the mods disagree about, and still
asked the user to pick a winner. Someone who has never opened a modding tool
could not tell which files these were, what they do, or why it matters.

Many of these duels are about files that are the same, byte for byte, in
both mods: shared helper scripts of one author, or a fix that two mods both
ship. Those need no decision at all.

## Decision

1. A finding about archives offers every file it is about: its textures and
   meshes first, as before, then the other files, up to 400. The duel
   lists them all; the window names each by its kind (script, menu, quest
   start data, animation, settings, SKSE plugin, sound, …) and says what
   that kind is for and what it likely means when the other mod's version
   is not used.
2. For those files, the engine reads every version the setup has (in the
   archives in load order, the loose file in the data directory, and the
   loose copies the mod manager keeps back for the two mods concerned) and
   tells, without prose: size, date of the file or its archive, which
   earlier version has the same bytes, the text of text files (up to
   256 KiB), PNG and JPEG pictures, and for compiled scripts their name and
   functions. A function's code is compared by its instruction count and
   the source line of each instruction, counted from its first line, read
   from the debug information the Papyrus compiler writes by default.
3. The window shows, instead of an empty picture, whether the two versions
   are the same; if not, the two texts line by line, the two pictures, or
   the functions that differ or exist on one side only.
4. Where every file of a duel is the same on both sides, the window says so
   and offers to mark the duel as nothing to decide. That records the side
   that wins now as the decision and hands nothing to the mod manager.

Everything is read only, through the existing readers of archives and the
mod manager's folder.

## Consequences

- Duels about other files can be decided knowingly, and identical ones
  cleared with one click.
- Findings about archives carry more preview paths; the task list still
  shows pictures only for textures and meshes.
- Comparing reads each archive's file list once per duel; for a setup with
  many large archives that takes a few seconds the first time a duel opens.
- Scripts compiled without debug information show only their name and
  whether the bytes differ.
- Reports from before this change name only the examples their findings
  quote; the window asks for a new scan to see every file.

## Alternatives considered

- **Decompiling scripts** to show their source: far more code to maintain,
  and a person who never opened a modding tool cannot read Papyrus either.
  Which functions differ says enough to judge "this mod changes how X
  works".
- **A separate decision kind "nothing to decide"**: would change the stored
  decisions and every place that counts open duels. Recording the side that
  wins now has the same effect, since both sides bring the same bytes.
