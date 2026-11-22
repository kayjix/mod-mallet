// Texts of the repair of paths over hidden objects (ADR 0058).

import type { Lang } from "./i18n";

export interface RepairTexts {
  /** The button that shows the repair and chooses it. */
  repair: string;
  /** The button once chosen: it undoes the choice. */
  repaired: string;
  working: string;
  failed: string;
  /** What the repair does, with the colours of the view. */
  summary: (removed: number, added: number) => string;
  /** No path could be laid below: characters go around. */
  around: (count: number) => string;
  water: (count: number) => string;
  lostDoors: (count: number) => string;
  /** A mesh whose record cannot be copied. */
  notWritable: string;
  legendRemoved: string;
  legendAdded: string;
  /** While the ways after the repair are followed. */
  checkingReach: string;
  /** What the repair makes reachable again that cutting the paths would lose. */
  reachedAgain: (names: string[]) => string;
  /** Said once chosen: what the patch will do. */
  chosen: string;
  panelTitle: string;
  panelSummary: (count: number) => string;
  write: string;
  writing: string;
  install: string;
  installHint: string;
  dropAll: string;
  nothingWritten: string;
  written: (count: number, masters: string) => string;
  outdated: string;
  refusedTitle: (count: number) => string;
  reasons: {
    "nothing-to-repair": string;
    "unknown-field": (field: string) => string;
    "not-found": string;
    "unsupported-game": string;
  };
  limits: string;
}

const piece = (count: number) => (count === 1 ? "1 piece of path" : `${count} pieces of path`);
const stueck = (count: number) => (count === 1 ? "1 Wegstück" : `${count} Wegstücke`);

const en: RepairTexts = {
  repair: "Repair the paths",
  repaired: "Paths repaired · undo",
  working: "Working out new paths…",
  failed: "Mod Mallet could not work out new paths here.",
  summary: (removed, added) =>
    added === 0
      ? `${piece(removed)} on what goes are taken out (red). Nothing to walk on lies below, so characters go around.`
      : `${piece(removed)} on what goes are taken out (red), ${added} new ones laid on the ground below (green). Characters take that way instead.`,
  around: (count) =>
    count === 1
      ? "Under one of the parts nothing can be walked on: characters go around it."
      : `Under ${count} of the parts nothing can be walked on: characters go around them.`,
  water: (count) => `${piece(count)} of the new ones run through water: characters swim there.`,
  lostDoors: (count) =>
    count === 1
      ? "One door stood on what goes: characters other than you can no longer walk through it."
      : `${count} doors stood on what goes: characters other than you can no longer walk through them.`,
  notWritable: "One of these paths cannot be copied, so it stays as it is.",
  legendRemoved: "taken out",
  legendAdded: "laid new",
  checkingReach: "Following the ways characters take after the repair…",
  reachedAgain: (names) => `With the repair, characters reach again: ${names.join(", ")}.`,
  chosen:
    "Chosen. On the patch page, a small plugin carries the repaired paths into the game; removing it brings the old ones back.",
  panelTitle: "Repaired paths",
  panelSummary: (count) =>
    count === 1 ? "1 net of paths to repair" : `${count} nets of paths to repair`,
  write: "Write the plugin",
  writing: "Writing…",
  install: "Install in Vortex",
  installHint:
    "Vortex installs the plugin like a downloaded mod. Remove it there to bring the old paths back.",
  dropAll: "Repair none",
  nothingWritten: "None of these paths could be repaired; nothing was written.",
  written: (count, masters) =>
    `Written: ${count === 1 ? "1 repaired net" : `${count} repaired nets`}, loading after ${masters}.`,
  outdated: "The choices changed since; write the plugin again.",
  refusedTitle: (count) =>
    count === 1 ? "1 net is not repaired:" : `${count} nets are not repaired:`,
  reasons: {
    "nothing-to-repair": "nothing over hidden objects runs on it any more",
    "unknown-field": (field) =>
      `its cell has a field Mod Mallet does not know how to copy (${field})`,
    "not-found": "its cell was not found in the plugins",
    "unsupported-game": "Mod Mallet writes plugins only for Skyrim Special Edition",
  },
  limits:
    "The repair lays new paths on the ground where the hidden objects stood. It writes no cover for fights and leaves the game's map of long ways as it is; a plugin loaded later that changes the same paths undoes it.",
};

