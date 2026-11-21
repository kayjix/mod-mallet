// Moving placed objects a little instead of hiding them (ADR 0060).
//
// A move is kept among the patch's choices under its own key prefix,
// `move:<plugin>|<localId>`, its value a small JSON text: how far the
// object goes along each axis and how far it turns about the upright axis,
// with where it stood, so the plugin can find it. The patch of Base Object
// Swapper reads none of these keys; pins are untouched by them.
//
// The plugin of moves is written from them on the patch page. In the 3D
// views every move is drawn where it puts the object.

import type { AreaObject } from "./generated/area-objects";
import type { ObjectMove, ObjectRef } from "./generated/rendered-report";
import { matchKey, type PatchChoices } from "./patch";

/** The prefix of a move among the choices. */
const MOVE = "move:";

/** One object's move, as kept among the choices. */
export interface Move {
  /** How far it goes east, north and up, in whole game units (about 1.4 cm each). */
  dx: number;
  dy: number;
  dz: number;
  /** How far it turns about the upright axis, in whole degrees, clockwise seen from above. */
  rz: number;
  /** Where it stood, in whole game units. */
  at: [number, number, number];
  /** The interior cell it stands in, by its editor ID; null outside. */
  cell: string | null;
}

/** About how many centimetres one game unit is. */
export const CM_PER_UNIT = 1.428;

/** A move's key among the choices. */
export const moveKey = (object: ObjectRef) => `${MOVE}${object.plugin}|${object.localId}`;

/** Whether a choice key is a move's. */
export const isMoveKey = (key: string) => key.startsWith(MOVE);

function parseMove(value: string | null): Move | null {
  if (value === null) {
    return null;
  }
  try {
    const raw = JSON.parse(value) as Partial<Move>;
    const number = (field: unknown) =>
      typeof field === "number" && Number.isFinite(field) ? Math.round(field) : 0;
    const at = Array.isArray(raw.at) ? raw.at : [];
    return {
      dx: number(raw.dx),
      dy: number(raw.dy),
      dz: number(raw.dz),
      rz: number(raw.rz),
      at: [number(at[0]), number(at[1]), number(at[2])],
      cell: typeof raw.cell === "string" ? raw.cell : null,
    };
  } catch {
    return null;
  }
}

/** Whether a move changes nothing. */
export const isStill = (move: Move) =>
  move.dx === 0 && move.dy === 0 && move.dz === 0 && move.rz % 360 === 0;

/** Every move among the choices, by the moved object's {@link matchKey}. */
const cache = new WeakMap<PatchChoices, Map<string, { object: ObjectRef; move: Move }>>();

export function movesOf(choices: PatchChoices): Map<string, { object: ObjectRef; move: Move }> {
  const known = cache.get(choices);
  if (known !== undefined) {
    return known;
  }
  const found = new Map<string, { object: ObjectRef; move: Move }>();
  for (const [key, value] of Object.entries(choices)) {
    if (!isMoveKey(key)) {
      continue;
    }
    const bar = key.lastIndexOf("|");
    const localId = Number(key.slice(bar + 1));
    const move = parseMove(value);
    if (bar <= MOVE.length || !Number.isInteger(localId) || move === null || isStill(move)) {
      continue;
    }
    const object = { plugin: key.slice(MOVE.length, bar), localId };
    found.set(matchKey(object), { object, move });
  }
  cache.set(choices, found);
  return found;
}

/** Every move among the choices, by the moved object's key in the 3D views. */
export function moveKeysOf(choices: PatchChoices): Map<string, Move> {
  return new Map([...movesOf(choices)].map(([key, { move }]) => [key, move]));
}

/** The move of one object, if it has one. */
export function moveOf(choices: PatchChoices, object: ObjectRef): Move | null {
  return movesOf(choices).get(matchKey(object))?.move ?? null;
}

