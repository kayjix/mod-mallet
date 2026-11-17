# 0038 A place up close: the objects themselves around a point

- Status: accepted; changed by [ADR 0040](0040-collisions-as-a-worklist.md): the order in which a placed object is turned
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
   for everything placed around the point it looks at: one and a half to
   three cells in each direction, more the higher the camera is, so that
   what fills the window is detailed. A dashed square on the ground shows
   the area, and the window counts the models still to come; the closest
   come first. The camera can come down to a few metres. The objects are
   read anew for that area, from every plugin in load order: only the
   blocks of cells around the point are unpacked, the rest of the
   worldspace is skipped. The models of the base objects are
   read once and kept. About half a second for an area of Whiterun's
   outskirts with nearly two thousand objects.
2. **Each object is drawn with its own mesh**, placed, turned and sized as
   the game places it. Meshes and textures are found through one index of
   the setup's files, made once, and fetched once per model.
3. **The parts of collisions are tinted and named.** The level of detail
   stays drawn under the objects, pushed back a little so that the objects
   win where both are: some places, such as Whiterun inside its walls, are
   only in the level of detail of this worldspace.
4. **An object is picked by clicking it**, exactly where it is drawn (each
   object is drawn once more in a colour of its own, and the colour under
   the pointer tells which): its name, the plugin whose version the game
   uses, the plugin that placed it, its number there and its model.
5. Each model is drawn once for all its objects (instancing), which keeps
   a few thousand objects fluid.
6. Only reading, as everywhere.

## Consequences

- Objects that are not in a cell of the area but reach into it (a long
  bridge placed in the next cell) are cut off at the edge of the area.
- The land under the objects is the level of detail's, not the game's full
  terrain; objects may stand a little above or below it.
- Characters, items lying about and effects are not drawn. Markers, which
  the game does not show, are left out.
- Choosing an object to hide or to move from this view is the next step;
  hiding uses the patch of ADR 0025.
