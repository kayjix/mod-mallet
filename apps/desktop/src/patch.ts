// The patch's choice of objects, shared by the patch view and the task list.
//
// For each collision the user picks the mod whose objects disappear. The
// choice lives in the window, starts from the patch that is installed, and
// is turned into a patch only on the patch view.

import { isCollision } from "./arrange";
import type { MapSpot, ObjectRef, RenderedReport } from "./generated/rendered-report";

/** The chosen mod per collision finding; null keeps both. */
export type PatchChoices = Record<string, string | null>;

/** One collision finding with what can be hidden of it, side by side. */
export interface Pair {
  id: string;
  headline: string;
  /** The two plugins, each with the objects of it that can be hidden. */
  sides: { plugin: string; objects: ObjectRef[] }[];
  /** The plugin suggested for hiding, if any. */
  suggested: string | null;
}

/** The pair of one collision finding, from its spots; null if nothing in it can be hidden. */
export function pairFor(id: string, headline: string, found: MapSpot[]): Pair | null {
  const sides = new Map<string, ObjectRef[]>();
  const votes = new Map<string, number>();
  for (const part of found.flatMap((spot) => spot.parts ?? [])) {
    const { plugin } = part.object;
    sides.set(plugin, [...(sides.get(plugin) ?? []), ...(part.hideable ? [part.object] : [])]);
    if (part.suggested) {
      votes.set(plugin, (votes.get(plugin) ?? 0) + 1);
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
 * The choices the installed patch stands for: per collision, the side of
 * which it hides the most objects.
 */
export function choicesOf(pairs: Pair[], installed: Set<string>): Record<string, string | null> {
  const choices: Record<string, string | null> = {};
  for (const pair of pairs) {
    let best: { plugin: string; hidden: number } | null = null;
    for (const side of pair.sides) {
      const hidden = side.objects.filter((object) => installed.has(keyOf(object))).length;
      if (hidden > 0 && (best === null || hidden > best.hidden)) {
        best = { plugin: side.plugin, hidden };
      }
    }
    if (best !== null) {
      choices[pair.id] = best.plugin;
    }
  }
  return choices;
}

/** The objects hidden by a choice. */
export function objectsOf(pairs: Pair[], choices: PatchChoices): ObjectRef[] {
  return pairs.flatMap(
    (pair) => pair.sides.find((side) => side.plugin === (choices[pair.id] ?? null))?.objects ?? [],
  );
}

/** The objects of the patch in the game, by key. */
export function installedKeys(report: RenderedReport): Set<string> {
  return new Set((report.installedPatch?.objects ?? []).map(keyOf));
}
