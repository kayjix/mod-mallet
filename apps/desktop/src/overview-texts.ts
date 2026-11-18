// The texts of the overview pages: the health on the home page, the task
// page and the mods page. Kept apart from `i18n.ts` so that these pages can
// change without touching the texts of the rest of the window.

import { useSession } from "./Actions";
import type { Step } from "./arrange";
import type { Impact, Target, Verdict } from "./health";
import type { Lang } from "./i18n";

/** How the mods page sorts. */
export type ModSort = "problems" | "collisions" | "name";

/** Which mods the mods page shows. */
export type ModFilter = "all" | "problems" | "disabled";

export interface OverviewTexts {
  healthTitle: string;
  verdict: Record<Verdict, string>;
  verdictNote: Record<Verdict, string>;
  scoreOf: string;
  reasonsTitle: string;
  impact: Record<Impact | "collisions", string>;
  reasonOpenSpots: (count: number) => string;
  reasonWaitingSpots: (count: number) => string;
  reasonFindings: (title: string, count: number) => string;
  go: Record<Target, string>;
  points: (points: number) => string;
  nothingLowers: string;
  incomplete: string;
  setAside: (count: number) => string;
  adviceNotCounted: (count: number) => string;
  howCounted: string;
  howCountedText: string;
  noScanYet: string;
  statTasks: string;
  statSpots: string;
  statMods: string;

  tasksHeadline: (count: number) => string;
  tasksHeadlineNone: string;
  step: Record<Step, { title: string; hint: string }>;
  optional: string;
  crashTag: string;
  collisionsTitle: string;
  collisionsSummary: (open: number, total: number, pairs: number) => string;
  collisionsAllDecided: (total: number) => string;
  collisionsNone: string;
  collisionsWaiting: (count: number) => string;
  collisionsOpen: string;
  collisionsToPatch: string;
  adviceEmpty: string;
  showFolded: (count: number) => string;

  modsSearch: string;
  modsSort: string;
  modsSortBy: Record<ModSort, string>;
  modsFilter: Record<ModFilter, string>;
  modsShown: (shown: number, total: number) => string;
  modsNone: string;
  colMod: string;
  colVersion: string;
  colState: string;
  colProblems: string;
  enabled: string;
  disabled: string;
  newer: (version: string) => string;
  page: string;
  problemsTitle: (count: number) => string;
  findingsCount: (count: number) => string;
  spotsCount: (count: number) => string;
  noProblems: string;
  sectionFindings: string;
  sectionCollisions: string;
  sectionOverridden: string;
  sectionAdvice: string;
  collisionSpots: (open: number, total: number) => string;
  decideInWorklist: string;
  wins: (topic: string, winner: string) => string;
  undecidedTopic: (topic: string) => string;
  nothingAbout: string;
}

