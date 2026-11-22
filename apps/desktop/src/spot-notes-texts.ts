// Texts of the spot panel's notes as small badges, and of the switch that
// shows them in full: kept apart from `i18n.ts`, as every package does.

import type { Lang } from "./i18n";

export interface SpotNotesTexts {
  /** The switch beside "Fast forward" that shows every note in full. */
  showNotes: string;
  showNotesHint: string;
  /** A badge's tooltip ends with how to open it. */
  openHint: string;
  /** Characters walk on the part a choice would hide. */
  walked: string;
  /** Hiding leaves every place nearby reachable. */
  reachAll: (checked: number) => string;
  /** Hiding changes the ways to some places. */
  reachChanges: (count: number) => string;
  /** More that keeping a door changes. */
  doorChanges: (count: number) => string;
  /** Ways that keeping a door loses. */
  doorLost: (count: number) => string;
  /** What the repair of the paths changes, short. */
  repaired: string;
  /** Moving a part aside instead of hiding it (ADR 0060). */
  moveInstead: string;
  /** A door's checks, short; the full sentence is the tooltip. */
  leadsBack: string;
  leadsElsewhere: string;
  walkable: string;
  notWalkable: string;
  otherSide: string;
  otherSideNot: string;
}

export const spotNotesTexts: Record<Lang, SpotNotesTexts> = {
  en: {
    showNotes: "Show notes",
    showNotesHint:
      "Show every note in full: whether characters walk on a part, which places they still reach once it is hidden, and what keeping a door changes. Off, each note is a small badge in its card; point at one to read it, click it to open it.",
    openHint: "Click to read it in full.",
    walked: "Characters walk here",
    reachAll: (checked) =>
      checked === 1 ? "Its 1 place stays in reach" : `All ${checked} places stay in reach`,
    reachChanges: (count) => (count === 1 ? "1 way changes" : `${count} ways change`),
    doorChanges: (count) => (count === 1 ? "1 more change" : `${count} more changes`),
    doorLost: (count) => (count === 1 ? "1 way is lost" : `${count} ways are lost`),
    repaired: "Paths repaired",
    moveInstead: "Or move a part aside",
    leadsBack: "Leads back",
    leadsElsewhere: "Leads elsewhere",
    walkable: "Walked through",
    notWalkable: "Nobody walks through",
    otherSide: "Also beyond",
    otherSideNot: "Not beyond",
  },
  de: {
    showNotes: "Hinweise zeigen",
    showNotesHint:
      "Jeden Hinweis ganz zeigen: ob Figuren über ein Teil laufen, welche Orte sie noch erreichen, wenn es ausgeblendet ist, und was das Behalten einer Tür ändert. Aus, ist jeder Hinweis ein kleines Abzeichen in seiner Karte; zeig darauf, um ihn zu lesen, klick, um ihn zu öffnen.",
    openHint: "Klicken, um ihn ganz zu lesen.",
    walked: "Hier laufen Figuren",
    reachAll: (checked) =>
      checked === 1 ? "Sein 1 Ort bleibt erreichbar" : `Alle ${checked} Orte bleiben erreichbar`,
    reachChanges: (count) => (count === 1 ? "1 Weg ändert sich" : `${count} Wege ändern sich`),
    doorChanges: (count) => (count === 1 ? "1 weitere Änderung" : `${count} weitere Änderungen`),
    doorLost: (count) => (count === 1 ? "1 Weg geht verloren" : `${count} Wege gehen verloren`),
    repaired: "Wege repariert",
    moveInstead: "Oder ein Teil verschieben",
    leadsBack: "Führt zurück",
    leadsElsewhere: "Führt woandershin",
    walkable: "Figuren gehen durch",
    notWalkable: "Niemand geht durch",
    otherSide: "Auch dahinter",
    otherSideNot: "Dahinter nicht",
  },
};
