# 0060 Objects are moved a little, through a plugin Mod Mallet writes

- Status: accepted
- Date: 2026-11-21
- Changes: [ADR 0025](0025-patches-through-base-object-swapper.md) (a patch
  that changes records is decided here, for the place of placed objects);
  [ADR 0053](0053-shareable-patches-in-parts.md) (step 4, "moving
  objects", is decided here, with a plugin rather than Base Object
  Swapper's transforms)

## Context

Where two mods build into each other, Mod Mallet so far offered one remedy:
hide one side's objects through Base Object Swapper (ADR 0025). Often both
structures are worth keeping, and the overlap is a matter of a metre: a
scaffold of one mod stands half inside a wall of the other. The user wants
to nudge an object, or the parts of a structure the magic wand selects
(ADR 0055), a little to one side, so that both stay, to the eye and to
the game.

Moving a placed object means changing its `DATA` field (position and
rotation) in a plugin that loads after every other version of it. ADR 0025
explained why Mod Mallet had avoided writing plugins: an override sits
inside copies of its cell and worldspace, those copies have to be the
versions that win, and every reference to another record inside them has
to be rewritten for the new plugin's masters. ADR 0052 decided a plugin
for navigation meshes; another worker builds it (ADR 0058) and needs the
same nesting of groups.

## Decision

### A plugin writer for overrides

- `mallet_formats_bethesda::plugin_write` writes a plugin with a `TES4`
  header (format version, record count, author, description, masters) and
  puts records into the nesting a plugin needs: interior cells in blocks
  and sub-blocks by the last two decimal digits of their number, exterior
  cells in their worldspace's children, the persistent cell first, the
  others in blocks of 32 and sub-blocks of 8 cells labelled y, x, and each
  cell's records in its persistent or temporary children. Other records go
  into top-level groups of their type.
- It knows nothing of what records mean. The caller hands it records whose
  form IDs are already written for the new plugin's masters, with their
  form version (the reader now reports it, `plugin::read_request_versions`).
  The API is `PluginBuilder::add_to_interior` and
  `PluginBuilder::add_to_exterior` (a worldspace, a cell and the records
  for its persistent or temporary children) and `add_top_level`; the
  navigation meshes of ADR 0052/0058 go into the same cells' temporary
  children through the same calls.
- The plugin is flagged as light where the game knows light plugins and
  its own new records fit one. A plugin of overrides only always fits.
- Round trips through the repository's own reader are tested, and a
  property test checks that anything written reads back undamaged.

### The plugin of moves

- A move names an object (plugin and number), how far it goes along x, y
  and z in whole game units and how far it turns about the upright axis in
  whole degrees, as the game counts (clockwise from above), with where it
  stood and, inside, the cell's editor ID, so it can be found without
  reading every cell.
- `mallet_game_bethesda::moves::build` finds, in the load order, the
  version the game uses of each object, of the cell it lies in and, outside,
  of its worldspace. It copies each, changes only the object's position and
  rotation, and writes `Mod Mallet - Moves.esp`. Its masters are the
  plugins whose versions it copies and every plugin those versions refer
  to, in load order, so the mod manager sorts it after all of them.
- A turn is applied to the placement matrix (`Rx(−x)·Ry(−y)·Rz(−z)`, the
  same as the desktop's `placement`) from the world's side and turned back
  into angles; an upright object only adds to its angle about z.
- Which bytes of a field are references is known per record type and
  field (`REFR`, `CELL`, `WRLD` of Skyrim Special Edition, after xEdit's
  definitions), including attached scripts (`VMAD`) and a worldspace's
  large references (`RNAM`). **A field not in the table is not copied
  blindly: the object is refused, and the user is told which field.**
- A localized plugin keeps names in string tables. The copy is not
  localized, so names (`FULL`) are written as text from the tables,
  English first, else whatever language the setup has; where none can be
  found the object is refused. A worldspace's offset table (`OFST`) points
  into the file it came from and is left out.
- The engine (`create_moves`) leaves an earlier `Mod Mallet - Moves.esp`
  out of the load order it reads, writes the plugin in an archive next to
  the patches in Mod Mallet's own folder, and reports what it moves, its
  masters and what it refused. `Action::MoveObjects` writes it and asks
  Vortex to install it through the link, on the user's click only, like
  the patch (ADR 0020, 0025). Nothing is written into the game.
- Only Skyrim Special Edition is supported; other games refuse with a
  reason.

### Choices and the window

- A move is kept among the patch's choices under `move:<plugin>|<localId>`,
  its value a small JSON text (`dx`, `dy`, `dz`, `rz`, `at`, `cell`). The
  Base Object Swapper patch reads none of these keys; clearing the hiding
  decisions keeps moves, as it keeps pins.
- The object card of the 3D views has *Move a little*: arrows for east,
  west, north and south, up and down, turning both ways, a step from 10 cm
  to 5 m and a turn from 1° to 90°. *Move the parts together* takes what
  the magic wand selected and moves it as one, turning it about its
  middle. Each nudge is drawn at once where it puts the objects, and the
  check against the other mod's parts (at a spot) or every other mod's
  objects around is done again by the models' boxes: "no longer touches",
  "still overlaps by N cm", "now overlaps". Moving a part of an open spot
  keeps both there. *Undo* steps back, *Back to where it was* drops the
  move.
- Objects the game relies on (ADR 0046) cannot be moved; those it may
  miss warn with their reasons first (teleport doors, enable parents,
  linked references and the rest). Where characters walk on an object
  (ADR 0051), the card says that their paths stay where they are.
- The spot panel of the collisions says which parts of a side are moved,
  with *Back to where it was*, or how to move one. The patch page lists
  the moves, writes the plugin, shows what it could not move and why, and
  installs it.

## What moving cannot fix

Moving changes where the game draws and collides with the object. Three
things built around the old place stay where they were:

- **Navigation meshes.** The paths characters walk on are triangles in the
  cell's navigation mesh, laid over the object where it stood. Moved
  sideways, the paths hang in the air beside it or run into it; moved up
  or down, characters sink into it or float above it. Mod Mallet warns
  where paths lie on a moved object, but does not move them. A later step
  could shift the vertices of the triangles that lie on the object by the
  same move, keeping their edges and links, check that what is left still
  hangs together as ADR 0052 does, and write the changed meshes through
  this writer into the same cells (ADR 0058). It needs the triangles that
  lie *on* the object told apart from those that only touch it, and a
  rule for the seams where the moved triangles meet the ground's.
- **Level of detail.** Objects seen from afar are drawn from meshes that
  tools such as DynDOLOD or xLODGen generate from the load order. Until
  they are generated again, the distant view shows the object where it
  was, and it jumps when the cell loads. A later step could say which
  moved objects are visible from afar and remind the user to generate the
  distant view again; generating it is not Mod Mallet's work.
- **Things the game's physics let fall into place.** Loose clutter (cups,
  books, baskets) is moved by the game's physics, and a save keeps where
  it came to rest once the cell was visited. In such a save the plugin's
  position is not used, and a plate on a moved table stays where the table
  was. A later step could move what lies on a moved object along with it
  (the wand's contents, ADR 0055) and tell the user that a save which has
  seen the cell keeps the old place of loose things; it cannot change the
  save.

Two smaller limits follow from the format:

- An object moved past the edge of its cell stays a child of its old cell,
  as an override cannot move it into another; the game loads it with that
  cell. Nudges are small, so this matters only at a cell's edge.
- Names copied from string tables are written in English where the setup
  has English tables. A game in another language with English tables shows
  an English name for a cell the plugin copies.

## Consequences

- Both structures of a collision can stay, and removing the mod `Mod
  Mallet - Moves` in Vortex puts every object back; static objects are not
  kept in saves, so this works in existing saves.
- The plugin carries copies of the winning cell and worldspace records. If
  the load order changes later, it may undo another mod's later change of
  them; writing it anew after a change of the load order fixes that.
  Because its masters include the plugins it copies from, it cannot load
  before them.
- On the user's setup a move of a JK's Whiterun Outskirts piece next to a
  Lux Via scaffold near Whiterun named four masters, among them the two
  plugins that give the main worldspace's winning record its water, and
  was written in about half a second.
- The writer is shared with the navigation mesh repair (ADR 0058).
- The collision check in the window is by boxes, so it errs on the side of
  overlapping; the scan's own check by models (ADR 0039) is the one that
  counts after the next scan.

## Alternatives considered

- **Base Object Swapper's transforms.** A `[Transforms]` section of a swap
  file can change where a placed object stands while the game runs, with
  no plugin. It needs the script extender plugin, applies when the object
  loads (after the game has placed its collision once), and is not
  understood by tools that generate the distant view or check plugins. A
  plugin is what every other tool reads.
- **Copying records without rewriting references, by giving the new
  plugin the same masters as the source.** Works for one source plugin;
  a move copies from several (the object from a mod, the worldspace from
  another), whose master lists differ.
- **Copying every field, known or not.** Fewer refusals, but a reference
  in an unknown field would point at the wrong record after the masters
  change, silently. A refusal with the field's name is honest and can be
  lifted field by field.
