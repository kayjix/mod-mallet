# 0074 The level of detail makes room for the real objects

- Status: accepted
- Date: 2026-11-22
- Changes: ADR 0037 (the map draws the game's own level of detail) and
  ADR 0038 (a place up close)

## Context

Far away the 3D view draws the game's own level of detail (ADR 0037): the
land and, on it, the reduced objects of towns, castles and bridges. Up
close it draws the real objects around the point the camera looks at
(ADR 0038). The ground already makes room: where the game's own ground is
drawn, the land of the level of detail is left out (ADR 0041). The reduced
objects stayed. In Riften they showed through the roofs and chimneys of
the real houses, and at a door in a city wall only the reduced wall was
seen, closed over the door the user was to judge.

Leaving out the reduced objects of every block that overlaps the place
was tried first and switched off at once: a town such as Whiterun stands
in a world of its own, and in Tamriel it is only there as level of
detail. Left out by block, it vanished and left a hole. The same holds the
other way: a city world draws Tamriel's level of detail, which has the
town's houses, the walls and the land outside.

The game itself leaves out the level of detail of the cells it has
loaded. The window has no such link between a reduced object and the
real ones it stands for: a block's reduced objects are one mesh.

## Decision

1. **The real objects are seen from above.** Whenever the objects up
   close change (an area is loaded, a model or texture arrives, an object
   is moved), they are drawn from straight above into a depth texture
   that covers the area and a little more: for each point, how high the
   highest real object there reaches. Hidden objects count too: where the
   game has loaded a place it draws no level of detail, whether an object
   there is shown or not. Surfaces whose texture is still on its way are
   left out, as when they are drawn.
2. **A reduced object is left out where a real one stands, and only
   there.** A point of a reduced object is not drawn when a real object
   stands within about a metre of it and reaches at least as high as it,
   less 160 game units (2.3 m): a reduced roof or chimney is never quite
   where the real one is. Above that, and wherever no real object stands,
   the level of detail stays.
3. This applies only while the objects up close are drawn and the game's
   detail is switched on. The land of the level of detail is not affected;
   ADR 0041 decides about it.

## Consequences

- Up close the real houses, walls and doors are seen without reduced ones
  in or over them, and towns that are only level of detail stay, in
  Tamriel as in the city worlds on its land.
- While the models of a place arrive, the reduced objects give way one
  real object at a time; nothing is left out before its real object is
  drawn.
- Where a mod removed an object and the level of detail was not made
  anew, its reduced object stays up close: nothing real stands there. The
  game would not draw it in a loaded cell.
- A reduced object that stands more than 2.3 m higher than the real one
  under it, such as a town of another world behind a real ground plate,
  stays visible above that height.
- A real object that is thin from above, such as a fence, may leave a
  reduced one next to it.
- The texture takes up to 2048 by 2048 depth values (16 MB of graphics
  memory) and is drawn again only when the objects change.

## Alternatives considered

- **Leaving out whole blocks or cells** of reduced objects where real ones
  are loaded: tears holes where a town is only level of detail.
- **Leaving out everything within a distance of the camera**: the same
  holes, and the level of detail reappears inside the area as the camera
  moves.
- **Matching reduced objects to real ones by their boxes** (rectangles
  from above): the box of a long diagonal wall covers half a town and
  leaves out reduced houses that have no real counterpart.
