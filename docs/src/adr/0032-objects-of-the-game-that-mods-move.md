# 0032 Objects of the game that mods move count as the mod's

- Status: accepted
- Date: 2026-11-16

## Context

ADR 0023 compares only the objects that mods add. A mod that moves a
house of the game into the way of another mod's house was not found,
because that needs the game's own placements, and ADR 0023 assumed
reading them would cost too much.

Measured on a real setup, reading every placed object of the game's five
plugins (about 860 000 references, no other records) takes about 0.4
seconds. What a mod changes about an object of the game is already read:
its version of the reference, with the position it gives it.

Mods change objects of the game for many reasons: to switch them off, to
attach them to a script, to fix a floating rock by a few units. Only a
change of place, turn or size can make an object clash with another
mod's.

## Decision

1. The game adapter keeps, apart from the objects mods add, the winning
   version of every object of the game that a mod changes. If there are
   any, it reads the game's own versions of those objects in a second pass
   over the game's plugins (references only).
2. An object of the game counts as **moved** if the mod puts it at least
   32 units away, turns it by at least 0.1 radian about one axis, resizes
   it by at least a tenth, or puts it into another cell. An object the game
   switches off and the mod brings in counts as moved as well.
3. A moved object is added to the placed objects with `moved` set: the
   plugin that moved it and where the game has it (`Moved`, `Pose`). It
   then **counts as that plugin's** (`PlacedObject::owner`) in the
   comparison of ADR 0023, on the map, and in "is contained in".
4. A clash with a moved object is reported only if **the move causes it**:
   at the game's own place the object would not clash. The game's objects
   as shipped are not compared, so a clash that was there before is not
   the mod's doing.

## Consequences

- A town or fix mod that moves objects of the game into another mod's
  objects is found, named by the mod that moved them.
- A scan takes about 0.4 seconds longer when mods change objects of the
  game, which is nearly always.
- On a real setup, six new pairs appear, among them a fix mod moving a
  ceiling piece into a building mod's ceiling.
- Clashes between the game's objects as shipped and a mod's new objects
  are still not compared; that would mean comparing every object of the
  game.
