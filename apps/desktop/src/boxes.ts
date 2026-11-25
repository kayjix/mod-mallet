// Boxes of models as the game places them: turned, scaled and moved, and
// whether two of them meet. Pure, so that a script can test it without a view.

/** A box turned and placed in the world: its middle, its three axes (unit length) and half sizes. */
export interface PlacedBox {
  centre: [number, number, number];
  axes: [[number, number, number], [number, number, number], [number, number, number]];
  half: [number, number, number];
}

/** A model's box as `matrix` (column by column, with scale) places it. */
export function placedBox(
  box: [[number, number, number], [number, number, number]],
  matrix: Float32Array,
): PlacedBox {
  const [low, high] = box;
  const middle = [0, 1, 2].map((axis) => ((low[axis] ?? 0) + (high[axis] ?? 0)) / 2);
  const at = (row: number) =>
    (matrix[row] ?? 0) * (middle[0] ?? 0) +
    (matrix[4 + row] ?? 0) * (middle[1] ?? 0) +
    (matrix[8 + row] ?? 0) * (middle[2] ?? 0) +
    (matrix[12 + row] ?? 0);
  const column = (index: number): [number, number, number] => [
    matrix[index * 4] ?? 0,
    matrix[index * 4 + 1] ?? 0,
    matrix[index * 4 + 2] ?? 0,
  ];
  const axes = [column(0), column(1), column(2)];
  const lengths = axes.map((axis) => Math.hypot(...axis) || 1);
  return {
    centre: [at(0), at(1), at(2)],
    axes: axes.map((axis, index) => axis.map((value) => value / (lengths[index] ?? 1))) as [
      [number, number, number],
      [number, number, number],
      [number, number, number],
    ],
    half: [0, 1, 2].map(
      (axis) => (((high[axis] ?? 0) - (low[axis] ?? 0)) / 2) * (lengths[axis] ?? 1),
    ) as [number, number, number],
  };
}

const dot = (a: readonly number[], b: readonly number[]) =>
  (a[0] ?? 0) * (b[0] ?? 0) + (a[1] ?? 0) * (b[1] ?? 0) + (a[2] ?? 0) * (b[2] ?? 0);

const cross = (a: readonly number[], b: readonly number[]): [number, number, number] => [
  (a[1] ?? 0) * (b[2] ?? 0) - (a[2] ?? 0) * (b[1] ?? 0),
  (a[2] ?? 0) * (b[0] ?? 0) - (a[0] ?? 0) * (b[2] ?? 0),
  (a[0] ?? 0) * (b[1] ?? 0) - (a[1] ?? 0) * (b[0] ?? 0),
];

/**
 * Whether two placed boxes overlap or come within `margin` of each other:
 * no axis separates them (the separating axis test, fifteen axes).
 */
export function boxesMeet(a: PlacedBox, b: PlacedBox, margin: number): boolean {
  const between = [0, 1, 2].map((axis) => (b.centre[axis] ?? 0) - (a.centre[axis] ?? 0));
  const reach = (box: PlacedBox, axis: readonly number[]) =>
    box.axes.reduce(
      (sum, own, index) => sum + Math.abs(dot(own, axis)) * (box.half[index] ?? 0),
      0,
    );
  const axes: [number, number, number][] = [...a.axes, ...b.axes];
  for (const own of a.axes) {
    for (const other of b.axes) {
      const axis = cross(own, other);
      const length = Math.hypot(...axis);
      if (length > 1e-6) {
        axes.push(axis.map((value) => value / length) as [number, number, number]);
      }
    }
  }
  return axes.every(
    (axis) => Math.abs(dot(between, axis)) <= reach(a, axis) + reach(b, axis) + margin,
  );
}

/**
 * How far two placed boxes reach into each other, in game units: the
 * least overlap along any separating axis. Zero or less where they do not
 * meet; then minus the gap between them.
 */
export function overlapDepth(a: PlacedBox, b: PlacedBox): number {
  const between = [0, 1, 2].map((axis) => (b.centre[axis] ?? 0) - (a.centre[axis] ?? 0));
  const reach = (box: PlacedBox, axis: readonly number[]) =>
    box.axes.reduce(
      (sum, own, index) => sum + Math.abs(dot(own, axis)) * (box.half[index] ?? 0),
      0,
    );
  const axes: [number, number, number][] = [...a.axes, ...b.axes];
  for (const own of a.axes) {
    for (const other of b.axes) {
      const axis = cross(own, other);
      const length = Math.hypot(...axis);
      if (length > 1e-6) {
        axes.push(axis.map((value) => value / length) as [number, number, number]);
      }
    }
  }
  return Math.min(
    ...axes.map((axis) => reach(a, axis) + reach(b, axis) - Math.abs(dot(between, axis))),
  );
}

/**
 * Whether two placed boxes meet within `margin`, exactly as
 * {@link boxesMeet} tells, after a quick test that rules out boxes too far
 * apart for it: most parts of a place are far from each other.
 *
 * Where no axis of `a` separates them, the middles are at most
 * `ra + √3·rb + √3·margin` apart (`ra`, `rb` half their diagonals): the
 * reach of `b` along three axes at right angles is at most `√3·rb` in
 * all; and the same the other way round.
 */
