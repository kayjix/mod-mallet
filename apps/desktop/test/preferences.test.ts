// Tests that preferences are learned from the user's decisions, and only
// from theirs (ADR 0086), and what applying one decides.
//
// Made up: a mesh mod that wins every duel about meshes, a city mod kept
// over a road mod at every spot, and the suggestions the app gave.

import assert from "node:assert/strict";
import { test } from "node:test";
import type { PatchPreferences } from "../src/generated/preferences";
import type { MapPassage, MapSpot, ModDuel, Topic } from "../src/generated/rendered-report";
import { keepChoice, spotKey } from "../src/patch.ts";
import {
  changedKeys,
  conflictsOf,
  decideSpots,
  favouriteId,
  historyOf,
  LATER_AFTER,
  learn,
  type OpenCheck,
  offerOf,
  type PreferenceWorld,
  pairId,
  planOf,
  preferredSide,
  type Rule,
  restoreKeys,
  rulesOf,
  votesOf,
} from "../src/preferences.ts";

const MESH = "Mesh Mod";

let duelCount = 0;
function duel(
  a: string,
  b: string,
  topics: Topic[] = ["meshes"],
  suggestion: 0 | 1 | null = null,
): ModDuel {
  duelCount += 1;
  return {
    id: `duel-${duelCount}`,
    findings: [],
    resolutions: [],
    sides: [
      { name: a, plugins: [] },
      { name: b, plugins: [] },
    ],
    topics: topics.map((topic) => ({ topic, count: 3 })),
    suggestion: suggestion === null ? null : { side: suggestion, reason: "specialist" },
  };
}

const CITY = "City.esp";
const ROAD = "Road.esp";
const FINDING = "objects.overlapping:city+road";

function spot(id: number, suggestedHide: string | null = null, doors = false): MapSpot {
  const part = (plugin: string, localId: number) => ({
    object: { plugin, localId },
    name: `Part${localId}`,
    hideable: true,
    suggested: plugin === suggestedHide,
  });
  return {
    console: "",
    finding: FINDING,
    objects: "",
    location: { type: "exterior", world: "Tamriel", x: 0, y: 0 },
    parts: [part(CITY, id), part(ROAD, 1000 + id)],
    doors: doors ? { doors: [], plans: [] } : null,
  } as MapSpot;
}

const modOf = (plugin: string) =>
  plugin === CITY ? "City Overhaul" : plugin === ROAD ? "Road Mod" : plugin;

function world(duels: ModDuel[], spots: MapSpot[] = [], passages: MapPassage[] = []) {
  const byFinding = new Map<string, MapSpot[]>();
  for (const one of spots) {
    byFinding.set(one.finding, [...(byFinding.get(one.finding) ?? []), one]);
  }
  return { duels, spots: byFinding, passages, modOf } satisfies PreferenceWorld;
}

const pick = (choices: Record<string, string | null>, d: ModDuel, side: 0 | 1) => {
  choices[`duel:${d.id}`] = String(side);
};

test("a mod kept in five duels about meshes against several mods is a favourite", () => {
  const duels = ["A", "B", "C", "D", "E"].map((other) => duel(MESH, other));
  const choices: Record<string, string | null> = {};
  for (const d of duels) {
    pick(choices, d, 0);
  }
  const learned = learn(votesOf(choices, world(duels)));
  const favourite = learned.find((entry) => entry.id === favouriteId(MESH, "meshes"));
  assert.ok(favourite, "the mesh mod is the favourite for meshes");
  assert.equal(favourite.chosen, 5);
  assert.equal(favourite.of, 5);
  // Each pair was decided once: no pair preference.
  assert.equal(
    learned.some((entry) => entry.kind === "pair"),
    false,
  );
});

test("four decisions are not enough, and three of five is not clear", () => {
  const four = ["A", "B", "C", "D"].map((other) => duel(MESH, other));
  const choices: Record<string, string | null> = {};
  for (const d of four) {
    pick(choices, d, 0);
  }
  assert.equal(learn(votesOf(choices, world(four))).length, 0);

  const five = ["A", "B", "C", "D", "E"].map((other) => duel(MESH, other));
  const mixed: Record<string, string | null> = {};
  for (const [index, d] of five.entries()) {
    pick(mixed, d, index < 3 ? 0 : 1);
  }
  assert.equal(
    learn(votesOf(mixed, world(five))).some((entry) => entry.id === favouriteId(MESH, "meshes")),
    false,
  );

  // Four of five is 80 %: clear.
  const most: Record<string, string | null> = {};
  for (const [index, d] of five.entries()) {
    pick(most, d, index < 4 ? 0 : 1);
  }
  assert.ok(
    learn(votesOf(most, world(five))).some((entry) => entry.id === favouriteId(MESH, "meshes")),
  );
});

