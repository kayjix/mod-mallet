// The clusters of the guided review (ADR 0050): the spots of the same two
// mods that lie close together, such as the bridges at Markarth, decided
// together, segment by segment where wanted.

import { objectKey } from "./area-layer";
import { type Decided, decidedOf as decidedTogether } from "./decisions";
import type { MapPassage, MapSpot } from "./generated/rendered-report";
import type { CollisionSpot } from "./health";
import type { WorkKind } from "./map-places";
import { type PatchChoices, spotKey } from "./patch";

/** Spots of the same two mods this close belong to one cluster, in game units (about 30 m). */
const CLUSTER_GAP = 2048;

/** The worst collisions first. */
const KIND_ORDER: WorkKind[] = ["duplicate", "door", "overlap", "deep", "touch"];

/** One spot of a cluster, with its identity among the choices. */
export interface ClusterSpot {
  item: CollisionSpot;
  key: string;
}

/** The spots of the same two mods that lie close together. */
export interface Cluster {
  /** Its identity: the key of its first spot. */
  key: string;
  /** The two mods' plugins, in the order the first spot names them. */
  plugins: [string, string];
  spots: ClusterSpot[];
  /** The worst kind of collision among its spots. */
  kind: WorkKind;
  /** The point in its middle, in game units. */
  centre: [number, number, number];
  /** How far its spots lie from the middle, in game units. */
  radius: number;
  /**
   * Where it is a passage (ADR 0072): the passage, and its parts by
   * {@link objectKey}. Its structures end where the passage ends.
   */
  passage?: { passage: MapPassage; keys: Set<string> };
}

export type { Decided };

/** Where a spot lies, to group spots by their worldspace or interior cell. */
function placeKey(item: CollisionSpot): string {
  const { location } = item.spot;
  return location.type === "interior" ? `i:${location.cell}` : `w:${location.world}`;
}

/** The two plugins of a spot, as it names them. */
function pluginsOf(item: CollisionSpot): [string, string] {
  const plugins = [...new Set((item.spot.parts ?? []).map((part) => part.object.plugin))];
  return [plugins[0] ?? "", plugins[1] ?? plugins[0] ?? ""];
}

/**
 * The clusters of `spots`: those of the same two mods in the same place
 * whose points lie within {@link CLUSTER_GAP} of one another, chained. The
 * worst first; the order does not change as spots are decided.
 */
export function clustersOf(spots: CollisionSpot[]): Cluster[] {
  const groups = new Map<string, ClusterSpot[]>();
  for (const item of spots) {
    if ((item.spot.parts ?? []).length === 0) {
      continue;
    }
    const pair = pluginsOf(item)
      .map((plugin) => plugin.toLowerCase())
      .sort()
      .join("+");
    const group = `${pair}|${placeKey(item)}`;
    groups.set(group, [
      ...(groups.get(group) ?? []),
      { item, key: spotKey(item.spot) ?? item.finding },
    ]);
  }
  const clusters: Cluster[] = [];
  for (const members of groups.values()) {
    const root = members.map((_, index) => index);
    const find = (index: number): number => {
      let at = index;
      while (root[at] !== at) {
        at = root[at] ?? at;
      }
      return at;
    };
    members.forEach((a, i) => {
      members.forEach((b, j) => {
        if (j <= i) {
          return;
        }
        const pa = a.item.spot.position;
        const pb = b.item.spot.position;
        // Spots without a point (in an interior) count as close.
        const close =
          pa == null || pb == null || Math.hypot(pa[0] - pb[0], pa[1] - pb[1]) <= CLUSTER_GAP;
        if (close) {
          root[find(j)] = find(i);
        }
      });
    });
    const joined = new Map<number, ClusterSpot[]>();
    members.forEach((member, index) => {
      const at = find(index);
      joined.set(at, [...(joined.get(at) ?? []), member]);
    });
    for (const list of joined.values()) {
      list.sort((a, b) => a.key.localeCompare(b.key));
      const first = list[0];
      if (first === undefined) {
        continue;
      }
      const points = list
        .map((member) => member.item.spot.position)
        .filter((point): point is [number, number, number] => point != null);
      const centre: [number, number, number] =
        points.length === 0
          ? [0, 0, 0]
          : ([0, 1, 2].map(
              (axis) => points.reduce((sum, point) => sum + (point[axis] ?? 0), 0) / points.length,
            ) as [number, number, number]);
      const radius = Math.max(
        0,
        ...points.map((point) => Math.hypot(point[0] - centre[0], point[1] - centre[1])),
      );
      const kind =
        KIND_ORDER.find((candidate) => list.some((member) => member.item.kind === candidate)) ??
        "touch";
      clusters.push({
        key: first.key,
        plugins: pluginsOf(first.item),
        spots: list,
        kind,
        centre: points.length === 0 ? (first.item.spot.position ?? [0, 0, 0]) : centre,
        radius,
      });
    }
  }
  return clusters.sort(
    (a, b) =>
      KIND_ORDER.indexOf(a.kind) - KIND_ORDER.indexOf(b.kind) ||
      b.spots.length - a.spots.length ||
      a.key.localeCompare(b.key),
  );
}

