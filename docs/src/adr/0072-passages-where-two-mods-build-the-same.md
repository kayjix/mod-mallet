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
   - a side never grows into a wide flat part under the one it comes from
     (a floor, a slab of ground, a road at least 7 m wide): it carries more
     than this structure.
   - *What is structurally part of a version comes along* (added
     2026-11-22, after the stable at Markarth: hiding JK's version left
     pieces of its roof in the air). Each side then takes, link by link,
     the same mod's parts that are embedded in what it takes (a quarter or
     more inside it) or rest on it (a point of the underside on or in it,
     and its middle clearly higher: a roof on the walls, a trim in the
     roof, a ridge on top), whether or not the other mod comes near them.
     A part goes only when at least half of what holds it goes, so a roof
     shared with the next building stays there. A part beside another at
     the same height is not part of it (the next building's wall). Nothing
     rests on a wide flat part at the bottom of the passage, and a part 7 m
     long or more carries nothing an eighth of its length or less: a cliff
     or a slab is the ground under barrels and stalls, not their structure.
     A part held by a single other part is at most twice as long as it when
     set into it, and at most half as long when only resting on it (a
     barrel on a walkway comes along, a house on it does not). A part is at
     most four times as long as what holds it (unless shorter than 7 m),
     and nothing is taken farther than about 6 m from the passage's box.
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

*Amended 2026-11-22:* Map collisions no longer lists the passages above
the spots or shows the three views; they laid out badly there and drew
three scenes at once. The worklist groups by *Region*, *Mod pair* or
*Passage* instead. Grouped by passage, each passage is a group with its
spots, which stay decidable one by one; picking the passage's name frames
it in the page's one 3D view. A switch in that view shows the place now or
with one mod's version only, and the panel's first line keeps A's version,
B's version or both for the whole passage, leaving parts the game ties in.
A spot inside a passage links to it. The three views side by side stay in
the review's collisions step, as decided in point 4.

## Consequences

- On the reference setup 46 passages hold 1478 of 1599 spots. Sky Haven
  Temple's forge is one passage of 155 and 158 parts with 117 twins, in a
  box of about 43 × 50 × 22 m; the Markarth bridge is one of 63 parts of
  JK's Markarth Outskirts and 83 of Lux Via in about 51 × 25 × 17 m, with a
  second, smaller one of 20 and 10 parts at the town's gate.
- With what is structurally part of a version taken along, the stable at
  Markarth (Immersive Citizens against JK's Markarth Outskirts) grows from
  57 of JK's parts to 85, the second half of the stable, its chimney, walls
  and what stands inside among them. Some passages grow much more, as a
  whole building of one mod now comes with its walls: at Nightgate Inn 243
  of JK's parts against 6 of Cutting Room Floor, at the stable's other
  neighbour (arnima) 194 against 6. The suggestion by size says to keep the
  building there; a passage is still decided only by the user.
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
