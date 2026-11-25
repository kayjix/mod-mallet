// The magic wand of the 3D views (ADR 0055): which parts it takes along
// with a picked one at each level, and what a part is for that (built,
// clutter or nature). Pure, so that a script can test it without a view.

import { boxesTouch, CELLS, chainFrom, holds, type PlacedBox } from "./boxes.ts";

/**
 * Folders of models of nature and the like: trees, plants, grass, rocks and
 * the rest of the landscape, effects and lights. They stand next to what is
 * built and touch it, but never belong to it.
 */
const NATURE =
  /(^|[\\/])(landscape|trees|plants|flora|grass|rocks?|effects|fx|lights?|sky|water|actors|creatures)[\\/]/i;

/** Folders of trees, which stay out of structures even as leaves. */
const TREES = /(^|[\\/])trees[\\/]/i;

/** Folders of small plants: decor a mod puts on what it builds. */
const PLANTS = /(^|[\\/])(plants|flora|grass)[\\/]/i;

/** The folder of clutter: crates, barrels, sacks and whatever else lies about. */
const CLUTTER = /(^|[\\/])clutter[\\/]/i;

/** Folders of trees, plants and grass. */
const FLORA = /(^|[\\/])(trees|plants|flora|grass)[\\/]/i;

/** Whether a model is a tree, a plant or grass. */
export function isFlora(model: string): boolean {
  return FLORA.test(model);
}

/** Whether a model is nature rather than something placed by hand: trees, rocks, landscape. */
export function isNature(model: string): boolean {
  return NATURE.test(model);
}

/** Below this longest side, in game units (about 35 cm), a part is clutter, not building. */
const SMALL = 24;

/** What a part is, for growing a structure. */
export type PartKind = "nature" | "clutter" | "built";

/**
 * Whether a part is decor that a built structure takes along where it
 * touches it: clutter, or small plants; never trees, rocks or landscape.
 */
export function isDecor(model: string, kind: PartKind): boolean {
  if (TREES.test(model)) {
    return false;
  }
  return kind === "clutter" || PLANTS.test(model);
}

/**
 * What a part is, for growing a structure: nature, clutter lying about
 * (by its folder or its size), or something built.
 */
export function kindOf(model: string, size: number): PartKind {
  if (NATURE.test(model)) {
    return "nature";
  }
  return CLUTTER.test(model) || size < SMALL ? "clutter" : "built";
}

/** The longest side of a model's box at `scale`, in game units. */
export function sizeOf(
  box: [[number, number, number], [number, number, number]],
  scale: number,
): number {
  const [low, high] = box;
  return Math.max(high[0] - low[0], high[1] - low[1], high[2] - low[2]) * scale;
}

/**
 * A model's family: its file name without the number of the variant, so
 * that `UpperShelf01` and `UpperShelf02`, or every piece of a stone wall,
 * are alike.
 */
export function modelFamily(model: string): string {
  const name = (model.split(/[\\/]/).pop() ?? model).toLowerCase().replace(/\.nif$/, "");
  return name.replace(/[\d_ -]+[a-z]?$/, "") || name;
}

/**
 * What the magic wand takes at each level (ADR 0055, amended 2026-11-25),
 * from the picked part alone (0) to the most it takes.
 */
export const WAND_LEVELS = {
  /** What lies on or in it. */
  contents: 1,
  /** The same model right beside it. */
  beside: 2,
  /** Its kind in a row. */
  row: 3,
  /** The structure of its set of models it is part of: a scaffold tower. */
  structure: 4,
  /** The building it is part of, of any set of models: a gatehouse with its tower. */
  building: 5,
  /** Everything of its sort joined to it, the old "whole structure". */
  joined: 6,
} as const;

/** The highest level of the magic wand (ADR 0055); level 0 takes the object alone. */
export const WAND_TOP = WAND_LEVELS.joined;

/**
 * What the magic wand chains at the levels "beside", "row" and "joined":
 * which parts are alike (the same model, the same family of models, or the
 * same kind of thing: built, clutter or nature), and how far apart two
 * links may be, in game units.
 */
