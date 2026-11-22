// A made-up level of detail for the sample (ADR 0037, 0074), shown with
// `?lod` in the address: a few blocks of land around the docks with
// reduced houses on them. Two houses are also placed up close, one is
// only in the level of detail, as a town that stands in another world.

import type { AreaObject } from "../generated/area-objects";
import type { LodMesh } from "../generated/lod-mesh";
import type { LodTile } from "../generated/lod-tiles";
import type { MapTerrain } from "../generated/rendered-report";

const CELL = 4096;
/** Whether the sample draws its made-up level of detail. */
export const SAMPLE_LOD =
  typeof window !== "undefined" && new URLSearchParams(window.location.search).has("lod");

/** The houses: where they stand, half their size, and whether they are placed up close too. */
const HOUSES: { at: [number, number]; half: [number, number, number]; real: boolean }[] = [
  { at: [27324, 6744], half: [150, 110, 120], real: true },
  { at: [26924, 5244], half: [120, 160, 150], real: true },
  // In the level of detail only: it stays, however close the camera.
  { at: [25824, 7044], half: [140, 140, 130], real: false },
];
/** The model of a placed house; its size is in {@link houseBox}. */
export const SAMPLE_HOUSE = "sample\\house";

function base64Of(numbers: Float32Array | Uint32Array): string {
  const bytes = new Uint8Array(numbers.buffer);
  let text = "";
  for (const byte of bytes) {
    text += String.fromCharCode(byte);
  }
  return btoa(text);
}

/** The heights of the sample's land, read once. */
function heightsOf(terrain: MapTerrain): (x: number, y: number) => number {
  const binary = atob(terrain.heights);
  const width = terrain.columns * terrain.samples;
  const values = new Float32Array(width * terrain.rows * terrain.samples);
  for (let index = 0; index < values.length; index++) {
    const value = binary.charCodeAt(index * 2) | (binary.charCodeAt(index * 2 + 1) << 8) || 1;
    values[index] = terrain.lowest + ((value - 1) / 65534) * (terrain.highest - terrain.lowest);
  }
  return (x, y) => {
    const column = Math.floor((x / CELL - terrain.west) * terrain.samples);
    const row = Math.floor((terrain.south + terrain.rows - y / CELL) * terrain.samples);
    return values[row * width + column] ?? 0;
  };
}

let heights: ((x: number, y: number) => number) | null = null;
const heightAt = (terrain: MapTerrain, x: number, y: number) => {
  heights ??= heightsOf(terrain);
  return heights(x, y);
};

/** The blocks of the made-up level of detail: four by four cells each. */
export function sampleLodTiles(): LodTile[] {
  const tiles: LodTile[] = [];
  for (const x of [4, 8]) {
    for (const y of [-4, 0, 4]) {
      tiles.push({ level: 4, x, y, terrain: true, objects: true });
    }
  }
  return tiles;
}

/** A box without a floor, as triangles. */
function box(
  positions: number[],
  indices: number[],
  centre: [number, number, number],
  half: [number, number, number],
) {
  const faces = [
    [
      [-1, -1, 1],
      [1, -1, 1],
      [1, 1, 1],
      [-1, 1, 1],
    ],
    [
      [-1, -1, -1],
      [1, -1, -1],
      [1, -1, 1],
      [-1, -1, 1],
    ],
    [
      [1, -1, -1],
      [1, 1, -1],
      [1, 1, 1],
      [1, -1, 1],
    ],
    [
      [1, 1, -1],
      [-1, 1, -1],
      [-1, 1, 1],
      [1, 1, 1],
    ],
    [
      [-1, 1, -1],
      [-1, -1, -1],
      [-1, -1, 1],
      [-1, 1, 1],
    ],
  ];
  for (const face of faces) {
    const start = positions.length / 3;
    for (const corner of face) {
      positions.push(
        ...corner.map((value, axis) => (centre[axis] ?? 0) + value * (half[axis] ?? 1)),
      );
    }
    indices.push(start, start + 1, start + 2, start, start + 2, start + 3);
  }
}

