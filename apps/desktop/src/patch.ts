// The patch's choice of objects, shared by the patch view, the task list
// and the collisions' worklist.
//
// For each collision the user picks the mod whose objects disappear: for
// the whole finding at once, or spot by spot where one spot needs another
// answer than the rest. The choice lives in the window, starts from the
// patch that is installed, and is turned into a patch only on the patch
// view.

import { isCollision } from "./arrange";
import type { MapSpot, ObjectRef, RenderedReport, SpotPart } from "./generated/rendered-report";

/**
 * The chosen mod per collision finding (by its id) or per spot (by
 * {@link spotKey}); null keeps both. A spot's own choice wins over its
 * finding's.
 */
export type PatchChoices = Record<string, string | null>;

/** One spot of a collision with what of it can be hidden. */
export interface PairSpot {
  key: string;
  parts: SpotPart[];
}

/** One collision finding with what can be hidden of it, side by side. */
export interface Pair {
  id: string;
  headline: string;
  /** The two plugins, each with the objects of it that can be hidden. */
  sides: { plugin: string; objects: ObjectRef[] }[];
  /** The plugin suggested for hiding, if any. */
  suggested: string | null;
  /** Its spots that name their objects. */
  spots: PairSpot[];
}

/** The prefix of a spot's own choice among the {@link PatchChoices}. */
const SPOT = "spot:";

/**
 * The prefix of an object hidden on its own, outside any collision (or
 * besides it); the value is the plugin that placed it.
 */
const OBJECT = "object:";

/** An object's identity among the choices, when it is hidden on its own. */
export const objectChoiceKey = (object: ObjectRef) => `${OBJECT}${object.plugin}|${object.localId}`;

/** The objects hidden on their own. */
export function freeObjects(choices: PatchChoices): ObjectRef[] {
  return Object.entries(choices).flatMap(([key, plugin]) => {
    if (!key.startsWith(OBJECT) || plugin === null) {
      return [];
    }
    const localId = Number(key.slice(key.lastIndexOf("|") + 1));
    return Number.isInteger(localId) ? [{ plugin, localId }] : [];
  });
}

/** The choices with one object hidden on its own, or no longer. */
export function chooseObject(
  choices: PatchChoices,
  object: ObjectRef,
  hide: boolean,
): PatchChoices {
  const next = { ...choices };
  if (hide) {
    next[objectChoiceKey(object)] = object.plugin;
  } else {
    delete next[objectChoiceKey(object)];
  }
  return next;
}

/**
 * A spot's identity among the choices: the objects in it, which do not
 * depend on the load order (ADR 0007); null for a spot that names none.
 */
export function spotKey(spot: MapSpot): string | null {
  const parts = spot.parts ?? [];
  return parts.length === 0
    ? null
    : `${SPOT}${spot.finding}|${signatureOf(parts.map((part) => part.object))}`;
}

/** The pair of one collision finding, from its spots; null if nothing in it can be hidden. */
export function pairFor(id: string, headline: string, found: MapSpot[]): Pair | null {
  const sides = new Map<string, ObjectRef[]>();
  const votes = new Map<string, number>();
  const spots: PairSpot[] = [];
  for (const spot of found) {
    const key = spotKey(spot);
    if (key !== null) {
      spots.push({ key, parts: spot.parts ?? [] });
    }
    for (const part of spot.parts ?? []) {
      const { plugin } = part.object;
      sides.set(plugin, [...(sides.get(plugin) ?? []), ...(part.hideable ? [part.object] : [])]);
      if (part.suggested) {
        votes.set(plugin, (votes.get(plugin) ?? 0) + 1);
      }
    }
  }
  if (sides.size === 0) {
    return null;
  }
  const suggested = [...votes.entries()].sort((x, y) => y[1] - x[1])[0]?.[0] ?? null;
  return {
    id,
    headline,
    sides: [...sides.entries()].map(([plugin, objects]) => ({ plugin, objects })),
    suggested,
    spots,
  };
}

export function pairsOf(report: RenderedReport, ignored: Set<string>): Pair[] {
  const spots = new Map<string, MapSpot[]>();
  for (const spot of report.map.spots) {
    spots.set(spot.finding, [...(spots.get(spot.finding) ?? []), spot]);
  }
  return report.findings
    .filter((rendered) => isCollision(rendered) && !ignored.has(rendered.finding.id))
    .flatMap((rendered) => {
      const pair = pairFor(
        rendered.finding.id,
        rendered.headline,
        spots.get(rendered.finding.id) ?? [],
      );
      return pair === null ? [] : [pair];
    });
}

