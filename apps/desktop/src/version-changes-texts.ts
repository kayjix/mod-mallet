// The texts of the banner about what a game or mod update changed
// (ADR 0079), in every language of the app.

import type { Lang } from "./i18n";

export interface VersionChangesTexts {
  /** The heading: since when, and what went from where to where. */
  since: (date: string | null) => string;
  game: (from: string, to: string) => string;
  extender: (from: string, to: string) => string;
  plugins: (count: number) => string;
  mods: (count: number) => string;
  /** What no longer fits, and how bad that is. */
  unfit: (count: number) => string;
  allFit: string;
  /** What usually happens next. */
  waitNote: string;
  /** Opens the task list. */
  toTasks: string;
  /** Hides the banner until the next change. */
  dismiss: string;
  /** The list of updated plugins and mods, folded. */
  details: string;
}

export const versionChangesTexts: Record<Lang, VersionChangesTexts> = {
  en: {
    since: (date) =>
      date === null ? "Since the game was last started" : `Since your last scan (${date})`,
    game: (from, to) => `the game updated from ${from} to ${to}`,
    extender: (from, to) => `the script extender from ${from} to ${to}`,
    plugins: (count) =>
      count === 1
        ? "1 script extender plugin was updated"
        : `${count} script extender plugins were updated`,
    mods: (count) => (count === 1 ? "1 mod was updated" : `${count} mods were updated`),
    unfit: (count) =>
      count === 1 ? "This plugin no longer fits:" : `These ${count} plugins no longer fit:`,
    allFit: "Everything still fits.",
    waitNote:
      "After a game update, most authors publish a fitting version within days. Until then you can switch a plugin's mod off, or roll the game back. Each task says which applies.",
    toTasks: "To the tasks",
    dismiss: "Got it",
    details: "What was updated",
  },
  de: {
    since: (date) =>
      date === null ? "Seit dem letzten Spielstart" : `Seit deinem letzten Scan (${date})`,
    game: (from, to) => `das Spiel wurde von ${from} auf ${to} aktualisiert`,
    extender: (from, to) => `der Script Extender von ${from} auf ${to}`,
    plugins: (count) =>
      count === 1
        ? "1 Script-Extender-Plugin wurde aktualisiert"
        : `${count} Script-Extender-Plugins wurden aktualisiert`,
    mods: (count) =>
      count === 1 ? "1 Mod wurde aktualisiert" : `${count} Mods wurden aktualisiert`,
    unfit: (count) =>
      count === 1 ? "Dieses Plugin passt nicht mehr:" : `Diese ${count} Plugins passen nicht mehr:`,
    allFit: "Alles passt weiterhin.",
    waitNote:
      "Nach einem Update des Spiels bringen die meisten Autoren innerhalb weniger Tage eine passende Version. Bis dahin kannst du die Mod eines Plugins abschalten oder das Spiel zurücksetzen. Bei jeder Aufgabe steht, was davon gilt.",
    toTasks: "Zu den Aufgaben",
    dismiss: "Verstanden",
    details: "Was aktualisiert wurde",
  },
};
