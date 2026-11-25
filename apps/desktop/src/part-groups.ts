// The parts of a passage as lists to pick from (ADR 0080): readable names,
// parts of the same name together with a count, picking several rows the
// way a file list does, and what of the losing mod still collides after a
// decision. Pure, so that a script can test it without a window.

/** A part's name to read: the model's file name without its folder and ending. */
export function readableName(name: string): string {
  const file = name.split(/[\\/]/).pop() ?? name;
  const bare = file.replace(/\.nif$/i, "").trim();
  return bare === "" ? name : bare;
}

/** Parts of one name, in the order they came. */
export interface NameGroup<T> {
  name: string;
  members: T[];
}

/**
 * The parts by their readable name, the names in alphabetical order
 * (ignoring case), the parts of a name in the order they came.
 */
export function groupByName<T extends { name: string }>(parts: readonly T[]): NameGroup<T>[] {
  const groups = new Map<string, NameGroup<T>>();
  for (const part of parts) {
    const name = readableName(part.name);
    const group = groups.get(name.toLowerCase()) ?? { name, members: [] };
    group.members.push(part);
    groups.set(name.toLowerCase(), group);
  }
  return [...groups.values()].sort((a, b) =>
    a.name.localeCompare(b.name, undefined, { sensitivity: "base" }),
  );
}

/** How a click picks: only this row, this row added or taken away (Ctrl), or every row up to it (Shift). */
export type PickHow = "only" | "toggle" | "range";

/**
 * What is picked after a click on row `row` of `rows` (each row the keys
 * it stands for: one part, or every part of a name). `anchor` is the row
 * clicked last without Shift; a range runs from it to `row` and is added
 * to what was picked before it was started.
 */
export function pickRows(
  current: ReadonlySet<string>,
  rows: readonly (readonly string[])[],
  anchor: number | null,
  row: number,
  how: PickHow,
): Set<string> {
  const keys = rows[row] ?? [];
  if (how === "toggle") {
    const next = new Set(current);
    const all = keys.length > 0 && keys.every((key) => next.has(key));
    for (const key of keys) {
      if (all) {
        next.delete(key);
      } else {
        next.add(key);
      }
    }
    return next;
  }
  if (how === "range" && anchor !== null && rows[anchor] !== undefined) {
    const next = new Set(current);
    const [low, high] = anchor <= row ? [anchor, row] : [row, anchor];
    for (let at = low; at <= high; at += 1) {
      for (const key of rows[at] ?? []) {
        next.add(key);
      }
    }
    return next;
  }
  const same = keys.length === current.size && keys.every((key) => current.has(key));
  // A second plain click on the only picked row lets it go.
  return same ? new Set() : new Set(keys);
}

/** One spot of a place: its key, and its parts with their keys and plugins. */
export interface SpotParts {
  key: string;
  parts: readonly { key: string; plugin: string }[];
}

/** What of the losing mod still collides at a place after a decision. */
export interface StillColliding {
  /** The spots where a part of each side is still there. */
  spots: string[];
  /** The losing mod's parts still there at those spots, by key, each once. */
  parts: string[];
}

/**
 * The parts of `loser` that are not `hidden` at spots where a part of
 * another mod is not hidden either: where the two mods still collide
 * after `loser` was given up there, because the game needs some of its
 * parts there or because they were kept by hand.
 */
export function stillColliding(
  spots: readonly SpotParts[],
  loser: string,
  hidden: ReadonlySet<string>,
): StillColliding {
  const losing = loser.toLowerCase();
  const found: StillColliding = { spots: [], parts: [] };
  const seen = new Set<string>();
  for (const spot of spots) {
    const there = spot.parts.filter((part) => !hidden.has(part.key));
    const mine = there.filter((part) => part.plugin.toLowerCase() === losing);
    const theirs = there.filter((part) => part.plugin.toLowerCase() !== losing);
    if (mine.length === 0 || theirs.length === 0) {
      continue;
    }
    found.spots.push(spot.key);
    for (const part of mine) {
      if (!seen.has(part.key)) {
        seen.add(part.key);
        found.parts.push(part.key);
      }
    }
  }
  return found;
}

/** What a part is, roughly, by where its model lies: to tell a list of names apart at a glance. */
export type PartKind =
  | "building"
  | "furniture"
  | "clutter"
  | "light"
  | "door"
  | "plant"
  | "rock"
  | "effect"
  | "other";

/** The folders of models and the kind each stands for, the most telling first. */
const KIND_FOLDERS: [RegExp, PartKind][] = [
  [/(^|\/)(doors?\/|[^/]*door[^/]*\.nif$)/i, "door"],
  [/(^|\/)(furniture|beds?|chairs?|tables?|benches?)(\/|$)/i, "furniture"],
  [/(^|\/)(lights?|lighting|candles?|torch(es)?|lanterns?)(\/|$)/i, "light"],
  [/(^|\/)(plants|trees|flora|grass)(\/|$)/i, "plant"],
  [/(^|\/)(rocks?|cliffs?|landscape|mountains?)(\/|$)/i, "rock"],
  [/(^|\/)(effects|fx|magic)(\/|$)/i, "effect"],
  [/(^|\/)(clutter|containers?|loot|props?)(\/|$)/i, "clutter"],
  [/(^|\/)(architecture|buildings?|dungeons|towns?|walls?|bridges?|docks?)(\/|$)/i, "building"],
];

/** The kind of a part from its model's path below the data directory; "other" where it says nothing. */
export function partKind(model: string): PartKind {
  const path = model.replaceAll("\\", "/").replace(/^(data\/)?meshes\//i, "");
  for (const [folder, kind] of KIND_FOLDERS) {
    if (folder.test(path)) {
      return kind;
    }
  }
  return "other";
}
