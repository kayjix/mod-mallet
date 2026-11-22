// Where the camera of a 3D view goes to show some located objects (see
// `locate.tsx`). Pure, so that a script can test it without a view.

/** The point and the reach of a group of objects, in game units. */
export interface Framing {
  /** Where the camera looks: the middle of the objects. */
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

/**
 * How far the camera stands from located objects, in the unit of
 * `radius`: far enough to see all of them with some room around, but a
 * camera already about right is left where it is, so that the view does
 * not jump between two objects side by side. `least` is the closest the
 * camera should come, as objects have a size of their own.
 */
export function framedDistance(radius: number, current: number, least: number): number {
  const wanted = Math.max(least, radius * 2.6);
  return current >= wanted * 0.6 && current <= wanted * 1.8 ? current : wanted;
}
