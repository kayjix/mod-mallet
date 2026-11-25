// Tests of the queue that asks for small pictures of meshes, with a made-up engine.

import assert from "node:assert/strict";
import { test } from "node:test";
import type { MeshThumbnail } from "../src/generated/mesh-thumbnail.ts";
import { ThumbQueue } from "../src/mesh-thumbs.ts";

type Asked = { path: string; version: string | null }[];

/** An engine that answers when told to, and remembers what it was asked. */
function engine() {
  const calls: Asked[] = [];
  const answers: (() => void)[] = [];
  const ask = (requests: Asked) => {
    calls.push(requests);
    return new Promise<MeshThumbnail[]>((resolve) => {
      answers.push(() =>
        resolve(requests.map(({ path, version }) => ({ path, version, image: `img:${path}` }))),
      );
    });
  };
  return { calls, answers, ask };
}

const settle = () => new Promise((resolve) => setTimeout(resolve, 0));

test("rows that come into view together are asked for in one call", async () => {
  const { calls, answers, ask } = engine();
  const queue = new ThumbQueue(ask);
  const found = ["a", "b", "c"].map((path) => queue.get(path, null));
  await settle();
  assert.equal(calls.length, 1);
  assert.deepEqual(
    calls[0]?.map((request) => request.path),
    ["a", "b", "c"],
  );
  for (const answer of answers) {
    answer();
  }
  assert.deepEqual(
    (await Promise.all(found)).map((thumbnail) => thumbnail?.image),
    ["img:a", "img:b", "img:c"],
  );
});

test("a picture is asked for once and the newest rows go first", async () => {
  const { calls, answers, ask } = engine();
  const queue = new ThumbQueue(ask);
  for (let index = 0; index < 20; index++) {
    queue.get(`m${index}`, null);
  }
  queue.get("m3", null);
  await settle();
  // Two calls at once, six pictures each, the last rows first.
  assert.equal(calls.length, 2);
  assert.deepEqual(
    calls[0]?.map((request) => request.path),
    ["m14", "m15", "m16", "m17", "m18", "m19"],
  );
  assert.equal(queue.pending, 8);
  answers[0]?.();
  await settle();
  assert.equal(calls.length, 3);
});

test("a row scrolled away before its turn is not asked for", async () => {
  const { calls, ask } = engine();
  const queue = new ThumbQueue(ask);
  for (let index = 0; index < 12; index++) {
    queue.get(`m${index}`, null);
  }
  await settle();
  queue.get("late", "mod:x");
  queue.forget("late", "mod:x");
  assert.equal(queue.pending, 0);
  assert.equal(
    calls.flat().some((request) => request.path === "late"),
    false,
  );
});

test("a picture two rows want stays wanted when one goes", async () => {
  const { ask } = engine();
  const queue = new ThumbQueue(ask);
  for (let index = 0; index < 12; index++) {
    queue.get(`m${index}`, null);
  }
  await settle();
  queue.get("shared", null);
  queue.get("shared", null);
  queue.forget("shared", null);
  assert.equal(queue.pending, 1);
});

test("a failed call is asked again next time", async () => {
  let fail = true;
  let calls = 0;
  const queue = new ThumbQueue(async (requests) => {
    calls += 1;
    if (fail) {
      throw new Error("no engine");
    }
    return requests.map(({ path }) => ({ path, image: "x" }));
  });
  assert.equal(await queue.get("a", null), null);
  fail = false;
  assert.equal((await queue.get("a", null))?.image, "x");
  assert.equal(calls, 2);
});
