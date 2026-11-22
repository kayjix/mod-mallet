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

import { withDoorMoves } from "./doors";
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
  objectChoiceKey,
  objectsOf,
  type Pair,
  type PatchChoices,
  partsOf,
} from "./patch";
import { repairsOf } from "./repair-paths";
import { splitWaiting } from "./switched-off";

/** The prefix of a pin among the choices; a pin is no decision. */
const PIN = "pin:";

/** The prefix of a file choice of Asset collisions among a patch's decisions. */
export const DUEL = "duel:";

/** The name the first patch gets, as patches had before there were several. */
export const FIRST_PATCH_NAME = "Mod Mallet Patch";

/** The version of the manifest's format this window writes. */
const MANIFEST_FORMAT = 1;

/** A new identity for a patch, different from those in the book. */
export function newPatchId(book: PatchBook | null): string {
  const taken = new Set((book?.patches ?? []).map((patch) => patch.id));
  for (let attempt = 0; ; attempt += 1) {
    const id = `p${Date.now().toString(36)}${attempt === 0 ? "" : `-${attempt}`}`;
    if (!taken.has(id)) {
      return id;
    }
  }
}

/** A name not used by another patch of the book: `wanted`, or with a number after it. */
export function freeName(book: PatchBook, wanted: string, except?: string): string {
  const taken = new Set(
    book.patches.filter((patch) => patch.id !== except).map((patch) => patch.name.toLowerCase()),
  );
  const base = wanted.trim() === "" ? FIRST_PATCH_NAME : wanted.trim();
  if (!taken.has(base.toLowerCase())) {
    return base;
  }
  for (let number = 2; ; number += 1) {
    const name = `${base} (${number})`;
    if (!taken.has(name.toLowerCase())) {
      return name;
    }
  }
}

/**
 * The book made the first time from the choices as they were kept before
 * there were several patches: one patch with every decision, the pins
 * apart.
 */
export function bookFromChoices(choices: PatchChoices): PatchBook {
  const decisions: PatchChoices = {};
  const pins: string[] = [];
  for (const [key, value] of Object.entries(choices)) {
    if (key.startsWith(PIN)) {
      pins.push(key);
    } else {
      decisions[key] = value;
    }
  }
  const id = newPatchId(null);
  return {
    patches: [{ id, name: FIRST_PATCH_NAME, decisions, unticked: [] }],
    active: id,
    pins,
  };
}

/** Whether a patch is somebody else's, whose decisions the window never changes. */
export const isForeign = (patch: BookPatch) => patch.foreign === true;

/** The patch new decisions go to: the active one, or else the first of the user's own. */
export function activePatch(book: PatchBook): BookPatch | undefined {
  return (
    book.patches.find((patch) => patch.id === book.active && !isForeign(patch)) ??
    book.patches.find((patch) => !isForeign(patch))
  );
}

/** A patch's decisions that are ticked. */
export function tickedOf(patch: BookPatch): PatchChoices {
  const unticked = new Set(patch.unticked ?? []);
  if (unticked.size === 0) {
    return patch.decisions;
  }
  return Object.fromEntries(Object.entries(patch.decisions).filter(([key]) => !unticked.has(key)));
}

/** The patches that count, in the order in which they count: loaded ones first. */
export function countedPatches(book: PatchBook): BookPatch[] {
  const on = book.patches.filter((patch) => patch.off !== true);
  return [...on.filter(isForeign), ...on.filter((patch) => !isForeign(patch))];
}

/** The choices every page reads: the pins, and the ticked decisions of every patch that counts. */
export function countedChoices(book: PatchBook): PatchChoices {
  const choices: PatchChoices = {};
  for (const pin of book.pins ?? []) {
    choices[pin] = "pinned";
  }
  for (const patch of countedPatches(book)) {
    Object.assign(choices, tickedOf(patch));
  }
  return choices;
}

/** A copy of a patch with its decisions and unticked keys copied too, to change. */
function copyOf(patch: BookPatch): BookPatch {
  return { ...patch, decisions: { ...patch.decisions }, unticked: [...(patch.unticked ?? [])] };
}

/**
 * The book with a page's change put in: `before` is what the page read,
 * `after` what it wants. A changed decision goes to the user's own patch
 * that decides it now (the last that counts), or else to the active one.
 * A decision taken back is taken out of the user's own patches; in a patch
 * by someone else it is unticked, which leaves its author's decisions as
 * they are.
 */
