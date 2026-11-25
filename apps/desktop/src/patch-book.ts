// Several patches, each a set of decisions with a name (ADR 0070).
//
// The window keeps a patch book per game: the patches in an order, the one
// new decisions go to, and the pins, which belong to none. Every page reads
// and writes one map of choices, as before (`PatchChoices`); this module
// turns the book into that map and puts a page's change back into the
// right patch.
//
// What counts: the ticked decisions of every patch that is not switched
// off. Patches by someone else come first and the user's own after, each
// in the order of the book; on the same key the later one wins, so the
// user's own decisions win over loaded ones.

import { doorRelinksOf, placeName, withDoorMoves } from "./doors";
import type { BookPatch, PatchBook } from "./generated/patch-book";
import type {
  MapSpot,
  ObjectRef,
  Patch,
  PatchManifest,
  RenderedReport,
} from "./generated/rendered-report";
import { objectMovesOf } from "./moves";
import {
  decisionAt,
  matchKey,
  objectChoiceKey,
  objectsOf,
  type Pair,
  type PatchChoices,
  partsOf,
} from "./patch";
import { repairsOf } from "./repair-paths";
import { splitWaiting } from "./switched-off";

export * from "./book";

import { countedPatches, DUEL, FIRST_PATCH_NAME, PIN, tickedOf } from "./book";

/** The version of the manifest's format this window writes. */
const MANIFEST_FORMAT = 1;
/** The decisions an archive without a manifest stands for: each object hidden on its own. */
export function decisionsOfObjects(objects: ObjectRef[]): PatchChoices {
  return Object.fromEntries(objects.map((object) => [objectChoiceKey(object), object.plugin]));
}

/** Plugin file names in a decision's key, as they are spelt there. */
const PLUGIN_IN_KEY = /[^:|/,+]+?\.(?:esp|esm|esl)(?=$|[:|/,+])/gi;

/** The plugins a patch's decisions are about, each once, by lower-case name. */
export function pluginsOf(decisions: PatchChoices): Map<string, string> {
  const found = new Map<string, string>();
  for (const [key, value] of Object.entries(decisions)) {
    if (key.startsWith(DUEL) || key.startsWith(NO_REPAIR)) {
      continue;
    }
    for (const match of key.matchAll(PLUGIN_IN_KEY)) {
      const name = match[0].replace(/^(?:file|data|object|spot|move|repair)$/i, "").trim();
      if (name !== "" && !found.has(name.toLowerCase())) {
        found.set(name.toLowerCase(), name);
      }
    }
    if (typeof value === "string" && /\.(?:esp|esm|esl)$/i.test(value)) {
      found.set(value.toLowerCase(), found.get(value.toLowerCase()) ?? value);
    }
  }
  return found;
}

/** Where a patch stands in the game. */
export type PatchState = "in-game" | "switched-off" | "not-installed";

/** The mod a patch was installed as, by its archive's name, and whether it is on. */
export function patchMod(
  report: RenderedReport,
  name: string,
): { id: string; source: string; name: string; enabled: boolean } | null {
  const wanted = archiveWord(name).toLowerCase();
  const found = report.mods.find((info) => {
    const names = [info.mod.id, info.mod.name].map((each) =>
      each.toLowerCase().replace(/\.(zip|7z|rar)$/, ""),
    );
    return names.includes(wanted);
  });
  return found === undefined
    ? null
    : {
        id: found.mod.id,
        source: found.mod.source,
        name: found.mod.name,
        enabled: found.enabled,
      };
}

/**
 * A patch's name as its archive and mod carry it: the characters every file
 * system and installer accepts, as the engine writes it.
 */
