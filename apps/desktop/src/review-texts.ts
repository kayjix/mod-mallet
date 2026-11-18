// The texts of the guided review (ADR 0050) and of its three-way stage.
// Kept apart from `i18n.ts`, like the other pages' texts.

import { useSession } from "./Actions";
import type { Lang } from "./i18n";

/** The steps of the review, in their order. */
export type ReviewStep = "install" | "order" | "assets" | "map" | "collisions";

export interface ReviewTexts {
  nav: string;
  title: string;
  intro: string;
  step: Record<ReviewStep, { title: string; hint: string }>;
  stepOf: (index: number, total: number) => string;
  open: (count: number) => string;
  seen: string;
  done: string;
  back: string;
  next: string;
  markSeen: string;
  finish: string;
  finished: string;
  continueAt: (step: string) => string;
  start: string;
  more: (count: number) => string;
  toTasks: string;
  toMods: string;
  toAssets: string;
  toMap: string;
  nothing: string;
  modsLine: (enabled: number, total: number, newer: number) => string;
  worldLine: (cells: number, places: number) => string;
  /** The collisions step. */
  structureOf: (index: number, total: number) => string;
  keep: (name: string) => string;
  keepBoth: string;
  skip: string;
  undo: string;
  decidedCount: (decided: number, total: number) => string;
  allDecided: string;
  parts: (name: string, count: number) => string;
  gameParts: (count: number) => string;
  /** The stage. */
  viewNow: string;
  viewOnly: (name: string) => string;
  markings: string;
  fit: string;
  topView: string;
  loading: string;
  missing: string;
  stageHint: string;
}

const en: ReviewTexts = {
  nav: "Go through it",
  title: "Go through your setup",
  intro:
    "Step by step: what is installed, the load order, the assets, the map and the collisions. Stop whenever you like; the next time starts where you left off.",
  step: {
    install: {
      title: "Installation",
      hint: "Which mods are installed, and what is missing or out of date.",
    },
    order: {
      title: "Load order",
      hint: "What depends on the order of the plugins: LOOT's rules and what one plugin undoes of another.",
    },
    assets: {
      title: "Assets",
      hint: "Where two mods bring their own textures, meshes or faces for the same thing.",
    },
    map: {
      title: "World map",
      hint: "The worldspaces, with what mods add to them.",
    },
    collisions: {
      title: "Collisions",
      hint: "Structure by structure: two mods' buildings or bridges standing in each other.",
    },
  },
  stepOf: (index, total) => `Step ${index} of ${total}`,
  open: (count) => `${count} open`,
  seen: "looked at",
  done: "done",
  back: "Back",
  next: "Next step",
  markSeen: "Looks right, next step",
  finish: "Finish",
  finished: "You have been through everything. New scans may bring new things; come back here.",
  continueAt: (step) => `Continue: ${step}`,
  start: "Go through it",
  more: (count) => `${count} more on the page itself`,
  toTasks: "To the tasks",
  toMods: "To the mods",
  toAssets: "To the assets",
  toMap: "To the map",
  nothing: "Nothing here.",
  modsLine: (enabled, total, newer) =>
    `${enabled} of ${total} mods switched on${newer > 0 ? `, ${newer} with a newer version` : ""}.`,
  worldLine: (cells, places) => `mods place objects in ${cells} cells · ${places} places`,
  structureOf: (index, total) => `Structure ${index} of ${total}`,
  keep: (name) => `Keep ${name}'s`,
  keepBoth: "Keep both",
  skip: "Skip",
  undo: "Undo",
  decidedCount: (decided, total) => `${decided} of ${total} spots decided`,
  allDecided: "Every collision is decided.",
  parts: (name, count) => `${name}: ${count} ${count === 1 ? "part" : "parts"}`,
  gameParts: (count) =>
    count === 1
      ? "1 part belongs to the game and stays"
      : `${count} parts belong to the game and stay`,
  viewNow: "Now",
  viewOnly: (name) => `Only ${name}`,
  markings: "Markings",
  fit: "Fit",
  topView: "From above",
  loading: "Reading what stands here…",
  missing: "What stands here cannot be read.",
  stageHint: "Drag to turn · right-drag to move · wheel to zoom",
};

const de: ReviewTexts = {
  nav: "Durchgehen",
  title: "Dein Setup durchgehen",
  intro:
    "Schritt für Schritt: was installiert ist, die Ladereihenfolge, die Assets, die Karte und die Kollisionen. Hör auf, wann du willst; beim nächsten Mal geht es dort weiter.",
  step: {
    install: {
      title: "Installation",
      hint: "Welche Mods installiert sind, und was fehlt oder veraltet ist.",
    },
    order: {
      title: "Ladereihenfolge",
      hint: "Was von der Reihenfolge der Plugins abhängt: LOOTs Regeln und was ein Plugin einem anderen rückgängig macht.",
    },
    assets: {
      title: "Assets",
      hint: "Wo zwei Mods eigene Texturen, Modelle oder Gesichter für dasselbe mitbringen.",
    },
    map: {
      title: "Weltkarte",
      hint: "Die Weltbereiche, mit dem, was Mods dort hinzufügen.",
    },
    collisions: {
      title: "Kollisionen",
      hint: "Bauwerk für Bauwerk: Gebäude oder Brücken zweier Mods, die ineinander stehen.",
    },
  },
  stepOf: (index, total) => `Schritt ${index} von ${total}`,
  open: (count) => `${count} offen`,
  seen: "angesehen",
  done: "erledigt",
  back: "Zurück",
  next: "Nächster Schritt",
  markSeen: "Passt so, weiter",
  finish: "Fertig",
  finished:
    "Du bist alles durchgegangen. Neue Scans können Neues bringen; dann geht es hier weiter.",
  continueAt: (step) => `Weiter: ${step}`,
  start: "Durchgehen",
  more: (count) => `${count} weitere auf der Seite selbst`,
  toTasks: "Zu den Aufgaben",
  toMods: "Zu den Mods",
  toAssets: "Zu den Assets",
  toMap: "Zur Karte",
  nothing: "Hier ist nichts.",
  modsLine: (enabled, total, newer) =>
    `${enabled} von ${total} Mods eingeschaltet${newer > 0 ? `, ${newer} mit neuerer Version` : ""}.`,
  worldLine: (cells, places) => `Mods stellen Objekte in ${cells} Zellen · ${places} Orte`,
  structureOf: (index, total) => `Bauwerk ${index} von ${total}`,
  keep: (name) => `${name} behalten`,
  keepBoth: "Beide behalten",
  skip: "Überspringen",
  undo: "Rückgängig",
  decidedCount: (decided, total) => `${decided} von ${total} Stellen entschieden`,
  allDecided: "Jede Kollision ist entschieden.",
  parts: (name, count) => `${name}: ${count} ${count === 1 ? "Teil" : "Teile"}`,
  gameParts: (count) =>
    count === 1
      ? "1 Teil gehört zum Spiel und bleibt"
      : `${count} Teile gehören zum Spiel und bleiben`,
  viewNow: "Jetzt",
  viewOnly: (name) => `Nur ${name}`,
  markings: "Markierungen",
  fit: "Einpassen",
  topView: "Von oben",
  loading: "Lese, was hier steht …",
  missing: "Was hier steht, lässt sich nicht lesen.",
  stageHint: "Ziehen dreht · rechts ziehen verschiebt · Rad zoomt",
};

export const reviewTexts: Record<Lang, ReviewTexts> = { en, de };

/** The review's texts in the window's language. */
export function useReviewTexts(): ReviewTexts {
  return reviewTexts[useSession().lang];
}
