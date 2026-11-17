# 0038 A place up close: the objects themselves around a point

- Status: accepted
- Date: 2026-11-17
- Changes: ADR 0011 (no renderer of our own), for a small area around a point

## Context

The level of detail of ADR 0037 shows the world as it looks from afar.
Collisions are about what it leaves out: a mod's pavilion standing in
another mod's signpost. To see them, and later to decide which of the two
to remove or move, the objects themselves have to be drawn where they
stand, with the objects of the game around them.

The scan reads every placed object, but keeps only those of mods: the
game's own are not a mod's doing and would cost memory for nothing.

## Decision

1. **When the camera comes close** (a few cells) and rests, the window asks
   for everything placed within one and a half cells of the point it looks
   at. The objects are read anew for that area, from every plugin in load
   order: only the blocks of cells around the point are unpacked, the
   rest of the worldspace is skipped. The models of the base objects are
   read once and kept. About half a second for an area of Whiterun's
   outskirts with nearly two thousand objects.
2. **Each object is drawn with its own mesh**, placed, turned and sized as
   the game places it. Meshes and textures are found through one index of
   the setup's files, made once, and fetched once per model.
3. **The parts of collisions are tinted and named.** The reduced objects
   of the level of detail are left out where the objects themselves are
   drawn.
4. Only reading, as everywhere.

## Consequences

- Objects that are not in a cell of the area but reach into it (a long
  bridge placed in the next cell) are cut off at the edge of the area.
- The land under the objects is the level of detail's, not the game's full
  terrain; objects may stand a little above or below it.
- Characters, items lying about and effects are not drawn. Markers, which
  the game does not show, are left out.
- Choosing an object to hide or to move from this view is the next step;
  hiding uses the patch of ADR 0025.
