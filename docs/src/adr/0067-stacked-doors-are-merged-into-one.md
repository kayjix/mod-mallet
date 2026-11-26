# 0067 Stacked doors are found and merged into one

- Status: accepted; changed by [ADR 0088](0088-door-paths-re-attached-to-the-kept-door.md): the paths of merged doors are re-attached to the kept door
- Date: 2026-11-21
- Changes: ADR 0060 (the plugin of moves also leads doors elsewhere);
  ADR 0039 and ADR 0040 (doors of one group are a spot of their
  own, not a collision of two objects); ADR 0025 (what hiding a door
  leaves behind is said, and the changes it needs are planned)

## Context

Many mods add doors. Another mod often adds its own door at the same
spot, a few centimetres in front of or behind the first because its wall
is thicker or its frame moved, or a mod puts its door where the game has
one. The game shows both: one flickers through the other, the player opens
whichever is in front, and characters walk through the door their paths
name, which may be the other one. A real second door right behind the
first, a porch with an inner door, is rare.

Hiding one of the doors (ADR 0025) is often right. But a door is more
than what is seen:

- A door that leads somewhere has a teleport (`XTEL`): the door on the
  other side, where the player arrives there and which way they face. The
  door on the other side has a teleport back.
- Characters walk through a door where a navigation mesh has a door
  triangle naming it (ADR 0052 reads them).
- Quests, AI packages and scripts may name it (ADR 0046 tells which).

Whatever used the hidden door has to use the kept one: the door on the
other side has to lead to it, the door triangles have to name it.

Collisions so far compare only what mods place; doors of the game that no
mod touched are not among them, although a mod's door on top of one of
the game's is the usual case. Teleport doors are persistent, so the
worklist never offered to hide them.

## Decision

1. **Every door is read, the game's included.** A door is a placed
   object whose base is a `DOOR`, or one with a teleport. Of each the
   version the game uses is kept, with its teleport, its enable parent
   (`XESP`), whether its base has a model, and which plugins change it.
   The door triangles of the navigation meshes the game uses are kept as
   well. The game's own plugins are read for this once and kept in the
   cache until their files change (`doors-3`), like their land.
2. **Doors that are not seen do not stack.** A door switched off at the
   start, or through its enable parent, is left out: a quest that swaps
   one door for another never shows both. So is a door whose base has no
   model: mods put such a door where they take the game's away and keep
   its record for quests (Open Cities does).
3. **Doors stack** when they are in the same place, at most about 1.4 m
   apart, both seen, and either lead to the same door or room (outside:
   within about 7 m), or are turned alike (or turned round) with their
   openings over each other: sideways within the narrower door's half
   width, at most about 70 cm in depth and 45 cm in height. A mod is
   taken to be coherent within itself: two doors of the same plugin stack
   only as copies, the same door within about 11 cm turned the same way
   about every axis (within 15 degrees), and not two working ways (both
   with a door on the other side that leads back, as a ladder up and one
   down). The leaves of a cabinet, a gate of two wings or a door behind a
   door are one thing, not a stack. Doors that only open, without a
   teleport, stack only as such copies as well, whichever plugins they are
   of. Doors that stack are joined into groups. A group of the game's
   doors alone is not reported.
4. **The finding is `doors.stacked`**, one per pair of plugins that placed
   the doors (both the same where one plugin stacks its own), with a spot
   of the kind `door` per group. Its subject names the plugins' files
   only; the doors are in the spot, by plugin and number (ADR 0007).
   Spots of a group are not also reported as collisions of two objects.
5. **Each door of a group says** where it leads, whether the door there
   leads back, whether characters walk through it and through the door on
   the other side, how many things refer to it, whether it is the one in
   front (seen from where the player arrives) and how far it stands from
   the first. Two doors within 2 units in depth flicker.
