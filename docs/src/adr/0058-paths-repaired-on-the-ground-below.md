# 0058 Paths over hidden objects are repaired on the ground below

- Status: accepted
- Date: 2026-11-21
- Changes: [ADR 0052](0052-paths-over-hidden-objects-are-cut-out.md) (its
  first rule, "a repair cuts out; it never adds", and its step of writing
  the meshes); [ADR 0051](0051-paths-over-hidden-objects.md) (its texts:
  "Mod Mallet cannot do it yet")

## Context

ADR 0051 finds the paths characters walk on (the triangles of navigation
meshes) that run over objects the patch hides; ADR 0052 decided to cut them
out and to check what that leaves. Cutting alone leaves characters without
a way where the object stood. The user's cases:

- At Falskaar's harbour (Tamriel 6,26) JK's Skyrim moors an Imperial ship,
  `imperialship01base` (`JKs Skyrim.esp` 0x1C14). JK's version of the
  game's mesh `skyrim.esm:102F4F`, the one the game uses, runs 109
  triangles over its deck. JK's mesh leaves a hole in the paths on the sea
  bed where the hull stands.
- JK's Sky Haven Temple (Tamriel −35,1 and −35,2) lays 11 floor pieces,
  rubble and platform stairs over the land. The installed patch hides
  them; 101 triangle-and-object pairs of the mod's own paths lie on them.

The user wants characters to reach the same places as before, by a way
around or under the removed part, never over it.

### What the game needs, and what Mod Mallet can give it

What was found out, partly by reading all 44,454 `NVNM` fields of the
user's setup (Skyrim SE 1.7.104, about 320 plugins), partly from xEdit's
definitions and the modding community's documentation:

- **The `NAVM` record.** One field, `NVNM` (version 12): vertices,
  triangles (corners, three edges, flags, cover flags), edge links to
  other meshes, door triangles, a list of triangles with cover, and a
  search grid of size × size squares listing the triangles in each, row by
  row from the south. Skyrim's layout has no waypoints (Fallout 4's
  version 15 adds them). Every field of the setup reads completely and
  writes back byte for byte (`navmesh_write`). 472 records carry other
  fields (`ONAM`, `PNAM`, `NNAM`) besides it, whose meaning is not known;
  they are copied unchanged, form IDs in them included (neither case here
  has them). 57 fields exceed 64 KiB and need an `XXXX` field in front
  when written.
- **The field's "checksum"** is not one: JK's Skyrim changes the mesh at
  the harbour and keeps the game's value. It is copied unchanged.
- **The search grid.** The Creation Kit lists a triangle in the squares it
  overlaps; listing it in every square its box touches is a superset (every
  listed square of the setup lies within it). New triangles are listed so;
  triangles taken out are listed nowhere, so the game's search for the
  triangle under a point no longer finds them.
- **Triangles run anticlockwise** seen from above (all 6,058,028 of them,
  apart from 1,026 without area). Bit 11 (`Found`) is set on almost all,
  `Water` (0x0200) on 652,271: the game's water paths lie on the sea or lake
  bed, below the water's surface, not on it.
- **Edge links** have three kinds in the setup (0 to step across, 1 and 2
  up and down ledges). A link names a mesh and a triangle by number; both
  meshes must link to each other, which the Creation Kit does when it
  "finalizes" meshes. A link Mod Mallet adds is of kind 0 and is written on
  both sides.
- **The navigation info map** (`NAVI`, one record, with an `NVMI` entry per
  mesh: where it lies, its doors, the meshes it joins, islands) is, as far
  as the community documents it, what the game plans long ways with. A
  plugin's `NAVI` holds the entries of its meshes, and the game merges them
  (not checked in the game here). Mod Mallet does not write it: the repair
  joins only meshes that were joined before or lie in the same place, and
  keeps their doors, except doors whose triangles are taken out. A stale
  entry lets a character plan a way the meshes then refuse, as ADR 0052
  already accepted.
- **Cover** (the cover flags and the cover list) is computed by the
  Creation Kit from what stands beside a triangle. New triangles get none;
  triangles taken out leave the list. In a fight, characters do not take
  cover on the new paths.
- **Finalizing** in the Creation Kit recomputes links, cover and the info
  map from the collision of everything placed. Mod Mallet does not have
  that collision and cannot do it.

## Decision

