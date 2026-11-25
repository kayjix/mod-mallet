// Where the arrow keys move the focus between the rows of the UI kit
// (ADR 0076, 0077). Pure, so that a script can test it without a window.

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
