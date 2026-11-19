// The texts of a task's row, opened, in every language of the app.

import { useSession } from "./Actions";
import type { Lang } from "./i18n";

/** What an opened task row says beside the finding's own texts. */
export interface TaskRowTexts {
  /** The button that opens a mod's page. */
  modPage: (name: string) => string;
  /** The button that shows the map filtered to one mod. */
  modOnMap: (name: string) => string;
  /** The heading of the list of missing textures. */
  missingTextures: (count: number) => string;
  /** The meshes that use a texture. */
  usedBy: (meshes: string) => string;
  /** The heading of the mods under "more about it". */
  mods: string;
}

const en: TaskRowTexts = {
  modPage: (name) => `Page of ${name}`,
  modOnMap: (name) => `${name} on the map`,
  missingTextures: (count) =>
    count === 1 ? "The missing texture" : `The ${count} missing textures`,
  usedBy: (meshes) => `for ${meshes}`,
  mods: "Mods",
};

const de: TaskRowTexts = {
  modPage: (name) => `Seite von ${name}`,
  modOnMap: (name) => `${name} auf der Karte`,
  missingTextures: (count) =>
    count === 1 ? "Die fehlende Textur" : `Die ${count} fehlenden Texturen`,
  usedBy: (meshes) => `für ${meshes}`,
  mods: "Mods",
};

const taskRowTexts: Record<Lang, TaskRowTexts> = { en, de };

/** The texts of a task's row in the window's language. */
export function useTaskRowTexts(): TaskRowTexts {
  return taskRowTexts[useSession().lang];
}