1. **A repair takes out and lays new.** It replaces ADR 0052's first rule.
   In the versions of the meshes the game uses:
   - The triangles on the hidden objects are taken out: they keep their
     numbers (ADR 0052), nothing leads onto them or away from them, they
     leave the search grid and the cover list, and their doors are lost.
     Neighbouring meshes that led onto them are changed so that they lead
     nowhere.
   - Below the objects that carried paths, new triangles are laid on what
     lies below: outside on the land, through points every 128 units (the
     land's own spacing); inside on the floor the object stood on, through
     the corners of the paths around. They join the open edges of the
     paths around the object (within 320 units seen from above, and within
     64 units of the land or the object's foot in height), as an even
     triangulation (Delaunay) that keeps those edges. A new triangle goes
     into the mesh of the cell it lies in, outside, and joins triangles of
     other meshes with links on both sides.
   - No triangle is laid where paths already run (within 96 units in
     height), steeper than 60 degrees, farther than 320 units from the
     hidden object, or in an area that joins no path around: characters
     go around there. Under water, new triangles are water, as the game's
     own are.
2. **The result is in the analyses' representation** (`NavmeshShape`, new
   triangles numbered after the old), so that whether doors, beds and
   markers stay reachable can be checked on it (ADR 0059). Taken-out
   triangles stay in it, leading nowhere.
3. **The records are built from the plugin's own record.** For each mesh
   that changes, the engine reads the record of the version the game uses
   and changes its field as the repair says, everything else byte for
   byte; form IDs are rewritten for the patch's masters by
   `NavmeshRecord::fields_for`. Mod Mallet writes neither `NAVI` nor
   cover. The plugin that carries the records is the patch plugin of ADR
   0060; the records go into the temporary children of their cell, which
   the record names with its worldspace and grid position.
4. **The window shows it before it is chosen.** Where it warns that
   characters walk on what a choice hides (the spot on the collisions page,
   the guided review), "Repair the paths" asks the engine
   (`walkway_repair`) and draws the triangles taken out in red and the new
   ones in green over the 3D view. Choosing it stores `repair:<navmesh>`
   among the patch choices for each mesh it changes; writers that do not
   know the prefix leave the choice alone.

## Consequences

- At Falskaar's harbour, hiding the ship's hull takes out 109 triangles
  of `skyrim.esm:102F4F` and lays 192 new ones on the sea bed where the
  hull stood, all under water, in one area joined to the sea bed's paths
  around; nothing is left without a way below. Two doors of JK's ship stood
  on its deck and are lost (`jks skyrim.esp:001C1E`, `001C2A`).
- At JK's Sky Haven Temple, the 11 hidden pieces carry 76 distinct
  triangles (101 counted per object, because pieces overlap): 29 of
  `skyrim.esm:1046F8` (all of it, in −35,2) and 47 of `skyrim.esm:1046FA`
  (−35,1). 77 new triangles are laid on the land, 32 and 45, in three
  areas joined to the paths around; no door is lost. Links between the two
  repaired meshes lead both ways (16 of them).
- Working out a repair takes about two seconds on the user's setup; the
  repaired fields read back with the analyses' reader.
- The repair is a heuristic like the check of ADR 0051. It does not know
  what else stands where the object stood (another object that stays, a
  piling under a pier): new paths may run through it. New triangles may
  cross a cell's border a little. They are joined only to edges at the
  height of the ground, so a pier or deck beside the object is not joined
  to the ground below it.
- A plugin loaded after the patch that changes the same mesh undoes the
  repair, as ADR 0052 says; a mesh that changes after the repair was
  chosen needs it worked out again.
- Not in reach: writing the navigation info map, cover, ledge links,
  door triangles for new doors, and finalizing as the Creation Kit does.

## Alternatives considered

- **Lay the cut triangles on the ground below, triangle for triangle.**
  Keeps the numbers of every triangle, but the deck's outline is not the
  hole's: at the harbour, the deck dropped on the sea bed would have joined
  nothing. The hole's own edges decide where new paths join.
- **Fill only closed holes.** The paths on the sea bed at the harbour do
  not close around the hull; they run on along the pier. Joining open
  edges near the object reaches those that are there.
- **Delete the taken-out triangles.** Every triangle after them would get
  a new number, and every mesh that links into it would have to be
  rewritten as well (ADR 0052).
- **Write the navigation info map too.** It needs the island data and
  joins of every mesh it lists, which only the Creation Kit computes; an
  entry that is slightly off costs a planned way at worst.
