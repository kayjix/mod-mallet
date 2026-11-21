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
  chosen: "The patch will carry the repaired paths. Removing the patch brings the old ones back.",
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
    "Der Patch bringt die reparierten Wege mit. Entfernst du den Patch, sind die alten wieder da.",
};

export const repairTexts: Record<Lang, RepairTexts> = { en, de };
