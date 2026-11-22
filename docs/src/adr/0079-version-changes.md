# 0079 Version changes are read from the files, and compared with the last scan

- Status: accepted
- Date: 2026-11-22
- Builds on: ADR 0026 (the toolbox compares the script extender and the
  Address Library with the game version), ADR 0030 (the LOOT masterlist),
  ADR 0006 (the store is a cache)

## Context

Roadmap milestone 9 is about what a game update or a mod update does to a
setup. Its reference problem: Steam updated Skyrim Special Edition to
1.7.104, and the script extender switched RaceMenu's library
(`skee64.dll`) off. The user keeps Steam's automatic updates on, so this
will happen again with every game update.

Until now Mod Mallet learned about script extender plugins only from the
extender's log, which describes the *last start* of the game. After a Steam
update the log still tells of the old version until the game is started,
and once a plugin is removed the log's entry is dropped (ADR 0026). Nothing
said which plugins *will* be switched off, nor why, nor what to do.

The script extender decides from data each plugin carries. Since SKSE 2.1
(game 1.6) a plugin exports `SKSEPlugin_Version`, an 848-byte structure:
its name, the game versions it was built for (up to 16, packed as
`major << 24 | minor << 16 | build << 4`), flags saying whether it finds
the game's code through the Address Library or by patterns, whether it
depends on the layout of the game's structures, and the oldest script
extender it accepts. Older plugins only export `SKSEPlugin_Query`, which
the extender for 1.6 and later does not load; support libraries export
neither. The game's program and the extender's loader carry their versions
in their Windows version information (`SkyrimSE.exe` 1.7.104.0,
`skse64_loader.exe` 0.2.3.1).

Read on the reference setup, the rule SKSE applies is: a plugin runs in
any game version if it finds the code without fixed addresses (Address
Library or patterns) *and* uses the structures since 1.6.629 or none;
every other plugin runs only in the versions it lists. All 46 plugins of
that setup follow this rule exactly as the extender's own log reports them,
including ones that list stray versions next to their flags.

## Decision

1. **The versions are read from the files, as they are now.** The game
   adapter reads the version information of the game's program and of the
   script extender's loader, the game versions the extender brings a
   library for (`skse64_1_7_104.dll`), the versions the Address Library
   has a file for, and, of every DLL in the plugin folder, its file version
   and what it says to the script extender (`mallet_game_bethesda::versions`).
   A small reader of Windows programs (`pe`) reads only the headers, the
   export table, the exported data and the version resource, never a whole
   file. Only SKSE's version data is interpreted so far; other games' DLLs
   are listed with their versions only.
2. **The analysis applies the extender's rule** (`mallet_analysis::versions`)
   to the game version the program has, so that a plugin is judged before
   the game is started again. New kinds:
   - `versions.plugin-wrong-game`: built for other versions only; usually
     needs only the author's rebuild, within days;
   - `versions.plugin-needs-newer-game`: built for newer versions only,
     usually after a mod update;
   - `versions.plugin-before-anniversary`: made for the game before 1.6;
     needs a real change by the author;
   - `versions.plugin-needs-newer-extender`: the user can update the
     extender;
   - `versions.extender-lacks-game`: the extender has no library for the
     game version, so nothing that needs it runs;
   - `versions.address-library-lacks-game` and
     `versions.address-library-missing`: the plugins that find the game
     through it, counted;
   - `versions.master-updated` (hint): a plugin builds on a plugin whose mod
     was updated since the last scan while its own mod was not;
   - `versions.loot-warning` (hint): LOOT's general warnings whose
     conditions are about versions (`version(…)`, `product_version(…)`),
     quoted (ADR 0035).

   Each finding says how bad it is and which way out applies: wait for the
   author's rebuild, install the newer version the mod manager already
   knows, switch the mod off through the mod manager (the action, on the
   user's click), or roll the game back through Steam's Betas or a
   downgrade patcher. Where the extender's log of the last start, in the
   same game version, confirms the verdict, the finding says so; the log's
   own `extender.plugin-incompatible` is then not repeated for a file the
   version checks have read.
3. **Each report keeps the versions it saw** (`ScanReport::versions`: the
   game, the version of the last start, the extender, every plugin's file
   version), and **a scan compares them with the stored scan before it**
   (`ScanReport::version_changes`): the game, the extender, the plugins and
   the mods that changed, matched by identifier or, since Vortex gives an
   updated mod a new one, by page and name. Without an earlier scan, a game
   updated since its last start counts as a change. The engine reads the
   previous report from the store before it replaces it.
4. **A change is kept** from scan to scan while something it broke is
   still broken, and for a week after it was noticed, so that the live
   rescans of the desktop app do not hide it at once. Changes in a row are
   joined from the first version to the last.
5. **The overview shows it at the top**: "Since your last scan (date): the
   game updated from X to Y; these 4 plugins no longer fit", with each
   plugin's headline and its mod, what usually happens next, and the way to
   the tasks. Putting it away is remembered in the viewer's browser only.
   The command line prints the same as one sentence under its header.
6. **The toolbox** takes the game version from the program first, and
   gains a line counting the script extender plugins that fit the game
   version, and lines for the libraries many mods need (PapyrusUtil,
   powerofthree's Papyrus Extender, JContainers) when they are installed,
   each ready or outdated by the same rule.

## Consequences

- A Steam update is noticed at the next scan, before the game is started,
  with the plugins that will be switched off named.
- A scan opens every DLL of the plugin folder and the game's program, a
  few small reads each; on the reference setup, 47 DLLs, well under a
  second.
- Whether a version-independent plugin really works in a new game version
  is not known from its data: Address Library entries may be missing for
  the new version, or the game's code may have changed. Mod Mallet reports
  only what the script extender will refuse, and the Address Library file
  the plugins will look for.
- The comparison needs the store. A scan without it (`--no-cache`) reports
  no changes, only what does not fit.
- The report's `game.version` is now the program's version where it can be
  read, not the one the log names.
- Other games' script extenders (F4SE, SKSEVR, …) describe their plugins
  differently; their DLLs are read for versions, not judged.

## Alternatives considered

- **Only the extender's log.** It is certain, but always one game start
  behind and blind to plugins already removed.
- **A list of known plugins and the versions they support.** It would go
  stale with every update; the plugins carry this data themselves.
- **A history of every scan in the store.** The store is a cache that a new
  scan can rebuild (ADR 0006); one previous report and the carried change
  are enough for "since your last scan".
