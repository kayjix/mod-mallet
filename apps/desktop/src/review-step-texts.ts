// The texts of the steps of "Step by step" before the collisions (ADR
// 0083): what the app is for, what the installation step checks, the
// assets' duels in pictures and the map of the open places. Short, for
// someone who never opened a modding tool.

import { useSession } from "./Actions";
import type { Lang } from "./i18n";

export interface ReviewStepTexts {
  /** The installation step: who does what, and what it checks. */
  decides: string;
  decidesNote: string;
  carries: (manager: string) => string;
  carriesNote: string;
  shows: string;
  showsNote: string;
  installPurpose: string;
  modsOn: string;
  modsOnValue: (on: number, total: number) => string;
  newer: string;
  plugins: string;
  toFix: string;
  groupMods: string;
  groupPlugins: string;
  pluginsNote: (manager: string) => string;
  nothingToFix: string;
  /** The assets step. */
  assetsPurpose: string;
  assetsDecided: (decided: number, total: number) => string;
  openAssets: string;
  biggest: string;
  inGame: string;
  compare: string;
  noPicture: string;
  loose: string;
  allAssetsDecided: string;
  suggested: (name: string) => string;
  decided: (name: string) => string;
  /** The map of the open places. */
  mapPurpose: string;
  mapSummary: (open: number, worlds: number) => string;
  legendOpen: string;
  legendDecided: string;
  interiors: string;
  noPlaces: string;
  placeTitle: (pair: string, kind: string, decided: boolean) => string;
  toMap: string;
  /** The report at the end. */
  tileCollisions: string;
  tileAssets: string;
  tileInstall: string;
  tileHidden: string;
  decidedOf: (decided: number, total: number) => string;
  patchTitle: string;
  patchPurpose: (manager: string) => string;
  pairLine: (places: number, decided: number) => string;
  keptAt: (mod: string, count: number) => string;
  bothAt: (count: number) => string;
  bothKept: string;
  hiddenParts: (mod: string, count: number) => string;
  nothingDecided: string;
  openTitle: string;
  openPlaces: (count: number) => string;
  openAssetDuels: (count: number) => string;
  openInstall: (count: number) => string;
  pinned: (count: number) => string;
  nothingOpen: string;
  goThere: string;
  tryTitle: string;
  tryPatch: (manager: string) => string;
  tryStart: string;
  tryPlace: (pair: string, mod: string) => string;
  tryBoth: (pair: string) => string;
  tryConsole: string;
  copy: string;
  copied: string;
  fullReport: string;
}

