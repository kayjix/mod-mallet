// The user's patches as a book (ADR 0070): which patches count, in which
// order, which one is being edited, and how a page's change goes into it.
// Only what the book itself holds; what the patches do in the game is in
// `patch-book.ts`, which also offers everything here.

import type { BookPatch, PatchBook } from "./generated/patch-book";

/** The choices of the pages: per finding, spot, object and so on. */
type PatchChoices = Record<string, string | null>;

/** The prefix of a pin among the choices; a pin is no decision. */
export const PIN = "pin:";

/** The prefix of a file choice of Asset collisions among a patch's decisions. */
export const DUEL = "duel:";

/** The name the first patch gets, as patches had before there were several. */
export const FIRST_PATCH_NAME = "Mod Mallet Patch";

/** A new identity for a patch, different from those in the book. */
export function newPatchId(book: PatchBook | null): string {
  const taken = new Set((book?.patches ?? []).map((patch) => patch.id));
  for (let attempt = 0; ; attempt += 1) {
    const id = `p${Date.now().toString(36)}${attempt === 0 ? "" : `-${attempt}`}`;
    if (!taken.has(id)) {
      return id;
    }
  }
}

/** A name not used by another patch of the book: `wanted`, or with a number after it. */
export function freeName(book: PatchBook, wanted: string, except?: string): string {
  const taken = new Set(
    book.patches.filter((patch) => patch.id !== except).map((patch) => patch.name.toLowerCase()),
  );
  const base = wanted.trim() === "" ? FIRST_PATCH_NAME : wanted.trim();
  if (!taken.has(base.toLowerCase())) {
    return base;
  }
  for (let number = 2; ; number += 1) {
    const name = `${base} (${number})`;
    if (!taken.has(name.toLowerCase())) {
      return name;
    }
  }
}

/**
 * The book made the first time from the choices as they were kept before
 * there were several patches: one patch with every decision, the pins
 * apart.
 */
export function bookFromChoices(choices: PatchChoices): PatchBook {
  const decisions: PatchChoices = {};
  const pins: string[] = [];
  for (const [key, value] of Object.entries(choices)) {
    if (key.startsWith(PIN)) {
      pins.push(key);
    } else {
      decisions[key] = value;
    }
  }
  const id = newPatchId(null);
  return {
    patches: [{ id, name: FIRST_PATCH_NAME, decisions, unticked: [] }],
    active: id,
    pins,
  };
}

/** Whether a patch is somebody else's, whose decisions the window never changes. */
export const isForeign = (patch: BookPatch) => patch.foreign === true;

/** The patch new decisions go to: the active one, or else the first of the user's own. */
export function activePatch(book: PatchBook): BookPatch | undefined {
  return (
    book.patches.find((patch) => patch.id === book.active && !isForeign(patch)) ??
    book.patches.find((patch) => !isForeign(patch))
  );
}

/** A patch's decisions that are ticked. */
export function tickedOf(patch: BookPatch): PatchChoices {
  const unticked = new Set(patch.unticked ?? []);
  if (unticked.size === 0) {
    return patch.decisions;
  }
  return Object.fromEntries(Object.entries(patch.decisions).filter(([key]) => !unticked.has(key)));
}

/**
 * The patches that count, in the order in which they count: those switched
 * on, loaded ones first, then the user's own, and the patch being edited
 * last, whether it is switched on or not. On the same decision the later
 * wins, so the views show what the user is editing.
 */
export function countedPatches(book: PatchBook): BookPatch[] {
  const edited = activePatch(book);
  const on = book.patches.filter((patch) => patch.off !== true && patch.id !== edited?.id);
  return [
    ...on.filter(isForeign),
    ...on.filter((patch) => !isForeign(patch)),
    ...(edited === undefined ? [] : [edited]),
  ];
}

/**
 * The book with a copy of a patch to work on from it: the copy is the
 * user's own, becomes the patch being edited, and the original is switched
 * off here, so that what the copy changes shows at once.
 */
export function withDuplicate(
  book: PatchBook,
  id: string,
  name: string,
): { book: PatchBook; id: string } {
  const original = book.patches.find((patch) => patch.id === id);
  if (original === undefined) {
    return { book, id: book.active };
  }
  const copyId = newPatchId(book);
  const copy: BookPatch = {
    id: copyId,
    name: freeName(book, name),
    decisions: { ...original.decisions },
    unticked: [...(original.unticked ?? [])],
  };
  return {
    id: copyId,
    book: {
      ...book,
      active: copyId,
      patches: [
        ...book.patches.map((patch) => (patch.id === id ? { ...patch, off: true } : patch)),
        copy,
      ],
    },
  };
}

/** The choices every page reads: the pins, and the ticked decisions of every patch that counts. */
export function countedChoices(book: PatchBook): PatchChoices {
  const choices: PatchChoices = {};
  for (const pin of book.pins ?? []) {
    choices[pin] = "pinned";
  }
  for (const patch of countedPatches(book)) {
    Object.assign(choices, tickedOf(patch));
  }
  return choices;
}

/** A copy of a patch with its decisions and unticked keys copied too, to change. */
function copyOf(patch: BookPatch): BookPatch {
  return { ...patch, decisions: { ...patch.decisions }, unticked: [...(patch.unticked ?? [])] };
}

