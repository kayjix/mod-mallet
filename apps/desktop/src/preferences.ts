// The user's preferences between mods, learned from their decisions
// (ADR 0086).
//
// Someone who keeps picking the same mod, Static Mesh Improvement Mod for
// meshes or one city overhaul over another, has a preference without
// having said so. This module reads it from the decisions made so far:
// per mod and topic (meshes, textures, faces, other files, placed
// objects) how often it was kept against others, and per pair of mods how
// often one was kept over the other. A preference is clear from
// {@link MIN_DECISIONS} decisions on with {@link MIN_SHARE} in its favour.
//
// The app must not learn its own advice. A decision that agrees with a
// suggestion (the size of a side, ADR 0050; the reason Mod Mallet gives
// for a duel; the user's ranking) weighs {@link SUGGESTED_WEIGHT}; a
// decision made by applying a preference weighs nothing. A passage decided
// as a whole (ADR 0072) is one decision, however many spots it has.
//
// Pure and local: nothing leaves the window.

import { spotsOfPassage } from "./decisions.ts";
import type { PatchPreferences, PreferenceRule } from "./generated/preferences";
import type { DuelSide, MapPassage, MapSpot, ModDuel, Topic } from "./generated/rendered-report";
import { keptBy, type PatchChoices, spotKey } from "./patch.ts";

/** What a preference is about. */
export type PreferenceTopic = "meshes" | "textures" | "faces" | "files" | "places";

/** The topics, in the order they are shown. */
export const TOPICS: readonly PreferenceTopic[] = [
  "meshes",
  "textures",
  "faces",
  "files",
  "places",
];

/** From this many decisions on (weighted, see {@link SUGGESTED_WEIGHT}) a preference can be clear. */
export const MIN_DECISIONS = 5;

/** The share of the decisions a mod must have won for a clear preference. */
export const MIN_SHARE = 0.8;

/** What a decision that agrees with a suggestion weighs, against one the user made alone. */
export const SUGGESTED_WEIGHT = 0.25;

/**
 * Against how many other mods a favourite of a topic must have won: won
 * against one mod only, it is a preference of that pair.
 */
export const MIN_OPPONENTS = 2;

/** The prefix of a duel's pick among the choices (ADR 0081). */
const DUEL = "duel:";

/** A mod's name as preferences compare it. */
export const modKey = (name: string) => name.trim().toLowerCase();

/** The topic of preference a topic of a duel belongs to. */
export function topicOf(topic: Topic): PreferenceTopic {
  switch (topic) {
    case "meshes":
    case "textures":
    case "faces":
      return topic;
    case "objects":
      return "places";
    default:
      return "files";
  }
}

/** The topics of a duel as preferences see them, each once. */
export function duelTopics(duel: ModDuel): PreferenceTopic[] {
  return [...new Set(duel.topics.map((entry) => topicOf(entry.topic)))];
}

/** One decision, as evidence of a preference. */
export interface Vote {
  /** The key of the decision among the choices. */
  key: string;
  /** The mod kept, by its name. */
  winner: string;
  /** The mod given up, by its name. */
  loser: string;
  topics: PreferenceTopic[];
  /** Which part of one decision it is: a spot of a passage of n spots is 1/n. */
  share: number;
  /** What it weighs: its share, less where it agrees with a suggestion, none where a preference made it. */
  weight: number;
}

/** What the decisions are read against. */
export interface PreferenceWorld {
  /** The duels between mods (ADR 0048). */
  duels: readonly ModDuel[];
  /** The collisions' spots, by finding. */
  spots: Map<string, MapSpot[]>;
  /** The passages the scan found (ADR 0072). */
  passages?: readonly MapPassage[];
  /** The mod a plugin belongs to, by name; the plugin itself if none is known. */
  modOf: (plugin: string) => string;
  /** The name of a duel's side, as {@link PreferenceWorld.modOf} names mods; its own name if not given. */
  sideName?: (side: DuelSide) => string;
  /** The side the user's ranking makes win in a duel, if it does. */
  ranked?: (duel: ModDuel) => 0 | 1 | null;
}

/** What of a report names the mods. */
export interface ModNames {
  mods: readonly { mod: { id: string; name: string } }[];
  loadOrder?: readonly { name: string; provider?: { id: string; name: string } | null }[];
  switchedOffPlugins?: readonly { plugin: string; mod: { id: string; name: string } }[];
}