const de: RepairTexts = {
  repair: "Wege reparieren",
  repaired: "Wege repariert · rückgängig",
  working: "Neue Wege werden berechnet …",
  failed: "Mod Mallet konnte hier keine neuen Wege berechnen.",
  summary: (removed, added) =>
    added === 0
      ? `${stueck(removed)} auf dem, was weg kommt, werden herausgenommen (rot). Darunter liegt nichts zum Laufen, also gehen Figuren außen herum.`
      : `${stueck(removed)} auf dem, was weg kommt, werden herausgenommen (rot), ${added} neue auf den Boden darunter gelegt (grün). Figuren nehmen dann diesen Weg.`,
  around: (count) =>
    count === 1
      ? "Unter einem der Teile kann man nicht laufen: Figuren gehen außen herum."
      : `Unter ${count} der Teile kann man nicht laufen: Figuren gehen außen herum.`,
  water: (count) => `${stueck(count)} der neuen führen durch Wasser: Dort schwimmen Figuren.`,
  lostDoors: (count) =>
    count === 1
      ? "Eine Tür stand auf dem, was weg kommt: Außer dir kann niemand mehr hindurchgehen."
      : `${count} Türen standen auf dem, was weg kommt: Außer dir kann niemand mehr hindurchgehen.`,
  notWritable: "Einer dieser Wege lässt sich nicht kopieren und bleibt, wie er ist.",
  legendRemoved: "herausgenommen",
  legendAdded: "neu gelegt",
  checkingReach: "Die Wege der Figuren nach der Reparatur werden verfolgt …",
  reachedAgain: (names) => `Mit der Reparatur erreichen Figuren wieder: ${names.join(", ")}.`,
  chosen:
    "Gewählt. Auf der Patch-Seite bringt ein kleines Plugin die reparierten Wege ins Spiel; entfernst du es, sind die alten wieder da.",
  panelTitle: "Reparierte Wege",
  panelSummary: (count) =>
    count === 1 ? "1 Wegenetz zu reparieren" : `${count} Wegenetze zu reparieren`,
  write: "Plugin schreiben",
  writing: "Wird geschrieben …",
  install: "In Vortex installieren",
  installHint:
    "Vortex installiert das Plugin wie eine heruntergeladene Mod. Entferne es dort, um die alten Wege zurückzuholen.",
  dropAll: "Keine reparieren",
  nothingWritten: "Keiner dieser Wege ließ sich reparieren; nichts wurde geschrieben.",
  written: (count, masters) =>
    `Geschrieben: ${count === 1 ? "1 repariertes Netz" : `${count} reparierte Netze`}, geladen nach ${masters}.`,
  outdated: "Die Wahl hat sich seitdem geändert; schreibe das Plugin neu.",
  refusedTitle: (count) =>
    count === 1 ? "1 Netz wird nicht repariert:" : `${count} Netze werden nicht repariert:`,
  reasons: {
    "nothing-to-repair": "darüber läuft kein Weg mehr über versteckten Objekten",
    "unknown-field": (field) =>
      `seine Zelle hat ein Feld, das Mod Mallet nicht zu kopieren weiß (${field})`,
    "not-found": "seine Zelle wurde in den Plugins nicht gefunden",
    "unsupported-game": "Mod Mallet schreibt Plugins nur für Skyrim Special Edition",
  },
  limits:
    "Die Reparatur legt neue Wege auf den Boden, wo die versteckten Objekte standen. Deckung für Kämpfe schreibt sie nicht, und die Karte des Spiels für weite Wege bleibt, wie sie ist; ein später geladenes Plugin, das dieselben Wege ändert, macht sie rückgängig.",
};

export const repairTexts: Record<Lang, RepairTexts> = { en, de };
