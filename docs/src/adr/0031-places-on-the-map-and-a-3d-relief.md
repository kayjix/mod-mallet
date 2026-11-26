# 0031 The map shows the places of the game and of mods, also as a 3D relief

- Status: accepted; changed by [ADR 0037](0037-the-map-draws-the-games-own-lod.md), which draws the game's level of detail where there is one; [ADR 0048](0048-one-job-per-page.md): which worldspaces the map lists
- Date: 2026-11-16
- Changes: ADR 0011 (no renderer of our own) and ADR 0029, for the land

## Context

The map of [ADR 0024](0024-views-map-and-live-scans.md) shows cells and
collisions on a grid. It answers "where do mods place objects", but not
"where is that": a cell number means nothing to someone who knows the
game by its towns, caves and forts. Interactive maps of the game on the
web show exactly those places, sorted by kind (city, cave, fort, dragon
lair) and to be filtered, and people find their way with them at once.

For a modded setup the interesting question is a different one: which of
these places are the game's own, which did a mod add, which did a mod
change or remove, and near which of them do two mods collide.

The game already says where its places are. Every place on the world map
is a *map marker*, a reference to the game's marker object with a name, a
kind and a position. Mods that add a place add a marker, and mods that
move or rename one change the game's marker.

ADR 0011 rules out drawing the world, and ADR 0029 kept that for terrain.
The land, though, is already in the report: four heights per cell side,
read from the game's own `LAND` records. Drawn as a surface, with the
terrain picture of ADR 0027 laid over it, that is a relief, not the
world: no meshes, no objects, no lighting of the game.

## Decision

1. **Map markers are read from every plugin.** A marker is a `REFR` with
   an `XMRK` field; its kind is the first byte of `TNAM`, its name the
   `FULL` field, its flags `FNAM`. Names of localized plugins (the game's
   own) are looked up in their string tables (`strings/<plugin>_<language>.strings`),
   loose or in an archive; English is taken if the setup has several
   languages. The markers of the game's own plugins are read once, from
   the persistent cells only, and kept in the cache like their land.
2. **Each marker has an origin**, decided by the analysis: the *game*'s
   own and unchanged; *added* by a mod; *changed* by a mod (moved more
   than 256 units, renamed or of another kind); *removed* by a mod
   (deleted or switched off). The mod is named.
3. **The 60 kinds of the game are put into about 35 groups**, the ones the
   game's map shows: city, town, cave, fort, Nordic ruin and so on. The
   castles and palaces of the holds count as castle and city.
4. **The report carries the markers per worldspace** and, for every
   collision, the **position** between the objects involved, so that it
   can be pinned exactly. A worldspace is on the map if mods place objects
   in it or it has markers, and its land covers both.
5. **The window shows the places** on the map with a filter like the one
   of the web maps: by origin, with counts, and by kind, with counts,
   "show all" and "hide all". A picked collision names the nearest place
   and how far away it is. Every place has the console command that takes
   the player there.
6. **A 3D relief** of a worldspace is drawn from the report's heights,
   with the terrain picture laid over it where there is one, the cells of
   mods tinted, and places and collisions standing on it as pins. It is a
   view of the same report, drawn with WebGL in the window, without a
   library; it loads only when chosen and frees its memory when left.

## Consequences

- Someone who knows the game finds a collision by its place: "between
  the two houses at Riverwood", not "cell 5, -11".
- A mod's new places show up on the map before the game is started, and
  so do places a mod moved away or removed.
- The relief is coarse: one height per 1024 units, about 15 metres.
  Mountains and valleys are recognisable; a house is not. That is
  enough to see where something is, and the game remains the way to
  look at it (ADR 0011 still holds for everything above the ground).
- The report grows by the markers (a few hundred per worldspace) and by
  the land of the whole worldspace rather than of the cells of mods:
  about 300 kilobytes for Skyrim.
- The first scan after the game was installed or updated reads the
  persistent cells of the game's plugins once more, about a second.
- Places that have no map marker (an NPC, a skill book, a word wall) are
  not shown. They are not places of the world map, and finding them would
  mean reading many more records for little gain in finding conflicts.

## Alternatives considered

- **Using the data of a web map.** It is someone else's work, under its
  own terms, describes only the game without mods, and needs the network
  (ADR 0014).
- **Location records (`LCTN`)** instead of markers. They describe areas
  and their names, not where on the map a place is; the marker is what the
  game itself shows.
- **A full 3D view with objects.** That is the renderer ADR 0011 rules
  out; the boxes of ADR 0023 drawn in the relief would suggest a precision
  the relief does not have.
