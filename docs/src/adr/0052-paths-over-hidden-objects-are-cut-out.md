# 0052 Paths over hidden objects are cut out of the navigation meshes

- Status: accepted; changed by [ADR 0058](0058-paths-repaired-on-the-ground-below.md): new paths are laid on the ground below
- Date: 2026-11-19
- Changes: the vision's non-goal "not an automatic navmesh generator"
  (repairing is no longer only manual work); ADR 0051 (its repair, "a
  later feature", is decided here); ADR 0025 (a patch that changes records
  is decided here, for navigation meshes only)

## Context

ADR 0051 finds the paths characters walk on that run over objects Mod
Mallet hides, and warns before a choice. It leaves the paths as they are:
the only remedies were to keep the object or to hide it knowingly.

The user's case is a mod that adds a second bridge beside the game's own,
as Luxvia's additions sometimes do. The user does not want the second
bridge and hides all its parts. The mod's navigation mesh still runs over
it, so characters walk on air there. What the user wants is that
characters going from one bank to the other take the game's bridge
instead, which still has its paths. Where nothing else leads there, as to
a boat moored at a hidden jetty, no repair can help, and the user wants to
be told so.

What the game reads, and what the tools do with it:

- A navigation mesh (`NAVM`) keeps its surface in one field, `NVNM`
  (Skyrim: version 12; layout in `crates/mallet-formats-bethesda/src/navmesh.rs`
  after xEdit's definition). Each triangle names, for each of its three
  edges, the triangle beyond it, or `0xFFFF` for none; a flag per edge
  says the number is that of a link to a neighbouring mesh instead, which
  names that mesh and its triangle. Door triangles, cover triangles and a
  search grid follow, all naming triangles by number.
- The game's characters find their way over triangles connected by those
  edges and links. The worldspace's navigation info map (`NAVI`) lists,
  per mesh, the meshes and doors it connects to, for planning long ways.
- A plugin that changes a navigation mesh replaces it as a whole. xEdit
  shows the field decoded but cannot merge two versions; the Creation Kit
  rebuilds edges, links and cover when a mesh is finalized there. A
  deleted navigation mesh crashes the game; meshes are changed, never
  deleted.
- The vision's non-goals left repairing navigation meshes as manual work
  in the Creation Kit. The user asked for Mod Mallet to do it.

## Decision

1. **A repair cuts out; it never adds.** Where the triangles of a mesh the
   game uses run over a hidden object (ADR 0051), Mod Mallet takes them
   out of the paths: none of their edges leads anywhere any more, no
   other triangle of the mesh leads onto them, and links from neighbouring
   meshes onto them lead nowhere. The game's characters then find their
   way over what is left, such as the game's own bridge. Nothing is
   generated: no triangle, edge or link is added, so a way that does not
   exist elsewhere is not made.
2. **Triangles keep their numbers.** Cut triangles stay in the field with
   their vertices; only edges and the edge-link flags change. Links of
   neighbouring meshes, door triangles, cover and the search grid name
   triangles by number and so stay valid. The field keeps its size
   (`navmesh::cut`, `navmesh::unlink`).
3. **What is left is checked before anything is written.** The paths are
   followed through every mesh the game uses around the hidden objects,
   across links between them; a link to a mesh that was not read leads
   out into the rest of the world. Of what hung together before the cut,
   a part that no longer leads out is cut off; where nothing led out, as
   in an interior cell, the part with the most doors is where characters
   come from and the others are cut off. Parts smaller than about 20
   square metres without a door are pieces beside the object and do not
   count. A door whose triangle is cut out can no longer be used.
4. **A repair that cuts something off warns first.** The window shows
   what would be cut off, where, how large and with which doors, and the
   user decides: keep the object, or repair anyway. A repair that cuts
   nothing off needs no further question beyond the hiding itself.
5. **The repair is a plugin Mod Mallet writes**, `ModMallet_Paths.esp`,
   flagged as light: overrides of the changed meshes, each a copy of the
   version the game uses with its field cut, and the copies of their cells
   and worldspaces that a plugin needs to hold them. It goes through
   Vortex like the patch of ADR 0025, on the user's click, and loads after
   every plugin whose meshes it changes. Removing it brings the paths
   back; navigation meshes are not kept in saves. The navigation info map
   is not changed: it would still list a connection across a cut bridge,
   which at worst lets a character try a way the meshes then refuse.
6. **In steps.** First: reading how the triangles hang together, cutting
   a field, and the check, answered by the engine command
   `hidden_walkways` as `repair` (this decision's first commit). Next: the
   plugin writer (header, masters, the nesting of groups, copies of the
   winning cell and worldspace records with every form ID rewritten for
   the patch's masters, localized names), the button in the review and on
   the collisions page, and the scan telling whether the installed patch's
   paths were repaired. Only Skyrim's layout is read, as in ADR 0051.

## Consequences

- Hiding the second bridge no longer has to leave characters walking on
  air; the repair is undone like the rest of the patch.
- On the user's setup, the installed patch hides 913 objects; 905
  triangles of 36 meshes the game uses lie on them. Cut out by the plugin
  whose objects are hidden, those of Lux Via (28 triangles), JK's Skyrim,
  JK's Sky Haven Temple and JK's outskirts of Markarth and Whiterun leave
  every remaining path reachable. Those of Open Cities would lose 7 doors,
  and in all six parts of 24 to 91 square metres would be cut off, among
  them part of a Legacy of the Dragonborn interior with a door. Below 20
  square metres there were 15 more parts of one to seven triangles next to
  hidden objects, which the threshold leaves alone.
- Mod Mallet writes a plugin for the first time. Copying cells and
  worldspaces is what ADR 0025 avoided; it is limited to the records that
  hold a changed mesh, and the copies are the winning versions.
- A plugin that loads after the repair and changes the same mesh undoes
  it; the scan's `navmeshes.changes-undone` reports that, and a new patch
  repairs again.
- Which triangles run over an object stays the heuristic of ADR 0051. A
  triangle along the side of an object that is missed stays walkable but
  leads nowhere new; a character standing on a cut triangle at that
  moment stands on an island.
- Repairing doubled doors, the user's next idea (a mod moves a door, the
  game's stays, one is hidden, and what led to it should lead to the
  other), needs the door's links and its teleport moved and is not part
  of this decision.

## Alternatives considered

- **Delete the triangles.** Every triangle after them would get a new
  number, and every link of a neighbouring mesh, door triangle, cover
  entry and grid cell naming them would have to be rewritten, in other
  meshes as well.
- **Set the triangles' "deleted" flag.** xEdit names bit 3 of a
  triangle's flags so, but what the game does with it is not known.
- **Go back to an older version of the mesh.** It loses every other path
  of the mod that changed it, not only those over the hidden object.
- **Generate the paths anew,** as the Creation Kit does. It needs the
  collision of everything placed and is a tool of its own; laying new
  paths stays work for the Creation Kit.
- **An xEdit script that cuts the mesh.** xEdit would write the plugin and
  rewrite form IDs, but the user would have to run it, and it could not
  check what is cut off. The user chose that Mod Mallet writes the patch.
