// Tests of what the magic wand takes at each level, with made-up scenes of
// boxes: no view, no game files. Run by `npm test --workspace apps/desktop`.

import assert from "node:assert/strict";
import { test } from "node:test";
import { CELLS, occupancy } from "../src/boxes.ts";
import { WAND_LEVELS, type WandPart, wandChain } from "../src/wand-chain.ts";

/** A part of a scene: a name to tell it by, and the wand's view of it. */
interface Named extends WandPart {
  name: string;
}

/**
 * A part with a box standing straight, from its middle and half sizes,
 * of the mod `owner`; its model lies in the folder `set`.
 */
function part(
  name: string,
  set: string,
  centre: [number, number, number],
  half: [number, number, number],
  owner = "town.esp",
): Named {
  return {
    name,
    model: `meshes/${set}/${name.replace(/[^a-z]/gi, "")}.nif`,
    owner,
    plugin: owner,
    box: {
      centre,
      axes: [
        [1, 0, 0],
        [0, 1, 0],
        [0, 0, 1],
      ],
      half,
    },
    size: 2 * Math.max(...half),
    cells: null,
  };
}

/**
 * A house of the set `houses` with its middle at `x`: a floor, four walls
 * standing on it and a roof on the walls, 4.2 m wide.
 */
function house(prefix: string, x: number): Named[] {
  return [
    part(`${prefix}Floor`, "houses", [x, 0, 10], [210, 210, 10]),
    part(`${prefix}WallSouth`, "houses", [x, -200, 170], [210, 10, 150]),
    part(`${prefix}WallNorth`, "houses", [x, 200, 170], [210, 10, 150]),
    part(`${prefix}WallWest`, "houses", [x - 200, 0, 170], [10, 210, 150]),
    part(`${prefix}WallEast`, "houses", [x + 200, 0, 170], [10, 210, 150]),
    part(`${prefix}Roof`, "houses", [x, 0, 335], [220, 220, 15]),
  ];
}

/**
 * A scaffold tower of the set `scaffold`: legs, a platform on them, legs
 * on that, a platform on top with rails round it, and a ramp set against
 * the lower platform.
 */
function tower(): Named[] {
  const corners: [number, number][] = [
    [-90, -90],
    [90, -90],
    [90, 90],
    [-90, 90],
  ];
  return [
    ...corners.map(([x, y], index) => part(`Leg${index}`, "scaffold", [x, y, 150], [10, 10, 150])),
    part("PlatformLow", "scaffold", [0, 0, 310], [110, 110, 10]),
    ...corners.map(([x, y], index) =>
      part(`UpperLeg${index}`, "scaffold", [x, y, 470], [10, 10, 150]),
    ),
    part("PlatformTop", "scaffold", [0, 0, 630], [110, 110, 10]),
    part("RailSouth", "scaffold", [0, -105, 670], [110, 5, 30]),
    part("RailNorth", "scaffold", [0, 105, 670], [110, 5, 30]),
    part("RailWest", "scaffold", [-105, 0, 670], [5, 110, 30]),
    part("RailEast", "scaffold", [105, 0, 670], [5, 110, 30]),
    part("Ramp", "scaffold", [190, 0, 300], [100, 40, 10]),
  ];
}

/** The names of the parts the wand takes from the part named `start` at `level`, sorted. */
function taken(parts: Named[], start: string, level: number): string[] {
  const index = parts.findIndex((one) => one.name === start);
  assert.ok(index >= 0, `no part ${start}`);
  return wandChain(parts, index, level)
    .map((at) => parts[at]?.name ?? "?")
    .sort();
}

const names = (parts: Named[]) => parts.map((one) => one.name).sort();

test("this building takes one of two houses 2 m apart", () => {
  const first = house("A", 0);
  const second = house("B", 560);
  const scene = [...first, ...second];
  assert.deepEqual(taken(scene, "AWallSouth", WAND_LEVELS.structure), names(first));
  assert.deepEqual(taken(scene, "AWallSouth", WAND_LEVELS.building), names(first));
  assert.deepEqual(taken(scene, "BRoof", WAND_LEVELS.building), names(second));
});

test("this building takes a tower with all its scaffolds and rails", () => {
  const scene = tower();
  assert.deepEqual(taken(scene, "RailNorth", WAND_LEVELS.structure), names(scene));
  assert.deepEqual(taken(scene, "Leg2", WAND_LEVELS.building), names(scene));
});

