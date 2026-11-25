// Tests that every page sees the same decisions (ADR 0081). Each test
// replays what the user does on one page and checks what the others show,
// through the selectors those pages read: the map's object card and 3D
// view, the collisions' worklist and passage panel, the guided review's
// stage, and the patch that is written. The window around them is played
// by the same functions the window uses: the book (ADR 0070), the split of
// waiting decisions (ADR 0061) and the merge with the patch in the game.
//
// Made up: a bunkhouse two mods build (a passage with two spots and a
// crate no spot names), three doors stacked in one doorway, and a lamp
// outside any collision.

import assert from "node:assert/strict";
import { test } from "node:test";
import {
  activePatch,
  bookFromChoices,
  countedChoices,
  withNewPatch,
  withPatch,
} from "../src/book.ts";
import {
  bookWith,
  choicesAtStart,
  chooseDuel,
  type DecisionWorld,
  decidedOf,
  decidedOfParts,
  keepSide,
  leftoversOf,
  mergeDoors,
  snapshotOf,
  spotsByFinding,
  withLegacyDuels,
} from "../src/decisions.ts";
import type { PatchBook } from "../src/generated/patch-book";
import { keepPassage } from "../src/passage-choices.ts";
import {
  keepChoice,
  matchKey,
  objectsOf,
  type PatchChoices,
  pairsOf,
  spotKey,
} from "../src/patch.ts";

const HOUSE = "House Mod.esp";
const TOWN = "Town Mod.esp";
const THIRD = "Third Mod.esp";
const PAIR = "objects.overlap-house-town";
const DOORS = "doors.stacked-doorway";

const part = (plugin: string, localId: number, name: string, hideable = true) => ({
  object: { plugin, localId },
  name,
  hideable,
  suggested: false,
});

const wall = part(HOUSE, 1, "Wall");
const roof = part(HOUSE, 2, "Roof");
const townWall = part(TOWN, 11, "Wall");
const ladder = part(TOWN, 12, "Ladder");
const crate = part(TOWN, 13, "Crate");
const doorA = part(HOUSE, 21, "Door");
const doorB = part(TOWN, 22, "Door");
const doorC = part(THIRD, 31, "Door");
const lamp = { plugin: TOWN, localId: 40 };

const location = { type: "exterior" as const, world: "Tamriel", x: 0, y: 0 };
const spotAt = (finding: string, parts: ReturnType<typeof part>[], doors?: unknown) => ({
  finding,
  parts,
  location,
  position: [0, 0, 0] as [number, number, number],
  console: "",
  objects: "",
  doors: doors ?? null,
});

const walls = spotAt(PAIR, [wall, townWall]);
const top = spotAt(PAIR, [roof, ladder]);
const doorObjects = [doorA, doorB, doorC].map((door) => door.object);
const plan = (keep: (typeof doorObjects)[number]) => ({
  keep,
  hide: doorObjects.filter((other) => other !== keep),
  changes: [],
  lost: [],
});
const group = {
  doors: [doorA, doorB, doorC].map((door) => ({
    object: door.object,
    plugin: door.object.plugin,
    name: door.name,
    front: true,
    leadsBack: true,
  })),
  plans: doorObjects.map(plan),
  flicker: true,
};
const doorway = spotAt(DOORS, [doorA, doorB, doorC], group);

const passage = {
  plugins: [HOUSE, TOWN] as [string, string],
  parts: [
    [wall, roof],
    [townWall, ladder, crate],
  ],
  min: [-100, -100, 0],
  max: [100, 100, 300],
  centre: [0, 0, 150],
  console: "",
  duplicates: 0,
};

// biome-ignore lint/suspicious/noExplicitAny: made-up spots carry only what the code reads.
const allSpots: any[] = [walls, top, doorway];
const finding = (id: string) => ({
  finding: { id, kind: id.split("-")[0], args: {}, severity: "warning" },
  headline: id,
  actions: [],
});
/** The report, as far as the decisions read it; `installed` the objects the patch in the game hides. */
function reportWith(installed: { plugin: string; localId: number }[] = [], off: string[] = []) {
  return {
    findings: [finding(PAIR), finding(DOORS)],
    map: { spots: allSpots, passages: [passage] },
    installedPatch: { objects: installed, patches: [] },
    switchedOffPlugins: off.map((plugin) => ({
      plugin,
      loaded: false,
      mod: { name: plugin.replace(/\.esp$/, "") },
    })),
  } as never;
}