/**
 * How a report names mods, alike for plugins and the sides of duels: by
 * the mod manager's identity, with the name the mod manager shows. A
 * duel's side may carry the identity as its name; the spots know only
 * plugins.
 */
export function namesOf(report: ModNames): Pick<PreferenceWorld, "modOf" | "sideName"> {
  const byId = new Map(report.mods.map((info) => [info.mod.id, info.mod.name]));
  const byPlugin = new Map<string, string>();
  for (const plugin of report.loadOrder ?? []) {
    if (plugin.provider) {
      byPlugin.set(plugin.name.toLowerCase(), byId.get(plugin.provider.id) ?? plugin.provider.name);
    }
  }
  for (const plugin of report.switchedOffPlugins ?? []) {
    byPlugin.set(plugin.plugin.toLowerCase(), byId.get(plugin.mod.id) ?? plugin.mod.name);
  }
  return {
    modOf: (plugin) => byPlugin.get(plugin.toLowerCase()) ?? plugin,
    sideName: (side) => (side.mod ? byId.get(side.mod.id) : undefined) ?? side.name,
  };
}

/** The two mods of a duel, by name. */
function duelMods(duel: ModDuel, world: PreferenceWorld): [string, string] {
  const name = world.sideName ?? ((side: DuelSide) => side.name);
  return [name(duel.sides[0]), name(duel.sides[1])];
}

/** The plugins of a spot's parts, each once in the spelling of the first. */
function pluginsOf(spot: MapSpot): string[] {
  const found = new Map<string, string>();
  for (const part of spot.parts ?? []) {
    const lower = part.object.plugin.toLowerCase();
    if (!found.has(lower)) {
      found.set(lower, part.object.plugin);
    }
  }
  return [...found.values()];
}

/** Whether the size of a spot's sides suggests hiding `plugin` there (ADR 0050). */
function suggestsHiding(spots: readonly MapSpot[], plugin: string): boolean {
  const lower = plugin.toLowerCase();
  let votes = 0;
  let against = 0;
  for (const part of spots.flatMap((spot) => spot.parts ?? [])) {
    if (part.suggested) {
      if (part.object.plugin.toLowerCase() === lower) {
        votes += 1;
      } else {
        against += 1;
      }
    }
  }
  return votes > against;
}

/** The side of a passage its size suggests keeping, by the rule of ADR 0050. */
function passageKeeps(passage: MapPassage): number | null {
  const [a, b] = passage.parts;
  const [va, vb] = passage.volumes;
  if (a.length === 0 || b.length === 0) {
    return null;
  }
  if (va >= 20 * vb || (a.length >= 20 * b.length && va >= 5 * vb)) {
    return 0;
  }
  if (vb >= 20 * va || (b.length >= 20 * a.length && vb >= 5 * va)) {
    return 1;
  }
  return null;
}

/**
 * Every decision in `choices` that kept one mod against another, as a
 * vote: the picks of duels, collisions decided as a whole, spots decided
 * on their own (a passage's spots together counting as one), and stacked
 * doors merged into one. Objects hidden on their own say nothing about a
 * preference and are left out, as are decisions to keep both. Decisions
 * in `fromPreference` were made by applying a preference and weigh
 * nothing.
 */