/**
 * The book with a page's change put in: `before` is what the page read,
 * `after` what it wants. Every decision made or changed goes to the patch
 * being edited, which counts last and so wins in the views. A decision
 * taken back is taken out of the patch being edited, and unticked in any
 * other patch that still makes it, which leaves that patch's decisions as
 * they are (ADR 0070).
 */
export function withChange(book: PatchBook, before: PatchChoices, after: PatchChoices): PatchBook {
  const keys = new Set([...Object.keys(before), ...Object.keys(after)]);
  const changed = [...keys].filter(
    (key) => before[key] !== after[key] || key in before !== key in after,
  );
  if (changed.length === 0) {
    return book;
  }
  const patches = book.patches.map(copyOf);
  let pins = [...(book.pins ?? [])];
  const byId = new Map(patches.map((patch) => [patch.id, patch]));
  const counted = countedPatches({ ...book, patches }).map((patch) => byId.get(patch.id) ?? patch);
  const edited = activePatch({ ...book, patches });
  for (const key of changed) {
    if (key.startsWith(PIN)) {
      pins = pins.filter((pin) => pin !== key);
      if (key in after) {
        pins.push(key);
      }
      continue;
    }
    if (key in after) {
      if (edited === undefined) {
        continue;
      }
      edited.decisions[key] = after[key] ?? null;
      edited.unticked = (edited.unticked ?? []).filter((unticked) => unticked !== key);
      continue;
    }
    for (const patch of counted) {
      if (!(key in patch.decisions) || (patch.unticked ?? []).includes(key)) {
        continue;
      }
      if (patch.id === edited?.id) {
        delete patch.decisions[key];
      } else {
        patch.unticked = [...(patch.unticked ?? []), key];
      }
    }
  }
  return { ...book, patches, pins };
}

/** The book with one patch changed by `change`. */
export function withPatch(
  book: PatchBook,
  id: string,
  change: (patch: BookPatch) => BookPatch,
): PatchBook {
  return {
    ...book,
    patches: book.patches.map((patch) => (patch.id === id ? change(copyOf(patch)) : patch)),
  };
}

/** The book with a new empty patch of the user's own, which becomes the active one. */
export function withNewPatch(book: PatchBook, name: string): { book: PatchBook; id: string } {
  const id = newPatchId(book);
  return {
    id,
    book: {
      ...book,
      active: id,
      patches: [...book.patches, { id, name: freeName(book, name), decisions: {}, unticked: [] }],
    },
  };
}

/** The book without a patch. Another of the user's own becomes active; one is made if none is left. */
export function withoutPatch(book: PatchBook, id: string): PatchBook {
  const patches = book.patches.filter((patch) => patch.id !== id);
  const rest = { ...book, patches };
  if (patches.some((patch) => !isForeign(patch))) {
    return { ...rest, active: activePatch(rest)?.id ?? rest.active };
  }
  return withNewPatch(rest, FIRST_PATCH_NAME).book;
}

/** The book with decisions of one patch ticked or unticked. */
export function withTicked(
  book: PatchBook,
  id: string,
  keys: string[],
  ticked: boolean,
): PatchBook {
  return withPatch(book, id, (patch) => {
    const rest = (patch.unticked ?? []).filter((key) => !keys.includes(key));
    return { ...patch, unticked: ticked ? rest : [...rest, ...keys] };
  });
}

/**
 * The book with decisions moved or copied from one patch to another. A
 * patch by someone else gives copies only; nothing goes into one.
 */
export function withMoved(
  book: PatchBook,
  from: string,
  to: string,
  keys: string[],
  copy: boolean,
): PatchBook {
  const source = book.patches.find((patch) => patch.id === from);
  const target = book.patches.find((patch) => patch.id === to);
  if (source === undefined || target === undefined || isForeign(target) || from === to) {
    return book;
  }
  const keep = copy || isForeign(source);
  let next = withPatch(book, to, (patch) => {
    for (const key of keys) {
      if (key in source.decisions) {
        patch.decisions[key] = source.decisions[key] ?? null;
      }
    }
    patch.unticked = (patch.unticked ?? []).filter((key) => !keys.includes(key));
    return patch;
  });
  if (!keep) {
    next = withPatch(next, from, (patch) => {
      for (const key of keys) {
        delete patch.decisions[key];
      }
      patch.unticked = (patch.unticked ?? []).filter((key) => !keys.includes(key));
      return patch;
    });
  }
  return next;
}

/** The book with a loaded patch added, as somebody else's when it names an author. */
export function withLoaded(
  book: PatchBook,
  loaded: {
    archive: string;
    name: string;
    author: string | null;
    version: string | null;
    decisions: PatchChoices;
  },
): { book: PatchBook; id: string } {
  const id = newPatchId(book);
  const foreign = loaded.author !== null && loaded.author.trim() !== "";
  const patch: BookPatch = {
    id,
    name: freeName(book, loaded.name),
    author: loaded.author,
    version: loaded.version,
    loadedFrom: loaded.archive,
    foreign,
    decisions: { ...loaded.decisions },
    unticked: [],
  };
  return {
    id,
    book: { ...book, patches: [...book.patches, patch], active: foreign ? book.active : id },
  };
}
