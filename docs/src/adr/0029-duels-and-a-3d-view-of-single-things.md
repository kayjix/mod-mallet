# 0029 Mods are decided pair by pair; single things are shown in 3D

- Status: accepted; changed by [ADR 0033](0033-comparison-stage-and-ranking.md) (comparison view, stage, ranking); [ADR 0031](0031-places-on-the-map-and-a-3d-relief.md): the land, drawn as a relief; [ADR 0048](0048-one-job-per-page.md): where duels and the ranking are shown
- Date: 2026-11-16
- Changes: ADR 0011 (no renderer of our own), for single objects and heads

## Context

The task list showed one entry per finding. On a real setup that is a few
hundred entries, and most of them are the same few pairs of mods getting
in each other's way: one face overhaul against a content mod in 34 faces,
two town mods in a dozen objects. Deciding entry by entry is tiring, and
it tears mods apart: a little of one mod here, a little of the other
there, without anyone meaning to.

The flat pictures of ADR 0028 show colour, but a dark face texture or a
replaced mesh says little on its own. To decide between two mods one has
to see the thing.

## Decision

1. **Duels.** The findings about two mods are gathered into one duel per
   pair (`mallet_analysis::duels`): what they disagree about by topic
   (faces, textures, meshes, objects, contained plugins), who wins each
   topic now, and the findings it covers. The task list shows duels first
   and the remaining findings below.
2. **A suggestion with a reason**, by two rules:
   - *Built on*: if a plugin of one side names a plugin of the other as
     its master, it was made to change that mod and wins.
   - *Specialist*: for the topic the pair disputes most, the side whose
     disagreements with any mod are mostly about that topic wins, if the
     difference is clear (25 percentage points). A face overhaul whose
     conflicts are nearly all faces beats a content mod that touches faces
     in passing.
   Otherwise there is no suggestion, and the comparison has to decide.
3. **One decision for the whole pair.** Picking a side runs every action
   that makes it win (plugin rules through the mod manager) and puts the
   other side's colliding objects into the patch. The decision is kept in
   the settings and can be taken back; rules already handed to Vortex are
   removed there.
4. **A 3D view of single things.** Meshes and heads of the user's setup
   are drawn in the window, turning, with their colour textures; a face's
   texture colours its head. It shows one thing, or two versions of it side
   by side, never the world. Meshes of Skyrim (version 83) and Skyrim
   Special Edition (100) are read, including skinned heads. It loads only
   when opened and frees the graphics memory when closed.

## Consequences

- ADR 0011 still holds for the world: there is no 3D map, no lighting of
  scenes, no terrain. What is drawn is close to the game, not the game:
  no normal maps, no lighting model of the game, no alpha in textures.
- The suggestion rules are deliberately simple and explained in every
  duel; they can be wrong, and the user has the last word.
- A mod rule for file conflicts between two mods (Vortex's "load after")
  is not used yet; duels about textures and meshes are decided by plugin
  order or by hand.