const en: OverviewTexts = {
  healthTitle: "How your setup is doing",
  verdict: {
    good: "Playable",
    suffers: "Playable, but the quality suffers",
    crashes: "Crashes likely",
  },
  verdictNote: {
    good: "Nothing open breaks the game. What is left is optional.",
    suffers: "The game starts, but some things look or behave wrong until they are dealt with.",
    crashes:
      "Something still open is known to crash the game or keep it from starting. Deal with that first.",
  },
  scoreOf: "of 100",
  reasonsTitle: "What lowers it most",
  impact: {
    crash: "can crash",
    broken: "wrong in the game",
    quality: "quality",
    note: "note",
    advice: "optional",
    scan: "not read",
    collisions: "collisions",
  },
  reasonOpenSpots: (count) =>
    count === 1 ? "1 collision spot not decided yet" : `${count} collision spots not decided yet`,
  reasonWaitingSpots: (count) =>
    count === 1
      ? "1 decided spot is not in the patch yet"
      : `${count} decided spots are not in the patch yet`,
  reasonFindings: (title, count) => (count === 1 ? title : `${title} (${count})`),
  go: {
    tasks: "To the tasks",
    collisions: "To the collisions",
    patch: "To the patch",
    mods: "To the mods",
  },
  points: (points) => `−${points}`,
  nothingLowers: "Nothing open lowers the score.",
  incomplete: "Mod Mallet could not read everything, so the score may be too kind.",
  setAside: (count) =>
    count === 1
      ? "1 finding you set aside does not count."
      : `${count} findings you set aside do not count.`,
  adviceNotCounted: (count) =>
    count === 1
      ? "1 recommendation is optional and does not count."
      : `${count} recommendations are optional and do not count.`,
  howCounted: "How the score is counted",
  howCountedText:
    "It starts at 100. Each open finding takes points off by what it does to the game: whatever can crash it the most, then what looks or behaves wrong, then what only costs quality. Findings Mod Mallet is less sure about count less, and many of one sort count less than one each. Collision spots that are not decided take off up to 30 points; a decided spot no longer counts once the patch is built. Findings you set aside and recommendations do not count.",
  noScanYet: "Scan once, and this shows how your setup is doing.",
  statTasks: "Things to do",
  statSpots: "Collision spots to decide",
  statMods: "Mods enabled",

  tasksHeadline: (count) => (count === 1 ? "1 thing to do" : `${count} things to do`),
  tasksHeadlineNone: "Nothing to do apart from the collisions",
  step: {
    "one-click": {
      title: "Done with one click",
      hint: "Mod Mallet hands these to your mod manager. You confirm there.",
    },
    install: {
      title: "Install what is missing",
      hint: "A mod needs a file or another mod that is not there.",
    },
    update: {
      title: "Update, rebuild or clean",
      hint: "A mod or a generated file no longer fits the game or the setup.",
    },
    choose: {
      title: "Decide which mod wins",
      hint: "Two mods change the same thing. One of them gives way, or a patch joins them.",
    },
    scan: {
      title: "Mod Mallet could not read everything",
      hint: "Until this is solved, some checks did not run.",
    },
    check: {
      title: "Look at it",
      hint: "Mod Mallet has no fixed recipe for these.",
    },
    advice: {
      title: "Recommendations",
      hint: "Optional. Suggestions that may make the setup better; nothing breaks if you leave them.",
    },
    notes: {
      title: "Good to know",
      hint: "Nothing to do. This is how the setup works.",
    },
    ignored: {
      title: "Set aside",
      hint: "You chose to leave these as they are. They do not count.",
    },
  },
  optional: "optional",
  crashTag: "can crash",
  collisionsTitle: "Collisions",
  collisionsSummary: (open, total, pairs) =>
    `${open} of ${total} spots still to decide, between ${pairs} ${pairs === 1 ? "pair" : "pairs"} of mods.`,
  collisionsAllDecided: (total) =>
    total === 1 ? "The one spot is decided." : `All ${total} spots are decided.`,
  collisionsNone: "No objects of two mods collide.",
  collisionsWaiting: (count) =>
    count === 1
      ? "1 decided spot is not in the patch yet."
      : `${count} decided spots are not in the patch yet.`,
  collisionsOpen: "Open the worklist",
  collisionsToPatch: "To the patch",
  adviceEmpty: "No recommendations at the moment.",
  showFolded: (count) => (count === 1 ? "Show 1" : `Show ${count}`),

  modsSearch: "Search mods",
  modsSort: "Sort",
  modsSortBy: {
    problems: "Most problems first",
    collisions: "Most collisions first",
    name: "By name",
  },
  modsFilter: { all: "All", problems: "With problems", disabled: "Disabled" },
  modsShown: (shown, total) => (shown === total ? `${total} mods` : `${shown} of ${total} mods`),
  modsNone: "No mod matches.",
  colMod: "Mod",
  colVersion: "Version",
  colState: "State",
  colProblems: "Problems",
  enabled: "Enabled",
  disabled: "Disabled",
  newer: (version) => `${version} available`,
  page: "Page",
  problemsTitle: (count) => (count === 1 ? "1 open problem" : `${count} open problems`),
  findingsCount: (count) => (count === 1 ? "1 finding" : `${count} findings`),
  spotsCount: (count) => (count === 1 ? "1 spot" : `${count} spots`),
  noProblems: "No open problems",
  sectionFindings: "What was found",
  sectionCollisions: "Collides with",
  sectionOverridden: "Overridden content",
  sectionAdvice: "Recommendations",
  collisionSpots: (open, total) =>
    open === 0 ? `all ${total} decided` : `${open} of ${total} spots open`,
  decideInWorklist: "Decide in the worklist",
  wins: (topic, winner) => `${topic}: ${winner} wins`,
  undecidedTopic: (topic) => `${topic}: not decided`,
  nothingAbout: "Mod Mallet found nothing about this mod.",
};

