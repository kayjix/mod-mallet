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
  /** The button that lists Mod Mallet among Vortex's tools. */
  addToVortex: string;
  /** The button that takes it off the list again. */
  removeFromVortex: string;
  /** In place of the update button once the extension is in Vortex's folder. */
  placed: string;
  /** Why Vortex has to start again before the extension works. */
  placedHint: string;
  restartVortex: string;
  startVortex: string;
  restartingVortex: string;
  restartedVortex: string;
  restartHint: string;
  updateLink: string;
  getIt: string;
}

export const toolboxTexts: Record<Lang, ToolboxTexts> = {
  en: {
    optional: (count) => `Optional (${count})`,
    optionalHint: "Not needed to play. Each says what it is for.",
    listed: "Tools in Vortex",
    listedHint:
      "Programs for working on your mods that Vortex knows. Start them from Vortex's dashboard, under Tools: Vortex makes sure they see your mods. Mod Mallet only looks.",
    addToVortex: "Add Mod Mallet to Vortex's tools",
    removeFromVortex: "Remove Mod Mallet from Vortex's tools",
    placed: "✓ Extension placed",
    placedHint: "Vortex loads the new extension when it starts again: restart Vortex.",
    restartVortex: "Restart Vortex",
    startVortex: "Start Vortex",
    restartingVortex: "Closing Vortex and starting it again…",
    restartedVortex: "Vortex is starting.",
    restartHint:
      "Closes Vortex the way its window's close button does, and starts it again. Vortex saves as usual first.",
    updateLink: "Update the Vortex extension",
    getIt: "Get it",
  },
  de: {
    optional: (count) => `Optional (${count})`,
    optionalHint: "Zum Spielen nicht nötig. Bei jedem steht, wofür es gut ist.",
    listed: "Werkzeuge in Vortex",
    listedHint:
      "Programme zum Bearbeiten deiner Mods, die Vortex kennt. Starte sie über Vortex' Übersicht unter Werkzeuge: Vortex sorgt dafür, dass sie deine Mods sehen. Mod Mallet schaut nur nach.",
    addToVortex: "Mod Mallet zu Vortex' Werkzeugen hinzufügen",
    removeFromVortex: "Mod Mallet aus Vortex' Werkzeugen entfernen",
    placed: "✓ Erweiterung hinterlegt",
    placedHint: "Vortex lädt die neue Erweiterung beim nächsten Start: Vortex neu starten.",
    restartVortex: "Vortex neu starten",
    startVortex: "Vortex starten",
    restartingVortex: "Vortex wird geschlossen und wieder gestartet …",
    restartedVortex: "Vortex startet.",
    restartHint:
      "Schließt Vortex wie über das Schließen-Kreuz des Fensters und startet es wieder. Vortex speichert vorher wie immer.",
    updateLink: "Vortex-Erweiterung aktualisieren",
    getIt: "Holen",
  },
};