const WAND_STEPS: Record<number, { alike: "model" | "family" | "kind"; gap: number }> = {
  [WAND_LEVELS.beside]: { alike: "model", gap: 14 },
  [WAND_LEVELS.row]: { alike: "family", gap: 140 },
  [WAND_LEVELS.joined]: { alike: "kind", gap: 14 },
};

/** How far from the picked object the magic wand reaches at most, in game units (about 30 m). */
export const WAND_REACH = 2048;

/** How many parts the magic wand chains at most. */
const WAND_MOST = 300;

/** How close decor must come to a part to lie on it, in game units. */
const CONTENT_MARGIN = 4;

/**
 * At "everything joined", how many times longer than the picked part a
 * link may be: a road or a slab of ground of the same mod touches half a
 * town, and hiding it tears holes.
 */
const WAND_LARGER = 4;

/** A link may always be this long, in game units (about 7 m). */
const WAND_LARGE = 512;

/** How close two parts of a building must come to be built together, in game units (about 11 cm). */
const BUILT_MARGIN = 0;

/** How close the underside of a part must come to what it rests on, in game units (about 23 cm). */
const REST = 16;

/** How much of a part must lie in another's surfaces for it to be built into it. */
const EMBEDDED = 0.25;

/** How much of its holder's height a resting part's middle stands above the holder's middle. */
const ABOVE = 0.25;

/**
 * A part at least this long, in game units (about 7 m), and {@link DWARF}
 * times longer than another is the ground that one stands on (a city
 * wall, a cliff, a slab), not the building it belongs to.
 */
const LARGE = 490;

/** See {@link LARGE}. */
const DWARF = 8;

/**
 * A building is complete where what the wand took hangs on the rest by at
 * most this share of its links: a stair to the next house, the end of a
 * wall a tower leans on.
 */
const LOOSE = 0.1;

/** A model's set: the folder it lies in, such as the scaffolds of Whiterun. */
export function modelSet(model: string): string {
  return model
    .toLowerCase()
    .replace(/\\/g, "/")
    .replace(/\/[^/]*$/, "");
}

/** What the wand knows of a part of the view. */
export interface WandPart {
  /** Its model, below the data directory. */
  model: string;
  /** The mod it belongs to (see `ownerOf`), lower case. */
  owner: string;
  /** The plugin whose version of it wins, lower case. */
  plugin: string;
  /** Its model's box as placed. */
  box: PlacedBox;
  /** The longest side of its model's box as placed, in game units. */
  size: number;
  /** Which cells of its model's box hold surfaces (see `occupancy`); null for the whole box. */
  cells: Uint8Array | null;
}

/**
 * The parts the magic wand takes along with `parts[start]` at `level`, by
 * index, the start first: from level 1 what lies on or in it, from level 2
 * on the parts of its mod that belong with it, with what lies on or in
 * them. Parts of other mods are passed over, and only parts within
 * {@link WAND_REACH} of the start count. Each level takes what the
 * "structure" and "building" levels below it take.
 */
