// Tests of deciding a whole passage and showing one of its parts again,
// with a made-up passage: a bunkhouse two mods build, one of them with a
// ladder on its roof.

import assert from "node:assert/strict";
import { test } from "node:test";
import { hiddenByHand, keepPassage } from "../src/passage-choices.ts";
import {
  chooseObject,
  decisionAt,
  hiddenKeysOf,
  matchKey,
  type PatchChoices,
  revealObjects,
  spotKey,
} from "../src/patch.ts";

const HOUSE = "House Mod.esp";
const TOWN = "Town Mod.esp";

const part = (plugin: string, localId: number, name: string, hideable = true) => ({
  object: { plugin, localId },
  name,
  hideable,
  suggested: false,
});

const wall = part(HOUSE, 1, "Wall");
const roof = part(HOUSE, 2, "Roof");
const townWall = part(TOWN, 11, "Wall");
const ladder = part(TOWN, 12, "Ladder");
const crate = part(TOWN, 13, "Crate");

const spotAt = (parts: ReturnType<typeof part>[]) => ({
  finding: "collision-house-town",
  parts,
  location: { type: "exterior", world: "Tamriel", x: 0, y: 0 },
  position: [0, 0, 0],
});
// Two spots: the walls inside each other, the roof through the ladder. The
// crate is part of the passage but of no spot.
const walls = spotAt([wall, townWall]);
const top = spotAt([roof, ladder]);
// biome-ignore lint/suspicious/noExplicitAny: made-up spots carry only what the code reads.
const spots = new Map<string, any[]>([["collision-house-town", [walls, top]]]);
const keys = [spotKey(walls as never), spotKey(top as never)] as string[];

const passage = {
  plugins: [HOUSE, TOWN] as [string, string],
  parts: [
    [wall, roof],
    [townWall, ladder, crate],
  ],
  min: [-100, -100, 0],
  max: [100, 100, 300],
};
const cluster = {
  plugins: passage.plugins,
  spots: keys.map((key) => ({ key })),
  passage: { passage: passage as never },
};

const hidden = (choices: PatchChoices) => hiddenKeysOf(spots, choices);

test("keeping one version hides the other's parts, spots and the part of no spot alike", () => {
  const next = keepPassage({}, spots, cluster, null, 0);
  for (const gone of [townWall, ladder, crate]) {
    assert.ok(hidden(next).has(matchKey(gone.object)), gone.name);
  }
  for (const kept of [wall, roof]) {
    assert.ok(!hidden(next).has(matchKey(kept.object)), kept.name);
  }
  for (const key of keys) {
    assert.equal(decisionAt(next, walls.finding, key, walls.parts as never), TOWN);
  }
});

test("a part the passage decision hid is shown again, and the passage keeps its decision elsewhere", () => {
  const decided = keepPassage({}, spots, cluster, null, 0);
  const shown = revealObjects(decided, spots, [ladder.object]);
  assert.ok(!hidden(shown).has(matchKey(ladder.object)), "the ladder is shown again");
  // Its spot keeps both; the other spot and the part of no spot stay decided.
  assert.equal(decisionAt(shown, top.finding, keys[1] as string, top.parts as never), null);
  assert.equal(decisionAt(shown, walls.finding, keys[0] as string, walls.parts as never), TOWN);
  assert.ok(hidden(shown).has(matchKey(townWall.object)));
  assert.ok(hidden(shown).has(matchKey(crate.object)));
});

test("what the game ties in stays when the other version goes", () => {
  const next = keepPassage({}, spots, cluster, null, 0, new Set([matchKey(crate.object)]));
  assert.ok(!hidden(next).has(matchKey(crate.object)));
});

test("keeping a version shows its parts hidden by hand, and says which they were", () => {
  const byHand = chooseObject({}, wall.object, true);
  const found = hiddenByHand(byHand, passage as never, null, null, 0);
  assert.deepEqual(found.map(matchKey), [matchKey(wall.object)]);
  const next = revealObjects(keepPassage(byHand, spots, cluster, null, 0), spots, found);
  assert.ok(!hidden(next).has(matchKey(wall.object)));
});

test("keeping both shows every part of the passage", () => {
  const decided = keepPassage({}, spots, cluster, null, 0);
  const both = keepPassage(decided, spots, cluster, null, null);
  assert.equal(hidden(both).size, 0);
});
