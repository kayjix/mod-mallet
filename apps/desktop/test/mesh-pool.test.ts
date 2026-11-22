// Tests of what a camera sees and of the normals of a mesh, with made-up numbers.

import assert from "node:assert/strict";
import { test } from "node:test";
import { lookAt, multiply, perspective } from "../src/gl.ts";
import { normalsOf, Sight } from "../src/mesh-pool.ts";

/** A camera at the origin looking along +y, 800 pixels wide. */
const sight = () =>
  new Sight(
    multiply(perspective(Math.PI / 3, 1.5, 0.1, 100), lookAt([0, 0, 0], [0, 1, 0], [0, 0, 1])),
    800,
  );

test("a sphere ahead is seen", () => {
  assert.equal(sight().sees(0, 10, 0, 1), true);
});

test("a sphere behind the camera is not seen", () => {
  assert.equal(sight().sees(0, -10, 0, 1), false);
});

test("a sphere beside the view is not seen, unless it reaches into it", () => {
  assert.equal(sight().sees(40, 10, 0, 1), false);
  assert.equal(sight().sees(40, 10, 0, 40), true);
});

test("a sphere beyond the far plane is not seen", () => {
  assert.equal(sight().sees(0, 150, 0, 1), false);
});

test("a sphere too small to be seen is left out only when asked", () => {
  // 0.01 across at 90 units is far less than a pixel.
  assert.equal(sight().sees(0, 90, 0, 0.005), true);
  assert.equal(sight().sees(0, 90, 0, 0.005, 1.5), false);
  assert.equal(sight().sees(0, 10, 0, 1, 1.5), true);
});

test("a flat triangle's normals point up", () => {
  const normals = normalsOf(
    new Float32Array([0, 0, 0, 1, 0, 0, 0, 1, 0]),
    new Uint32Array([0, 1, 2]),
  );
  assert.deepEqual([...normals], [0, 0, 1, 0, 0, 1, 0, 0, 1]);
});

test("indices past the points are passed over", () => {
  const normals = normalsOf(new Float32Array([0, 0, 0]), new Uint32Array([0, 5, 9]));
  assert.deepEqual([...normals], [0, 0, 0]);
});