export function wandChain(parts: readonly WandPart[], start: number, level: number): number[] {
  const first = parts[start];
  if (first === undefined) {
    return [];
  }
  if (level <= 0) {
    return [start];
  }
  const owner = first.owner;
  const firstKind = kindOf(first.model, first.size);
  const [x, y, z] = first.box.centre;
  // The parts a chain may reach, and the decor of the mod that may lie on
  // them: decor counts also where the mod's version of another's object
  // wins (a book it moved onto its shelf, ADR 0050).
  const near = parts.flatMap((part, index) => {
    const ours = index === start || part.owner === owner || part.plugin === owner;
    const [ox, oy, oz] = part.box.centre;
    return ours && Math.hypot(ox - x, oy - y, oz - z) <= WAND_REACH ? [index] : [];
  });
  const own = near.filter((index) => index === start || parts[index]?.owner === owner);
  const chain = [start];
  const known = new Set(chain);
  const add = (found: readonly number[]) => {
    for (const index of found) {
      if (!known.has(index)) {
        known.add(index);
        chain.push(index);
      }
    }
  };
  const step = WAND_STEPS[level];
  if (step !== undefined) {
    const family = modelFamily(first.model);
    const alike = (part: WandPart) =>
      step.alike === "model"
        ? part.model.toLowerCase() === first.model.toLowerCase()
        : step.alike === "family"
          ? modelFamily(part.model) === family
          : kindOf(part.model, part.size) === firstKind &&
            part.size <= Math.max(first.size * WAND_LARGER, WAND_LARGE);
    const links = own.filter((index) => {
      const part = parts[index];
      return index === start || (part !== undefined && alike(part));
    });
    add(
      chainFrom(
        links.map((index) => parts[index]?.box ?? first.box),
        links.indexOf(start),
        step.gap,
        WAND_MOST,
      ).map((at) => links[at] ?? start),
    );
  }
  if (level >= WAND_LEVELS.structure) {
    add(buildingOf(parts, start, own, true));
  }
  if (level >= WAND_LEVELS.building) {
    add(buildingOf(parts, start, own, false));
  }
  return [...chain, ...contentsOf(parts, near, chain)];
}

/**
 * The building `parts[start]` is part of, by index, the start first: of
 * the parts `own` of the same kind (built or clutter), and of the same set
 * of models where `sameSet`, those built into one another or standing on
 * one another (see {@link builtTogether}), up to where they hang on the
 * rest by only a few links. The wand takes the part most tied to what it
 * has taken, by the share of its links that lead into it, one after the
 * other. Once what it took is held to the rest by at most {@link LOOSE} of
 * its links, it stops as soon as one more part would not tie it more
 * loosely still: two houses joined by a stair, a tower set against a city
 * wall.
 */
function buildingOf(
  parts: readonly WandPart[],
  start: number,
  own: readonly number[],
  sameSet: boolean,
): number[] {
  const first = parts[start];
  if (first === undefined) {
    return [];
  }
  const kind = kindOf(first.model, first.size);
  const set = modelSet(first.model);
  const candidates = own.filter((index) => {
    const part = parts[index];
    return (
      part !== undefined &&
      kindOf(part.model, part.size) === kind &&
      (!sameSet || modelSet(part.model) === set)
    );
  });
  const known = new Map<number, number[]>();
  const linksOf = (index: number): number[] => {
    let found = known.get(index);
    if (found === undefined) {
      const part = parts[index];
      found =
        part === undefined
          ? []
          : candidates.filter((other) => {
              const candidate = parts[other];
              return other !== index && candidate !== undefined && builtTogether(part, candidate);
            });
      known.set(index, found);
    }
    return found;
  };
  const taken = [start];
  const inside = new Set(taken);
  // The links of what is taken, and how many of them lead out of it.
  let links = linksOf(start).length;
  let out = links;
  let loose = false;
  while (taken.length < WAND_MOST) {
    let best = -1;
    let bestInto = 0;
    let bestShare = -1;
    for (const from of taken) {
      for (const next of linksOf(from)) {
        if (inside.has(next)) {
          continue;
        }
        const its = linksOf(next);
        const into = its.filter((other) => inside.has(other)).length;
        const share = into / Math.max(1, its.length);
        if (share > bestShare) {
          best = next;
          bestInto = into;
          bestShare = share;
        }
      }
    }
    if (best < 0) {
      break;
    }
    const nextLinks = links + linksOf(best).length;
    const nextOut = out + linksOf(best).length - 2 * bestInto;
    // Once loose, a part joins only while it ties what is taken more
    // loosely still, and only when more than half of its links lead into
    // it, or half and it stands on what is taken (as passages take what
    // rests on them, ADR 0072): a wall's next piece, held by the tower at
    // one end and by the wall at the other, stays; a gate on the tower's
    // platform comes along.
    const held = 2 * bestInto - linksOf(best).length;
    const part = parts[best];
    const standsOnTaken =
      part !== undefined &&
      linksOf(best).some((other) => {
        const holder = parts[other];
        return inside.has(other) && holder !== undefined && restsOn(part, holder);
      });
    if (
      loose &&
      (held < 0 ||
        (held === 0 && !standsOnTaken) ||
        nextOut * Math.max(1, links) >= out * Math.max(1, nextLinks))
    ) {
      break;
    }
    taken.push(best);
    inside.add(best);
    links = nextLinks;
    out = nextOut;
    loose = loose || out <= LOOSE * links;
  }
  return taken;
}

