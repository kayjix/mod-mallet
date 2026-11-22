// One source of truth for what the user decided (ADR 0081).
//
// Every page reads the same choices (`PatchChoices`), but each used to
// work out on its own what they hide and why, and several drifted apart:
// the map, the collisions, the guided review and the patch did not always
// agree whether an object was hidden. This module answers those questions
// once, for a snapshot of the choices, and every page asks it:
//
// - whether an object is hidden, and by what: on its own, by a spot's
//   decision, by stacked doors merged into one, or by a passage decided as
//   a whole; which patch the deciding choice belongs to; and whether a
//   decision that waits for a switched-off mod would hide it;
// - what is decided for a spot or a passage, exceptions included;
// - the choices with objects hidden or shown again, whatever hid them.
//
// Pure, so that the tests replay what the pages do without a window.

import {
  activePatch,
  countedChoices,
  countedPatches,
  DUEL,
  isForeign,
  tickedOf,
  withChange,
} from "./book.ts";
import type { PatchBook } from "./generated/patch-book";
import type {
  DoorGroup,
  DoorMerge,
  InstalledPatch,
  MapPassage,
  MapSpot,
  ObjectRef,
  RenderedReport,
  SpotPart,
  SwitchedOffPlugin,
} from "./generated/rendered-report";
import { stillColliding } from "./part-groups.ts";
import {
  choicesOf,
  chooseObject,
  chooseSpot,
  decisionAt,
  forgetObjects,
  freeObjects,
  hiddenAt,
  hiddenKeysOf,
  installedKeys,
  keepChoice,
  keptBy,
  matchKey,
  mergeChoices,
  type PatchChoices,
  pairsOf,
  revealObjects,
  spotKey,
  withoutMisspelt,
} from "./patch.ts";
import { splitWaiting, type Waiting, withWaiting } from "./switched-off.ts";

/** What is decided for some spots: nothing yet, both kept, one plugin hidden, or not all alike. */
export type Decided =
  | { type: "open" }
  | { type: "kept" }
  | { type: "hidden"; plugin: string }
  | { type: "mixed"; open: number };

/** What hides an object. */
export type HiddenBy =
  /** Hidden on its own, outside any decision about a collision. */
  | "own"
  /** By the decision of a collision's spot, or of its whole finding. */
  | "spot"
  /** By stacked doors merged into another door (ADR 0067). */
  | "door"
  /** By the decision of a whole passage (ADR 0072). */
  | "passage";

/** A patch of the book, as far as a page names it. */
export interface PatchRef {
  id: string;
  name: string;
  /** Whether it is the patch being edited. */
  edited: boolean;
}

/** Why an object is hidden. */
export interface Hiding {
  by: HiddenBy;
  /** The spot whose decision hides it, by its key; null if hidden on its own. */
  spot: string | null;
  /** The passage it belongs to, by {@link passageId}, where `by` is "passage". */
  passage: string | null;
  /**
   * The patches that count and make the deciding choice, in the order they
   * count; the last one wins. Empty without a book.
   */
  patches: PatchRef[];
}

/** What a snapshot of the decisions is made from. */
export interface DecisionInput {
  /** The choices that count: those of the patches switched on, without the waiting ones. */
  choices: PatchChoices;
  /** The spots of the collisions, by finding. */
  spots: Map<string, MapSpot[]>;
  /** The findings set aside: their spots' decisions go into no patch. */
  ignored?: ReadonlySet<string>;
  /** The passages the scan found (ADR 0072). */
  passages?: readonly MapPassage[];
  /** The choices that wait for switched-off mods (ADR 0061). */
  waiting?: PatchChoices;
  /** The user's patches (ADR 0070), to say which one a decision is in. */
  book?: PatchBook | null;
}

