// The texts of the report of what was decided (ADR 0050), shown at the end
// of the review and on the patch page, and written next to the patch.

import { useSession } from "./Actions";
import type { Lang } from "./i18n";

export interface ReportTexts {
  title: string;
  intro: string;
  none: string;
  pairLine: (places: number, decided: number, open: number) => string;
  hiddenCount: (name: string, count: number) => string;
  nothingHidden: string;
  placeOpen: string;
  placeKept: string;
  placeHidden: (name: string) => string;
  placeMixed: string;
  exceptions: (count: number) => string;
  stays: (names: string) => string;
  elsewhere: string;
  leftoversTitle: string;
  leftover: (mod: string, total: number, left: number) => string;
  save: string;
  saved: (path: string) => string;
  savedWithPatch: string;
  fileTitle: (when: string) => string;
}

const en: ReportTexts = {
  title: "Report",
  intro:
    "What your decisions do, pair by pair of mods: what is hidden, what was kept, what is still open, and what stays because the game needs it.",
  none: "Nothing is decided yet.",
  pairLine: (places, decided, open) =>
    `${places} ${places === 1 ? "place" : "places"}: ${decided} decided, ${open} open`,
  hiddenCount: (name, count) => `${name}: ${count} hidden`,
  nothingHidden: "nothing hidden",
  placeOpen: "open",
  placeKept: "both stay",
  placeHidden: (name) => `${name}'s hidden`,
  placeMixed: "decided differently by section",
  exceptions: (count) => (count === 1 ? "1 exception" : `${count} exceptions`),
  stays: (names) => `stays, the game needs it: ${names}`,
  elsewhere: "Hidden on their own elsewhere",
  leftoversTitle: "You might not need these any more",
  leftover: (mod, total, left) =>
    `Of ${mod}, only ${left} of its ${total} objects are left in your game. You might not need it any more; uninstalling it in Vortex is up to you.`,
  save: "Save as a file",
  saved: (path) => `Saved: ${path}`,
  savedWithPatch: "The report is also saved next to the patch whenever you create it.",
  fileTitle: (when) => `Mod Mallet: what the patch does (${when})`,
};

const de: ReportTexts = {
  title: "Bericht",
  intro:
    "Was deine Entscheidungen bewirken, Mod-Paar für Mod-Paar: was ausgeblendet ist, was behalten wurde, was noch offen ist, und was bleibt, weil das Spiel es braucht.",
  none: "Noch ist nichts entschieden.",
  pairLine: (places, decided, open) =>
    `${places} ${places === 1 ? "Bereich" : "Bereiche"}: ${decided} entschieden, ${open} offen`,
  hiddenCount: (name, count) => `${name}: ${count} ausgeblendet`,
  nothingHidden: "nichts ausgeblendet",
  placeOpen: "offen",
  placeKept: "beide bleiben",
  placeHidden: (name) => `${name} ausgeblendet`,
  placeMixed: "je Abschnitt verschieden entschieden",
  exceptions: (count) => (count === 1 ? "1 Ausnahme" : `${count} Ausnahmen`),
  stays: (names) => `bleibt, das Spiel braucht es: ${names}`,
  elsewhere: "Anderswo einzeln ausgeblendet",
  leftoversTitle: "Diese brauchst du vielleicht nicht mehr",
  leftover: (mod, total, left) =>
    `Von ${mod} sind in deinem Spiel nur noch ${left} von ${total} Objekten übrig. Vielleicht brauchst du ihn nicht mehr; ob du ihn in Vortex deinstallierst, entscheidest du.`,
  save: "Als Datei speichern",
  saved: (path) => `Gespeichert: ${path}`,
  savedWithPatch:
    "Der Bericht wird auch neben dem Patch gespeichert, jedes Mal wenn du ihn erstellst.",
  fileTitle: (when) => `Mod Mallet: was der Patch bewirkt (${when})`,
};

export const reportTexts: Record<Lang, ReportTexts> = { en, de };

/** The report's texts in the window's language. */
export function useReportTexts(): ReportTexts {
  return reportTexts[useSession().lang];
}
