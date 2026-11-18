// The segments of a place (ADR 0050): the two mods' structures around the
// spots of a place, joined where they share a part, with their parts and
// how much room they take. Shared by the three-way stage and by the check
// of every place in the background.

import { type AreaLayer, objectKey } from "./area-layer";
import type { AreaObject, AreaObjects, ObjectRef } from "./generated/area-objects";

/** One spot of a place, with the parts its structures start from. */
export interface StageGroup {
  key: string;
  seeds: [ObjectRef[], ObjectRef[]];
}

/**
 * A segment: the structures of the two mods that hang together around one
 * or more spots, and those spots (ADR 0050).
 */
export interface StageSegment {
  /** The spots, by their groups' keys. */
  groups: string[];
  parts: [AreaObject[], AreaObject[]];
  /** How much room each side's parts take: their model boxes as placed, in cubic game units. */
  volumes: [number, number];
}

/** How close two parts must come to count as one structure, in game units (about 6 cm). */
export const STRUCTURE_MARGIN = 4;

/**
 * How far from the spot a structure reaches, in game units (about 30 m):
 * the walls of a whole town touch each other, but the user decides the
 * gate, not the town.
 */
export const STRUCTURE_REACH = 2048;

/** The objects of `area` that a structure starts from. */
function seedObjects(area: AreaObjects, seeds: ObjectRef[]): AreaObject[] {
  const wanted = new Set(seeds.map((seed) => objectKey(seed.plugin, seed.localId)));
  return area.objects.filter((object) =>
    wanted.has(objectKey(object.id.plugin, object.id.localId)),
  );
}

/**
 * The segments of the spots `groups` in the area `layer` shows: the
 * structures grown from each spot's parts within `reach` of `centre`,
 * joined where they share a part.
 */
export function findSegments(
  layer: AreaLayer,
  area: AreaObjects,
  groups: StageGroup[],
  centre: readonly number[],
  reach: number,
): StageSegment[] {
  const near = { x: centre[0] ?? 0, y: centre[1] ?? 0, radius: reach };
  // What grows from each seed, once per seed.
  const grown = new Map<string, AreaObject[]>();
  const growFrom = (seed: AreaObject) => {
    const key = objectKey(seed.id.plugin, seed.id.localId);
    let parts = grown.get(key);
    if (parts === undefined) {
      parts = layer.connected(seed, STRUCTURE_MARGIN, near);
      for (const part of parts) {
        // A part reached from another seed of the same kind grows the same.
        grown.set(objectKey(part.id.plugin, part.id.localId), parts);
      }
      grown.set(key, parts);
    }
    return parts;
  };
  const grow = (from: AreaObject[]) => {
    const found = new Map<string, AreaObject>();
    for (const seed of from) {
      for (const part of growFrom(seed)) {
        found.set(objectKey(part.id.plugin, part.id.localId), part);
      }
    }
    return [...found.values()];
  };
  const perGroup = groups.map((group) => ({
    key: group.key,
    parts: [grow(seedObjects(area, group.seeds[0])), grow(seedObjects(area, group.seeds[1]))] as [
      AreaObject[],
      AreaObject[],
    ],
  }));
  return segmentsOf(perGroup).map((segment) => ({
    ...segment,
    volumes: [volume(layer, segment.parts[0]), volume(layer, segment.parts[1])],
  }));
}

/** How much room `parts` take together, by their model boxes as placed. */
function volume(layer: AreaLayer, parts: AreaObject[]): number {
  return parts.reduce((sum, part) => sum + (layer.volumeOf(part) ?? 0), 0);
}

/**
 * The spots' structures joined where they share a part: each join is one
 * segment, with the spots it covers and the parts of both mods.
 */
function segmentsOf(
  perGroup: { key: string; parts: [AreaObject[], AreaObject[]] }[],
): Omit<StageSegment, "volumes">[] {
  const owner = new Map<string, number>();
  const root = perGroup.map((_, index) => index);
  const find = (index: number): number => {
    let at = index;
    while (root[at] !== at) {
      at = root[at] ?? at;
    }
    return at;
  };
  perGroup.forEach((group, index) => {
    for (const part of [...group.parts[0], ...group.parts[1]]) {
      const key = objectKey(part.id.plugin, part.id.localId);
      const before = owner.get(key);
      if (before === undefined) {
        owner.set(key, index);
      } else {
        root[find(index)] = find(before);
      }
    }
  });
  const joined = new Map<number, Omit<StageSegment, "volumes">>();
  perGroup.forEach((group, index) => {
    const at = find(index);
    const segment = joined.get(at) ?? { groups: [], parts: [[], []] };
    segment.groups.push(group.key);
    for (const side of [0, 1] as const) {
      const known = new Set(
        segment.parts[side].map((part) => objectKey(part.id.plugin, part.id.localId)),
      );
      for (const part of group.parts[side]) {
        if (!known.has(objectKey(part.id.plugin, part.id.localId))) {
          segment.parts[side].push(part);
        }
      }
    }
    joined.set(at, segment);
  });
  return [...joined.values()];
}
