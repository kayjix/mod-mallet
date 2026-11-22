// What the rows of the UI kit work out for themselves (ADR 0076, 0077): a
// mod's initials and colour for its badge, and where the arrow keys move
// the focus. Pure, so that a script can test it without a window.

/** File endings that make a plugin's or archive's name, not a mod's. */
const FILE_ENDING = /.(esp|esm|esl|bsa|ba2|dll)$/i;

/** Short words that do not start a mod's initials. */
const FILLER = new Set(["a", "an", "the", "of", "and", "for", "in", "on", "to", "s"]);

/**
 * Up to two letters that stand for a mod in its badge: the first letters
 * of its first two words that say something, or the first two letters of
 * a name of one word. Plugin and archive names lose their ending first.
 */
export function initialsOf(name: string): string {
  const base = name.replace(FILE_ENDING, "");
  const words = base
    .split(/[^\p{L}\p{N}]+/u)
    // "LegacyoftheDragonborn" and "BSHeartland" are two words written as one.
    .flatMap((word) => word.split(/(?<=\p{Ll})(?=\p{Lu})|(?<=\p{Lu})(?=\p{Lu}\p{Ll})/u))
    .filter((word) => word !== "");
  const telling = words.filter((word) => !FILLER.has(word.toLowerCase()));
  const chosen = telling.length > 0 ? telling : words;
  const [first, second] = chosen;
  if (first === undefined) {
    return "?";
  }
  const letters =
    second === undefined ? [...first].slice(0, 2).join("") : `${[...first][0]}${[...second][0]}`;
  return letters.toUpperCase();
}

/**
 * A hue for a mod's badge, from its name: the same mod has the same colour
 * everywhere, and different mods most likely differ. Only to tell badges
 * apart; it means nothing.
 */
export function hueOf(name: string): number {
  // FNV-1a over the UTF-16 units, folded into the circle of hues.
  let hash = 0x811c9dc5;
  for (let index = 0; index < name.length; index++) {
    hash ^= name.charCodeAt(index);
    hash = Math.imul(hash, 0x01000193) >>> 0;
  }
  return hash % 360;
}

/** A key that moves the focus between the rows of a list. */
export type RowKey = "ArrowDown" | "ArrowUp" | "Home" | "End";

/**
 * Where the focus goes from row `index` of `length` rows on a key: one row
 * down or up, stopping at the ends, or to the first or last row. -1 when
 * there are no rows.
 */
export function nextRow(index: number, length: number, key: RowKey): number {
  if (length <= 0) {
    return -1;
  }
  switch (key) {
    case "ArrowDown":
      return Math.min(length - 1, index + 1);
    case "ArrowUp":
      return Math.max(0, index - 1);
    case "Home":
      return 0;
    case "End":
      return length - 1;
  }
}

/** Whether a key moves between rows. */
export function isRowKey(key: string): key is RowKey {
  return key === "ArrowDown" || key === "ArrowUp" || key === "Home" || key === "End";
}