/** What every page asks about the decisions, for one snapshot of them. */
export interface Decisions {
  /** The choices that count. */
  readonly choices: PatchChoices;
  /** The collisions' spots whose decisions count, by finding: none set aside. */
  readonly spots: Map<string, MapSpot[]>;
  /** The objects hidden, by {@link matchKey}; shared, so not to be changed. */
  readonly hiddenKeys: Set<string>;
  /** The objects hidden, each once. */
  readonly hiddenObjects: ObjectRef[];
  /** Whether an object is hidden. */
  isHidden(object: ObjectRef): boolean;
  /** Why an object is hidden; null where it is not. */
  hiding(object: ObjectRef): Hiding | null;
  /** Whether a decision that waits for a switched-off mod would hide an object that is shown now. */
  waits(object: ObjectRef): boolean;
  /** Whether an object was shown again on purpose, after something had hidden it. */
  shownAgain(object: ObjectRef): boolean;
  /** What is decided for a spot: the plugin whose objects go, null to keep all, undefined if open. */
  decisionOf(spot: MapSpot): string | null | undefined;
  /** What is decided for a whole passage, a part shown again or hidden by hand counting as an exception. */
  decisionOfPassage(passage: MapPassage): Decided;
  /**
   * The side picked in each duel of files (ADR 0054), by duel: from the
   * `duel:` choices of the patches that count, as every other decision.
   */
  readonly duels: Record<string, number>;
  /** The patches that count and make a choice, by its key, in the order they count. */
  patchesOf(key: string): PatchRef[];
  /**
   * The snapshot the choices `next` would make, as a page asks "what if"
   * before it changes anything: the same spots, findings set aside,
   * waiting decisions and patches as this one.
   */
  whatIf(next: PatchChoices): Decisions;
  /**
   * The choices with one side kept at some spots (see {@link keepSide}), or
   * both kept where `plan.loser` is null.
   */
  keepSide(plan: SidePlan): PatchChoices;
  /** The choices with `objects` shown again, whatever hid them. */
  show(objects: ObjectRef[]): PatchChoices;
  /**
   * The choices with `objects` hidden: on their own, or a door of stacked
   * doors through the door decision, which merges the doors into the one
   * left (ADR 0067).
   */
  hide(objects: ObjectRef[]): PatchChoices;
}

/** A passage's identity, the same as the review's: its two plugins and its first part. */
export function passageId(passage: MapPassage): string {
  const first = passage.parts.flat()[0]?.object;
  return `passage|${passage.plugins.map((plugin) => plugin.toLowerCase()).join("+")}|${
    first === undefined ? passage.console : matchKey(first)
  }`;
}

/** The spots that lie in a passage: those whose parts all belong to it. */
export function spotsOfPassage(passage: MapPassage, spots: Iterable<MapSpot>): MapSpot[] {
  const inside = new Set(passage.parts.flat().map((part) => matchKey(part.object)));
  const found: MapSpot[] = [];
  for (const spot of spots) {
    const parts = spot.parts ?? [];
    if (parts.length > 0 && parts.every((part) => inside.has(matchKey(part.object)))) {
      found.push(spot);
    }
  }
  return found;
}

/** The objects whose choice says they were shown again on purpose, by {@link matchKey}. */
function shownAgainKeys(choices: PatchChoices): Set<string> {
  const found = new Set<string>();
  for (const [key, value] of Object.entries(choices)) {
    if (value === null && key.startsWith("object:")) {
      const bar = key.lastIndexOf("|");
      const localId = Number(key.slice(bar + 1));
      if (bar > 0 && Number.isInteger(localId)) {
        found.add(matchKey({ plugin: key.slice("object:".length, bar), localId }));
      }
    }
  }
  return found;
}

/**
 * What is decided for `spots`, all together. Where they belong to a
 * `passage` decided alike, a part of it shown again on purpose on the side
 * that goes, or hidden on its own on the side that stays, makes the
 * decision one with exceptions (`mixed` with nothing open).
 */