const worldOf = (report = reportWith(), ignored = new Set<string>()): DecisionWorld => ({
  spots: spotsByFinding(allSpots),
  ignored,
  passages: [passage as never],
  // biome-ignore lint/suspicious/noExplicitAny: made-up report.
  switchedOff: (report as any).switchedOffPlugins,
  // biome-ignore lint/suspicious/noExplicitAny: made-up report.
  installedPatch: (report as any).installedPatch,
});

/**
 * The window: a book, what every page reads from it, and a page's change
 * put back in, as `App.tsx` does.
 */
class Window {
  book: PatchBook;
  world: DecisionWorld;
  constructor(book: PatchBook, world = worldOf()) {
    this.book = book;
    this.world = world;
  }
  get snapshot() {
    return snapshotOf(this.book, this.world);
  }
  get decisions() {
    return this.snapshot.decisions;
  }
  /** A page changes the choices it read. */
  change(next: (decisions: ReturnType<typeof snapshotOf>["decisions"]) => PatchChoices) {
    const { decisions, waiting } = this.snapshot;
    this.book = bookWith(this.book, waiting, next(decisions));
  }
  /** What the pages show of one object. */
  look(object: { plugin: string; localId: number }) {
    const d = this.decisions;
    return {
      // The map's object card, the collisions' card and the review's part list.
      card: d.isHidden(object),
      // The 3D views of the map, the collisions and the review draw it as a ghost.
      ghost: d.hiddenKeys.has(matchKey(object)),
      // The patch that is written hides it.
      patch: objectsOf(
        pairsOf(
          {
            findings: [finding(PAIR), finding(DOORS)],
            map: { spots: [...this.world.spots.values()].flat() },
          } as never,
          this.world.ignored as Set<string>,
        ),
        d.choices,
      )
        .map(matchKey)
        .includes(matchKey(object)),
      by: d.hiding(object)?.by ?? null,
    };
  }
}

const shown = { card: false, ghost: false, patch: false, by: null };
const hiddenBy = (by: string) => ({ card: true, ghost: true, patch: true, by });

const fresh = () => new Window(bookFromChoices({}));

test("an object hidden on the map is hidden in the collisions, the review and the patch, and shown again from any", () => {
  const window = fresh();
  // The map's object card hides the lamp, which no collision names, and the ladder.
  window.change((d) => d.hide([lamp]));
  window.change((d) => d.hide([ladder.object]));
  assert.deepEqual(window.look(lamp), hiddenBy("own"));
  assert.deepEqual(window.look(ladder.object), hiddenBy("own"));
  // The worklist sees the spot of the ladder decided by it: the ladder's plugin goes there.
  assert.equal(window.decisions.decisionOf(top as never), TOWN);
  // Shown again from the collisions' card: every page shows it.
  window.change((d) => d.show([ladder.object]));
  assert.deepEqual(window.look(ladder.object), shown);
  assert.equal(window.decisions.decisionOf(top as never), undefined);
  // And the lamp from the review's part list.
  window.change((d) => d.show([lamp]));
  assert.deepEqual(window.look(lamp), shown);
  assert.equal(window.decisions.shownAgain(lamp), true);
});

test("a spot decided in the collisions hides its parts on the map, and the map's card shows them again", () => {
  const window = fresh();
  window.change((d) => ({ ...d.choices, [spotKey(walls as never) as string]: TOWN }));
  // Not yet the whole passage: the spot's decision hides it.
  assert.deepEqual(window.look(townWall.object), hiddenBy("spot"));
  window.change((d) => d.show([townWall.object]));
  assert.deepEqual(window.look(townWall.object), shown);
  // The spot keeps both now: decided, not open.
  assert.equal(window.decisions.decisionOf(walls as never), null);
});