const de: OverviewTexts = {
  healthTitle: "So steht es um dein Setup",
  verdict: {
    good: "Spielbar",
    suffers: "Spielbar, aber die Qualität leidet",
    crashes: "Abstürze wahrscheinlich",
  },
  verdictNote: {
    good: "Nichts Offenes macht das Spiel kaputt. Was übrig ist, ist freiwillig.",
    suffers:
      "Das Spiel startet, aber manches sieht falsch aus oder verhält sich falsch, bis es erledigt ist.",
    crashes:
      "Etwas noch Offenes bringt das Spiel bekanntermaßen zum Absturz oder lässt es nicht starten. Das zuerst erledigen.",
  },
  scoreOf: "von 100",
  reasonsTitle: "Was am meisten abzieht",
  impact: {
    crash: "Absturzgefahr",
    broken: "Fehler im Spiel",
    quality: "Qualität",
    note: "Hinweis",
    advice: "freiwillig",
    scan: "nicht gelesen",
    collisions: "Kollisionen",
  },
  reasonOpenSpots: (count) =>
    count === 1
      ? "1 Kollisionsstelle noch nicht entschieden"
      : `${count} Kollisionsstellen noch nicht entschieden`,
  reasonWaitingSpots: (count) =>
    count === 1
      ? "1 entschiedene Stelle fehlt noch im Patch"
      : `${count} entschiedene Stellen fehlen noch im Patch`,
  reasonFindings: (title, count) => (count === 1 ? title : `${title} (${count})`),
  go: {
    tasks: "Zu den Aufgaben",
    collisions: "Zu den Kollisionen",
    patch: "Zum Patch",
    mods: "Zu den Mods",
  },
  points: (points) => `−${points}`,
  nothingLowers: "Nichts Offenes zieht etwas ab.",
  incomplete: "Mod Mallet konnte nicht alles lesen, der Wert ist deshalb vielleicht zu freundlich.",
  setAside: (count) =>
    count === 1
      ? "1 beiseitegelegter Befund zählt nicht mit."
      : `${count} beiseitegelegte Befunde zählen nicht mit.`,
  adviceNotCounted: (count) =>
    count === 1
      ? "1 Empfehlung ist freiwillig und zählt nicht mit."
      : `${count} Empfehlungen sind freiwillig und zählen nicht mit.`,
  howCounted: "Wie der Wert entsteht",
  howCountedText:
    "Er beginnt bei 100. Jeder offene Befund zieht Punkte ab, je nachdem, was er im Spiel anrichtet: am meisten, was es abstürzen lässt, dann, was falsch aussieht oder sich falsch verhält, dann, was nur Qualität kostet. Befunde, bei denen sich Mod Mallet weniger sicher ist, zählen weniger, und viele einer Sorte zählen weniger als je einer. Nicht entschiedene Kollisionsstellen ziehen bis zu 30 Punkte ab; eine entschiedene Stelle zählt nicht mehr, sobald der Patch gebaut ist. Beiseitegelegte Befunde und Empfehlungen zählen nicht.",
  noScanYet: "Einmal prüfen, dann steht hier, wie es um dein Setup steht.",
  statTasks: "Aufgaben",
  statSpots: "Kollisionsstellen offen",
  statMods: "Mods aktiv",

  tasksHeadline: (count) => (count === 1 ? "1 Sache zu tun" : `${count} Sachen zu tun`),
  tasksHeadlineNone: "Außer den Kollisionen nichts zu tun",
  step: {
    "one-click": {
      title: "Mit einem Klick erledigt",
      hint: "Mod Mallet gibt das an deine Mod-Verwaltung weiter. Bestätigen musst du dort.",
    },
    install: {
      title: "Fehlendes nachinstallieren",
      hint: "Eine Mod braucht eine Datei oder eine andere Mod, die nicht da ist.",
    },
    update: {
      title: "Aktualisieren, neu erzeugen oder bereinigen",
      hint: "Eine Mod oder eine erzeugte Datei passt nicht mehr zum Spiel oder zum Setup.",
    },
    choose: {
      title: "Entscheiden, welche Mod gewinnt",
      hint: "Zwei Mods ändern dasselbe. Eine von beiden weicht, oder ein Patch verbindet sie.",
    },
    scan: {
      title: "Mod Mallet konnte nicht alles lesen",
      hint: "Solange das so ist, liefen manche Prüfungen nicht.",
    },
    check: {
      title: "Ansehen",
      hint: "Dafür hat Mod Mallet kein festes Rezept.",
    },
    advice: {
      title: "Empfehlungen",
      hint: "Freiwillig. Vorschläge, die das Setup verbessern können; nichts geht kaputt, wenn du sie lässt.",
    },
    notes: {
      title: "Gut zu wissen",
      hint: "Nichts zu tun. So funktioniert das Setup eben.",
    },
    ignored: {
      title: "Beiseitegelegt",
      hint: "Die hast du gelassen, wie sie sind. Sie zählen nicht mit.",
    },
  },
  optional: "freiwillig",
  crashTag: "Absturzgefahr",
  collisionsTitle: "Kollisionen",
  collisionsSummary: (open, total, pairs) =>
    `${open} von ${total} Stellen noch zu entscheiden, zwischen ${pairs} ${pairs === 1 ? "Mod-Paar" : "Mod-Paaren"}.`,
  collisionsAllDecided: (total) =>
    total === 1 ? "Die eine Stelle ist entschieden." : `Alle ${total} Stellen sind entschieden.`,
  collisionsNone: "Keine Objekte zweier Mods kollidieren.",
  collisionsWaiting: (count) =>
    count === 1
      ? "1 entschiedene Stelle fehlt noch im Patch."
      : `${count} entschiedene Stellen fehlen noch im Patch.`,
  collisionsOpen: "Zur Kollisionsliste",
  collisionsToPatch: "Zum Patch",
  adviceEmpty: "Gerade keine Empfehlungen.",
  showFolded: (count) => (count === 1 ? "1 zeigen" : `${count} zeigen`),

  modsSearch: "Mods suchen",
  modsSort: "Sortieren",
  modsSortBy: {
    problems: "Meiste Probleme zuerst",
    collisions: "Meiste Kollisionen zuerst",
    name: "Nach Name",
  },
  modsFilter: { all: "Alle", problems: "Mit Problemen", disabled: "Abgeschaltet" },
  modsShown: (shown, total) => (shown === total ? `${total} Mods` : `${shown} von ${total} Mods`),
  modsNone: "Keine Mod passt.",
  colMod: "Mod",
  colVersion: "Version",
  colState: "Status",
  colProblems: "Probleme",
  enabled: "Aktiv",
  disabled: "Aus",
  newer: (version) => `${version} verfügbar`,
  page: "Seite",
  problemsTitle: (count) => (count === 1 ? "1 offenes Problem" : `${count} offene Probleme`),
  findingsCount: (count) => (count === 1 ? "1 Befund" : `${count} Befunde`),
  spotsCount: (count) => (count === 1 ? "1 Stelle" : `${count} Stellen`),
  noProblems: "Keine offenen Probleme",
  sectionFindings: "Was gefunden wurde",
  sectionCollisions: "Kollidiert mit",
  sectionOverridden: "Überschriebene Inhalte",
  sectionAdvice: "Empfehlungen",
  collisionSpots: (open, total) =>
    open === 0 ? `alle ${total} entschieden` : `${open} von ${total} Stellen offen`,
  decideInWorklist: "In der Kollisionsliste entscheiden",
  wins: (topic, winner) => `${topic}: ${winner} gewinnt`,
  undecidedTopic: (topic) => `${topic}: nicht entschieden`,
  nothingAbout: "Mod Mallet hat zu dieser Mod nichts gefunden.",
};

export const overviewTexts: Record<Lang, OverviewTexts> = { en, de };

/** The texts of the overview pages in the window's language. */
export function useOverviewTexts(): OverviewTexts {
  return overviewTexts[useSession().lang];
}
