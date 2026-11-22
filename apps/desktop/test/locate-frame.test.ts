// Tests of where the camera goes to show located objects.

import assert from "node:assert/strict";
import { test } from "node:test";
import { framedDistance, framing } from "../src/locate-frame.ts";

test("no objects have no frame", () => {
  assert.equal(framing([]), null);
});

test("one object is framed on itself, with no reach", () => {
  assert.deepEqual(framing([[10, 20, 30]]), { target: [10, 20, 30], radius: 0 });
});

test("several objects are framed on their middle, reaching to the farthest", () => {
  const frame = framing([
    [0, 0, 0],
    [100, 0, 0],
  ]);
  assert.deepEqual(frame, { target: [50, 0, 0], radius: 50 });
});

test("a camera far from the objects comes closer, one too close backs off", () => {
  assert.equal(framedDistance(0.1, 5, 0.08), 0.26);
  assert.equal(framedDistance(0.1, 0.01, 0.08), 0.26);
});

test("a camera about right is left where it is", () => {
  assert.equal(framedDistance(0.1, 0.3, 0.08), 0.3);
});

test("a single object is looked at from at least the least distance", () => {
  assert.equal(framedDistance(0, 9, 0.08), 0.08);
});
