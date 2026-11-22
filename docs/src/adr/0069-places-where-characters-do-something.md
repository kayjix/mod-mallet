# 0069 Places where characters do something can be seen, removed and moved

- Status: accepted
- Date: 2026-11-22
- Changes: [ADR 0059](0059-where-characters-still-go.md) (a place that
  loses what it stood on can be moved onto the ground or hidden too, not
  only reported); [ADR 0060](0060-objects-moved-a-little-through-a-plugin.md)
  (moving is offered for invisible markers, with whether characters reach
  the new place)

## Context

Characters sit, lean, sleep and work at placed objects: furniture (`FURN`)
such as chairs, beds, forges and chopping blocks, and idle markers
(`IDLM`), spots where they stand and do something. Many are invisible in
the game: a lean marker against a ship's railing or a sit marker on the
ground is furniture whose model is a marker. Mods that give a town's
people new routines add hundreds of them.

The 3D views did not show them, as markers have no model to draw (ADR
0038). ADR 0059 only names them as "gone" when what they stand on is
hidden. The user wants to remove such places or move them, so that
characters stop doing the thing, or do it somewhere else, and wants to be
offered a way out when a decision hides what they stand on: on JK's ship
at the harbour of Tamriel 6,26, eight lean and sit markers stand on the
deck.

How the game finds them:

- **A linked reference.** A character (`ACHR`) or another placed object
  names the place in `XLKR`, and a routine says "go to my linked
  reference". Places linked one to the next form a chain characters walk
  along, as a ship's guard does from railing to railing.
- **A routine's place or target.** An AI package names the placed object
  as where to go or what to use (`PLDT`, `PLD2`, `PTDA` of type 0).
- **By kind, near a point.** Most routines (sandbox, sit, sleep, eat) look
  for any furniture or idle marker of a kind near a place. Such a routine
  does not name the object at all.
- **A quest alias** with the place as its forced reference, or a
  location's special objects (`LCSR`).

ADR 0046 already reads the first, second and fourth as ties of placed
objects.

## Decision

1. **The places are read with the objects of a place.** The game crate
   reads, once per setup, every furniture and idle marker base object with
   its keywords (`KWDA`, naming `KYWD` records) and sorts it by what
   characters do there: sit, lean, sleep, chop wood, mine, smith, craft,
   cook, pray, other work, stand, other furniture
   (`mallet_core::action_places::ActionKind`). It sorts by the words of
   the base's internal name, its keywords' internal names and its model's
   file name, the most telling first (a `CraftingAlchemyWorkbench` is
   crafting, not smithing; a `WorkBench` is smithing, not sitting).
   `AreaObjects` gains `actions`: every placed furniture and idle marker
   around the point or in the cell, drawn or not, with its kind, the plugin
   that placed it and the one whose version the game uses, its base's
   internal name, where it stands and the way it faces.
2. **The 3D views draw an icon over each place** (MapView3D,
   InteriorView, CompareStage3D), by what characters do there, with a tick
   the way they face; faint and struck through where the patch hides it,
   where its move puts it with a dashed line from where it stood. A switch
   in each view turns the icons off. A click on an icon picks the place as
   an object, drawn or not, and its card says what it is, the mod that
   placed it, **who uses it** from its ties (ADR 0046): a character linked
   to it, a chain of linked places, a routine, a quest, a location's role,
   a script, or, with no tie, that characters whose routine looks for such
   a place nearby may use it. It also says **whether characters reach
   it**, where it stands or where its move puts it.
3. **Reach of single spots.** `mallet_analysis::reachability::reach_spots`
   tells for points, over the paths of a place (the same graph as ADR
   0059), whether a path characters reach leads there (within the same
   three and a half metres), whether it stands on paths no way leads to,
   or whether no path is near; and the nearest point within about 30
   metres on paths characters reach. The engine command
   `spots_reach(place, centre, spots, hidden)` reads the cells around the
   spots and cuts the paths over `hidden` out first (ADR 0052), so that a
   check can be made as if the patch hid those objects. The card counts
   the objects nearby that the patch hides.