export function boxesTouch(a: PlacedBox, b: PlacedBox, margin: number): boolean {
  const dx = b.centre[0] - a.centre[0];
  const dy = b.centre[1] - a.centre[1];
  const dz = b.centre[2] - a.centre[2];
  const ra = outerRadius(a);
  const rb = outerRadius(b);
  // A little over √3, for rounding.
  const root3 = 1.7321;
  const reach = Math.min(ra + root3 * rb, root3 * ra + rb) + root3 * margin;
  return dx * dx + dy * dy + dz * dz <= reach * reach && boxesMeet(a, b, margin);
}

/** Half the diagonal of a placed box: no point of it is farther from its middle. */
export function outerRadius(box: PlacedBox): number {
  return Math.hypot(...box.half);
}

/** Whether the point `at` lies inside the placed box `box`. */
export function holds(box: PlacedBox, at: readonly number[]): boolean {
  const between = [0, 1, 2].map((axis) => (at[axis] ?? 0) - (box.centre[axis] ?? 0));
  return box.axes.every((axis, index) => Math.abs(dot(between, axis)) <= (box.half[index] ?? 0));
}

/**
 * The parts reached from the part at `start` link by link, at most `most`:
 * two parts link where their boxes come within `gap` game units of each
 * other. The caller passes only the parts that may link. Returns the
 * indices of the chain's parts, the start first.
 */
export function chainFrom(
  boxes: PlacedBox[],
  start: number,
  gap: number,
  most = Number.POSITIVE_INFINITY,
): number[] {
  if (boxes[start] === undefined) {
    return [];
  }
  const outer = boxes.map(outerRadius);
  const found = [start];
  const left = new Set(boxes.map((_, index) => index));
  left.delete(start);
  for (let at = 0; at < found.length && found.length < most; at += 1) {
    const from = found[at] ?? start;
    const box = boxes[from];
    if (box === undefined) {
      continue;
    }
    for (const other of [...left]) {
      const candidate = boxes[other];
      if (candidate === undefined || found.length >= most) {
        continue;
      }
      // Far apart by their spheres already: no axis needed.
      const apart = Math.hypot(
        candidate.centre[0] - box.centre[0],
        candidate.centre[1] - box.centre[1],
        candidate.centre[2] - box.centre[2],
      );
      if (apart > (outer[from] ?? 0) + (outer[other] ?? 0) + gap) {
        continue;
      }
      if (boxesMeet(box, candidate, gap)) {
        found.push(other);
        left.delete(other);
      }
    }
  }
  return found;
}

/** How far apart two points are. */
const apart = (a: readonly number[], b: readonly number[]) =>
  Math.hypot((a[0] ?? 0) - (b[0] ?? 0), (a[1] ?? 0) - (b[1] ?? 0), (a[2] ?? 0) - (b[2] ?? 0));

/** How many cells a model's box is cut into along each side, for {@link occupancy}. */
export const CELLS = 6;

/**
 * Which cells of a model's box hold some of its surfaces: the box cut
 * into {@link CELLS} along each side, one byte per cell (x first, then y,
 * then z), 1 where a triangle passes. A house's box is hollow inside, a
 * stair's box empty above its steps; the cells tell the room a model
 * really takes from the room its box takes.
 */
export function occupancy(
  box: [readonly number[], readonly number[]],
  shapes: readonly { positions: Float32Array; indices: Uint32Array }[],
): Uint8Array {
  const [low, high] = box;
  const size = [0, 1, 2].map((axis) => Math.max((high[axis] ?? 0) - (low[axis] ?? 0), 1e-3));
  const cells = new Uint8Array(CELLS * CELLS * CELLS);
  const cellOf = (value: number, axis: number) =>
    Math.min(
      CELLS - 1,
      Math.max(0, Math.floor(((value - (low[axis] ?? 0)) / (size[axis] ?? 1)) * CELLS)),
    );
  // Triangles are sampled at half a cell's length, so that none slips
  // through a cell unmarked.
  const step = Math.max(...size) / CELLS / 2;
  for (const { positions, indices } of shapes) {
    for (let at = 0; at + 2 < indices.length; at += 3) {
      const corner = (which: number) => {
        const index = (indices[at + which] ?? 0) * 3;
        return [positions[index] ?? 0, positions[index + 1] ?? 0, positions[index + 2] ?? 0];
      };
      const [a, b, c] = [corner(0), corner(1), corner(2)];
      const longest = Math.max(apart(a, b), apart(b, c), apart(c, a));
      const steps = Math.min(64, Math.max(1, Math.ceil(longest / step)));
      for (let i = 0; i <= steps; i += 1) {
        for (let j = 0; j <= steps - i; j += 1) {
          const u = i / steps;
          const v = j / steps;
          const w = 1 - u - v;
          const point = [0, 1, 2].map(
            (axis) => (a[axis] ?? 0) * w + (b[axis] ?? 0) * u + (c[axis] ?? 0) * v,
          );
          const x = cellOf(point[0] ?? 0, 0);
          const y = cellOf(point[1] ?? 0, 1);
          const z = cellOf(point[2] ?? 0, 2);
          cells[(x * CELLS + y) * CELLS + z] = 1;
        }
      }
    }
  }
  return cells;
}
