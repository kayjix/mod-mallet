// The texts of the patch page, in every language of the app.

import type { Lang } from "./i18n";

/** What the patch page says. */
export interface PatchTexts {
  needsSwapper: string;
  /** The objects the decisions hide, installed and not yet, in one line. */
  summary: (chosen: number, installed: number | null, waiting: number) => string;
  installedBy: (mod: string) => string;
  create: string;
  install: string;
  installHint: string;
  clear: string;
  clearHint: string;
  created: string;
  stale: string;
  useInstalled: string;
  useInstalledHint: string;
  differsHow: string;
  confirmClear: (objects: number) => string;
  confirmClearYes: string;
  confirmInstalled: (waiting: number) => string;
  confirmInstalledYes: string;
  cancel: string;
  replaced: string;
  undoReplace: string;
  decided: string;
  nothingDecided: string;
  /** What was decided for a pair as a whole. */
  hide: (plugin: string) => string;
  keepBoth: string;
  mixed: string;
  spotsOnly: string;
  objects: (count: number) => string;
  ownSpots: (count: number) => string;
  change: string;
  changeHint: string;
  undo: string;
  inPatch: string;
  waitingMark: string;
  /** Decided, but not in the installed patch yet. */
  waiting: (objects: number, pairs: number) => string;
  /** Not decided yet. */
  open: (spots: number, pairs: number) => string;
  toCollisions: string;
  own: (count: number) => string;
  ownNote: string;
  showAgain: string;
}

const en: PatchTexts = {
  needsSwapper:
    "A patch hides objects with Base Object Swapper, a script extender plugin that is not installed.",
  summary: (chosen, installed, waiting) =>
    [
      chosen === 1 ? "1 object hidden" : `${chosen} objects hidden`,
      installed === null ? "no patch installed" : `installed: ${installed}`,
      ...(waiting > 0 ? [`${waiting} not in the game yet`] : []),
    ].join(" · "),
  installedBy: (mod) => `The installed patch is the mod “${mod}”.`,
  create: "Create",
  install: "Install through Vortex",
  installHint:
    "Mod Mallet writes a small mod that shows an invisible marker in place of these objects while the game runs. Remove the mod in Vortex to bring them back.",
  clear: "Clear",
  clearHint: "Forget every decision; the patch then hides nothing.",
  created: "Created:",
  stale: "The decisions changed since. Create the patch again before installing it.",
  useInstalled: "Go back to the installed patch",
  useInstalledHint:
    "Throws away what you decided since the patch was installed, and takes the installed patch's choices instead.",
  differsHow:
    "Your decisions differ from the installed patch. To bring them into the game: Create, then Install through Vortex.",
  confirmClear: (objects) =>
    `Forget every decision? The patch would then hide nothing instead of ${objects} objects. You can undo it right after.`,
  confirmClearYes: "Forget every decision",
  confirmInstalled: (waiting) =>
    `Throw away what you decided since the patch was installed (${waiting} objects not in the game yet), and go back to it? To keep your decisions instead, create and install the patch. You can undo it right after.`,
  confirmInstalledYes: "Go back to the installed patch",
  cancel: "Cancel",
  replaced: "Your decisions were replaced.",
  undoReplace: "Undo",
  decided: "Decided",
  nothingDecided:
    "Nothing is decided yet. Decide the collisions in Collisions; what you decide is listed here.",
  hide: (plugin) => `hide ${plugin}’s objects`,
  keepBoth: "keep both",
  mixed: "decided differently for its kinds of collision",
  spotsOnly: "decided spot by spot",
  objects: (count) => (count === 1 ? "1 object" : `${count} objects`),
  ownSpots: (count) =>
    count === 1 ? "1 spot decided on its own" : `${count} spots decided on their own`,
  change: "Change",
  changeHint: "Open it in Collisions",
  undo: "Undo",
  inPatch: "in the patch",
  waitingMark: "waiting",
  waiting: (objects, pairs) =>
    `Waiting: ${objects === 1 ? "1 decided object is" : `${objects} decided objects are`} not in the installed patch yet${
      pairs > 0 ? `, in ${pairs === 1 ? "1 pair" : `${pairs} pairs`} of mods` : ""
    }. Create the patch and install it.`,
  open: (spots, pairs) =>
    `Not decided yet: ${spots === 1 ? "1 spot" : `${spots} spots`} in ${pairs === 1 ? "1 pair" : `${pairs} pairs`} of mods.`,
  toCollisions: "To Collisions",
  own: (count) =>
    count === 1 ? "1 object hidden on its own" : `${count} objects hidden on their own`,
  ownNote:
    "Picked one by one in the 3D view, or in the installed patch without a collision of this scan.",
  showAgain: "Show again",
};

