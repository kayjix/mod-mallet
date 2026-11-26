# 0039 Collisions are checked with the models, and touches count

- Status: accepted; changed by [ADR 0040](0040-collisions-as-a-worklist.md): the order in which a placed object is turned; [ADR 0067](0067-stacked-doors-are-merged-into-one.md): doors of one group are a spot of their own
- Date: 2026-11-17
- Changes: ADR 0023 (placed objects compared by their boxes)

## Context

ADR 0023 compares the objects that mods place by the box around each
model. A box is a rough guess: an arch, a bridge or an L-shaped house
leaves most of its box empty, and an object standing in that empty space
was reported although nothing visibly collides. The other way round, two
objects that only touch or cut a little into each other, a fence running
into another mod's wall, a cart standing half in a market stall, were left
out on purpose, because boxes cannot tell a touch from two boxes that
merely meet.

Mod Mallet reads meshes well by now (Skyrim, Fallout 4, Oblivion). The
user wants every spot where objects of two mods stand in or touch each
other tracked, so that each can be shown on the map and, later, hidden or
moved. Within one mod such contacts are mostly intended (a rock set into
the ground, a pillar in a wall), so they stay unreported.

## Decision

1. **Boxes find candidates, models decide.** Pairs of objects of two
   plugins that do not know each other (ADR 0023) and whose boxes meet are
   candidates. The engine asks the analysis which models those candidates
   use, reads only these models from the files the game uses, and hands
   their triangles to the analysis as facts (`ModelFacts`). Duplicates of
   the same object are not affected.
2. **The test**: every edge of one model's triangles is tested against the
   nearby triangles of the other, inside the space both take up, sorted
   into a grid. Where an edge passes through a triangle, the surfaces
   cross. The length of the stretch over which they cross, the longest side
   of the box around all crossing points, is the collision's *depth*.
3. **Overlaps** (ADR 0023's rule: largely inside each other) are kept only
   if the models cross, where both models are known. Where a model cannot
   be read, the box decides as before.
4. **Touches** are new: candidates that are not overlaps, whose models
   cross over at least 16 units (about 25 cm). They are reported only
   where both models are known, as a hint, one finding per pair of
   plugins (`objects.touching`), and never for landscape or snow laid over
   other things. An object of the game that a mod moved touches only if it
   did not already touch where the game places it.
5. **Every spot says what it is**: `MapSpot.kind` is `duplicate`,
   `overlap` or `touch`, and `MapSpot.depth` the crossing stretch where
   models were compared. The map colours spots by kind.
6. Water planes are not solid.

## Consequences

- On a real setup of about 400 plugins, about 900 models are read in under
  two seconds; comparing them adds about a second to the scan.
- Some reported overlaps disappear (on the reference setup 21 of 179),
  and touches appear: on the reference setup about 1,400 spots between
  69 pairs of plugins, most of them between mods known to need patches
  (for example a road mod and a town mod). One large object of one mod can
  touch many small objects of another; each is its own spot.
- The depth is the length of the crossing, not how far one object sinks
  into the other: a rug lying a little into a floor crosses it over its
  whole edge.
- Models with more than 200,000 triangles, or more than 60,000 in the
  space two objects share, are not compared; the boxes decide for them.

## Alternatives considered

- **Reading the models during the ingest of every plugin.** Exact for all
  pairs, but would read tens of thousands of models for the few hundred
  that matter.
- **Measuring how deep one object sinks into the other.** Needs to know
  what is inside a model, which open meshes, as most of the games' are, do
  not tell reliably.
