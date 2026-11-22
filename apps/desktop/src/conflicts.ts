// Which collisions and door spots an object picked in a 3D view is part
// of, and which of them hiding it decided. Pure, so that a script can test
// it without a view.

/** The least a spot says for this: the objects it is about. */
export interface ConflictSpot {
  parts?: readonly { object: { plugin: string; localId: number }; name: string }[] | null;
}

type Ref = { plugin: string; localId: number };

const keyOf = (object: Ref) => `${object.plugin.toLowerCase()}:${object.localId}`;

/** What the spots of an object say. */
export interface Conflicts<S> {
  /** Spots the objects are part of that nothing decides yet. */
  open: S[];
  /** Spots that are decided only because one of the objects is hidden on its own. */
  decidedByHiding: S[];
}

/**
 * The spots `objects` are part of, told apart by whether they are still
 * open. `decision` says what is decided for a spot (undefined: nothing),
 * `explicit` whether the spot or its finding has a choice of its own; a
 * spot decided without one is decided by a part hidden on its own. Spots
 * that one of `except` is part of are left out, for a second list beside
 * the first.
 */
export function conflictsOf<S extends ConflictSpot>(
  spots: Iterable<S>,
  objects: readonly Ref[],
  decision: (spot: S) => string | null | undefined,
  explicit: (spot: S) => boolean,
  except: readonly Ref[] = [],
): Conflicts<S> {
  const wanted = new Set(objects.map(keyOf));
  const left = new Set(except.map(keyOf));
  const open: S[] = [];
  const decidedByHiding: S[] = [];
  for (const spot of spots) {
    const keys = (spot.parts ?? []).map((part) => keyOf(part.object));
    if (!keys.some((key) => wanted.has(key)) || keys.some((key) => left.has(key))) {
      continue;
    }
    const decided = decision(spot);
    if (decided === undefined) {
      open.push(spot);
    } else if (typeof decided === "string" && !explicit(spot)) {
      decidedByHiding.push(spot);
    }
  }
  return { open, decidedByHiding };
}

/**
 * The names of the parts of a spot that are not among `objects` and not of
 * their plugins: what they collide with.
 */
export function collidingWith(spot: ConflictSpot, objects: readonly Ref[]): string[] {
  const ours = new Set(objects.map((object) => object.plugin.toLowerCase()));
  const names = (spot.parts ?? [])
    .filter((part) => !ours.has(part.object.plugin.toLowerCase()))
    .map((part) => part.name);
  return [...new Set(names)];
}
