# 0064 Mod Organizer 2 as the second mod manager

- Status: accepted
- Date: 2026-11-21
- Builds on: [ADR 0009](0009-mod-managers.md) (mod managers behind one
  trait), [ADR 0018](0018-ask-before-reading.md) (ask before reading),
  [ADR 0020](0020-actions-through-the-mod-manager.md) (actions through the
  mod manager)

## Context

ADR 0009 put every mod manager behind the `ModSource` trait and promised
Mod Organizer 2 as the second adapter, early enough to prove that the trait
is not shaped like Vortex. A large part of the community uses Mod
Organizer, and every Wabbajack modlist installs into it.

Mod Organizer works differently from Vortex in the one way that matters
most here: it never puts the mods' files into the game. Each mod stays in
a folder of its own. While a program started from Mod Organizer runs, a
virtual file system shows the game the files of every switched-on mod in
its data directory, merged by the mods' priority, with the overwrite
folder on top. The game directory on disk holds only the game's own files.
A scan that reads the data directory, as Mod Mallet's did, sees a game
without mods.

Its records are plain files, written through Qt's settings class:

```text
<instance>/ModOrganizer.ini         gameName, gamePath, selected_profile,
                                    base_directory and the other folders
<instance>/profiles/<p>/modlist.txt the mods, highest priority first:
                                    +on, -off, *not managed (DLC);
                                    names ending in _separator are headings
<instance>/profiles/<p>/plugins.txt the game's list of plugins
<instance>/profiles/<p>/loadorder.txt
<instance>/profiles/<p>/settings.ini LocalSaves, LocalSettings
<instance>/mods/<mod>/meta.ini      version, modid, gameName, url,
                                    installationFile
<instance>/overwrite/               files programs wrote while running
```

An instance is **portable** (`ModOrganizer.ini` and `portable.txt` beside
`ModOrganizer.exe`, anywhere on the disk) or **global**
(`%LOCALAPPDATA%\ModOrganizer\<name>`). Values may be wrapped as
`@ByteArray(...)` and use Qt's escapes (`\\`, `\"`, `\x263a`); folders may
start with `%BASE_DIR%`.

Mod Organizer offers no way for another program to switch a mod on, move
it or reinstall it while it runs. It rewrites `modlist.txt` and
`plugins.txt` from its own memory, so a change made in those files behind
its back is lost or, worse, half kept. Its plugin interface (Python) could
take requests, but that would mean installing code into Mod Organizer the
way the Vortex extension is installed into Vortex.

## Decision

1. **A new adapter crate, `mallet-source-mo2`**, depends on `mallet-core`
   only, like the Vortex adapter; only `mallet-engine` wires it in. It
   reads an instance, its `ModOrganizer.ini`, the profile in use (or one
   named), each listed mod's `meta.ini`, and the names of the files in each
   mod's folder and in the overwrite folder. The game's own files, Mod
   Organizer's `meta.ini` at the top of a mod's folder and files Mod
   Organizer hid (`.mohidden`) are not mod files. Separators and the
   entries Mod Organizer does not manage (`*`) are not mods. Each parser
   has a property test that it never panics.
2. **The seam holds without a new question.** `Mo2Setup` answers the three
   questions of `ModSource` as Vortex does: the mod whose file the game
   sees (the switched-on mod with the highest priority, below the overwrite
   folder), every file a mod brings, and the switched-off mods that have a
   file. Mods are named by their folder, which is also the name Mod
   Organizer shows; the overwrite folder is the mod `Overwrite`. The
   version, the newer version, the page (the noted `https` address, else
   the Nexus Mods page from `modid` and `gameName`) and whether the mod is
   switched on go into the report's list of mods.
3. **The data directory is read as the game sees it.** The game adapter
   takes an optional list of folders laid over the data directory, the
   folder that wins first, which is what Mod Organizer's virtual file
   system does (`ingest::VirtualData`). Plugins and archives are taken from
   the first folder that has them, loose files from the first folder that
   has the path, and the load order library is given the same folders
   (`set_additional_plugins_directories`). The profile's folder is the
   folder of `plugins.txt`. For Vortex the list is empty and nothing
   changes.
4. **The user chooses the manager, per game.** The settings of each game
   keep `manager` (`vortex` or `mo2`). Until the user chose, the one found
   is suggested: Vortex if it deployed into the game or Mod Organizer has
   no instance, else Mod Organizer. Finding out opens no file (ADR 0018):
   it looks whether Vortex's folder exists and lists the folder of the
   global instances. The first-run page and the places in Settings show
   the managers, each instance of Mod Organizer as its own choice when
   there are several, and the places the chosen one would read; nothing is
   kept until the user confirms. A portable instance is named by its
   folder like any other place. Which global instance manages which game
   is read only once the user agreed; an instance of another game is not
   used. The command line takes `--manager mo2`, `--mo2-instance` and
   `--mo2-profile`.
5. **New places.** The instance (`manager-instance`) and the folder of
   mods (`manager-mods`, whose files are read where the game sees them)
   are places like the others, found as "named in Mod Organizer's
   settings" (a new origin, `manager-settings`). The places of Vortex are
   not read for a game managed by Mod Organizer.
