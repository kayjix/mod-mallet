// The report of what was decided (ADR 0050), per pair of mods: what of
// which mod is hidden, place by place with names, what was kept and with
// which exceptions, what is still open, and what stays because the game
// needs it. Built in the window from the patch's choices; the scan only
// read what is there.

import type { AreaObject } from "./generated/area-objects";
import type { MapSpot, RenderedReport } from "./generated/rendered-report";
import { reportSpots } from "./health";
import { isGameObject } from "./object-actions";
import { hiddenKeysOf, matchKey, type PatchChoices } from "./patch";
import { type Cluster, clustersOf, type Decided, decidedOf } from "./review-clusters";

/** What the window remembers of an object it once showed: name, plugin, worldspace, place. */
interface KnownObject {
  n: string;
  p: string;
  w: string;
  x: number;
  y: number;
}

/** How many objects the window remembers per game, the latest kept. */
const KNOWN_LIMIT = 20000;

const knownKey = (game: string) => `mallet.objects.${game}`;

function readKnown(game: string): Record<string, KnownObject> {
  try {
    const text = localStorage.getItem(knownKey(game));
    return text === null ? {} : (JSON.parse(text) as Record<string, KnownObject>);
  } catch {
    return {};
  }
}

/**
 * Remembers the names and places of objects the review showed, so that the
 * report can name the parts hidden on their own: the choices keep only
 * their plugin and number. A convenience: without it the report counts.
 */
export function rememberObjects(game: string, world: string, objects: AreaObject[]) {
  if (game === "" || objects.length === 0) {
    return;
  }
  const known = readKnown(game);
  for (const object of objects) {
    known[matchKey(object.id)] = {
      n: object.name ?? object.model.split(/[\\/]/).pop() ?? object.model,
      p: object.id.plugin,
      w: world,
      x: Math.round(object.position[0]),
      y: Math.round(object.position[1]),
    };
  }
  const keys = Object.keys(known);
  if (keys.length > KNOWN_LIMIT) {
    for (const key of keys.slice(0, keys.length - KNOWN_LIMIT)) {
      delete known[key];
    }
  }
  try {
    localStorage.setItem(knownKey(game), JSON.stringify(known));
  } catch {
    // Only a convenience.
  }
}

/** One place of a pair in the report. */
export interface PlaceReport {
  console: string;
  spots: number;
  decided: Decided;
  /** What is hidden there, by plugin: the objects' names. */
  hidden: Map<string, string[]>;
  /** Objects decided otherwise than the place as a whole. */
  exceptions: number;
  /** Objects that stay because the game needs them or they are the game's own. */
  stays: string[];
}

/** The places of one pair of mods. */
export interface PairReport {
  plugins: [string, string];
  places: PlaceReport[];
  /** How many objects of each plugin are hidden at these places. */
  hiddenCount: Map<string, number>;
  open: number;
  decided: number;
}

/** A mod of which, after the decisions, little is left in the game. */
export interface Leftover {
  mod: string;
  total: number;
  left: number;
}

export interface PatchReport {
  pairs: PairReport[];
  /** Objects hidden on their own outside the places, by plugin, with names. */
  elsewhere: Map<string, string[]>;
  leftovers: Leftover[];
}

/** Below this share of its objects left, and no more than {@link FEW_LEFT}, a mod may go. */
const SMALL_SHARE = 0.15;
const FEW_LEFT = 30;

/** A name for comparing a mod's name with a plugin's file name. */
const plain = (name: string) =>
  name
    .toLowerCase()
    .replace(/\.(esp|esm|esl)$/, "")
    .replace(/[^a-z0-9]/g, "");

/** The plugin of a key from {@link matchKey}. */
const pluginOfKey = (key: string) => key.slice(0, key.lastIndexOf(":"));

/** The spots of the report by finding, as the patch's choices are read with. */
function spotsByFinding(report: RenderedReport): Map<string, MapSpot[]> {
  const found = new Map<string, MapSpot[]>();
  for (const spot of report.map.spots) {
    found.set(spot.finding, [...(found.get(spot.finding) ?? []), spot]);
  }
  return found;
}

/** The world or cell of a cluster's spots. */
function worldOf(cluster: Cluster): string {
  const location = cluster.spots[0]?.item.spot.location;
  return location === undefined
    ? ""
    : location.type === "interior"
      ? location.cell
      : location.world;
}

