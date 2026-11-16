# 0026 The app has a toolbox, a log, and can restart Vortex

- Status: accepted
- Date: 2026-11-15

## Context

Many problems of a modded game are not in the mods but next to them: a
Steam update moves the game to a version the script extender or the
Address Library does not cover yet, the Visual C++ runtime is missing,
Vortex is closed, or still runs an older version of Mod Mallet's link.
A user who has never opened a modding tool does not know these exist, let
alone how to check them.

When an action through the link failed on a real machine, nothing told
what had happened. The engine already reported its steps through
`tracing`, but the desktop app has no console, so nobody saw them. The
cause in that case was invisible from inside the app: Mod Mallet had been
started from a packaged app, whose private copy of `AppData` hid the
patch from Vortex.

Loading a new version of the link needs Vortex to be restarted, which the
user had to do by hand each time.

## Decision

1. **A toolbox.** The engine looks at what a setup needs besides its
   mods and returns a list of `Tool`s (`mallet-core`): an identifier, how
   much the setup depends on it (required, recommended, optional), its
   state (ready, missing, needs an update, not running), and where known
   its version, path and the page to get it from. Like findings, tools
   carry no prose; `mallet-messages` puts them into words. What a game
   needs lives in its crate (`mallet_game_skyrimse::tools`), what belongs
   to a mod manager in its adapter. The desktop app shows the list as a
   view and only the overall state in its sidebar.
2. **Versions are compared, not only presence.** The script extender and
   the Address Library each come with one file per game version. The game
   version the extender last logged decides whether the files present
   fit.
3. **Restarting Vortex on request.** The Vortex adapter can close Vortex
   the way its window's close button does (`taskkill` without `/F`), wait
   up to half a minute for it to be gone, and start it again. It never
   kills Vortex. This happens only when the user presses the button.
4. **A log file.** The desktop app writes the engine's events to
   `%LOCALAPPDATA%\mod-mallet\logs\mod-mallet.log`, one plain line per
   event, and sets the file aside at 2 MB. The Vortex extension keeps
   its own log in the shared folder. Both are meant to be read by the
   user, by someone helping them, and by coding assistants
   (`docs/src/reference/logs.md`).
5. **The extension creates the patches folder.** So that it is always
   the folder Vortex sees, whatever environment Mod Mallet was started
   from.

## Consequences

- An entry in the extender's log for a plugin that is no longer in the
  game is not reported any more: the log describes the last start of the
  game, and the user may already have removed the plugin.
- Looking at the toolbox starts `tasklist` every fifteen seconds while the
  window is visible. It reads only.
- Adding a game means adding its list of tools as well.
- Every tool has a group (mod manager, link, game, helper). The sidebar
  shows whichever tool is the mod manager and its link, so a second mod
  manager only has to add its own tools.
- The log contains paths and mod names from the user's machine. It is
  never sent anywhere by Mod Mallet.