/**
 * Whether two parts are built together: one is built into the other (a
 * quarter of its surfaces lie on the other's) or stands on it (its
 * underside on the other's surfaces, its middle clearly higher). A part
 * beside another at the same height is neither, and nothing is built
 * together with a part that dwarfs it (see {@link LARGE}).
 */
export function builtTogether(a: WandPart, b: WandPart): boolean {
  if (!boxesTouch(a.box, b.box, REST) || dwarfs(a, b) || dwarfs(b, a)) {
    return false;
  }
  return shareIn(a, b) >= EMBEDDED || shareIn(b, a) >= EMBEDDED || restsOn(a, b) || restsOn(b, a);
}

/** Whether `big` is so much longer than `small` that it is the ground under it. */
function dwarfs(big: WandPart, small: WandPart): boolean {
  return big.size >= LARGE && big.size >= DWARF * small.size;
}

/**
 * How far from the middle of one of its cells the surfaces of `part` may
 * lie, along each axis of `other`, with `margin` added: half a cell of
 * `part` as turned against `other`. A post is thin across and long
 * along, and so are its cells.
 */
function cellReach(part: WandPart, other: WandPart, margin: number): [number, number, number] {
  const along = (axis: readonly number[]) =>
    margin +
    part.box.axes.reduce(
      (sum, own, index) =>
        sum +
        Math.abs(own[0] * (axis[0] ?? 0) + own[1] * (axis[1] ?? 0) + own[2] * (axis[2] ?? 0)) *
          ((part.box.half[index] ?? 0) / CELLS),
      0,
    );
  const [x, y, z] = other.box.axes;
  return [along(x), along(y), along(z)];
}

/** The points each part's surfaces take, worked out once. */
const solidPoints = new WeakMap<WandPart, number[][]>();

/** The points a part's surfaces take: the middles of the cells that hold some, in the world. */
function pointsOf(part: WandPart): number[][] {
  const known = solidPoints.get(part);
  if (known !== undefined) {
    return known;
  }
  const { box, cells } = part;
  const points: number[][] = [];
  for (let i = 0; i < CELLS; i += 1) {
    for (let j = 0; j < CELLS; j += 1) {
      for (let k = 0; k < CELLS; k += 1) {
        if (cells !== null && cells[(i * CELLS + j) * CELLS + k] !== 1) {
          continue;
        }
        const along = [i, j, k].map((cell) => ((cell + 0.5) / CELLS) * 2 - 1);
        points.push(
          [0, 1, 2].map(
            (row) =>
              box.centre[row as 0 | 1 | 2] +
              box.axes.reduce(
                (sum, axis, index) =>
                  sum + axis[row as 0 | 1 | 2] * (box.half[index] ?? 0) * (along[index] ?? 0),
                0,
              ),
          ),
        );
      }
    }
  }
  solidPoints.set(part, points);
  return points;
}

/**
 * Whether the point `at` lies within `margin` (along each of the part's
 * axes) of a cell of `part` that holds surfaces.
 */