test("picks that follow a suggestion or a preference teach little or nothing", () => {
  const duels = ["A", "B", "C", "D", "E"].map((other) => duel(MESH, other, ["meshes"], 0));
  const choices: Record<string, string | null> = {};
  for (const d of duels) {
    pick(choices, d, 0);
  }
  assert.equal(learn(votesOf(choices, world(duels))).length, 0, "suggested picks weigh a quarter");

  // Twenty of them do make a preference.
  const many = Array.from({ length: 20 }, (_, index) =>
    duel(MESH, `Other ${index}`, ["meshes"], 0),
  );
  const all: Record<string, string | null> = {};
  for (const d of many) {
    pick(all, d, 0);
  }
  assert.ok(learn(votesOf(all, world(many))).some((entry) => entry.mod === MESH));

  // Picks the ranking made weigh a quarter too.
  const ranked = ["A", "B", "C", "D", "E"].map((other) => duel(MESH, other));
  const byRanking: Record<string, string | null> = {};
  for (const d of ranked) {
    pick(byRanking, d, 0);
  }
  assert.equal(learn(votesOf(byRanking, { ...world(ranked), ranked: () => 0 })).length, 0);

  // Picks a preference made weigh nothing.
  const own = ["A", "B", "C", "D", "E"].map((other) => duel(MESH, other));
  const fromPreference: Record<string, string | null> = {};
  for (const d of own) {
    pick(fromPreference, d, 0);
  }
  const applied = new Set(Object.keys(fromPreference));
  assert.equal(learn(votesOf(fromPreference, world(own), applied)).length, 0);
});

test("a duel about meshes and textures counts for both topics", () => {
  const duels = ["A", "B", "C", "D", "E"].map((other) => duel(MESH, other, ["meshes", "textures"]));
  const choices: Record<string, string | null> = {};
  for (const d of duels) {
    pick(choices, d, 0);
  }
  const ids = learn(votesOf(choices, world(duels))).map((entry) => entry.id);
  assert.ok(ids.includes(favouriteId(MESH, "meshes")));
  assert.ok(ids.includes(favouriteId(MESH, "textures")));
});

test("spots decided one by one make a preference of the pair", () => {
  const spots = [1, 2, 3, 4, 5].map((id) => spot(id));
  const choices: Record<string, string | null> = {};
  for (const one of spots) {
    choices[spotKey(one) ?? ""] = ROAD;
  }
  const learned = learn(votesOf(choices, world([], spots)));
  const pair = learned.find((entry) => entry.id === pairId("City Overhaul", "Road Mod"));
  assert.ok(pair, "the city is kept over the road");
  assert.equal(pair.kind, "pair");
  assert.equal(pair.chosen, 5);
  // Only one mod was beaten: no favourite of places.
  assert.equal(
    learned.some((entry) => entry.kind === "favourite"),
    false,
  );
  // Keeping both says nothing.
  const kept: Record<string, string | null> = {};
  for (const one of spots) {
    kept[spotKey(one) ?? ""] = null;
  }
  assert.equal(votesOf(kept, world([], spots)).length, 0);
});

test("a passage decided as a whole counts once, however many spots it has", () => {
  const spots = [1, 2, 3, 4, 5, 6].map((id) => spot(id));
  const passage = {
    plugins: [CITY, ROAD],
    parts: [
      spots.flatMap((one) => (one.parts ?? []).filter((part) => part.object.plugin === CITY)),
      spots.flatMap((one) => (one.parts ?? []).filter((part) => part.object.plugin === ROAD)),
    ],
    volumes: [10, 10],
    spots: 6,
    console: "",
  } as unknown as MapPassage;
  const choices: Record<string, string | null> = {};
  for (const one of spots) {
    choices[spotKey(one) ?? ""] = ROAD;
  }
  const votes = votesOf(choices, world([], spots, [passage]));
  const total = votes.reduce((sum, vote) => sum + vote.share, 0);
  assert.ok(Math.abs(total - 1) < 1e-9, "six spots of one passage are one decision");
  assert.equal(learn(votes).length, 0);
});

test("spots whose decision follows the size suggestion weigh a quarter", () => {
  const spots = [1, 2, 3, 4, 5].map((id) => spot(id, ROAD));
  const choices: Record<string, string | null> = {};
  for (const one of spots) {
    choices[spotKey(one) ?? ""] = ROAD;
  }
  const votes = votesOf(choices, world([], spots));
  assert.ok(votes.every((vote) => vote.weight === 0.25));
  assert.equal(learn(votes).length, 0);
});

test("stacked doors merged into one vote for the kept door's mod", () => {
  const doors = spot(1, null, true);
  const kept = doors.parts?.[0]?.object;
  assert.ok(kept);
  const votes = votesOf({ [spotKey(doors) ?? ""]: keepChoice(kept) }, world([], [doors]));
  assert.equal(votes.length, 1);
  assert.equal(votes[0]?.winner, "City Overhaul");
  assert.equal(votes[0]?.loser, "Road Mod");
});