test("a passage decided, one part shown again from the map: the passage counts as decided with an exception", () => {
  const window = fresh();
  const cluster = {
    plugins: passage.plugins,
    spots: [walls, top].map((spot) => ({ key: spotKey(spot as never) as string })),
    passage: { passage: passage as never },
  };
  window.change((d) => keepPassage(d.choices, d.spots, cluster, null, 0));
  assert.deepEqual(window.decisions.decisionOfPassage(passage as never), {
    type: "hidden",
    plugin: TOWN,
  });
  for (const gone of [townWall, ladder, crate]) {
    assert.deepEqual(window.look(gone.object), hiddenBy("passage"), gone.name);
  }
  // The ladder, a part of a spot, shown again from the map's object card.
  window.change((d) => d.show([ladder.object]));
  assert.deepEqual(window.look(ladder.object), shown);
  assert.deepEqual(window.decisions.decisionOfPassage(passage as never), {
    type: "mixed",
    open: 0,
  });
  assert.deepEqual(window.look(townWall.object), hiddenBy("passage"));
  assert.deepEqual(window.look(crate.object), hiddenBy("passage"));
});

test("a part of a passage no spot names, shown again, is an exception too", () => {
  const window = fresh();
  const cluster = {
    plugins: passage.plugins,
    spots: [walls, top].map((spot) => ({ key: spotKey(spot as never) as string })),
    passage: { passage: passage as never },
  };
  window.change((d) => keepPassage(d.choices, d.spots, cluster, null, 0));
  window.change((d) => d.show([crate.object]));
  assert.deepEqual(window.look(crate.object), shown);
  // Both the collisions' passage panel and the review read the same.
  const d = window.decisions;
  const members = [walls, top].map((spot) => ({ spot: spot as never, finding: PAIR }));
  assert.deepEqual(d.decisionOfPassage(passage as never), { type: "mixed", open: 0 });
  assert.deepEqual(decidedOf(members, d.choices, passage as never), { type: "mixed", open: 0 });
  // Kept both on the map afterwards, and a part hidden by hand: an exception as well.
  window.change((d) => keepPassage(d.choices, d.spots, cluster, null, null));
  assert.deepEqual(window.decisions.decisionOfPassage(passage as never), { type: "kept" });
  window.change((d) => d.hide([roof.object]));
  assert.deepEqual(window.decisions.decisionOfPassage(passage as never), {
    type: "mixed",
    open: 0,
  });
});

test("doors hidden one by one through the object card merge into the last, and 'All of them stay' undoes every hiding", () => {
  const window = fresh();
  const key = spotKey(doorway as never) as string;
  // One door hidden from the map's card: on its own, the others stay.
  window.change((d) => d.hide([doorB.object]));
  assert.deepEqual(window.look(doorB.object), hiddenBy("own"));
  // A second one: one door is left, so the doors are merged into it.
  window.change((d) => d.hide([doorC.object]));
  assert.equal(window.decisions.decisionOf(doorway as never), keepChoice(doorA.object));
  assert.deepEqual(window.look(doorB.object), hiddenBy("door"));
  assert.deepEqual(window.look(doorC.object), hiddenBy("door"));
  assert.deepEqual(window.look(doorA.object), shown);
  // The kept door hidden as well, from the collisions' card: every door is gone.
  window.change((d) => d.hide([doorA.object]));
  for (const door of doorObjects) {
    assert.equal(window.look(door).card, true);
  }
  // "All of them stay" in the door panel.
  window.change((d) => mergeDoors(d.choices, key, group as never, null, d.spots));
  for (const door of doorObjects) {
    assert.deepEqual(window.look(door), shown);
  }
  assert.equal(window.decisions.decisionOf(doorway as never), null);
});

test("doors merged in the panel, a hidden door shown again from the map: the others stay hidden on their own", () => {
  const window = fresh();
  const key = spotKey(doorway as never) as string;
  window.change((d) => mergeDoors(d.choices, key, group as never, plan(doorA.object), d.spots));
  window.change((d) => d.show([doorB.object]));
  assert.deepEqual(window.look(doorB.object), shown);
  assert.deepEqual(window.look(doorC.object), hiddenBy("own"));
  assert.equal(window.decisions.decisionOf(doorway as never), null);
});

