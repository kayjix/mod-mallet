// Tests of what the magic wand offers for a selection, with made-up parts:
// no view, no game files. Run by `npm test --workspace apps/desktop`.

import assert from "node:assert/strict";
import { test } from "node:test";
import { offersFor, splitParts } from "../src/wand-parts.ts";

/** A part with a name to tell it by and whether the game relies on it. */
const part = (name: string, anchored = false) => ({ name, anchored });

const names = (parts: { name: string }[]) => parts.map((one) => one.name);

test("a selection of visible parts offers hiding them, and nothing else", () => {
  const group = [part("a"), part("b"), part("c")];
  const split = splitParts(group, () => false);
  assert.deepEqual(names(split.visible), ["a", "b", "c"]);
  assert.equal(split.hidden.length, 0);
  assert.deepEqual(offersFor(split, false), [{ direction: "hide", count: 3, primary: true }]);
});

test("a selection of hidden parts offers showing them again, and nothing else", () => {
  const group = [part("a"), part("b")];
  const split = splitParts(group, () => true);
  assert.deepEqual(names(split.hidden), ["a", "b"]);
  assert.deepEqual(offersFor(split, true), [{ direction: "show", count: 2, primary: true }]);
});

test("a mixed selection offers both ways, the picked object's own first", () => {
  const hiddenNames = new Set(["a", "b"]);
  const group = [part("a"), part("b"), part("c"), part("d"), part("e")];
  const split = splitParts(group, (one) => hiddenNames.has(one.name));
  assert.equal(split.hidden.length, 2);
  assert.equal(split.visible.length, 3);
  assert.deepEqual(offersFor(split, true), [
    { direction: "show", count: 2, primary: true },
    { direction: "hide", count: 3, primary: false },
  ]);
  assert.deepEqual(offersFor(split, false), [
    { direction: "hide", count: 3, primary: true },
    { direction: "show", count: 2, primary: false },
  ]);
});

test("parts the game relies on are never offered for hiding, and are counted as staying", () => {
  const group = [part("a"), part("tied", true), part("b")];
  const split = splitParts(group, () => false);
  assert.deepEqual(names(split.hideable), ["a", "b"]);
  assert.equal(split.tied, 1);
  assert.deepEqual(offersFor(split, false), [{ direction: "hide", count: 2, primary: true }]);
});

test("hidden parts the game relies on can still be shown again", () => {
  const group = [part("tied", true), part("a")];
  const split = splitParts(group, (one) => one.name === "tied");
  assert.equal(split.tied, 0);
  assert.deepEqual(offersFor(split, true), [
    { direction: "show", count: 1, primary: true },
    { direction: "hide", count: 1, primary: false },
  ]);
});

test("a selection of only tied visible parts offers nothing", () => {
  const split = splitParts([part("tied", true)], () => false);
  assert.deepEqual(offersFor(split, false), []);
});

test("an empty selection offers nothing", () => {
  assert.deepEqual(
    offersFor(
      splitParts([], () => false),
      false,
    ),
    [],
  );
});