const en: ReviewStepTexts = {
  decides: "Mod Mallet decides",
  decidesNote: "It finds what clashes and asks you. On its own it changes nothing.",
  carries: (manager) => `${manager} carries it out`,
  carriesNote: "It installs, switches off, and puts in the patch with your decisions.",
  shows: "The game shows it",
  showsNote: "Start it and look at the places you decided.",
  installPurpose:
    "This step checks that every mod is there and complete and that the plugins fit together. A missing file or master stops the game at the start.",
  modsOn: "mods switched on",
  modsOnValue: (on, total) => `${on} of ${total}`,
  newer: "with a newer version",
  plugins: "plugins loaded",
  toFix: "to fix here",
  groupMods: "Mods and installers",
  groupPlugins: "Plugins and their order",
  pluginsNote: (manager) => `${manager} sets the order. Mod Mallet only says where it matters.`,
  nothingToFix: "Nothing to fix here.",
  assetsPurpose:
    "Two mods bring the same file, and the game shows only one. Pick whose look you want.",
  assetsDecided: (decided, total) => `${decided} of ${total} decided`,
  openAssets: "Open Asset collisions",
  biggest: "The biggest ones",
  inGame: "in the game now",
  compare: "Compare",
  noPicture: "no picture",
  loose: "loose file",
  allAssetsDecided: "Every asset collision is decided.",
  suggested: (name) => `suggested: ${name}`,
  decided: (name) => `decided: ${name}`,
  mapPurpose: "Where the collisions are. Click a point to go there.",
  mapSummary: (open, worlds) =>
    `${open} open ${open === 1 ? "place" : "places"} in ${worlds} ${worlds === 1 ? "world" : "worlds"}`,
  legendOpen: "open",
  legendDecided: "decided",
  interiors: "Inside",
  noPlaces: "No collisions with a place to show.",
  placeTitle: (pair, kind, decided) => `${pair} · ${kind}${decided ? " · decided" : ""}`,
  toMap: "Open the big map",
  tileCollisions: "places decided",
  tileAssets: "asset collisions decided",
  tileInstall: "to fix in the installation",
  tileHidden: "objects the patch hides",
  decidedOf: (decided, total) => `${decided} of ${total}`,
  patchTitle: "What the patch does",
  patchPurpose: (manager) =>
    `The patch hides what you decided against. Nothing is deleted: switch the patch off in ${manager} and everything is back.`,
  pairLine: (places, decided) =>
    `${decided} of ${places} ${places === 1 ? "place" : "places"} decided`,
  keptAt: (mod, count) => `${mod} kept at ${count}`,
  bothAt: (count) => `both kept at ${count}`,
  bothKept: "✓ both stay",
  hiddenParts: (mod, count) => `${count} ${count === 1 ? "part" : "parts"} of ${mod} hidden`,
  nothingDecided: "Nothing decided yet. The collisions step is where you decide.",
  openTitle: "Still open",
  openPlaces: (count) => `${count} ${count === 1 ? "place" : "places"} with collisions`,
  openAssetDuels: (count) => `${count} asset ${count === 1 ? "collision" : "collisions"}`,
  openInstall: (count) => `${count} ${count === 1 ? "thing" : "things"} to fix in the installation`,
  pinned: (count) => `${count} pinned to solve later`,
  nothingOpen: "Nothing is open.",
  goThere: "Go there",
  tryTitle: "Try it in the game",
  tryPatch: (manager) => `Switch the patch on in ${manager} and let it deploy.`,
  tryStart: "Start the game and load a save: it should reach the world without a crash.",
  tryPlace: (pair, mod) => `${pair}: only ${mod}'s version should stand there.`,
  tryBoth: (pair) => `${pair}: both stay; look whether that is fine.`,
  tryConsole: "Open the console with ^ or ~, type the command and press Enter to go there.",
  copy: "Copy",
  copied: "Copied",
  fullReport: "The full report, with the name of every part",
};