const de: PatchTexts = {
  needsSwapper:
    "Ein Patch blendet Objekte mit Base Object Swapper aus, einem Script-Extender-Plugin, das nicht installiert ist.",
  summary: (chosen, installed, waiting) =>
    [
      chosen === 1 ? "1 Objekt ausgeblendet" : `${chosen} Objekte ausgeblendet`,
      installed === null ? "kein Patch installiert" : `installiert: ${installed}`,
      ...(waiting > 0 ? [`${waiting} noch nicht im Spiel`] : []),
    ].join(" · "),
  installedBy: (mod) => `Der installierte Patch ist die Mod „${mod}“.`,
  create: "Erstellen",
  install: "Über Vortex installieren",
  installHint:
    "Mod Mallet schreibt eine kleine Mod, die diese Objekte im Spiel durch einen unsichtbaren Marker ersetzt. Entfernst du die Mod in Vortex, sind sie wieder da.",
  clear: "Zurücksetzen",
  clearHint: "Alle Entscheidungen vergessen; der Patch blendet dann nichts aus.",
  created: "Erstellt:",
  stale:
    "Die Entscheidungen haben sich seitdem geändert. Erstelle den Patch neu, bevor du ihn installierst.",
  useInstalled: "Zurück zum installierten Patch",
  useInstalledHint:
    "Verwirft, was du seit dem Installieren entschieden hast, und nimmt stattdessen die Auswahl des installierten Patches.",
  differsHow:
    "Deine Entscheidungen weichen vom installierten Patch ab. Um sie ins Spiel zu bringen: Erstellen, dann Über Vortex installieren.",
  confirmClear: (objects) =>
    `Alle Entscheidungen vergessen? Der Patch würde dann nichts mehr ausblenden statt ${objects} Objekte. Du kannst es direkt danach rückgängig machen.`,
  confirmClearYes: "Alle Entscheidungen vergessen",
  confirmInstalled: (waiting) =>
    `Verwerfen, was du seit dem Installieren entschieden hast (${waiting} Objekte noch nicht im Spiel), und zum installierten Patch zurück? Um deine Entscheidungen zu behalten, erstelle und installiere stattdessen den Patch. Du kannst es direkt danach rückgängig machen.`,
  confirmInstalledYes: "Zurück zum installierten Patch",
  cancel: "Abbrechen",
  replaced: "Deine Entscheidungen wurden ersetzt.",
  undoReplace: "Rückgängig",
  decided: "Entschieden",
  nothingDecided:
    "Noch ist nichts entschieden. Entscheide die Kollisionen unter Kollisionen; was du entscheidest, steht dann hier.",
  hide: (plugin) => `Objekte von ${plugin} ausblenden`,
  keepBoth: "beide behalten",
  mixed: "je nach Art der Kollision anders entschieden",
  spotsOnly: "Stelle für Stelle entschieden",
  objects: (count) => (count === 1 ? "1 Objekt" : `${count} Objekte`),
  ownSpots: (count) =>
    count === 1 ? "1 Stelle einzeln entschieden" : `${count} Stellen einzeln entschieden`,
  change: "Ändern",
  changeHint: "Unter Kollisionen öffnen",
  undo: "Rückgängig",
  inPatch: "im Patch",
  waitingMark: "wartet",
  waiting: (objects, pairs) =>
    `Wartet: ${objects === 1 ? "1 entschiedenes Objekt ist" : `${objects} entschiedene Objekte sind`} noch nicht im installierten Patch${
      pairs > 0 ? `, in ${pairs === 1 ? "1 Mod-Paar" : `${pairs} Mod-Paaren`}` : ""
    }. Erstelle den Patch und installiere ihn.`,
  open: (spots, pairs) =>
    `Noch nicht entschieden: ${spots === 1 ? "1 Stelle" : `${spots} Stellen`} in ${pairs === 1 ? "1 Mod-Paar" : `${pairs} Mod-Paaren`}.`,
  toCollisions: "Zu den Kollisionen",
  own: (count) =>
    count === 1 ? "1 Objekt einzeln ausgeblendet" : `${count} Objekte einzeln ausgeblendet`,
  ownNote:
    "Einzeln in der 3D-Ansicht gewählt, oder im installierten Patch ohne Kollision dieses Scans.",
  showAgain: "Wieder zeigen",
};

export const patchTexts: Record<Lang, PatchTexts> = { en, de };
