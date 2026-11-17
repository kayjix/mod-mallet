# 0036 One adapter for the Bethesda family, settings per game

- Status: accepted
- Date: 2026-11-17

## Context

Mod Mallet read one game, Skyrim Special Edition. Its name, its folders,
its registry entry, its script extender, the identifier Vortex and LOOT
know it by and the names of its own plugins were constants spread over the
adapter, the engine, the LOOT reader and the window.

People who mod Skyrim often mod Skyrim VR, the 2011 Skyrim, Fallout 4, New
Vegas, Fallout 3 or Oblivion as well. These games share their formats:
plugins of records in groups, archives of files, meshes in the same family
of formats. They also share most of what their records mean: placed
objects, cells and worldspaces, the land, map markers. What differs is
small and known: where a game is installed, which plugins and archives are
its own, how archives are named after plugins, which script extender it has
and where that logs, which tools belong to it. Two format details differ
more: Fallout 4 packs its files in `.ba2` archives, and Oblivion's records
have 20-byte headers instead of 24.

[ADR 0010](0010-games.md) foresaw an adapter crate per game. Seven more
crates that would share nearly all their code would be the guess that ADR
warned against, in the other direction.

## Decision

- **One adapter, `mallet-game-bethesda`**, reads every game of the family
  (the crate was `mallet-game-skyrimse`). What differs between the games is
  one table, `game::Game`, with one entry per game. Code elsewhere in the
  adapter asks the table instead of naming a game.
- **The engine is told the game** by its identifier, the one Vortex and
  LOOT use (`skyrimse`, `fallout4`, ...). Without one it is Skyrim Special
  Edition, as before. The command line has `--game`.
- **Games are found** where their launchers say they put them: Bethesda's
  registry entry, Steam's game libraries, GOG Galaxy's registry entry. A
  directory counts only if the game's program is in it.
- **Everything the user decides belongs to one game.** The settings file
  keeps one set of settings per game, and which game was looked at last.
  The places, the findings set aside, the duels decided and the ranking of
  one game never mix with another's. A settings file from before is read as
  Skyrim Special Edition's. Each game has a cache of its own.
- **Switching the game starts the window over.** Nothing of the last game's
  report may stay on screen.
- **What is not known for a game is left out, not guessed.** The kinds of
  map markers are only known in Skyrim's numbering; the other games' markers
  are shown without a kind. Faces are compared only in games that keep a
  face mesh per character. Patches through Base Object Swapper are offered
  only where it runs. A game's support says so: `full` for Skyrim Special
  Edition, `basic` for the others.
- **The formats crate reads the formats of all these games**: `.ba2`
  archives, and plugins with 20-byte headers, detected from the file.

## Consequences

- A new game of the family is one entry in the table, a few texts, and
  tests.
- `Basic` games get the checks that only need the shared formats: the load
  order, file conflicts between loose files and archives, missing masters,
  LOOT's rules, placed objects, the map's relief and the toolbox. Checks
  that need more knowledge of a game come one at a time.
- Meshes of Fallout 4 and Oblivion are not read for textures yet (their
  mesh versions differ); a mesh that cannot be read is left out, as before.
- The map shows the land's picture only for games that keep terrain LOD
  textures in Skyrim's layout (the Skyrim games and Fallout 4); the others
  show the relief alone.
- The core gained game-neutral types for the window: `GameChoice`,
  `KnownGame` and `GameSupport`, and two ways a place can be found (`steam`,
  `gog`).
- Oblivion Remastered keeps its data in a different folder below the game
  and is not part of this decision.

## Alternatives considered

- **A crate per game**, as ADR 0010 expected. Rejected: each crate would
  repeat the reading of plugins, archives, objects, markers and land, and
  differ in a dozen constants.
- **A trait per game.** Rejected for now: the differences are data, not
  behaviour. A trait can come when a game needs code of its own.
- **One settings file per game.** Rejected: one readable file with a section
  per game is easier to find and to back up, and the file already existed.
