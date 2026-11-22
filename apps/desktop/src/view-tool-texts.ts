// Texts of the tools every 3D view shares: kept apart from `i18n.ts`, as
// every package does.

import type { Lang } from "./i18n";

export interface ViewToolTexts {
  markings: string;
  markingsHint: string;
  /** What the markings mean where the page says nothing of its own. */
  legendCollisions: string;
  legendHidden: string;
  trees: string;
  treesHint: string;
  zoomIn: string;
  zoomOut: string;
  fromAbove: string;
  showAll: string;
}

export const viewToolTexts: Record<Lang, ViewToolTexts> = {
  en: {
    markings: "Markings",
    markingsHint:
      "Off: the place as the game would show it, nothing tinted and what the patch hides left out. The same in every 3D view. Key V on Map collisions.",
    legendCollisions: "Parts of collisions",
    legendHidden: "Red and see-through: what the patch hides.",
    trees: "Show trees",
    treesHint:
      "Trees, plants and grass placed near. Off, they are left out, so that they do not hide what mods built. The same in every 3D view.",
    zoomIn: "Closer",
    zoomOut: "Farther",
    fromAbove: "From above",
    showAll: "Show all",
  },
  de: {
    markings: "Markierungen",
    markingsHint:
      "Aus: der Ort, wie das Spiel ihn zeigt, nichts eingefärbt und was der Patch ausblendet weggelassen. In jeder 3D-Ansicht gleich. Taste V in Karten-Kollisionen.",
    legendCollisions: "Teile von Kollisionen",
    legendHidden: "Rot und durchscheinend: was der Patch ausblendet.",
    trees: "Bäume zeigen",
    treesHint:
      "Bäume, Pflanzen und Gras in der Nähe. Aus, bleiben sie weg, damit sie nicht verdecken, was Mods gebaut haben. In jeder 3D-Ansicht gleich.",
    zoomIn: "Näher",
    zoomOut: "Weiter weg",
    fromAbove: "Von oben",
    showAll: "Alles zeigen",
  },
};
