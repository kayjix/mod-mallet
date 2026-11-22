// Tests of where located objects are, and of the arrow that points to
// those out of sight.

import assert from "node:assert/strict";
import { test } from "node:test";
import { edgePointer, framing } from "../src/locate-frame.ts";

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

// With the identity, a point's x and y are where it lands, from -1 to 1.
const identity = [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1];

test("a point in sight gets no arrow", () => {
  assert.equal(edgePointer(identity, [0.5, -0.5, 0], 200, 100), null);
});

test("a point to the right gets an arrow at the right edge, pointing right", () => {
  const pointer = edgePointer(identity, [3, 0, 0], 200, 100, 10);
  assert.deepEqual(pointer, { x: 190, y: 50, angle: 0 });
});

test("a point above gets an arrow at the top edge, pointing up", () => {
  const pointer = edgePointer(identity, [0, 5, 0], 200, 100, 10);
  assert.equal(pointer?.x, 100);
  assert.equal(pointer?.y, 10);
  assert.equal(pointer?.angle, -Math.PI / 2);
});

test("a point behind the camera is pointed to on its side", () => {
  // w of -1: behind. Its x is to the right, so the arrow is too.
  const behind = [...identity];
  behind[15] = -1;
  const pointer = edgePointer(behind, [0.2, 0, 0], 200, 100, 10);
  assert.equal(pointer?.x, 190);
  assert.equal(pointer?.angle, 0);
});
