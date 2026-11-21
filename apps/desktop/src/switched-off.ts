// Decisions about mods that are switched off (ADR 0061).
//
// What the user decided while a mod was on is kept when the mod goes off:
// they may switch it on again. While it is off and the game no longer
// loads its plugin, those decisions wait. (While the game still loads it,
// its collisions are real and the decisions count as before.) They
// do not count as decided, do not go into the patch and do not show as
// open work. The window keeps every choice, and hands its pages only those
// that do not wait; what a page changes is put back together with the
// waiting ones before it is saved.

import type { InstalledPatch, SwitchedOffPlugin } from "./generated/rendered-report";
import type { PatchChoices } from "./patch";

/** The decisions that wait for mods that are switched off. */
export interface Waiting {
  /** The choices that wait, as they are kept. */
  choices: PatchChoices;
  /** How many decisions wait: per collision, per spot, per object; pins left out. */
  decisions: number;
  /** The mods they wait for, by the names the mod manager shows. */
  mods: string[];
  /** The objects of switched-off plugins the patch in the game still hides. */
  installed: number;
  /** The plugins of those objects. */
  installedPlugins: string[];
  /** The plugins of switched-off mods the game still loads: nothing about them waits. */
  stillLoaded: string[];
}

/** No decision waits. */
export const NOTHING_WAITS: Waiting = {
  choices: {},
  decisions: 0,
  mods: [],
  installed: 0,
  installedPlugins: [],
  stillLoaded: [],
};

/** Characters that may stand before a plugin's name in a choice's key. */
const BEFORE = new Set([":", "/", "|", ","]);
/** Characters that may stand after it. */
const AFTER = new Set(["+", "|", ":", ","]);

/**
 * The plugins of `plugins` (in lower case) that a choice's key names:
 * a collision by the files of its two plugins, a spot by its objects, an
 * object hidden on its own by its plugin. Pins name their spot.
 */
export function pluginsInKey(key: string, plugins: Iterable<string>): string[] {
  const lower = key.toLowerCase();
  const found: string[] = [];
  for (const plugin of plugins) {
    let from = 0;
    for (;;) {
      const at = lower.indexOf(plugin, from);
      if (at < 0) {
        break;
      }
      const before = at === 0 ? undefined : lower[at - 1];
      const after = lower[at + plugin.length];
      if (
        (before === undefined || BEFORE.has(before)) &&
        (after === undefined || AFTER.has(after))
      ) {
        found.push(plugin);
        break;
      }
      from = at + 1;
    }
  }
  return found;
}

/** The plugin an installed object was hidden against, by its key in lower case. */
function againstOf(installedPatch: InstalledPatch | null | undefined): Map<string, string> {
  const found = new Map<string, string>();
  for (const known of installedPatch?.patches ?? []) {
    for (const part of known.patch.parts) {
      if (part.against == null) {
        continue;
      }
      for (const object of part.objects) {
        found.set(`${object.plugin}|${object.localId}`.toLowerCase(), part.against.toLowerCase());
      }
    }
  }
  return found;
}

/**
 * The choices split into those that count and those that wait for a mod
 * that is switched off. An object hidden on its own waits as well when the
 * patch in the game hid it against a plugin of such a mod.
 */
export function splitWaiting(
  choices: PatchChoices,
  switchedOff: SwitchedOffPlugin[],
  installedPatch?: InstalledPatch | null,
): { active: PatchChoices; waiting: Waiting } {
  const stillLoaded = switchedOff
    .filter((plugin) => plugin.loaded)
    .map((plugin) => plugin.plugin)
    .sort((a, b) => a.localeCompare(b));
  const gone = switchedOff.filter((plugin) => !plugin.loaded);
  if (gone.length === 0) {
    return { active: choices, waiting: { ...NOTHING_WAITS, stillLoaded } };
  }
  const modOf = new Map<string, string>();
  for (const plugin of gone) {
    modOf.set(plugin.plugin.toLowerCase(), plugin.mod.name);
  }
  const plugins = [...modOf.keys()];
  const against = againstOf(installedPatch);
  const active: PatchChoices = {};
  const waitingChoices: PatchChoices = {};
  const mods = new Set<string>();
  let decisions = 0;
  for (const [key, value] of Object.entries(choices)) {
    let named = pluginsInKey(key, plugins);
    if (named.length === 0 && key.startsWith("object:")) {
      const other = against.get(key.slice("object:".length).toLowerCase());
      named = other !== undefined && modOf.has(other) ? [other] : [];
    }
    if (named.length === 0) {
      active[key] = value;
      continue;
    }
    waitingChoices[key] = value;
    if (!key.startsWith("pin:") && !(key.startsWith("object:") && value === null)) {
      decisions += 1;
    }
    for (const plugin of named) {
      mods.add(modOf.get(plugin) ?? plugin);
    }
  }
  const installedPlugins = new Set<string>();
  let installed = 0;
  for (const object of installedPatch?.objects ?? []) {
    const plugin = object.plugin.toLowerCase();
    if (modOf.has(plugin)) {
      installed += 1;
      installedPlugins.add(
        gone.find((known) => known.plugin.toLowerCase() === plugin)?.plugin ?? object.plugin,
      );
    }
  }
  return {
    active,
    waiting: {
      choices: waitingChoices,
      decisions,
      mods: [...mods].sort((a, b) => a.localeCompare(b)),
      installed,
      installedPlugins: [...installedPlugins].sort((a, b) => a.localeCompare(b)),
      stillLoaded,
    },
  };
}

/** The choices a page made, with the waiting ones put back. */
export function withWaiting(choices: PatchChoices, waiting: Waiting): PatchChoices {
  return { ...waiting.choices, ...choices };
}
