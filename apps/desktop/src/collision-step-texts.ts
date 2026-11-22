// The texts of the collisions step of the guided review laid out by side
// (ADR 0080): the bar's buttons, the column of each mod, the part lists
// and what is left after a decision. Short: the pictures say the rest.

import { useSession } from "./Actions";
import type { Lang } from "./i18n";

export interface CollisionStepTexts {
  measure: (count: number) => string;
  measureTip: string;
  take: (count: number) => string;
  takeTip: string;
  openInMap: string;
  openInMapTip: string;
  legendHidden: string;
  alsoHere: string;
  /** What the views pick out, and the way back to everything. */
  showing: (what: string) => string;
  showAll: string;
  goingParts: (count: number, mod: string) => string;
  tiedParts: (count: number, mod: string) => string;
  stayingParts: (count: number, mod: string) => string;
  walkedParts: (count: number, mod: string) => string;
  leftParts: (count: number, mod: string) => string;
  /** A side's column. */
  effectGo: (mod: string, go: number, total: number) => string;
  effectNothing: (mod: string) => string;
  effectNothingTip: string;
  badgeTied: (count: number) => string;
  badgeStay: (count: number) => string;
  badgeResting: (count: number) => string;
  badgeRestingStays: (count: number) => string;
  badgeWalked: (count: number) => string;
  badgeGame: (count: number) => string;
  showThese: string;
  showWhich: string;
  tiedExplain: string;
  stayExplain: string;
  restingExplain: string;
  restingStaysExplain: string;
  gameExplain: string;
  hideSingles: (mod: string) => string;
  keptHere: string;
  /** The part lists. */
  listHint: string;
  picked: (count: number) => string;
  hidePicked: string;
  showPicked: string;
  clearPicked: string;
  oneByOne: string;
  member: (name: string, index: number) => string;
  hiddenOf: (hidden: number, total: number) => string;
  stays: string;
  gameOwn: string;
  hideOne: (name: string) => string;
  showOne: (name: string) => string;
  /** What is left of the losing mod after a decision. */
  left: (count: number, mod: string) => string;
  leftKeepBoth: string;
  leftKeepBothTip: string;
  leftHide: string;
  leftNeeded: string;
  leftOpen: string;
}

const en: CollisionStepTexts = {
  measure: (count) => `Measure open places (${count})`,
  measureTip:
    "Reads every open place in the background and measures how much room each mod takes there. Places where one mod is clearly bigger get a suggestion.",
  take: (count) => `Take suggestions (${count})`,
  takeTip:
    "Decides every place with a clear suggestion: one side takes at least 20 times the room of the other. What the game needs stays. Undo takes the whole batch back.",
  openInMap: "Open in Map collisions",
  openInMapTip:
    "The same place in Map collisions, in one big view. The back button brings you here.",
  legendHidden: "already hidden",
  alsoHere: "Also built here:",
  showing: (what) => `Shown: ${what}`,
  showAll: "Show everything",
  goingParts: (count, mod) =>
    count === 1 ? `the 1 ${mod} part that goes` : `the ${count} ${mod} parts that go`,
  tiedParts: (count, mod) => `the ${count} tied ${count === 1 ? "part" : "parts"} of ${mod}`,
  stayingParts: (count, mod) =>
    count === 1 ? `the 1 ${mod} part that stays` : `the ${count} ${mod} parts that stay`,
  walkedParts: (count, mod) =>
    `the ${count} ${mod} ${count === 1 ? "part" : "parts"} characters walk on`,
  leftParts: (count, mod) => `the ${count} ${mod} ${count === 1 ? "part" : "parts"} left`,
  effectGo: (mod, go, total) =>
    go === total
      ? `all ${total} ${mod} ${total === 1 ? "part goes" : "parts go"}`
      : `${go} of ${total} ${mod} parts go`,
  effectNothing: (mod) => `nothing of ${mod} can go`,
  effectNothingTip: "The game needs these parts. “Keep both here” settles the place.",
  badgeTied: (count) => `${count} tied`,
  badgeStay: (count) => `${count} ${count === 1 ? "stays" : "stay"}`,
  badgeResting: (count) => `${count} resting on it ${count === 1 ? "goes" : "go"} too`,
  badgeRestingStays: (count) => `${count} resting on it ${count === 1 ? "stays" : "stay"}`,
  badgeWalked: (count) => `walked on: ${count}`,
  badgeGame: (count) => `${count} the game's own`,
  showThese: "Show these",
  showWhich: "Show which",
  tiedExplain: "Tied into the game: hiding them is possible, but may break something.",
  stayExplain: "The game needs these. They stay.",
  restingExplain: "Things standing on what goes. They go with it:",
  restingStaysExplain: "Things standing on it that stay, and then float:",
  gameExplain: "Objects of the game itself that a mod moved. They stay.",
  hideSingles: (mod) => `Hide single parts of ${mod} here`,
  keptHere: "kept here",
  listHint: "Click to pick · Ctrl or Shift for more",
  picked: (count) => `${count} picked`,
  hidePicked: "Hide",
  showPicked: "Show again",
  clearPicked: "Pick none",
  oneByOne: "One by one",
  member: (name, index) => `${name} · ${index}`,
  hiddenOf: (hidden, total) => (total === 1 ? "hidden" : `${hidden} of ${total} hidden`),
  stays: "needed, stays",
  gameOwn: "the game's own",
  hideOne: (name) => `Hide ${name}`,
  showOne: (name) => `Show ${name} again`,
  left: (count, mod) =>
    count === 1
      ? `1 ${mod} part remains and still collides here`
      : `${count} ${mod} parts remain and still collide here`,
  leftKeepBoth: "Keep both for those",
  leftKeepBothTip: "Leaves them as they are. Everything else stays as decided.",
  leftHide: "Hide them anyway",
  leftNeeded: "the game needs them",
  leftOpen: "Show them",
};

