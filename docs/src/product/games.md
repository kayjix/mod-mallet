# What works for which game

Mod Mallet reads the games of the Bethesda family through one adapter
(ADR 0036). Skyrim Special Edition was the first and has the most; this page
says, feature by feature, how far each other game has come, and why where it
has not (ADR 0078).

Marks:

- **✔** works, checked read-only against a real installation;
- **✔°** works by construction (same formats and code as a checked game),
  but not checked against a real installation;
- **◐** partly: see the reason below the table;
- **✘** missing or not applicable: see the reason below the table.

The checks of November 2026 were made on one machine with Skyrim Special
Edition, Skyrim (2011), Fallout 4, Oblivion and Oblivion Remastered
installed. Enderal Special Edition, Skyrim VR, Fallout 4 VR, Fallout 3 and
New Vegas were not installed.

| Feature | Skyrim SE | Enderal SE | Skyrim VR | Skyrim (2011) | Fallout 4 | Fallout 4 VR | Fallout 3, New Vegas | Oblivion | Oblivion Remastered | Morrowind |
|---|---|---|---|---|---|---|---|---|---|---|
| Found on the machine | ✔ | ✔° | ✔° | ✔ | ✔ | ✔° | ✔° | ✔ | ✔ | ✘ |
| Plugins, masters, load order | ✔ | ✔° | ✔° | ✔ | ✔ | ✔° | ✔° | ✔ | ✔ | ✘ |
| LOOT's rules | ✔ | ✔° | ✔° | ✔° | ✔° | ✔° | ✔° | ✔° | ✔ | ✘ |
| Archives and loose files | ✔ | ✔° | ✔° | ✔ | ✔ | ✔° | ✔° | ✔ | ✔ | ✘ |
| Meshes and their textures | ✔ | ✔° | ✔° | ✔ | ✔ | ✔° | ✔° | ◐ | ✘ | ✘ |
| Faces against their records | ✔ | ✔° | ✔° | ✔° | ✔° | ✔° | ✘ | ✘ | ✘ | ✘ |
| Script extender log | ✔ | ✔° | ✔° | ✔° | ✔° | ✔° | ◐ | ◐ | ◐ | ✘ |
| Crash reports | ✔ | ✔° | ✔° | ◐ | ✔° | ✔° | ✘ | ✘ | ✘ | ✘ |
| Placed objects colliding, by their models | ✔ | ✔° | ✔° | ◐ | ✔ | ✔° | ✔° | ✔ | ✘ | ✘ |
| A place up close in 3D | ✔ | ✔° | ✔° | ◐ | ✔ | ✔° | ✔° | ✔ | ✘ | ✘ |
| Map: places and cells | ✔ | ✔° | ✔° | ✔ | ✔ | ✔° | ✔° | ✔ | ✔ | ✘ |
| Map: picture from above | ✔ | ✔° | ✔° | ✔ | ✔ | ✔° | ✘ | ✘ | ✘ | ✘ |
| Map: relief and level of detail in 3D | ✔ | ✔° | ✔° | ✔ | ✔ | ✔° | ◐ | ◐ | ◐ | ✘ |
| Walkways, paths mode, reachability | ✔ | ✔° | ✔° | ✔ | ✔ | ✔° | ✔° | ✘ | ✘ | ✘ |
| Patches (hiding objects) | ✔ | ✔° | ✔° | ✘ | ✘ | ✘ | ✘ | ✘ | ✘ | ✘ |
| Plugins Mod Mallet writes: moves, paths, doors | ✔ | ✔° | ✘ | ✘ | ✘ | ✘ | ✘ | ✘ | ✘ | ✘ |
| Passages | ✔ | ✔° | ◐ | ◐ | ◐ | ◐ | ◐ | ◐ | ✘ | ✘ |
| Catalogue | ✔ | ✔° | ✔° | ✔° | ✔ | ✔° | ✘ | ✘ | ✘ | ✘ |
| Vortex | ✔ | ✔° | ✔° | ✔° | ✔° | ✔° | ✔° | ✔° | ✔ | ✘ |
| Mod Organizer 2 | ✔ | ✔° | ✔° | ✔° | ✔° | ✔° | ✔° | ✔° | ✘ | ✘ |
| Toolbox | ✔ | ✔° | ✔° | ✔° | ✔ | ✔° | ✔° | ✔ | ✔ | ✘ |

## Measured on the reference machine

- **Fallout 4** (Steam, never started, so no list of active plugins; a list
  in a scratch folder switched on the four mods present): 17 plugins of the
  game and its Creation Club content, 71 archives, the Commonwealth with its
  map picture (192 × 192 cells), 3,060 blocks of level of detail with 36
  textured land blocks at the coarsest level, 378 objects within a cell of
  one point with all 128 of their models read for collisions (260,500
  triangles), and, after this round, 40 destinations checked for
  reachability and 26 navigation meshes drawn in the paths mode around the
  same point. `Fallout4.esm` holds 7,894 navigation meshes; all are read.
  The catalogue: 10,328 things and 431 sets from the 21 plugins, read in
  under three seconds, among them 8 pistols, 15 rifles, 2 shotguns, 30
  heavy weapons, 35 melee weapons, 39 grenades and mines, 443 clothes and
  pieces for the body, 156 for the head, 85 for the arms, 71 for the legs
  and 810 characters with a face of their own; 11,121 of their 11,232
  models are in the data folder or its archives. Names come in English and
  German from the game's string tables.
- **Skyrim (2011)**: everything above, but of the models around a point in
  Whiterun's hold 157 of 184 were read; the rest are older shapes (strips of
  triangles) the reader skips.
- **Oblivion** (never started; a list in a scratch folder switched on
  `Oblivion.esm`): 774 objects around the Imperial City, 52 of their 53
  models read, the ground of 9 cells. No navigation meshes.
