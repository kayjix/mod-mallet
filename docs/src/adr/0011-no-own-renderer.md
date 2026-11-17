# 0011 No renderer of our own

- Status: accepted; changed by [ADR 0029](0029-duels-and-a-3d-view-of-single-things.md) for single objects and heads, by [ADR 0031](0031-places-on-the-map-and-a-3d-relief.md) for a relief of the land, and by [ADR 0037](0037-the-map-draws-the-games-own-lod.md) for the game's own level of detail
- Date: 2026-11-15

## Context

Many findings are about how the game looks: overlapping buildings,
mismatched textures. Showing them is valuable. Rendering Skyrim's world
faithfully means reading meshes, materials, terrain, lighting and LOD, which
amounts to rebuilding a good part of the engine. Tools that render already
exist, and the most faithful one is the game.

## Decision

Mod Mallet does not render the game world in 3D.

- **A map in 2D** is drawn from the terrain LOD textures that the game and
  its mods already ship (`textures/terrain/<worldspace>/...`), read from the
  user's own installation. Findings are drawn on top as outlines.
- **Looking at a problem** happens in the game. First through a console
  command the user can paste, later through a small script extender plugin
  that takes the player to a finding and highlights the objects involved.
- **Editing** happens in the Creation Kit or xEdit.

## Consequences

- No game assets are distributed with Mod Mallet, which avoids licensing
  questions; the map is built from files the user already has.
- Worldspaces added by mods get a map for free if the mod ships LOD.
- A problem cannot be previewed before starting the game. For "which of
  these two mods looks better" Mod Mallet can show what differs, not a
  picture.
- The in-game bridge is native code tied to game versions. It must be built
  on Address Library so that it survives game updates.
