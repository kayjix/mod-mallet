// Texts of moving objects a little instead of hiding them (ADR 0060), kept
// apart from `i18n.ts` like the texts of every newer part of the window.

import { useSession } from "./Actions";
import type { Lang } from "./i18n";

export interface MovesTexts {
  move: string;
  moveTogether: (count: number) => string;
  moveHint: string;
  close: string;
  locked: string;
  someLocked: (count: number) => string;
  careful: string;
  step: string;
  turnStep: string;
  west: string;
  east: string;
  north: string;
  south: string;
  up: string;
  down: string;
  turnLeft: string;
  turnRight: string;
  /** How far an object is moved, in words; `null` parts are left out. */
  moved: (east: number, north: number, up: number, degrees: number) => string;
  notMoved: string;
  noLongerTouches: (name: string) => string;
  stillOverlaps: (name: string, centimetres: number) => string;
  nowOverlaps: (name: string, centimetres: number) => string;
  touchesNothing: string;
  boxesNote: string;
  walked: (pieces: number) => string;
  back: string;
  intoPlugin: string;
  spotMoved: (count: number) => string;
  spotHint: string;
  /** The patch page. */
  title: string;
  summary: (count: number) => string;
  create: string;
  creating: string;
  install: string;
  installHint: string;
  written: (moved: number, masters: string) => string;
  writtenLight: string;
  outdated: string;
  nothingWritten: string;
  refusedTitle: (count: number) => string;
  reasons: {
    "not-found": string;
    gone: string;
    "unknown-field": (field: string) => string;
    "name-not-found": string;
    "unsupported-game": string;
  };
  putAllBack: string;
  limits: string;
}

const centimetres = (units: number) => Math.round(Math.abs(units) * 1.428);

const length = (units: number, lang: Lang) => {
  const cm = centimetres(units);
  if (cm >= 100) {
    const metres = (cm / 100).toFixed(cm >= 1000 ? 0 : 1);
    return `${lang === "de" ? metres.replace(".", ",") : metres} m`;
  }
  return `${cm} cm`;
};

