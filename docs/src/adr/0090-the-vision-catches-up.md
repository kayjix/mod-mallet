# 0090 The vision draws its lines where Mod Mallet's own work ends

- Status: accepted
- Date: 2026-11-26
- Changes: [ADR 0003](0003-scope-and-non-goals.md), the non-goals in
  [Vision and non-goals](../product/vision.md); takes up what
  [ADR 0052](0052-paths-over-hidden-objects-are-cut-out.md) already changed

## Context

ADR 0003 and the vision said Mod Mallet is not a plugin editor ("complex
patches are generated as scripts for xEdit"), not a renderer ("there is no
3D view of the game world") and not an automatic navmesh generator. Over
the following rounds the user asked for, and got, the opposite of each
sentence as written:

- Places are drawn in 3D with their objects, land and distant view, worked
  through spot by spot, passage by passage (ADR 0038, 0040, 0042, 0072),
  with a paths mode on top (ADR 0071).
- Mod Mallet writes plugins of its own: objects moved instead of hidden
  (ADR 0060), navigation meshes cut and repaired (ADR 0052, 0058), the
  paths of merged doors re-attached (ADR 0088).
- Repairs lay new triangles where an object was taken out (ADR 0058).
- Decisions are kept in one place and hold on every page (ADR 0081), and
  preferences are learned from them (ADR 0086).

Each step was decided in its own ADR, but the non-goals were never redrawn,
so they no longer answer a feature request. Asked to look at the
philosophy again, the line had to be drawn anew rather than repeated.

## Decision

The vision is rewritten (see [Vision and non-goals](../product/vision.md)).
The rule behind its non-goals:

**Mod Mallet builds what carries out a decision the user made in its
views, and nothing that would make it a general tool of another kind.**

From that:

- **Plugins.** Mod Mallet writes only plugins of its own, each for one
  narrow purpose that follows from a decision (moves, path repairs, merged
  doors), into its own output, installed and switched off through the mod
  manager. It never changes another mod's plugin and offers no free record
  editing.
- **3D.** Mod Mallet draws a place in order to decide about it. It does not
  try to look like the game: no characters, effects, weather or the game's
  lighting.
- **Navigation meshes.** Paths are repaired only where Mod Mallet's own
  decisions broke them. Paths for new land or buildings, and the Creation
  Kit's finalizing, stay out.
- **Mod manager.** Unchanged: Mod Mallet asks the mod manager; it does not
  install, enable or order by itself.

The principles gain three that the app already follows: *show, then ask*;
*the user decides, Mod Mallet remembers*; and *plain words, a little
charm*. "Only certain findings may ever be fixed without asking" becomes
"nothing is changed in a setup without the user's click", which is what
the app does. "Reuse before building" names when building is right: where
no existing tool can be driven from outside for the job.

## Consequences

- A request is checked against one question: does it carry out a decision
  the user makes in Mod Mallet, or does it turn Mod Mallet into a mod
  manager, a plugin editor, a game viewer or a navmesh generator?
- Requests that stay declined: a free record editor, a lighting- or
  character-faithful world view, laying paths for new land, installing or
  ordering mods without the mod manager.
- The plugin writer and the 3D views are part of the product now, not
  exceptions; they carry the same care as the scan (no panics on any input,
  property tests, read-back tests of what is written).

## Alternatives considered

- **Keep the old non-goals and treat the new work as exceptions.** Rejected:
  a non-goal with five exceptions answers nothing.
- **Drop the non-goals.** Rejected: without a line the project grows into a
  worse copy of Vortex, xEdit and the Creation Kit at once, which is what
  ADR 0003 warned against.