const de: CollisionStepTexts = {
  measure: (count) => `Offene Bereiche ausmessen (${count})`,
  measureTip:
    "Liest jeden offenen Bereich im Hintergrund und misst, wie viel Raum jeder Mod dort einnimmt. Wo ein Mod klar größer ist, gibt es einen Vorschlag.",
  take: (count) => `Vorschläge übernehmen (${count})`,
  takeTip:
    "Entscheidet jeden Bereich mit klarem Vorschlag: eine Seite nimmt mindestens 20-mal so viel Raum ein wie die andere. Was das Spiel braucht, bleibt. Rückgängig nimmt alle auf einmal zurück.",
  openInMap: "In Karten-Kollisionen öffnen",
  openInMapTip:
    "Derselbe Bereich in Karten-Kollisionen, in einer großen Ansicht. Mit Zurück kommst du hierher.",
  legendHidden: "schon ausgeblendet",
  alsoHere: "Hier baut auch:",
  showing: (what) => `Gezeigt: ${what}`,
  showAll: "Alles zeigen",
  goingParts: (count, mod) =>
    count === 1
      ? `das 1 Teil von ${mod}, das wegfällt`
      : `die ${count} Teile von ${mod}, die wegfallen`,
  tiedParts: (count, mod) =>
    count === 1
      ? `das 1 eingebundene Teil von ${mod}`
      : `die ${count} eingebundenen Teile von ${mod}`,
  stayingParts: (count, mod) =>
    count === 1
      ? `das 1 Teil von ${mod}, das bleibt`
      : `die ${count} Teile von ${mod}, die bleiben`,
  walkedParts: (count, mod) =>
    count === 1
      ? `das 1 Teil von ${mod}, auf dem Figuren laufen`
      : `die ${count} Teile von ${mod}, auf denen Figuren laufen`,
  leftParts: (count, mod) =>
    count === 1 ? `das 1 übrige Teil von ${mod}` : `die ${count} übrigen Teile von ${mod}`,
  effectGo: (mod, go, total) =>
    go === total
      ? `${total === 1 ? "das 1 Teil" : `alle ${total} Teile`} von ${mod} ${total === 1 ? "fällt" : "fallen"} weg`
      : `${go} von ${total} Teilen von ${mod} fallen weg`,
  effectNothing: (mod) => `von ${mod} kann nichts weg`,
  effectNothingTip: "Das Spiel braucht diese Teile. „Hier beide behalten“ entscheidet den Bereich.",
  badgeTied: (count) => `${count} eingebunden`,
  badgeStay: (count) => `${count} ${count === 1 ? "bleibt" : "bleiben"}`,
  badgeResting: (count) => `${count} darauf ${count === 1 ? "geht" : "gehen"} mit`,
  badgeRestingStays: (count) => `${count} darauf ${count === 1 ? "bleibt" : "bleiben"}`,
  badgeWalked: (count) => `Figuren laufen auf ${count}`,
  badgeGame: (count) => `${count} vom Spiel`,
  showThese: "Diese zeigen",
  showWhich: "Zeigen, welche",
  tiedExplain: "Ins Spiel eingebunden: ausblenden geht, kann aber etwas stören.",
  stayExplain: "Das Spiel braucht diese Teile. Sie bleiben.",
  restingExplain: "Was auf dem steht, das wegfällt. Es geht mit:",
  restingStaysExplain: "Was darauf steht und bleibt, schwebt dann:",
  gameExplain: "Objekte des Spiels selbst, die ein Mod verschoben hat. Sie bleiben.",
  hideSingles: (mod) => `Einzelne Teile von ${mod} hier ausblenden`,
  keptHere: "hier behalten",
  listHint: "Klicken wählt aus · Strg oder Umschalt für mehrere",
  picked: (count) => `${count} ausgewählt`,
  hidePicked: "Ausblenden",
  showPicked: "Wieder zeigen",
  clearPicked: "Auswahl aufheben",
  oneByOne: "Einzeln",
  member: (name, index) => `${name} · ${index}`,
  hiddenOf: (hidden, total) =>
    total === 1 ? "ausgeblendet" : `${hidden} von ${total} ausgeblendet`,
  stays: "gebraucht, bleibt",
  gameOwn: "vom Spiel",
  hideOne: (name) => `${name} ausblenden`,
  showOne: (name) => `${name} wieder zeigen`,
  left: (count, mod) =>
    count === 1
      ? `1 Teil von ${mod} bleibt und kollidiert hier noch`
      : `${count} Teile von ${mod} bleiben und kollidieren hier noch`,
  leftKeepBoth: "Dort beide behalten",
  leftKeepBothTip: "Lässt sie, wie sie sind. Alles andere bleibt, wie entschieden.",
  leftHide: "Trotzdem ausblenden",
  leftNeeded: "das Spiel braucht sie",
  leftOpen: "Zeigen",
};

export const collisionStepTexts: Record<Lang, CollisionStepTexts> = { en, de };

/** The collisions step's texts in the window's language. */
export function useCollisionStepTexts(): CollisionStepTexts {
  return collisionStepTexts[useSession().lang];
}