export function votesOf(
  choices: PatchChoices,
  world: PreferenceWorld,
  fromPreference: ReadonlySet<string> = new Set(),
): Vote[] {
  const votes: Vote[] = [];
  const duels = new Map(world.duels.map((duel) => [duel.id, duel]));
  const weightOf = (key: string, share: number, suggested: boolean) =>
    fromPreference.has(key) ? 0 : suggested ? share * SUGGESTED_WEIGHT : share;

  // The spots by key, and how many spots each spot's passage has.
  const byKey = new Map<string, MapSpot>();
  for (const list of world.spots.values()) {
    for (const spot of list) {
      const key = spotKey(spot);
      if (key !== null) {
        byKey.set(key, spot);
      }
    }
  }
  const allSpots = [...byKey.values()];
  const passageOf = new Map<string, { size: number; keeps: string | null }>();
  for (const passage of world.passages ?? []) {
    const inside = spotsOfPassage(passage, allSpots);
    const keeps = passageKeeps(passage);
    for (const spot of inside) {
      const key = spotKey(spot);
      if (key !== null) {
        passageOf.set(key, {
          size: inside.length,
          keeps: keeps === null ? null : (passage.plugins[keeps]?.toLowerCase() ?? null),
        });
      }
    }
  }

  for (const [key, value] of Object.entries(choices)) {
    if (value === null) {
      continue;
    }
    if (key.startsWith(DUEL)) {
      const duel = duels.get(key.slice(DUEL.length));
      const side = Number(value);
      if (duel === undefined || (side !== 0 && side !== 1)) {
        continue;
      }
      const other = side === 0 ? 1 : 0;
      const suggested = duel.suggestion?.side === side || world.ranked?.(duel) === side;
      votes.push({
        key,
        winner: duelMods(duel, world)[side],
        loser: duelMods(duel, world)[other],
        topics: duelTopics(duel),
        share: 1,
        weight: weightOf(key, 1, suggested),
      });
      continue;
    }
    const found = world.spots.get(key);
    if (found !== undefined) {
      // A collision decided as a whole: one decision.
      const plugins = [...new Set(found.flatMap(pluginsOf))];
      const winner = plugins.find((plugin) => plugin.toLowerCase() !== value.toLowerCase());
      if (winner !== undefined && plugins.length === 2) {
        votes.push({
          key,
          winner: world.modOf(winner),
          loser: world.modOf(value),
          topics: ["places"],
          share: 1,
          weight: weightOf(key, 1, suggestsHiding(found, value)),
        });
      }
      continue;
    }
    const spot = byKey.get(key);
    if (spot === undefined) {
      continue;
    }
    const plugins = pluginsOf(spot);
    const kept = keptBy(value);
    if (kept !== null) {
      // Stacked doors merged into one: the kept door's mod against each
      // other mod whose doors go.
      const winner = world.modOf(kept.plugin);
      const losers = [
        ...new Set(
          plugins
            .filter((plugin) => plugin.toLowerCase() !== kept.plugin.toLowerCase())
            .map(world.modOf),
        ),
      ].filter((loser) => modKey(loser) !== modKey(winner));
      for (const loser of losers) {
        votes.push({
          key,
          winner,
          loser,
          topics: ["places"],
          share: 1 / losers.length,
          weight: weightOf(key, 1 / losers.length, false),
        });
      }
      continue;
    }
    const winner = plugins.find((plugin) => plugin.toLowerCase() !== value.toLowerCase());
    if (winner === undefined || plugins.length !== 2) {
      continue;
    }
    const passage = passageOf.get(key);
    const share = 1 / (passage?.size ?? 1);
    const suggested =
      passage === undefined
        ? suggestsHiding([spot], value)
        : passage.keeps === winner.toLowerCase();
    votes.push({
      key,
      winner: world.modOf(winner),
      loser: world.modOf(value),
      topics: ["places"],
      share,
      weight: weightOf(key, share, suggested),
    });
  }
  // A mod against itself (two plugins of one mod) says nothing.
  return votes.filter((vote) => modKey(vote.winner) !== modKey(vote.loser));
}

/** A preference read from the decisions. */
export interface Learned {
  /** Its identity, which stays while the mods keep their names. */
  id: string;
  /** A mod's favourite topic, or one mod over another in everything. */
  kind: "favourite" | "pair";
  /** The mod preferred, by its name. */
  mod: string;
  /** The mod it is preferred over, for a pair. */
  over: string | null;
  /** The topic, for a favourite. */
  topic: PreferenceTopic | null;
  /** How many of the user's own decisions about it kept it, rounded. */
  chosen: number;
  /** How many of the user's own decisions were about it, rounded. */
  of: number;
  /** How many more decisions kept it by following a suggestion, rounded. */
  followed: number;
  /** The weighted share of the decisions in its favour. */
  share: number;
  /** The weighted number of decisions. */
  weight: number;
}

/** The identity of a favourite. */
export const favouriteId = (mod: string, topic: PreferenceTopic) =>
  `favourite|${modKey(mod)}|${topic}`;

/** The identity of one mod preferred over another. */
export const pairId = (mod: string, over: string) => `pair|${modKey(mod)}|${modKey(over)}`;

/** What the votes say about one mod, in a topic or against another mod. */
interface Tally {
  name: string;
  /** Weighted decisions won and lost. */
  won: number;
  lost: number;
  /** The user's own decisions won, and all their own about it. */
  ownWon: number;
  ownOf: number;
  /** Decisions won by following a suggestion. */
  followed: number;
  /** The mods it won against, by {@link modKey}. */
  beaten: Set<string>;
}

