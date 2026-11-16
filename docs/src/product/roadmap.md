# Roadmap

Each milestone is useful on its own and is measured against a problem that
was found by hand in a real setup. A milestone is done when Mod Mallet finds
that problem without help and the reference cases are covered by synthetic
tests.

Milestones 1 to 4 need no user interface beyond the command line. The
desktop app grows only once the findings are right.

| # | Milestone | Reference problem it must find |
|---|---|---|
| 0 | **Foundation** (done). Workspace, lints, CI, documentation, and one check running end to end. | A script extender plugin that the game switched off because it does not support the game version. |
| 1 | **Who wins** (done for files and characters). Read the load order, loose files, archives and character records; determine which file and which version of a character the game uses. Other record types follow as analyses need them. | Loose files of one mod silently overriding the same files packed in another mod's archive. Records of an NPC overhaul overridden by a later plugin. |
| 2 | **Installer re-check** (done). Read each mod's installer from its archive and compare the options that were chosen with what the installer would tick now. A plugin that builds on a plugin which is not installed is reported as well. Further setup checks (shader framework present, generated files out of date) are still open. | A compatibility patch that was not installed because its target mod was installed afterwards. |
| 3 | **References** (done for loose meshes that replace a mesh from an archive). Read which textures a mesh uses and compare a replacement mesh with the one it replaces. | Meshes of one mod pointing at their own textures, so a texture overhaul leaves visible mismatched patches. |
| 4 | **Placed objects** (done for objects that mods add; see [ADR 0023](../adr/0023-placed-objects-by-their-bounds.md)). Read object placements and bounds; find duplicates and overlaps between plugins. Still open: objects of the game that mods move. | The same structure placed by two mods in one spot. Two mods' objects standing inside each other. |
| 5 | **Desktop app** (begun: a first-run screen that asks before reading and the settings behind it, [ADR 0018](../adr/0018-ask-before-reading.md); the report as a task list, [ADR 0022](../adr/0022-the-report-is-a-task-list.md)). Done since: views, a mod list with versions and pages, a cell map of where mods place objects, findings set aside, scanning again when the setup changes ([ADR 0024](../adr/0024-views-map-and-live-scans.md)). Done since as well: a toolbox of what the setup needs besides its mods, with a button that restarts Vortex, a settings page, and a log file ([ADR 0026](../adr/0026-toolbox-log-and-restarting-vortex.md)). Done since as well: a home page, light and dark themes, findings shown as options with buttons, the map on the game's own terrain textures with every cell to pick ([ADR 0027](../adr/0027-findings-offer-decisions.md)). Still open: a folder dialog for the places, LOD tiles from archives, pictures of what a finding is about (faces need BC7 and 3D), mod profiles. | |
| 6 | **Actions** (begun: a mod can be reinstalled in Vortex from its finding, through a Vortex extension, with a guide to the options to tick; see [ADR 0020](../adr/0020-actions-through-the-mod-manager.md)). Since then: plugin rules and switching mods off through Vortex ([ADR 0027](../adr/0027-findings-offer-decisions.md)). Still open: ticking the missing installer options in advance, a patch plugin that changes records (patches that hide objects exist, through Base Object Swapper; see [ADR 0025](../adr/0025-patches-through-base-object-swapper.md)), handing file decisions to Vortex, writing a patch plugin that disables objects. | |
| 7 | **In-game bridge.** Jump to a finding in the game and highlight the objects involved. | |
| 8 | **Knowledge and navmesh.** LOOT masterlist, community rules, detection of navmesh conflicts. | A door that leads onto a navmesh triangle which the winning plugin removed. |
| 9 | **Version changes.** Say what a game update or a mod update did to the setup: which script extender plugins, libraries and patches no longer fit, which of those only need the small update their authors usually publish quickly (Address Library, a rebuilt plugin), and which depend on a change the mod's author has to make. The toolbox already compares the script extender and the Address Library with the game version ([ADR 0026](../adr/0026-toolbox-log-and-restarting-vortex.md)). | RaceMenu's library switched off after Steam updated the game to 1.7.104. |

## Adapters planned along the way

- **Mod Organizer 2** as the second mod manager, to prove that the
  [`ModSource`](../adr/0009-mod-managers.md) seam holds.
- **Fallout 4** as the second game, for the same reason on the game side.
- **Generators** such as DynDOLOD, Nemesis or Seamless Studio: their output
  is derived from the load order and goes stale when the load order changes.
  Mod Mallet should notice that and say so.