6. **A plan per door.** Keeping one door means: the patch hides the
   others; every door elsewhere that led to a hidden door leads to the
   kept one (`RecordChange::Teleport`, the arrival moved by as far as the
   doors stand apart); a kept door that leads nowhere takes over the
   hidden one's way; every door triangle that named a hidden door names
   the kept one (`RecordChange::NavmeshDoor`). Whichever door is kept, the
   way through stays: it leads where the group leads, and the door there
   leads back to it. A door whose teleport names a door that is not there
   leads nowhere, as one without a teleport; the card says so. Only where
   the doors lead to places apart (`DoorGroup::apart`: not the same door,
   room or, outside, place within about 7 m) is there no merge: the game
   cannot take the player to both. Keeping one door then keeps its own way
   and only hides the others; their places are listed as lost, and the
   window says plainly that keeping both is the other choice. The suggested
   door is not one the plugin of
   another door of the group changed (it put that door aside, as Open
   Cities does with the game's Riften door); then the one whose other side
   leads back and that characters walk through on both sides, then the
   one most things refer to, then the plan with the fewest changes, then
   the one in front. It is a hint; any door can be kept.
7. **The teleports go into the plugin of moves.** A door's new teleport is
   an override of the door's winning version, copied as ADR 0060 copies a
   moved object, with its cell and worldspace, its references rewritten
   for the new masters, and only `XTEL` changed: the door on the other
   side named as the kept door, and where the player arrives moved by as
   far as the doors stand apart. A kept door without a teleport gets the
   hidden one's, placed before its position as the game's doors have it.
   The overrides are written into the same plugin as the moves, `Mod
   Mallet - Moves.esp`, and not into one of their own: a door that is both
   moved and led elsewhere has to be one override, and two plugins of Mod
   Mallet that override the same door would undo each other. A move
   therefore carries an optional door target (`ObjectMove::teleport`).
   The targets are not kept as choices of their own: they follow from the
   spot's decision when the plugin is written, so there is one source.
   The hiding of the other doors stays the Base Object Swapper patch
   (ADR 0025). Both are written and installed on the patch page, on the
   user's click.
8. **A merge is the spot's decision.** The spot's choice among the
   patch's choices keeps one door and hides the others there
   (`keep:<plugin>|<id>`, whichever plugins the others are of). Every
   place that asks what is hidden reads it: the worklist, the object card
   in the 3D view, the magic wand, the patch and the plugin of moves. The
   object card of a door the decision hides says so and offers to show it
   again; showing one door again sets the spot to keep all, and the other
   doors it hid stay hidden on their own. Hiding a door in the card, or
   with the wand, so that one door is left, merges into that one; parts of
   a spot hidden on their own with one left over count as keeping it.
   *Keep all* sets the spot to keep all and removes every hiding of its
   doors, whatever its source.
9. **The door triangles wait for the navigation mesh writer.** Pointing
   the door triangles of the paths at the kept door changes navigation
   meshes, which the writer and repair of ADR 0058 write. Until it lands,
   the plan names those changes as data (`RecordChange::NavmeshDoor`), the
   window says they come later, and characters still walk to where the
   hidden door stood, right beside the kept one.
10. **The window.** The collisions' worklist has the kind *doors*; a spot
   of doors shows them side by side with where each leads and *merge into
   this one*, and what keeping it means in sentences ("Characters who used
   the door of B now use the door of A"). It says whether the characters'
   paths lead through the kept door, and asks the check of ADR 0059
   whether hiding the others leaves every way there (`walkway_reach` with
   the hidden doors). Each card names the door by plugin and number (as
   xEdit shows it without the load order's byte) and its base, and has an
   eye at its top right: while it is on, the 3D view lights up that door
   alone and draws the others as ghosts, or leaves them out without the
   markings. What keeping a door means starts with where it then leads
   ("It now leads to Tamriel 28,27; the door there leads back to it").
   The guided review shows the same for the doors among
   a structure's spots. Merging is the spot's choice where the other doors
   are one plugin's, otherwise each hidden on its own. The patch page
   lists a led door among the moves ("leads to the door …"). Beside the
   eye, a mark makes the door blink in the view for three seconds and
   turns the view to it. Where the doors are two mods', a button opens
   all collisions of those two mods.

## Consequences

- On the user's setup (Skyrim SE, about 170 mods), 8 groups between 8
  pairs of plugins: among them Skyrim Sewers' door in the Markarth
  Warrens with a Creation Club door 6 cm in front of it that leads
  nowhere, Cutting Room Floor's Riften gate 44 cm behind JK's Skyrim's,
  and the College of Winterhold's doors of Immersive College of
  Winterhold and JK's patch for it, 54 cm apart. The others are copies
  of one door within a mod. 7 of the 8 suggested merges need nothing but
  hiding. Before rules 2 and 3 were sharpened, 73 groups were found; most
  were the game's own (a secret cabinet door and its panel), doors
  swapped by enable parents and the four leaves of one mod's cabinet.
- The first scan that reads the game's doors reads Skyrim.esm and its
  kin once more, about 16 seconds here; later scans take them from the
  cache.
- The existing finding `navmeshes.door-cut-off` says that a later plugin's
  paths no longer lead through a mod's door. Where such a door stands in
  a group, the suggestion prefers the door the paths do lead through, and
  re-attaching the door triangles to it is part of the plan.
- Quests, packages and scripts that name a hidden door are not changed;
  the 3D view's card tells them (ADR 0046).
- The thresholds are named constants and are a heuristic: a door moved by
  more than 70 cm, or turned across, is not found.

## Alternatives considered

- **Treat doors as collisions of two objects.** Collisions know only what
  mods place and pair two objects; doors need the game's doors, all
  members of a group and where each leads.
- **A plugin of its own for the doors, or a shared "Changes" plugin.** A
  door both moved and led elsewhere would need two overrides that undo
  each other; renaming the plugin of moves would strand the plugins users
  installed. One plugin, as it is named, carries both.
- **A writer of its own for the door triangles now.** The navigation mesh
  writer of ADR 0058 is under way; two writers of the same records would
  be built side by side.
- **Merge by moving the hidden door's record onto the kept one.** Changing
  a door's base and position keeps every reference to it, but replaces the
  kept mod's door with another record and breaks what names the kept one.