/** One object as a text, for comparing. */
export function keyOf(object: ObjectRef): string {
  return `${object.plugin}:${object.localId}`;
}

/** A text that is the same for the same choice of objects, in any order. */
export function signatureOf(objects: ObjectRef[]): string {
  return [...new Set(objects.map(keyOf))].sort().join(",");
}

/**
 * What is decided for a spot: the plugin whose objects go, null to keep
 * both, undefined while nothing is decided.
 */
export function decisionAt(
  choices: PatchChoices,
  finding: string,
  key: string | null,
): string | null | undefined {
  if (key !== null && key in choices) {
    return choices[key] ?? null;
  }
  return finding in choices ? (choices[finding] ?? null) : undefined;
}

/** The choices with `plugin` chosen for a whole finding, its spots' own choices dropped. */
export function chooseFinding(
  choices: PatchChoices,
  pair: Pair,
  plugin: string | null,
): PatchChoices {
  const next: PatchChoices = { ...choices, [pair.id]: plugin };
  for (const spot of pair.spots) {
    delete next[spot.key];
  }
  return next;
}

/** The choices with `plugin` chosen for one spot. */
export function chooseSpot(
  choices: PatchChoices,
  key: string,
  plugin: string | null,
): PatchChoices {
  return { ...choices, [key]: plugin };
}

/** The choices without a spot's own choice: it follows its finding again. */
export function undecideSpot(choices: PatchChoices, key: string): PatchChoices {
  const next = { ...choices };
  delete next[key];
  return next;
}

/** The objects of a spot that hiding `plugin` there takes away. */
export function hiddenAt(parts: SpotPart[], plugin: string | null): ObjectRef[] {
  return parts
    .filter((part) => part.hideable && plugin !== null && part.object.plugin === plugin)
    .map((part) => part.object);
}

/**
 * The choices the installed patch stands for: per collision, the side of
 * which it hides the most objects, and per spot where the patch hides
 * something else there.
 */
export function choicesOf(
  pairs: Pair[],
  installed: Set<string>,
  installedObjects: ObjectRef[] = [],
): PatchChoices {
  const choices: PatchChoices = {};
  // What no collision names was hidden on its own.
  const named = new Set(
    pairs.flatMap((pair) =>
      pair.spots.flatMap((spot) => spot.parts.map((part) => keyOf(part.object))),
    ),
  );
  for (const object of installedObjects) {
    if (!named.has(keyOf(object))) {
      choices[objectChoiceKey(object)] = object.plugin;
    }
  }
  const hiddenSide = (sides: { plugin: string; objects: ObjectRef[] }[]) => {
    let best: { plugin: string; hidden: number } | null = null;
    for (const side of sides) {
      const hidden = side.objects.filter((object) => installed.has(keyOf(object))).length;
      if (hidden > 0 && (best === null || hidden > best.hidden)) {
        best = { plugin: side.plugin, hidden };
      }
    }
    return best?.plugin ?? null;
  };
  for (const pair of pairs) {
    const whole = hiddenSide(pair.sides);
    if (whole === null) {
      continue;
    }
    choices[pair.id] = whole;
    for (const spot of pair.spots) {
      const plugins = [...new Set(spot.parts.map((part) => part.object.plugin))];
      const here = hiddenSide(
        plugins.map((plugin) => ({ plugin, objects: hiddenAt(spot.parts, plugin) })),
      );
      if (here !== whole) {
        choices[spot.key] = here;
      }
    }
  }
  return choices;
}

/** The objects hidden by a choice, each once. */
export function objectsOf(pairs: Pair[], choices: PatchChoices): ObjectRef[] {
  const found = new Map<string, ObjectRef>();
  const add = (objects: ObjectRef[]) => {
    for (const object of objects) {
      found.set(keyOf(object), object);
    }
  };
  for (const pair of pairs) {
    const whole = choices[pair.id] ?? null;
    if (pair.spots.length === 0) {
      add(pair.sides.find((side) => side.plugin === whole)?.objects ?? []);
      continue;
    }
    for (const spot of pair.spots) {
      const plugin = spot.key in choices ? (choices[spot.key] ?? null) : whole;
      add(hiddenAt(spot.parts, plugin));
    }
  }
  add(freeObjects(choices));
  return [...found.values()];
}

/** The objects of the patch in the game, by key. */
export function installedKeys(report: RenderedReport): Set<string> {
  return new Set((report.installedPatch?.objects ?? []).map(keyOf));
}
