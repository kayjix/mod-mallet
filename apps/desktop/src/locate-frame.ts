// Where located objects are on the screen of a 3D view (see `locate.tsx`).
// The camera stays where the user put it; objects out of sight get an
// arrow at the edge of the view that points to them. Pure, so that a
// script can test it without a view.

/** How long located objects glow, and an arrow points to them, in milliseconds. */
export const LOCATE_MS = 3000;

/** The point and the reach of a group of objects, in game units. */
export interface Framing {
  /** The middle of the objects. */
  target: [number, number, number];
  /** How far the objects reach from it, sideways and up. */
  radius: number;
}

/** The middle and the reach of objects standing at `positions`; null for none. */
export function framing(positions: readonly (readonly number[])[]): Framing | null {
  if (positions.length === 0) {
    return null;
  }
  const mean = (axis: number) =>
    positions.reduce((sum, position) => sum + (position[axis] ?? 0), 0) / positions.length;
  const target: [number, number, number] = [mean(0), mean(1), mean(2)];
  const radius = Math.max(
    ...positions.map((position) =>
      Math.hypot(
        (position[0] ?? 0) - target[0],
        (position[1] ?? 0) - target[1],
        (position[2] ?? 0) - target[2],
      ),
    ),
  );
  return { target, radius };
}

/** An arrow at the edge of a view: where it sits, in pixels, and where it points. */
export interface EdgePointer {
  x: number;
  y: number;
  /** The direction it points, in radians: 0 to the right, π/2 down. */
  angle: number;
}

/**
 * The arrow at the edge of a `width` by `height` view that points to a
 * `point` out of sight, `margin` pixels inside the edge; null where the
 * point is in sight. `matrix` is the view's column-major matrix from the
 * point's space to the screen. A point behind the camera is pointed to
 * on the side it lies.
 */
export function edgePointer(
  matrix: ArrayLike<number>,
  point: readonly [number, number, number],
  width: number,
  height: number,
  margin = 28,
): EdgePointer | null {
  const [px, py, pz] = point;
  const m = (index: number) => matrix[index] ?? 0;
  const x = m(0) * px + m(4) * py + m(8) * pz + m(12);
  const y = m(1) * px + m(5) * py + m(9) * pz + m(13);
  const w = m(3) * px + m(7) * py + m(11) * pz + m(15);
  const size = Math.max(Math.abs(w), 1e-6);
  const nx = x / size;
  const ny = y / size;
  if (w > 1e-6 && Math.abs(nx) <= 1 && Math.abs(ny) <= 1) {
    return null;
  }
  // Behind the camera and straight ahead or behind: down, where one turns.
  const dx = Math.abs(nx) + Math.abs(ny) < 1e-9 ? 0 : nx * (width / 2);
  const dy = Math.abs(nx) + Math.abs(ny) < 1e-9 ? height / 2 : (0 - ny) * (height / 2);
  const halfWidth = Math.max(1, width / 2 - margin);
  const halfHeight = Math.max(1, height / 2 - margin);
  const scale = Math.min(
    dx === 0 ? Number.POSITIVE_INFINITY : halfWidth / Math.abs(dx),
    dy === 0 ? Number.POSITIVE_INFINITY : halfHeight / Math.abs(dy),
  );
  return {
    x: width / 2 + dx * scale,
    y: height / 2 + dy * scale,
    angle: Math.atan2(dy, dx),
  };
}

/** Draws an `EdgePointer` on a 2D canvas: a filled arrow with a dark rim, in `colour`. */
export function paintEdgePointer(
  context: CanvasRenderingContext2D,
  pointer: EdgePointer,
  colour: string,
) {
  context.save();
  context.translate(pointer.x, pointer.y);
  context.rotate(pointer.angle);
  context.beginPath();
  context.moveTo(16, 0);
  context.lineTo(-8, -12);
  context.lineTo(-3, 0);
  context.lineTo(-8, 12);
  context.closePath();
  context.fillStyle = colour;
  context.strokeStyle = "rgb(0 0 0 / 70%)";
  context.lineWidth = 2;
  context.fill();
  context.stroke();
  context.restore();
}