const tally = (name: string): Tally => ({
  name,
  won: 0,
  lost: 0,
  ownWon: 0,
  ownOf: 0,
  followed: 0,
  beaten: new Set(),
});

/** Counts one vote for or against the mod of `entry`. */
function count(entry: Tally, vote: Vote, won: boolean) {
  const own = vote.weight > 0 && vote.weight >= vote.share - 1e-9;
  if (own) {
    entry.ownOf += vote.share;
  }
  if (won) {
    entry.won += vote.weight;
    if (own) {
      entry.ownWon += vote.share;
    } else if (vote.weight > 0) {
      entry.followed += vote.share;
    }
    if (vote.weight > 0) {
      entry.beaten.add(modKey(vote.loser));
    }
  } else {
    entry.lost += vote.weight;
  }
}

/** A tally as a preference learned. */
function asLearned(
  id: string,
  entry: Tally,
  kind: Learned["kind"],
  over: string | null,
  topic: PreferenceTopic | null,
): Learned {
  return {
    id,
    kind,
    mod: entry.name,
    over,
    topic,
    chosen: Math.round(entry.ownWon),
    of: Math.round(entry.ownOf),
    followed: Math.round(entry.followed),
    share: entry.won / (entry.won + entry.lost),
    weight: entry.won + entry.lost,
  };
}

/**
 * The clear preferences in `votes`: per mod and topic where it won against
 * at least {@link MIN_OPPONENTS} mods, and per pair of mods. The strongest
 * first.
 */
export function learn(votes: readonly Vote[]): Learned[] {
  const favourites = new Map<string, Tally & { topic: PreferenceTopic }>();
  const pairs = new Map<string, Tally & { over: string }>();
  for (const vote of votes) {
    for (const [mod, over, won] of [
      [vote.winner, vote.loser, true],
      [vote.loser, vote.winner, false],
    ] as const) {
      for (const topic of vote.topics) {
        const id = favouriteId(mod, topic);
        const entry = favourites.get(id) ?? { ...tally(mod), topic };
        count(entry, vote, won);
        favourites.set(id, entry);
      }
      const id = pairId(mod, over);
      const entry = pairs.get(id) ?? { ...tally(mod), over };
      count(entry, vote, won);
      pairs.set(id, entry);
    }
  }
  const clear = (entry: Tally) => {
    const weight = entry.won + entry.lost;
    return weight >= MIN_DECISIONS - 1e-9 && entry.won / weight >= MIN_SHARE - 1e-9;
  };
  const found: Learned[] = [];
  for (const [id, entry] of favourites) {
    if (clear(entry) && entry.beaten.size >= MIN_OPPONENTS) {
      found.push(asLearned(id, entry, "favourite", null, entry.topic));
    }
  }
  for (const [id, entry] of pairs) {
    if (clear(entry)) {
      found.push(asLearned(id, entry, "pair", entry.over, null));
    }
  }
  return found.sort((a, b) => b.weight * b.share - a.weight * a.share || a.id.localeCompare(b.id));
}

/** A preference that counts: learned and not turned down, or said by the user. */
export interface Rule {
  id: string;
  mod: string;
  over: string | null;
  topic: PreferenceTopic | null;
  /** Whether the user said it, rather than it being learned. */
  explicit: boolean;
}

/** The identity of a preference the user said. */
export const explicitId = (rule: Pick<PreferenceRule, "mod" | "over" | "topic">) =>
  `own|${modKey(rule.mod)}|${rule.over === null || rule.over === undefined ? "*" : modKey(rule.over)}|${rule.topic ?? "*"}`;

/** Whether a topic is one of preferences. */
export const isTopic = (topic: string | null | undefined): topic is PreferenceTopic =>
  TOPICS.includes(topic as PreferenceTopic);

/** What is kept about preferences, with nothing said yet. */
export const NO_PREFERENCES: PatchPreferences = { rules: [], answers: {}, applied: {} };

/**
 * The preferences that count: those the user said, and, unless learning
 * is switched off, those learned and not turned down.
 */
