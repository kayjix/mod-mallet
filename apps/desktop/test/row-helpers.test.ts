// Tests of where the arrow keys move the focus between rows.

import assert from "node:assert/strict";
import { test } from "node:test";
import { isRowKey, nextRow } from "../src/ui/row-helpers.ts";

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
