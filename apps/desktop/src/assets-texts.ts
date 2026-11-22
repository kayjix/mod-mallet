// The texts of the assets page (ADR 0048): the duels about files, the
// comparison stage and the ranking. Kept apart from `i18n.ts`, like the
// other pages' texts, so that the pages can change side by side.

import { useSession } from "./Actions";
import type { Lang } from "./i18n";

export interface AssetsTexts {
  /** The page's tabs, as one group. */
  tabsLabel: string;
  /** A group of the duel list, with how many of it are open. */
  groupOpen: (count: number) => string;
  none: string;
  /** Duels about placed objects, which Collisions decides. */
  objectsElsewhere: (count: number) => string;
  toCollisions: string;
  /** The compact line other pages show for a duel. */
  lineDecided: (name: string) => string;
  lineSuggested: (name: string) => string;
  lineOpen: string;
  /** The patches the picks of the decided duels are in (ADR 0081). */
  decidedIn: (patches: string[]) => string;
  /** Every duel decided: they are listed, and where a new pick goes. */
  allDecided: (count: number, patches: string[]) => string;
}

const en: AssetsTexts = {
  tabsLabel: "Assets",
  groupOpen: (count) => `${count} open`,
  none: "No two mods bring their own textures, meshes or faces for the same thing.",
  objectsElsewhere: (count) =>
    count === 1
      ? "1 pair of mods places objects in the same spots; that is decided under Collisions."
      : `${count} pairs of mods place objects in the same spots; that is decided under Collisions.`,
  toCollisions: "To the collisions",
  lineDecided: (name) => `${name} wins`,
  lineSuggested: (name) => `suggested: ${name}`,
  lineOpen: "not decided",
  decidedIn: (patches) => `in “${patches.join("”, “")}”`,
  allDecided: (count, patches) =>
    `All ${count} are decided, in “${patches.join("”, “")}”, and listed below. What you pick now goes into the patch you are editing.`,
};

const de: AssetsTexts = {
  tabsLabel: "Assets",
  groupOpen: (count) => `${count} offen`,
  none: "Keine zwei Mods bringen eigene Texturen, Modelle oder Gesichter für dasselbe mit.",
  objectsElsewhere: (count) =>
    count === 1
      ? "1 Mod-Paar stellt Objekte an dieselben Stellen; das wird unter Kollisionen entschieden."
      : `${count} Mod-Paare stellen Objekte an dieselben Stellen; das wird unter Kollisionen entschieden.`,
  toCollisions: "Zu den Kollisionen",
  lineDecided: (name) => `${name} gewinnt`,
  lineSuggested: (name) => `Vorschlag: ${name}`,
  lineOpen: "nicht entschieden",
  decidedIn: (patches) => `in „${patches.join("“, „")}“`,
  allDecided: (count, patches) =>
    `Alle ${count} sind entschieden, in „${patches.join("“, „")}“, und stehen unten. Was du jetzt wählst, geht in den Patch, den du bearbeitest.`,
};

export const assetsTexts: Record<Lang, AssetsTexts> = { en, de };

/** The assets page's texts in the window's language. */
export function useAssetsTexts(): AssetsTexts {
  return assetsTexts[useSession().lang];
}
