# 0065 Oblivion Remastered: the old game's data below the installation, Unreal mods beside it

- Status: accepted
- Date: 2026-11-21
- Changes: ADR 0036 (one adapter for the Bethesda family)

## Context

The Elder Scrolls IV: Oblivion Remastered (2025) runs the old game's logic
on the old game's data, and draws everything with Unreal Engine 5 from
assets of its own. ADR 0036 left it out because it keeps its data in a
different folder. What it looks like on a disk, as found on the reference
machine (Steam, about 20 active plugins, a few Unreal mods, deployed by
Vortex) and in the documentation of its tools:

- **Where it installs.** Steam (app 2623190) installs it into
  `steamapps/common/Oblivion Remastered`, with `OblivionRemastered.exe` in
  that folder and the Unreal program in
  `OblivionRemastered/Binaries/Win64`. The Game Pass / Xbox app version
  installs below `XboxGames` on a drive the user picks, with its program
  in `Binaries/WinGDK`; the exact folder names could not be confirmed on a
  machine, and the game is not sold on GOG. It writes no Bethesda registry
  entry that was found.
- **The old game's data.** `OblivionRemastered/Content/Dev/ObvData` is the
  old game's folder: `Oblivion.ini` and a `Data` folder with
  `Oblivion.esm`, the DLC and Knights plugins, the new `AltarESPMain.esp`,
  `AltarDeluxe.esp`, `AltarGymNavigation.esp` and
  `TamrielLeveledRegion.esp`, and the classic archives (`BSA` version 103,
  `Oblivion - Meshes.bsa` and so on). Plugins have Oblivion's 20-byte
  record headers. The list of active plugins, `Plugins.txt`, lies in that
  `Data` folder, not in `AppData`.
- **The load order** is the order of `Plugins.txt`; a line starting with
  `#` is a plugin switched off. `Oblivion.esm` loads first; unlike the old
  game, masters are not moved before other plugins. LOOT's library
  (`libloadorder`, `GameId::OblivionRemastered`) reads it so, from the
  installation's top folder.
- **Script extender.** OBSE64 lies beside the Unreal program
  (`Binaries/Win64/obse64_loader.exe`, plugins in `Binaries/Win64/OBSE/Plugins`)
  and logs into `Documents/My Games/Oblivion Remastered/OBSE/Logs`. It runs
  with the Steam version only.
- **Unreal mods.** Unreal Engine loads packages (`.pak` with `.utoc` and
  `.ucas`) from `OblivionRemastered/Content/Paks/~mods`; UE4SS, installed
  into `Binaries/Win64/ue4ss` with a small proxy library beside the
  program, loads logic mods from `Content/Paks/LogicMods` and Lua mods from
  `ue4ss/Mods`.
- **Vortex** knows the game as `oblivionremastered`. Its default mod type
  deploys into the installation's top folder; the data folder, the pak
  folder, the binaries and UE4SS's mods are mod types of their own, each
  with its manifest named after the type
  (`vortex.deployment.oblivionremastered-data-folder.json` in `Data`,
  `...-pak-modtype.json` in `~mods`, and so on). Pak mods whose files come
  in their own folders land in the top folder's manifest.

## Decision

1. **Oblivion Remastered is a game of the family** (`oblivionremastered`)
   of the Oblivion records. It is found in Steam's libraries by its
   program. Game Pass installs are not looked for; the user can name the
   folder.
2. **The folder the checks read is `ObvData`.** For this game alone, the
   directory every check reads below is the old game's folder, four
   folders below the installation (`Game::data_root`). The game sees its
   plugins and archives there under `Data`, as the other games do, so paths
   keep starting with `data/` and every check of plugins, masters, load
   order, LOOT's rules, archives and records works unchanged. A folder the
   user names may be the installation or `ObvData`. What lies outside, the
   script extender, UE4SS and the Unreal mods, is found from the
   installation (`locate::install_dir`), and so is the load order, which
   LOOT's library reads from there.
3. **The data folder's manifest** is the one Vortex names after the
   game's data mod type.
4. **Unreal mods are listed, not checked.** Each package in `~mods` and
   `LogicMods` is listed once with its files, its size and the mod it came
   from, as Vortex's manifests in that folder or the installation's top
   folder say. Only folder listings and those manifests are read; no
   package is opened.
5. **What the game does not draw is not read or shown.** Meshes and
   textures in the data folder are not what the game draws
   (`Game::draws_its_data`), so loose meshes, meshes replaced from
   archives, the textures meshes name, and the models of placed objects are
   not read. The pages built on them, collisions, assets and the catalogue,
   say in one sentence why they are empty for this game. The report says
   so in `game.unrealVisuals`.
6. **OBSE64's log is not read yet**, and the game has no editor. The
   toolbox shows OBSE64 and UE4SS by their files.

What carries over: the plugins and masters, the load order and LOOT's
rules, the records through the Oblivion reader (placed objects, the map's
relief and places), the archives, which are classic
BSAs, and the files Vortex deployed. What does not: the 3D views and every
check of meshes and textures. Crash logs: the game writes Unreal crash
dumps (`ue4ss/crash_*.dmp`), not text Mod Mallet can read; none are read.

## Consequences

- On the reference machine the game is found in the Steam library on
  `F:`, its 20 active plugins and 19 archives are read, and its four
  Unreal packages are listed, each with its mod: two from the pak
  folder's manifest, two from the top folder's. Vortex's manifests there
  name a drive the game has since moved from.
- A deployment manifest that names a folder on another drive, because the
  game was moved since Vortex last deployed, is now read where it lies when
  it was found in its usual place, for every game. A manifest the user
  names must still describe the game it is used with.
- `ScriptExtender::loader` and `plugin_dir` are relative to the
  installation, `log` and `Game::editor` are optional.
- Vortex brings no picture of this game along unless its extension for the
  game is part of Vortex's own installation; the start page then shows the
  game's initial, as for any game without a picture.

## Alternatives considered

- **Reading below the installation, with `data/` replaced by the long
  path.** Rejected: dozens of places in the analyses, the patches and the
  window name paths `data/...`; they would all have to learn a second
  spelling for one game.
- **Leaving Oblivion Remastered to a crate of its own.** Rejected for the
  same reason as in ADR 0036: its plugins and archives are Oblivion's.
- **Checking Unreal packages against each other** (two packages replacing
  the same asset). Left for later: it needs reading the packages' tables
  of contents (`.utoc`), and the order Unreal loads them in.
