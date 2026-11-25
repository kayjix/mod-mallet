// The texts of preferences between mods (ADR 0086): the offer that pops
// up when a pattern is clear, its preview, the mark on a conflict a
// preference covers, and the section of the settings.

import { useSession } from "./Actions";
import type { Lang } from "./i18n";
import type { PreferenceTopic } from "./preferences";

export interface PreferenceTexts {
  /** A topic after "for": "meshes", "faces and characters". */
  topic: Record<PreferenceTopic, string>;
  /** A preference in a few words. */
  favourite: (mod: string, topic: string) => string;
  pair: (mod: string, over: string) => string;
  pairFor: (mod: string, over: string, topic: string) => string;
  always: (mod: string) => string;
  /** How often it was kept. */
  kept: (chosen: number, of: number) => string;
  followed: (count: number) => string;
  /** The mallet in the corner, with how many favourites wait. */
  mascot: (count: number) => string;
  /** The offer. */
  offerTitle: string;
  keptSentence: (chosen: number, of: number) => string;
  overChip: (over: string) => string;
  offerWhat: (mod: string, topic: PreferenceTopic, count: number) => string;
  offerPairWhat: (mod: string, count: number) => string;
  pager: (index: number, count: number) => string;
  previous: string;
  next: string;
  showList: string;
  hideList: string;
  apply: (count: number) => string;
  more: string;
  never: string;
  minimise: string;
  close: string;
  managerNote: (mod: string) => string;
  keepHint: string;
  /** The whole of applying, for the settings. */
  applyNote: string;
  open: string;
  duelRow: (winner: string, loser: string) => string;
  spotRow: (loser: string, objects: string) => string;
  /** While and after applying. */
  checking: (done: number, total: number) => string;
  applied: (count: number) => string;
  walked: (count: number) => string;
  stopped: (message: string) => string;
  failed: (count: number) => string;
  undoAll: string;
  undone: string;
  undoNote: string;
  /** The mark on a conflict. */
  mark: string;
  markFor: (mod: string) => string;
  markFrom: string;
  markTip: (rule: string) => string;
  /** The settings. */
  tab: string;
  title: string;
  intro: string;
  learning: string;
  learningLabel: string;
  learningNote: string;
  learned: string;
  learnedNone: (min: number) => string;
  inUse: string;
  turnedOff: string;
  turnOff: string;
  useAgain: string;
  openConflicts: (count: number) => string;
  applyHere: (count: number) => string;
  own: string;
  ownNone: string;
  remove: string;
  addTitle: string;
  addMod: string;
  addTopic: string;
  addOver: string;
  everything: string;
  everyOther: string;
  add: string;
  pickMod: string;
}

const enTopicConflicts: Record<PreferenceTopic, [string, string]> = {
  meshes: ["mesh conflict", "mesh conflicts"],
  textures: ["texture conflict", "texture conflicts"],
  faces: ["conflict about faces", "conflicts about faces"],
  files: ["conflict about other files", "conflicts about other files"],
  places: ["collision of placed objects", "collisions of placed objects"],
};

