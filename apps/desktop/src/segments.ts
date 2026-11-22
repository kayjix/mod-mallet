// The segments of a place (ADR 0050): the two mods' structures around the
// spots of a place, joined where they share a part, with their parts and
// how much room they take. Shared by the three-way stage and by the check
// of every place in the background.

import { type AreaLayer, boxesTouch, objectKey } from "./area-layer";
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
  /** The parts that are decor resting on others (clutter, small plants), by key. */
  decor: string[];
}

/** How close two parts must come to count as one structure, in game units (about 6 cm). */
export const STRUCTURE_MARGIN = 4;

/**
 * How far from the spot a structure reaches, in game units (about 30 m):
 * the walls of a whole town touch each other, but the user decides the
 * gate, not the town.
 */
export const STRUCTURE_REACH = 2048;

/** The start of the key of a group that no spot seeds, only a passage's part (ADR 0072). */
const UNSEEDED = "unseeded:";

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
 * joined where they share a part. Within a passage (ADR 0072), `within`
 * holds its parts by {@link objectKey}: the structures keep only those,
 * and the decor resting on them.
 */
export function findSegments(
  layer: AreaLayer,
  area: AreaObjects,
  groups: StageGroup[],
  centre: readonly number[],
  reach: number,
  within: Set<string> | null = null,
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
  // The same structure is grown from many spots; what of it lies in the
  // passage is worked out once for each list of parts.
  const inside = new Map<string, AreaObject[]>();
  const bounded = (parts: AreaObject[]) => {
    if (within === null) {
      return parts;
    }
    const key = parts.map((part) => objectKey(part.id.plugin, part.id.localId)).join(",");
    let kept = inside.get(key);
    if (kept === undefined) {
      kept = inPassage(layer, parts, within);
      inside.set(key, kept);
    }
    return kept;
  };
  const perGroup = groups.map((group) => ({
    key: group.key,
    parts: [
      bounded(grow(seedObjects(area, group.seeds[0]))),
      bounded(grow(seedObjects(area, group.seeds[1]))),
    ] as [AreaObject[], AreaObject[]],
  }));
  if (within !== null) {
    // The passage's parts that no spot's structure reached (a pile of
    // ingots on a forge) grow on their own and join the segment they
    // share a part with, or make one of their own.
    const sides = [0, 1].map(
      (side) =>
        new Set(
          groups
            .flatMap((group) => group.seeds[side] ?? [])
            .map((seed) => seed.plugin.toLowerCase()),
        ),
    );
    const covered = new Set(
      perGroup.flatMap((group) =>
        [...group.parts[0], ...group.parts[1]].map((part) =>
          objectKey(part.id.plugin, part.id.localId),
        ),
      ),
    );
    for (const object of area.objects) {
      const key = objectKey(object.id.plugin, object.id.localId);
      if (covered.has(key) || !isPassagePart(object, within)) {
        continue;
      }
      const parts = bounded(growFrom(object));
      for (const part of parts) {
        covered.add(objectKey(part.id.plugin, part.id.localId));
      }
      const first =
        sides[0]?.has(object.id.plugin.toLowerCase()) === true ||
        sides[0]?.has(object.plugin.toLowerCase()) === true;
      perGroup.push({
        key: `${UNSEEDED}${key}`,
        parts: first ? [parts, []] : [[], parts],
      });
    }
  }
  // What of the passage hangs on no spot's structure is one segment, not
  // one per loose part.
  const joined = segmentsOf(perGroup).map((segment) => ({
    ...segment,
    groups: segment.groups.filter((key) => !key.startsWith(UNSEEDED)),
  }));
  const seeded = joined.filter((segment) => segment.groups.length > 0);
  const loose = joined.filter((segment) => segment.groups.length === 0);
  const rest =
    loose.length === 0
      ? []
      : [
          {
            groups: [],
            parts: [
              loose.flatMap((segment) => segment.parts[0]),
              loose.flatMap((segment) => segment.parts[1]),
            ] as [AreaObject[], AreaObject[]],
          },
        ];
  return [...seeded, ...rest].map((segment) => ({
    ...segment,
    volumes: [volume(layer, segment.parts[0]), volume(layer, segment.parts[1])],
    decor: [...segment.parts[0], ...segment.parts[1]]
      .filter((part) => layer.isDecorPart(part) === true)
      .map((part) => objectKey(part.id.plugin, part.id.localId)),
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
): Omit<StageSegment, "volumes" | "decor">[] {
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
  const joined = new Map<number, Omit<StageSegment, "volumes" | "decor">>();
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

/** Whether `object` is one of the passage's parts `within`, under either plugin it goes by. */
export function isPassagePart(object: AreaObject, within: Set<string>): boolean {
  return (
    within.has(objectKey(object.id.plugin, object.id.localId)) ||
    within.has(objectKey(object.plugin, object.id.localId))
  );
}

/**
 * Of a structure's `parts`, those of the passage `within` and the decor
 * resting on them (ADR 0072): a passage hides the other side's version of
 * what both build, and nothing beyond it.
 */
function inPassage(layer: AreaLayer, parts: AreaObject[], within: Set<string>): AreaObject[] {
  const kept = parts.filter((part) => isPassagePart(part, within));
  const boxes = kept.flatMap((part) => {
    const box = layer.boxOf(part, null);
    return box === null ? [] : [box];
  });
  const resting = parts.filter((part) => {
    if (isPassagePart(part, within) || layer.isDecorPart(part) !== true) {
      return false;
    }
    const box = layer.boxOf(part, null);
    return box !== null && boxes.some((other) => boxesTouch(other, box, STRUCTURE_MARGIN));
  });
  return [...kept, ...resting];
}
