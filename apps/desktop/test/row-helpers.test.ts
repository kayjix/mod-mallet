// Tests of what the rows of the UI kit work out for themselves: a mod's
// initials and colour, and where the arrow keys move the focus.

import assert from "node:assert/strict";
import { test } from "node:test";
import { hueOf, initialsOf, isRowKey, nextRow } from "../src/ui/row-helpers.ts";

test("initials say which mod it is", () => {
  assert.equal(initialsOf("Unofficial Skyrim Special Edition Patch"), "US");
  assert.equal(initialsOf("LegacyoftheDragonborn"), "LD");
  assert.equal(initialsOf("JK's Skyrim all in one"), "JS");
  assert.equal(initialsOf("-Skyrim 202X 10.5.2 - Architecture PART 1"), "S2");
  assert.equal(initialsOf("Wyrmstooth"), "WY");
  assert.equal(initialsOf("BSHeartland.esm"), "BH");
  assert.equal(initialsOf("the"), "TH");
  assert.equal(initialsOf("--"), "?");
  assert.equal(initialsOf(""), "?");
  assert.equal(initialsOf("Ébène des Ïles"), "ÉD");
});

test("a mod keeps its hue, and hues spread", () => {
  assert.equal(hueOf("Town"), hueOf("Town"));
  const hues = new Set(["Town", "Castle", "Trees", "Faces", "Patch", "Water"].map(hueOf));
  assert.ok(hues.size >= 5);
  for (const hue of hues) {
    assert.ok(hue >= 0 && hue < 360);
  }
});

test("the arrow keys move between rows and stop at the ends", () => {
  assert.equal(nextRow(0, 3, "ArrowDown"), 1);
  assert.equal(nextRow(2, 3, "ArrowDown"), 2);
  assert.equal(nextRow(0, 3, "ArrowUp"), 0);
  assert.equal(nextRow(2, 3, "ArrowUp"), 1);
  assert.equal(nextRow(1, 3, "Home"), 0);
  assert.equal(nextRow(1, 3, "End"), 2);
  // Not on a row yet: down goes to the first.
  assert.equal(nextRow(-1, 3, "ArrowDown"), 0);
  assert.equal(nextRow(0, 0, "ArrowDown"), -1);
  assert.ok(isRowKey("Home"));
  assert.ok(!isRowKey("Enter"));
});