const en: PreferenceTexts = {
  topic: {
    meshes: "meshes (3D models)",
    textures: "textures",
    faces: "faces and characters",
    files: "other files",
    places: "buildings and placed objects",
  },
  favourite: (mod, topic) => `${mod} for ${topic}`,
  pair: (mod, over) => `${mod} over ${over}`,
  pairFor: (mod, over, topic) => `${mod} over ${over}, for ${topic}`,
  always: (mod) => `${mod}, for everything`,
  kept: (chosen, of) => `you kept it ${chosen} of ${of} times`,
  followed: (count) => `and ${count} more times by taking a suggestion`,
  mascot: (count) =>
    `Mal the Rubber Mallet — ${count === 1 ? "1 favourite" : `${count} favourites`}`,
  offerTitle: "Mal noticed a favourite",
  keptSentence: (chosen, of) => `You kept it ${chosen} of ${of} times.`,
  overChip: (over) => `over ${over}`,
  offerWhat: (mod, topic, count) =>
    count === 1
      ? `Apply prefers ${mod} in the 1 other open ${enTopicConflicts[topic][0]}, in the patch you are editing.`
      : `Apply prefers ${mod} in the ${count} other open ${enTopicConflicts[topic][1]}, in the patch you are editing.`,
  offerPairWhat: (mod, count) =>
    count === 1
      ? `Apply prefers ${mod} in the 1 other place where the two are in conflict, in the patch you are editing.`
      : `Apply prefers ${mod} in the ${count} other places where the two are in conflict, in the patch you are editing.`,
  pager: (index, count) => `${index} of ${count}`,
  previous: "Previous favourite",
  next: "Next favourite",
  showList: "Show list",
  hideList: "Hide list",
  apply: (count) => `Apply to ${count}`,
  more: "More",
  never: "Don't suggest this again",
  minimise: "Minimise",
  close: "Close",
  managerNote: (mod) =>
    `For conflicts about files, the mod manager is asked to let ${mod}'s files win, as when you pick by hand.`,
  keepHint:
    "Parts the game needs stay, and places where characters walk on what would go are left for you. One undo takes it all back.",
  applyNote:
    "When a preference is clear, Mal the Rubber Mallet, small in the corner of the window, counts how many wait; click Mal to see them. Applying one decides its open conflicts in the patch you are editing. Parts the game needs stay, and places where characters walk on what would go are left for you. For conflicts about files, the mod manager is asked to let the kept mod's files win, as when you pick by hand. One undo takes the decisions back in Mod Mallet.",
  open: "Open",
  duelRow: (winner, loser) => `${winner} wins over ${loser}`,
  spotRow: (loser, objects) => `${objects}: the part of ${loser} goes`,
  checking: (done, total) => `Checking where characters walk… ${done} of ${total}`,
  applied: (count) =>
    count === 1
      ? "1 conflict decided from your preference."
      : `${count} conflicts decided from your preference.`,
  walked: (count) =>
    count === 1
      ? "1 left open: characters walk on parts that would go."
      : `${count} left open: characters walk on parts that would go.`,
  stopped: (message) => `Stopped, the mod manager cannot be reached: ${message}`,
  failed: (count) =>
    count === 1
      ? "The mod manager did not carry out 1 request."
      : `The mod manager did not carry out ${count} requests.`,
  undoAll: "Undo all",
  undone: "Taken back.",
  undoNote:
    "Undo takes the decisions back in Mod Mallet. What the mod manager was asked to do for files stays there, until you pick otherwise.",
  mark: "suggested: your preference",
  markFor: (mod) => `suggested: ${mod}, your preference`,
  markFrom: "from your preference",
  markTip: (rule) => `Your preference: ${rule}`,
  tab: "Preferences",
  title: "Preferences between mods",
  intro:
    "When you keep choosing the same mod, Mod Mallet notices and suggests it in the other conflicts. It learns only from your own decisions on this computer; nothing is sent anywhere.",
  learning: "Learning",
  learningLabel: "Learn preferences from my decisions",
  learningNote:
    "Decisions you made by taking a suggestion count a quarter; those made by applying a preference do not count.",
  learned: "Learned from your decisions",
  learnedNone: (min) =>
    `Nothing clear yet. A preference shows once you kept a mod at least ${min} times, and 4 out of 5 times or more.`,
  inUse: "in use",
  turnedOff: "turned off",
  turnOff: "Turn off",
  useAgain: "Use again",
  openConflicts: (count) => (count === 1 ? "1 open conflict" : `${count} open conflicts`),
  applyHere: (count) => `Apply to ${count}`,
  own: "Set by you",
  ownNone: "None yet.",
  remove: "Remove",
  addTitle: "Always prefer",
  addMod: "Mod",
  addTopic: "For",
  addOver: "Over",
  everything: "everything",
  everyOther: "every other mod",
  add: "Add",
  pickMod: "Pick a mod",
};

const deTopicConflicts: Record<PreferenceTopic, [string, string]> = {
  meshes: ["Mesh-Konflikt", "Mesh-Konflikten"],
  textures: ["Textur-Konflikt", "Textur-Konflikten"],
  faces: ["Konflikt um Gesichter", "Konflikten um Gesichter"],
  files: ["Konflikt um andere Dateien", "Konflikten um andere Dateien"],
  places: ["Kollision platzierter Objekte", "Kollisionen platzierter Objekte"],
};

