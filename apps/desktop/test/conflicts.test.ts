// Tests of which spots a picked object is part of, with made-up spots.

import assert from "node:assert/strict";
import { test } from "node:test";
import { collidingWith, conflictsOf } from "../src/conflicts.ts";

const part = (plugin: string, localId: number, name: string) => ({
  object: { plugin, localId },
  name,
});

const shelf = {
  id: "shelf",
  parts: [part("Old Town.esp", 1, "Shelf"), part("Water Mod.esp", 7, "Divider")],
};
const dock = {
  id: "dock",
  parts: [part("Old Town.esp", 2, "Dock"), part("Water Mod.esp", 8, "Dock")],
};
const both = {
  id: "both",
  parts: [part("Old Town.esp", 1, "Shelf"), part("Texture Pack.esp", 9, "Stall")],
};
const spots = [shelf, dock, both];

const undecided = () => undefined;
const never = () => false;

test("every open spot the object is part of is listed, the others are not", () => {
  const found = conflictsOf(spots, [{ plugin: "Old Town.esp", localId: 1 }], undecided, never);
  assert.deepEqual(
    found.open.map((spot) => spot.id),
    ["shelf", "both"],
  );
  assert.equal(found.decidedByHiding.length, 0);
});

test("plugins are told apart without regard to case", () => {
  const found = conflictsOf(spots, [{ plugin: "old town.ESP", localId: 2 }], undecided, never);
  assert.deepEqual(
    found.open.map((spot) => spot.id),
    ["dock"],
  );
});

test("a spot decided by a part hidden on its own says so, one with a choice of its own does not", () => {
  const decision = (spot: { id: string }) => (spot.id === "dock" ? "Old Town.esp" : undefined);
  const byHiding = conflictsOf(spots, [{ plugin: "Old Town.esp", localId: 2 }], decision, never);
  assert.deepEqual(
    byHiding.decidedByHiding.map((spot) => spot.id),
    ["dock"],
  );
  assert.equal(byHiding.open.length, 0);
  const chosen = conflictsOf(spots, [{ plugin: "Old Town.esp", localId: 2 }], decision, () => true);
  assert.equal(chosen.decidedByHiding.length, 0);
  assert.equal(chosen.open.length, 0);
});

test("a spot kept as both is decided, but not by hiding", () => {
  const found = conflictsOf(spots, [{ plugin: "Old Town.esp", localId: 2 }], () => null, never);
  assert.equal(found.open.length, 0);
  assert.equal(found.decidedByHiding.length, 0);
});

test("spots another object of the list is part of are left out of the second list", () => {
  const found = conflictsOf(
    spots,
    [
      { plugin: "Old Town.esp", localId: 1 },
      { plugin: "Old Town.esp", localId: 2 },
    ],
    undecided,
    never,
    [{ plugin: "Old Town.esp", localId: 1 }],
  );
  assert.deepEqual(
    found.open.map((spot) => spot.id),
    ["dock"],
  );
});

test("what a spot collides with are the names of the other plugins' parts", () => {
  assert.deepEqual(collidingWith(shelf, [{ plugin: "Old Town.esp", localId: 1 }]), ["Divider"]);
  assert.deepEqual(collidingWith(both, [{ plugin: "Old Town.esp", localId: 1 }]), ["Stall"]);
  assert.deepEqual(collidingWith({}, []), []);
});
