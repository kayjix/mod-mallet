// Tests of the patch book (ADR 0070): which patches count, which one is
// being edited, and where a page's change goes, with made-up decisions.

import assert from "node:assert/strict";
import { test } from "node:test";
import {
  activePatch,
  bookFromChoices,
  countedChoices,
  withChange,
  withDuplicate,
  withLoaded,
  withNewPatch,
  withPatch,
} from "../src/book.ts";

const spot = "spot:pair|Town.esp:1";
const other = "spot:pair|Road.esp:2";

/** A book with one patch of 2 decisions, switched off, and a new empty one being edited. */
function bookWithANewPatch() {
  const first = bookFromChoices({ [spot]: "Town.esp", [other]: null, "pin:spot:x": "pinned" });
  const firstId = first.active;
  const off = withPatch(first, firstId, (patch) => ({ ...patch, off: true }));
  const { book, id } = withNewPatch(off, "New patch");
  return { book, firstId, newId: id };
}

test("the first book keeps every decision in one patch and the pins apart", () => {
  const book = bookFromChoices({ [spot]: "Town.esp", "pin:spot:x": "pinned" });
  assert.equal(book.patches.length, 1);
  assert.deepEqual(book.patches[0]?.decisions, { [spot]: "Town.esp" });
  assert.deepEqual(book.pins, ["pin:spot:x"]);
  assert.equal(countedChoices(book)["pin:spot:x"], "pinned");
});

test("a new patch is the one being edited, and a switched-off patch does not count", () => {
  const { book, newId } = bookWithANewPatch();
  assert.equal(activePatch(book)?.id, newId);
  assert.equal(spot in countedChoices(book), false);
});

test("the patch being edited counts even when it is switched off, and wins", () => {
  const { book, firstId, newId } = bookWithANewPatch();
  // The first patch on again and deciding the spot one way, the edited one the other.
  let both = withPatch(book, firstId, (patch) => ({ ...patch, off: false }));
  both = withPatch(both, newId, (patch) => ({
    ...patch,
    off: true,
    decisions: { [spot]: "Road.esp" },
  }));
  assert.equal(countedChoices(both)[spot], "Road.esp");
});

test("a change on a page goes into the patch being edited, not into the one that decided it", () => {
  const { book, firstId, newId } = bookWithANewPatch();
  const on = withPatch(book, firstId, (patch) => ({ ...patch, off: false }));
  const before = countedChoices(on);
  const after = withChange(on, before, { ...before, [spot]: "Road.esp" });
  const first = after.patches.find((patch) => patch.id === firstId);
  const edited = after.patches.find((patch) => patch.id === newId);
  assert.equal(first?.decisions[spot], "Town.esp");
  assert.equal(edited?.decisions[spot], "Road.esp");
  assert.equal(countedChoices(after)[spot], "Road.esp");
});

test("a decision taken back is unticked in the other patch, which keeps it", () => {
  const { book, firstId } = bookWithANewPatch();
  const on = withPatch(book, firstId, (patch) => ({ ...patch, off: false }));
  const before = countedChoices(on);
  const { [spot]: _gone, ...rest } = before;
  const after = withChange(on, before, rest);
  const first = after.patches.find((patch) => patch.id === firstId);
  assert.equal(first?.decisions[spot], "Town.esp");
  assert.deepEqual(first?.unticked, [spot]);
  assert.equal(spot in countedChoices(after), false);
});

test("duplicating copies the decisions, edits the copy and switches the original off", () => {
  const { book, firstId } = bookWithANewPatch();
  const on = withPatch(book, firstId, (patch) => ({ ...patch, off: false }));
  const { book: copied, id } = withDuplicate(on, firstId, "Mod Mallet Patch (copy)");
  const copy = copied.patches.find((patch) => patch.id === id);
  assert.equal(copied.active, id);
  assert.deepEqual(copy?.decisions, { [spot]: "Town.esp", [other]: null });
  assert.equal(copied.patches.find((patch) => patch.id === firstId)?.off, true);
  // What the copy takes back is gone from the views at once.
  const before = countedChoices(copied);
  const { [spot]: _gone, ...rest } = before;
  assert.equal(spot in countedChoices(withChange(copied, before, rest)), false);
});

test("a loaded patch by somebody else is never the one being edited", () => {
  const { book, newId } = bookWithANewPatch();
  const { book: loaded } = withLoaded(book, {
    archive: "Two Towns.zip",
    name: "Two Towns",
    author: "Someone",
    version: "1.0",
    decisions: { [spot]: "Town.esp" },
  });
  assert.equal(activePatch(loaded)?.id, newId);
});
