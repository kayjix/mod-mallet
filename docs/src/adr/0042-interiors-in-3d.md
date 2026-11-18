# 0042 Interiors in 3D

- Status: accepted
- Date: 2026-11-18
- Extends: ADR 0038 (a place up close) to interior cells

## Context

Collisions inside buildings, caves and dungeons are found like those
outside, but the window could only name the cell and offer the console
command to go there: the close-up view of ADR 0038 reads a worldspace
around a point, and an interior cell is no part of a worldspace. It has no
grid position, no land and no level of detail; it is named by its editor
ID, and everything in it lies in the cell's own children.

## Decision

1. **Interior cells are listed once**: the first time an interior is
   asked for, the `CELL` group of every active plugin is read without
   unpacking what is placed in the cells, and each interior cell is kept
   by its editor ID in lower case, the version the game uses naming it,
   with the plugins that have a version of it. Where two cells share an
   editor ID, the one that comes first by its record keeps it.
2. **Looking at a cell reads only that cell**: its persistent and
   temporary children, and only from the plugins that have a version of
   the cell. Each placed object is taken in the version of the last of
   them, as in the game; deleted and initially disabled ones are left out.
   The models come from the same index as outside (ADR 0038). About half
   a second for the first cell, including the list, and a few hundredths
   of a second for each after it.
3. **The result has the shape of an area** (`AreaObjects`): the cell's
   editor ID in place of the worldspace, the middle of what is placed and
   how far it reaches. The window draws it with the same layer as outside,
   in a view of its own (`InteriorView`) without land, level of detail or
   pins, framed on the collision's point, with the same picking and the
   same card of a picked object.
4. **Walls seen from behind are left out by default**, so that a room can
   be looked into from outside; a switch draws both sides of every
   surface. There is no lighting of the cell.
5. Only reading, as everywhere.

## Consequences

- Collisions inside can be looked at and decided in place like those
  outside.
- Leaving out back faces assumes the meshes wind their triangles as the
  game does; surfaces made to be seen from both sides show one side only
  while the switch is on.
- Characters, effects and the cell's own lighting and fog are not drawn.
- The list of interior cells is kept for the setup until the app is
  started anew; plugins added meanwhile are not in it.