export function decidedOf(
  spots: { spot: MapSpot; finding: string }[],
  choices: PatchChoices,
  passage?: MapPassage,
): Decided {
  const values = spots.map(({ spot, finding }) => {
    const parts = spot.parts ?? [];
    const value = decisionAt(choices, finding, spotKey(spot), parts);
    // A spot that keeps one door counts by the plugin of the doors it hides.
    return keptBy(value) === null ? value : (hiddenAt(parts, value ?? null)[0]?.plugin ?? null);
  });
  const open = values.filter((value) => value === undefined).length;
  if (open === values.length) {
    return { type: "open" };
  }
  if (open > 0) {
    return { type: "mixed", open };
  }
  const plugins = new Set(
    values.map((value) => (value === null ? "" : (value ?? "").toLowerCase())),
  );
  if (plugins.size > 1) {
    return { type: "mixed", open: 0 };
  }
  const only = values[0];
  const decided: Decided =
    only === null || only === undefined ? { type: "kept" } : { type: "hidden", plugin: only };
  if (passage === undefined) {
    return decided;
  }
  // The passage's parts no spot names were hidden on their own with it, and
  // can be shown again on their own: that is an exception too.
  const going = decided.type === "hidden" ? decided.plugin.toLowerCase() : null;
  const again = shownAgainKeys(choices);
  const free = new Set(freeObjects(choices).map(matchKey));
  const exception = passage.parts.flat().some((part) => {
    const key = matchKey(part.object);
    const goes = going !== null && part.object.plugin.toLowerCase() === going;
    return goes ? again.has(key) : free.has(key);
  });
  return exception ? { type: "mixed", open: 0 } : decided;
}

/** The plan a spot's doors are merged by, if its choice keeps one of them. */
export function mergedBy(group: DoorGroup, decided: string | null | undefined): DoorMerge | null {
  const kept = keptBy(decided);
  return kept === null
    ? null
    : (group.plans.find((plan) => matchKey(plan.keep) === matchKey(kept)) ?? null);
}

/**
 * The choices with a group of stacked doors merged into the door `plan`
 * keeps: the spot's choice keeps that door and hides the others
 * ({@link keepChoice}), and the kept door is shown again however it was
 * hidden. With `plan` null every door stays ("All of them stay"): the spot
 * keeps all, and every hiding of its doors goes, whatever its source.
 */
export function mergeDoors(
  choices: PatchChoices,
  key: string,
  group: DoorGroup,
  plan: DoorMerge | null,
  spots: Map<string, MapSpot[]>,
): PatchChoices {
  if (plan === null) {
    const doors = group.doors.map((door) => door.object);
    return chooseSpot(forgetObjects(revealObjects(choices, spots, doors), doors), key, null);
  }
  const next = forgetObjects(revealObjects(choices, spots, [plan.keep]), [plan.keep, ...plan.hide]);
  return chooseSpot(next, key, keepChoice(plan.keep));
}

/**
 * The choices with the door `object` of a group hidden, as the object card
 * or the magic wand hides it: where one door is then left, the group is
 * merged into it; otherwise the door is hidden on its own.
 */
export function hideDoor(
  choices: PatchChoices,
  key: string,
  group: DoorGroup,
  object: ObjectRef,
  spots: Map<string, MapSpot[]>,
): PatchChoices {
  const hidden = hiddenKeysOf(spots, choices);
  const left = group.doors.filter(
    (door) => matchKey(door.object) !== matchKey(object) && !hidden.has(matchKey(door.object)),
  );
  const [only] = left;
  const plan =
    left.length === 1 && only !== undefined
      ? group.plans.find((candidate) => matchKey(candidate.keep) === matchKey(only.object))
      : undefined;
  return plan === undefined
    ? chooseObject(choices, object, true)
    : mergeDoors(choices, key, group, plan, spots);
}

/** One spot's decision that hides an object. */
interface SpotHiding {
  spot: MapSpot;
  key: string;
  /** The choice it comes from: the spot's own, or its finding's. */
  source: string | null;
  door: boolean;
}

