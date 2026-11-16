# 0023 Placed objects are compared by the boxes around their models

- Status: accepted
- Date: 2026-11-15

## Context

The fourth milestone is about mods that put things into the same spot: the
same house placed twice by two town overhauls, a tree of one mod growing
through the roof of another. Neither mod is broken, both change different
records, and no mod manager shows it.

To find it, Mod Mallet has to know where every object of every mod stands
and how large it is. A real setup with three hundred plugins places over a
million objects through mods. Each object is a reference record inside the
nested groups of a cell; its size comes from the base object it places,
which may be defined in another plugin. The exact shape of an object is in
its model, a mesh file in an archive.

## Decision

- The plugin reader can descend into the nested groups of cells and
  worldspaces, and tells for each record which groups it lies in
  (`Request`, `GroupLabel` in `mallet-formats-bethesda`).
- The game adapter keeps, for each placed object, the version the game
  uses. It reads placed objects only from plugins that are not the game's
  own, and leaves out objects the game itself places and changes to them.
  Objects that the winning version deletes or switches off are left out.
- An object's size is the **bounding box of its base object** (`OBND`),
  turned and scaled as placed. Models are not read.
- An object counts only if it is **solid**: it has a model, and the model
  is not a marker, an effect or a small plant. Whether it is a piece of
  **landscape** follows from its model's folder as well. The adapter
  decides both from the model path; the analysis only reads the flags.
- Two objects of **different plugins** are reported when they are the same
  base object less than 32 units apart (*duplicated*), or when at least half
  of the smaller lies inside the larger and the overlap fills at least a
  tenth of the larger (*overlapping*). How much lies inside is measured at
  125 points spread through the smaller box. Two pieces of landscape are
  never reported as overlapping.
- Pairs of plugins where one has the other as a master, or a third plugin
  has both, are left out: their authors knew of each other.
- Findings are **one per pair of plugins and kind**, with the number of
  places and a few examples, named by base object and place. They use a
  new subject, `Subject::Pair`, whose two parties are ordered by name, so
  the identity does not depend on the load order (ADR 0007).

## Consequences

- The milestone's reference problems are found without reading a single
  mesh. A full scan of a large setup takes about five seconds longer.
- A box is not a model. An arch, an L-shaped building or a bridge has empty
  space in its box, and objects in that space are reported although
  nothing visibly collides. Overlaps are therefore a *hint*, duplicates are
  *likely*.
- Mods that move or change the game's own objects are not compared. A mod
  that moves a house of the game into the way of another mod's house is
  not found yet; that needs the game's own placements, which are many
  times more than all mods together.
- Compatibility patches that resolve a clash by other means than switching
  objects off (for example by a script) are not recognised, but they make
  the pair "known" and therefore silent.

## Alternatives considered

- **Comparing the models.** Exact, but needs every mesh unpacked and a
  collision test on triangles; far slower, and the box answers the
  question "do two mods use the same spot" well enough.
- **Comparing every object, including the game's own.** Would catch moved
  vanilla objects, at the cost of reading and comparing several million
  more placements on every scan.
- **One finding per object.** Exact, but a known incompatibility of two
  town mods would fill the report with a hundred lines.