export function rulesOf(learned: readonly Learned[], kept: PatchPreferences): Rule[] {
  const rules: Rule[] = (kept.rules ?? []).map((rule) => ({
    id: explicitId(rule),
    mod: rule.mod,
    over: rule.over ?? null,
    topic: isTopic(rule.topic) ? rule.topic : null,
    explicit: true,
  }));
  if (kept.learningOff === true) {
    return rules;
  }
  for (const entry of learned) {
    if (kept.answers?.[entry.id]?.answer !== "never") {
      rules.push({
        id: entry.id,
        mod: entry.mod,
        over: entry.over,
        topic: entry.topic,
        explicit: false,
      });
    }
  }
  return rules;
}

/** Something two mods are in conflict about that a preference can decide. */
export type Conflict =
  | {
      type: "duel";
      /** The duel's identity, and its key among the choices without the prefix. */
      id: string;
      duel: ModDuel;
      /** The two mods, by name. */
      mods: [string, string];
      topics: PreferenceTopic[];
    }
  | {
      type: "spot";
      /** The spot's key among the choices. */
      id: string;
      spot: MapSpot;
      /** The two plugins of its parts. */
      plugins: [string, string];
      /** Their mods, by name. */
      mods: [string, string];
      topics: PreferenceTopic[];
    };

/**
 * Every conflict a preference can decide: the duels about files (those
 * about placed objects are decided at their spots), and the spots of two
 * plugins of two mods. Stacked doors are merged by the door panel, not by
 * a side, and are left out.
 */
export function conflictsOf(world: PreferenceWorld): Conflict[] {
  const found: Conflict[] = [];
  for (const duel of world.duels) {
    const topics = duelTopics(duel).filter((topic) => topic !== "places");
    if (topics.length > 0) {
      found.push({
        type: "duel",
        id: duel.id,
        duel,
        mods: duelMods(duel, world),
        topics,
      });
    }
  }
  for (const list of world.spots.values()) {
    for (const spot of list) {
      const key = spotKey(spot);
      const plugins = pluginsOf(spot);
      const [a, b] = plugins;
      if (key === null || spot.doors != null || a === undefined || b === undefined) {
        continue;
      }
      const mods: [string, string] = [world.modOf(a), world.modOf(b)];
      if (plugins.length === 2 && modKey(mods[0]) !== modKey(mods[1])) {
        found.push({ type: "spot", id: key, spot, plugins: [a, b], mods, topics: ["places"] });
      }
    }
  }
  return found;
}

/** Whether a preference speaks about a conflict, and for which side. */
function sideOf(rule: Rule, conflict: Conflict): 0 | 1 | null {
  const [a, b] = conflict.mods.map(modKey);
  const mod = modKey(rule.mod);
  const side = mod === a ? 0 : mod === b ? 1 : null;
  if (side === null) {
    return null;
  }
  const other = side === 0 ? b : a;
  if (rule.over !== null && modKey(rule.over) !== other) {
    return null;
  }
  if (rule.topic !== null && !conflict.topics.includes(rule.topic)) {
    return null;
  }
  return side;
}

/** What preferences say about a conflict: the side to keep and why. */
export interface Preferred {
  side: 0 | 1;
  rule: Rule;
}

/**
 * The side the preferences keep in a conflict, if they agree: a
 * preference about the pair goes before a favourite, one the user said
 * before one learned. Where equally strong preferences pull both ways,
 * none.
 */
export function preferredSide(rules: readonly Rule[], conflict: Conflict): Preferred | null {
  const rank = (rule: Rule) => (rule.over !== null ? 2 : 0) + (rule.explicit ? 1 : 0);
  let best: Preferred | null = null;
  let tied = false;
  for (const rule of rules) {
    const side = sideOf(rule, conflict);
    if (side === null) {
      continue;
    }
    if (best === null || rank(rule) > rank(best.rule)) {
      best = { side, rule };
      tied = false;
    } else if (rank(rule) === rank(best.rule) && side !== best.side) {
      tied = true;
    }
  }
  return tied ? null : best;
}

/** One conflict a preference would decide. */
export interface PlanItem {
  conflict: Conflict;
  /** The side kept. */
  side: 0 | 1;
  /** The mod kept and the one given up, by name. */
  winner: string;
  loser: string;
  /** For a spot, the plugin whose parts go. */
  plugin: string | null;
}

/** What a page knows about what is decided already. */
export interface OpenCheck {
  /** Whether a duel is still open. */
  duelOpen: (id: string) => boolean;
  /** Whether a spot is still open. */
  spotOpen: (spot: MapSpot) => boolean;
}

/**
 * The open conflicts a preference would decide, where the other
 * preferences do not pull the other way: `rule` must be the one that
 * decides each.
 */