/** The spots without those of findings set aside. */
function countedSpots(
  spots: Map<string, MapSpot[]>,
  ignored: ReadonlySet<string> | undefined,
): Map<string, MapSpot[]> {
  if (ignored === undefined || ignored.size === 0) {
    return spots;
  }
  const found = new Map<string, MapSpot[]>();
  for (const [finding, list] of spots) {
    if (!ignored.has(finding)) {
      found.set(finding, list);
    }
  }
  return found;
}

/** The snapshot of the decisions every page asks (see {@link Decisions}). */
export function decisionsOf(input: DecisionInput): Decisions {
  const { choices, passages = [], waiting = {}, book = null } = input;
  const spots = countedSpots(input.spots, input.ignored);
  // Every object hidden, and the first spot decision that hides it.
  const free = new Map(freeObjects(choices).map((object) => [matchKey(object), object]));
  const bySpot = new Map<string, SpotHiding>();
  const hiddenObjects = new Map<string, ObjectRef>(free);
  for (const [finding, list] of spots) {
    for (const spot of list) {
      const key = spotKey(spot);
      const parts = spot.parts ?? [];
      const decided = decisionAt(choices, finding, key, parts);
      if (key === null || typeof decided !== "string") {
        continue;
      }
      const source = key in choices ? key : finding in choices ? finding : null;
      for (const object of hiddenAt(parts, decided)) {
        const match = matchKey(object);
        hiddenObjects.set(match, hiddenObjects.get(match) ?? object);
        if (!bySpot.has(match)) {
          bySpot.set(match, {
            spot,
            key,
            source,
            door: spot.doors != null && keptBy(decided) !== null,
          });
        }
      }
    }
  }
  const hiddenKeys = new Set(hiddenObjects.keys());
  const again = shownAgainKeys(choices);

  // The passages decided as a whole, with the plugin whose parts go.
  const allSpots = [...spots.values()].flat();
  const passageOf = new Map<string, { passage: MapPassage; id: string; spots: MapSpot[] }>();
  for (const passage of passages) {
    const entry = { passage, id: passageId(passage), spots: spotsOfPassage(passage, allSpots) };
    for (const part of passage.parts.flat()) {
      passageOf.set(matchKey(part.object), entry);
    }
  }
  const decisionOfPassage = (passage: MapPassage) =>
    decidedOf(
      spotsOfPassage(passage, allSpots).map((spot) => ({ spot, finding: spot.finding })),
      choices,
      passage,
    );
  // A passage hides a part when all its spots are decided, some of them
  // against the part's plugin; others may be exceptions.
  const passageHides = (object: ObjectRef) => {
    const entry = passageOf.get(matchKey(object));
    if (entry === undefined || entry.spots.length === 0) {
      return null;
    }
    const plugin = object.plugin.toLowerCase();
    let against = false;
    for (const spot of entry.spots) {
      const parts = spot.parts ?? [];
      const decided = decisionAt(choices, spot.finding, spotKey(spot), parts);
      if (decided === undefined) {
        return null;
      }
      against ||= hiddenAt(parts, decided).some((hidden) => hidden.plugin.toLowerCase() === plugin);
    }
    return against ? entry : null;
  };

  // Which patches make a choice, in the order in which they count.
  const counted =
    book === null ? [] : countedPatches(book).map((patch) => ({ patch, ticked: tickedOf(patch) }));
  const edited = book === null ? undefined : activePatch(book)?.id;
  const patchesOf = (keys: string[]): PatchRef[] =>
    counted
      .filter(({ ticked }) => keys.some((key) => key in ticked))
      .map(({ patch }) => ({ id: patch.id, name: patch.name, edited: patch.id === edited }));
  const ownKeys = (object: ObjectRef) =>
    Object.keys(choices).filter(
      (key) =>
        key.startsWith("object:") &&
        key.toLowerCase() === `object:${object.plugin}|${object.localId}`.toLowerCase(),
    );

  let waitingKeys: Set<string> | null = null;
  const waitingHidden = () => {
    if (waitingKeys === null) {
      waitingKeys =
        Object.keys(waiting).length === 0
          ? new Set()
          : hiddenKeysOf(spots, { ...waiting, ...choices });
    }
    return waitingKeys;
  };

  const doorAt = (object: ObjectRef) => {
    const match = matchKey(object);
    for (const list of spots.values()) {
      for (const spot of list) {
        const key = spotKey(spot);
        const group = spot.doors;
        if (key !== null && group?.doors.some((door) => matchKey(door.object) === match)) {
          return { key, group };
        }
      }
    }
    return null;
  };

  const duels: Record<string, number> = {};
  for (const [key, value] of Object.entries(choices)) {
    const side = Number(value);
    if (key.startsWith(DUEL) && value !== null && (side === 0 || side === 1)) {
      duels[key.slice(DUEL.length)] = side;
    }
  }

  const snapshot: Decisions = {
    whatIf: (next) => (next === choices ? snapshot : decisionsOf({ ...input, choices: next })),
    keepSide: (plan) => keepSide(choices, spots, plan),
    choices,
    spots,
    duels,
    patchesOf: (key) => patchesOf([key]),
    hiddenKeys,
    hiddenObjects: [...hiddenObjects.values()],
    isHidden: (object) => hiddenKeys.has(matchKey(object)),
    hiding: (object) => {
      const match = matchKey(object);
      if (!hiddenKeys.has(match)) {
        return null;
      }
      const atSpot = bySpot.get(match);
      // Decided at a spot, explicitly: by the doors, the passage or the spot.
      if (atSpot !== undefined && atSpot.source !== null) {
        const passage = atSpot.door ? null : passageHides(object);
        return {
          by: atSpot.door ? "door" : passage !== null ? "passage" : "spot",
          spot: atSpot.key,
          passage: passage?.id ?? null,
          patches: patchesOf([atSpot.source]),
        };
      }
      // Hidden on its own: with its passage, where that hides its side.
      const passage = passageHides(object);
      return {
        by: passage !== null ? "passage" : "own",
        spot: null,
        passage: passage?.id ?? null,
        patches: patchesOf(ownKeys(object)),
      };
    },
    waits: (object) => !hiddenKeys.has(matchKey(object)) && waitingHidden().has(matchKey(object)),
    shownAgain: (object) => again.has(matchKey(object)),
    decisionOf: (spot) => decisionAt(choices, spot.finding, spotKey(spot), spot.parts ?? []),
    decisionOfPassage,
    show: (objects) => revealObjects(choices, spots, objects),
    hide: (objects) => {
      let next = choices;
      for (const object of objects) {
        const door = doorAt(object);
        next =
          door === null
            ? chooseObject(next, object, true)
            : hideDoor(next, door.key, door.group, object, spots);
      }
      return next;
    },
  };
  return snapshot;
}

