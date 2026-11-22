// What a patch decides, entry by entry, in words (ADR 0070).
//
// Each decision of a patch becomes an entry: what was decided, said plainly
// ("JK's Nightgate Inn kept, Cutting Room Floor's LanternPost01 hidden"),
// in a group, with where it was decided, so that the patch page can take
// the user there.

import { placeName } from "./doors";
import type { BookPatch } from "./generated/patch-book";
import type { MapSpot, ObjectRef, RenderedReport, SpotPart } from "./generated/rendered-report";
import { CM_PER_UNIT, isMoveKey, movesOf } from "./moves";
import { keptBy, matchKey, type Pair, type PatchChoices } from "./patch";
import { DUEL, NO_REPAIR, pluginsOf } from "./patch-book";
import type { PatchTexts } from "./patch-texts";
import { REPAIR } from "./repair-paths";
import { pluginsInKey } from "./switched-off";

/** The groups of a patch's entries, in the order the page lists them. */
export const GROUPS = ["collisions", "objects", "moves", "paths", "files", "other"] as const;
export type Group = (typeof GROUPS)[number];

/** Where an entry was decided. */
export type Target =
  | { kind: "spot"; spot: MapSpot }
  | { kind: "pair"; mods: [string, string] }
  | { kind: "map"; spot: MapSpot }
  | { kind: "duel"; id: string };

/** One decision of a patch, in words. */
export interface Entry {
  /** The decision's key, which names the entry. */
  key: string;
  group: Group;
  /** Within the collisions, the pair of mods it is about. */
  section: string | null;
  text: string;
  /** The place, in words, where it has one. */
  where: string | null;
  target: Target | null;
  ticked: boolean;
}

/** What the entries are built from. */
export interface EntryContext {
  report: RenderedReport;
  pairs: Pair[];
  spots: Map<string, MapSpot>;
  p: PatchTexts;
  /** The mod a plugin belongs to, by the name the mod manager shows. */
  modOf: (plugin: string) => string;
}

/** The pair of every collision finding, by the finding's id. */
function pairsById(pairs: Pair[]): Map<string, Pair> {
  return new Map(pairs.map((pair) => [pair.id, pair]));
}

/** Every part of a spot, by its object's {@link matchKey}, with the spot it is in. */
function partsByObject(report: RenderedReport): Map<string, { part: SpotPart; spot: MapSpot }> {
  const found = new Map<string, { part: SpotPart; spot: MapSpot }>();
  for (const spot of report.map.spots) {
    for (const part of spot.parts ?? []) {
      const key = matchKey(part.object);
      if (!found.has(key)) {
        found.set(key, { part, spot });
      }
    }
  }
  return found;
}

/** An object's name: what it is, or else its number in its plugin. */
function nameOf(object: ObjectRef, part: SpotPart | undefined, p: PatchTexts): string {
  return part?.name ?? p.numbered(object.localId.toString(16).padStart(6, "0").toUpperCase());
}

/** The names of several objects, the first few of them. */
function namesOf(parts: SpotPart[]): string {
  const names = [...new Set(parts.map((part) => part.name))];
  return names.length <= 3 ? names.join(", ") : `${names.slice(0, 3).join(", ")} …`;
}

/** The object a key of an object hidden or moved names. */
function objectOfKey(key: string, prefix: string): ObjectRef | null {
  const bar = key.lastIndexOf("|");
  const localId = Number(key.slice(bar + 1));
  return bar > prefix.length && Number.isInteger(localId)
    ? { plugin: key.slice(prefix.length, bar), localId }
    : null;
}

/** The two mods of a pair, sorted, as the heading of its section. */
const sectionOf = (pair: Pair) => [...pair.mods].sort((a, b) => a.localeCompare(b)).join(" ⟷ ");

/** Where a repair was asked for, read from its choice's value. */
function repairPlace(
  value: string | null,
): { place: string; x: number | null; y: number | null } | null {
  if (value === null) {
    return null;
  }
  try {
    const parsed = JSON.parse(value) as { place?: unknown; x?: unknown; y?: unknown };
    return typeof parsed.place === "string"
      ? {
          place: parsed.place,
          x: typeof parsed.x === "number" ? parsed.x : null,
          y: typeof parsed.y === "number" ? parsed.y : null,
        }
      : null;
  } catch {
    return null;
  }
}

/** The spot nearest to a place a repair names, to show it. */
function spotNear(
  report: RenderedReport,
  place: { place: string; x: number | null; y: number | null },
): MapSpot | undefined {
  let best: { spot: MapSpot; distance: number } | undefined;
  for (const spot of report.map.spots) {
    const location = spot.location;
    const here = location.type === "interior" ? location.cell : location.world;
    if (here.toLowerCase() !== place.place.toLowerCase()) {
      continue;
    }
    const [x, y] = spot.position ?? [0, 0, 0];
    const distance =
      place.x === null || place.y === null ? 0 : Math.hypot(x - place.x, y - place.y);
    if (best === undefined || distance < best.distance) {
      best = { spot, distance };
    }
  }
  return best?.spot;
}

