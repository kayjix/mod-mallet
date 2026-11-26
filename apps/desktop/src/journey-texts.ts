// The texts of the window's frame (ADR 0048): the sidebar, the page of all
// games and the overview of one game. Kept apart from `i18n.ts`, like the
// other pages' texts, so that the pages can change side by side.

import { useSession } from "./Actions";
import type { Lang } from "./i18n";

export interface JourneyTexts {
  navOverview: string;
  navAssets: string;
  /** What clicking the game in the sidebar does. */
  gameCardHint: string;

  allGamesTitle: string;
  allGamesNote: string;
  switchTo: string;
  toOverview: string;
  lastScanAt: (when: string) => string;
  soon: string;
  startTasks: string;
  startSpots: string;
  startAssets: string;
  startPatch: string;
  /** What the name in the sidebar does. */
  brandHint: string;
  inUse: string;
  notScanned: string;
  /** Under the score of the game in use: it is the one of now. */
  scoreNow: string;
  /** Under the last score of another game: from which scan it is. */
  scoreAt: (when: string) => string;

  scanFirst: string;
  tasksOpen: (count: number) => string;
  tasksNone: string;
  tasksCrash: (count: number) => string;
  spotsOpen: (count: number) => string;
  spotsNone: string;
  spotsWaiting: (count: number) => string;
  spotsAllDecided: string;
  patchHidden: (count: number) => string;
  patchNone: string;
  patchNotCurrent: string;
  patchCurrent: string;
  assetsOpen: (count: number) => string;
  assetsAllDecided: string;
  assetsNone: string;
  assetsRanked: (count: number) => string;
  modsEnabled: (enabled: number, total: number) => string;
  modsNewer: (count: number) => string;
  mapWorlds: (count: number) => string;
  mapMore: (count: number) => string;
  toolsAttention: (count: number) => string;
  toolsReady: string;
  toolsUnknown: string;
}

const en: JourneyTexts = {
  navOverview: "Overview",
  navAssets: "Assets",
  gameCardHint: "The overview of this game",

  allGamesTitle: "Games",
  allGamesNote: "Pick the game to work on. Each game has its own scan, tasks and patch.",
  switchTo: "Switch to this game",
  toOverview: "To the overview",
  lastScanAt: (when) => `scanned ${when}`,
  soon: "soon",
  startTasks: "Things to do",
  startSpots: "Collision spots to decide",
  startAssets: "Pairs of mods to compare",
  startPatch: "Objects hidden by the patch",
  brandHint: "Start: all games",
  inUse: "In use",
  notScanned: "Not scanned yet",
  scoreNow: "as it stands now",
  scoreAt: (when) => `scan of ${when}`,

  scanFirst: "Scan this game to see how it stands and what there is to do.",
  tasksOpen: (count) => (count === 1 ? "1 thing to do" : `${count} things to do`),
  tasksNone: "Nothing to do",
  tasksCrash: (count) =>
    count === 1 ? "1 of them can crash the game" : `${count} of them can crash the game`,
  spotsOpen: (count) => (count === 1 ? "1 spot to decide" : `${count} spots to decide`),
  spotsNone: "No collisions",
  spotsWaiting: (count) => `${count} decided, not in the patch yet`,
  spotsAllDecided: "All decided",
  patchHidden: (count) => (count === 1 ? "hides 1 object" : `hides ${count} objects`),
  patchNone: "No patch in the game",
  patchNotCurrent: "Not up to date with your decisions",
  patchCurrent: "Up to date",
  assetsOpen: (count) =>
    count === 1 ? "1 pair of mods to compare" : `${count} pairs of mods to compare`,
  assetsAllDecided: "All compared",
  assetsNone: "No two mods change the same textures, meshes or faces",
  assetsRanked: (count) => `${count} mods in your ranking`,
  modsEnabled: (enabled, total) => `${enabled} of ${total} switched on`,
  modsNewer: (count) => (count === 1 ? "1 has a newer version" : `${count} have a newer version`),
  mapWorlds: (count) => (count === 1 ? "1 worldspace" : `${count} worldspaces`),
  mapMore: (count) => `and ${count} more`,
  toolsAttention: (count) =>
    count === 1 ? "1 tool needs attention" : `${count} tools need attention`,
  toolsReady: "Everything ready",
  toolsUnknown: "Not looked at yet",
};

