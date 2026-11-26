# 0080 The collisions step is decided by side, with the pictures

- Status: accepted; changed by [ADR 0083](0083-step-by-step-says-what-is-at-stake.md): how the sections of a place and their parts are laid out
- Date: 2026-11-22
- Changes: ADR 0050 (a guided review through the homes): the collisions step
  is laid out by side, with the pictures

## Context

The collisions step of "Go through it" (ADR 0050) shows a passage (ADR
0072) in three views: now, only the first mod, only the second. A walk
through a real setup (JK's Whiterun's Outskirts and Lux Via at Honningbrew
Meadery, passage 4 of 90) found the views good and everything around them
too wordy:

- The step repeated its number and title, which the row of steps above
  already shows, and the two buttons for every place sat on a line of
  their own with a sentence of explanation.
- What keeping a side does was a paragraph per side: "Keep JK's Whiterun's
  Outskirts's here: 111 of 117 parts of Lux Via can go. What stands on it
  goes with it: TreeFloraHangingMoss01 ×6 … 4 parts are tied into the
  game…". Nothing in it could be seen in the views.
- The lists of objects one by one named every plank on its own line, could
  hide only one at a time, and could not show where an object is.
- After a decision, parts of the mod given up that the game needs, that
  characters walk on or that were kept by hand still stood inside the other
  mod's, and the step moved on without saying so.

People who install mods read pictures faster than sentences.

## Decision

- The step has no head of its own. "Measure open places (N)" and "Take
  suggestions (N)" sit at the right of the row of steps, their explanation
  behind an ⓘ. Under the steps: previous place, "Passage 4 of 90 · 9 of 90
  decided", next place; then the pair of mods, where, and "Open in Map
  collisions".
- A passage's summary is one line; why one version has to go is behind its
  ⓘ.
- Pinning the place and undo are tools of the views' bar (ADR 0077). What
  the colours and marks mean is one line under the views, with the other
  mods that built there.
- Each mod has a column in its colour of the views (blue and orange,
  Okabe–Ito): its keep button; what keeping it does to the other mod in a
  few words ("111 of 117 Lux Via parts go") with "Show these"; its notes as
  badges (tied in, walked on, stay, resting on it, the game's own) that
  open to say why, with "Show which" and "Hide anyway"; and "Hide single
  parts of this mod here", which opens its parts as a list.
- "Show these" and "Show which" show exactly those parts in the views:
  they keep their looks, everything else is drawn dark and grey, and the
  camera frames them, from above when there are more than 24, so that a
  group reads as one. A note over the views says what is shown and leads
  back to everything.
- A list of parts puts parts of one name in one row with a count, opened
  one by one where wanted. Rows are picked like files (click, Ctrl, Shift;
  a name's row picks all of that name) and hidden or shown again together.
  Each row locates its parts (they glow, the camera stays, an arrow at the
  edge points to them, ADR 0055); pointing at a row, or picking it, lights
  its parts in the views.
- After a decision, the mod given up may still collide: at the place's
  spots, its parts left where a part of the other is left too; in a
  passage, its parts left in the sections given up and among the passage's
  other parts. The step then says so in one line ("12 Water Mod parts
  remain and still collide here") and stays at the place, with "Keep both
  for those", "Hide them anyway" (the parts that can be hidden) and "Show
  them". "Keep both for those" changes no choice, since those parts are
  shown already: the review remembers per game that the user saw them,
  and the line goes.
- The review remembers the place it shows. "Open in Map collisions" opens
  the same passage in Map collisions' one view, as a place in the window's
  history (ADR 0070), so that the back button comes back to it.

## Consequences

- The views carry what the sentences said; the texts left are short
  enough to translate and to read at a glance.
- The 3D layer has one more look, dimmed, for objects in the background.
- Parts left on purpose are not choices of a patch: they change nothing
  in the patch. They were first remembered in the window's own storage, so
  that another machine asked again; they are now kept in the settings (see
  the amendment below).
- More than two mods at a place stay a fallback: the others are named in
  the line under the views and can be picked out there; columns are for
  the two mods a passage is about.

## Alternatives considered

- **Keep both for those as a choice of the spots.** Setting the spots
  where parts are left to keep both, with the rest hidden one by one,
  marks those sections as kept in the report, but a passage's parts away
  from its spots have no spot to set, and the choices would grow for no
  change in the game.
- **A column for every mod at the place.** A passage is a pair by
  construction; a third column for a mod with a handful of objects would
  take the room of the views.

## Amendment 2026-11-26: parts left on purpose are kept with the decisions

Kept in the window's storage, "Keep both for those" was asked again on
another machine, after the window's storage was cleared, or after a new
installation. The parts left on purpose now go into the user's book of
patches (ADR 0070) beside the pins, as `leftOnPurpose`, by the object's
key. Like a pin, the list is no decision and belongs to no patch: it is
not written into a patch and does not change what any patch hides. Every
page reads it from the one snapshot of the decisions (ADR 0081). The
review moves the parts it kept in the window's storage into the book the
next time it is open, and forgets them there.