const de: ReviewStepTexts = {
  decides: "Mod Mallet entscheidet",
  decidesNote: "Es findet, was sich beißt, und fragt dich. Von sich aus ändert es nichts.",
  carries: (manager) => `${manager} führt aus`,
  carriesNote: "Es installiert, schaltet ab und spielt den Patch mit deinen Entscheidungen ein.",
  shows: "Das Spiel zeigt es",
  showsNote: "Starte es und sieh dir die entschiedenen Stellen an.",
  installPurpose:
    "Dieser Schritt prüft, ob jeder Mod da und vollständig ist und ob die Plugins zusammenpassen. Fehlt eine Datei oder ein Master, startet das Spiel nicht.",
  modsOn: "Mods eingeschaltet",
  modsOnValue: (on, total) => `${on} von ${total}`,
  newer: "mit neuerer Version",
  plugins: "Plugins geladen",
  toFix: "hier zu beheben",
  groupMods: "Mods und Installer",
  groupPlugins: "Plugins und ihre Reihenfolge",
  pluginsNote: (manager) =>
    `Die Reihenfolge stellt ${manager} ein. Mod Mallet sagt nur, wo es darauf ankommt.`,
  nothingToFix: "Hier ist nichts zu beheben.",
  assetsPurpose:
    "Zwei Mods bringen dieselbe Datei mit, und das Spiel zeigt nur eine. Wähle, wessen Aussehen du willst.",
  assetsDecided: (decided, total) => `${decided} von ${total} entschieden`,
  openAssets: "Asset-Kollisionen öffnen",
  biggest: "Die größten",
  inGame: "jetzt im Spiel",
  compare: "Vergleichen",
  noPicture: "kein Bild",
  loose: "lose Datei",
  allAssetsDecided: "Jede Asset-Kollision ist entschieden.",
  suggested: (name) => `Vorschlag: ${name}`,
  decided: (name) => `entschieden: ${name}`,
  mapPurpose: "Wo die Kollisionen liegen. Ein Klick auf einen Punkt führt hin.",
  mapSummary: (open, worlds) =>
    `${open} ${open === 1 ? "offene Stelle" : "offene Stellen"} in ${worlds} ${worlds === 1 ? "Welt" : "Welten"}`,
  legendOpen: "offen",
  legendDecided: "entschieden",
  interiors: "Innen",
  noPlaces: "Keine Kollisionen mit einem Ort zum Zeigen.",
  placeTitle: (pair, kind, decided) => `${pair} · ${kind}${decided ? " · entschieden" : ""}`,
  toMap: "Die große Karte öffnen",
  tileCollisions: "Bereiche entschieden",
  tileAssets: "Asset-Kollisionen entschieden",
  tileInstall: "in der Installation zu beheben",
  tileHidden: "Objekte blendet der Patch aus",
  decidedOf: (decided, total) => `${decided} von ${total}`,
  patchTitle: "Was der Patch tut",
  patchPurpose: (manager) =>
    `Der Patch blendet aus, wogegen du dich entschieden hast. Gelöscht wird nichts: schalte den Patch in ${manager} ab, und alles ist wieder da.`,
  pairLine: (places, decided) =>
    `${decided} von ${places} ${places === 1 ? "Bereich" : "Bereichen"} entschieden`,
  keptAt: (mod, count) => `${mod} behalten an ${count}`,
  bothAt: (count) => `beide behalten an ${count}`,
  bothKept: "✓ beide bleiben",
  hiddenParts: (mod, count) => `${count} ${count === 1 ? "Teil" : "Teile"} von ${mod} ausgeblendet`,
  nothingDecided: "Noch nichts entschieden. Entschieden wird im Schritt Kollisionen.",
  openTitle: "Noch offen",
  openPlaces: (count) => `${count} ${count === 1 ? "Bereich" : "Bereiche"} mit Kollisionen`,
  openAssetDuels: (count) => `${count} Asset-${count === 1 ? "Kollision" : "Kollisionen"}`,
  openInstall: (count) =>
    `${count} ${count === 1 ? "Sache" : "Sachen"} in der Installation zu beheben`,
  pinned: (count) => `${count} gemerkt, später selbst lösen`,
  nothingOpen: "Nichts ist offen.",
  goThere: "Hingehen",
  tryTitle: "Im Spiel ausprobieren",
  tryPatch: (manager) => `Schalte den Patch in ${manager} ein und lass ihn einspielen.`,
  tryStart:
    "Starte das Spiel und lade einen Spielstand: es sollte ohne Absturz in die Welt kommen.",
  tryPlace: (pair, mod) => `${pair}: dort sollte nur noch die Fassung von ${mod} stehen.`,
  tryBoth: (pair) => `${pair}: beide bleiben; sieh nach, ob das so passt.`,
  tryConsole: "Öffne die Konsole mit ^ oder ~, tippe den Befehl und drücke Enter, um hinzukommen.",
  copy: "Kopieren",
  copied: "Kopiert",
  fullReport: "Der ausführliche Bericht, mit dem Namen jedes Teils",
};

export const reviewStepTexts: Record<Lang, ReviewStepTexts> = { en, de };

/** The steps' texts in the window's language. */
export function useReviewStepTexts(): ReviewStepTexts {
  return reviewStepTexts[useSession().lang];
}
