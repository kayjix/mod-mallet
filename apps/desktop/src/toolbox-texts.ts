// The texts of the toolbox page that the engine does not render, kept apart
// from `i18n.ts` so that the page can be changed on its own.

import type { Lang } from "./i18n";

export interface ToolboxTexts {
  /** The heading of the folded list of optional tools, with their number. */
  optional: (count: number) => string;
  /** What the optional tools have in common. */
  optionalHint: string;
  /** The heading of the tools Vortex lists for the game. */
  listed: string;
  /** What those tools are and how to start them. */
  listedHint: string;
  /** In place of the update button once the extension is in Vortex's folder. */
  placed: string;
  /** Why Vortex has to start again before the extension works. */
  placedHint: string;
}

export const toolboxTexts: Record<Lang, ToolboxTexts> = {
  en: {
    optional: (count) => `Optional (${count})`,
    optionalHint: "Not needed to play. Each says what it is for.",
    listed: "Tools in Vortex",
    listedHint:
      "Programs for working on your mods that Vortex knows. Start them from Vortex's dashboard, under Tools: Vortex makes sure they see your mods. Mod Mallet only looks.",
    placed: "✓ Extension placed",
    placedHint: "Vortex loads the new extension when it starts again: restart Vortex.",
  },
  de: {
    optional: (count) => `Optional (${count})`,
    optionalHint: "Zum Spielen nicht nötig. Bei jedem steht, wofür es gut ist.",
    listed: "Werkzeuge in Vortex",
    listedHint:
      "Programme zum Bearbeiten deiner Mods, die Vortex kennt. Starte sie über Vortex' Übersicht unter Werkzeuge: Vortex sorgt dafür, dass sie deine Mods sehen. Mod Mallet schaut nur nach.",
    placed: "✓ Erweiterung hinterlegt",
    placedHint: "Vortex lädt die neue Erweiterung beim nächsten Start: Vortex neu starten.",
  },
};
