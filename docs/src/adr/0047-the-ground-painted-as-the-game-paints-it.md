# 0047 The ground painted as the game paints it

- Status: accepted
- Date: 2026-11-18
- Changes: ADR 0041 (the game's own ground up close), which coloured the
  ground with the level of detail's land texture

## Context

Up close the ground has the game's own heights (ADR 0041), but its colour
came from the land texture of the level of detail: a few pixels per metre,
made for looking from afar. Next to the objects, whose textures are
sharp, the earth looked smeared, and paths, rocks and grass on the ground
could not be told apart.

The game paints its land from layers. Each quarter of an outside cell has
a base texture (`BTXT`) and up to about eight layers over it (`ATXT`),
each with its opacity at the 17 × 17 points of the quarter (`VTXT`). A
layer names a landscape texture (`LTEX`), which names a texture set
(`TXST`) whose first texture is the colour; Oblivion's landscape textures
name the file themselves (`ICON`). The result is tinted by the land's
vertex colours. Mods change all of these, the textures included.

## Decision

1. **The layers are read with the land**: the same pass over the plugins
   that reads the heights (ADR 0041) reads each quarter's base and layers
   in the version the game uses, the layers in their order, their
   opacities as bytes. A layer that shows nowhere is left out.
2. **Landscape textures are resolved once per setup**: the `LTEX` and
   `TXST` records of every active plugin, later ones winning, give each
   landscape texture its colour texture. The land sent to the window names
   the texture files; the files themselves come as all others do, from
   the setup's archives and loose files as the game would find them.
3. **The window paints like the game**: per quarter, the base, then each
   layer mixed over it by its opacity, interpolated between the points,
   times the vertex colours. The textures are fetched at 512 pixels and
   kept in one texture array of 32 places, the most used of an area first;
   each quarter's opacities are two small textures of four layers each.
   One drawing call per quarter.
4. **Until a quarter's base texture is there**, it keeps the colour of
   ADR 0041: the level of detail's land texture, else the vertex colours.
5. The textures repeat six times along a cell. The plugins do not say how
   often; this is an assumption, not yet compared with the game, and one
   constant to adjust.
6. Only reading, as everywhere.

## Consequences

- The ground up close shows its paths, grass, rocks and shores as sharp as
  the objects on it, and a mod that retextures the land shows here too.
- The colour and light of the game's own shader are not matched exactly:
  no normal maps, no specular, no shadows; the light is the map's.
- An area with more than 32 different ground textures draws the rest
  with the quarter's fallback colour.
- Grass, which the game places on the land by its textures, is not
  drawn.
