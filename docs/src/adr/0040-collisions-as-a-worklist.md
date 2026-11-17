# 0040 Collisions as a worklist, decided spot by spot

- Status: accepted
- Date: 2026-11-17
- Changes: ADR 0025 (what the patch hides is chosen per collision finding)

## Context

A collision finding stands for a pair of mods: every spot where objects of
the two collide. In a real setup one such finding holds hundreds of spots
(556 in the largest), and they are not all alike. At one spot the first
mod's fence should go, at another the second mod's tower. Choosing one mod
per finding, as ADR 0025 did, makes the user decide for all of them at
once without having seen them.

With the objects drawn up close (ADR 0038) and each spot known as a
duplicate, an overlap or a touch (ADR 0039), each spot can be looked at
and decided where it is.

## Decision

1. **The collisions view is a worklist of spots.** Each spot whose objects
   are named is one entry, grouped by the place of the game's map it is
   nearest (or the interior it is in), or by the pair of mods. Duplicates
   come first, then overlaps by how deep they cross, then touches; groups
   with the most serious spots come first. Touches are left out of the
   list until the user asks for them. The duels between two mods
   (ADR 0029) and the remaining findings follow below.
2. **Picking a spot flies the 3D view there**, with its objects lit up and
   what the patch would hide drawn as ghosts (see-through and red). The
   user decides above the view: hide one side's objects, keep both, or
   decide for every spot of this pair of mods at once. An object clicked
   in the view can be hidden from its card. By default the view then goes
   on to the next spot not yet decided. Arrow keys go through the spots,
   digits decide.
3. **A spot can have a choice of its own.** The patch's choices hold, as
   before, one mod per finding, and also one per spot, named by the
   objects in it (so the name survives a new load order, ADR 0007). A
   spot's own choice wins over its finding's. Choosing for a whole finding
   drops the spots' own choices. The installed patch is read back the
   same way: per finding the side it hides most of, and per spot where it
   hides something else.
4. **The map tells the kinds apart:** duplicates red, overlaps orange,
   touches grey and smaller.
5. Nothing changes in the game until the user creates and installs the
   patch on the patch view, as before.

## Consequences

- A spot whose objects are not named (a few, from older readings) is not
  in the worklist; its finding stays below.
- The choices still live only in the window and are lost when it closes,
  unless the patch was installed: the installed patch is read back.