export function planOf(
  rule: Rule,
  rules: readonly Rule[],
  conflicts: readonly Conflict[],
  open: OpenCheck,
): PlanItem[] {
  const items: PlanItem[] = [];
  for (const conflict of conflicts) {
    const isOpen =
      conflict.type === "duel" ? open.duelOpen(conflict.id) : open.spotOpen(conflict.spot);
    if (!isOpen || sideOf(rule, conflict) === null) {
      continue;
    }
    const preferred = preferredSide(rules.includes(rule) ? rules : [...rules, rule], conflict);
    if (preferred === null || preferred.rule.id !== rule.id) {
      continue;
    }
    const other = preferred.side === 0 ? 1 : 0;
    items.push({
      conflict,
      side: preferred.side,
      winner: conflict.mods[preferred.side],
      loser: conflict.mods[other],
      plugin: conflict.type === "spot" ? conflict.plugins[other] : null,
    });
  }
  return items;
}

/** A preference worth offering, with what it would decide. */
export interface Offer {
  learned: Learned;
  items: PlanItem[];
}

/** How many more decisions about a preference bring it back after "Not now". */
export const LATER_AFTER = 5;

/**
 * The learned preference to offer now, if any: the strongest one with open
 * conflicts it would decide that the user has not answered, or answered
 * "Not now" to before {@link LATER_AFTER} more decisions about it.
 */
export function offerOf(
  learned: readonly Learned[],
  kept: PatchPreferences,
  rules: readonly Rule[],
  conflicts: readonly Conflict[],
  open: OpenCheck,
): Offer | null {
  if (kept.learningOff === true) {
    return null;
  }
  for (const entry of learned) {
    const answer = kept.answers?.[entry.id];
    if (
      answer !== undefined &&
      (answer.answer !== "later" || entry.of < (answer.at ?? 0) + LATER_AFTER)
    ) {
      continue;
    }
    const rule = rules.find((candidate) => candidate.id === entry.id);
    if (rule === undefined) {
      continue;
    }
    const items = planOf(rule, rules, conflicts, open);
    if (items.length > 0) {
      return { learned: entry, items };
    }
  }
  return null;
}

/**
 * The choices with the spots of a plan decided: the plugin given up goes
 * at each spot. Only the parts that may go are hidden, as by every spot
 * decision; what the game needs stays. Duels are not decided here: their
 * picks ask the mod manager, which the window does.
 */
export function decideSpots(choices: PatchChoices, items: readonly PlanItem[]): PatchChoices {
  const next = { ...choices };
  for (const item of items) {
    if (item.conflict.type === "spot" && item.plugin !== null) {
      next[item.conflict.id] = item.plugin;
    }
  }
  return next;
}

/**
 * The choices with the decisions of `keys` put back as they were in
 * `before`, for one undo of a whole batch; everything else as it is now.
 */
export function restoreKeys(
  now: PatchChoices,
  before: PatchChoices,
  keys: readonly string[],
): PatchChoices {
  const next = { ...now };
  for (const key of keys) {
    if (key in before) {
      next[key] = before[key] ?? null;
    } else {
      delete next[key];
    }
  }
  return next;
}

/** The keys whose decisions differ between two sets of choices. */
export function changedKeys(before: PatchChoices, after: PatchChoices): string[] {
  const keys = new Set([...Object.keys(before), ...Object.keys(after)]);
  return [...keys].filter(
    (key) => key in before !== key in after || (before[key] ?? null) !== (after[key] ?? null),
  );
}

/**
 * The choices of the user's own patches, all of them, switched off or
 * not, for learning: a preference does not depend on which patch counts
 * now. Where two patches decide the same, the later one wins, and the
 * patch being edited last.
 */
export function historyOf(
  patches: readonly {
    id: string;
    decisions: PatchChoices;
    foreign?: boolean;
    unticked?: string[];
  }[],
  edited: string | null,
): PatchChoices {
  const own = patches.filter((patch) => patch.foreign !== true);
  const ordered = [
    ...own.filter((patch) => patch.id !== edited),
    ...own.filter((patch) => patch.id === edited),
  ];
  const merged: PatchChoices = {};
  for (const patch of ordered) {
    const unticked = new Set(patch.unticked ?? []);
    for (const [key, value] of Object.entries(patch.decisions)) {
      if (!unticked.has(key)) {
        merged[key] = value;
      }
    }
  }
  return merged;
}
