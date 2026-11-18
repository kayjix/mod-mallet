// The texts of the toolbox page that the engine does not render, kept apart
// from `i18n.ts` so that the page can be changed on its own.

import type { Lang } from "./i18n";

export interface ToolboxTexts {
  /** The heading of the folded list of optional tools, with their number. */
  optional: (count: number) => string;
  /** What the optional tools have in common. */
  optionalHint: string;
}

export const toolboxTexts: Record<Lang, ToolboxTexts> = {
  en: {
    optional: (count) => `Optional (${count})`,
    optionalHint: "Not needed to play. Each says what it is for.",
  },
  de: {
    optional: (count) => `Optional (${count})`,
    optionalHint: "Zum Spielen nicht nötig. Bei jedem steht, wofür es gut ist.",
  },
};
