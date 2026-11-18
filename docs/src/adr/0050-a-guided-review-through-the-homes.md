# 0050 A guided review through the homes, collisions structure by structure

- Status: accepted
- Date: 2026-11-19
- Changes: ADR 0048 (every thing has one home: a second exception), ADR 0040
  (collisions decided spot by spot: also structure by structure)

## Context

ADR 0048 gave every kind of thing one home: findings on Tasks, collisions
on Collisions, duels about files on Assets. Each page is a list to work
through in any order. The user wants to be led instead: through the setup
step by step, in an order that makes sense, able to stop and to carry on
later. In their words, the steps are the installation (which mods), the
load order (does it fit what they want), the assets, the world map, and
the collisions, structure by structure.

The collisions are the heart of it. On the reference setup, two bridges,
one of Lux Via and one of JK's Whiterun Outskirts, stand inside each
other, each built of about forty parts. The worklist of ADR 0040 shows
that as dozens of spots, one per pair of touching parts; deciding them one
by one is slow and hides what the user actually decides: which bridge
stays. The user wants to see the two structures side by side, as the place
looks now, with only the one and with only the other, each mod's parts in
a colour of its own.

## Decision

1. **The review is a page of its own, *Go through it* (*Durchgehen*)**,
   in the sidebar right under the overview. The start page and the
   overview lead to it, with the step where the user stopped.
2. **Five steps, in this order:**
   1. *Installation*: the mods, how many are switched on, which have a
      newer version, and the open findings about what is installed
      (missing masters and requirements, installer options, script
      extender plugins, recommendations about whole mods).
   2. *Load order*: the findings that depend on the order of the plugins
      (LOOT's rules and messages, incompatible plugins, generated plugins
      out of date, plugins to clean, navigation meshes another plugin
      undoes).
   3. *Assets*: the duels about files that are still open, each leading
      to its comparison on the assets page.
   4. *World map*: the worldspaces with what mods add to them, each
      leading to the map.
   5. *Collisions*: one structure at a time, decided in the review itself.
3. **The review owns nothing.** It shows the homes' own rows (the task
   page's finding rows, the assets page's duel lines) and writes into the
   same state (patch choices, duel decisions, findings set aside). What is
   decided in the review shows on Collisions, Assets and Patch, and the
   other way round. This is the second exception to ADR 0048's rule that
   a thing is shown in full only at home: the review is a path through
   the homes, not a home of its own.
4. **A structure is the parts of one mod that hang together**: starting
   from the parts of a collision spot, every part of the same plugin whose
   model box (turned and placed as the game places it) meets another
   within a hand's breadth, transitively (`AreaLayer.connected`, ADR
   0040). Not the whole mod, not the whole cell.
5. **A structure is shown three times, through one camera**: the place as
   it is now (with what the patch hides left out), with only the first
   mod's structure, and with only the second's. The first mod's parts are
   blue, the second's orange (Okabe–Ito, told apart with any colour
   vision); red keeps meaning *hidden*. The markings can be switched off
   to see the place as the game shows it. Dragging turns all three views.
6. **Deciding a structure** hides all its parts that the patch may hide
   (objects of the game are never hidden), as objects hidden on their own
   (`chooseObject`), which decides every spot they are part of (ADR 0040's
   `decisionAt`). *Keep both* marks the spot as decided both ways and shows
   both structures again (`revealObjects`). *Undo* takes the parts out of
   the patch's choices. The next structure is the next spot still open:
   the spots a decided structure covered are decided with it.
7. **Progress is what is decided**, wherever it was decided: open duels,
   open spots, open findings. Steps that are only looked at (the
   installation, the load order, the map) are marked as seen, and the
   step the user was on is remembered, both in the window's storage per
   game; losing them costs a click, never a decision.

## Consequences

- The collisions can be decided where the user thinks in structures; the
  worklist stays for spots one by one and the pairs for a mod against a
  mod as a whole. Three ways now lead to the same choices; the review is
  the guided one, the other two stay the pages to look things up.
- The three views draw the area three times per frame. For a few
  thousand objects that is fluid; the models are read once.
- Finding a structure needs the models' boxes, so it is known only once
  the area's models are read; until then the view shows the spot's own
  parts.
- The assets step leads to the assets page for now. Its own three-way
  view (the first mod, the game's own file, the second mod, and which one
  wins in the load order now) uses the same stage and comes next.

## Alternatives considered

- **A mode of the overview.** The overview says how the game stands; a
  mode that walks through decisions would give it a second job.
- **Walking the worklist's spots in order.** One bridge of forty parts
  is dozens of spots. The user decides which bridge stays, once.
- **The structure as all of a mod's objects in the cell.** A mod places
  unrelated things next to each other; hiding them all with the bridge
  takes away more than the user decided.