/**
 * One side of a place kept against the other (ADR 0050, 0080): the kept
 * side's parts shown again however they were hidden, the parts of the side
 * given up that may go hidden on their own, and the spots of the place
 * decided against that side. With `loser` null both sides stay: every part
 * of `kept` is shown again, and the spots keep both.
 */
export interface SidePlan {
  /** The parts that stay, shown again however they were hidden. */
  kept: ObjectRef[];
  /** The parts that go; what the game needs is left out of them by the caller. */
  gone: ObjectRef[];
  /** The keys of the spots decided by it. */
  spots: string[];
  /** The plugin given up, or null to keep both. */
  loser: string | null;
}

/** The choices with a {@link SidePlan} applied. */
export function keepSide(
  choices: PatchChoices,
  spots: Map<string, MapSpot[]>,
  plan: SidePlan,
): PatchChoices {
  let next = revealObjects(choices, spots, plan.kept);
  for (const object of plan.loser === null ? [] : plan.gone) {
    next = chooseObject(next, object, true);
  }
  for (const key of plan.spots) {
    next = chooseSpot(next, key, plan.loser);
  }
  return next;
}

/** A part of a place as the review lists it. */
export interface PlacePart {
  id: ObjectRef;
}

/** What of a mod given up still collides at a place, after some choices. */
export interface Leftovers<P extends PlacePart> {
  /** The plugin given up. */
  loser: string;
  /** Its index among the place's two plugins. */
  side: 0 | 1;
  /** Its parts still there, each once. */
  parts: P[];
}

