// The texts of the task rows and the task page around them, in every
// language of the app.

import { useSession } from "./Actions";
import type { Lang } from "./i18n";
import type { ListView, PartId } from "./task-plan";

/** What a tile of the task page chooses. */
type TileId = ListView | "collisions";

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
  /** A row's action was handed to the mod manager. */
  handedOn: string;
  /** What is shown when nothing at all is to do. */
  allClearNote: string;
  /** A finding was set aside, with the way back. */
  setAside: (title: string) => string;
  /** What the tiles choose between, for a screen reader. */
  tilesLabel: string;
  /** Each tile: its name, what it means in a few words, and a longer line over its list. */
  tile: Record<TileId, { label: string; meaning: string; hint: string }>;
  /** On a tile: how many of its findings can crash the game. */
  crashCount: (count: number) => string;
  /** The title of each part of a list. */
  partTitle: Record<PartId, string>;
  /** A category with nothing in it. */
  emptyCategory: string;
  /** The way to the toolbox. */
  toToolbox: string;
  /** The way to the mods page. */
  toMods: string;
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
  handedOn: "Handed to your mod manager",
  allClearNote: "Nothing Mod Mallet found crashes the game or looks wrong in it.",
  setAside: (title) => `Set aside: ${title}`,
  tilesLabel: "What there is",
  tile: {
    open: {
      label: "Everything open",
      meaning: "Crash risks first",
      hint: "",
    },
    collisions: {
      label: "Collisions",
      meaning: "Spots where two mods' objects meet",
      hint: "Spots still to decide; they are decided in the worklist.",
    },
    "one-click": {
      label: "One click",
      meaning: "Your mod manager does it",
      hint: "Mod Mallet hands these to your mod manager. You confirm there.",
    },
    decide: {
      label: "Decisions",
      meaning: "Which of two mods wins",
      hint: "Two mods change the same thing. One gives way, or a patch joins them.",
    },
    yourself: {
      label: "Do it yourself",
      meaning: "Install, update, look into it",
      hint: "Mod Mallet cannot do these for you; each row says how.",
    },
    advice: {
      label: "Recommendations",
      meaning: "Optional, may help",
      hint: "Optional. Nothing breaks if you leave them.",
    },
    notes: {
      label: "Good to know",
      meaning: "Nothing to do",
      hint: "Nothing to do. This is how the setup works.",
    },
    ignored: {
      label: "Set aside",
      meaning: "Left as they are, not counted",
      hint: "You chose to leave these as they are. They do not count.",
    },
  },
  crashCount: (count) => (count === 1 ? "1 crash risk" : `${count} crash risks`),
  partTitle: {
    crash: "First: what can crash the game",
    "one-click": "Done with one click in your mod manager",
    decide: "Decide which mod wins",
    yourself: "Do it yourself",
    advice: "Recommendations",
    notes: "Good to know, nothing to do",
    ignored: "Set aside, not counted",
  },
  emptyCategory: "Nothing here right now.",
  toToolbox: "To the toolbox",
  toMods: "To the mods",
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
  handedOn: "An deine Mod-Verwaltung übergeben",
  allClearNote:
    "Nichts, was Mod Mallet gefunden hat, lässt das Spiel abstürzen oder sieht darin falsch aus.",
  setAside: (title) => `Beiseitegelegt: ${title}`,
  tilesLabel: "Was es gibt",
  tile: {
    open: {
      label: "Alles Offene",
      meaning: "Absturzgefahren zuerst",
      hint: "",
    },
    collisions: {
      label: "Kollisionen",
      meaning: "Stellen, an denen sich zwei Mods treffen",
      hint: "Stellen, die noch zu entscheiden sind; entschieden wird in der Kollisionsliste.",
    },
    "one-click": {
      label: "Ein Klick",
      meaning: "Erledigt deine Mod-Verwaltung",
      hint: "Mod Mallet gibt das an deine Mod-Verwaltung weiter. Bestätigen musst du dort.",
    },
    decide: {
      label: "Entscheidungen",
      meaning: "Welche von zwei Mods gewinnt",
      hint: "Zwei Mods ändern dasselbe. Eine gibt nach, oder ein Patch verbindet sie.",
    },
    yourself: {
      label: "Selbst erledigen",
      meaning: "Installieren, aktualisieren, nachsehen",
      hint: "Das kann Mod Mallet nicht für dich tun; jede Zeile sagt, wie es geht.",
    },
    advice: {
      label: "Empfehlungen",
      meaning: "Freiwillig, kann helfen",
      hint: "Freiwillig. Nichts geht kaputt, wenn du sie lässt.",
    },
    notes: {
      label: "Gut zu wissen",
      meaning: "Nichts zu tun",
      hint: "Nichts zu tun. So funktioniert das Setup eben.",
    },
    ignored: {
      label: "Beiseitegelegt",
      meaning: "Gelassen, zählt nicht mit",
      hint: "Die hast du gelassen, wie sie sind. Sie zählen nicht mit.",
    },
  },
  crashCount: (count) => (count === 1 ? "1 Absturzgefahr" : `${count} Absturzgefahren`),
  partTitle: {
    crash: "Zuerst: was das Spiel abstürzen lassen kann",
    "one-click": "Mit einem Klick in deiner Mod-Verwaltung",
    decide: "Entscheiden, welche Mod gewinnt",
    yourself: "Selbst erledigen",
    advice: "Empfehlungen",
    notes: "Gut zu wissen, nichts zu tun",
    ignored: "Beiseitegelegt, zählt nicht mit",
  },
  emptyCategory: "Hier ist gerade nichts.",
  toToolbox: "Zum Werkzeugkasten",
  toMods: "Zu den Mods",
};

const taskRowTexts: Record<Lang, TaskRowTexts> = { en, de };

/** The texts of the task rows in the window's language. */
export function useTaskRowTexts(): TaskRowTexts {
  return taskRowTexts[useSession().lang];
}