- **Oblivion Remastered**: as Oblivion for records; its meshes are not what
  it draws (ADR 0065). LOOT's rules: its 20 active plugins were checked
  against a masterlist written for the check that named them; a missing
  master, an incompatible pair and a note on a condition came out as
  findings, a rule on a switched-off plugin did not. OBSE64 0.1.0 is
  installed, with its library for game version 0.411.140, but the game's
  program says it is 1.512.105 (in the text of its version information;
  the numbers beside it are Unreal Engine's, 5.3.2, which the versions
  page shows today). OBSE64 is behind the game there, which Mod Mallet
  cannot tell yet. It has never written a log on that machine; a log
  written for the check, naming its one plugin, was read with the plugin
  found in OBSE64's folder beside the Unreal program.
- **Enderal Special Edition** is not installed: only an empty folder of the
  first Enderal (on Skyrim 2011) remains in the Steam library on `D:`. Its
  support is built and tested against a made-up installation; the numbers
  of a real one are still to be taken.

## Why, where it does not work

**Found on the machine.** Morrowind is not a game of this adapter.

**LOOT's rules.** Every game but Morrowind. Oblivion Remastered is handed
to LOOT's library from its installation's top folder, which finds the old
game's data below it (ADR 0092). Only Skyrim Special Edition's masterlist
was on the reference machine; Oblivion Remastered was checked with a
masterlist written for the check.

**Meshes and their textures.** Oblivion: its meshes name textures in an
older way, read only in part. Oblivion Remastered: the meshes in its data
folder are not what it draws (ADR 0065).

**Faces against their records.** The Fallout 3 and Oblivion families make
faces at run time from the character's record; there is no face mesh to
compare.

**Script extender log.** Fallout 3, New Vegas and Oblivion: the log is
listed, but the extenders of these games log less, and the game version is
not read from it. Oblivion Remastered: OBSE64, forked from SFSE, writes
SKSE's lines into `My Games/Oblivion Remastered/OBSE/Logs/obse64.txt`
(the place and the lines are taken from its library, since no log was on
the reference machine); which plugins it loaded or refused is read, the
game version is not: OBSE64 packs it as SKSE does, and this game's
versions (0.411.140, 1.512.105) do not fit that packing, nor the one Mod
Mallet compares versions in. For the same reason its library per game
version is not checked against the game.


**Crash reports.** The readers know Crash Logger, Trainwreck and .NET Script
Framework, made for Skyrim Special Edition; Buffout 4 writes the same shape
for Fallout 4, next to F4SE's log. Skyrim (2011): reports are looked for in
the same places, but whether its crash loggers write the same shape is not
checked. The crash loggers of the older games write other shapes; Oblivion
Remastered writes Unreal crash dumps.

**Collisions and a place up close.** Skyrim (2011): meshes built from strips
of triangles are skipped. Oblivion Remastered: not what the game draws.

**Map picture.** The picture comes from the terrain textures Skyrim and
Fallout 4 keep for the distance (`textures/terrain/<world>/`). The older
games keep their distant land in another form, not read yet.

**Relief and level of detail.** The relief from the ground's records works
for every game; the level of detail of Skyrim and Fallout 4 (`.btr` and
`.bto`) is drawn on top. The older games' distant land is not read.

**Walkways.** Skyrim and Fallout 4 keep the paths characters walk in
navigation meshes of two layouts (version 12 and 15) that Mod Mallet reads.
Fallout 3 and New Vegas use navigation meshes of an older layout (version
11), with vertices, triangles, links and doors in fields of their own; they
are read too, as xEdit describes them, and tested against made-up meshes
only, since neither game is installed on the reference machine. Only
Skyrim's layout is cut or written. Oblivion has path grids of points and
lines, a different thing altogether.

**Patches.** Mod Mallet's patches are read by Base Object Swapper, which
exists for Skyrim Special Edition and VR only (and works for Enderal, on the
same program). For the other games the patch page says so.

**Plugins Mod Mallet writes.** Moves, repaired paths and merged doors are
written as plugins in Skyrim Special Edition's record format. Skyrim VR's
plugins are nearly the same but not tested; Fallout 4's records differ and
need a writer of their own.

**Passages.** Found and shown for every game whose objects are read; deciding
one hides objects through a patch, so it is decided only where patches work.

**Catalogue.** It reads the item records of the Skyrim games and Fallout 4
(weapons, armour, clothes and their models). Fallout 4's weapons are
sorted by their `WeaponType…` keywords into pistols, rifles, shotguns,
heavy weapons, melee weapons and grenades, its armour by its body slots,
with pieces for the arms and the legs; what players cannot use is flagged
in the record itself. Fallout 4 builds most guns from parts at run time:
the catalogue shows the model the weapon's record names, which for those
guns is only the receiver. The older games' records differ more; none is
read yet.

**Mod Organizer 2.** The game names it writes are mapped for every game but
Oblivion Remastered.

## What Morrowind would take

Morrowind (2002) is a Bethesda game, but of an older engine:

- plugins (`.esm`, `.esp`) of a different format: 16-byte record headers,
  records named by text instead of form IDs, and cells by their name or
  grid position;
- archives in an older BSA version (100), meshes in NIF 4.0, no navigation
  meshes but path grids;
- a load order kept in `Morrowind.ini` (`[Game Files]`) and by file date,
  which LOOT's library reads (`GameId::Morrowind`); OpenMW keeps its own;
- the Morrowind Code Patch and MWSE instead of a script extender.

So it needs its own record reader and most of the analyses would read
other records. Vortex supports it, Mod Organizer 2 does too. It is left for
later; the start page shows it as coming.
