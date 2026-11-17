# 0041 The game's own ground up close

- Status: accepted
- Date: 2026-11-17
- Changes: ADR 0038 (a place up close), whose objects stood on the level of
  detail's land

## Context

Up close (ADR 0038) the objects are drawn where the game places them, but
the land under them was still the level of detail's (ADR 0037): a few
hundred points for a block of four by four cells, smoothed and lowered by
the tool that made it. Docks, floors and paths float above it or sink into
it, and a reader cannot tell whether an object stands wrong or the land is
only drawn wrong. Collisions are judged by exactly these places.

The game's own ground is in the plugins: each outside cell has a `LAND`
record whose `VHGT` field holds the heights of 33 by 33 points, a starting
height and then the difference to the neighbouring point, in steps of 8
game units. A mod that reshapes the land replaces the record. The scan
already reads these heights for the map (8 by 8 samples per cell, kept in
the facts); the full grid is not kept, since it would make the facts of a
large worldspace several times larger for a view that needs a few cells.

## Decision

1. **The ground is read with the objects.** When the window asks for the
   objects around a point, the same pass over the plugins in load order
   also reads the `CELL` and `LAND` records of the same blocks of cells:
   the later plugin's land replaces the earlier one's, as in the game. The
   window asks for the land with a command of its own (`area_land`, the
   same world, point and radius); whichever of the two asks second takes
   its part from that pass instead of reading every plugin again. A deleted
   `LAND` record removes the cell's land.
2. **Every point of the grid is sent**: 33 by 33 heights per cell in game
   units, and the vertex colours of the `VCLR` field where the land has
   them, Base64-encoded as the meshes are. Seven by seven cells, the
   largest area, are about 50 000 points.
3. **The ground is coloured with the land texture of the level of
   detail.** Its block's mesh says how its texture lies on the land; the
   window fits that mapping and lays the same texture over the fine
   ground, so that the two meet without a seam in colour and are lit
   alike. Where a block has no land texture, the ground is a plain earth
   tinted by the vertex colours. The game's own ground textures (several
   layers per quarter cell, blended by alpha maps) are not drawn.
4. **Where the fine ground is drawn, the coarser land is left out**: the
   level of detail's land and the relief of the height samples are
   discarded within the cells it covers. Their water stays, so that the
   docks of a harbour still stand over water.
5. Only reading, as everywhere.

## Consequences

- Objects up close stand on the land the game has: the foot of a dock, a
  floor or a path meets the ground where it does in the game.
- At the edge of the area the fine ground meets the level of detail's land,
  which may lie a little higher or lower; a small step can show there.
- A cell inside the area without land of its own (rare, at the edge of a
  worldspace) leaves a hole where the coarser land is discarded.
- The pass over the plugins also unpacks the `LAND` records of the blocks
  around the point: a few kilobytes per cell and plugin.
- Drawing the game's own ground textures, and the water of each cell at
  its own height, are possible next steps.