test("switching the patch being edited changes what every page shows, and where a change goes", () => {
  const first = bookFromChoices({ [`object:${lamp.plugin}|${lamp.localId}`]: lamp.plugin });
  const firstId = first.active;
  const { book, id: secondId } = withNewPatch(first, "Second");
  const window = new Window(book);
  window.change((d) => d.hide([ladder.object]));
  // Each object says which patch hides it.
  assert.deepEqual(
    window.decisions.hiding(lamp)?.patches.map((patch) => [patch.id, patch.edited]),
    [[firstId, false]],
  );
  assert.deepEqual(
    window.decisions.hiding(ladder.object)?.patches.map((patch) => [patch.id, patch.edited]),
    [[secondId, true]],
  );
  // The first patch switched off: the lamp is shown on every page.
  window.book = withPatch(window.book, firstId, (patch) => ({ ...patch, off: true }));
  assert.deepEqual(window.look(lamp), shown);
  // Edited again, it counts while switched off, so that its changes show.
  window.book = { ...window.book, active: firstId };
  assert.equal(activePatch(window.book)?.id, firstId);
  assert.deepEqual(window.look(lamp), hiddenBy("own"));
  assert.deepEqual(window.look(ladder.object), hiddenBy("own"));
  // Shown again from the review: the change goes into the first patch.
  window.change((d) => d.show([lamp]));
  assert.deepEqual(window.look(lamp), shown);
  const firstNow = window.book.patches.find((patch) => patch.id === firstId);
  assert.equal(firstNow?.decisions[`object:${lamp.plugin}|${lamp.localId}`], null);
});

test("after a restart the patch in the game hides nothing the user showed again", () => {
  // The patch in the game hides the lamp, the ladder and the crate.
  const report = reportWith([
    lamp,
    ladder.object,
    crate.object,
    // As the patch writes plugins: in lower case.
    { plugin: TOWN.toLowerCase(), localId: townWall.object.localId },
  ]);
  const window = new Window(bookFromChoices(choicesAtStart(report, new Set(), null)));
  for (const object of [lamp, ladder.object, crate.object, townWall.object]) {
    assert.equal(window.look(object).card, true, matchKey(object));
  }
  // Shown again on the map; the book was kept as choices before ADR 0070.
  window.change((d) => d.show([ladder.object, lamp]));
  const kept: PatchChoices = countedChoices(window.book);
  const restarted = new Window(bookFromChoices(choicesAtStart(report, new Set(), kept)));
  for (const object of [ladder.object, lamp]) {
    assert.deepEqual(restarted.look(object), shown, matchKey(object));
  }
  assert.equal(restarted.look(crate.object).card, true);
  assert.equal(restarted.look(townWall.object).card, true);
  // Kept as a book, read back: the same on every page.
  const reread = new Window(JSON.parse(JSON.stringify(window.book)));
  for (const object of [lamp, ladder.object, crate.object, townWall.object]) {
    assert.deepEqual(reread.look(object), window.look(object), matchKey(object));
  }
});

test("a finding set aside hides nothing on any page, nor in the patch", () => {
  const window = fresh();
  const key = spotKey(doorway as never) as string;
  window.change((d) => mergeDoors(d.choices, key, group as never, plan(doorA.object), d.spots));
  assert.equal(window.look(doorB.object).card, true);
  window.world = worldOf(reportWith(), new Set([DOORS]));
  assert.deepEqual(window.look(doorB.object), shown);
});

test("a decision about a switched-off mod waits: no page shows it hidden, and each can tell", () => {
  const window = fresh();
  window.change((d) => d.hide([doorC.object]));
  assert.equal(window.look(doorC.object).card, true);
  window.world = worldOf(reportWith([], [THIRD]));
  assert.deepEqual(window.look(doorC.object), shown);
  assert.equal(window.decisions.waits(doorC.object), true);
  // A change made meanwhile keeps the waiting decision.
  window.change((d) => d.hide([lamp]));
  window.world = worldOf();
  assert.equal(window.look(doorC.object).card, true);
  assert.equal(window.look(lamp).card, true);
});