const de: PreferenceTexts = {
  topic: {
    meshes: "Meshes (3D-Modelle)",
    textures: "Texturen",
    faces: "Gesichter und Figuren",
    files: "andere Dateien",
    places: "Gebäude und platzierte Objekte",
  },
  favourite: (mod, topic) => `${mod} für ${topic}`,
  pair: (mod, over) => `${mod} vor ${over}`,
  pairFor: (mod, over, topic) => `${mod} vor ${over}, für ${topic}`,
  always: (mod) => `${mod}, für alles`,
  kept: (chosen, of) => `${chosen} von ${of} Mal behalten`,
  followed: (count) => `und ${count}-mal mehr durch einen übernommenen Vorschlag`,
  mascot: (count) => `Mal, der Gummihammer – ${count === 1 ? "1 Favorit" : `${count} Favoriten`}`,
  offerTitle: "Mal hat einen Favoriten bemerkt",
  keptSentence: (chosen, of) => `Du hast sie bei ${chosen} von ${of} Entscheidungen behalten.`,
  overChip: (over) => `vor ${over}`,
  offerWhat: (mod, topic, count) =>
    count === 1
      ? `Anwenden bevorzugt ${mod} auch im letzten offenen Fall (${deTopicConflicts[topic][0]}), im Patch, den du gerade bearbeitest.`
      : `Anwenden bevorzugt ${mod} auch in den ${count} weiteren offenen ${deTopicConflicts[topic][1]}, im Patch, den du gerade bearbeitest.`,
  offerPairWhat: (mod, count) =>
    count === 1
      ? `Anwenden bevorzugt ${mod} auch an der 1 weiteren Stelle, an der sich die beiden in die Quere kommen, im Patch, den du gerade bearbeitest.`
      : `Anwenden bevorzugt ${mod} auch an den ${count} weiteren Stellen, an denen sich die beiden in die Quere kommen, im Patch, den du gerade bearbeitest.`,
  pager: (index, count) => `${index} von ${count}`,
  previous: "Voriger Favorit",
  next: "Nächster Favorit",
  showList: "Aufklappen",
  hideList: "Zuklappen",
  apply: (count) => `Auf ${count} anwenden`,
  more: "Mehr",
  never: "Nicht mehr vorschlagen",
  minimise: "Minimieren",
  close: "Schließen",
  managerNote: (mod) =>
    `Bei Konflikten um Dateien wird der Mod-Manager gebeten, die Dateien von ${mod} gewinnen zu lassen – wie bei einer Wahl von Hand.`,
  keepHint:
    "Teile, die das Spiel braucht, bleiben; Stellen, an denen Figuren über das laufen, was wegkäme, bleiben für dich offen. Ein Rückgängig nimmt alles zurück.",
  applyNote:
    "Ist eine Vorliebe eindeutig, zählt Mal, der Gummihammer, klein in der Ecke des Fensters, wie viele warten; klick Mal an, um sie zu sehen. Anwenden entscheidet ihre offenen Konflikte im Patch, den du gerade bearbeitest. Teile, die das Spiel braucht, bleiben; Stellen, an denen Figuren über das laufen, was wegkäme, bleiben für dich offen. Bei Konflikten um Dateien wird der Mod-Manager gebeten, die Dateien der behaltenen Mod gewinnen zu lassen – wie bei einer Wahl von Hand. Ein Rückgängig nimmt die Entscheidungen in Mod Mallet zurück.",
  open: "Öffnen",
  duelRow: (winner, loser) => `${winner} gewinnt gegen ${loser}`,
  spotRow: (loser, objects) => `${objects}: der Teil von ${loser} kommt weg`,
  checking: (done, total) => `Prüfe, wo Figuren laufen … ${done} von ${total}`,
  applied: (count) =>
    count === 1
      ? "1 Konflikt nach deiner Vorliebe entschieden."
      : `${count} Konflikte nach deiner Vorliebe entschieden.`,
  walked: (count) =>
    count === 1
      ? "1 bleibt offen: Figuren laufen über Teile, die wegkämen."
      : `${count} bleiben offen: Figuren laufen über Teile, die wegkämen.`,
  stopped: (message) => `Angehalten, der Mod-Manager ist nicht erreichbar: ${message}`,
  failed: (count) =>
    count === 1
      ? "Der Mod-Manager hat 1 Bitte nicht ausgeführt."
      : `Der Mod-Manager hat ${count} Bitten nicht ausgeführt.`,
  undoAll: "Alles rückgängig",
  undone: "Zurückgenommen.",
  undoNote:
    "Rückgängig nimmt die Entscheidungen in Mod Mallet zurück. Worum der Mod-Manager für Dateien gebeten wurde, bleibt dort, bis du anders wählst.",
  mark: "Vorschlag: deine Vorliebe",
  markFor: (mod) => `Vorschlag: ${mod}, deine Vorliebe`,
  markFrom: "nach deiner Vorliebe",
  markTip: (rule) => `Deine Vorliebe: ${rule}`,
  tab: "Vorlieben",
  title: "Vorlieben zwischen Mods",
  intro:
    "Wenn du immer wieder dieselbe Mod wählst, merkt Mod Mallet das und schlägt sie bei den anderen Konflikten vor. Es lernt nur aus deinen eigenen Entscheidungen auf diesem Computer; nichts wird verschickt.",
  learning: "Lernen",
  learningLabel: "Vorlieben aus meinen Entscheidungen lernen",
  learningNote:
    "Entscheidungen, bei denen du einen Vorschlag übernommen hast, zählen ein Viertel; solche, die durch eine angewandte Vorliebe entstanden, zählen nicht.",
  learned: "Aus deinen Entscheidungen gelernt",
  learnedNone: (min) =>
    `Noch nichts Eindeutiges. Eine Vorliebe erscheint, sobald du eine Mod mindestens ${min}-mal behalten hast, und zwar 4 von 5 Mal oder öfter.`,
  inUse: "in Gebrauch",
  turnedOff: "ausgeschaltet",
  turnOff: "Ausschalten",
  useAgain: "Wieder nutzen",
  openConflicts: (count) => (count === 1 ? "1 offener Konflikt" : `${count} offene Konflikte`),
  applyHere: (count) => `Auf ${count} anwenden`,
  own: "Von dir festgelegt",
  ownNone: "Noch keine.",
  remove: "Entfernen",
  addTitle: "Immer bevorzugen",
  addMod: "Mod",
  addTopic: "Für",
  addOver: "Vor",
  everything: "alles",
  everyOther: "jeder anderen Mod",
  add: "Hinzufügen",
  pickMod: "Mod wählen",
};

export const preferenceTexts: Record<Lang, PreferenceTexts> = { en, de };

export function usePreferenceTexts(): PreferenceTexts {
  return preferenceTexts[useSession().lang];
}