export function withChange(book: PatchBook, before: PatchChoices, after: PatchChoices): PatchBook {
  const keys = new Set([...Object.keys(before), ...Object.keys(after)]);
  const changed = [...keys].filter(
    (key) => before[key] !== after[key] || key in before !== key in after,
  );
  if (changed.length === 0) {
    return book;
  }
  const patches = book.patches.map(copyOf);
  let pins = [...(book.pins ?? [])];
  const byId = new Map(patches.map((patch) => [patch.id, patch]));
  const counted = countedPatches({ ...book, patches }).map((patch) => byId.get(patch.id) ?? patch);
  const active = activePatch({ ...book, patches });
  for (const key of changed) {
    if (key.startsWith(PIN)) {
      pins = pins.filter((pin) => pin !== key);
      if (key in after) {
        pins.push(key);
      }
      continue;
    }
    const holders = counted.filter(
      (patch) => key in patch.decisions && !(patch.unticked ?? []).includes(key),
    );
    if (key in after) {
      const own = [...holders].reverse().find((patch) => !isForeign(patch));
      const target = own ?? active;
      if (target === undefined) {
        continue;
      }
      target.decisions[key] = after[key] ?? null;
      target.unticked = (target.unticked ?? []).filter((unticked) => unticked !== key);
    } else {
      for (const patch of holders) {
        if (isForeign(patch)) {
          patch.unticked = [...(patch.unticked ?? []), key];
        } else {
          delete patch.decisions[key];
        }
      }
    }
  }
  return { ...book, patches, pins };
}

/** The book with one patch changed by `change`. */
export function withPatch(
  book: PatchBook,
  id: string,
  change: (patch: BookPatch) => BookPatch,
): PatchBook {
  return {
    ...book,
    patches: book.patches.map((patch) => (patch.id === id ? change(copyOf(patch)) : patch)),
  };
}

/** The book with a new empty patch of the user's own, which becomes the active one. */
export function withNewPatch(book: PatchBook, name: string): { book: PatchBook; id: string } {
  const id = newPatchId(book);
  return {
    id,
    book: {
      ...book,
      active: id,
      patches: [...book.patches, { id, name: freeName(book, name), decisions: {}, unticked: [] }],
    },
  };
}

/** The book without a patch. Another of the user's own becomes active; one is made if none is left. */
export function withoutPatch(book: PatchBook, id: string): PatchBook {
  const patches = book.patches.filter((patch) => patch.id !== id);
  const rest = { ...book, patches };
  if (patches.some((patch) => !isForeign(patch))) {
    return { ...rest, active: activePatch(rest)?.id ?? rest.active };
  }
  return withNewPatch(rest, FIRST_PATCH_NAME).book;
}

/** The book with decisions of one patch ticked or unticked. */
export function withTicked(
  book: PatchBook,
  id: string,
  keys: string[],
  ticked: boolean,
): PatchBook {
  return withPatch(book, id, (patch) => {
    const rest = (patch.unticked ?? []).filter((key) => !keys.includes(key));
    return { ...patch, unticked: ticked ? rest : [...rest, ...keys] };
  });
}

/**
 * The book with decisions moved or copied from one patch to another. A
 * patch by someone else gives copies only; nothing goes into one.
 */
export function withMoved(
  book: PatchBook,
  from: string,
  to: string,
  keys: string[],
  copy: boolean,
): PatchBook {
  const source = book.patches.find((patch) => patch.id === from);
  const target = book.patches.find((patch) => patch.id === to);
  if (source === undefined || target === undefined || isForeign(target) || from === to) {
    return book;
  }
  const keep = copy || isForeign(source);
  let next = withPatch(book, to, (patch) => {
    for (const key of keys) {
      if (key in source.decisions) {
        patch.decisions[key] = source.decisions[key] ?? null;
      }
    }
    patch.unticked = (patch.unticked ?? []).filter((key) => !keys.includes(key));
    return patch;
  });
  if (!keep) {
    next = withPatch(next, from, (patch) => {
      for (const key of keys) {
        delete patch.decisions[key];
      }
      patch.unticked = (patch.unticked ?? []).filter((key) => !keys.includes(key));
      return patch;
    });
  }
  return next;
}

/** The book with a loaded patch added, as somebody else's when it names an author. */
export function withLoaded(
  book: PatchBook,
  loaded: {
    archive: string;
    name: string;
    author: string | null;
    version: string | null;
    decisions: PatchChoices;
  },
): { book: PatchBook; id: string } {
  const id = newPatchId(book);
  const foreign = loaded.author !== null && loaded.author.trim() !== "";
  const patch: BookPatch = {
    id,
    name: freeName(book, loaded.name),
    author: loaded.author,
    version: loaded.version,
    loadedFrom: loaded.archive,
    foreign,
    decisions: { ...loaded.decisions },
    unticked: [],
  };
  return {
    id,
    book: { ...book, patches: [...book.patches, patch], active: foreign ? book.active : id },
  };
}

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
    if (key.startsWith(DUEL)) {
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
  const repairs = repairsOf(active, hidden);
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
