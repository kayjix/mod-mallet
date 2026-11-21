# 0068 Which plugin needs which

- Status: accepted
- Date: 2026-11-21

## Context

A plugin names the plugins it builds on, its masters, and the game closes
while starting when one of them is not loaded. On a real setup with 382
active plugins there are about 1,500 such links: a lighting mod's base
plugin is needed by 94 others, a mod of cut content by six patches. Before
removing a mod, someone who installs mods wants to know what would break;
before removing a patch, what it needs. The scan already reads every
active plugin's header, masters included, but the report carried only
the findings derived from them (a master that is missing), not the links
themselves.

## Decision

1. **The report carries the load order** (`ScanReport::load_order`): every
   active plugin in the order the game loads it, the game's own plugins,
   the Creation Club content it loads on its own (including
   `_ResourcePack.esl`) and the plugins of `plugins.txt`. Each with its
   masters as it names them, whether it comes with the game, the Creation
   Club or a mod (`WorldOrigin`, as the map uses it), and the mod it comes
   from. The field is added with a default, so `SCHEMA_VERSION` stays.
2. **A master that is not in the load order is not loaded.** The report
   does not mark it again: the analysis `missing-masters` reports the one
   whose file is gone, and the views show any master that is not loaded
   beside the plugin that needs it.
3. **What would have to go with a mod is computed in `mallet-analysis`**
   (`dependencies::stranded_by`): every active plugin that would lose a
   master, directly or because a plugin it needs would have to go too.
   A recommendation to remove a mod names these plugins.
4. **The desktop app draws the load order as an arc diagram**, on a
   canvas, since 400 plugins and 1,500 arcs are too many for the page's
   elements: a tab "Dependencies" on the Mods page with the whole
   diagram, and a strip of it on each mod's page. What a plugin needs and
   what needs it differ by colour, by side (above or below the line), by
   line (solid or dashed) and by mark (triangle or dot), so that the
   difference does not rest on colour alone.

## Consequences

- A mod's page tells in one sentence what its plugins need and how many
  plugins of how many mods need them, each named and linked.
- The report grows by about 40 kilobytes for 400 plugins.
- The view works out the direct links in the window, from the report;
  what would go with a mod, transitively, stays in the engine's analysis.

## Alternatives considered

- **A graph with nodes placed by force.** It shows clusters, but not the
  load order, which is what decides whether a plugin can find its
  masters; with 400 plugins it turns into a ball of lines.
- **A matrix of plugins by plugins.** Exact, but 400 by 400 cells is
  unreadable at the size of a window.