test("a new patch starts with every file duel open, and the old patch keeps its picks", () => {
  // Picks made before the duels were part of the patches (ADR 0081), and a
  // second patch started afresh with the first switched off.
  const first = withLegacyDuels(bookFromChoices({}), { "textures-a": 0, "meshes-b": 1 });
  const firstId = first.active;
  assert.deepEqual(new Window(first).decisions.duels, { "textures-a": 0, "meshes-b": 1 });
  const { book } = withNewPatch(first, "Fresh");
  const window = new Window(withPatch(book, firstId, (patch) => ({ ...patch, off: true })));
  // The asset page, the overview and the patch page read the same: nothing decided.
  assert.deepEqual(window.decisions.duels, {});
  // A pick now goes into the new patch, and says so.
  window.change((d) => chooseDuel(d.choices, "textures-a", 1));
  assert.deepEqual(window.decisions.duels, { "textures-a": 1 });
  assert.deepEqual(
    window.decisions.patchesOf("duel:textures-a").map((patch) => [patch.name, patch.edited]),
    [["Fresh", true]],
  );
  // The old patch kept its own picks.
  const old = window.book.patches.find((patch) => patch.id === firstId);
  assert.deepEqual(old?.decisions, { "duel:textures-a": "0", "duel:meshes-b": "1" });
  // Switched on again, its picks count, the one being edited wins where both pick.
  window.book = withPatch(window.book, firstId, (patch) => ({ ...patch, off: false }));
  assert.deepEqual(window.decisions.duels, { "textures-a": 1, "meshes-b": 1 });
  // Taking the pick back on the asset page leaves the old patch's pick out too.
  window.change((d) => chooseDuel(d.choices, "meshes-b", null));
  assert.equal(window.decisions.duels["meshes-b"], undefined);
});

test("picks kept outside the patches go into the first patch once", () => {
  const book = bookFromChoices({ "duel:kept": "1" });
  const moved = withLegacyDuels(book, { kept: 0, new: 1 });
  // A pick a patch already has stays as the patch has it.
  assert.deepEqual(moved.patches[0]?.decisions, { "duel:kept": "1", "duel:new": "1" });
  assert.equal(withLegacyDuels(moved, { kept: 0, new: 1 }), moved);
});

// The ladder as a part the game needs, which the patch may not hide.
const neededLadder = { ...ladder, hideable: false };
const topNeeded = spotAt(PAIR, [roof, neededLadder]);
const neededWorld = () => ({ ...worldOf(), spots: spotsByFinding([walls, topNeeded, doorway]) });

/** The bunkhouse as Go through it reads it: its two spots, and its sections. */
function reviewPlace(accepted: Set<string> = new Set()) {
  const entries = [walls, topNeeded].map((spot) => ({
    key: spotKey(spot as never) as string,
    spot: spot as never,
    finding: PAIR,
  }));
  return {
    sides: passage.plugins,
    spots: entries,
    partOf: (part: { object: { plugin: string; localId: number } }) => ({ id: part.object }),
    // The walls with their spot, and a section of the crate alone, without spots.
    sections: [
      {
        spots: [entries[0] as (typeof entries)[number]],
        parts: [[{ id: wall.object }], [{ id: townWall.object }]] as const,
      },
      { spots: [], parts: [[], [{ id: crate.object }]] as const },
    ],
    accepted,
  };
}

const keys = [walls, top].map((spot) => spotKey(spot as never) as string);