/** The choices with the moves of `changes` set; a still move or null puts the object back. */
export function withMoves(
  choices: PatchChoices,
  changes: { object: ObjectRef; move: Move | null }[],
): PatchChoices {
  const next = { ...choices };
  for (const { object, move } of changes) {
    // Any spelling of the plugin goes.
    for (const key of Object.keys(next)) {
      if (isMoveKey(key) && key.toLowerCase() === moveKey(object).toLowerCase()) {
        delete next[key];
      }
    }
    if (move !== null && !isStill(move)) {
      next[moveKey(object)] = JSON.stringify(move);
    }
  }
  return next;
}

/** Only the moves of the choices, to keep them when the hiding decisions are replaced. */
export function movesOnly(choices: PatchChoices): PatchChoices {
  return Object.fromEntries(Object.entries(choices).filter(([key]) => isMoveKey(key)));
}

/** The moves as the plugin of moves takes them, in a fixed order. */
export function objectMovesOf(choices: PatchChoices): ObjectMove[] {
  return [...movesOf(choices).values()]
    .sort((a, b) => matchKey(a.object).localeCompare(matchKey(b.object)))
    .map(({ object, move }) => ({
      object,
      cell: move.cell,
      from: move.at,
      offset: [move.dx, move.dy, move.dz],
      turn: move.rz,
    }));
}

/** A text that is the same for the same moves, to tell whether a written plugin is up to date. */
export function movesSignature(choices: PatchChoices): string {
  return JSON.stringify(objectMovesOf(choices));
}

/** A point turned about the upright axis through `pivot` by `degrees`, the way the game turns. */
function turnedAbout(
  point: [number, number],
  pivot: [number, number],
  degrees: number,
): [number, number] {
  // The game turns clockwise seen from above.
  const angle = (-degrees * Math.PI) / 180;
  const [c, s] = [Math.cos(angle), Math.sin(angle)];
  const [x, y] = [point[0] - pivot[0], point[1] - pivot[1]];
  return [pivot[0] + c * x - s * y, pivot[1] + s * x + c * y];
}

/**
 * The moves of a group of objects moved as one: shifted by `offset` and
 * turned by `degrees` about the upright axis through `pivot`. Each object
 * turns by the same angle and goes round the pivot.
 */
export function groupMoves(
  objects: AreaObject[],
  pivot: [number, number],
  offset: [number, number, number],
  degrees: number,
  cell: string | null,
  base: (object: AreaObject) => Move | null,
): { object: ObjectRef; move: Move }[] {
  return objects.map((object) => {
    const before = base(object);
    const at: [number, number, number] = before?.at ?? [
      Math.round(object.position[0]),
      Math.round(object.position[1]),
      Math.round(object.position[2]),
    ];
    // Where the object stands with its move so far, then moved further.
    const [x, y] = [object.position[0] + (before?.dx ?? 0), object.position[1] + (before?.dy ?? 0)];
    const [tx, ty] = turnedAbout([x, y], pivot, degrees);
    return {
      object: object.id,
      move: {
        dx: Math.round(tx + offset[0] - object.position[0]),
        dy: Math.round(ty + offset[1] - object.position[1]),
        dz: (before?.dz ?? 0) + offset[2],
        rz: ((((before?.rz ?? 0) + degrees) % 360) + 360) % 360,
        at,
        cell: before?.cell ?? cell,
      },
    };
  });
}

/**
 * Where an object stands with its move, column by column as `placement`
 * gives it: turned about the world's upright axis through its own origin,
 * then shifted.
 */
export function movedMatrix(matrix: Float32Array, move: Move | null): Float32Array {
  if (move === null || isStill(move)) {
    return matrix;
  }
  const angle = (-move.rz * Math.PI) / 180;
  const [c, s] = [Math.cos(angle), Math.sin(angle)];
  const moved = new Float32Array(matrix);
  for (let column = 0; column < 3; column += 1) {
    const x = matrix[column * 4] ?? 0;
    const y = matrix[column * 4 + 1] ?? 0;
    moved[column * 4] = c * x - s * y;
    moved[column * 4 + 1] = s * x + c * y;
  }
  moved[12] = (matrix[12] ?? 0) + move.dx;
  moved[13] = (matrix[13] ?? 0) + move.dy;
  moved[14] = (matrix[14] ?? 0) + move.dz;
  return moved;
}