/**
 * What is decided for `spots`, all together; where they are a passage's,
 * a part of it shown again or hidden by hand is an exception (ADR 0081).
 */
export function decidedOf(
  spots: ClusterSpot[],
  choices: PatchChoices,
  passage?: MapPassage,
): Decided {
  return decidedTogether(
    spots.map(({ item }) => ({ spot: item.spot, finding: item.finding })),
    choices,
    passage,
  );
}

/** How many spots of a cluster are still open. */
export function openIn(cluster: Cluster): number {
  return cluster.spots.filter(({ item }) => item.state === "open").length;
}

/** The parts of a passage by {@link objectKey}, both sides. */
export function passageKeys(passage: MapPassage): Set<string> {
  return new Set(
    passage.parts.flat().map((part) => objectKey(part.object.plugin, part.object.localId)),
  );
}

/** A passage's identity: its two plugins and its first part, none of which depend on the load order. */
export function passageKey(passage: MapPassage): string {
  const first = passage.parts.flat()[0]?.object;
  return `passage|${passage.plugins.map((plugin) => plugin.toLowerCase()).join("+")}|${
    first === undefined ? passage.console : objectKey(first.plugin, first.localId)
  }`;
}

/**
 * The spots of `spots` in each passage of `passages` (ADR 0072): those
 * whose parts all belong to it. A spot lies in one passage at most.
 */
export function spotsInPassages<T extends { spot: MapSpot }>(
  spots: T[],
  passages: MapPassage[],
): Map<string, T[]> {
  const owner = new Map<string, string>();
  for (const passage of passages) {
    const key = passageKey(passage);
    for (const part of passage.parts.flat()) {
      owner.set(objectKey(part.object.plugin, part.object.localId), key);
    }
  }
  const found = new Map<string, T[]>();
  for (const item of spots) {
    const parts = item.spot.parts ?? [];
    const keys = new Set(
      parts.map((part) => owner.get(objectKey(part.object.plugin, part.object.localId))),
    );
    const [only] = [...keys];
    if (parts.length === 0 || keys.size !== 1 || only === undefined) {
      continue;
    }
    found.set(only, [...(found.get(only) ?? []), item]);
  }
  return found;
}

/**
 * The places to decide (ADR 0050, 0072): first the passages, where two
 * mods build the same thing twice, the most serious first, each decided
 * as a whole; then the remaining spots by {@link clustersOf}. A passage
 * none of whose spots is left (all set aside) is left out.
 */
export function collisionClusters(spots: CollisionSpot[], passages: MapPassage[]): Cluster[] {
  const inPassages = spotsInPassages(spots, passages);
  const taken = new Set<CollisionSpot>([...inPassages.values()].flat());
  const first: Cluster[] = passages.flatMap((passage) => {
    const key = passageKey(passage);
    const members = (inPassages.get(key) ?? [])
      .map((item) => ({ item, key: spotKey(item.spot) ?? item.finding }))
      .sort((a, b) => a.key.localeCompare(b.key));
    if (members.length === 0) {
      return [];
    }
    const kind =
      KIND_ORDER.find((candidate) => members.some((member) => member.item.kind === candidate)) ??
      "touch";
    const [low, high] = [passage.min, passage.max];
    return [
      {
        key,
        plugins: passage.plugins,
        spots: members,
        kind,
        centre: passage.centre,
        radius: Math.hypot((high[0] - low[0]) / 2, (high[1] - low[1]) / 2),
        passage: { passage, keys: passageKeys(passage) },
      },
    ];
  });
  return [...first, ...clustersOf(spots.filter((item) => !taken.has(item)))];
}
