# 0061 Switched-off mods, and mods worth removing

- Status: accepted; changed by [ADR 0073](0073-switched-off-mods-taken-out-through-vortex.md): switched-off mods still loaded are taken out through Vortex; [ADR 0076](0076-task-rows-crash-risks-first.md): the wide-collider advice on the task page
- Date: 2026-11-21
- Changes: ADR 0043 (suggestions about whole mods), points 5 and 7

## Context

On the reference setup, the user switched Immersive Citizens - AI Overhaul
off in Vortex, scanned again, and still found its collisions in the
worklist and in the guided review. Read-only, on that setup:

- Vortex's state, as the link writes it, has the mod switched off in the
  profile in use.
- Vortex had deployed since: its deployment manifest no longer lists a
  file of the mod.
- The game's Data folder still holds `Immersive Citizens - AI
  Overhaul.esp` and its archive, and `plugins.txt` still lists the plugin
  as active. These files are not Vortex's: they are ordinary files, not
  linked to the copies in Vortex's staging folder, and the plugin is a
  different version from the one there. A copy was in the folder before
  Vortex deployed the mod; Vortex set it aside and put it back when it took
  its own away, and kept the plugin active.

So the game loads the mod, and its collisions are real. What was missing
was saying why, and what takes the mod out. A second way to the same
state is a mod switched off in Vortex that Vortex has not deployed since.

Two related worries:

- Decisions made about a mod's collisions while it was on must not be lost
  when it goes off: the user may switch it on again. While it is off, they
  must not count, go into the patch or show as open work.
- ADR 0043 suggests mods that collide widely, but the suggestion sat among
  the optional recommendations of the tasks and nowhere else, and said
  "remove" for every such mod alike. For a mod whose places are mostly
  small and clear, such as Lux Via, removing is the wrong advice; and before
  advising to remove any mod, the user needs to know what relies on it.

## Decision

1. **The engine names the plugins of switched-off mods** in the report
   (`switchedOffPlugins`): every plugin in the Data folder that a mod
   brings which the mod manager keeps installed but switched off, with the
   mod and whether the game loads it all the same. Plugins another mod that
   is on deployed, and the game's own plugins (official and Creation Club),
   are left out. A new question to the mod manager (ADR 0009),
   `ModSource::switched_off_files`, lists the switched-off mods' files.
2. **A plugin of a switched-off mod the game still loads is a warning**,
   `plugins.switched-off-but-loaded` (certain). It says which files keep it
   in the game, whether the mod manager has not deployed since or a copy it
   did not install is left behind, what to do about each, and which plugins
   build on it. Its collisions stay in the worklist, since the game shows
   them; on such a spot the worklist says why.
3. **Decisions about a plugin the game no longer loads wait.** The window
   keeps every choice. It hands its pages only those that name no such
   plugin (a collision by its plugins, a spot by its objects, an object
   hidden on its own by its plugin or the plugin it was hidden against), and
   puts the waiting ones back whenever a page saves. Waiting decisions do
   not count as decided, do not go into the patch and are not open work. The
   patch page says how many wait and for which mods, and when the patch in
   the game still hides objects of such plugins, that the next patch leaves
   them out. When the mod is on again and loaded, they count again.
4. **The suggestion that a mod collides widely gets a tone**, from what is
   in the finding:
   - with rooms of its own (ADR 0043, point 7): keep it, decide the places;
   - with at least half of its places clear, where one object takes up at
     least 20 times the room of the other by their boxes (`clearCount`, as
     the guided review's clear suggestions of ADR 0050): take every clear
     suggestion in the guided review;
   - otherwise: think about removing it, with the number of places and mods.
   It is shown where the user works: among the recommendations of the tasks,
   first; on the mods page, as a tag and at the top of the mod; on the
   collisions page, when the list is narrowed to the mod or a pair with it.
5. **Before removing is advised, what relies on the mod is said**: the
   plugins of other mods that build on its plugins, by mod
   (`dependents`), and the base objects of other mods whose models only this
   mod brings (`assetUsers`). References of other mods' records to its forms
   are not read by the scan and are not counted.
6. **Switching off is offered for every such mod, on a click.** This
   replaces points 5 and 7 of ADR 0043, which offered no button for a mod
   others build on or with rooms of its own: the texts now say what would
   break or be lost, and where plugins build on it, the button first asks,
   naming them. A mod others build on can still not be switched off without
   that question.

## Consequences

- On the reference setup the warning names Immersive Citizens - AI
  Overhaul, its two files and the Lux patch that builds on it; its 84
  collision spots stay listed with a note.
- Of the seven mods that collide widely there, JK's Markarth Outskirts is
  the one to think about removing (48 of 296 places clear); Lux Via (64 of
  99) and JK's Windhelm Outskirts (41 of 62) point to the guided review;
  Cutting Room Floor and three JK's interiors are to be kept for their
  rooms, each naming the patches that build on it (six mods for Cutting
  Room Floor).
- What other mods' records refer to is not known to the scan; removing a
  mod can still break a mod that uses its forms without naming its plugin as
  a master, which is rare, since the game requires the master to load such
  a reference.
- Whether other mods already bring back what a mod restores, such as cut
  content, is not detected.

## Alternatives considered

- **Leave a switched-off mod's collisions out while the game still loads
  it.** The worklist would then not match the game, and the user would
  decide nothing about objects that are there.
- **Offer to deploy in Vortex from the warning.** On the reference setup
  Vortex had deployed; the files that stay are not Vortex's, and deploying
  would change nothing. Vortex's own deploy is one click there.