function solidNear(part: WandPart, at: readonly number[], margins: readonly number[]): boolean {
  const { box, cells } = part;
  const ranges: [number, number][] = [];
  for (let axis = 0; axis < 3; axis += 1) {
    const direction = box.axes[axis as 0 | 1 | 2];
    const half = box.half[axis as 0 | 1 | 2];
    const margin = margins[axis] ?? 0;
    const value =
      ((at[0] ?? 0) - box.centre[0]) * direction[0] +
      ((at[1] ?? 0) - box.centre[1]) * direction[1] +
      ((at[2] ?? 0) - box.centre[2]) * direction[2];
    if (Math.abs(value) > half + margin) {
      return false;
    }
    const cell = (along: number) =>
      half < 1e-3
        ? 0
        : Math.min(CELLS - 1, Math.max(0, Math.floor(((along + half) / (2 * half)) * CELLS)));
    ranges.push([cell(value - margin), cell(value + margin)]);
  }
  if (cells === null) {
    return true;
  }
  const [x0, x1] = ranges[0] ?? [0, -1];
  const [y0, y1] = ranges[1] ?? [0, -1];
  const [z0, z1] = ranges[2] ?? [0, -1];
  for (let i = x0; i <= x1; i += 1) {
    for (let j = y0; j <= y1; j += 1) {
      for (let k = z0; k <= z1; k += 1) {
        if (cells[(i * CELLS + j) * CELLS + k] === 1) {
          return true;
        }
      }
    }
  }
  return false;
}

/** How much of `part`'s surfaces lie in or on `other`'s, from 0 to 1. */
function shareIn(part: WandPart, other: WandPart): number {
  const points = pointsOf(part);
  if (points.length === 0) {
    return 0;
  }
  const reach = [BUILT_MARGIN, BUILT_MARGIN, BUILT_MARGIN];
  return points.filter((point) => solidNear(other, point, reach)).length / points.length;
}

/** How far a placed box reaches up and down from its middle. */
function halfHeight(box: PlacedBox): number {
  return box.axes.reduce((sum, axis, index) => sum + Math.abs(axis[2]) * (box.half[index] ?? 0), 0);
}

/**
 * Whether `part` stands on `holder`: the lowest of its surfaces lie on the
 * holder's top, and its middle stands clearly higher. Where the holder
 * goes on upward right above the contact, the part stands beside it (a
 * rail along a city wall), not on it.
 */
function restsOn(part: WandPart, holder: WandPart): boolean {
  if (part.box.centre[2] < holder.box.centre[2] + ABOVE * 2 * halfHeight(holder.box)) {
    return false;
  }
  const points = pointsOf(part);
  if (points.length === 0) {
    return false;
  }
  const band = (2 * halfHeight(part.box)) / CELLS;
  const lowest = Math.min(...points.map((point) => point[2] ?? 0));
  const reach = cellReach(part, holder, REST);
  const lift = REST + band / 2;
  // Above the contact, the same reach sideways, none up and down.
  const beside = reach.map(
    (value, axis) => value * (1 - Math.abs(holder.box.axes[axis as 0 | 1 | 2][2])),
  );
  return points.some(
    (point) =>
      (point[2] ?? 0) <= lowest + band &&
      solidNear(holder, point, reach) &&
      !solidNear(holder, [point[0] ?? 0, point[1] ?? 0, (point[2] ?? 0) + lift], beside),
  );
}

/**
 * What lies on or in the parts `chain`, of the parts `near`: decor whose
 * surfaces touch one of theirs, or anything whose middle is inside one,
 * among its surfaces (not in the empty room of a hollow house or tower).
 * The chain does not grow from these.
 */
function contentsOf(
  parts: readonly WandPart[],
  near: readonly number[],
  chain: readonly number[],
): number[] {
  const inChain = new Set(chain);
  const holders = chain.flatMap((index) => {
    const part = parts[index];
    return part === undefined ? [] : [part];
  });
  return near.filter((index) => {
    const part = parts[index];
    if (part === undefined || inChain.has(index)) {
      return false;
    }
    if (isDecor(part.model, kindOf(part.model, part.size))) {
      return holders.some(
        (holder) =>
          boxesTouch(holder.box, part.box, CONTENT_MARGIN) &&
          pointsOf(part).some((point) =>
            solidNear(holder, point, cellReach(part, holder, CONTENT_MARGIN)),
          ),
      );
    }
    return holders.some(
      (holder) =>
        holds(holder.box, part.box.centre) && solidNear(holder, part.box.centre, [0, 0, 0]),
    );
  });
}
