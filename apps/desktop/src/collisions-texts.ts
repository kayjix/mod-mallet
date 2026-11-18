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
  around: (metres: number) => string;
  aroundWaiting: string;
  aroundPreview: (plugin: string) => string;
  aroundPreviewShort: string;
  aroundHide: (plugin: string, count: number) => string;
  aroundCareful: (count: number) => string;
  aroundCarefulHint: string;
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
    around: (metres) => `Around · ${metres} m`,
    aroundWaiting: "Come close in 3D to see what stands around.",
    aroundPreview: (plugin) =>
      `Look at it without ${plugin}'s objects around here, without deciding`,
    aroundPreviewShort: "Preview",
    aroundCareful: (count) => `${count} of them with care`,
    aroundCarefulHint:
      "The game may miss these: they are drawn from afar, or something refers to them. Pick one in 3D to see why.",
    aroundHide: (plugin, count) =>
      count === 1 ? `Hide 1 object of ${plugin}` : `Hide ${count} objects of ${plugin}`,
  },
  de: {
    also: "Außerdem",
    alsoNote: "Kollisionen dieser Mods, die keine Stelle zum Ansehen nennen.",
    takeSuggestions: (count) =>
      count === 1 ? "Den 1 Vorschlag übernehmen" : `Alle Vorschläge übernehmen (${count})`,
    openThePair: "Dieses Paar als Ganzes entscheiden",
    showOnMap: "Auf der Karte zeigen",
    compareFiles: "Ihre Dateien vergleichen",
    around: (metres) => `Im Umkreis · ${metres} m`,
    aroundWaiting: "Komm in 3D näher, um zu sehen, was hier steht.",
    aroundPreview: (plugin) => `Ohne die Objekte von ${plugin} hier ansehen, ohne zu entscheiden`,
    aroundPreviewShort: "Vorschau",
    aroundCareful: (count) => `davon ${count} mit Vorsicht`,
    aroundCarefulHint:
      "Das Spiel könnte sie vermissen: Sie werden aus der Ferne gezeigt, oder etwas verweist auf sie. Klick eines in 3D an, um zu sehen, warum.",
    aroundHide: (plugin, count) =>
      count === 1
        ? `1 Objekt von ${plugin} ausblenden`
        : `${count} Objekte von ${plugin} ausblenden`,
  },
};