/** A place as {@link leftoversOf} reads it. */
export interface LeftoverPlace<P extends PlacePart> {
  /** The place's two plugins. */
  sides: readonly [string, string];
  /** Its spots. */
  spots: readonly { key: string; spot: MapSpot; finding: string }[];
  /** A part of a spot as the page lists it. */
  partOf: (part: SpotPart) => P;
  /**
   * Where the place is a passage (ADR 0072): its sections, each with its
   * spots and each side's parts. A section without spots counts as given
   * up with its side.
   */
  sections?: readonly {
    spots: readonly { spot: MapSpot; finding: string }[];
    parts: readonly [readonly P[], readonly P[]];
  }[];
  /** The parts the user said to leave as they are, by {@link matchKey}. */
  accepted: ReadonlySet<string>;
}

/**
 * What of a mod given up still collides at a place after the choices of
 * `then` (ADR 0080): parts the game needs or characters walk on, tied ones
 * and parts kept by hand. At its spots, where a part of the other mod is
 * left too; in a passage, every part left in the sections given up. Parts
 * the user said to leave as they are no longer count. Null when nothing is
 * left; the first side given up with something left otherwise.
 */
export function leftoversOf<P extends PlacePart>(
  then: Decisions,
  place: LeftoverPlace<P>,
): Leftovers<P> | null {
  const hidden = then.hiddenKeys;
  for (const side of [0, 1] as const) {
    const loser = place.sides[side];
    const givenUp = (decided: Decided) =>
      decided.type === "hidden" && decided.plugin.toLowerCase() === loser.toLowerCase();
    const decidedSpots = place.spots.filter((entry) => givenUp(decidedOf([entry], then.choices)));
    if (decidedSpots.length === 0) {
      continue;
    }
    const found = new Map<string, P>();
    const atSpots = new Set(
      stillColliding(
        decidedSpots.map(({ key, spot }) => ({
          key,
          parts: (spot.parts ?? []).map((part) => ({
            key: matchKey(part.object),
            plugin: part.object.plugin,
          })),
        })),
        loser,
        hidden,
      ).parts,
    );
    for (const { spot } of decidedSpots) {
      for (const part of spot.parts ?? []) {
        if (atSpots.has(matchKey(part.object))) {
          found.set(matchKey(part.object), place.partOf(part));
        }
      }
    }
    for (const section of place.sections ?? []) {
      const given =
        section.spots.length === 0 || givenUp(decidedOf([...section.spots], then.choices));
      for (const part of given ? section.parts[side] : []) {
        if (!hidden.has(matchKey(part.id))) {
          found.set(matchKey(part.id), part);
        }
      }
    }
    for (const key of place.accepted) {
      found.delete(key);
    }
    if (found.size > 0) {
      return { loser, side, parts: [...found.values()] };
    }
  }
  return null;
}

/** What the window knows besides the book that the decisions depend on. */
export interface DecisionWorld {
  /** The spots of the collisions, by finding. */
  spots: Map<string, MapSpot[]>;
  /** The findings set aside. */
  ignored?: ReadonlySet<string>;
  /** The passages the scan found. */
  passages?: readonly MapPassage[];
  /** The plugins of mods that are switched off (ADR 0061). */
  switchedOff?: SwitchedOffPlugin[];
  /** The patch in the game, as the scan read it. */
  installedPatch?: InstalledPatch | null;
}