export function archiveWord(name: string): string {
  const kept = name.replace(/[^A-Za-z0-9 \-.'()&+]/g, " ");
  const word = kept
    .split(/\s+/)
    .filter((part) => part !== "")
    .join(" ")
    .slice(0, 48)
    .replace(/^[. ]+|[. ]+$/g, "");
  return word === "" ? FIRST_PATCH_NAME : word;
}

/** Where a patch stands in the game, from its mod and from what the game's swap files say. */
export function stateOf(report: RenderedReport, patch: BookPatch): PatchState {
  const mod = patchMod(report, patch.name);
  if (mod !== null) {
    return mod.enabled ? "in-game" : "switched-off";
  }
  const installed = (report.installedPatch?.patches ?? []).some(
    (known) => known.patch.name.toLowerCase() === patch.name.toLowerCase(),
  );
  return installed ? "in-game" : "not-installed";
}

/** The objects the game's swap files hide for the patch of this name. */
export function installedObjectsOf(report: RenderedReport, name: string): ObjectRef[] {
  return (report.installedPatch?.patches ?? [])
    .filter((known) => known.patch.name.toLowerCase() === name.toLowerCase())
    .flatMap((known) => known.patch.parts.flatMap((part) => part.objects));
}

/**
 * The patch as it is written and installed: its ticked decisions, without
 * those that wait for switched-off mods (ADR 0061), as parts, moves and
 * repairs, with its manifest.
 */
export function writtenPatch(
  report: RenderedReport,
  pairs: Pair[],
  patch: BookPatch,
  modOfPlugin: (plugin: string) => string | null,
): Patch {
  const ticked = tickedOf(patch);
  const active = splitWaiting(
    ticked,
    report.switchedOffPlugins ?? [],
    report.installedPatch,
  ).active;
  const hidden = objectsOf(pairs, active);
  // With the doors that merged doors lead elsewhere (ADR 0067), which the
  // door decisions of its spots say.
  const spots = new Map<string, MapSpot[]>();
  for (const spot of report.map.spots) {
    spots.set(spot.finding, [...(spots.get(spot.finding) ?? []), spot]);
  }
  const moves = withDoorMoves(objectMovesOf(active), active, spots);
  // The paths over what it hides are repaired by itself, place by place,
  // unless the user turned that off for a place (ADR 0070).
  const repairs = repairsOf(active, hidden);
  for (const area of repairAreasOf(report, hidden)) {
    if (!(noRepairKey(area) in active)) {
      repairs.places.push({ place: area.place, centre: area.centre, navmeshes: [], every: true });
    }
  }
  // The paths of merged doors re-attached to the kept ones (ADR 0088).
  repairs.places.push(...doorRelinksOf(active, spots));
  const plugins = [...pluginsOf(ticked).values()].sort((a, b) => a.localeCompare(b));
  const mods = [
    ...new Set(plugins.map(modOfPlugin).filter((mod): mod is string => mod !== null)),
  ].sort((a, b) => a.localeCompare(b));
  const manifest: PatchManifest = {
    format: MANIFEST_FORMAT,
    name: patch.name,
    author: patch.author ?? null,
    version: patch.version ?? null,
    decisions: Object.fromEntries(Object.entries(ticked).filter(([key]) => !key.startsWith(PIN))),
    plugins,
    mods,
  };
  return {
    name: patch.name,
    author: patch.author ?? null,
    version: patch.version ?? null,
    parts: partsOf(pairs, active),
    moves: moves.length > 0 ? { moves } : null,
    repairs: repairs.places.length > 0 ? repairs : null,
    manifest,
  };
}

/** One place where two patches decide differently. */
export interface Disagreement {
  /** The key both decide, or the spot both decide through different keys. */
  key: string;
  /** What the first patch decides there; null to keep both. */
  first: string | null;
  /** What the second patch decides there. */
  second: string | null;
}

/** Two patches that count and decide some places differently. */
export interface PatchConflict {
  first: BookPatch;
  second: BookPatch;
  places: Disagreement[];
}

/**
 * Every pair of patches that count and decide something differently: the
 * same key with another value, or a spot of a collision decided through
 * different keys with another result (ADR 0070).
 */
export function conflictsOf(book: PatchBook, pairs: Pair[]): PatchConflict[] {
  const counted = countedPatches(book).map((patch) => ({ patch, choices: tickedOf(patch) }));
  const conflicts: PatchConflict[] = [];
  for (const [index, first] of counted.entries()) {
    for (const second of counted.slice(index + 1)) {
      const places: Disagreement[] = [];
      const seen = new Set<string>();
      for (const [key, value] of Object.entries(first.choices)) {
        if (key.startsWith(PIN) || !(key in second.choices)) {
          continue;
        }
        const other = second.choices[key] ?? null;
        if ((value ?? null) !== other) {
          places.push({ key, first: value ?? null, second: other });
          seen.add(key);
        }
      }
      for (const pair of pairs) {
        for (const spot of pair.spots) {
          if (seen.has(spot.key) || seen.has(pair.id)) {
            continue;
          }
          const a = decisionAt(first.choices, pair.id, spot.key, spot.parts);
          const b = decisionAt(second.choices, pair.id, spot.key, spot.parts);
          if (a !== undefined && b !== undefined && a !== b) {
            places.push({ key: spot.key, first: a, second: b });
          }
        }
      }
      if (places.length > 0) {
        conflicts.push({ first: first.patch, second: second.patch, places });
      }
    }
  }
  return conflicts;
}

/** The decisions of a patch that are about a duel's file choice. */
export function duelsOf(decisions: PatchChoices): { id: string; side: number }[] {
  return Object.entries(decisions).flatMap(([key, value]) => {
    const side = Number(value);
    return key.startsWith(DUEL) && Number.isInteger(side)
      ? [{ id: key.slice(DUEL.length), side }]
      : [];
  });
}

/** The prefix of a place whose paths a patch does not repair by itself (ADR 0070). */
export const NO_REPAIR = "norepair:";

/** A place where a patch hides objects, whose paths it repairs by itself. */
export interface RepairArea {
  /** Its identity among a patch's decisions, after {@link NO_REPAIR}. */
  key: string;
  /** The worldspace, or the interior cell by its editor ID. */
  place: string;
  /** Outside, the middle of the cell; inside, null. */
  centre: [number, number] | null;
  /** The place in words. */
  label: string;
  /** How many hidden objects stand there. */
  objects: number;
}

/** The decision that turns repairing the paths of a place off. */
export const noRepairKey = (area: RepairArea) => `${NO_REPAIR}${area.key}`;

/** Game units per side of an outside cell. */
const CELL = 4096;

/**
 * The places where `hidden` objects stand, cell by cell, as the spots of
 * the report place them. Objects no spot names are left out: where they
 * stand is not known here.
 */
export function repairAreasOf(report: RenderedReport, hidden: ObjectRef[]): RepairArea[] {
  const where = new Map<string, MapSpot>();
  for (const spot of report.map.spots) {
    for (const part of spot.parts ?? []) {
      const key = matchKey(part.object);
      if (!where.has(key)) {
        where.set(key, spot);
      }
    }
  }
  const areas = new Map<string, RepairArea>();
  for (const object of hidden) {
    const spot = where.get(matchKey(object));
    if (spot === undefined) {
      continue;
    }
    const location = spot.location;
    const area: RepairArea =
      location.type === "interior"
        ? {
            key: `cell:${location.cell}`,
            place: location.cell,
            centre: null,
            label: "",
            objects: 0,
          }
        : {
            key: `${location.world}|${location.x},${location.y}`,
            place: location.world,
            centre: [Math.round((location.x + 0.5) * CELL), Math.round((location.y + 0.5) * CELL)],
            label: "",
            objects: 0,
          };
    const known = areas.get(area.key) ?? { ...area, label: placeName(location) };
    known.objects += 1;
    areas.set(area.key, known);
  }
  return [...areas.values()].sort((a, b) => a.label.localeCompare(b.label));
}
