// The few words of the shared UI kit itself (ADR 0077): what a screen
// reader hears for a note's sign, the kit's undo button, and the words
// around a sorting button and a note that opens.

import type { Lang } from "../i18n";

/** The tones a note, a chip or a status can have. */
export type Tone = "info" | "ok" | "warning" | "danger" | "neutral";

export interface UiTexts {
  /** What each tone's sign means, read before the note's text. */
  tone: Record<Tone, string>;
  /** The button that takes a just made change back. */
  undo: string;
  /** The sorting button's tooltip: the order now, and that a click changes it. */
  sortedBy: (order: string) => string;
  /** On a note that opens: to read the rest. */
  more: string;
}

export const uiTexts: Record<Lang, UiTexts> = {
  en: {
    tone: {
      info: "Note",
      ok: "Done",
      warning: "Careful",
      danger: "Problem",
      neutral: "",
    },
    undo: "Undo",
    sortedBy: (order) => `Sorted: ${order}. Click for the next order.`,
    more: "More",
  },
  de: {
    tone: {
      info: "Hinweis",
      ok: "Erledigt",
      warning: "Vorsicht",
      danger: "Problem",
      neutral: "",
    },
    undo: "Rückgängig",
    sortedBy: (order) => `Sortiert: ${order}. Klick für die nächste Reihenfolge.`,
    more: "Mehr",
  },
};
