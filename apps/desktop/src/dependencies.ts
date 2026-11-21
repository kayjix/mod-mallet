// Which plugin needs which (ADR 0068): the load order of the report as a
// graph, and what one plugin or the plugins of one mod need and are
// needed by.

import type { LoadedPlugin, ModInfo } from "./generated/rendered-report";

/** Whether a plugin comes with the game, its Creation Club content or a mod. */
export type WorldOrigin = LoadedPlugin["origin"];

/** An active plugin at its place in the load order. */
export interface PluginNode {
  /** Its place in the load order, from 0. */
  index: number;
  /** The file name, as the load order spells it. */
  name: string;
  /** Whether it comes with the game, the Creation Club or a mod. */
  origin: WorldOrigin;
  /** The mod it comes from, by the mod manager's identity, if known. */
  modId: string | null;
  /** The name to show for its mod, if it comes from one. */
  modName: string | null;
  /** Whether the mod is in the report's list of mods, so it has a page. */
  hasPage: boolean;
  /** The plugins it needs that are loaded, by place. */
  masters: number[];
  /** The plugins it needs that are not loaded, as it names them. */
  missing: string[];
  /** The plugins that need it, by place. */
  dependents: number[];
}

/** A stretch of the load order whose plugins share an origin. */
export interface Band {
  origin: WorldOrigin;
  /** The first place of the stretch. */
  from: number;
  /** The last place of the stretch. */
  to: number;
}

/** The load order as a graph of plugins and the plugins they need. */
export interface DependencyGraph {
  nodes: PluginNode[];
  /** Places by file name in lower case. */
  byName: Map<string, number>;
  /** How many master links join two loaded plugins. */
  links: number;
  /** The stretches of the game's own plugins, the Creation Club and the mods. */
  bands: Band[];
}

/** The load order of a report as a graph, with each plugin's mod named as the mod manager names it. */
export function buildGraph(loadOrder: LoadedPlugin[], mods: ModInfo[]): DependencyGraph {
  const modsById = new Map(mods.map((info) => [info.mod.id, info]));
  const byName = new Map<string, number>();
  for (const [index, plugin] of loadOrder.entries()) {
    byName.set(plugin.name.toLowerCase(), index);
  }
  const nodes: PluginNode[] = loadOrder.map((plugin, index) => {
    const known = plugin.provider == null ? undefined : modsById.get(plugin.provider.id);
    return {
      index,
      name: plugin.name,
      origin: plugin.origin,
      modId: plugin.provider?.id ?? null,
      modName: known?.mod.name ?? plugin.provider?.name ?? null,
      hasPage: known !== undefined,
      masters: [],
      missing: [],
      dependents: [],
    };
  });
  let links = 0;
  for (const [index, plugin] of loadOrder.entries()) {
    const node = nodes[index];
    if (node === undefined) {
      continue;
    }
    for (const master of plugin.masters) {
      const at = byName.get(master.toLowerCase());
      const other = at === undefined ? undefined : nodes[at];
      if (at === undefined || other === undefined) {
        node.missing.push(master);
      } else if (at !== index && !node.masters.includes(at)) {
        node.masters.push(at);
        other.dependents.push(index);
        links += 1;
      }
    }
  }
  const bands: Band[] = [];
  for (const node of nodes) {
    const last = bands.at(-1);
    if (last !== undefined && last.origin === node.origin) {
      last.to = node.index;
    } else {
      bands.push({ origin: node.origin, from: node.index, to: node.index });
    }
  }
  return { nodes, byName, links, bands };
}

/** The places of the plugins a mod brings. */
export function pluginsOfMod(graph: DependencyGraph, modId: string): number[] {
  return graph.nodes.filter((node) => node.modId === modId).map((node) => node.index);
}

/** Plugins that belong together in a list: those of one mod, or one plugin of the game. */
export interface Group {
  key: string;
  /** The mod, if the plugins come from one with a page. */
  modId: string | null;
  /** What to call the group: the mod's name, or the plugin's name without extension. */
  label: string;
  /** The plugins, by place. */
  plugins: number[];
  /** Whether the group is one plugin of no mod, named by the plugin alone. */
  solo: boolean;
}

/** What a set of plugins needs, and what needs them. */
export interface Neighbours {
  /** The plugins looked at, by place. */
  focus: number[];
  /** The loaded plugins they need, outside the set, by mod. */
  needs: Group[];
  /** The plugins outside the set that need one of them, by mod. */
  neededBy: Group[];
  /** Plugins they need that are not loaded: which plugin needs which. */
  missing: { plugin: string; master: string }[];
  /** How many plugins outside the set need one of them. */
  neededByPlugins: number;
  /** How many mods those plugins come from; a plugin of no known mod counts alone. */
  neededByMods: number;
}

/** A plugin's name without its extension, as people call it. */
export function stem(name: string): string {
  const dot = name.lastIndexOf(".");
  return dot > 0 ? name.slice(0, dot) : name;
}

function groupsOf(graph: DependencyGraph, places: number[]): Group[] {
  const groups = new Map<string, Group>();
  for (const place of [...places].sort((a, b) => a - b)) {
    const node = graph.nodes[place];
    if (node === undefined) {
      continue;
    }
    const key = node.modId === null ? `plugin:${node.name.toLowerCase()}` : `mod:${node.modId}`;
    let group = groups.get(key);
    if (group === undefined) {
      group = {
        key,
        modId: node.hasPage ? node.modId : null,
        label: node.modName ?? stem(node.name),
        plugins: [],
        solo: node.modId === null,
      };
      groups.set(key, group);
    }
    group.plugins.push(place);
  }
  return [...groups.values()];
}

/** What the plugins at `focus` need, and what needs them. */
export function neighbours(graph: DependencyGraph, focus: number[]): Neighbours {
  const inFocus = new Set(focus);
  const needs = new Set<number>();
  const neededBy = new Set<number>();
  const missing: { plugin: string; master: string }[] = [];
  for (const place of focus) {
    const node = graph.nodes[place];
    if (node === undefined) {
      continue;
    }
    for (const master of node.masters) {
      if (!inFocus.has(master)) {
        needs.add(master);
      }
    }
    for (const dependent of node.dependents) {
      if (!inFocus.has(dependent)) {
        neededBy.add(dependent);
      }
    }
    for (const master of node.missing) {
      missing.push({ plugin: node.name, master });
    }
  }
  const neededByGroups = groupsOf(graph, [...neededBy]);
  return {
    focus,
    needs: groupsOf(graph, [...needs]),
    neededBy: neededByGroups,
    missing,
    neededByPlugins: neededBy.size,
    neededByMods: neededByGroups.length,
  };
}

/** The plugins that need others most, by how many need them, the most first. */
export function mostNeeded(graph: DependencyGraph, count: number): PluginNode[] {
  return [...graph.nodes]
    .filter((node) => node.origin === "mod" && node.dependents.length > 0)
    .sort((a, b) => b.dependents.length - a.dependents.length || a.index - b.index)
    .slice(0, count);
}

/** The plugins whose name or mod contains `needle`, in load order. */
export function search(graph: DependencyGraph, needle: string): PluginNode[] {
  const wanted = needle.trim().toLowerCase();
  if (wanted === "") {
    return [];
  }
  return graph.nodes.filter(
    (node) =>
      node.name.toLowerCase().includes(wanted) ||
      (node.modName ?? "").toLowerCase().includes(wanted),
  );
}

/** Where the Mods page is to open: a mod's page, by its identity, or a plugin in the diagram, by file name. */
export interface DependencyTarget {
  mod?: string;
  plugin?: string;
}