const en: MovesTexts = {
  move: "Move a little",
  moveTogether: (count) => `Move the ${count} parts together`,
  moveHint:
    "Instead of hiding it, put it a little to one side, so both mods' buildings can stay. The game's file stays as it is; a small plugin carries the move.",
  close: "Done",
  locked: "The game relies on this staying where it is, so it cannot be moved.",
  someLocked: (count) =>
    count === 1
      ? "1 of the parts cannot be moved: the game relies on it staying where it is. Move the parts on their own instead."
      : `${count} of the parts cannot be moved: the game relies on them staying where they are. Move the parts on their own instead.`,
  careful: "The game may rely on where this stands. Move it only a little:",
  step: "Step",
  turnStep: "Turn by",
  west: "West",
  east: "East",
  north: "North",
  south: "South",
  up: "Up",
  down: "Down",
  turnLeft: "Turn anticlockwise",
  turnRight: "Turn clockwise",
  moved: (east, north, up, degrees) => {
    const parts = [
      east === 0 ? null : `${length(east, "en")} ${east > 0 ? "east" : "west"}`,
      north === 0 ? null : `${length(north, "en")} ${north > 0 ? "north" : "south"}`,
      up === 0 ? null : `${length(up, "en")} ${up > 0 ? "up" : "down"}`,
      degrees % 360 === 0
        ? null
        : `turned ${degrees > 180 ? 360 - degrees : degrees}° ${degrees > 180 ? "anticlockwise" : "clockwise"}`,
    ].filter((part) => part !== null);
    return `Moved ${parts.join(", ")}.`;
  },
  notMoved: "Where the mod put it.",
  noLongerTouches: (name) => `No longer touches ${name}.`,
  stillOverlaps: (name, cm) => `Still overlaps ${name} by about ${cm} cm.`,
  nowOverlaps: (name, cm) => `Now overlaps ${name} by about ${cm} cm.`,
  touchesNothing: "Touches nothing of another mod here.",
  boxesNote:
    "Measured by the boxes around the models, so a curved or open model may already be free a little earlier.",
  walked: (pieces) =>
    `Characters walk on this (${pieces === 1 ? "1 piece of path" : `${pieces} pieces of path`}). Their paths stay where they are: once it is moved, they walk beside it on thin air or bump into it. Move it only a little, or up or down not at all.`,
  back: "Back to where it was",
  intoPlugin:
    "Moves go into the plugin Mod Mallet - Moves. Write and install it on the Patch page.",
  spotMoved: (count) =>
    count === 1
      ? "1 part is moved a little here instead of hidden."
      : `${count} parts are moved a little here instead of hidden.`,
  spotHint: "To keep both, pick a part in the view and move it a little to one side.",
  title: "Moved objects",
  summary: (count) => (count === 1 ? "1 object moved a little" : `${count} objects moved a little`),
  create: "Write the moves",
  creating: "Writing…",
  install: "Install the moves in Vortex",
  installHint:
    "Vortex installs the plugin Mod Mallet - Moves like a downloaded mod. Remove it there to put every object back.",
  written: (moved, masters) =>
    `Written: it moves ${moved === 1 ? "1 object" : `${moved} objects`} and loads after ${masters}.`,
  writtenLight: "It is flagged as light, so it takes no place in the load order.",
  outdated: "The moves changed since they were written. Write them again before installing.",
  nothingWritten: "None of the objects could be moved, so nothing was written.",
  refusedTitle: (count) =>
    count === 1 ? "1 object cannot be moved:" : `${count} objects cannot be moved:`,
  reasons: {
    "not-found": "it is no longer where it was. Scan again and move it anew.",
    gone: "the game no longer shows it.",
    "unknown-field": (field) =>
      `it carries data Mod Mallet does not know how to copy safely (${field}).`,
    "name-not-found": "the name of its place could not be read from the game's string tables.",
    "unsupported-game":
      "Mod Mallet writes plugins only for Skyrim Special Edition and Enderal so far.",
  },
  putAllBack: "Put all back",
  limits:
    "What moving cannot fix: the paths characters walk on, the distant view and loose things the game lets fall into place stay as they were.",
};

