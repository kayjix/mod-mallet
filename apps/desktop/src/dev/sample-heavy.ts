// A heavy place for the sample, shown with `?heavy` in the address: about
// four hundred parts of two mods built into each other around every spot,
// as at a big temple of a real setup, among a few thousand objects of the
// game with hundreds of models, each of several textured surfaces. It is
// there to measure how fast the views draw (see `render-stats.ts`).

import type { AreaObject } from "../generated/area-objects";
import type { LodMesh } from "../generated/lod-mesh";

/** Whether the sample places its heavy scene around every spot. */
export const SAMPLE_HEAVY =
  typeof window !== "undefined" && new URLSearchParams(window.location.search).has("heavy");

/** How many models the game's objects use, and how many objects there are. */
const CLUTTER_MODELS = 400;
const CLUTTER = 2600;
/** How many textures the models share. */
const TEXTURES = 80;
/** The structures' tiles: columns and rows of each mod's floor. */
const COLUMNS = 10;
const ROWS = 19;

const FLOOR = "heavy\\temple\\floor";
const WALL = "heavy\\temple\\wall";
const THING = "heavy\\clutter\\thing";

/** A small random generator that gives the same numbers every time. */
function random(seed: number): () => number {
  let state = seed >>> 0 || 1;
  return () => {
    state ^= state << 13;
    state ^= state >>> 17;
    state ^= state << 5;
    return (state >>> 0) / 4294967296;
  };
}

/**
 * The heavy scene around a spot at `cx`, `cy` whose ground is at `ground`:
 * a floor of Old Town touching its bridge, a floor of Texture Pack laid
 * into it, and the game's clutter all around.
 */
export function heavyObjects(cx: number, cy: number, ground: number): AreaObject[] {
  const objects: AreaObject[] = [];
  const place = (
    plugin: string,
    localId: number,
    model: string,
    position: [number, number, number],
    turn: number,
    scale = 1,
  ) =>
    objects.push({
      id: { plugin, localId },
      plugin,
      model,
      name: null,
      position,
      rotation: [0, 0, turn],
      scale,
    });
  for (let column = 0; column < COLUMNS; column++) {
    for (let row = 0; row < ROWS; row++) {
      const x = cx - 342 + column * 76;
      const y = cy + 52 + row * 76;
      const index = column * ROWS + row;
      place("Old Town.esp", 0x9000 + index, `${FLOOR}${index % 12}.nif`, [x, y, ground + 60], 0);
      place(
        "Texture Pack.esp",
        0x9000 + index,
        `${WALL}${index % 15}.nif`,
        [x + 20, y + 20, ground + 63],
        0,
      );
    }
  }
  const next = random(Math.round(cx) * 31 + Math.round(cy));
  for (let index = 0; index < CLUTTER; index++) {
    const angle = next() * Math.PI * 2;
    const away = 300 + next() * 3400;
    // Some models are used often, most a few times.
    const model = Math.floor(next() ** 2 * CLUTTER_MODELS);
    place(
      "Skyrim.esm",
      0x40000 + index,
      `${THING}${model}.nif`,
      [cx + Math.cos(angle) * away, cy + Math.sin(angle) * away, ground + next() * 200],
      next() * Math.PI * 2,
      0.6 + next(),
    );
  }
  return objects;
}

/** The half sizes of a heavy model, or none for another model. */
function halfOf(name: string): [number, number, number] | undefined {
  if (name.startsWith(FLOOR)) {
    return [38, 38, 4];
  }
  if (name.startsWith(WALL)) {
    return [38, 38, 4];
  }
  if (name.startsWith(THING)) {
    const number = Number.parseInt(name.slice(THING.length), 10) || 0;
    const next = random(number + 7);
    return [6 + next() * 60, 6 + next() * 60, 6 + next() * 60];
  }
  return undefined;
}

function base64Of(numbers: Float32Array | Uint32Array): string {
  const bytes = new Uint8Array(numbers.buffer);
  let text = "";
  for (const byte of bytes) {
    text += String.fromCharCode(byte);
  }
  return btoa(text);
}

/** How finely each face of a heavy model is cut, in squares per side. */
const CUTS = 6;

/**
 * A heavy model: a box whose faces are cut finely, in three surfaces of
 * two faces each, each with its own texture; the last one alpha tested.
 */
export function heavyMesh(name: string): LodMesh | null {
  const half = halfOf(name);
  if (half === undefined) {
    return null;
  }
  const number = Number.parseInt(name.replace(/\D+/g, ""), 10) || 0;
  // The six faces: the axis they face along and its sign.
  const faces: [number, number][] = [
    [2, -1],
    [2, 1],
    [0, -1],
    [0, 1],
    [1, -1],
    [1, 1],
  ];
  const shapes = [0, 1, 2].map((part) => {
    const positions: number[] = [];
    const uvs: number[] = [];
    const indices: number[] = [];
    for (const [axis, sign] of faces.slice(part * 2, part * 2 + 2)) {
      const u = (axis + 1) % 3;
      const v = (axis + 2) % 3;
      const start = positions.length / 3;
      for (let row = 0; row <= CUTS; row++) {
        for (let column = 0; column <= CUTS; column++) {
          const point = [0, 0, 0];
          point[axis] = sign * (half[axis] ?? 1);
          point[u] = ((column / CUTS) * 2 - 1) * (half[u] ?? 1);
          point[v] = ((row / CUTS) * 2 - 1) * (half[v] ?? 1);
          positions.push(...point);
          uvs.push(column / CUTS, row / CUTS);
        }
      }
      for (let row = 0; row < CUTS; row++) {
        for (let column = 0; column < CUTS; column++) {
          const a = start + row * (CUTS + 1) + column;
          const b = a + 1;
          const c = a + CUTS + 1;
          const d = c + 1;
          indices.push(a, b, d, a, d, c);
        }
      }
    }
    return {
      positions: base64Of(new Float32Array(positions)),
      indices: base64Of(new Uint32Array(indices)),
      uvs: base64Of(new Float32Array(uvs)),
      texture: `heavy\\textures\\t${(number * 3 + part) % TEXTURES}.dds`,
      alphaTest: part === 2,
    };
  });
  return { shapes };
}

const textures = new Map<string, string>();

/** A heavy model's texture: a checkered square in a colour of its own, as a data URL. */
export function heavyTexture(path: string): string | null {
  if (!path.startsWith("heavy\\")) {
    return null;
  }
  const known = textures.get(path);
  if (known !== undefined) {
    return known;
  }
  const canvas = document.createElement("canvas");
  canvas.width = 128;
  canvas.height = 128;
  const context = canvas.getContext("2d");
  if (context === null) {
    return null;
  }
  const number = Number.parseInt(path.replace(/\D+/g, ""), 10) || 0;
  context.fillStyle = `hsl(${(number * 47) % 360} 45% 55%)`;
  context.fillRect(0, 0, 128, 128);
  context.fillStyle = `hsl(${(number * 47 + 30) % 360} 40% 35%)`;
  for (let row = 0; row < 8; row++) {
    for (let column = (row % 2) * 1; column < 8; column += 2) {
      context.fillRect(column * 16, row * 16, 16, 16);
    }
  }
  const url = canvas.toDataURL("image/png");
  textures.set(path, url);
  return url;
}
