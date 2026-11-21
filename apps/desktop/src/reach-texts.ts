// Texts of what hiding objects does to the ways characters take (ADR 0059):
// kept apart from `i18n.ts`, as every package of that plan does.

import type { Destination } from "./generated/walkway-reach";
import type { Lang } from "./i18n";

export interface ReachTexts {
  /** What a destination is, for a sentence: "the door Door01". */
  what: (destination: Destination) => string;
  /** Characters could no longer use it: it stands on what is hidden. */
  gone: (what: string) => string;
  /** No way leads there any more. */
  cutOff: (what: string) => string;
  /** A much longer way, or only through the places around. */
  longer: (what: string, from: string, before: number, after: number | null) => string;
  /** More changes than are listed. */
  more: (count: number) => string;
  /** Everything nearby stays reachable. */
  allReached: (checked: number) => string;
  /** While the ways are followed. */
  checking: string;
  /** What the lines in the 3D view mean. */
  legend: string;
}

/** A name in quotes, or nothing. */
const quoted = (name: string | null | undefined, open: string, close: string) =>
  name ? ` ${open}${name}${close}` : "";

const en: ReachTexts = {
  what: (destination) => {
    const name = quoted(destination.name, "“", "”");
    switch (destination.kind) {
      case "door":
        return `the door${name}`;
      case "furniture":
        return name === "" ? "a bed, seat or work place" : `the bed, seat or work place${name}`;
      case "idle-marker":
        return `a spot where characters stand or work${name}`;
      case "map-marker":
        return `the place of the map${name}`;
      case "used":
        return destination.usedBy
          ? `a spot that “${destination.usedBy}” sends characters to`
          : "a spot a quest or routine sends characters to";
    }
  },
  gone: (what) =>
    `${capital(what)} stands on what would be hidden: characters could no longer use it.`,
  cutOff: (what) => `Characters could no longer reach ${what}.`,
  longer: (what, from, before, after) =>
    after === null
      ? `From ${from}, characters would reach ${what} only by a way through the places around (${before} m before).`
      : `From ${from}, characters would reach ${what} only by a long way round: ${after} m instead of ${before} m.`,
  more: (count) => (count === 1 ? "and 1 more" : `and ${count} more`),
  allReached: (checked) =>
    checked === 1
      ? "Characters would still reach the 1 place they go to nearby."
      : `Characters would still reach all ${checked} places they go to nearby.`,
  checking: "Following the ways characters take…",
  legend: "grey: the way now · green: the way round after · red: no way left",
};

const de: ReachTexts = {
  what: (destination) => {
    const name = quoted(destination.name, "„", "“");
    switch (destination.kind) {
      case "door":
        return `die Tür${name}`;
      case "furniture":
        return name === "" ? "ein Bett, Sitz oder Arbeitsplatz" : `das Möbel${name}`;
      case "idle-marker":
        return `eine Stelle zum Stehen oder Arbeiten${name}`;
      case "map-marker":
        return `der Ort der Karte${name}`;
      case "used":
        return destination.usedBy
          ? `eine Stelle für „${destination.usedBy}“`
          : "eine Stelle für eine Quest oder einen Tagesablauf";
    }
  },
  gone: (what) =>
    `${capital(what)} steht auf dem, was versteckt würde: Figuren könnten es nicht mehr benutzen.`,
  cutOff: (what) => `Figuren kämen nicht mehr zu ${dative(what)}.`,
  longer: (what, from, before, after) =>
    after === null
      ? `Figuren kämen von ${dative(from)} zu ${dative(what)} nur noch auf einem Weg durch die Umgebung (vorher ${before} m).`
      : `Figuren kämen von ${dative(from)} zu ${dative(what)} nur noch auf einem Umweg: ${after} m statt ${before} m.`,
  more: (count) => (count === 1 ? "und 1 weiterer" : `und ${count} weitere`),
  allReached: (checked) =>
    checked === 1
      ? "Figuren kämen weiter zu dem 1 Ort in der Nähe, zu dem sie gehen."
      : `Figuren kämen weiter zu allen ${checked} Orten in der Nähe, zu denen sie gehen.`,
  checking: "Die Wege der Figuren werden verfolgt …",
  legend: "grau: der Weg jetzt · grün: der Umweg danach · rot: kein Weg mehr",
};

/** The first letter in capitals. */
function capital(text: string): string {
  return text.charAt(0).toUpperCase() + text.slice(1);
}

/** A German phrase of `what` after "zu" or "von": its article in the dative. */
function dative(text: string): string {
  return text
    .replace(/^die Tür/, "der Tür")
    .replace(/^das Möbel/, "dem Möbel")
    .replace(/^der Ort/, "dem Ort")
    .replace(/^eine Stelle/, "einer Stelle")
    .replace(/^ein Bett, Sitz oder Arbeitsplatz/, "einem Bett, Sitz oder Arbeitsplatz");
}

export const reachTexts: Record<Lang, ReachTexts> = { en, de };
