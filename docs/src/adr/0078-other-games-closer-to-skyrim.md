# 0078 Other games closer to Skyrim: Enderal, Fallout 4's paths, and saying what does not apply

- Status: accepted
- Date: 2026-11-22
- Changes: ADR 0036 (the games of the family), ADR 0052 (navigation meshes
  read), ADR 0065 (pages that do not apply)

## Context

Skyrim Special Edition has by far the most of Mod Mallet: collisions with
the models, the 3D close-up, the map's picture and level of detail,
walkways and what can still be reached, moves, paths and door plugins,
passages, the catalogue, crash reports and LOOT's rules. The other games of
the family were read for their formats; nobody had looked at which of the
features already work for them, and the window showed some pages empty, or
offered Skyrim's tools, for games they do not fit.

What was found, read-only, on the reference machine (Skyrim Special
Edition, Skyrim 2011, Fallout 4, Oblivion and Oblivion Remastered
installed):

- **Fallout 4** already had almost all of it through the shared formats:
  its placed objects and their models (all 128 models around one point
  read, 260,500 triangles), the place up close, the map's picture from its
  terrain textures, and the level of detail (`.btr`, `.bto`, 3,060 blocks).
  What it lacked were the paths: its navigation meshes are of version 15,
  and the reader knew Skyrim's version 12 only, so no path was drawn and no
  destination checked.
- **Fallout 4's navigation meshes** keep Skyrim's parts in the same order:
  vertices, triangles, links to neighbouring meshes, door triangles, then
  cover, waypoints and the search grid. A triangle is 21 bytes instead of
  16 (a height and one byte before the flags, which lie at byte 17), a link
  11 instead of 10 (one byte after the triangle). Read so, every one of the
  7,894 navigation meshes of `Fallout4.esm` ends exactly where its search
  grid does, the link bits of the flags agree with the edges of all 3.66
  million triangles, and all 283,269 links name a triangle their mesh has.
- **Enderal: Forgotten Stories (Special Edition)** is a total conversion of
  Skyrim Special Edition sold as a game of its own (Steam app 976620). It
  ships `SkyrimSE.exe` and is told from Skyrim by `Enderal Launcher.exe`
  beside it, as LOOT's load order library does. It keeps its settings and
  saves in `My Games/Enderal Special Edition`, its list of active plugins
  in `AppData/Local/Enderal Special Edition` (the library finds it there
  when it sees the launcher), its own master `Enderal - Forgotten
  Stories.esm` beside Enderal's versions of Skyrim's masters, SkyUI, and
  archives named `E - …` and `L - …`. Vortex, Nexus Mods and LOOT call it
  `enderalspecialedition`; Mod Organizer 2 calls it "Enderal Special
  Edition". SKSE64, the address library and Base Object Swapper are those of
  Skyrim Special Edition. Vortex brings no picture of it, but one of the
  first Enderal; the extension that adds the game to Vortex is downloaded
  and keeps its picture in Vortex's `plugins` folder. On the reference
  machine Enderal Special Edition is not installed: only an empty folder of
  the first Enderal (on Skyrim 2011) remains.
- **What cannot apply.** The catalogue knows Skyrim's item records only;
  patches need Base Object Swapper, which exists for Skyrim; the plugins
  Mod Mallet writes copy Skyrim Special Edition's records; Oblivion keeps
  path grids, Fallout 3 and New Vegas navigation meshes of an older layout.

## Decision

1. **Fallout 4's navigation meshes are read** with the layout above, by the
   same functions as Skyrim's (`navmesh::geometry`, `navmesh::doors`): the
   version a field starts with chooses the sizes. Walkways, the paths mode,
   reachability and passages' paths work for Fallout 4 and Fallout 4 VR as
   for Skyrim. Only Skyrim's layout is cut or written: Mod Mallet writes no
   plugin for Fallout 4.
2. **Enderal Special Edition is a game of its own** (`enderalspecialedition`)
   of the Skyrim family, with Skyrim Special Edition's program, formats and
   tools. A directory is a game's when its program is there and, where two
   games share a program, the file that tells them apart (`Game::launcher`)
   is there for this game and not for the other (`locate::is_installed_in`).
   It is looked for in Steam's libraries only: it writes no Bethesda
   registry entry, and its GOG identifier could not be confirmed. Its own
   master and SkyUI count as the game's plugins, its `E - ` and `L - `
   archives as the game's own. Everything built for Skyrim Special Edition
   applies to it, the plugins Mod Mallet writes and the patches included.
3. **A game's picture** comes from Vortex's own installation as before, or
   else from the downloaded extension whose `index.js` names the game's
   identifier in quotes and that has a `gameart.jpg`. A game may name
   related games whose picture stands for it (`Game::related_art`): Enderal
   Special Edition shows the first Enderal's when Vortex has none of its
   own. Mod Mallet still ships no pictures of games.
4. **The report names what a game lacks** (`GameInfo::lacks`): the
   catalogue, patches, the plugins Mod Mallet writes, the walkways. It
   follows from the game table (`Game::lacks`). Where one applies, the
   window says in one sentence why: the catalogue and patch pages instead
   of their content, the paths mode of the 3D views instead of "no paths
   here". The refusal of a plugin to write names Enderal beside Skyrim
   Special Edition.
5. **What works for which game is kept in one page**,
   `docs/src/product/games.md`, with the marks *checked on an
   installation*, *by construction*, *partly* and *missing*, and a reason
   for each gap. Morrowind is left for later; the page says what it would
   take.

## Consequences

- On the reference machine, around one point of Fallout 4's Commonwealth,
  40 destinations are now checked for reachability and 26 navigation meshes
  drawn in the paths mode, where there were none.
- Enderal Special Edition is built and tested against a made-up
  installation only. The numbers of a real one (its plugins, archives and
  the names of its base archives) are still to be taken; if Enderal's base
  archives are named otherwise, they are counted as mods' archives until
  the table is corrected.
- `Game::registry_key` is optional.
- Fallout 4 stays `GameSupport::Basic`: it lacks the catalogue, patches and
  written plugins. The start page's short description of a basic game says
  less than Fallout 4 now has.

## Alternatives considered

- **Enderal as Skyrim Special Edition with another folder.** Rejected: its
  plugins, settings, Nexus domain, LOOT folder and Vortex identifier are its
  own; a user with both games installed must be able to switch between
  them like between any two games.
- **Reading Fallout 4's navigation meshes in a reader of their own.**
  Rejected: the parts are the same in the same order; only the length of
  two kinds of entry differs. One reader keeps what the analyses see the
  same for both games.
- **Hiding the pages that do not apply.** Rejected for now: the pages keep
  their place in the window for every game, so that nothing moves when the
  user switches games; a sentence says why one is empty.
