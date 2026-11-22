// The texts of the guided review (ADR 0050) and of its three-way stage.
// Kept apart from `i18n.ts`, like the other pages' texts.

import { useSession } from "./Actions";
import type { Lang } from "./i18n";

/** The steps of the review, in their order. */
export type ReviewStep = "install" | "order" | "assets" | "map" | "collisions" | "report";

export interface ReviewTexts {
  nav: string;
  step: Record<ReviewStep, { title: string }>;
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
  allDecided: string;
  parts: (name: string, count: number) => string;
  /** The stage. */
  viewNow: string;
  viewOnly: (name: string) => string;
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
  suggestedTag: string;
  suggestReason: (
    winner: string,
    winnerParts: number,
    winnerRoom: string,
    loser: string,
    loserParts: number,
    loserRoom: string,
    single: string | null,
  ) => string;
  segmentSuggested: (name: string) => string;
  measuring: (done: number, total: number) => string;
  measureStop: string;
  takeAllDone: (
    taken: number,
    doubtful: number,
    stuck: number,
    asking: number,
    walked: number,
  ) => string;
  pickHint: string;
  partsTitle: (count: number) => string;
  singlyHidden: (count: number) => string;
  withExceptions: (count: number) => string;
  pickedHidden: (name: string) => string;
  pickedShown: (name: string) => string;
  pickedStays: (name: string) => string;
  pickedOther: (name: string, mod: string) => string;
  checkingTies: string;
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
  loading: string;
  missing: string;
  stageHint: string;
}

const en: ReviewTexts = {
  nav: "Go through it",
  step: {
    install: {
      title: "Installation",
    },
    order: {
      title: "Load order",
    },
    assets: {
      title: "Assets",
    },
    map: {
      title: "World map",
    },
    collisions: {
      title: "Collisions",
    },
    report: {
      title: "Report",
    },
  },
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
  allDecided: "Every collision is decided.",
  parts: (name, count) => `${name}: ${count} ${count === 1 ? "part" : "parts"}`,
  viewNow: "Now",
  viewOnly: (name) => `Only ${name}`,
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
  suggestedTag: "suggested",
  suggestReason: (winner, winnerParts, winnerRoom, loser, loserParts, loserRoom, single) =>
    `${winner} builds ${winnerParts} ${winnerParts === 1 ? "part" : "parts"} here (about ${winnerRoom} m³); ${
      single !== null
        ? `${loser} has a single one, ${single} (about ${loserRoom} m³)`
        : `${loser} only ${loserParts} (about ${loserRoom} m³)`
    }. Keeping ${winner}'s is suggested.`,
  segmentSuggested: (name) => `suggested: ${name}`,
  measuring: (done, total) => `Measuring places… ${done} of ${total}`,
  measureStop: "Stop",
  takeAllDone: (taken, doubtful, stuck, asking, walked) =>
    `${taken} ${taken === 1 ? "place" : "places"} decided as suggested. Left to go through: ${doubtful} without a clear suggestion${
      asking > 0
        ? `, ${asking} where only parts tied into the game could go (you decide those)`
        : ""
    }${walked > 0 ? `, ${walked} where characters walk on what would go` : ""}${stuck > 0 ? `, ${stuck} where nothing can go` : ""}. Undo takes the whole batch back.`,
  pickHint: "click an object in the views to hide it or show it again",
  partsTitle: (count) => `The ${count} objects one by one`,
  singlyHidden: (count) => (count === 1 ? "1 hidden on its own" : `${count} hidden on their own`),
  withExceptions: (count) => (count === 1 ? "(1 exception)" : `(${count} exceptions)`),
  pickedHidden: (name) => `${name} is hidden now.`,
  pickedShown: (name) => `${name} is shown again.`,
  pickedStays: (name) => `${name} stays: the game needs it.`,
  pickedOther: (name, mod) => `${name} belongs to ${mod}, not to the two mods compared here.`,
  checkingTies: "Looking at what the game needs here…",
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
  loading: "Reading what stands here…",
  missing: "What stands here cannot be read.",
  stageHint: "Drag to turn · right-drag to move · wheel to zoom",
};

const de: ReviewTexts = {
  nav: "Durchgehen",
  step: {
    install: {
      title: "Installation",
    },
    order: {
      title: "Ladereihenfolge",
    },
    assets: {
      title: "Assets",
    },
    map: {
      title: "Weltkarte",
    },
    collisions: {
      title: "Kollisionen",
    },
    report: {
      title: "Bericht",
    },
  },
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
  allDecided: "Jede Kollision ist entschieden.",
  parts: (name, count) => `${name}: ${count} ${count === 1 ? "Teil" : "Teile"}`,
  viewNow: "Jetzt",
  viewOnly: (name) => `Nur ${name}`,
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
  suggestedTag: "Vorschlag",
  suggestReason: (winner, winnerParts, winnerRoom, loser, loserParts, loserRoom, single) =>
    `${winner} baut hier ${winnerParts} ${winnerParts === 1 ? "Teil" : "Teile"} (etwa ${winnerRoom} m³); ${
      single !== null
        ? `${loser} hat ein einzelnes, ${single} (etwa ${loserRoom} m³)`
        : `${loser} nur ${loserParts} (etwa ${loserRoom} m³)`
    }. Vorschlag: ${winner} behalten.`,
  segmentSuggested: (name) => `Vorschlag: ${name}`,
  measuring: (done, total) => `Messe Bereiche … ${done} von ${total}`,
  measureStop: "Anhalten",
  takeAllDone: (taken, doubtful, stuck, asking, walked) =>
    `${taken} ${taken === 1 ? "Bereich" : "Bereiche"} nach Vorschlag entschieden. Zum Durchgehen bleiben ${doubtful} ohne klaren Vorschlag${
      asking > 0
        ? `, ${asking}, in denen nur ins Spiel Eingebundenes weg könnte (das entscheidest du)`
        : ""
    }${walked > 0 ? `, ${walked}, in denen Figuren auf dem laufen, was weg würde` : ""}${stuck > 0 ? `, ${stuck}, in denen nichts weg kann` : ""}. Rückgängig nimmt alle auf einmal zurück.`,
  pickHint: "ein Objekt in den Ansichten anklicken blendet es aus oder wieder ein",
  partsTitle: (count) => `Die ${count} Objekte einzeln`,
  singlyHidden: (count) =>
    count === 1 ? "1 einzeln ausgeblendet" : `${count} einzeln ausgeblendet`,
  withExceptions: (count) => (count === 1 ? "(1 Ausnahme)" : `(${count} Ausnahmen)`),
  pickedHidden: (name) => `${name} ist jetzt ausgeblendet.`,
  pickedShown: (name) => `${name} ist wieder sichtbar.`,
  pickedStays: (name) => `${name} bleibt: das Spiel braucht es.`,
  pickedOther: (name, mod) => `${name} gehört zu ${mod}, nicht zu den beiden verglichenen Mods.`,
  checkingTies: "Prüfe, was das Spiel hier braucht …",
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
  loading: "Lese, was hier steht …",
  missing: "Was hier steht, lässt sich nicht lesen.",
  stageHint: "Ziehen dreht · rechts ziehen verschiebt · Rad zoomt",
};

export const reviewTexts: Record<Lang, ReviewTexts> = { en, de };

/** The review's texts in the window's language. */
export function useReviewTexts(): ReviewTexts {
  return reviewTexts[useSession().lang];
}
