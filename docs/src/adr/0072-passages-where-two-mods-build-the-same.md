# 0072 Passages: where two mods build the same thing, decided as one

- Status: accepted
- Date: 2026-11-22
- Changes: ADR 0040 (what a passage is), ADR 0050 (what the review's places
  are and in which order they come)

## Context

The worklist of ADR 0040 shows one spot per pair of colliding parts, and the
review of ADR 0050 groups the spots of two mods within about 30 m and grows
each mod's structure from them. The magic wand of ADR 0055 takes more of a
structure by level. On the reference setup that is still the wrong work in
two places:

- At Sky Haven Temple's forge, *JK's Sky Haven Temple* and *JKs Skyrim*
  build the same complex twice: about 300 parts each, a third of them the
  same model at the same spot, close to and inside each other, at about 700
  spots. Going spot by spot, or tuning the wand or the radius until all of
  it is caught, is work the app can do. The user decides one thing: whose
  version stays. Keeping both loads the same models twice in the same
  place.
- At Markarth, *JK's Markarth Outskirts* and *Lux Via* each build the bridge,
  differently. Both mods reach far beyond it (Lux Via over all of Skyrim).
  Neither is to go; only the bridge is to be decided, one way or the other.
  A structure grown from the spots runs on into the town's walls and the
  roads, so the decision must stop where the two versions part.

## Decision

1. **A passage** is where two mods build the same thing: their colliding
   parts, grown through each mod's own structure, but only as far as the
   other mod's parts reach. The scan finds them (`mallet-analysis`,
   `objects/passages.rs`), per pair of plugins and worldspace or cell:
   - it starts from the collisions of the pair (duplicates, overlaps, deep
     and other touches, ADR 0039);
   - a side grows through its own parts whose boxes, turned and placed as
     the game places them, come within about 11 cm of a part already in,
     never through rocks, trees or the landscape;
   - a part grown so joins only where it comes within about 90 cm of the
     other side's parts in the passage: there the two versions stand in
     each other or nearly coincide. It joins no farther than about 30 m
     from a collision of the two;
   - what hangs together (through a side's own structure, through coming
     near the other side, or through a collision) is one passage. One with
     fewer than two spots, fewer than two parts on a side or fewer than five
     parts in all is a single spot, not a passage.
2. **Each passage is summed up**: the two plugins, each side's parts, the
   spots in it, the *twins* (parts with the same model as one of the other
   side within about 17 cm), how much of each side's room lies inside the
   other's parts, the room each side takes, and how serious it is:
   *built twice* (at least two twins, and at least one in five of the
   smaller side's parts), *inside each other* (a quarter or more of the
   side with less room inside the other) or *across each other*. Built
   twice comes first, then inside each other, then the most parts. The
   window names a passage by the place of the map it is nearest, or its
   interior.
3. **Passages come first.** The collisions' worklist lists them above the
   spots, each with its two mods, its parts and spots and what is decided.
   A spot that lies in a passage says so and leads to it. In the review's
   collisions step the passages are the first places, before the places of
   the remaining spots.
4. **A passage is decided as a whole**, on the review's stage (ADR 0050):
   the passage framed in three views (now, only the first mod's version,
   only the second's), each side in its colour, and *Keep A's version
   here* or *Keep B's version here*. The structures on the stage end at the
   passage: they keep only its parts and the decor of the same mod resting
   on them (clutter, small plants), so a decision hides the other side's
   parts inside the passage and never beyond it.
5. **Refining stays.** The passage's segments, its objects one by one, and
   the worklist's spots with the magic wand stay for exceptions, such as
   keeping one lantern. Parts the game needs or ties in (ADR 0046) and parts
   characters walk on (ADR 0051, 0059) are honoured as everywhere; the
   notes about paths and reach show for the passage as a whole. Parts of a
   passage that no spot's structure reaches (ingots lying on a forge) form
   one segment of their own.
6. **The suggestion by size** (ADR 0050, point 12) applies per passage: from
   the stage's measure once read, and before that from the scan's (parts
   and the room of their boxes).

## Consequences

- On the reference setup 46 passages hold 1478 of 1599 spots. Sky Haven
  Temple's forge is one passage of 155 and 158 parts with 117 twins, in a
  box of about 43 × 50 × 22 m; the Markarth bridge is one of 63 parts of
  JK's Markarth Outskirts and 83 of Lux Via in about 51 × 25 × 17 m, with a
  second, smaller one of 20 and 10 parts at the town's gate.
- The scan measures with the boxes the base objects give (their bounds),
  the stage with the models' boxes. The two can differ a little; the stage
  keeps what the scan found and adds only decor.
- The thresholds are a first guess, as the suggestion's ratios are.
- What ADR 0040 called a passage (spots of the same finding within 14 m)
  remains for spots in no passage.

## Alternatives considered

- **Growing in the window only**, from the stage's structures. They grow
  from each spot without knowing the other side, so they run on into the
  town; and the worklist would know of passages only for places already
  read in 3D.
- **A wider wand or radius.** It cuts through a structure at a random
  point, and does not know where the other mod's version ends.
