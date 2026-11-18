# 0051 Paths over hidden objects are found before and after hiding

- Status: accepted
- Date: 2026-11-19
- Changes: ADR 0025 (patches hide objects through Base Object Swapper: what
  hiding leaves behind is now checked)

## Context

A patch of Mod Mallet hides placed objects through Base Object Swapper
(ADR 0025): the object shows an invisible marker instead of its model, so
it is neither drawn nor solid. Nothing in any plugin changes, and that
includes the navigation meshes, the invisible nets of triangles on which
the game's characters find their way. A mod that builds a deck, a pier, a
bridge or a ship lays its net over it. Hide the object and the net still
runs there: townspeople, guards, followers and animals walk on air or get
stuck at its edge, and the player falls through.

The user ran into this at Falskaar's harbour, where JK's Skyrim moors an
Imperial ship next to Falskaar's longboat. JK's Skyrim replaces the game's
navigation mesh of that cell with one that runs over its ship's deck. The
user wanted to hide the ship and asked whether that breaks the paths.

Mod Mallet already reads navigation meshes, but only which plugin replaces
whose and which doors they connect (the `navmeshes.*` findings), not where
they run.

## Decision

1. **The triangles are read.** The format crate reads the vertices and
   triangles of a Skyrim navigation mesh (`NVNM`, version 12). Its
   vertices are in the coordinates of the worldspace or the interior cell,
   like placed objects. Other games' layouts are not read, so the check
   says nothing there; only Skyrim has Base Object Swapper anyway.
2. **Every version, in the cells of the hidden objects.** Only the cells
   an object to hide reaches into are read, from every active plugin,
   with every plugin's version of every navigation mesh there and the
   ground of those cells. The version the game uses is the last; the
   others are kept and marked, because they say whether the path is the
   object's own mod's or another one's.
3. **A triangle runs over an object when the object carries its middle.**
   The surface is the object's model where it was read (ADR 0039), else
   its box. A triangle counts when, straight below or slightly above its
   middle, the surface lies between 16 units above and 48 units below it:
   the game lays its nets a little above what characters stand on.
   Outside, a surface less than 32 units (about a step) above the ground
   does not count: the ground would carry everyone as well. Nor does an
   object less than 32 units high, a rug or a vine on a floor: it lies on
   something that carries as well. Inside there is no ground to compare
   with, and every other carried triangle counts.
4. **The scan checks the patch in the game.** The objects the installed
   swap file hides become facts (`hidden_objects`), their cells and models
   are read with the scan, and a finding `patch.hides-walkway` (warning,
   likely) names, per plugin whose objects are hidden, how many objects
   carry paths the game uses, which ones, where, how many triangles and
   from which plugins the paths come. Versions the game does not use do
   no harm and are not reported. Objects of the game that no mod moved
   are not among the scan's placed objects and are not checked.
5. **The window asks before a choice.** The engine command
   `hidden_walkways(world or cell, centre, objects)` answers the same
   question for objects not hidden yet, from the objects the user chose:
   every version of every path over each of them, with whether the game
   uses it. The collisions page and the guided review can warn right at
   the decision. Choices are the window's (`Settings::patch_choices`); the
   engine does not interpret them.
6. **The texts say what happens and what to do:** keep the object (let
   the other mod's object give way, or keep both), or accept it knowingly
   where nobody needs to walk. Repairing the paths, by removing the
   triangles from a patched copy of the navigation mesh, is a later
   feature, and the texts say Mod Mallet cannot do it yet.

## Consequences

- At Falskaar's harbour: hiding JK's ship hull leaves 109 triangles of
  the navigation mesh JK's Skyrim uses in Tamriel 6,26 on thin air. Its
  rope ladders and Falskaar's longboat carry no path.
- On the user's setup, the installed patch (77 objects) hides 11 floor
  pieces, rubble and platform stairs of JK's Sky Haven Temple on which
  101 triangles of that mod's own paths lie.
- A scan with an installed patch reads the cells of the hidden objects
  once more: 0.7 seconds for those 77 objects. Without a patch nothing is
  read.
- The check is a heuristic. A path along the side of an object, or one
  whose triangles are much larger than the object, may be missed; a
  floor piece inside is counted although hiding it may leave a floor of
  another object below, and a thin plank bridge without railings is not.
  The tolerances are named constants and can be adjusted with real cases.
- The game's own objects that no mod moved can be hidden by the window,
  but the scan does not know their shape. The command still checks them,
  because it reads every object near the point.

## Alternatives considered

- **Compare boxes only.** Fast, but a ship's box reaches down to its keel
  and around its masts; paths on the pier beside it would count. The
  models are read for collisions already.
- **Check the saved choices in the scan.** The choices name findings and
  spots as well as objects, in keys only the window understands. The scan
  reports what the game does now, the installed patch; the window asks
  about what is not installed yet.
- **Report every version.** Paths of overridden versions are never used;
  they would only frighten. The command gives them, marked, so the window
  can say whose path it is.