/** What was decided, per pair of mods, for the report (ADR 0050). */
export function patchReport(
  report: RenderedReport,
  ignored: Set<string>,
  choices: PatchChoices,
  game: string,
): PatchReport {
  const hidden = hiddenKeysOf(spotsByFinding(report), choices);
  const known = readKnown(game);
  const names = new Map<string, string>();
  for (const spot of report.map.spots) {
    for (const part of spot.parts ?? []) {
      names.set(matchKey(part.object), part.name);
    }
  }
  const nameOf = (key: string) =>
    names.get(key) ??
    known[key]?.n ??
    `${pluginOfKey(key)} ${Number(key.slice(key.lastIndexOf(":") + 1))
      .toString(16)
      .padStart(6, "0")
      .toUpperCase()}`;
  const clusters = clustersOf(reportSpots(report, ignored, choices));
  const claimed = new Set<string>();
  const pairs = new Map<string, PairReport>();
  for (const cluster of clusters) {
    const plugins = cluster.plugins.map((plugin) => plugin.toLowerCase());
    const world = worldOf(cluster);
    const parts = cluster.spots.flatMap(({ item }) => item.spot.parts ?? []);
    const own = new Set(parts.map((part) => matchKey(part.object)));
    // The objects hidden here: the spots' own, and those hidden on their
    // own that the window saw standing here.
    const reach = cluster.radius + 2048;
    const here = [...hidden].filter((key) => {
      if (claimed.has(key)) {
        return false;
      }
      if (own.has(key)) {
        return true;
      }
      const seen = known[key];
      return (
        seen !== undefined &&
        seen.w === world &&
        plugins.includes(seen.p.toLowerCase()) &&
        Math.hypot(seen.x - cluster.centre[0], seen.y - cluster.centre[1]) <= reach
      );
    });
    for (const key of here) {
      claimed.add(key);
    }
    const byPlugin = new Map<string, string[]>();
    for (const key of here) {
      const plugin =
        cluster.plugins.find((candidate) => candidate.toLowerCase() === pluginOfKey(key)) ??
        pluginOfKey(key);
      byPlugin.set(plugin, [...(byPlugin.get(plugin) ?? []), nameOf(key)]);
    }
    const decided = decidedOf(cluster.spots, choices);
    // Exceptions: of the side hidden, parts that stay; of the side kept,
    // parts hidden; where both stay, anything hidden.
    let exceptions = 0;
    if (decided.type === "hidden") {
      const loser = decided.plugin.toLowerCase();
      exceptions =
        parts.filter(
          (part) =>
            part.hideable &&
            part.object.plugin.toLowerCase() === loser &&
            !hidden.has(matchKey(part.object)),
        ).length + here.filter((key) => pluginOfKey(key) !== loser).length;
    } else if (decided.type === "kept") {
      exceptions = here.length;
    }
    const stays = [
      ...new Set(
        parts
          .filter((part) => !part.hideable || isGameObject({ id: part.object }))
          .filter((part) => !hidden.has(matchKey(part.object)))
          .map((part) => part.name),
      ),
    ];
    const pairKey = [...plugins].sort().join("+");
    const pair = pairs.get(pairKey) ?? {
      plugins: cluster.plugins,
      places: [],
      hiddenCount: new Map<string, number>(),
      open: 0,
      decided: 0,
    };
    pair.places.push({
      console: cluster.spots[0]?.item.spot.console ?? "",
      spots: cluster.spots.length,
      decided,
      hidden: byPlugin,
      exceptions,
      stays,
    });
    for (const [plugin, list] of byPlugin) {
      pair.hiddenCount.set(plugin, (pair.hiddenCount.get(plugin) ?? 0) + list.length);
    }
    if (decided.type === "open" || (decided.type === "mixed" && decided.open > 0)) {
      pair.open += 1;
    } else {
      pair.decided += 1;
    }
    pairs.set(pairKey, pair);
  }
  const elsewhere = new Map<string, string[]>();
  for (const key of hidden) {
    if (!claimed.has(key)) {
      const plugin = known[key]?.p ?? pluginOfKey(key);
      elsewhere.set(plugin, [...(elsewhere.get(plugin) ?? []), nameOf(key)]);
    }
  }
  return {
    pairs: [...pairs.values()].sort(
      (a, b) => b.places.length - a.places.length || a.plugins[0].localeCompare(b.plugins[0]),
    ),
    elsewhere,
    leftovers: leftoversOf(report, hidden),
  };
}

/**
 * The mods of which little is left once the patch hides what was decided:
 * from the suggestions about whole mods (ADR 0043), which know how many
 * objects a mod places and whether it has rooms of its own. A mod with
 * rooms of its own is never named; nor one whose plugin cannot be told.
 */
function leftoversOf(report: RenderedReport, hidden: Set<string>): Leftover[] {
  const plugins = new Set(
    report.map.spots.flatMap((spot) => (spot.parts ?? []).map((part) => part.object.plugin)),
  );
  const found: Leftover[] = [];
  for (const { finding } of report.findings) {
    if (finding.kind !== "mods.collides-widely") {
      continue;
    }
    const mod = finding.args.mod;
    const total = Number(finding.args.total ?? 0);
    const interiors = Number(finding.args.interiors ?? 0);
    if (typeof mod !== "string" || total <= 0 || interiors > 0) {
      continue;
    }
    const name = plain(mod);
    const own = [...plugins].filter((plugin) => {
      const candidate = plain(plugin);
      return (
        candidate !== "" &&
        (candidate === name || name.includes(candidate) || candidate.includes(name))
      );
    });
    if (own.length === 0) {
      continue;
    }
    const lower = new Set(own.map((plugin) => plugin.toLowerCase()));
    const gone = [...hidden].filter((key) => lower.has(pluginOfKey(key))).length;
    const left = Math.max(0, total - gone);
    if (gone > 0 && left <= FEW_LEFT && left / total <= SMALL_SHARE) {
      found.push({ mod, total, left });
    }
  }
  return found;
}

/** Names with how many of each, the most first: "Plank ×3 · Pillar". */
export function counted(names: string[]): string {
  const counts = new Map<string, number>();
  for (const name of names) {
    counts.set(name, (counts.get(name) ?? 0) + 1);
  }
  return [...counts.entries()]
    .sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]))
    .map(([name, count]) => (count === 1 ? name : `${name} ×${count}`))
    .join(" · ");
}