/** The spots of a report, by finding. */
export function spotsByFinding(spots: readonly MapSpot[]): Map<string, MapSpot[]> {
  const found = new Map<string, MapSpot[]>();
  for (const spot of spots) {
    found.set(spot.finding, [...(found.get(spot.finding) ?? []), spot]);
  }
  return found;
}

/** What a report tells the decisions. */
export function worldOf(
  report: RenderedReport | null,
  ignored: ReadonlySet<string>,
): DecisionWorld {
  return {
    spots: spotsByFinding(report?.map.spots ?? []),
    ignored,
    passages: report?.map.passages ?? [],
    switchedOff: report?.switchedOffPlugins ?? [],
    installedPatch: report?.installedPatch ?? null,
  };
}

/**
 * The snapshot every page reads, from the book: the ticked decisions of
 * the patches that count, the pins, and without what waits for switched-off
 * mods. A page's change goes back with {@link bookWith}.
 */
export function snapshotOf(
  book: PatchBook | null,
  world: DecisionWorld,
): { decisions: Decisions; waiting: Waiting } {
  const { active, waiting } = splitWaiting(
    book === null ? {} : countedChoices(book),
    world.switchedOff ?? [],
    world.installedPatch ?? null,
  );
  return {
    decisions: decisionsOf({
      choices: active,
      spots: world.spots,
      ignored: world.ignored,
      passages: world.passages,
      waiting: waiting.choices,
      book,
    }),
    waiting,
  };
}

/**
 * The book with a page's change put in: `wanted` are the choices the page
 * wants, read from the snapshot; the waiting ones are put back, and the
 * change goes into the patch being edited (ADR 0070).
 */
export function bookWith(book: PatchBook, waiting: Waiting, wanted: PatchChoices): PatchBook {
  return withChange(book, countedChoices(book), withWaiting(wanted, waiting));
}

/**
 * The choices at the first start with several patches, or after the
 * settings were lost: what the patch in the game hides counts as decided,
 * and what the user chose since and kept wins over it, "shown again"
 * included. Objects an earlier reading of the patch kept under another
 * spelling go.
 */
export function choicesAtStart(
  report: RenderedReport,
  ignored: Set<string>,
  stored: PatchChoices | null,
): PatchChoices {
  const pairs = pairsOf(report, ignored);
  const installed = choicesOf(pairs, installedKeys(report), report.installedPatch?.objects ?? []);
  return withoutMisspelt(mergeChoices(installed, stored ?? {}), pairs);
}

/**
 * The book with the duel picks kept outside it before (ADR 0081): each pick
 * no patch has yet goes into the user's first patch, the one there was when
 * there were no others, and so counts only while that patch does.
 */
export function withLegacyDuels(book: PatchBook, legacy: Record<string, number>): PatchBook {
  const known = new Set(book.patches.flatMap((patch) => Object.keys(patch.decisions)));
  const missing = Object.entries(legacy).filter(([id]) => !known.has(`${DUEL}${id}`));
  const first = book.patches.find((patch) => !isForeign(patch));
  if (missing.length === 0 || first === undefined) {
    return book;
  }
  return {
    ...book,
    patches: book.patches.map((patch) =>
      patch.id === first.id
        ? {
            ...patch,
            decisions: {
              ...patch.decisions,
              ...Object.fromEntries(missing.map(([id, side]) => [`${DUEL}${id}`, String(side)])),
            },
          }
        : patch,
    ),
  };
}

/** The choices with a duel's side picked, or the pick taken back with null. */
export function chooseDuel(choices: PatchChoices, id: string, side: number | null): PatchChoices {
  const next = { ...choices };
  if (side === null) {
    delete next[`${DUEL}${id}`];
  } else {
    next[`${DUEL}${id}`] = String(side);
  }
  return next;
}
