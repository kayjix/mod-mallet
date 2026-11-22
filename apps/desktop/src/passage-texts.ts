// The texts of the passages (ADR 0072): where two mods build the same
// thing twice, decided as a whole. Kept apart from `i18n.ts`, like the
// other pages' texts.

import { useSession } from "./Actions";
import type { MapPassage } from "./generated/rendered-report";
import type { Lang } from "./i18n";

type Severity = MapPassage["severity"];

export interface PassageTexts {
  /** What a passage is, by how serious it is, short. */
  kind: Record<Severity, string>;
  /** What a passage is and why one version has to go, by how serious it is. */
  why: Record<Severity, string>;
  /** A passage named by the place it is nearest. */
  title: (kind: string, place: string) => string;
  /** What each side brings. */
  sides: (first: string, firstParts: number, second: string, secondParts: number) => string;
  spots: (count: number) => string;
  twins: (count: number) => string;
  /** How much of one side stands in the other, in percent. */
  inside: (name: string, percent: number, other: string) => string;
  /** That the choice ends at the passage's edge. */
  bounded: string;
  keepVersion: (name: string) => string;
  /** The way from a spot to the passage it lies in. */
  openPassage: string;
  inPassage: (title: string) => string;
  /** Where the review is among the places, at a passage. */
  passageOf: (index: number, total: number) => string;
  /** The segment of a passage's parts that hang on no spot. */
  looseParts: (index: number) => string;
  /** The worklist's grouping by passage, in Map collisions. */
  groupBy: string;
  groupByHint: string;
  /** The switch in the 3D view: the place now, or with one mod's version only. */
  versionNow: string;
  version: (name: string) => string;
  versionHint: string;
  /** The sections of a passage, under its head. */
  sections: (count: number) => string;
  section: (
    index: number,
    first: string,
    firstParts: number,
    second: string,
    secondParts: number,
  ) => string;
  measuring: string;
  /** Why a passage is decided as a whole, short. */
  whyShort: string;
  /** Parts a decision of the whole passage left, as the game ties them in. */
  held: (count: number) => string;
  heldHint: string;
}

const en: PassageTexts = {
  kind: {
    doubled: "Built twice",
    inside: "Inside each other",
    crossing: "Across each other",
  },
  why: {
    doubled:
      "A big package of duplicates, close to and inside each other. Keep one version or the other: otherwise the game loads the same models twice in the same place, which can make it crash.",
    inside:
      "The two mods build the same thing here, their parts standing inside each other. Keep one version: both together flicker and block the way.",
    crossing:
      "The two mods build the same thing here, their parts crossing each other. Keep one version, or both if they fit together.",
  },
  title: (kind, place) => `${kind}: ${place}`,
  sides: (first, firstParts, second, secondParts) =>
    `${first}: ${firstParts} ${firstParts === 1 ? "part" : "parts"} · ${second}: ${secondParts} ${
      secondParts === 1 ? "part" : "parts"
    }`,
  spots: (count) => (count === 1 ? "1 spot" : `${count} spots`),
  twins: (count) =>
    count === 1
      ? "1 part twice (the same model within a few cm)"
      : `${count} parts twice (the same model within a few cm)`,
  inside: (name, percent, other) => `${percent} % of ${name} stands inside ${other}`,
  bounded:
    "Only the parts inside this passage go, with what rests on them. Everything else of either mod stays.",
  keepVersion: (name) => `Keep ${name}'s version here`,
  openPassage: "Decide the whole passage",
  inPassage: (title) => `Part of a passage: ${title}`,
  passageOf: (index, total) => `Passage ${index} of ${total}`,
  looseParts: (index) => `Section ${index} · the passage's other parts`,
  groupBy: "Passage",
  groupByHint:
    "Where two mods build the same thing twice, each passage is a group of its own: pick its name to decide it as a whole, or a spot in it to decide that one alone. The other spots follow by region.",
  versionNow: "Now",
  version: (name) => `${name}'s version`,
  versionHint:
    "How the place looks in the game: as the patch has it now, or with only one mod's version of the passage. Looking decides nothing.",
  sections: (count) => (count === 1 ? "1 section" : `${count} sections`),
  section: (index, first, firstParts, second, secondParts) =>
    `Section ${index} · ${first}: ${firstParts} · ${second}: ${secondParts}`,
  measuring: "Measuring the passage…",
  whyShort: "Why as a whole",
  held: (count) =>
    count === 1
      ? "1 part stays: the game relies on it"
      : `${count} parts stay: the game relies on them`,
  heldHint: "Pick such a part in the view to see why, and decide it on its own.",
};

