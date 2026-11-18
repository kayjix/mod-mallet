// The suggested side of a place or segment (ADR 0050), by size alone:
// where one mod builds a house and the other has a lantern post behind it,
// keep the house. Apart from LOOT's word and the suggestions about whole
// mods; this one only measures.

import type { StageSegment } from "./segments";

/** From this ratio of room taken, the bigger side is clearly the one to keep. */
export const VOLUME_RATIO = 20;

/** From this ratio of parts, with at least {@link SUPPORT_RATIO} the room, the side with more parts is. */
export const PART_RATIO = 20;
export const SUPPORT_RATIO = 5;

/** Game units in a metre, for room in cubic metres. */
const UNITS_PER_METRE = 70;

/** The side to keep, and what the two sides bring. */
export interface Suggestion {
  side: 0 | 1;
  parts: [number, number];
  /** Room taken, in cubic metres. */
  volumes: [number, number];
  /** The names of the parts of the side to give up, if it has a single one. */
  single: string | null;
}

/**
 * The side clearly to keep in `segments`, if one is: at least
 * {@link VOLUME_RATIO} times the room of the other, however many parts
 * either has (one huge bridge against a fence of thirty posts keeps the
 * bridge); or at least {@link PART_RATIO} times the parts with at least
 * {@link SUPPORT_RATIO} times the room. Otherwise none: the place is for
 * the user to look at.
 */
export function suggestionOf(segments: StageSegment[]): Suggestion | null {
  const parts: [number, number] = [0, 1].map((side) =>
    segments.reduce((sum, segment) => sum + (segment.parts[side]?.length ?? 0), 0),
  ) as [number, number];
  const volumes: [number, number] = [0, 1].map((side) =>
    segments.reduce((sum, segment) => sum + (segment.volumes[side] ?? 0), 0),
  ) as [number, number];
  if (parts[0] === 0 || parts[1] === 0) {
    return null;
  }
  for (const side of [0, 1] as const) {
    const other = side === 0 ? 1 : 0;
    const byRoom = volumes[side] >= VOLUME_RATIO * volumes[other];
    const byParts =
      parts[side] >= PART_RATIO * parts[other] && volumes[side] >= SUPPORT_RATIO * volumes[other];
    if (byRoom || byParts) {
      const losing = segments.flatMap((segment) => segment.parts[other]);
      const metres = UNITS_PER_METRE ** 3;
      return {
        side,
        parts,
        volumes: [volumes[0] / metres, volumes[1] / metres],
        single:
          losing.length === 1
            ? (losing[0]?.name ?? losing[0]?.model.split(/[\\/]/).pop() ?? null)
            : null,
      };
    }
  }
  return null;
}
