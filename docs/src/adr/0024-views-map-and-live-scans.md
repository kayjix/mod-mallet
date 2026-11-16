# 0024 The desktop app has views, a map, and scans again by itself

- Status: accepted
- Date: 2026-11-15

## Context

The task list of [ADR 0022](0022-the-report-is-a-task-list.md) made clear
what to do, but the app still felt like a report with buttons. Using it on
a real setup showed four gaps:

- A finding about a script extender plugin did not say which mod to update
  or where to get the update.
- A reinstall opened an installer of seven pages; which option to tick, and
  on which page, the user had to find out alone.
- Objects of two mods in the same spot were listed by name, but nothing
  showed *where*, and nothing helped to go and look.
- After a fix in Vortex the user had to remember to scan again.

## Decision

- The window has a **sidebar with views**: *Tasks* (the task list),
  *Map*, *Mods* and *Places*. The game, the state of the link to Vortex,
  "live" and the language sit at its foot.
- The report carries **what the mod manager knows about each mod**:
  version, newer version, author and the page it came from. A finding
  shows the mod it concerns on its line, and its details offer to open the
  mod's page. Only pages on Nexus Mods are opened, by the system's own
  browser, without a shell.
- The **reinstall action names the options to tick**, each with its page
  ("page 5 of 7"), step, group and the installer's own description. Steps,
  groups and options are put in the order the installer shows them, which
  is alphabetical unless the installer says otherwise.
- The report carries a **map**: for every worldspace, the cells in which
  mods place objects, how many and which mods, and every collision of two
  mods' objects as a spot with the finding it belongs to and the console
  command that takes the player there (`cow` outside, `coc` inside). The
  map is drawn by the app as a grid; nothing of the game's own map images
  is used.
- A plugin whose solid objects mostly stand in another plugin as well is
  reported as **contained** in it, with the plugins that build on it.
- Findings can be **set aside**. The choice is kept in the settings file,
  not in the cache, and set-aside findings are shown apart and not counted.
- The app **scans again by itself** when a file the scan starts from
  changes. It compares a fingerprint (size and time of those files) every
  five seconds; a scan only reads, so this needs no confirmation.

## Consequences

- The report grows by the map: about two megabytes for a setup with three
  hundred plugins. It is stored and sent to the window as before.
- The link extension hands over version, author and source of each mod
  (version 0.3.0), and Vortex has to be restarted once to load it.
- Live scanning means a scan right after Vortex deploys. A deploy writes
  several files one after another; the next fingerprint check after the
  scan catches the rest.
- The land under the map is drawn from the game's own height data: four
  samples per cell side, from the `LAND` records of the game and the mods.
  The land of the game's own plugins is read once and kept in the cache.
- Showing a spot in the Creation Kit is not possible from outside: it has
  no way to be told which cell to open. The console command is the way to
  look at a spot until the in-game bridge of the roadmap exists.
