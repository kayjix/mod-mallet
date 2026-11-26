# 0055 A magic wand for what belongs together, and one column per mod

- Status: accepted, amended 2026-11-25 (the levels take a structure and a building)
- Date: 2026-11-19
- Changes: ADR 0040 (what the card of a picked object takes along, and how
  a spot's choices are laid out)

## Context

A mod builds a structure from many placed objects: a pavilion is a roof on
four posts, a library is a row of shelves with books in them. Who hides
such a structure by hand wants all of it at once, and nothing more.

The card of an object picked in a 3D view first offered "the parts that
hang together with it" (boxes of the same mod touching), then a chain of
the same mod's parts through a fixed reach, with that reach drawn as a
see-through hull around every part. The user tested the chain on a real
setup, at Candlehearth Hall in Windhelm, and turned it down:

- The hulls appeared as soon as an object was picked, a cloud around every
  part, and looked like a mess.
- The chain ran over 220 parts to the far side of the building. A
  building's walls touch each other, so no reach is small enough to stop
  it, and it looked as if far too much would go.

What the user asked for instead is a magic wand, as in an image editor:
off at first, so that a click takes the object alone, and a level that
takes more the higher it is set, up to a limit that still makes sense.
Picking a shelf should take the books in it, then the other shelves of
the row, whether they touch or not, but not the arch beside it or the wall
behind.

At the same time the panel over the stage had too many buttons, pressed to
the left, and did not show that a spot is about two mods.

## Decision

1. **The magic wand has a level from 0 to 5**, set in a bar of tools along
   the top of every 3D view (map, a collision up close, interiors). It is
   off at first; the last level set is kept while the app runs.
   - 0: the object alone.
   - 1: with what lies on or in it.
   - 2: the same model right beside it (boxes within 20 cm), link by link.
   - 3: its kind in a row: the same family of models (the file name
     without the variant's number, so `Shelf01` and `Shelf02`), gaps up to
     2 m.
   - 4: the whole structure: every part of the same sort (built, clutter
     or nature, as in ADR 0050) within 20 cm, link by link, but no part
     more than four times as long as the picked one (7 m are always
     allowed): a road or a slab of ground of the same mod touches half a
     town, and hiding it tears holes.
   - 5: the structure with gaps up to 2 m.
2. **Only the same mod's parts are links.** A part belongs to the mod that
   placed it. Objects of other mods and of the game are passed over.
3. **What lies on or in a link comes along**, from level 1 on: decor of
   the mod (clutter, small plants) touching it, and any part of the mod
   whose middle is inside it. The wand does not grow on from these. Decor
   counts also where the mod's version of the game's object wins (the
   books JK moved onto its shelf, ADR 0050).
4. **There is a limit.** The wand reaches at most about 30 m from the
   picked object and chains at most 300 parts.
5. **Nothing is drawn but the parts.** The picked object is lit as before,
   what the wand took along about half as much. No hulls. The card says
   how many parts the wand took at which level and hides or shows them
   together; parts the game cannot do without stay.
   - **Both ways.** The wand works for hidden parts as for visible ones.
     It walks through hidden parts too (the views keep them as ghosts or
     left out, the wand knows them either way), and the card tells the
     selection apart by what the patch does with each part. From a hidden
     object it offers "Show these N again" for the hidden parts; from a
     visible one "Hide these N" for the visible parts. A selection of both
     says how many of each it holds and offers both ways, the picked
     object's own first. Hidden parts of the selection are drawn as lit
     ghosts, also where the view leaves hidden parts out, so that they can
     be seen before they are shown again.
   - **Locating.** Wherever parts are named or chosen (each side of a
     spot, the parts the game needs, the wand's selection) a small button
     makes exactly those parts glow in the 3D view for about three
     seconds, also where hidden, and turns the camera to them. The view
     that is open registers itself (`registerLocator` in `locate.tsx`);
     `AreaLayer.pulse` does the glowing.
   - **Conflicts.** The card of a picked object lists the open collisions
     and door spots it is part of, each with a link to decide it in the
     worklist, and says which collisions its being hidden decided (a spot
     with no choice of its own is decided by a part hidden on its own).
     Door spots are only listed and linked: the door panel decides them.
6. **One column per mod.** The panel over a collision up close has, from
   top to bottom:
   - a bar to go through the spots: the spot and where it is, previous,
     next, next open, going on after deciding, pinning to solve later, and
     showing it on the map;
   - three columns: on the left the first mod, on the right the second,
     each with hiding its part here, in the passage and its objects
     around, and with the preview; in the middle what concerns both:
     keeping both, how often the two collide and deciding the pair as a
     whole;
   - the line that warns where characters walk (ADR 0051);
   - the bar of tools on the view: the magic wand, how far "around"
     reaches, and the marks with what their colours mean.
   In a narrow window the two mods' columns stand side by side and the
   middle one goes below them; narrower still, everything is one column.
7. **The height slider of the relief is gone.** The 3D views draw the
   game's own world (ADR 0037, 0041), whose heights are not to be raised.

## Consequences

- At Candlehearth Hall the wand at level 4 still takes the whole building,
  as it is one structure; the levels below it take a shelf with its books,
  or a row of shelves. Which level fits is the user's call, and the card
  shows the count before anything is hidden.
- The family of a model is a guess from its file name. Pieces named
  differently (`WallCorner`, `WallSide`) are not one family; level 4 takes
  them as a structure.
- The magic wand only knows parts whose models are read; just after the
  view opens, parts far from the camera may still be missing.
- The review's structures and segments (ADR 0050) keep growing by touch;
  only the card of a picked object uses the wand.
- At Hollyfrost Farm the old chain ran from a small wall end over the
  road, slabs of ground and the roofs of the farm. Rocks and land are
  nature and never links from something built; roads and slabs are built,
  and the rule of size keeps them out.

## Alternatives considered

- **A chain through a fixed reach, drawn as hulls** (this ADR's first
  form). Turned down after testing on a real setup, for the reasons above.
- **A smaller reach, a radius around the picked part, or a number of
  steps.** A building's walls touch, so a smaller reach does not help; a
  radius or a number of steps cuts through a structure at a random point.
  The levels decide by what a part is instead.
- **Separate settings** for likeness, connection and contents. The user
  wanted one control that does not overwhelm.

## Amendment 2026-11-25: the levels take a structure and a building

The user tested the wand in Whiterun, where open cities skyrim and JKs
Skyrim both build at the gate. They picked a hidden scaffold part of JK's
(`WRScafStr02`) to bring back its scaffold guard tower. Levels 1 to 3 did
what their names say, and level 3 took 10 parts. Level 4, "the whole
structure", jumped to 130 parts: 74 hidden and 56 visible, across the
neighbouring buildings. Walls, stairs and fences touch from house to
house, so a chain by touch runs on until the wand's reach ends.

### Decision

1. **The wand has six levels.** Levels 1 to 3 stay as they were.
   - 4: **this structure.** The parts of the same set of models (the folder
     a model lies in, such as Whiterun's scaffolds) that are built into one
     another or stand on one another, with what lies on or in them.
   - 5: **the whole building**, of any set of models, the same way.
   - 6: **everything joined to it**: the old level 4, which chains by touch
     through what is of the same sort. It also holds what levels 4 and 5
     take. The old level 5 (the structure with gaps of 2 m) is gone; it only
     ever took more of the neighbours.

   A sixth level is worth its pip because "the structure" and "the
   building" are both things the user asks for. In Whiterun, the scaffold
   tower is one set of models; JK's gatehouse is that tower with a guard
   house, a stone gate and pieces of the city wall.
2. **Built together, not touching.** Two parts are built together where a
   quarter of one lies in the other's surfaces, or where one stands on the
   other: its lowest surfaces lie on the other's top, and its middle is
   clearly higher. This is the rule the passages use (ADR 0072). A part
   that stands beside another at the same height is not built together
   with it. Neither is a part on something at least 7 m long and eight
   times longer than itself, which is the ground under it: a city wall, a
   cliff, a slab.
3. **Surfaces, not boxes.** When a model is read, its box is cut into
   6 × 6 × 6 cells, and the cells its triangles pass through are noted. A
   house's box is hollow inside, and a stair's box is empty above the
   steps. Being built together and lying on or in something are measured
   on these cells, so a tower does not take what stands in its empty
   middle or merely within its box.
4. **A building ends where it hangs on the rest by little.** From the
   picked part, the wand takes the part with the largest share of its
   links into what it has taken, one after another. Once at most a tenth
   of the links of what it took lead out of it, it goes on only while one
   more part ties it more loosely still, and only with a part that has more
   than half of its links into it. A part with exactly half joins if it
   stands on what is taken. So a stair between two houses, or the next
   piece of a city wall a tower is set into, stays.
5. **The mod and the state.** Only the picked part's mod's parts are links,
   as before (for an object of the game, the mod whose version wins).
   What the card offers is unchanged: from a hidden part it shows the hidden
   parts again, and from a visible part it hides the visible ones.
6. **Preview.** Pointing at a pip lights what that level would take in the
   3D view, and a note below the pips says how many parts that is and how
   many of them are hidden. The level set and the card stay as they are
   until a pip is clicked.

### Consequences

- In Whiterun, from `WRScafStr02` (JKs Skyrim.esp), read from the real
  setup: level 3 takes 10 parts, level 4 takes 30 parts (all hidden: the
  seven scaffold parts with the barrels, crates, sacks and ropes on them,
  and the stone columns and arches standing inside the tower), level 5
  takes 72 parts (69 hidden, 3 visible: JK's gatehouse), and level 6 takes
  123 parts (70 hidden, 53 visible). Before, level 4 took 127 parts in the
  same reading (130 in the app).
- The set of models is a guess from the folder. A mod that builds one
  house from two sets gets the part's set at level 4 and the whole house
  at level 5.
- The cells cost one pass over a model's triangles when it is read.
  Growing a building in Whiterun takes about 25 ms.
- The wand's logic now lives in `wand-chain.ts` and the geometry of boxes
  in `boxes.ts`. Both are pure, and `test/wand-chain.test.ts` tests them
  with scenes of made-up boxes.