4. **Removing is hiding.** A place is hidden through the patch like any
   placed object (ADR 0025, 0046): Base Object Swapper puts the invisible
   marker in its place, so the reference stays, with its links, but is no
   longer furniture. The card says honestly what follows: "Characters stop
   leaning here. Their routine goes on elsewhere, or they stand idle." What
   that means by the way the place is found:
   - **linked reference or routine's target**: the routine still sends its
     characters to the spot, where nothing can be used; the game gives up
     the routine's step and goes on with the next, or they stand there.
     The tie stays "careful" (ADR 0046) and is named;
   - **by kind**: the routine finds another such place nearby, or none and
     goes on; nothing names this one;
   - **quest alias**: "locked", as ADR 0046 decides: neither hidden nor
     moved;
   - **a location's role**: careful; what the location's scripts and
     quests do with the role is not read.
5. **Moving is the plugin of moves** (ADR 0060). Furniture and idle
   markers are placed objects (`REFR`) like any other; the move turns them
   about the upright axis with the turn buttons, and the icon's tick shows
   the way characters will face. A linked reference follows the object to
   its new place. The collision check by boxes is left out for invisible
   markers, which collide with nothing; instead the card says "Reachable
   at the new place" or "Not reachable there: the nearest path is N m
   away", asked again after each nudge.
6. **Following hiding.** Under the note of what hiding breaks (ADR 0059),
   in a collision's spot panel and in the guided review, the furniture and
   idle markers that would lose what they stood on are listed, each with
   *Onto the ground*, a move to the nearest point of the paths characters
   still reach once the objects are hidden, keeping the way it faces, and
   *Hide too*; and both for all of them at once. A place a quest needs is
   left alone and said so. *Back to where it was* undoes either.

## Consequences

- On the user's setup, Tamriel around 6,26 (the harbour), the area holds
  38 places: JK's Skyrim's seven lean markers and a seat on its ship,
  Immersive Citizens' four beds, the game's mining, smithing and work
  spots. Hiding the ship's hull (`JKs Skyrim.esp:0x1C14`) loses eight
  lean and sit markers and the two doors (ADR 0059). With the hull hidden
  and its paths cut out, the nearest ground characters reach lies 3 to 11
  metres from each marker, on the pier. Their ties: each is linked to the
  next (a chain from railing to railing), and one is linked from the
  "Ship's Guard": a guard's patrol, which would walk to the pier with them.
  Both doors are locked (they lead into the ship).
- In Whiterun (`WhiterunWorld`, the market and around), 205 places, 38 of
  them Immersive Citizens': places to lean, sit, pray, chop wood and work,
  most of them invisible markers. Of 80 places Immersive Citizens
  places or changes, 18 have no tie (found by kind), 32 are linked from
  other places or characters, 24 are named by a routine, 7 have a role in a
  location; all 60 asked about are reached.
- The first area of a setup reads every furniture, idle marker and keyword
  once more (well under a second on the user's setup); a reach check of a
  few spots reads the paths of their cells, one to two seconds.
- The sorting by words is a guess, honest about it: a place it cannot sort
  is "a spot to stand" (idle markers) or "furniture characters use"; the
  base's internal name is shown alongside. The animations idle markers
  play (`IDLA`) are not read.
- Moving a place does not move the paths (ADR 0060): a place moved off the
  paths is said to be unreachable, and the offer onto the ground puts it on
  them. A place moved onto the ground keeps its facing; a lean marker may
  then face a railing that is gone, which the card asks the user to check
  in the 3D view.
- The reach of a spot is checked with the paths over hidden objects cut
  out, not repaired (ADR 0058); a repair that lays a floor under a hidden
  deck would reach more.
- A map-wide view of paths and places (ADR 0071) uses the same
  `ActionPlace` and `ActionKind`.

## Alternatives considered

- **Draw markers as their editor models.** The game's marker models are
  editor-only and many are missing from the archives; an icon by kind says
  more than a grey arrow.
- **Remove through a plugin that deletes the reference.** Deleting a
  placed object that others link to can crash the game; Base Object
  Swapper's swap keeps the reference and its links, and is undone by
  removing the patch.
- **Move what links to a place along with it.** A character's own place
  in the world is where they start, not where they work; moving the place
  is enough for the routine to follow.
- **Name the characters who use a place found by kind.** Which character
  uses which bed or seat is spread over routines, factions and scripts and
  decided while the game runs.