const de: JourneyTexts = {
  navOverview: "Übersicht",
  navAssets: "Assets",
  gameCardHint: "Die Übersicht dieses Spiels",

  allGamesTitle: "Spiele",
  allGamesNote:
    "Wähle das Spiel, an dem du arbeiten willst. Jedes Spiel hat seinen eigenen Scan, seine Aufgaben und seinen Patch.",
  switchTo: "Zu diesem Spiel wechseln",
  toOverview: "Zur Übersicht",
  lastScanAt: (when) => `geprüft ${when}`,
  soon: "bald",
  startTasks: "Aufgaben",
  startSpots: "Kollisionsstellen offen",
  startAssets: "Mod-Paare zu vergleichen",
  startPatch: "Objekte vom Patch ausgeblendet",
  brandHint: "Start: alle Spiele",
  inUse: "In Arbeit",
  notScanned: "Noch nicht geprüft",
  scoreNow: "Stand jetzt",
  scoreAt: (when) => `Prüfung vom ${when}`,

  scanFirst: "Prüfe dieses Spiel, um zu sehen, wie es steht und was zu tun ist.",
  tasksOpen: (count) => (count === 1 ? "1 Sache zu tun" : `${count} Sachen zu tun`),
  tasksNone: "Nichts zu tun",
  tasksCrash: (count) =>
    count === 1
      ? "1 davon kann das Spiel abstürzen lassen"
      : `${count} davon können das Spiel abstürzen lassen`,
  spotsOpen: (count) =>
    count === 1 ? "1 Stelle zu entscheiden" : `${count} Stellen zu entscheiden`,
  spotsNone: "Keine Kollisionen",
  spotsWaiting: (count) => `${count} entschieden, noch nicht im Patch`,
  spotsAllDecided: "Alles entschieden",
  patchHidden: (count) => (count === 1 ? "blendet 1 Objekt aus" : `blendet ${count} Objekte aus`),
  patchNone: "Kein Patch im Spiel",
  patchNotCurrent: "Nicht auf dem Stand deiner Entscheidungen",
  patchCurrent: "Auf dem Stand",
  assetsOpen: (count) =>
    count === 1 ? "1 Mod-Paar zu vergleichen" : `${count} Mod-Paare zu vergleichen`,
  assetsAllDecided: "Alles verglichen",
  assetsNone: "Keine zwei Mods ändern dieselben Texturen, Modelle oder Gesichter",
  assetsRanked: (count) => `${count} Mods in deiner Rangfolge`,
  modsEnabled: (enabled, total) => `${enabled} von ${total} eingeschaltet`,
  modsNewer: (count) =>
    count === 1 ? "1 hat eine neuere Version" : `${count} haben eine neuere Version`,
  mapWorlds: (count) => (count === 1 ? "1 Weltbereich" : `${count} Weltbereiche`),
  mapMore: (count) => `und ${count} weitere`,
  toolsAttention: (count) =>
    count === 1
      ? "1 Werkzeug braucht Aufmerksamkeit"
      : `${count} Werkzeuge brauchen Aufmerksamkeit`,
  toolsReady: "Alles bereit",
  toolsUnknown: "Noch nicht nachgesehen",
};

export const journeyTexts: Record<Lang, JourneyTexts> = { en, de };

/** The frame's texts in the window's language. */
export function useJourneyTexts(): JourneyTexts {
  return journeyTexts[useSession().lang];
}
