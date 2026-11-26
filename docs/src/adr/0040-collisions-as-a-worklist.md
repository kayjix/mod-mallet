# 0040 Collisions as a worklist, decided spot by spot

- Status: accepted; changed by [ADR 0046](0046-why-the-game-relies-on-an-object.md): which objects can be hidden on their own; [ADR 0072](0072-passages-where-two-mods-build-the-same.md): what a passage is; [ADR 0048](0048-one-job-per-page.md): deciding a pair of mods; [ADR 0050](0050-a-guided-review-through-the-homes.md): also structure by structure; [ADR 0055](0055-a-magic-wand-for-what-belongs-together.md): what the card of a picked object takes along; [ADR 0067](0067-stacked-doors-are-merged-into-one.md): doors of one group are a spot of their own
- Date: 2026-11-17
- Changes: ADR 0025 (what the patch hides is chosen per collision finding),
  ADR 0023 and ADR 0038 (the order in which a placed object is turned)

## Context

A collision finding stands for a pair of mods: every spot where objects of
the two collide. In a real setup one such finding holds hundreds of spots
(556 in the largest), and they are not all alike. At one spot the first
mod's fence should go, at another the second mod's tower. Choosing one mod
per finding, as ADR 0025 did, makes the user decide for all of them at
once without having seen them.

With the objects drawn up close (ADR 0038) and each spot known as a
duplicate, an overlap or a touch (ADR 0039), each spot can be looked at
and decided where it is. Trying that on a real setup showed more:

- Two mods often build the same thing twice, a bridge or a scaffold, each
  out of several parts. Their parts cross each other over metres without
  one standing inside the other, so ADR 0039 calls them touches, and they
  come as many spots close together.
- Seeing both parts lit up does not tell what the place looks like with
  one of them gone.
- Objects that no collision names (a third part of the same structure, or
  an object of the game) sometimes have to go as well.
- Placed objects turned about more than one axis were drawn, and compared,
  turned in the wrong order: tilted rocks stood across the view, beams
  hung out of their scaffold.

## Decision

1. **The collisions view has two tabs.** *Spots* fills the window: a
   worklist on the left, the spot in 3D on the right, nothing below to
   scroll to. *Mod against mod* lists the pairs of mods with their spots,
   the most first, to decide for a whole pair or to go through its spots,
   followed by the duels and the remaining findings.
2. **The worklist** groups the spots by the place of the game's map they
   are nearest (or the interior they are in), or by the pair of mods.
   Its kinds are duplicates, overlaps, *deep crossings* (touches over 2 m
   or more) and touches; the first three are shown at first. Groups with
   the most serious spots come first.
3. **Passages**: spots of the same two mods within 14 m of each other, in
   the same worldspace or interior, belong together. One decision hides
   one mod's parts in the whole passage; another also hides every other
   object of that mod around the passage, as far as the area read in 3D
   shows it.
4. **Looking before deciding.** Picking a spot flies the 3D view close to
   it, its objects lit up and what the patch would hide drawn as a red
   ghost. An eye beside each side leaves that side out without deciding.
   *Markings* off shows the place as the game would: nothing tinted, what
   is hidden left out. Deciding stays on the spot unless the user asks to
   go on to the next open one. Arrow keys go through the spots, digits
   decide, V switches the markings.
5. **A spot can have a choice of its own**, named by the objects in it
   (so the name survives a new load order, ADR 0007); it wins over its
   finding's choice. Choosing for a whole finding drops the spots' own
   choices.
6. **Any object can be hidden on its own** from its card in the 3D view,
   kept among the patch's choices by its plugin and number. Objects the
   game relies on are not offered: persistent ones (other things refer to
   them) and those that are part of the distant view. Objects of the game
   itself are offered with a warning that a quest may need them. The
   installed patch is read back the same way: what no collision names was
   hidden on its own. The patch view lists such objects to show again.
   A picked object also finds the parts of the same plugin that hang
   together with it (their models' spheres meet, as far as the area
   shows them, a bridge built of planks and posts); they can be marked
   and hidden at once. The map's 3D view offers the same, and a part of a
   collision opens on the collisions' page.
7. **The order of turning**: a placed object is turned clockwise about
   the world's z, then y, then x. The 3D view and the comparison of
   models (ADR 0039) use the same order.
8. The map tells the kinds apart: duplicates red, overlaps orange, touches
   grey and smaller. A collision on the map, and an object duel in the
   comparison view, open in the worklist.
9. Nothing changes in the game until the user creates and installs the
   patch on the patch view, as before.

## Consequences

- With the corrected order, some collisions of tilted objects come out
  differently in the next scan: spots appear and disappear, and some
  touches become overlaps or the other way round.
- A spot whose objects are not named (a few, from older readings) is not
  in the worklist; its finding stays on the second tab.
- What counts as "around a passage" depends on the area read in 3D; the
  button appears once it is read.
- The choices are kept per game; what the installed patch hides counts
  as decided as well, and a choice kept since wins over it.