/** Every decision of `patch` as an entry, grouped, in a fixed order. */
export function entriesOf(patch: BookPatch, context: EntryContext): Entry[] {
  const { report, pairs, spots, p, modOf } = context;
  const byId = pairsById(pairs);
  const parts = partsByObject(report);
  const unticked = new Set(patch.unticked ?? []);
  const moves = movesOf(patch.decisions);
  const duels = new Map(report.duels.map((duel) => [duel.duel.id, duel]));
  const entries: Entry[] = [];
  const otherMod = (pair: Pair, plugin: string) => {
    const other = pair.sides.find((side) => side.plugin !== plugin)?.plugin;
    return other === undefined ? "" : modOf(other);
  };
  for (const [key, value] of Object.entries(patch.decisions)) {
    if (key.startsWith("pin:") || key.startsWith(NO_REPAIR)) {
      continue;
    }
    const base = { key, ticked: !unticked.has(key), section: null, where: null, target: null };
    const pair = byId.get(key);
    if (pair !== undefined) {
      const places = pair.spots.length;
      const text =
        value === null
          ? p.bothKept(modOf(pair.sides[0]?.plugin ?? ""), modOf(pair.sides[1]?.plugin ?? ""))
          : p.kept(otherMod(pair, value), modOf(value), p.theirObjects);
      entries.push({
        ...base,
        group: "collisions",
        section: sectionOf(pair),
        text: p.everywhere(places, text),
        target: { kind: "pair", mods: pair.mods },
      });
      continue;
    }
    if (key.startsWith("spot:")) {
      const spot = spots.get(key);
      const of = spot === undefined ? undefined : byId.get(spot.finding);
      if (spot === undefined || of === undefined) {
        entries.push({ ...base, group: "other", text: `${key.slice(5, 60)}… — ${p.notFound}` });
        continue;
      }
      const spotParts = spot.parts ?? [];
      const plugins = [...new Set(spotParts.map((part) => part.object.plugin))];
      // Stacked doors merged into one (ADR 0067): one door kept, the rest hidden.
      const keeps = keptBy(value);
      if (keeps !== null) {
        const door = spotParts.find((part) => matchKey(part.object) === matchKey(keeps));
        entries.push({
          ...base,
          group: "moves",
          text: p.doorMerged(modOf(keeps.plugin), nameOf(keeps, door, p)),
          where: placeName(spot.location),
          target: { kind: "spot", spot },
        });
        continue;
      }
      const text =
        value === null
          ? p.bothKept(modOf(plugins[0] ?? ""), modOf(plugins[1] ?? ""))
          : p.kept(
              otherMod(of, value),
              modOf(value),
              namesOf(spotParts.filter((part) => part.hideable && part.object.plugin === value)) ||
                p.theirObjects,
            );
      entries.push({
        ...base,
        group: "collisions",
        section: sectionOf(of),
        text,
        where: placeName(spot.location),
        target: { kind: "spot", spot },
      });
      continue;
    }
    if (key.startsWith("object:")) {
      const object = objectOfKey(key, "object:");
      if (object === null) {
        entries.push({ ...base, group: "other", text: key });
        continue;
      }
      const known = parts.get(matchKey(object));
      const mod = modOf(object.plugin);
      const name = nameOf(object, known?.part, p);
      entries.push({
        ...base,
        group: "objects",
        text: value === null ? p.objectShown(mod, name) : p.objectHidden(mod, name),
        where: known === undefined ? null : placeName(known.spot.location),
        target: known === undefined ? null : { kind: "spot", spot: known.spot },
      });
      continue;
    }
    if (isMoveKey(key)) {
      const object = objectOfKey(key, "move:");
      const move = object === null ? undefined : moves.get(matchKey(object))?.move;
      if (object === null || move === undefined) {
        continue;
      }
      const known = parts.get(matchKey(object));
      const mod = modOf(object.plugin);
      const name = nameOf(object, known?.part, p);
      const centimetres = Math.round(Math.hypot(move.dx, move.dy, move.dz) * CM_PER_UNIT);
      entries.push({
        ...base,
        group: "moves",
        text: p.moved(mod, name, centimetres, move.rz),
        where: known === undefined ? move.cell : placeName(known.spot.location),
        target: known === undefined ? null : { kind: "spot", spot: known.spot },
      });
      continue;
    }
    if (key.startsWith(REPAIR)) {
      const at = repairPlace(value);
      const spot = at === null ? undefined : spotNear(report, at);
      entries.push({
        ...base,
        group: "paths",
        text: p.pathsRepaired(at?.place ?? key.slice(REPAIR.length)),
        where: spot === undefined ? null : placeName(spot.location),
        target: spot === undefined ? null : { kind: "spot", spot },
      });
      continue;
    }
    if (key.startsWith(DUEL)) {
      const id = key.slice(DUEL.length);
      const duel = duels.get(id);
      const side = Number(value);
      const winner = duel?.duel.sides[side === 1 ? 1 : 0]?.name;
      const loser = duel?.duel.sides[side === 1 ? 0 : 1]?.name;
      entries.push({
        ...base,
        group: "files",
        text:
          winner === undefined || loser === undefined ? p.unknownDuel : p.fileChoice(winner, loser),
        target: duel === undefined ? null : { kind: "duel", id },
      });
      continue;
    }
    // A collision the last scan did not find, for example of a mod that is
    // switched off: said by the mods its key names.
    const named = [...pluginsOf({ [key]: null }).values()];
    const [first, second] = named;
    if (first !== undefined && second !== undefined) {
      const mods = [modOf(first), modOf(second)];
      const hidden = named.find((plugin) => plugin.toLowerCase() === value?.toLowerCase());
      const kept = named.find((plugin) => plugin !== hidden);
      entries.push({
        ...base,
        group: "collisions",
        section: [...mods].sort((a, b) => a.localeCompare(b)).join(" ⟷ "),
        text: `${
          hidden === undefined || kept === undefined
            ? p.bothKept(mods[0] ?? first, mods[1] ?? second)
            : p.kept(modOf(kept), modOf(hidden), p.theirObjects)
        } — ${p.notScanned}`,
      });
      continue;
    }
    entries.push({ ...base, group: "other", text: key });
  }
  return entries.sort(
    (a, b) =>
      GROUPS.indexOf(a.group) - GROUPS.indexOf(b.group) ||
      (a.section ?? "").localeCompare(b.section ?? "") ||
      (a.where ?? "").localeCompare(b.where ?? "") ||
      a.text.localeCompare(b.text),
  );
}

