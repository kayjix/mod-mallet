// Tests of the part lists of a passage, with made-up parts: no view, no
// game files. Run by `npm test --workspace apps/desktop`.

import assert from "node:assert/strict";
import { test } from "node:test";
import { groupByName, pickRows, readableName, stillColliding } from "../src/part-groups.ts";

test("a model's path reads as its file name without the ending", () => {
  assert.equal(readableName("meshes\\architecture\\WRIntShield02.nif"), "WRIntShield02");
  assert.equal(readableName("TreeFloraHangingMoss01"), "TreeFloraHangingMoss01");
  assert.equal(readableName("a/b/Plank.NIF"), "Plank");
  assert.equal(readableName(".nif"), ".nif");
});

test("parts of one name go together, names in alphabetical order", () => {
  const parts = [
    { name: "Plank", id: 1 },
    { name: "beam.nif", id: 2 },
    { name: "x\\Plank.nif", id: 3 },
    { name: "Beam", id: 4 },
  ];
  const groups = groupByName(parts);
  assert.deepEqual(
    groups.map((group) => [group.name, group.members.map((member) => member.id)]),
    [
      ["beam", [2, 4]],
      ["Plank", [1, 3]],
    ],
  );
});

const rows = [["a"], ["b1", "b2"], ["c"], ["d"]];

test("a plain click picks only its row, and a second one lets it go", () => {
  const once = pickRows(new Set(["a", "c"]), rows, null, 1, "only");
  assert.deepEqual([...once].sort(), ["b1", "b2"]);
  assert.deepEqual([...pickRows(once, rows, 1, 1, "only")], []);
});

test("Ctrl adds a row or takes it away", () => {
  const added = pickRows(new Set(["a"]), rows, 0, 2, "toggle");
  assert.deepEqual([...added].sort(), ["a", "c"]);
  assert.deepEqual([...pickRows(added, rows, 2, 0, "toggle")], ["c"]);
  // A name with some of its parts picked is picked whole.
  assert.deepEqual([...pickRows(new Set(["b1"]), rows, 0, 1, "toggle")].sort(), ["b1", "b2"]);
});

test("Shift picks every row from the last one clicked, either way", () => {
  assert.deepEqual([...pickRows(new Set(["a"]), rows, 0, 2, "range")].sort(), [
    "a",
    "b1",
    "b2",
    "c",
  ]);
  assert.deepEqual([...pickRows(new Set(), rows, 3, 2, "range")].sort(), ["c", "d"]);
  // Without a row clicked before, Shift picks the row alone.
  assert.deepEqual([...pickRows(new Set(["a"]), rows, null, 3, "range")], ["d"]);
});

test("what of the losing mod is left where the winner is still there", () => {
  const spots = [
    {
      key: "s1",
      parts: [
        { key: "lux:1", plugin: "Lux Via.esp" },
        { key: "jk:1", plugin: "JK.esp" },
      ],
    },
    {
      key: "s2",
      parts: [
        { key: "lux:2", plugin: "Lux Via.esp" },
        { key: "jk:2", plugin: "JK.esp" },
      ],
    },
    {
      key: "s3",
      parts: [
        { key: "lux:1", plugin: "lux via.esp" },
        { key: "jk:3", plugin: "JK.esp" },
      ],
    },
  ];
  // lux:2 was hidden; lux:1 stays (tied in) and still meets JK at two spots.
  const left = stillColliding(spots, "Lux Via.esp", new Set(["lux:2"]));
  assert.deepEqual(left, { spots: ["s1", "s3"], parts: ["lux:1"] });
  // Where the winner's part is gone too, nothing collides there any more.
  assert.deepEqual(stillColliding(spots, "Lux Via.esp", new Set(["lux:2", "jk:1", "jk:3"])), {
    spots: [],
    parts: [],
  });
});
