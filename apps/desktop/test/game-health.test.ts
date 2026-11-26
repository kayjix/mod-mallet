// Tests of the health every game shows on the "All games" page (ADR 0048).

import assert from "node:assert/strict";
import { test } from "node:test";
import { gameScoreOf, lastHealthOf, sameHealth } from "../src/game-health.ts";
import type { KnownGame } from "../src/generated/game-choice";

const game = (id: string, lastHealth?: KnownGame["lastHealth"]): KnownGame => ({
  id,
  name: id,
  support: "full",
  dir: "D:/Games/x",
  origin: "steam",
  scriptExtender: "SKSE",
  lastHealth,
});

test("the score kept is whole, within 0 and 100, with the scan it was weighed from", () => {
  assert.deepEqual(lastHealthOf({ score: 71.6, verdict: "suffers" }, 1_764_000_000), {
    score: 72,
    verdict: "suffers",
    scannedAt: 1_764_000_000,
  });
  assert.equal(lastHealthOf({ score: 140, verdict: "good" }, 1).score, 100);
  assert.equal(lastHealthOf({ score: -3, verdict: "crashes" }, 1).score, 0);
});

test("the settings are written only when the score, verdict or scan changed", () => {
  const kept = lastHealthOf({ score: 80, verdict: "suffers" }, 100);
  assert.ok(sameHealth(kept, { ...kept }));
  assert.ok(!sameHealth(kept, { ...kept, score: 81 }));
  assert.ok(!sameHealth(kept, { ...kept, verdict: "good" }));
  assert.ok(!sameHealth(kept, { ...kept, scannedAt: 101 }));
  assert.ok(!sameHealth(null, kept));
  assert.ok(!sameHealth(undefined, kept));
});

test("the game in use shows its score of now, every other scanned game its last one", () => {
  const now = lastHealthOf({ score: 64, verdict: "suffers" }, 200);
  const before = lastHealthOf({ score: 50, verdict: "crashes" }, 100);
  const other = lastHealthOf({ score: 92, verdict: "good" }, 150);
  assert.deepEqual(gameScoreOf(game("skyrimse", before), true, now, true), {
    type: "score",
    health: now,
    now: true,
  });
  assert.deepEqual(gameScoreOf(game("fallout4", other), false, now, true), {
    type: "score",
    health: other,
    now: false,
  });
  // The game in use whose scan is gone still shows what it had.
  assert.deepEqual(gameScoreOf(game("skyrimse", before), true, null, true), {
    type: "score",
    health: before,
    now: false,
  });
});

test("a game never scanned says so; one that is not here says nothing", () => {
  assert.deepEqual(gameScoreOf(game("falloutnv"), false, null, true), { type: "unscanned" });
  assert.deepEqual(gameScoreOf(game("skyrimse"), true, null, true), { type: "unscanned" });
  assert.deepEqual(gameScoreOf(game("oblivion"), false, null, false), { type: "none" });
  assert.deepEqual(gameScoreOf(undefined, false, null, false), { type: "none" });
});