/** One mod a patch is about, with how many of its decisions name it. */
export interface TouchedMod {
  mod: string;
  plugins: string[];
  decisions: number;
  /** Whether the game has its plugins: loaded, switched off, or not there at all. */
  state: "loaded" | "switched-off" | "missing";
}

/** How much of a mod the game has, from nothing to all. */
const RANK = { missing: 0, "switched-off": 1, loaded: 2 } as const;

/** The mods a patch's decisions are about, the missing and switched-off ones first. */
export function modsOf(
  decisions: PatchChoices,
  report: RenderedReport,
  modOf: (plugin: string) => string,
): TouchedMod[] {
  const loaded = new Set((report.loadOrder ?? []).map((plugin) => plugin.name.toLowerCase()));
  const off = new Set(
    (report.switchedOffPlugins ?? [])
      .filter((plugin) => !plugin.loaded)
      .map((plugin) => plugin.plugin.toLowerCase()),
  );
  const plugins = pluginsOf(decisions);
  const lower = [...plugins.keys()];
  const counts = new Map<string, number>();
  for (const key of Object.keys(decisions)) {
    if (key.startsWith("pin:")) {
      continue;
    }
    for (const plugin of new Set(pluginsInKey(key, lower))) {
      counts.set(plugin, (counts.get(plugin) ?? 0) + 1);
    }
  }
  const byMod = new Map<string, TouchedMod>();
  for (const [plugin, name] of plugins) {
    const state = off.has(plugin) ? "switched-off" : loaded.has(plugin) ? "loaded" : "missing";
    const mod = modOf(name);
    const known = byMod.get(mod) ?? { mod, plugins: [], decisions: 0, state };
    known.plugins.push(name);
    known.decisions += counts.get(plugin) ?? 0;
    // A mod counts as there when any of its plugins is.
    known.state = RANK[state] > RANK[known.state] ? state : known.state;
    byMod.set(mod, known);
  }
  return [...byMod.values()].sort(
    (a, b) => RANK[a.state] - RANK[b.state] || a.mod.localeCompare(b.mod),
  );
}

/** A function that names the mod of a plugin, from the report. */
export function modNamer(report: RenderedReport): (plugin: string) => string {
  const names = new Map<string, string>();
  for (const plugin of report.loadOrder ?? []) {
    if (plugin.provider?.name) {
      names.set(plugin.name.toLowerCase(), plugin.provider.name);
    }
  }
  for (const plugin of report.switchedOffPlugins ?? []) {
    names.set(plugin.plugin.toLowerCase(), plugin.mod.name);
  }
  return (plugin) => names.get(plugin.toLowerCase()) ?? plugin;
}