6. **Actions become instructions.** Nothing is ever written into Mod
   Organizer's files or a mod's folder. An action for a setup managed by
   Mod Organizer is worded as what to do there by hand ("In Mod
   Organizer's list of mods on the left, untick Flora Mod"), and pressing
   its button returns a new status, `by-hand`, that says so instead of
   handing it to a link. The patch that hides objects is still written,
   into Mod Mallet's own folder, and the user installs it from there with
   Mod Organizer's "Install a new mod from an archive". The toolbox leaves
   out the Vortex extension and the restart of Vortex.

## Consequences

- Who wins (milestone 1) works the same for both managers: the reference
  test of loose files over another mod's archive passes unchanged against
  a synthetic portable instance, with a separator, a switched-off mod and a
  file in the overwrite folder.
- The maintainers have no Mod Organizer. Tested on the reference setup
  (Skyrim SE 1.7.104, Vortex, 162 deployed mods) with a made-up instance in
  a temporary folder whose mods folder is Vortex's staging folder, read
  only, and whose order was derived from Vortex's deployment so that every
  file Vortex deployed wins in the same mod. See the end of this record.
- What reads the data directory outside the scan proper still reads only
  the real one: the 3D views, the map's level of detail, the catalogue,
  the versions of a file, the models read for collisions, the paths near
  hidden objects, the toolbox's look at the script extender's plugins, the
  patch already in the game, and LOOT's conditions. Under Mod Organizer
  they see the game without mods until each is given the same folders.
- The installer re-check (milestone 2) does not run for Mod Organizer: it
  needs the options chosen in each installer, which Mod Organizer does not
  keep in a form Mod Mallet reads. `installationFile` and the downloads
  folder are read and ready for it.
- The games whose load order is the files' dates (Oblivion, Fallout 3, New
  Vegas) take the dates of the files in the mods' folders, which Mod
  Organizer may present differently; `loadorder.txt` is read but not yet
  used for them.
- A profile that keeps its own game INIs (`LocalSettings`) is read and
  logged, but the load order library cannot be told to look there; Mod
  Mallet reads no other INI of the game. Saved games are not read at all.
- Files a mod keeps for the game's root folder, which the Root Builder
  plugin of Mod Organizer puts there, are seen where they lie in the mod's
  folder, below `Data/root`; where Root Builder puts them is not known.
- The mods' folder listings are not cached yet, unlike Vortex's staging
  folder (ADR 0049).

## Alternatives considered

- **Reading the data directory while Mod Organizer's virtual file system
  is active,** by starting Mod Mallet from Mod Organizer. It sees exactly
  what the game sees, but only while Mod Organizer runs it, and it hides
  which mod a file came from.
- **Editing `modlist.txt` and `plugins.txt` for actions.** Mod Organizer
  rewrites them from memory, so the change is lost or mixed up, and it
  would break ADR 0005 and 0020.
- **A Mod Organizer plugin as the link,** like the Vortex extension. Mod
  Organizer's Python plugins can switch mods and could take requests
  through a folder. Left for later: it needs installing code into the
  user's Mod Organizer, and nobody here can try it yet.

## Other managers, as notes only

Nothing below is built (ADR 0003: nothing on speculation).

- **The Nexus Mods app** keeps every mod's files in a content-addressed
  store and its state in a database of its own, and writes the files of a
  loadout into the game folder, as Vortex does. For Bethesda games it is
  not ready; its early Skyrim support was set aside. Supporting it would
  take an adapter that reads which loadout file came from which mod, from
  an export or a command line of the app rather than its database, and
  actions through whatever outside entry the app offers. The data
  directory would be read as it is.
- **Wabbajack** installs a modlist into a portable Mod Organizer instance,
  so such setups are covered by this adapter once the user names the
  instance's folder. Two things would make them work better: taking the
  game directory from `gamePath` in `ModOrganizer.ini`, because these lists
  usually run a copy of the game in a "Stock Game" or "Game Root" folder of
  their own; and the Root Builder plugin, which many lists use. The
  modlist's own description could later tell which mods belong to the list
  and which the user added.
- **Kortex** also kept mods in folders behind a virtual file system, with
  its settings in XML. Its development has stopped and few use it. An
  adapter would look like this one, with another reader for its files.

## Tried on the reference setup

The made-up instance lists the 162 mods Vortex deployed, in an order that
lets each of the 337 files that several of them bring win in the mod
Vortex deployed it from. Both scans ran without the cache.

| | Vortex | Mod Organizer |
|---|---|---|
| Plugins, archives, replacement meshes read | 397, 209, 6,310 | 397, 209, 6,310 |
| Findings | 444 | 444, the same ones |
| Duels | 179 | 179 |
| Installers read | 39 | 0 (not for Mod Organizer) |
| Reading the instance | | 0.7 s |
| Whole scan | 186 s | 231 s |

The findings agree one by one once the manager's identifier and the names
are put aside: Vortex shows the names from its state, Mod Organizer the
folders. Because Vortex had already put the files into the data directory,
this shows that attribution and precedence agree, not that the data
directory is read from the mods' folders alone; that is what the synthetic
tests show. The longer scan comes from laying 163 folders over the data
directory for every plugin, archive and loose file, and is worth
measuring on a real Mod Organizer setup.