const de: PassageTexts = {
  kind: {
    doubled: "Doppelt gebaut",
    inside: "Ineinander",
    crossing: "Übereinander",
  },
  why: {
    doubled:
      "Ein großes Paket voller Duplikate, dicht bei- und ineinander. Behalte die eine oder die andere Fassung: Sonst lädt das Spiel dieselben Modelle doppelt an derselben Stelle, und das kann es abstürzen lassen.",
    inside:
      "Die beiden Mods bauen hier dasselbe, ihre Teile stecken ineinander. Behalte eine Fassung: Beide zusammen flackern und versperren den Weg.",
    crossing:
      "Die beiden Mods bauen hier dasselbe, ihre Teile kreuzen einander. Behalte eine Fassung, oder beide, wenn sie zusammenpassen.",
  },
  title: (kind, place) => `${kind}: ${place}`,
  sides: (first, firstParts, second, secondParts) =>
    `${first}: ${firstParts} ${firstParts === 1 ? "Teil" : "Teile"} · ${second}: ${secondParts} ${
      secondParts === 1 ? "Teil" : "Teile"
    }`,
  spots: (count) => (count === 1 ? "1 Stelle" : `${count} Stellen`),
  twins: (count) =>
    count === 1
      ? "1 Teil doppelt (dasselbe Modell auf wenige cm)"
      : `${count} Teile doppelt (dasselbe Modell auf wenige cm)`,
  inside: (name, percent, other) => `${percent} % von ${name} stecken in ${other}`,
  bounded:
    "Nur die Teile innerhalb dieser Passage verschwinden, mit dem, was auf ihnen liegt. Alles andere beider Mods bleibt.",
  keepVersion: (name) => `Hier die Fassung von ${name} behalten`,
  openPassage: "Die ganze Passage entscheiden",
  inPassage: (title) => `Teil einer Passage: ${title}`,
  passageOf: (index, total) => `Passage ${index} von ${total}`,
  looseParts: (index) => `Abschnitt ${index} · die übrigen Teile der Passage`,
  groupBy: "Passage",
  groupByHint:
    "Wo zwei Mods dasselbe doppelt bauen, ist jede Passage eine eigene Gruppe: Wähl ihren Namen, um sie als Ganzes zu entscheiden, oder eine Stelle darin, um nur diese zu entscheiden. Die übrigen Stellen folgen nach Region.",
  versionNow: "Jetzt",
  version: (name) => `Fassung von ${name}`,
  versionHint:
    "Wie der Ort im Spiel aussieht: so, wie der Patch ihn jetzt hat, oder nur mit der Fassung einer Mod für die Passage. Anschauen entscheidet nichts.",
  sections: (count) => (count === 1 ? "1 Abschnitt" : `${count} Abschnitte`),
  section: (index, first, firstParts, second, secondParts) =>
    `Abschnitt ${index} · ${first}: ${firstParts} · ${second}: ${secondParts}`,
  measuring: "Die Passage wird vermessen…",
  whyShort: "Warum als Ganzes",
  held: (count) =>
    count === 1
      ? "1 Teil bleibt: Das Spiel stützt sich darauf"
      : `${count} Teile bleiben: Das Spiel stützt sich darauf`,
  heldHint: "Wähl so ein Teil in der Ansicht, um zu sehen, warum, und entscheide es einzeln.",
};

export const passageTexts: Record<Lang, PassageTexts> = { en, de };

/** The passages' texts in the language of the session. */
export function usePassageTexts(): PassageTexts {
  const { lang } = useSession();
  return passageTexts[lang];
}
