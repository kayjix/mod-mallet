// Texts of the collisions' page that came with ADR 0048 (package B): kept
// apart from `i18n.ts`, as every package of that plan does.

import type { Lang } from "./i18n";

export interface CollisionsTexts {
  also: string;
  alsoNote: string;
  takeSuggestions: (count: number) => string;
  openThePair: string;
  showOnMap: string;
  compareFiles: string;
}

export const collisionsTexts: Record<Lang, CollisionsTexts> = {
  en: {
    also: "Also",
    alsoNote: "Collisions of these mods that name no spot to look at.",
    takeSuggestions: (count) =>
      count === 1 ? "Take the 1 suggestion" : `Take every suggestion (${count})`,
    openThePair: "Decide this pair as a whole",
    showOnMap: "Show on the map",
    compareFiles: "Compare their files",
  },
  de: {
    also: "Außerdem",
    alsoNote: "Kollisionen dieser Mods, die keine Stelle zum Ansehen nennen.",
    takeSuggestions: (count) =>
      count === 1 ? "Den 1 Vorschlag übernehmen" : `Alle Vorschläge übernehmen (${count})`,
    openThePair: "Dieses Paar als Ganzes entscheiden",
    showOnMap: "Auf der Karte zeigen",
    compareFiles: "Ihre Dateien vergleichen",
  },
};
