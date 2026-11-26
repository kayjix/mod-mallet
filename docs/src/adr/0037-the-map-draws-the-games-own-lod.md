# 0037 The map draws the game's own level of detail

- Status: accepted; [ADR 0074](0074-level-of-detail-makes-room-for-real-objects.md): the level of detail makes room for real objects up close
- Date: 2026-11-17
- Changes: ADR 0011 (no renderer of our own) and ADR 0031 (places on the map and a 3D relief), for the land and what stands on it as the game shows it from afar

## Context

The 3D relief of ADR 0031 is drawn from eight height samples per cell and
the game's coarsest terrain picture. It shows the shape of the land, but
not what is on it: a town is a coloured patch. Interactive maps on the web
show Skyrim far better, down to the walls and houses of Whiterun.

They do not need a renderer of the game for that. The game ships what it
shows of the distance itself, in every setup:

- terrain meshes (`meshes/terrain/<world>/<world>.<level>.<x>.<y>.btr`) and
  their textures (`textures/terrain/<world>/…dds`) for blocks of 4, 8, 16
  and 32 cells;
- object meshes for the same blocks (`…/objects/<world>.<level>.<x>.<y>.bto`):
  towns, castles, bridges and rocks, reduced, on one shared texture atlas.

They are ordinary meshes that Mod Mallet already reads (ADR 0029). Mods
that generate level of detail (xLODGen, DynDOLOD) replace them, loose or in
their archives, and then the map shows the setup as it is.

## Decision

1. **The relief draws the game's level-of-detail tiles** where the setup
   has them, and the height samples of ADR 0031 where it has none.
2. **Finer tiles near the camera.** The coarsest tiles cover the world;
   where the camera comes close, a tile gives way to its four finer ones,
   down to blocks of four cells. Tiles are loaded on demand, a few at a
   time, kept while they are used, and freed when they have not been for a
   while. Until a finer tile is there, the coarser one stays.
3. **Which version is read** follows the game: archives in load order,
   loose files last (ADR 0028). An index of the level-of-detail files of a
   worldspace is made once and kept while the window is open.
4. **Textures are fetched once per file** and shared by every tile that
   uses them, scaled down to what the window needs.

## Consequences

- The map shows what a player sees from a distance: no small objects, no
  objects that only exist close up, and towns as their reduced versions.
  That is enough to find a place; it is not enough to see two small
  objects standing inside each other. That needs the objects themselves,
  around a chosen point, which is a later step (the "detail around a
  point" of the roadmap).
- Trees have their own level of detail (`.btt`, billboards); they are not
  drawn yet.
- Only reading. Nothing is generated or written; a setup without level of
  detail for a worldspace keeps the relief of ADR 0031.