test("a city wall touching the tower is not taken with the building", () => {
  const scaffold = tower();
  // A wall of overlapping pieces 7 m high along the south side of the
  // tower, 10 cm from its legs, of the same mod.
  const wall = [-3, -2, -1, 0, 1, 2, 3].map((step) =>
    part(`Wall${step + 3}`, "citywalls", [step * 350, -140, 350], [200, 30, 350]),
  );
  const scene = [...scaffold, ...wall];
  assert.deepEqual(taken(scene, "RailNorth", WAND_LEVELS.structure), names(scaffold));
  assert.deepEqual(taken(scene, "RailNorth", WAND_LEVELS.building), names(scaffold));
  // Everything joined still reaches the wall where it touches.
  assert.ok(taken(scene, "Wall3", WAND_LEVELS.joined).length >= wall.length);
});

test("a tower set into a city wall at one place keeps the wall", () => {
  const scaffold = tower();
  const wall = [-3, -2, -1, 0, 1, 2, 3].map((step) =>
    part(`Wall${step + 3}`, "citywalls", [step * 350, -150, 350], [200, 30, 350]),
  );
  // The top platform reaches into the wall: one link between the two.
  const beam = part("Beam", "scaffold", [0, -130, 630], [20, 40, 10]);
  const scene = [...scaffold, beam, ...wall];
  assert.deepEqual(taken(scene, "RailNorth", WAND_LEVELS.building), names([...scaffold, beam]));
});

test("a building of two sets of models: the structure keeps to the picked one's set", () => {
  const scaffold = tower();
  // A stone gate standing on the tower's top platform.
  const gate = [
    part("GatePostWest", "castle", [-60, 0, 740], [15, 15, 100]),
    part("GatePostEast", "castle", [60, 0, 740], [15, 15, 100]),
    part("GateArch", "castle", [0, 0, 855], [90, 20, 15]),
  ];
  const scene = [...scaffold, ...gate];
  assert.deepEqual(taken(scene, "Leg0", WAND_LEVELS.structure), names(scaffold));
  assert.deepEqual(taken(scene, "Leg0", WAND_LEVELS.building), names(scene));
});

test("parts of another mod are never taken", () => {
  const ours = house("A", 0);
  const theirs = house("B", 0).map((one) => ({ ...one, owner: "other.esp", plugin: "other.esp" }));
  const scene = [...ours, ...theirs];
  for (const level of Object.values(WAND_LEVELS)) {
    assert.ok(taken(scene, "AFloor", level).every((name) => name.startsWith("A")));
  }
  assert.deepEqual(taken(scene, "AFloor", WAND_LEVELS.building), names(ours));
});

test("each level takes at least what the structure below it takes", () => {
  const scene = [...tower(), ...house("H", 900)];
  const structure = taken(scene, "Ramp", WAND_LEVELS.structure);
  const building = taken(scene, "Ramp", WAND_LEVELS.building);
  const joined = taken(scene, "Ramp", WAND_LEVELS.joined);
  assert.ok(structure.every((name) => building.includes(name)));
  assert.ok(building.every((name) => joined.includes(name)));
});

test("a hollow tower does not take what stands in its empty middle", () => {
  // A tower of one model: four walls round an empty room, 3 m across.
  const walls: { positions: Float32Array; indices: Uint32Array }[] = [];
  const quad = (points: number[][]) => {
    walls.push({
      positions: new Float32Array(points.flat()),
      indices: new Uint32Array([0, 1, 2, 0, 2, 3]),
    });
  };
  quad([
    [-150, -150, 0],
    [150, -150, 0],
    [150, -150, 600],
    [-150, -150, 600],
  ]);
  quad([
    [-150, 150, 0],
    [150, 150, 0],
    [150, 150, 600],
    [-150, 150, 600],
  ]);
  quad([
    [-150, -150, 0],
    [-150, 150, 0],
    [-150, 150, 600],
    [-150, -150, 600],
  ]);
  quad([
    [150, -150, 0],
    [150, 150, 0],
    [150, 150, 600],
    [150, -150, 600],
  ]);
  const cells = occupancy(
    [
      [-150, -150, 0],
      [150, 150, 600],
    ],
    walls,
  );
  assert.equal(cells.length, CELLS ** 3);
  assert.equal(cells[(2 * CELLS + 2) * CELLS + 3], 0);
  const shell = { ...part("Shell", "towers", [0, 0, 300], [150, 150, 300]), cells };
  // A post of the same mod in the middle of the empty room, touching nothing.
  const post = part("Post", "towers", [0, 0, 300], [10, 10, 200]);
  assert.deepEqual(taken([shell, post], "Shell", WAND_LEVELS.building), ["Shell"]);
  assert.deepEqual(taken([shell, post], "Shell", WAND_LEVELS.contents), ["Shell"]);
});
