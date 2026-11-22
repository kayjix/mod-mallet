// What the magic wand's selection offers (ADR 0055): the parts it took
// along are hidden or visible, and each state has its own way on: hidden
// parts are shown again, visible parts are hidden. Pure, so that a script
// can test it without a view.

/** The least a part must say for the wand to offer something for it. */
export interface WandPart {
  /** Whether the game cannot do without the part, so that it is not hidden. */
  anchored?: boolean;
}

/** A selection of the wand, told apart by what the patch does with each part. */
export interface PartsSplit<T> {
  /** The parts the patch hides now. */
  hidden: T[];
  /** The parts that are there now. */
  visible: T[];
  /** Of the visible ones, those that may be hidden. */
  hideable: T[];
  /** How many of the visible ones the game relies on and which stay. */
  tied: number;
}

/** Tells the parts of a selection apart by whether `isHidden` says the patch hides them. */
export function splitParts<T extends WandPart>(
  parts: readonly T[],
  isHidden: (part: T) => boolean,
): PartsSplit<T> {
  const hidden: T[] = [];
  const visible: T[] = [];
  for (const part of parts) {
    (isHidden(part) ? hidden : visible).push(part);
  }
  const hideable = visible.filter((part) => !part.anchored);
  return { hidden, visible, hideable, tied: visible.length - hideable.length };
}

/** One thing the selection can be asked to do. */
export interface WandOffer {
  direction: "show" | "hide";
  /** How many parts it takes. */
  count: number;
  /** Whether it is the way on from the picked object's own state. */
  primary: boolean;
}

/**
 * What to offer for a split selection, the way on from the picked object's
 * state first: from a hidden object showing the hidden parts again, from a
 * visible one hiding the visible parts. The other way is offered only
 * where some parts are in the other state, so a selection of one state
 * has one button, a mixed one two.
 */
export function offersFor<T>(split: PartsSplit<T>, pickedHidden: boolean): WandOffer[] {
  const show: WandOffer = {
    direction: "show",
    count: split.hidden.length,
    primary: pickedHidden,
  };
  const hide: WandOffer = {
    direction: "hide",
    count: split.hideable.length,
    primary: !pickedHidden,
  };
  const ordered = pickedHidden ? [show, hide] : [hide, show];
  return ordered.filter((offer) => offer.count > 0);
}
