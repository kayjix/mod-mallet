// Texts of the stacked doors (ADR 0067): kept apart from `i18n.ts`, for
// someone who installs mods but has never opened a modding tool.

import type { Lang } from "./i18n";

export interface DoorTexts {
  /** How many doors stand at the spot. */
  intro: (count: number) => string;
  flicker: string;
  front: string;
  /** How far a door stands from the one suggested to keep. */
  away: (centimetres: number) => string;
  leadsTo: (place: string) => string;
  opensOnly: string;
  leadsBack: string;
  leadsElsewhere: string;
  walkable: string;
  notWalkable: string;
  otherSideWalkable: string;
  otherSideNotWalkable: string;
  suggested: string;
  suggestedHint: string;
  mergeInto: string;
  merged: string;
  keepAll: string;
  keptAll: string;
  /** The heading of what keeping one door means. */
  ifKept: (door: string) => string;
  usersMove: (from: string, to: string) => string;
  otherSide: (place: string, to: string) => string;
  arrivalMoves: string;
  takesWay: (door: string, place: string) => string;
  paths: (count: number, to: string) => string;
  lost: (place: string, from: string) => string;
  nothingElse: string;
  notYet: string;
  lookWithout: string;
  /** A door named by its mod. */
  doorOf: (mod: string) => string;
}

export const doorTexts: Record<Lang, DoorTexts> = {
  en: {
    intro: (count) =>
      `${count} doors stand here almost on the same spot. The game shows all of them, and you open the one in front.`,
    flicker: "Two of them stand in exactly the same place and flicker through each other.",
    front: "in front",
    away: (centimetres) => `${centimetres} cm from the first`,
    leadsTo: (place) => `Leads to ${place}`,
    opensOnly: "Only opens; leads nowhere else",
    leadsBack: "The door there leads back here",
    leadsElsewhere: "The door there leads back through another door",
    walkable: "Characters walk through it",
    notWalkable: "No path of the characters goes through it",
    otherSideWalkable: "and through the door on the other side",
    otherSideNotWalkable: "but not through the door on the other side",
    suggested: "Suggested",
    suggestedHint:
      "Its other side leads back, characters walk through it, and the most other things lead to it.",
    mergeInto: "Merge into this one",
    merged: "Kept; the others are hidden",
    keepAll: "Keep all of them",
    keptAll: "All of them stay",
    ifKept: (door) => `If you keep ${door}:`,
    usersMove: (from, to) => `Characters who used ${from} now use ${to}.`,
    otherSide: (place, to) => `The door in ${place} leads to ${to} instead.`,
    arrivalMoves: "You come out in front of it.",
    takesWay: (door, place) => `${door} takes over the way to ${place}.`,
    paths: (count, to) =>
      count === 1
        ? `1 path of the characters is attached to ${to}.`
        : `${count} paths of the characters are attached to ${to}.`,
    lost: (place, from) => `${place} can then no longer be entered here: only ${from} led there.`,
    nothingElse: "Hiding is all it takes: nothing else used the other doors.",
    notYet:
      "Hiding works now. Pointing the other side and the paths at the kept door needs a small plugin that Mod Mallet writes in a later version. Until then the hidden door still works from the other side and lets you out right beside the kept one.",
    lookWithout: "Look at the place without this door, without deciding",
    doorOf: (mod) => `the door of ${mod}`,
  },
  de: {
    intro: (count) =>
      `${count} Türen stehen hier fast an derselben Stelle. Das Spiel zeigt alle, und du öffnest die, die vorne steht.`,
    flicker: "Zwei davon stehen genau an derselben Stelle und flackern durcheinander.",
    front: "vorne",
    away: (centimetres) => `${centimetres} cm von der ersten`,
    leadsTo: (place) => `Führt nach ${place}`,
    opensOnly: "Geht nur auf; führt nirgendwohin",
    leadsBack: "Die Tür dort führt hierher zurück",
    leadsElsewhere: "Die Tür dort führt durch eine andere Tür zurück",
    walkable: "Figuren gehen durch sie hindurch",
    notWalkable: "Kein Weg der Figuren führt durch sie",
    otherSideWalkable: "und durch die Tür auf der anderen Seite",
    otherSideNotWalkable: "aber nicht durch die Tür auf der anderen Seite",
    suggested: "Vorschlag",
    suggestedHint:
      "Ihre andere Seite führt zurück, Figuren gehen durch sie, und am meisten anderes führt zu ihr.",
    mergeInto: "In diese zusammenführen",
    merged: "Behalten; die anderen sind ausgeblendet",
    keepAll: "Alle behalten",
    keptAll: "Alle bleiben",
    ifKept: (door) => `Wenn du ${door} behältst:`,
    usersMove: (from, to) => `Figuren, die ${from} benutzt haben, benutzen jetzt ${to}.`,
    otherSide: (place, to) => `Die Tür in ${place} führt stattdessen zu ${to}.`,
    arrivalMoves: "Du kommst vor ihr heraus.",
    takesWay: (door, place) => `${door} übernimmt den Weg nach ${place}.`,
    paths: (count, to) =>
      count === 1
        ? `1 Weg der Figuren wird an ${to} angeschlossen.`
        : `${count} Wege der Figuren werden an ${to} angeschlossen.`,
    lost: (place, from) =>
      `${place} ist dann hier nicht mehr zu betreten: Nur ${from} führte dorthin.`,
    nothingElse: "Ausblenden genügt: Nichts anderes hat die anderen Türen benutzt.",
    notYet:
      "Ausblenden geht schon jetzt. Die andere Seite und die Wege auf die behaltene Tür umzustellen, braucht ein kleines Plugin, das Mod Mallet in einer späteren Version schreibt. Bis dahin funktioniert die ausgeblendete Tür von der anderen Seite aus weiter und lässt dich direkt neben der behaltenen heraus.",
    lookWithout: "Den Ort ohne diese Tür ansehen, ohne zu entscheiden",
    doorOf: (mod) => `die Tür von ${mod}`,
  },
};