/** A block's land, or its reduced houses: a little larger than the real ones, with a chimney. */
export function sampleLodMesh(
  terrain: MapTerrain,
  x: number,
  y: number,
  objects: boolean,
): LodMesh | null {
  const positions: number[] = [];
  const indices: number[] = [];
  const uvs: number[] = [];
  if (objects) {
    for (const house of HOUSES) {
      const [hx, hy] = house.at;
      if (hx < x * CELL || hx >= (x + 4) * CELL || hy < y * CELL || hy >= (y + 4) * CELL) {
        continue;
      }
      const ground = heightAt(terrain, hx, hy);
      const [w, d, h] = house.half;
      box(positions, indices, [hx, hy, ground + h + 6], [w * 1.06, d * 1.06, h + 6]);
      box(positions, indices, [hx + w * 0.5, hy, ground + 2 * h + 40], [18, 18, 40]);
    }
    if (indices.length === 0) {
      return null;
    }
  } else {
    const steps = 16;
    for (let j = 0; j <= steps; j++) {
      for (let i = 0; i <= steps; i++) {
        const px = (x + (i / steps) * 4) * CELL;
        const py = (y + (j / steps) * 4) * CELL;
        positions.push(px, py, heightAt(terrain, Math.min(px, (x + 3.99) * CELL), py));
        uvs.push(i / steps, 1 - j / steps);
      }
    }
    for (let j = 0; j < steps; j++) {
      for (let i = 0; i < steps; i++) {
        const a = j * (steps + 1) + i;
        indices.push(a, a + 1, a + steps + 2, a, a + steps + 2, a + steps + 1);
      }
    }
  }
  return {
    shapes: [
      {
        positions: base64Of(new Float32Array(positions)),
        indices: base64Of(new Uint32Array(indices)),
        uvs: uvs.length === 0 ? "" : base64Of(new Float32Array(uvs)),
        texture: objects ? "sample/objects/houses.dds" : "sample/terrain/land.dds",
        alphaTest: false,
      },
    ],
  };
}

/** A plain picture for a texture of the made-up level of detail. */
export function sampleLodTexture(path: string): string | null {
  const canvas = document.createElement("canvas");
  canvas.width = 64;
  canvas.height = 64;
  const context = canvas.getContext("2d");
  if (context === null) {
    return null;
  }
  // The reduced houses in a warm orange, so that they cannot be taken for the real ones.
  context.fillStyle = path.includes("/objects/") ? "#d9822b" : "#6f7d4a";
  context.fillRect(0, 0, 64, 64);
  return canvas.toDataURL();
}

/** The houses placed up close within `radius` of `cx`, `cy`. */
export function sampleHouses(
  terrain: MapTerrain,
  cx: number,
  cy: number,
  radius: number,
): AreaObject[] {
  return HOUSES.filter(
    (house) =>
      house.real && Math.abs(house.at[0] - cx) <= radius && Math.abs(house.at[1] - cy) <= radius,
  ).map((house) => {
    const [hx, hy] = house.at;
    const ground = heightAt(terrain, hx, hy);
    return {
      id: { plugin: "Skyrim.esm", localId: 0xc00 + HOUSES.indexOf(house) },
      plugin: "Skyrim.esm",
      model: `${SAMPLE_HOUSE}${HOUSES.indexOf(house)}.nif`,
      name: "House",
      position: [hx, hy, ground + house.half[2]],
      rotation: [0, 0, 0],
      scale: 1,
    };
  });
}

/** Half the size of a placed house's model, by the model's name. */
export function houseBox(model: string): [number, number, number] | undefined {
  const match = /^sample\\house(\d+)\.nif$/.exec(model);
  return match === null ? undefined : HOUSES[Number(match[1])]?.half;
}
