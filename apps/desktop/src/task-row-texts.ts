// The texts of the task rows and the task page around them, in every
// language of the app.

import { useSession } from "./Actions";
import type { Lang } from "./i18n";

/** What the task rows and the task page say beside the findings' own texts. */
export interface TaskRowTexts {
  /** The button that opens a mod's page. */
  modPage: (name: string) => string;
  /** The button that shows the map filtered to one mod. */
  modOnMap: (name: string) => string;
  /** The heading of the list of missing textures. */
  missingTextures: (count: number) => string;
  /** The meshes that use a texture. */
  usedBy: (meshes: string) => string;
  /** The heading of the mods of a finding. */
  mods: string;
  /** The button that opens a row. */
  details: string;
  /** The button of a group, folded. */
  showAll: (count: number) => string;
  /** The button of a group, open. */
  fold: string;
  /** More mods than a row names. */
  moreMods: (count: number) => string;
  /** How many mods a group is about. */
  modCount: (count: number) => string;
  /** The title of the part with the crash risks. */
  crashTitle: string;
  /** A row's action was handed to the mod manager. */
  handedOn: string;
  /** Under the headline: how many tasks can crash the game. */
  summaryCrash: (count: number) => string;
  /** How many tasks are wrong in the game. */
  summaryBroken: (count: number) => string;
  /** How many cost quality. */
  summaryQuality: (count: number) => string;
  /** How many keep checks from running. */
  summaryScan: (count: number) => string;
  /** No task can crash the game. */
  summaryNoCrash: string;
  /** Nothing at all to do. */
  allClear: string;
  /** What is shown when nothing at all is to do. */
  allClearNote: string;
  /** A finding was set aside, with the way back. */
  setAside: (title: string) => string;
  /** The button that brings it back. */
  undo: string;
}

const en: TaskRowTexts = {
  modPage: (name) => `Page of ${name}`,
  modOnMap: (name) => `${name} on the map`,
  missingTextures: (count) =>
    count === 1 ? "The missing texture" : `The ${count} missing textures`,
  usedBy: (meshes) => `for ${meshes}`,
  mods: "Mods",
  details: "Details",
  showAll: (count) => `Show all ${count}`,
  fold: "Fold",
  moreMods: (count) => `+${count} more`,
  modCount: (count) => (count === 1 ? "1 mod" : `${count} mods`),
  crashTitle: "First: what can crash the game",
  handedOn: "Handed to your mod manager",
  summaryCrash: (count) => (count === 1 ? "1 can crash the game" : `${count} can crash the game`),
  summaryBroken: (count) =>
    count === 1 ? "1 is wrong in the game" : `${count} are wrong in the game`,
  summaryQuality: (count) => (count === 1 ? "1 costs quality" : `${count} cost quality`),
  summaryScan: (count) =>
    count === 1 ? "1 keeps checks from running" : `${count} keep checks from running`,
  summaryNoCrash: "None of them crashes the game.",
  allClear: "Nothing to do",
  allClearNote: "Nothing Mod Mallet found crashes the game or looks wrong in it.",
  setAside: (title) => `Set aside: ${title}`,
  undo: "Undo",
};

const de: TaskRowTexts = {
  modPage: (name) => `Seite von ${name}`,
  modOnMap: (name) => `${name} auf der Karte`,
  missingTextures: (count) =>
    count === 1 ? "Die fehlende Textur" : `Die ${count} fehlenden Texturen`,
  usedBy: (meshes) => `für ${meshes}`,
  mods: "Mods",
  details: "Details",
  showAll: (count) => `Alle ${count} zeigen`,
  fold: "Einklappen",
  moreMods: (count) => `+${count} weitere`,
  modCount: (count) => (count === 1 ? "1 Mod" : `${count} Mods`),
  crashTitle: "Zuerst: was das Spiel abstürzen lassen kann",
  handedOn: "An deine Mod-Verwaltung übergeben",
  summaryCrash: (count) =>
    count === 1
      ? "1 kann das Spiel abstürzen lassen"
      : `${count} können das Spiel abstürzen lassen`,
  summaryBroken: (count) =>
    count === 1 ? "1 zeigt Fehler im Spiel" : `${count} zeigen Fehler im Spiel`,
  summaryQuality: (count) => (count === 1 ? "1 kostet Qualität" : `${count} kosten Qualität`),
  summaryScan: (count) =>
    count === 1 ? "1 verhindert Prüfungen" : `${count} verhindern Prüfungen`,
  summaryNoCrash: "Nichts davon lässt das Spiel abstürzen.",
  allClear: "Nichts zu tun",
  allClearNote:
    "Nichts, was Mod Mallet gefunden hat, lässt das Spiel abstürzen oder sieht darin falsch aus.",
  setAside: (title) => `Beiseitegelegt: ${title}`,
  undo: "Rückgängig",
};

const taskRowTexts: Record<Lang, TaskRowTexts> = { en, de };

/** The texts of the task rows in the window's language. */
export function useTaskRowTexts(): TaskRowTexts {
  return taskRowTexts[useSession().lang];
}
