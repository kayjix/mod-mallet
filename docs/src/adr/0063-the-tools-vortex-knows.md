# 0063 The tools Vortex knows, and Mod Mallet as one of them

- Status: accepted
- Date: 2026-11-21

## Context

Vortex lists programs for working on mods on its dashboard, under the
game's tools: xEdit (SSEEdit, with variants such as "Create or Patch" and
"Quick Auto Clean"), Wrye Bash, FNIS, BodySlide, the Creation Kit. Several
of Mod Mallet's findings end in "use SSEEdit" without saying whether the
user has it. The other direction was missing too: Mod Mallet is a tool
for the same people, but the way to it led through a shortcut of its own.

## What Vortex offers (Vortex 2.x, read from the installed app)

- **Where the tools are.** Vortex keeps them in its state, under
  `settings.gameMode.discovered.<game>.tools.<toolId>`. An entry has `id`,
  `name`, `path` (the program), `custom` (added by hand or by a
  collection, as opposed to found by the game's extension), `hidden`,
  start parameters and environment. The live state is in a database only
  Vortex should open (ADR 0009); the hourly backup file has the same
  shape but can be days old, since Vortex does not renew it on restart.
- **What Vortex does not list.** The tools a game extension *could* find
  but did not are not in the state at all. On the user's setup the state
  has FNIS and two SSEEdit entries (plus the script extender); Wrye Bash,
  BodySlide and the Creation Kit are not there. "Vortex does not know it
  yet" cannot be told apart from "not installed".
- **No registration call.** There is no `context.registerTool` or
  anything like it for extensions. Game extensions declare
  `supportedTools` in `registerGame`; that is for games. The way Vortex's
  own "add tool" dialog, and a collection that brings tools, add one is the
  action `ADD_DISCOVERED_TOOL` (`gameId`, `toolId`, `result`, `manual`).
  The reducer ignores it for a game Vortex has not found. The dashboard
  takes the tool's picture from `<userData>/<game>/icons/<toolId>.png`.
  That is what the extension does, too.
- **Where a tool's picture comes from.** For tools added by hand Vortex
  extracts the program's icon itself only in its own dialog; an extension
  has to put the PNG there.

## Decision

1. **The toolbox shows the tools Vortex lists**, in a section of their
   own, "Tools in Vortex", each under the name Vortex shows, with what it
   is for in one or two sentences and the program's path. A tool is *ready*
   when the program Vortex noted is still there, *missing* when it is
   gone. Optional tools never ask for attention. The engine reads them
   like the mods: from the link's live copy of the state if it has them and
   is newer, else from Vortex's backup (read-only). The link's copy now
   carries the tools (name, path, `custom`, `hidden`; not parameters or
   environment).
2. **Kinds are told apart by the program's file name** (`classify_listed_tool`
   in `mallet-game-bethesda`): `xedit`, `xedit-clean`, `wrye-bash`, `fnis`,
   `bodyslide`, `creation-kit`. The game, its script extender and Mod Mallet's
   own entry are not listed again; a tool the toolbox already has (the
   Creation Kit found in the game folder) is not listed twice; of two
   entries of the same kind, the first wins. A program of no known kind
   is listed under Vortex's name with a general line. Texts exist for each
   kind in English and German.
3. **Findings name the tool, they do not start it.** Under a finding about
   plugins that need cleaning (or hold unchanged records) the body says
   which tool in Vortex helps: "Quick Auto Clean is among your tools in
   Vortex: start it from the dashboard", or that Vortex lists none. The
   line is made by the window from the toolbox; findings keep carrying no
   prose.
4. **Mod Mallet launches no tool.** Vortex starts a tool with the setup the
   tool expects (the mods deployed, its own start hooks, "hide Vortex while
   it runs"); a tool started from outside would see a different game
   folder than the user thinks. Starting is one click on Vortex's
   dashboard, which the line points to. This can change if Vortex gets a
   call to start a tool; see Alternatives.
5. **Mod Mallet is added to Vortex's tools by the extension (0.8.0).**
   When Mod Mallet starts and the folder shared with the extension exists,
   it leaves `app.json` (its program, version) and `app-icon.png` there.
   The extension reads them, and for the game in use, if it is one Mod
   Mallet understands and Vortex has found it, dispatches
   `ADD_DISCOVERED_TOOL` with the program, `detach`, and `onStart:
   ignore`, and copies the icon to where Vortex looks. It adds the tool
   once per game (remembered in `dashboard-tool.json`): a tool the user
   removes stays removed. If Mod Mallet moves, the path is updated. The
   extension refuses a note that names anything but an absolute `.exe`
   that exists. The extension update stays the user's click in the
   toolbox; nothing is installed or started by Mod Mallet.
6. The note and icon are written only into Mod Mallet's own folder, and
   only if it exists, so a machine without the link is left alone.

## Consequences

- Without the new extension the dashboard entry does not appear; the tools
  are read from the backup instead (possibly stale, but they change
  rarely).
- Mod Mallet runs inside a packaged app's private `AppData` in the one case
  already described in the Vortex link reference; there, the note lands
  where the extension does not look. Start it as described in the logs page.
- A tool Vortex found by itself, not yet discovered, does not appear
  under "Tools in Vortex" at all. Naming such tools as "missing" would need a
  list of what Vortex could discover, copied out of its game extensions.
- Vortex's `ADD_DISCOVERED_TOOL` is not a documented extension API; a
  future Vortex may change it. The extension is guarded (it logs and goes on
  if the dispatch fails) and the tests pin the shape of the action.

## Alternatives considered

- **Reading the live state from Vortex's database.** Rejected, as in ADR
  0009: only Vortex opens it.
- **Starting tools from Mod Mallet.** Rejected, see decision 4. It would
  also be the first time Mod Mallet starts a program that changes plugins.
- **Looking for the tools on disk ourselves.** Would find programs Vortex
  does not know, and miss custom setups. The user asked for what Vortex
  knows.
- **Finding Mod Mallet's program from the extension** (registry, usual
  folders). Mod Mallet can run from any folder; only it knows.
- **Showing tools Vortex could discover but has not as "missing".**
  Needs a copy of other extensions' knowledge; see Consequences.