test("a pair preference goes before a favourite, one said before one learned", () => {
  const conflict = conflictsOf(world([duel(MESH, "Rival")]))[0];
  assert.ok(conflict);
  const favourite: Rule = {
    id: "f",
    mod: MESH,
    over: null,
    topic: "meshes",
    explicit: false,
  };
  const rivalPair: Rule = { id: "p", mod: "Rival", over: MESH, topic: null, explicit: false };
  assert.equal(preferredSide([favourite], conflict)?.side, 0);
  assert.equal(preferredSide([favourite, rivalPair], conflict)?.side, 1);
  const said: Rule = { ...favourite, id: "s", explicit: true };
  const rivalFavourite: Rule = { ...favourite, id: "r", mod: "Rival" };
  assert.equal(preferredSide([rivalFavourite, said], conflict)?.side, 0);
  // Two learned favourites pulling both ways decide nothing.
  assert.equal(preferredSide([favourite, rivalFavourite], conflict), null);
  // A favourite for textures says nothing about meshes.
  assert.equal(preferredSide([{ ...favourite, topic: "textures" }], conflict), null);
});

test("the offer names the open conflicts, and the answers are kept", () => {
  const decided = ["A", "B", "C", "D", "E"].map((other) => duel(MESH, other));
  const open = ["F", "G"].map((other) => duel(MESH, other));
  const textures = duel(MESH, "H", ["textures"]);
  const all = [...decided, ...open, textures];
  const choices: Record<string, string | null> = {};
  for (const d of decided) {
    pick(choices, d, 0);
  }
  const theWorld = world(all);
  const learned = learn(votesOf(choices, theWorld));
  const conflicts = conflictsOf(theWorld);
  const check: OpenCheck = {
    duelOpen: (id) => choices[`duel:${id}`] === undefined,
    spotOpen: () => true,
  };
  const nothing: PatchPreferences = {};
  const offer = offerOf(learned, nothing, rulesOf(learned, nothing), conflicts, check);
  assert.ok(offer);
  assert.equal(offer.learned.id, favouriteId(MESH, "meshes"));
  assert.deepEqual(
    offer.items.map((item) => item.conflict.id),
    open.map((d) => d.id),
  );
  assert.ok(offer.items.every((item) => item.side === 0 && item.winner === MESH));

  // "Not now" waits for more decisions; "Never" also stops the preference.
  const later: PatchPreferences = {
    answers: { [offer.learned.id]: { answer: "later", at: offer.learned.of } },
  };
  assert.equal(offerOf(learned, later, rulesOf(learned, later), conflicts, check), null);
  const grown = { ...offer.learned, of: offer.learned.of + LATER_AFTER };
  assert.ok(offerOf([grown], later, rulesOf([grown], later), conflicts, check));
  const never: PatchPreferences = { answers: { [offer.learned.id]: { answer: "never", at: 5 } } };
  assert.equal(rulesOf(learned, never).length, 0);
  assert.equal(offerOf(learned, never, rulesOf(learned, never), conflicts, check), null);

  // Switched off, nothing learned counts, but what the user said does.
  const off: PatchPreferences = {
    learningOff: true,
    rules: [{ mod: "H", topic: "textures" }],
  };
  const rules = rulesOf(learned, off);
  assert.equal(rules.length, 1);
  assert.equal(offerOf(learned, off, rules, conflicts, check), null);
  const [own] = rules;
  assert.ok(own);
  assert.deepEqual(
    planOf(own, rules, conflicts, check).map((item) => item.conflict.id),
    [textures.id],
  );
});

test("applying decides the open spots, and one undo takes the batch back", () => {
  const spots = [1, 2, 3].map((id) => spot(id));
  const doors = spot(9, null, true);
  const theWorld = world([], [...spots, doors]);
  const conflicts = conflictsOf(theWorld);
  assert.equal(conflicts.length, 3, "stacked doors are left to the door panel");
  const before: Record<string, string | null> = { [spotKey(spots[0] as MapSpot) ?? ""]: CITY };
  const rule: Rule = {
    id: "own",
    mod: "City Overhaul",
    over: "Road Mod",
    topic: null,
    explicit: true,
  };
  const items = planOf(rule, [rule], conflicts, {
    duelOpen: () => true,
    spotOpen: (one) => before[spotKey(one) ?? ""] === undefined,
  });
  assert.equal(items.length, 2);
  const after = decideSpots(before, items);
  assert.ok(items.every((item) => after[item.conflict.id] === ROAD));
  const keys = changedKeys(before, after);
  assert.equal(keys.length, 2);
  // Something else decided in between stays.
  const later = { ...after, "spot:other": ROAD };
  assert.deepEqual(restoreKeys(later, before, keys), { ...before, "spot:other": ROAD });
});

test("the history reads every own patch, the edited one last", () => {
  const history = historyOf(
    [
      { id: "old", decisions: { a: "1", b: "1" } },
      { id: "edited", decisions: { a: "2" }, unticked: [] },
      { id: "theirs", decisions: { c: "3" }, foreign: true },
      { id: "newer", decisions: { a: "4", b: null, d: "5" }, unticked: ["d"] },
    ],
    "edited",
  );
  assert.deepEqual(history, { a: "2", b: null });
});
