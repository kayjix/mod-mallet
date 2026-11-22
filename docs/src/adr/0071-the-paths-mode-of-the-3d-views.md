# 0071 The paths mode of the 3D views shows where characters walk

- Status: accepted
- Date: 2026-11-22
- Changes: [ADR 0058](0058-paths-repaired-on-the-ground-below.md) (its
  red-and-green layer of the repair also draws the paths mode);
  [ADR 0059](0059-where-characters-still-go.md) (the places characters walk
  to are drawn map-wide, not only where a way changes)

## Context

Mod Mallet reads the navigation meshes: the nets of triangles on which the
game's characters find their way. So far it showed them only piecemeal. A
spot warns that a choice hides an object characters walk on (ADR 0051).
"Repair the paths" draws the triangles taken out and laid new (ADR 0058).
The ways to doors and markers before and after are drawn (ADR 0059). There
was no way to look at the paths themselves.

The user wants to see them where the problems are. When a bridge is hidden,
the paths over it break, and the user wants to see that before choosing,
replace or repair it, and check the repair afterwards. A tab of its own is
only worth it for editing, which Mod Mallet does not do. What the user asked
for:

- the game's own paths;
- the paths a mod added or changed, with which mod;
- the repairs of `Mod Mallet - Paths.esp`;
- where the current choices leave paths over nothing;
- doors, the links between meshes, and where characters sit, lean, sleep and
  work.

What a navigation mesh tells (ADR 0051, 0052, 0058): every plugin that
changes a mesh replaces it whole, so who made a triangle can be told only by
comparing the versions. A triangle is kept unchanged when its three corners
are; the triangles keep no other identity across versions. A repair leaves
the triangles it takes out in place, leading nowhere (ADR 0052).

## Decision

1. **A mode, not a page.** The 3D view of the collisions page (outside and
   in interiors) and the relief of the world map get a switch "Paths" in
   the bar over the view. It is remembered in the window. With it on, the
   view draws the paths of the cells around the point the objects are read
   for (ADR 0038). Without an area up close, the mode asks the user to come
   closer.
2. **The engine reads an area's paths with who made them.** The command
   `area_paths(world or cell, centre, radius, hidden)` reads every plugin's
   version of every navigation mesh in the cells within the radius (at most
   two and a half cells, so at most six cells across) or in the interior
   cell. It keeps the last
   six sets of cells, so moving back and forth and changing choices do not
   read the plugins again. It answers, for the versions the game uses:
   - who made each triangle: the first version from which on every version
     has the triangle with the same three corners
     (`mallet_analysis::paths_origin`). A version in between that drops it
     makes the later one its maker. The makers are the game (its own
     plugins), a mod, or the plugin of paths;
   - the triangles that version takes out of the paths: they lead
     nowhere, though the version before led on from them;
   - how many triangles of the game's own last version are gone;
   - the middle of each door triangle and of each edge linked to a
     neighbouring mesh;
   - for the objects `hidden` that stand there (the window sends what the
     choices hide), the triangles of the versions the game uses that run
     over them, as ADR 0051 tells them, without those already taken out.
3. **Colours that also differ by pattern.** The colours are taken from the
   Okabe–Ito palette, and every kind has a pattern of its own:
   - the game's paths: sky blue, plain;
   - those of mods: orange, striped;
   - those the repair laid: bluish green, dotted;
   - those it took out: a vermilion outline;
   - those over nothing: reddish purple, cross-hatched.
   Marks keep their size on screen and differ by shape first:
   - a door: a white square;
   - a link: a small dot;
   - sit: a yellow circle;
   - lean: a triangle;
   - sleep: a blue diamond;
   - every kind of work (chopping wood, mining, smithing, crafting,
     cooking and other work): a vermilion square;
   - praying, standing and anything else: a grey dot.
   Everything is drawn see-through on top, so paths under the land or a
   deck show too. It is drawn by the same layer as the repair of ADR 0058
   (`paths-layer.ts`), whose red and green stay on top. The ways of ADR 0059
   stay with `reach.tsx`.
4. **A legend with the mods.** The legend over the view counts each kind in
   the cells read and lists the mods with paths there. Pointing at a mod
   lights its triangles in yellow and dims the rest; a click keeps them lit.
   A mod whose version is replaced entirely by a later one is listed as
   such.
5. **Problems in view.** Each collision listed whose parts the choices hide
   and that carries paths is a problem. It is marked by a ring at the middle
   of the pieces over nothing: reddish purple, or bluish green once the
   repair of every mesh concerned is chosen (`repair:<navmesh>`, ADR 0058).
   The legend lists the problems. "Open" opens the spot with its walkway
   warning and "Repair the paths". Objects the choices hide outside any
   listed collision are counted.
6. **Action places are drawn, not changed.** ADR 0069 reads them, with what
   characters do there (`mallet_core::action_places::ActionPlace`), into
   the objects of the area (`AreaObjects.actions`), and owns moving and
   removing them and their object card. The paths mode only draws them,
   with a switch of their own, from the objects the view has read.

## Consequences

- On the user's setup (read only, Skyrim SE, about 320 plugins), 16 cells
  around a point are read the first time in 0.6 to 2.3 seconds, and from
  the kept read afterwards in milliseconds. Reading the hidden objects and
  their models adds about half a second.
- At Falskaar's harbour (Tamriel 6,26), the cells around show 9,250
  triangles:
  - 8,157 the game's;
  - 839 JK's Skyrim's;
  - the rest of Immersive Citizens, Clockwork and the Unofficial Patch.
  Hiding JK's ship hull leaves 109 triangles of `skyrim.esm:102F4F` over
  nothing, as ADR 0051 found.
- At JK's Sky Haven Temple (Tamriel -35,1 and -35,2):
  - 1,294 triangles are the mod's;
  - 1,412 triangles of the game's own version are gone.
- Around Markarth's outskirts, JK's Markarth Outskirts and Open Cities make
  most of the paths. Lux Via's versions there are replaced entirely by
  later ones and show as such in the legend.
- The plugin of paths was not installed on the user's setup, so its
  triangles were seen only in tests and in the made-up data of the window.
- Who made a triangle is told by its corners. A mod that rebuilds a mesh
  with the same triangles numbered differently counts as keeping them. A
  corner moved by a unit makes the whole triangle the mod's.
- The view does not tell which mod made a triangle when one clicks on it in
  the view; the legend does, by lighting the mod's triangles.
- When the camera goes far, the paths of the last area stay drawn, like the
  objects.
- Action places come with the objects the view reads. The paths mode
  draws ADR 0069's twelve kinds as five marks: sit, lean and sleep each
  apart, every kind of work alike, and praying, standing and the rest
  alike. Which way a place faces is not drawn.

## Alternatives considered

- **A page of its own for the paths.** It only pays off for editing, which
  Mod Mallet leaves to the Creation Kit. The paths matter where a choice
  is made, so they are drawn there.
- **Colour by the version the game uses only.** It would paint a whole mesh
  in a mod's colour when the mod changed one triangle of it. The
  comparison per triangle shows what the mod changed.
- **Read the paths of the whole worldspace once.** Over 44,000 meshes in
  the user's setup; the cells around the point are enough and read in
  about a second.
