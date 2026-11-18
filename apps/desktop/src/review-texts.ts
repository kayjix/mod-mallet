// The texts of the guided review (ADR 0050) and of its three-way stage.
// Kept apart from `i18n.ts`, like the other pages' texts.

import { useSession } from "./Actions";
import type { Lang } from "./i18n";

/** The steps of the review, in their order. */
export type ReviewStep = "install" | "order" | "assets" | "map" | "collisions" | "report";

export interface ReviewTexts {
  nav: string;
  title: string;
  intro: string;
  step: Record<ReviewStep, { title: string; hint: string }>;
  stepOf: (index: number, total: number) => string;
  open: (count: number) => string;
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
  keepBoth: string;
  undo: string;
  allDecided: string;
  parts: (name: string, count: number) => string;
  gameParts: (count: number) => string;
  /** The stage. */
  viewNow: string;
  viewOnly: (name: string) => string;
  markings: string;
  showTrees: string;
  markingsHint: string;
  legendHidden: string;
  othersHere: (count: number) => string;
  othersShow: (name: string) => string;
  othersCount: (name: string, count: number) => string;
  /** Moving through the structures, and through the steps. */
  previousStructure: string;
  nextStructure: string;
  clusterOf: (index: number, total: number) => string;
  clusterSpots: (spots: number, segments: number) => string;
  segmentsTitle: string;
  segmentName: (index: number, spots: number) => string;
  segmentParts: (first: string, a: number, second: string, b: number) => string;
  stateMixed: (open: number) => string;
  showSegment: string;
  keepHere: (name: string) => string;
  compared: string;
  levelItem: (mod: string, name: string) => string;
  levelParts: (mod: string, parts: number, segments: number) => string;
  pickHint: string;
  partsTitle: (count: number) => string;
  partHide: string;
  partShow: string;
  partHideAnyway: string;
  partGame: string;
  partLocked: string;
  singlyHidden: (count: number) => string;
  withExceptions: (count: number) => string;
  pickedHidden: (name: string) => string;
  pickedShown: (name: string) => string;
  pickedStays: (name: string) => string;
  pickedOther: (name: string, mod: string) => string;
  checkingTies: string;
  keepEffect: (loser: string, go: number, total: number) => string;
  nothingCanGo: (loser: string) => string;
  carefulNote: (count: number) => string;
  lockedNote: (count: number) => string;
  whyTheyStay: (count: number) => string;
  neededByGame: string;
  hideAnyway: (loser: string, count: number) => string;
  keepBothHere: string;
  reportCollisions: (decided: number, total: number) => string;
  reportDecidedTitle: string;
  reportOpenTitle: string;
  reportModsTitle: string;
  reportOtherSteps: string;
  reportLine: (step: string, open: number) => string;
  goThere: string;
  restart: string;
  stateOpen: string;
  stateKept: string;
  stateHidden: (name: string) => string;
  stepBack: (step: string) => string;
  stepNext: (step: string) => string;
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
      hint: "Place by place: where two mods' buildings or bridges stand in each other, decided for the whole place or section by section.",
    },
    report: {
      title: "Report",
      hint: "What was decided, which mods were involved, and what is left open.",
    },
  },
  stepOf: (index, total) => `Step ${index} of ${total}`,
  open: (count) => `${count} open`,
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
  keepBoth: "Keep both",
  undo: "Undo",
  allDecided: "Every collision is decided.",
  parts: (name, count) => `${name}: ${count} ${count === 1 ? "part" : "parts"}`,
  gameParts: (count) =>
    count === 1
      ? "1 part belongs to the game and stays"
      : `${count} parts belong to the game and stay`,
  viewNow: "Now",
  viewOnly: (name) => `Only ${name}`,
  markings: "Markings",
  showTrees: "Show trees",
  markingsHint:
    "With the eye closed, the place looks as the game would show it: nothing tinted, and what is hidden left out.",
  legendHidden: "red: already hidden, by this or your other decisions",
  othersHere: (count) =>
    count === 1 ? "One more mod built here:" : `${count} more mods built here:`,
  othersShow: (name) => `Pick out what ${name} places here`,
  othersCount: (name, count) => `${name} (${count})`,
  previousStructure: "‹ Previous place",
  nextStructure: "Next place ›",
  clusterOf: (index, total) => `Place ${index} of ${total}`,
  clusterSpots: (spots, segments) =>
    `${spots} ${spots === 1 ? "spot" : "spots"} in ${segments} ${segments === 1 ? "section" : "sections"}`,
  segmentsTitle: "Sections of this place",
  segmentName: (index, spots) => `Section ${index} · ${spots} ${spots === 1 ? "spot" : "spots"}`,
  segmentParts: (first, a, second, b) => `${first} ${a} · ${second} ${b} parts`,
  stateMixed: (open) =>
    open > 0 ? `partly decided, ${open} open` : "decided differently by section",
  showSegment: "Show",
  keepHere: (name) => `Keep ${name}'s here`,
  compared: "Compared:",
  levelItem: (mod, name) => `${mod}: a single item, ${name}`,
  levelParts: (mod, parts, segments) =>
    `${mod}: ${parts} parts in ${segments} ${segments === 1 ? "section" : "sections"}`,
  pickHint: "click an object in the views to hide it or show it again",
  partsTitle: (count) => `The ${count} objects one by one`,
  partHide: "Hide",
  partShow: "Show again",
  partHideAnyway: "Hide anyway",
  partGame: "the game's own, stays",
  partLocked: "the game needs it, stays",
  singlyHidden: (count) => (count === 1 ? "1 hidden on its own" : `${count} hidden on their own`),
  withExceptions: (count) => (count === 1 ? "(1 exception)" : `(${count} exceptions)`),
  pickedHidden: (name) => `${name} is hidden now.`,
  pickedShown: (name) => `${name} is shown again.`,
  pickedStays: (name) => `${name} stays: the game needs it.`,
  pickedOther: (name, mod) => `${name} belongs to ${mod}, not to the two mods compared here.`,
  checkingTies: "Looking at what the game needs here…",
  keepEffect: (loser, go, total) =>
    go === total
      ? `all ${total} ${total === 1 ? "part" : "parts"} of ${loser} can go.`
      : `${go} of ${total} parts of ${loser} can go.`,
  nothingCanGo: (loser) =>
    `nothing of ${loser} can go here: the game needs these parts. "Keep both here" settles the place without hiding anything.`,
  carefulNote: (count) =>
    count === 1
      ? "1 part is tied into the game: hiding it is possible, but may break something."
      : `${count} parts are tied into the game: hiding them is possible, but may break something.`,
  lockedNote: (count) =>
    count === 1 ? "1 part stays: the game needs it." : `${count} parts stay: the game needs them.`,
  whyTheyStay: (count) => (count === 1 ? "Why 1 part stays" : `Why ${count} parts stay`),
  neededByGame: "the scan marks it as needed by the game",
  hideAnyway: (loser, count) =>
    count === 1
      ? `Hide the tied part of ${loser} anyway`
      : `Hide the ${count} tied parts of ${loser} anyway`,
  keepBothHere: "Keep both here",
  reportCollisions: (decided, total) => `${decided} of ${total} places decided`,
  reportDecidedTitle: "Decided",
  reportOpenTitle: "Still open",
  reportModsTitle: "Mods involved",
  reportOtherSteps: "The other steps",
  reportLine: (step, open) => (open === 0 ? `${step}: nothing open` : `${step}: ${open} open`),
  goThere: "Go there",
  restart: "Start again from the first step",
  stateOpen: "not decided yet",
  stateKept: "decided: both stay",
  stateHidden: (name) => `decided: ${name}'s is hidden`,
  stepBack: (step) => `‹ Step back: ${step}`,
  stepNext: (step) => `Next step: ${step} ›`,
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
      hint: "Ort für Ort: wo Gebäude oder Brücken zweier Mods ineinander stehen, für den ganzen Bereich oder Abschnitt für Abschnitt entschieden.",
    },
    report: {
      title: "Bericht",
      hint: "Was entschieden ist, welche Mods beteiligt waren, und was noch offen ist.",
    },
  },
  stepOf: (index, total) => `Schritt ${index} von ${total}`,
  open: (count) => `${count} offen`,
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
  keepBoth: "Beide behalten",
  undo: "Rückgängig",
  allDecided: "Jede Kollision ist entschieden.",
  parts: (name, count) => `${name}: ${count} ${count === 1 ? "Teil" : "Teile"}`,
  gameParts: (count) =>
    count === 1
      ? "1 Teil gehört zum Spiel und bleibt"
      : `${count} Teile gehören zum Spiel und bleiben`,
  viewNow: "Jetzt",
  viewOnly: (name) => `Nur ${name}`,
  markings: "Markierungen",
  showTrees: "Bäume zeigen",
  markingsHint:
    "Mit geschlossenem Auge sieht die Stelle aus, wie das Spiel sie zeigt: nichts eingefärbt, Ausgeblendetes weggelassen.",
  legendHidden: "rot: schon ausgeblendet, durch diese oder deine anderen Entscheidungen",
  othersHere: (count) =>
    count === 1 ? "Ein weiterer Mod baut hier:" : `${count} weitere Mods bauen hier:`,
  othersShow: (name) => `Hervorheben, was ${name} hier platziert`,
  othersCount: (name, count) => `${name} (${count})`,
  previousStructure: "‹ Vorheriger Bereich",
  nextStructure: "Nächster Bereich ›",
  clusterOf: (index, total) => `Bereich ${index} von ${total}`,
  clusterSpots: (spots, segments) =>
    `${spots} ${spots === 1 ? "Stelle" : "Stellen"} in ${segments} ${segments === 1 ? "Abschnitt" : "Abschnitten"}`,
  segmentsTitle: "Abschnitte dieses Bereichs",
  segmentName: (index, spots) =>
    `Abschnitt ${index} · ${spots} ${spots === 1 ? "Stelle" : "Stellen"}`,
  segmentParts: (first, a, second, b) => `${first} ${a} · ${second} ${b} Teile`,
  stateMixed: (open) =>
    open > 0 ? `teils entschieden, ${open} offen` : "je Abschnitt verschieden entschieden",
  showSegment: "Zeigen",
  keepHere: (name) => `Hier ${name} behalten`,
  compared: "Verglichen:",
  levelItem: (mod, name) => `${mod}: ein einzelnes Teil, ${name}`,
  levelParts: (mod, parts, segments) =>
    `${mod}: ${parts} Teile in ${segments} ${segments === 1 ? "Abschnitt" : "Abschnitten"}`,
  pickHint: "ein Objekt in den Ansichten anklicken blendet es aus oder wieder ein",
  partsTitle: (count) => `Die ${count} Objekte einzeln`,
  partHide: "Ausblenden",
  partShow: "Wieder zeigen",
  partHideAnyway: "Trotzdem ausblenden",
  partGame: "gehört zum Spiel, bleibt",
  partLocked: "das Spiel braucht es, bleibt",
  singlyHidden: (count) =>
    count === 1 ? "1 einzeln ausgeblendet" : `${count} einzeln ausgeblendet`,
  withExceptions: (count) => (count === 1 ? "(1 Ausnahme)" : `(${count} Ausnahmen)`),
  pickedHidden: (name) => `${name} ist jetzt ausgeblendet.`,
  pickedShown: (name) => `${name} ist wieder sichtbar.`,
  pickedStays: (name) => `${name} bleibt: das Spiel braucht es.`,
  pickedOther: (name, mod) => `${name} gehört zu ${mod}, nicht zu den beiden verglichenen Mods.`,
  checkingTies: "Prüfe, was das Spiel hier braucht …",
  keepEffect: (loser, go, total) =>
    go === total
      ? `alle ${total} ${total === 1 ? "Teil" : "Teile"} von ${loser} können weg.`
      : `${go} von ${total} Teilen von ${loser} können weg.`,
  nothingCanGo: (loser) =>
    `von ${loser} kann hier nichts weg: das Spiel braucht diese Teile. „Hier beide behalten“ entscheidet die Stelle, ohne etwas auszublenden.`,
  carefulNote: (count) =>
    count === 1
      ? "1 Teil ist ins Spiel eingebunden: ausblenden geht, kann aber etwas stören."
      : `${count} Teile sind ins Spiel eingebunden: ausblenden geht, kann aber etwas stören.`,
  lockedNote: (count) =>
    count === 1
      ? "1 Teil bleibt: das Spiel braucht es."
      : `${count} Teile bleiben: das Spiel braucht sie.`,
  whyTheyStay: (count) => (count === 1 ? "Warum 1 Teil bleibt" : `Warum ${count} Teile bleiben`),
  neededByGame: "der Scan kennzeichnet es als vom Spiel gebraucht",
  hideAnyway: (loser, count) =>
    count === 1
      ? `Das eingebundene Teil von ${loser} trotzdem ausblenden`
      : `Die ${count} eingebundenen Teile von ${loser} trotzdem ausblenden`,
  keepBothHere: "Hier beide behalten",
  reportCollisions: (decided, total) => `${decided} von ${total} Bereichen entschieden`,
  reportDecidedTitle: "Entschieden",
  reportOpenTitle: "Noch offen",
  reportModsTitle: "Beteiligte Mods",
  reportOtherSteps: "Die anderen Schritte",
  reportLine: (step, open) => (open === 0 ? `${step}: nichts offen` : `${step}: ${open} offen`),
  goThere: "Hingehen",
  restart: "Noch einmal von vorn",
  stateOpen: "noch nicht entschieden",
  stateKept: "entschieden: beide bleiben",
  stateHidden: (name) => `entschieden: ${name} wird ausgeblendet`,
  stepBack: (step) => `‹ Schritt zurück: ${step}`,
  stepNext: (step) => `Nächster Schritt: ${step} ›`,
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