test("a side kept in the review shows on every page, and the leftover line names what still collides", () => {
  const window = new Window(bookFromChoices({}), neededWorld());
  // House kept; the ladder is one the game needs, so it does not go.
  window.change((d) =>
    d.keepSide({
      kept: [wall.object, roof.object],
      gone: [townWall.object, crate.object],
      spots: keys,
      loser: TOWN,
    }),
  );
  // The map, the collisions and the patch agree.
  assert.deepEqual(window.look(townWall.object), hiddenBy("passage"));
  assert.deepEqual(window.look(crate.object), hiddenBy("passage"));
  assert.deepEqual(window.look(ladder.object), shown);
  assert.deepEqual(window.decisions.decisionOfPassage(passage as never), {
    type: "hidden",
    plugin: TOWN,
  });
  // The review's leftover line: the ladder still meets the roof.
  const left = leftoversOf(window.decisions, reviewPlace());
  assert.equal(left?.loser, TOWN);
  assert.equal(left?.side, 1);
  assert.deepEqual(
    left?.parts.map((part) => matchKey(part.id)),
    [matchKey(ladder.object)],
  );
  // Left as it is: the line goes.
  assert.equal(
    leftoversOf(window.decisions, reviewPlace(new Set([matchKey(ladder.object)]))),
    null,
  );
  // Hidden from the map's object card after all: the line goes there too.
  window.change((d) => d.hide([ladder.object]));
  assert.equal(leftoversOf(window.decisions, reviewPlace()), null);
  // The crate shown again on the map: the passage has an exception. The
  // crate is all of its section, which is now kept as "Keep both" there
  // keeps it, so it is no longer given up and nothing is left over.
  window.change((d) => d.show([crate.object]));
  assert.equal(leftoversOf(window.decisions, reviewPlace()), null);
  assert.deepEqual(
    decidedOfParts(window.decisions, [crate.object], { type: "hidden", plugin: TOWN }),
    { type: "kept" },
  );
  assert.deepEqual(window.decisions.decisionOfPassage(passage as never), {
    type: "mixed",
    open: 0,
  });
});

test("both kept in the review: nothing hidden on any page, nothing left over", () => {
  const window = new Window(bookFromChoices({}), neededWorld());
  window.change((d) =>
    d.keepSide({ kept: [ladder.object], gone: [ladder.object], spots: keys, loser: TOWN }),
  );
  window.change((d) =>
    d.keepSide({
      kept: [wall.object, roof.object, townWall.object, ladder.object, crate.object],
      gone: [],
      spots: keys,
      loser: null,
    }),
  );
  for (const part of [wall, roof, townWall, ladder, crate]) {
    assert.deepEqual(window.look(part.object), shown, part.name);
  }
  assert.deepEqual(window.decisions.decisionOfPassage(passage as never), { type: "kept" });
  assert.equal(leftoversOf(window.decisions, reviewPlace()), null);
});

test("asking 'what if' keeps the world: set-aside findings and waiting decisions count as they do now", () => {
  const window = fresh();
  window.change((d) => d.hide([doorC.object]));
  window.world = worldOf(reportWith([], [THIRD]), new Set([DOORS]));
  const now = window.decisions;
  // A review's decision not yet made, as the leftover line asks it.
  const then = now.whatIf(
    keepSide(now.choices, now.spots, {
      kept: [],
      gone: [townWall.object],
      spots: [keys[0] as string],
      loser: TOWN,
    }),
  );
  assert.equal(then.isHidden(townWall.object), true);
  assert.equal(now.isHidden(townWall.object), false);
  // The door's decision still waits, and its finding is still set aside.
  assert.equal(then.isHidden(doorC.object), false);
  assert.equal(then.waits(doorC.object), true);
  assert.equal(then.spots.has(DOORS), false);
  assert.equal(now.whatIf(now.choices), now);
});

// A passage's other parts as a section of their own, without spots, whose
// only part of the side given up is tied in: the crate, as the review would
// ask the game about it. Deciding the section must be recorded and read
// back, whatever it can hide.
function crateSection(accepted: Set<string> = new Set()) {
  const entries = [walls, top].map((spot) => ({
    key: spotKey(spot as never) as string,
    spot: spot as never,
    finding: PAIR,
  }));
  return {
    sides: passage.plugins,
    spots: entries,
    partOf: (part: { object: { plugin: string; localId: number } }) => ({ id: part.object }),
    sections: [
      {
        spots: entries,
        parts: [
          [{ id: wall.object }, { id: roof.object }],
          [{ id: townWall.object }, { id: ladder.object }],
        ] as const,
      },
      { spots: [], parts: [[], [{ id: crate.object }]] as const },
    ],
    accepted,
  };
}

