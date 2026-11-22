// What deciding a whole passage (ADR 0072) does to the patch's choices:
// one side's version kept, or both, and the kept side's parts hidden by
// hand. Pure, so that a script can test it without a window.

import { isGamePlugin } from "./game-plugins.ts";
import type { AreaObjects } from "./generated/area-objects";
import type { MapPassage, MapSpot, ObjectRef } from "./generated/rendered-report";
import {
  chooseObject,
  chooseSpot,
  freeObjects,
  matchKey,
  type PatchChoices,
  revealObjects,
} from "./patch.ts";
import type { StageSegment } from "./segments";

/** What deciding a passage needs of it: its spots' keys, its two plugins, and the passage. */
export interface PassageToDecide {
  plugins: [string, string];
  spots: { key: string }[];
  passage?: { passage: MapPassage };
}

/** One side's parts in the passage: its sections' parts once measured, else the scan's. */
export function sideParts(
  passage: MapPassage,
  segments: StageSegment[] | null,
  side: 0 | 1,
): ObjectRef[] {
  const measured = (segments ?? []).flatMap((segment) => segment.parts[side]);
  return measured.length > 0
    ? measured.map((object) => object.id)
    : passage.parts[side].map((part) => part.object);
}

/**
 * The choices with one side's version of the passage kept (`side`), or
 * both (`null`): the other side's parts hidden where the patch may hide
 * them, every spot of the passage decided alike, and the kept parts shown
 * again however they were hidden. `held` are parts the game ties in (ADR
 * 0046), by {@link matchKey}: they stay, to be decided one by one.
 */
export function keepPassage(
  choices: PatchChoices,
  spots: Map<string, MapSpot[]>,
  cluster: PassageToDecide,
  segments: StageSegment[] | null,
  side: 0 | 1 | null,
  held: Set<string> = new Set(),
): PatchChoices {
  const passage = cluster.passage?.passage;
  if (passage === undefined) {
    return choices;
  }
  // What the scan says may not go (referred to from elsewhere, or the
  // distant view), and what the game ties in.
  const needed = new Set([
    ...held,
    ...passage.parts
      .flat()
      .filter((part) => !part.hideable)
      .map((part) => matchKey(part.object)),
  ]);
  if (side === null) {
    let next = revealObjects(choices, spots, [
      ...sideParts(passage, segments, 0),
      ...sideParts(passage, segments, 1),
    ]);
    for (const { key } of cluster.spots) {
      next = chooseSpot(next, key, null);
    }
    return next;
  }
  const other = side === 0 ? 1 : 0;
  const loser = cluster.plugins[other];
  let next = revealObjects(choices, spots, sideParts(passage, segments, side));
  for (const object of sideParts(passage, segments, other)) {
    if (!isGamePlugin(object.plugin) && !needed.has(matchKey(object))) {
      next = chooseObject(next, object, true);
    }
  }
  for (const { key } of cluster.spots) {
    next = chooseSpot(next, key, loser);
  }
  return next;
}

/** How far around a passage's box an object counts as part of the place, in game units (3 m). */
const AROUND_BOX = 210;

/**
 * The objects of the side kept, `side`, that `choices` hide on their own
 * in or right around the passage: hidden by hand, with the wand or one by
 * one. Keeping that side's version means its whole version, so these are
 * shown again with it, and the panel says so.
 */
export function hiddenByHand(
  choices: PatchChoices,
  passage: MapPassage,
  segments: StageSegment[] | null,
  area: AreaObjects | null,
  side: 0 | 1,
): ObjectRef[] {
  const plugin = passage.plugins[side].toLowerCase();
  const inside = new Set(sideParts(passage, segments, side).map(matchKey));
  const [low, high] = [passage.min, passage.max];
  const near = (position: readonly number[]) =>
    [0, 1, 2].every(
      (axis) =>
        (position[axis] ?? 0) >= (low[axis] ?? 0) - AROUND_BOX &&
        (position[axis] ?? 0) <= (high[axis] ?? 0) + AROUND_BOX,
    );
  for (const object of area?.objects ?? []) {
    if (object.id.plugin.toLowerCase() === plugin && near(object.position)) {
      inside.add(matchKey(object.id));
    }
  }
  return freeObjects(choices).filter(
    (object) => object.plugin.toLowerCase() === plugin && inside.has(matchKey(object)),
  );
}
