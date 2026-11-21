# 0059 Where characters still go once objects are hidden

- Status: accepted
- Date: 2026-11-21
- Changes: ADR 0051 (the finding `patch.hides-walkway` names what becomes
  unreachable); ADR 0052 (its check of what is cut off is joined by one of
  where characters go)

## Context

ADR 0051 finds the paths characters walk on that run over hidden objects;
ADR 0052 cuts them out and checks which parts of the paths would no
longer hang together with the rest. A repair that lays new triangles
around or under the cut is being built beside this decision (ADR 0058).

Neither says what a player notices. A part of the paths of 30 square
metres means little; that the blacksmith can no longer reach the forge,
that nobody gets to the door of a house, or that guards walk round the
whole harbour because a jetty is gone, means a lot. The user wants to be
told plainly what breaks, before hiding, after hiding and after a repair,
and to see the way characters took and the way they will take.

What the game reads:

- The triangles of a navigation mesh lead onto each other by their edges
  and onto those of neighbouring meshes by links (ADR 0052). Door
  triangles are where characters step through a door; a door that leads
  into another cell names the door on the other side (`XTEL` of the placed
  door). The field was already read whole; nothing new is needed from it.
- What characters walk to are placed objects: doors (`DOOR`), furniture
  to sit, sleep or work at (`FURN`), idle markers, spots to stand or do
  something (`IDLM`), and map markers (`XMRK` on the placed object).
  Quests and characters' routines name further objects, often invisible
  markers, as places to go; the pointers at placed objects are already
  read (ADR 0046).

## Decision

1. **A graph of where characters can step.** Its nodes are the triangles
   of the versions of the meshes the game uses, in the cells read. Edges
   join neighbouring triangles, links join meshes that were both read, and
   a door joins its triangles with those of the door on the other side
   where both were read. A link to a mesh that was not read, and a door
   whose other side was not read, lead out into the rest of the world.
   A door triangle that leads nowhere, as when it is cut out, is no way
   out. Where nothing leads out at all, the largest part counts as where
   characters come from. A step costs the distance between the middles
   of the two triangles; going through a door costs nothing.
2. **Destinations.** Doors, furniture, idle markers and map markers in the
   cells read, in the version the game uses, read with the meshes. In the
   window, also the objects around that a quest's alias, a character's
   routine or a location's role names (ADR 0046), with who uses them.
   Each is reached on the nearest triangle on which one can stand, within
   about three and a half metres.
3. **One pure function compares two versions of the paths:**
   `mallet_analysis::reachability::reachability(before, after,
   destinations)`, over the same triangles the walkway analysis uses, so
   that a cut (ADR 0052) and a repair (ADR 0058) are checked alike. Of the
   destinations near what changed (within a cell, at most 48, the nearest
   first) that could be reached before, it reports:
   - **gone**: the triangle it was reached on is gone, and the nearest one
     left lies more than a step farther: it stood on the hidden object;
   - **cut off**: no way leads there from the rest of the world any more;
   - **longer**: from another destination nearby the way is at least half
     as long again and 15 metres longer, or leads only through the places
     around that were not read; the worst such pair is told, once.
   With each, the way before and after as points to draw.
4. **The scan tells it in the finding.** `patch.hides-walkway` gets three
   arguments: how many destinations characters could only reach over the
   hidden objects of that plugin, or that stand on them (`unreachable`),
   their names (`unreachableNames`), and how many would only be reached by
   a much longer way (`longer`), all as if the paths over the objects were
   cut out. The same thing placed twice at the same spot, as two mods do,
   counts once. The finding's identity does not change.
5. **The window asks.** The engine command
   `walkway_reach(world or cell, centre, objects)` reads the cells the
   objects reach and one cell around (so that a way round is found where
   it leaves their cells) and answers with each change, its destination,
   and the ways before and after. `walkway_reach_after` takes the paths
   after hiding from a function, so the repair is checked by the same
   command.
6. **The window says it plainly and draws it.** Under the walkway warning
   on the collisions page and in the guided review, sentences such as
   "Characters could no longer reach the door “ShipTrapdoor01”". In the
   3D view of a spot whose side is looked at without its parts, the way
   before is drawn grey and dashed, the way round after green, and a
   destination without a way left is marked red.

## Consequences

- On the user's setup, hiding JK's Skyrim's ship hull in Tamriel 6,26
  (109 triangles of `skyrim.esm:102F4F` in JK's version) leaves ten places
  without what they stood on: the ship's door and its trapdoor into the
  hold, and eight seats and lean markers on deck. Nothing on the pier is
  cut off.
- The installed patch's 11 pieces of JK's Sky Haven Temple (101
  triangles in Tamriel -35,1 and -35,2) carry the temple's forge yard:
  the Sky Haven Forge map marker, the forge, the smelter, the tanning
  rack, the workbenches, sharpening wheels, the alchemy and enchanting
  tables and their idle markers: 22 placed objects, 16 once those placed
  twice at the same spot count once. None of them could be used with the
  paths cut. Over the whole installed patch, the scan names such places
  for 9 of the 13 plugins whose hidden objects carry paths, among them 36
  of Open Cities' Whiterun smithy and yard.
- The scan reads the placed objects of the cells it already reads for
  the paths; the window's first ask reads the pointers of every plugin
  for the quests and routines (a few seconds, kept afterwards).
- The way is measured over the middles of triangles, not as characters
  walk; lengths are approximate, which is enough to tell a detour.
- Characters themselves are not placed: the texts name where characters
  go, not who. A destination farther than a cell from what changed is not
  checked, nor more than 48.
- The 3D view of an interior does not draw the ways yet, and the guided
  review gives the sentences without drawing.

## Alternatives considered

- **Count only the parts cut off (ADR 0052).** Says nothing about what a
  player loses; a cut through a yard may cut off no part and still put a
  forge out of reach because it stood on the hidden floor.
- **Measure from the nearest way out only.** Inside the few cells read,
  every part leads out somewhere at a cell's edge; a bridge's loss would
  rarely show as a longer way. Ways between the destinations near the
  change show it.
- **Name the characters.** Which character uses which bed or seat is
  spread over routines, factions and scripts; the routines that name a
  place are told where known.