test("a section of a passage's other parts, its only part tied: kept, both kept, undone, hidden anyway", () => {
  const window = fresh();
  const placeSpots = [walls, top].map((spot) => ({ spot: spot as never, finding: PAIR }));
  // What the review shows for the place and for the crate's section.
  const place = () => decidedOf(placeSpots, window.decisions.choices, passage as never);
  const section = () => decidedOfParts(window.decisions, [crate.object], place());

  // The place decided: House kept, the crate is tied in and stays.
  window.change((d) =>
    d.keepSide({
      kept: [wall.object, roof.object],
      gone: [townWall.object, ladder.object],
      spots: keys,
      loser: TOWN,
      stays: [crate.object],
    }),
  );
  assert.deepEqual(place(), { type: "hidden", plugin: TOWN });
  // The section goes with the place: decided, though its crate stays.
  assert.deepEqual(section(), { type: "hidden", plugin: TOWN });
  assert.deepEqual(window.look(crate.object), shown);
  assert.deepEqual(
    leftoversOf(window.decisions, crateSection())?.parts.map((part) => matchKey(part.id)),
    [matchKey(crate.object)],
  );
  const history = [window.book];

  // "Keep both" for the section: recorded, the section says both stay, nothing is left over.
  window.change((d) => d.keepSide({ kept: [crate.object], gone: [], spots: [], loser: null }));
  assert.deepEqual(section(), { type: "kept" });
  assert.equal(window.decisions.shownAgain(crate.object), true);
  assert.equal(leftoversOf(window.decisions, crateSection()), null);
  // For the place, the crate kept is an exception.
  assert.deepEqual(place(), { type: "mixed", open: 0 });
  history.push(window.book);

  // "Keep House's" for the section: nothing can go, the crate's own choice
  // goes and the section is decided with the place again.
  window.change((d) =>
    d.keepSide({ kept: [], gone: [], spots: [], loser: TOWN, stays: [crate.object] }),
  );
  assert.deepEqual(section(), { type: "hidden", plugin: TOWN });
  assert.deepEqual(place(), { type: "hidden", plugin: TOWN });
  assert.equal(window.decisions.shownAgain(crate.object), false);
  assert.deepEqual(window.look(crate.object), shown);

  // Undone: back to both kept, then to the place's decision alone.
  window.book = history.pop() as PatchBook;
  assert.deepEqual(section(), { type: "kept" });
  window.book = history.pop() as PatchBook;
  assert.deepEqual(section(), { type: "hidden", plugin: TOWN });

  // "Hide anyway": the crate goes, nothing is left over.
  window.change((d) => d.hide([crate.object]));
  assert.deepEqual(section(), { type: "hidden", plugin: TOWN });
  assert.equal(window.decisions.isHidden(crate.object), true);
  assert.equal(leftoversOf(window.decisions, crateSection()), null);
});

test("a section without spots in a place still open: its own choices decide it, a tied part alone cannot", () => {
  const window = fresh();
  const placeSpots = [walls, top].map((spot) => ({ spot: spot as never, finding: PAIR }));
  const place = () => decidedOf(placeSpots, window.decisions.choices, passage as never);
  const section = () => decidedOfParts(window.decisions, [crate.object], place());
  assert.deepEqual(section(), { type: "open" });
  // House kept there, the crate tied: nothing to hide, nothing to record.
  const before = window.decisions.choices;
  window.change((d) =>
    d.keepSide({ kept: [], gone: [], spots: [], loser: TOWN, stays: [crate.object] }),
  );
  assert.deepEqual(window.decisions.choices, before);
  assert.deepEqual(section(), { type: "open" });
  // Both kept: recorded.
  window.change((d) => d.keepSide({ kept: [crate.object], gone: [], spots: [], loser: null }));
  assert.deepEqual(section(), { type: "kept" });
  // House kept with the crate free to go: recorded as Town's hidden.
  window.change((d) => d.keepSide({ kept: [], gone: [crate.object], spots: [], loser: TOWN }));
  assert.deepEqual(section(), { type: "hidden", plugin: TOWN });
});
