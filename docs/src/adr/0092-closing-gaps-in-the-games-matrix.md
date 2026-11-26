# 0092 Closing gaps in the games matrix: Fallout 4's catalogue, the older navigation meshes, LOOT and OBSE64 for Oblivion Remastered

- Status: accepted
- Date: 2026-11-26
- Changes: ADR 0054 (the catalogue), ADR 0065 (Oblivion Remastered),
  ADR 0078 (other games closer to Skyrim)

## Context

`docs/src/product/games.md` lists, per game, what works and why not.
After ADR 0078 the gaps that paid off most were: LOOT's rules for Oblivion
Remastered, the catalogue for Fallout 4, the walkways of Fallout 3 and New
Vegas, and OBSE64's log. The start page described Fallout 4 as having "the
basics", far less than it has. What was found, read-only, on the reference
machine and in the tools' own files:

- **LOOT's library** (`libloot` 0.29) knows Oblivion Remastered
  (`GameType::OblivionRemastered`). It is handed the installation's top
  folder and finds `OblivionRemastered/Content/Dev/ObvData/Data` itself;
  the folder the checks read (ADR 0065) is not the one it wants. Only
  Skyrim Special Edition's masterlist is on the reference machine.
- **Fallout 4's item records** are Skyrim's with three differences. A
  weapon's `DNAM` starts with its ammunition, so the way it is held is not
  where Skyrim keeps it; what a weapon is comes from `WeaponType…`
  keywords (`WeaponTypePistol`, `WeaponTypeHeavyGun`, `WeaponTypeMelee1H`,
  each weapon having several). Weapons, armour and ammunition that players
  cannot use carry record flag `0x4` (158 of 252 weapons and 237 of 688
  pieces of armour in `Fallout4.esm`). The body slots differ: 33 is the
  body, 36 to 40 the clothes under armour, 41 to 45 the armour's pieces
  for torso, arms and legs, 30, 31 and 46 to 49 the head. The names of a
  weapon's object templates (`FULL` between `OBTE` and `STOP`) are not its
  own. The string tables are named by language code
  (`Fallout4_de.STRINGS`), not by language. Most guns are built from parts
  at run time; their record names only the receiver's model.
- **Fallout 3's and New Vegas's navigation meshes** (format version 11)
  keep in fields of their own what Skyrim keeps in `NVNM`: vertices
  (`NVVX`), triangles (`NVTR`, 16 bytes as in Skyrim), door triangles
  (`NVDP`: door, triangle, 2 bytes) and links to neighbouring meshes
  (`NVEX`, 10 bytes as in Skyrim); `DATA` counts them. Neither game is
  installed on the reference machine; the layout is xEdit's.
- **OBSE64** is forked from SFSE. Its library names its log
  `My Games/Oblivion Remastered/OBSE/Logs/obse64.txt` and writes SKSE's
  lines. It packs the game version as SKSE does
  (`major << 24 | minor << 16 | build << 4`); its library for 0.411.140
  holds `019B08C0`, where the minor number runs into the major's.
  Oblivion Remastered's programs carry Unreal Engine's version (5.3.2) in
  their fixed version information and the game's (1.512.105.0 on the
  reference machine) only in the text of it.

## Decision

1. **LOOT's rules for Oblivion Remastered** are evaluated by LOOT's library
   from the installation (`locate::install_dir`); its masterlist is looked
   for in Vortex's folder for `oblivionremastered` and in LOOT's
   `Oblivion Remastered` folder.
2. **The catalogue reads Fallout 4 and Fallout 4 VR** (`Game::has_catalogue`).
   Six weapon kinds are added (pistols, rifles, shotguns, heavy weapons,
   melee weapons, grenades and mines) and two kinds of armour pieces (arms,
   legs); the ammunition's name covers both games. The kind of a weapon is
   its most telling `WeaponType…` keyword; armour is sorted by Fallout 4's
   slots; record flag `0x4` leaves out what players cannot use; a material
   comes from the keywords of armour modifications (`ma_armor_Combat_Arm`
   is *Combat*, `ma_T60_Torso` *T60*), linings and paints name none. A name
   inside an object template is skipped for every game. String tables are
   looked for by language and by language code.
3. **The navigation meshes of Fallout 3 and New Vegas are read**
   (`navmesh::record_geometry`, `record_door_triangles`) into the same
   geometry as Skyrim's. Walkways, the paths mode, reachability and the
   doors of passages work for them. Only Skyrim's layout is cut or written.
4. **OBSE64's log is read** for the plugins it loaded or refused, from its
   place in `My Games`. Its plugins are looked for below the installation.
   No game version is taken from it, and its library per game version is
   not compared with the game: the versions of this game do not fit the
   packing, nor `RuntimeVersion`, which keeps the minor number in 8 bits. A
   finding about a refused plugin then speaks of "your game version".
5. **The start page says per game what works**, in a short line taken
   from the matrix (`reach` in the window's game list), instead of one
   sentence for every game that is not Skyrim Special Edition.

## Consequences

- On the reference machine, Fallout 4's catalogue holds 10,328 things and
  431 sets from 21 plugins, read in under three seconds, with names in
  English and German; 11,121 of their 11,232 models are found.
- Oblivion Remastered's 20 active plugins were checked against a
  masterlist written for the check; the findings came out as for the
  other games.
- On the reference machine OBSE64 is built for 0.411.140 while the game is
  1.512.105. Mod Mallet cannot say so yet: that needs the game version from
  the text of the program's version information and a version type wide
  enough for this game's numbers.
- Fallout 4's guns that are built from parts show only their receiver.
- The layout of the older navigation meshes is checked against made-up
  meshes only.

## Alternatives considered

- **Deriving the start page's line from `GameInfo::lacks`.** Rejected:
  the lacks name four parts only; Oblivion lacks much more (faces, crash
  reports, the map's picture), and a line built from them would oversell
  it.
- **Decoding OBSE64's number with the major taken as 0.** Rejected: it
  fits 0.411.140, but the game is already at 1.512.105, whose numbers
  overlap in the packing.
- **A reader of its own for the older navigation meshes in the game
  crate.** Rejected: the entries are Skyrim's, only spread over fields;
  one reader keeps what the analyses see the same for every game.