const de: MovesTexts = {
  move: "Ein Stück verschieben",
  moveTogether: (count) => `Die ${count} Teile zusammen verschieben`,
  moveHint:
    "Statt es auszublenden, rück es ein Stück zur Seite, damit die Bauten beider Mods bleiben können. Die Datei des Spiels bleibt, wie sie ist; ein kleines Plugin trägt die Verschiebung.",
  close: "Fertig",
  locked:
    "Das Spiel verlässt sich darauf, dass es dort steht, darum lässt es sich nicht verschieben.",
  someLocked: (count) =>
    count === 1
      ? "1 der Teile lässt sich nicht verschieben: Das Spiel verlässt sich darauf, dass es dort steht. Verschieb die Teile stattdessen einzeln."
      : `${count} der Teile lassen sich nicht verschieben: Das Spiel verlässt sich darauf, dass sie dort stehen. Verschieb die Teile stattdessen einzeln.`,
  careful: "Das Spiel verlässt sich vielleicht darauf, wo es steht. Verschieb es nur ein wenig:",
  step: "Schritt",
  turnStep: "Drehen um",
  west: "Westen",
  east: "Osten",
  north: "Norden",
  south: "Süden",
  up: "Hoch",
  down: "Runter",
  turnLeft: "Gegen den Uhrzeigersinn drehen",
  turnRight: "Im Uhrzeigersinn drehen",
  moved: (east, north, up, degrees) => {
    const parts = [
      east === 0 ? null : `${length(east, "de")} nach ${east > 0 ? "Osten" : "Westen"}`,
      north === 0 ? null : `${length(north, "de")} nach ${north > 0 ? "Norden" : "Süden"}`,
      up === 0 ? null : `${length(up, "de")} ${up > 0 ? "hoch" : "runter"}`,
      degrees % 360 === 0
        ? null
        : `${degrees > 180 ? 360 - degrees : degrees}° ${degrees > 180 ? "gegen den Uhrzeigersinn" : "im Uhrzeigersinn"} gedreht`,
    ].filter((part) => part !== null);
    return `Verschoben: ${parts.join(", ")}.`;
  },
  notMoved: "Wo die Mod es hingestellt hat.",
  noLongerTouches: (name) => `Berührt ${name} nicht mehr.`,
  stillOverlaps: (name, cm) => `Ragt noch etwa ${cm} cm in ${name}.`,
  nowOverlaps: (name, cm) => `Ragt jetzt etwa ${cm} cm in ${name}.`,
  touchesNothing: "Berührt hier nichts einer anderen Mod.",
  boxesNote:
    "Gemessen an den Kästen um die Modelle; ein gebogenes oder offenes Modell ist vielleicht schon etwas früher frei.",
  walked: (pieces) =>
    `Hier laufen Figuren (${pieces === 1 ? "1 Wegstück" : `${pieces} Wegstücke`}). Ihre Wege bleiben, wo sie sind: Ist es verschoben, laufen sie daneben durch die Luft oder stoßen dagegen. Verschieb es nur ein wenig, hoch oder runter am besten gar nicht.`,
  back: "Zurück an seinen Platz",
  intoPlugin:
    "Verschiebungen kommen ins Plugin Mod Mallet - Moves. Schreib und installier es auf der Seite Patch.",
  spotMoved: (count) =>
    count === 1
      ? "1 Teil ist hier ein Stück verschoben statt ausgeblendet."
      : `${count} Teile sind hier ein Stück verschoben statt ausgeblendet.`,
  spotHint: "Um beide zu behalten, wähl ein Teil in der Ansicht und rück es ein Stück zur Seite.",
  title: "Verschobene Objekte",
  summary: (count) =>
    count === 1 ? "1 Objekt ein Stück verschoben" : `${count} Objekte ein Stück verschoben`,
  create: "Verschiebungen schreiben",
  creating: "Wird geschrieben …",
  install: "Verschiebungen in Vortex installieren",
  installHint:
    "Vortex installiert das Plugin Mod Mallet - Moves wie eine heruntergeladene Mod. Entfernst du es dort, steht alles wieder an seinem Platz.",
  written: (moved, masters) =>
    `Geschrieben: Es verschiebt ${moved === 1 ? "1 Objekt" : `${moved} Objekte`} und lädt nach ${masters}.`,
  writtenLight: "Es ist als leicht markiert und nimmt keinen Platz in der Ladereihenfolge.",
  outdated:
    "Die Verschiebungen haben sich geändert, seit sie geschrieben wurden. Schreib sie neu, bevor du installierst.",
  nothingWritten: "Keines der Objekte ließ sich verschieben, darum wurde nichts geschrieben.",
  refusedTitle: (count) =>
    count === 1
      ? "1 Objekt lässt sich nicht verschieben:"
      : `${count} Objekte lassen sich nicht verschieben:`,
  reasons: {
    "not-found": "Es steht nicht mehr, wo es stand. Prüf neu und verschieb es noch einmal.",
    gone: "Das Spiel zeigt es nicht mehr.",
    "unknown-field": (field) =>
      `Es trägt Daten, die Mod Mallet nicht sicher kopieren kann (${field}).`,
    "name-not-found":
      "Der Name seines Ortes ließ sich nicht aus den Texttabellen des Spiels lesen.",
    "unsupported-game":
      "Mod Mallet schreibt Plugins bisher nur für Skyrim Special Edition und Enderal.",
  },
  putAllBack: "Alle zurückstellen",
  limits:
    "Was Verschieben nicht richtet: Die Wege der Figuren, die Fernsicht und lose Dinge, die das Spiel an ihren Platz fallen lässt, bleiben, wie sie waren.",
};

export const movesTexts: Record<Lang, MovesTexts> = { en, de };

/** The texts of moves in the window's language. */
export function useMovesTexts(): MovesTexts {
  return movesTexts[useSession().lang];
}
