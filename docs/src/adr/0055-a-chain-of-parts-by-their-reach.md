# 0055 A chain of parts by their reach

- Status: accepted
- Date: 2026-11-19
- Changes: ADR 0040 (what the card of a picked object takes along)

## Context

A mod builds a structure from many placed objects: a pavilion is a roof on
four posts, a bridge is planks on pillars. Who hides such a structure by
hand wants all of it at once. Until now the card of an object picked in a
3D view offered "the parts that hang together with it": parts of the same
mod whose model boxes, turned and placed, come within a few centimetres of
each other, with a slider for that gap. On top of that, what a structure
grows through depended on what the part was (built, clutter, decor), as
the review's structures do (ADR 0050).

The user was not satisfied with that grouping. Their reference case is a
pavilion around a signpost outside Whiterun, with another mod's banner
pole through its roof: picking one post should take the other posts and
the roof along, and nothing of the other mod. And the grouping could not be
seen: nothing in the view showed how far a part reaches, only which parts
were found after pressing "Mark them".

## Decision

1. **Every part reaches the same distance beyond its box.** The distance is
   fixed for all parts and set with a slider on the card, from 0 to 5 m,
   50 cm at first; the last setting is kept while the app runs.
2. **Parts chain through each other's reach.** Two parts of the same mod
   are linked where one's box comes within the other's reach (the boxes
   turned and placed as the game places them). The chain is every part
   reached from the picked one link by link. A part belongs to the mod that
   placed it, or to the mod whose version of it wins.
3. **Only other mods stop a chain.** Objects of other mods and of the game
   are passed over: they neither join nor break the chain. Nothing else
   stops it: no kind of part, no size and no count. It is bounded by what
   the view has read around its point.
4. **The chain is seen at once.** Picking an object lights it up as before;
   the rest of its chain lights up about half as much. Around every part of
   the chain its reach is drawn as a see-through hull (its box grown by the
   distance, with rounded edges), the picked part's stronger. A checkbox on
   the card turns the hulls off. Parts the patch hides keep their ghost
   look.
5. **Everywhere an object is picked in 3D.** On the map, up close at a
   collision (for the collision's own parts too) and in interiors. The
   card hides or shows the whole chain, as it did with the parts that hung
   together; parts the game cannot do without stay.

The chain is computed in the desktop app, where the models' boxes are: the
engine never reads the models' sizes for a whole place.

## Consequences

- The pavilion is one chain: the posts stand a hand below the roof, which
  the reach bridges, and the banner pole of the other mod stays out.
- A larger reach can take a whole town of one mod along, trees and rocks
  of that mod included. The hulls show it, and the slider takes it back.
- The review's structures and segments (ADR 0050) keep growing by touch
  and by kind; a click there still toggles one part. Only the card of a
  picked object uses the chain.
- The chain only knows parts whose models are read; parts far from the
  camera may still be missing just after the view opens.

## Alternatives considered

- **A reach by each part's size.** A roof would reach far, a candle hardly.
  The user preferred one distance for all, which can be seen and set.
- **Stopping at nature, at very large parts, or after a number of parts.**
  Offered; the user wanted only other mods to count.
- **A ring on the ground instead of a hull.** Flat rings do not show a roof
  reaching down to its posts.
