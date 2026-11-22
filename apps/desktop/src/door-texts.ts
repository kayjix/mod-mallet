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
  /** The door in `place` leads to the door of the mod `to`. */
  otherSide: (place: string, to: string) => string;
  arrivalMoves: string;
  paths: (count: number, to: string) => string;
  lost: (place: string, from: string) => string;
  nothingElse: string;
  /** Where the teleports go: the plugin of moves. */
  viaMoves: string;
  /** The paths are re-attached in a later version. */
  pathsLater: string;
  /** Characters reach the kept door. */
  keptReached: string;
  /** No path leads through the kept door. */
  keptNotOnPaths: string;
  /** In the list of moves: a door led to another door. */
  ledTo: (door: string) => string;
  /** Looks at one door alone in the 3D view. */
  lookAlone: string;
  /** Makes one door blink in the 3D view. */
  locate: string;
  /** A door the door decision hides, in its card in the view. */
  hiddenByDoors: string;
  showAgain: string;
  /** Every door of the spot is hidden. */
  wayLost: string;
  /** A spot's choice that keeps one door, in the worklist. */
  statusKept: (door: string) => string;
  /** The collisions of the mods whose doors these are. */
  pairLink: (first: string, second: string) => string;
  /** The version the game uses is another plugin's. */
  changedBy: (plugin: string) => string;
  /** A door whose way leads to a door that is not there. */
  broken: string;
  /** The doors lead to places apart. */
  apart: string;
  keepOnly: string;
  keptOnly: string;
  /** Where the kept door leads, and that the door there leads back. */
  outcomeLeadsBack: (place: string) => string;
  /** Where the kept door leads; the door there leads elsewhere. */
  outcomeLeads: (place: string) => string;
  /** The kept door leads nowhere, as none of them did. */
  outcomeOpensOnly: string;
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
    otherSide: (place, to) => `The door in ${place} leads to the door of ${to} instead.`,
    arrivalMoves: "You come out in front of it.",
    paths: (count, to) =>
      count === 1
        ? `1 path of the characters is to be attached to ${to} (later).`
        : `${count} paths of the characters are to be attached to ${to} (later).`,
    lost: (place, from) => `${place} can then no longer be entered here: only ${from} led there.`,
    nothingElse: "Hiding is all it takes: nothing else used the other doors.",
    viaMoves:
      "The patch hides the other doors. Leading the doors on the other side to the kept one goes into the plugin Mod Mallet - Moves: write and install it on the patch page.",
    pathsLater:
      "Re-attaching the paths of the characters comes with the repair of the paths in a later version. Until then they walk to where the hidden door stood, right beside the kept one.",
    keptReached: "Characters reach the kept door: their paths lead through it.",
    keptNotOnPaths:
      "No path of the characters leads through the kept door yet; they use the paths of the hidden one, beside it.",
    ledTo: (door) => `leads to the door ${door}`,
    lookAlone: "Look at this door alone in the view; the others show as ghosts",
    locate: "Show where this door is: it blinks in the view for a moment",
    hiddenByDoors: "Hidden by the door decision here.",
    showAgain: "Show again",
    wayLost:
      "Every door here is hidden, so the way in is lost. The check of the ways below shows where characters can no longer go; the repair of the paths is on the patch page.",
    statusKept: (door) => `${door} kept`,
    pairLink: (first, second) => `Everything ${first} and ${second} collide at`,
    changedBy: (plugin) => `as ${plugin} has it`,
    broken: "Leads to a door that is not there, so it only opens",
    apart:
      "These doors lead to different places; the game can't take you to both. Keep both, or keep one with its own way and give up the other's.",
    keepOnly: "Keep only this one",
    keptOnly: "Kept with its own way; the others are hidden",
    outcomeLeadsBack: (place) => `It now leads to ${place}; the door there leads back to it.`,
    outcomeLeads: (place) => `It now leads to ${place}.`,
    outcomeOpensOnly: "It only opens, as all of them do.",
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
    otherSide: (place, to) => `Die Tür in ${place} führt stattdessen zur Tür von ${to}.`,
    arrivalMoves: "Du kommst vor ihr heraus.",
    paths: (count, to) =>
      count === 1
        ? `1 Weg der Figuren wird an ${to} angeschlossen (später).`
        : `${count} Wege der Figuren werden an ${to} angeschlossen (später).`,
    lost: (place, from) =>
      `${place} ist dann hier nicht mehr zu betreten: Nur ${from} führte dorthin.`,
    nothingElse: "Ausblenden genügt: Nichts anderes hat die anderen Türen benutzt.",
    viaMoves:
      "Der Patch blendet die anderen Türen aus. Die Türen auf der anderen Seite zur behaltenen zu führen, kommt in das Plugin Mod Mallet - Moves: Schreibe und installiere es auf der Patch-Seite.",
    pathsLater:
      "Die Wege der Figuren neu anzuschließen, kommt mit der Reparatur der Wege in einer späteren Version. Bis dahin gehen sie dorthin, wo die ausgeblendete Tür stand, direkt neben der behaltenen.",
    keptReached: "Figuren erreichen die behaltene Tür: Ihre Wege führen durch sie.",
    keptNotOnPaths:
      "Noch führt kein Weg der Figuren durch die behaltene Tür; sie nehmen die Wege der ausgeblendeten, direkt daneben.",
    ledTo: (door) => `führt zur Tür ${door}`,
    lookAlone: "Nur diese Tür in der Ansicht zeigen; die anderen erscheinen als Geister",
    locate: "Zeigen, wo diese Tür ist: Sie blinkt kurz in der Ansicht",
    hiddenByDoors: "Von der Tür-Entscheidung hier ausgeblendet.",
    showAgain: "Wieder zeigen",
    wayLost:
      "Alle Türen hier sind ausgeblendet, der Weg hinein ist also verloren. Die Prüfung der Wege unten zeigt, wohin Figuren nicht mehr kommen; die Reparatur der Wege steht auf der Patch-Seite.",
    statusKept: (door) => `${door} behalten`,
    pairLink: (first, second) => `Alles, wo ${first} und ${second} kollidieren`,
    changedBy: (plugin) => `wie ${plugin} sie hat`,
    broken: "Führt zu einer Tür, die es nicht gibt, und geht deshalb nur auf",
    apart:
      "Diese Türen führen an verschiedene Orte; das Spiel kann dich nicht an beide bringen. Behalte beide, oder behalte eine mit ihrem eigenen Weg und gib den der anderen auf.",
    keepOnly: "Nur diese behalten",
    keptOnly: "Mit ihrem eigenen Weg behalten; die anderen sind ausgeblendet",
    outcomeLeadsBack: (place) => `Sie führt dann nach ${place}; die Tür dort führt zu ihr zurück.`,
    outcomeLeads: (place) => `Sie führt dann nach ${place}.`,
    outcomeOpensOnly: "Sie geht nur auf, wie alle hier.",
    doorOf: (mod) => `die Tür von ${mod}`,
  },
};
