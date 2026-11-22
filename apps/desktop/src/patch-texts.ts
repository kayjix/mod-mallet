// The texts of the patch page (ADR 0070), in every language of the app.

import type { Lang } from "./i18n";

/** What the patch page says. */
export interface PatchTexts {
  needsSwapper: string;
  shelf: string;
  newPatch: string;
  newPatchName: string;
  newPatchHint: string;
  load: string;
  loadHint: string;
  loadFailed: (reason: string) => string;
  loaded: (name: string) => string;
  active: string;
  activeHint: string;
  by: (author: string) => string;
  decisions: (count: number) => string;
  state: Record<"in-game" | "switched-off" | "not-installed" | "off-here" | "differs", string>;

  does: (hidden: number, moved: number, paths: number, files: number) => string;
  nothing: string;
  foreignNote: (author: string, version: string | null) => string;
  authorUnproven: string;
  loadedFrom: (archive: string) => string;
  makeOwn: string;
  makeOwnHint: string;
  stateInGame: (mod: string) => string;
  stateSwitchedOff: (mod: string) => string;
  stateNotInstalled: string;
  stateOffHere: string;
  stateOffEditing: string;
  differs: (objects: number) => string;
  install: string;
  update: string;
  installHint: string;
  write: string;
  writeHint: string;
  written: string;
  switchOff: string;
  switchOffHint: string;
  switchOn: string;
  switchOnHint: string;
  rename: string;
  save: string;
  cancel: string;
  remove: string;
  confirmRemove: (name: string, decisions: number, inGame: boolean) => string;
  removed: (name: string) => string;
  forget: string;
  forgetHint: string;
  confirmForget: (decisions: number) => string;
  forgotten: string;

  modsTitle: string;
  modsNote: string;
  modMissing: (mod: string, decisions: number) => string;
  modOff: (mod: string, decisions: number) => string;
  modCount: (decisions: number) => string;

  groups: Record<"collisions" | "objects" | "moves" | "paths" | "files" | "other", string>;
  everywhere: (places: number, text: string) => string;
  kept: (kept: string, hidden: string, things: string) => string;
  bothKept: (first: string, second: string) => string;
  theirObjects: string;
  objectHidden: (mod: string, name: string) => string;
  objectShown: (mod: string, name: string) => string;
  moved: (mod: string, name: string, centimetres: number, degrees: number) => string;
  doorMerged: (mod: string, name: string) => string;
  pathsRepaired: (place: string) => string;
  fileChoice: (winner: string, loser: string) => string;
  unknownDuel: string;
  notScanned: string;
  numbered: (number: string) => string;
  notFound: string;
  waits: string;
  waitsHint: string;
  tickHint: string;
  goHint: string;
  moveMenu: string;
  moveTo: (name: string) => string;
  copyTo: (name: string) => string;
  moveAll: string;

  conflictsTitle: string;
  conflict: (first: string, second: string, places: number) => string;
  conflictNote: string;
  keep: (name: string) => string;
  keepHint: (other: string) => string;
  foreignConflict: (name: string) => string;
  thisPatch: string;

  unknownInstalled: (name: string, objects: number) => string;
  takeInstalled: string;
  oldMod: (mod: string) => string;
  switchOldOff: string;
  open: (spots: number, pairs: number) => string;
  toCollisions: string;
  emptyBook: string;
  autoRepairTitle: string;
  autoRepairNote: string;
  autoRepairSummary: (places: number, off: number) => string;
  autoRepairTick: string;
  objectsHere: (count: number) => string;
  chipLabel: string;
  chipHint: string;
  duplicate: string;
  duplicateHint: string;
  copyName: (name: string) => string;
  editingBanner: string;
  edit: string;
  editHint: string;
  inGameOn: string;
  inGameOff: string;
  inPatchNote: string;
  patchLabel: (name: string) => string;
}

const plural = (count: number, one: string, many: string) =>
  count === 1 ? `1 ${one}` : `${count} ${many}`;

const en: PatchTexts = {
  needsSwapper:
    "Hiding objects needs Base Object Swapper, a script extender plugin that is not installed. Moves and repaired paths work without it.",
  shelf: "Your patches",
  newPatch: "New patch",
  newPatchName: "New patch",
  newPatchHint:
    "Starts an empty patch and makes it the one you edit: what you decide from now on goes into it. The other patches keep their decisions.",
  load: "Load a patch…",
  loadHint:
    "Reads a patch archive, your own or one somebody shared, back into Mod Mallet as decisions you can look at, change and install. Nothing is installed by loading.",
  loadFailed: (reason) => `The patch could not be loaded: ${reason}`,
  loaded: (name) => `Loaded “${name}”.`,
  active: "editing",
  activeHint: "You are editing this patch: new decisions go into it.",
  by: (author) => `by ${author}`,
  decisions: (count) => plural(count, "decision", "decisions"),
  state: {
    "in-game": "on in the game",
    "switched-off": "switched off in Vortex",
    "not-installed": "not installed yet",
    "off-here": "switched off",
    differs: "changed since installing",
  },

  does: (hidden, moved, paths, files) => {
    const parts = [
      ...(hidden > 0 ? [`hides ${plural(hidden, "object", "objects")}`] : []),
      ...(moved > 0 ? [`moves ${plural(moved, "object", "objects")}`] : []),
      ...(paths > 0 ? [`repairs the paths in ${plural(paths, "place", "places")}`] : []),
      ...(files > 0 ? [`keeps ${plural(files, "file choice", "file choices")}`] : []),
    ];
    if (parts.length === 0) {
      return "";
    }
    const last = parts.pop();
    const text = parts.length === 0 ? last : `${parts.join(", ")} and ${last}`;
    return `This patch ${text}.`;
  },
  nothing:
    "This patch decides nothing yet. While you edit it, what you decide in Map collisions, Asset collisions and the 3D views goes into it.",
  foreignNote: (author, version) =>
    `Made by ${author}${version === null ? "" : `, version ${version}`}. Mod Mallet does not change its decisions: untick what you don't want, decide over it in a patch of your own, or make it yours.`,
  authorUnproven: "Anybody can write any name as the author; nothing proves it yet.",
  loadedFrom: (archive) => `Loaded from ${archive}.`,
  makeOwn: "Make it mine",
  makeOwnHint:
    "Turns it into your own patch, which you can change. It keeps its name; give it another one before you share it.",
  stateInGame: (mod) => `In the game, as the mod “${mod}” in Vortex.`,
  stateSwitchedOff: (mod) =>
    `Switched off in Vortex (“${mod}”): it does nothing in the game. Its decisions are kept here.`,
  stateNotInstalled: "Not installed yet: nothing of it is in the game.",
  stateOffHere:
    "Switched off: its decisions are kept, but they do not count and the views do not show them.",
  stateOffEditing:
    "Switched off in the game. Because you are editing it, the views still show its decisions.",
  differs: (objects) =>
    `${plural(objects, "decided object is", "decided objects are")} not in the installed version yet. Update it in Vortex to bring them into the game.`,
  install: "Install in Vortex",
  update: "Update in Vortex",
  installHint:
    "Mod Mallet writes this patch as a small mod of its own and asks Vortex to install it. Switch the mod off in Vortex to bring everything back.",
  write: "Only write the archive",
  writeHint:
    "Writes the patch's archive into Mod Mallet's folder without installing it, for example to share it or to install it by hand.",
  written: "Written:",
  switchOff: "Switch off",
  switchOffHint:
    "Takes the patch out of the game through Vortex, and its decisions stop counting here. They are kept: switch it on again any time.",
  switchOn: "Switch on",
  switchOnHint:
    "Its decisions count again, and Vortex switches its mod on again if it is installed.",
  rename: "Rename",
  save: "Save",
  cancel: "Cancel",
  remove: "Remove",
  confirmRemove: (name, decisions, inGame) =>
    `Remove “${name}” and its ${plural(decisions, "decision", "decisions")} from Mod Mallet?${
      inGame
        ? " Its mod stays in Vortex: switch the patch off first to take it out of the game."
        : ""
    } You can undo it right after.`,
  removed: (name) => `“${name}” was removed.`,
  forget: "Forget all decisions",
  forgetHint: "Empties this patch; the other patches stay as they are.",
  confirmForget: (decisions) =>
    `Forget all ${plural(decisions, "decision", "decisions")} of this patch? The views then show these places as undecided. You can undo it right after.`,
  forgotten: "The decisions of this patch were forgotten.",

  modsTitle: "Mods this patch is about",
  modsNote:
    "A decision about a mod you don't have, or have switched off, does nothing in the game and needs nothing from you.",
  modMissing: (mod, decisions) =>
    `You don't have ${mod}; its ${plural(decisions, "decision", "decisions")} in this patch do nothing and need no decision.`,
  modOff: (mod, decisions) =>
    `${mod} is switched off; its ${plural(decisions, "decision", "decisions")} in this patch wait until you switch it on again.`,
  modCount: (decisions) => plural(decisions, "decision", "decisions"),

  groups: {
    collisions: "Places where mods collide",
    objects: "Objects hidden or shown on their own",
    moves: "Moved objects and merged doors",
    paths: "Repaired paths",
    files: "File choices",
    other: "Other decisions",
  },
  everywhere: (places, text) =>
    `Everywhere they collide (${plural(places, "place", "places")}): ${text}`,
  kept: (kept, hidden, things) => `${kept} kept, ${hidden}’s ${things} hidden`,
  bothKept: (first, second) => `${first} and ${second} both kept`,
  theirObjects: "objects",
  objectHidden: (mod, name) => `${mod}’s ${name} hidden`,
  objectShown: (mod, name) => `${mod}’s ${name} shown again`,
  moved: (mod, name, centimetres, degrees) =>
    `${mod}’s ${name} moved by ${centimetres} cm${degrees === 0 ? "" : ` and turned by ${degrees}°`}`,
  doorMerged: (mod, name) => `${mod}’s ${name} merged with the doors stacked on it`,
  pathsRepaired: (place) => `Paths repaired in ${place}`,
  fileChoice: (winner, loser) => `${winner}’s files used where ${loser} brings the same ones`,
  unknownDuel: "A file choice the last scan did not find",
  notScanned: "not in the last scan",
  numbered: (number) => `object ${number}`,
  notFound: "the last scan did not find this place",
  waits: "waits",
  waitsHint: "A mod it is about is switched off; the decision counts again when you switch it on.",
  tickHint:
    "Untick to keep the decision without it counting; Map collisions and the 3D views show the result at once.",
  goHint: "Show where it was decided",
  moveMenu: "Move or copy…",
  moveTo: (name) => `Move to “${name}”`,
  copyTo: (name) => `Copy to “${name}”`,
  moveAll: "Move or copy all…",

  conflictsTitle: "Patches that disagree",
  conflict: (first, second, places) =>
    `“${first}” and “${second}” decide ${plural(places, "place", "places")} differently.`,
  conflictNote:
    "In the game both patches apply: where one hides an object and the other keeps it, it stays hidden; where each hides the other’s object, both are gone.",
  keep: (name) => `Keep “${name}”`,
  keepHint: (other) => `Unticks these decisions in “${other}”.`,
  foreignConflict: (name) =>
    `“${name}” is somebody else’s patch; Mod Mallet does not change it. Keep your own decision with the button, or switch one of the two patches off. A compatibility patch that settles this in the game comes later.`,
  thisPatch: "this patch",

  unknownInstalled: (name, objects) =>
    `The game has the patch “${name}” (${plural(objects, "object", "objects")}), which Mod Mallet does not list yet.`,
  takeInstalled: "Take it into Mod Mallet",
  oldMod: (mod) =>
    `The old mod “${mod}” is still on. Its moves and paths are part of your patch now; once that is installed, switch the old mod off so that nothing is done twice.`,
  switchOldOff: "Switch the old mod off",
  emptyBook: "No patch yet. Start one, or load one somebody shared.",
  autoRepairTitle: "Paths repaired by the patch itself",
  autoRepairNote:
    "Where this patch hides something characters walk on, Mod Mallet repairs the paths there when it writes the patch, on the ground below. It only changes a path where the one the game uses runs over a hidden object; where another mod's path already goes around it, nothing is changed.",
  autoRepairSummary: (places, off) =>
    `Checked when the patch is written, in ${plural(places, "place", "places")} where it hides something${
      off > 0 ? `; turned off in ${off}` : ""
    }.`,
  autoRepairTick: "Untick to leave the paths of this place as they are.",
  objectsHere: (count) => plural(count, "hidden object", "hidden objects"),
  chipLabel: "Decisions go to",
  chipHint:
    "Everything you decide in Map collisions, Go through it, Asset collisions and the 3D views goes into this patch. Pick another one here, or start a new one.",
  duplicate: "Duplicate",
  duplicateHint:
    "Copies this patch into a new one that you then edit, and switches the original off here, so you can work from it as a starting point. Vortex keeps the original until you switch it off there or install the copy.",
  copyName: (name) => `${name} (copy)`,
  editingBanner: "You are editing this patch: what you decide anywhere in Mod Mallet goes into it.",
  edit: "Edit this patch",
  editHint: "From now on, what you decide goes into this patch.",
  inGameOn: "on in the game",
  inGameOff: "off in the game",
  inPatchNote:
    "These come with this patch: installing it brings them into the game, with no mod of their own.",
  patchLabel: (name) => `patch “${name}”`,
  open: (spots, pairs) =>
    `Not decided yet: ${plural(spots, "place", "places")} in ${plural(pairs, "pair", "pairs")} of mods.`,
  toCollisions: "Decide them",
};

const de: PatchTexts = {
  needsSwapper:
    "Zum Ausblenden braucht es Base Object Swapper, ein Script-Extender-Plugin, das nicht installiert ist. Verschiebungen und reparierte Wege gehen ohne.",
  shelf: "Deine Patches",
  newPatch: "Neuer Patch",
  newPatchName: "Neuer Patch",
  newPatchHint:
    "Beginnt einen leeren Patch und macht ihn zu dem, den du bearbeitest: Was du ab jetzt entscheidest, kommt hinein. Die anderen Patches behalten ihre Entscheidungen.",
  load: "Patch laden…",
  loadHint:
    "Liest ein Patch-Archiv – dein eigenes oder eines, das jemand geteilt hat – zurück in Mod Mallet, als Entscheidungen, die du ansehen, ändern und installieren kannst. Beim Laden wird nichts installiert.",
  loadFailed: (reason) => `Der Patch konnte nicht geladen werden: ${reason}`,
  loaded: (name) => `„${name}“ geladen.`,
  active: "wird bearbeitet",
  activeHint: "Du bearbeitest diesen Patch: Neue Entscheidungen kommen hinein.",
  by: (author) => `von ${author}`,
  decisions: (count) => (count === 1 ? "1 Entscheidung" : `${count} Entscheidungen`),
  state: {
    "in-game": "im Spiel an",
    "switched-off": "in Vortex ausgeschaltet",
    "not-installed": "noch nicht installiert",
    "off-here": "ausgeschaltet",
    differs: "seit dem Installieren geändert",
  },

  does: (hidden, moved, paths, files) => {
    const parts = [
      ...(hidden > 0 ? [`blendet ${hidden === 1 ? "1 Objekt" : `${hidden} Objekte`} aus`] : []),
      ...(moved > 0 ? [`verschiebt ${moved === 1 ? "1 Objekt" : `${moved} Objekte`}`] : []),
      ...(paths > 0 ? [`repariert die Wege an ${paths === 1 ? "1 Ort" : `${paths} Orten`}`] : []),
      ...(files > 0
        ? [`hält ${files === 1 ? "1 Dateientscheidung" : `${files} Dateientscheidungen`} fest`]
        : []),
    ];
    if (parts.length === 0) {
      return "";
    }
    const last = parts.pop();
    const text = parts.length === 0 ? last : `${parts.join(", ")} und ${last}`;
    return `Dieser Patch ${text}.`;
  },
  nothing:
    "Dieser Patch entscheidet noch nichts. Solange du ihn bearbeitest, kommt hinein, was du unter Karten-Kollisionen, Asset-Kollisionen und in den 3D-Ansichten entscheidest.",
  foreignNote: (author, version) =>
    `Gemacht von ${author}${version === null ? "" : `, Version ${version}`}. Mod Mallet ändert seine Entscheidungen nicht: Hake ab, was du nicht willst, entscheide in einem eigenen Patch darüber, oder mach ihn zu deinem.`,
  authorUnproven: "Als Autor kann jeder jeden Namen eintragen; noch beweist nichts, wer es war.",
  loadedFrom: (archive) => `Geladen aus ${archive}.`,
  makeOwn: "Zu meinem machen",
  makeOwnHint:
    "Macht ihn zu deinem eigenen Patch, den du ändern kannst. Er behält seinen Namen; gib ihm einen anderen, bevor du ihn teilst.",
  stateInGame: (mod) => `Im Spiel, als Mod „${mod}“ in Vortex.`,
  stateSwitchedOff: (mod) =>
    `In Vortex ausgeschaltet („${mod}“): Im Spiel bewirkt er nichts. Seine Entscheidungen bleiben hier.`,
  stateNotInstalled: "Noch nicht installiert: Nichts davon ist im Spiel.",
  stateOffHere:
    "Ausgeschaltet: Seine Entscheidungen bleiben, zählen aber nicht, und die Ansichten zeigen sie nicht.",
  stateOffEditing:
    "Im Spiel ausgeschaltet. Weil du ihn bearbeitest, zeigen die Ansichten seine Entscheidungen trotzdem.",
  differs: (objects) =>
    `${objects === 1 ? "1 entschiedenes Objekt ist" : `${objects} entschiedene Objekte sind`} noch nicht in der installierten Fassung. Aktualisiere ihn in Vortex, um sie ins Spiel zu bringen.`,
  install: "In Vortex installieren",
  update: "In Vortex aktualisieren",
  installHint:
    "Mod Mallet schreibt diesen Patch als eigene kleine Mod und bittet Vortex, sie zu installieren. Schaltest du die Mod in Vortex aus, ist alles wieder wie vorher.",
  write: "Nur das Archiv schreiben",
  writeHint:
    "Schreibt das Archiv des Patches in den Ordner von Mod Mallet, ohne es zu installieren – etwa zum Teilen oder um es selbst zu installieren.",
  written: "Geschrieben:",
  switchOff: "Ausschalten",
  switchOffHint:
    "Nimmt den Patch über Vortex aus dem Spiel, und seine Entscheidungen zählen hier nicht mehr. Sie bleiben erhalten: Schalte ihn jederzeit wieder ein.",
  switchOn: "Einschalten",
  switchOnHint:
    "Seine Entscheidungen zählen wieder, und Vortex schaltet seine Mod wieder ein, wenn sie installiert ist.",
  rename: "Umbenennen",
  save: "Speichern",
  cancel: "Abbrechen",
  remove: "Entfernen",
  confirmRemove: (name, decisions, inGame) =>
    `„${name}“ mit ${decisions === 1 ? "1 Entscheidung" : `${decisions} Entscheidungen`} aus Mod Mallet entfernen?${
      inGame
        ? " Seine Mod bleibt in Vortex: Schalte den Patch vorher aus, um ihn aus dem Spiel zu nehmen."
        : ""
    } Du kannst es direkt danach rückgängig machen.`,
  removed: (name) => `„${name}“ wurde entfernt.`,
  forget: "Alle Entscheidungen vergessen",
  forgetHint: "Leert diesen Patch; die anderen Patches bleiben, wie sie sind.",
  confirmForget: (decisions) =>
    `Alle ${decisions === 1 ? "1 Entscheidung" : `${decisions} Entscheidungen`} dieses Patches vergessen? Die Ansichten zeigen diese Stellen dann als nicht entschieden. Du kannst es direkt danach rückgängig machen.`,
  forgotten: "Die Entscheidungen dieses Patches wurden vergessen.",

  modsTitle: "Mods, um die es in diesem Patch geht",
  modsNote:
    "Eine Entscheidung über eine Mod, die du nicht hast oder ausgeschaltet hast, bewirkt im Spiel nichts und braucht nichts von dir.",
  modMissing: (mod, decisions) =>
    `Du hast ${mod} nicht; ${decisions === 1 ? "seine 1 Entscheidung" : `seine ${decisions} Entscheidungen`} in diesem Patch bewirken nichts und brauchen keine Entscheidung.`,
  modOff: (mod, decisions) =>
    `${mod} ist ausgeschaltet; ${decisions === 1 ? "seine 1 Entscheidung" : `seine ${decisions} Entscheidungen`} in diesem Patch warten, bis du die Mod wieder einschaltest.`,
  modCount: (decisions) => (decisions === 1 ? "1 Entscheidung" : `${decisions} Entscheidungen`),

  groups: {
    collisions: "Stellen, an denen Mods kollidieren",
    objects: "Einzeln aus- oder wieder eingeblendete Objekte",
    moves: "Verschobene Objekte und zusammengelegte Türen",
    paths: "Reparierte Wege",
    files: "Dateientscheidungen",
    other: "Andere Entscheidungen",
  },
  everywhere: (places, text) =>
    `Überall, wo sie kollidieren (${places === 1 ? "1 Stelle" : `${places} Stellen`}): ${text}`,
  kept: (kept, hidden, things) => `${kept} behalten, ${things} von ${hidden} ausgeblendet`,
  bothKept: (first, second) => `${first} und ${second} beide behalten`,
  theirObjects: "die Objekte",
  objectHidden: (mod, name) => `${name} von ${mod} ausgeblendet`,
  objectShown: (mod, name) => `${name} von ${mod} wieder eingeblendet`,
  moved: (mod, name, centimetres, degrees) =>
    `${name} von ${mod} um ${centimetres} cm verschoben${degrees === 0 ? "" : ` und um ${degrees}° gedreht`}`,
  doorMerged: (mod, name) => `${name} von ${mod} mit den Türen darüber zusammengelegt`,
  pathsRepaired: (place) => `Wege repariert in ${place}`,
  fileChoice: (winner, loser) => `Dateien von ${winner} genommen, wo ${loser} dieselben mitbringt`,
  unknownDuel: "Eine Dateientscheidung, die der letzte Scan nicht gefunden hat",
  notScanned: "nicht im letzten Scan",
  numbered: (number) => `Objekt ${number}`,
  notFound: "diese Stelle hat der letzte Scan nicht gefunden",
  waits: "wartet",
  waitsHint:
    "Eine Mod, um die es geht, ist ausgeschaltet; die Entscheidung zählt wieder, wenn du sie einschaltest.",
  tickHint:
    "Abhaken behält die Entscheidung, ohne dass sie zählt; Karten-Kollisionen und die 3D-Ansichten zeigen das Ergebnis sofort.",
  goHint: "Zeigen, wo das entschieden wurde",
  moveMenu: "Verschieben oder kopieren…",
  moveTo: (name) => `Nach „${name}“ verschieben`,
  copyTo: (name) => `Nach „${name}“ kopieren`,
  moveAll: "Alle verschieben oder kopieren…",

  conflictsTitle: "Patches, die sich widersprechen",
  conflict: (first, second, places) =>
    `„${first}“ und „${second}“ entscheiden ${places === 1 ? "1 Stelle" : `${places} Stellen`} verschieden.`,
  conflictNote:
    "Im Spiel gelten beide Patches: Wo der eine ein Objekt ausblendet und der andere es behält, bleibt es ausgeblendet; wo jeder das Objekt des anderen ausblendet, sind beide weg.",
  keep: (name) => `„${name}“ behalten`,
  keepHint: (other) => `Hakt diese Entscheidungen in „${other}“ ab.`,
  foreignConflict: (name) =>
    `„${name}“ ist der Patch von jemand anderem; Mod Mallet ändert ihn nicht. Behalte mit dem Knopf deine eigene Entscheidung, oder schalte einen der beiden Patches aus. Ein Kompatibilitäts-Patch, der das im Spiel regelt, kommt später.`,
  thisPatch: "diesen Patch",

  unknownInstalled: (name, objects) =>
    `Im Spiel ist der Patch „${name}“ (${objects === 1 ? "1 Objekt" : `${objects} Objekte`}), den Mod Mallet noch nicht aufführt.`,
  takeInstalled: "In Mod Mallet übernehmen",
  oldMod: (mod) =>
    `Die alte Mod „${mod}“ ist noch eingeschaltet. Ihre Verschiebungen und Wege gehören jetzt zu deinem Patch; sobald der installiert ist, schalte die alte Mod aus, damit nichts doppelt geschieht.`,
  switchOldOff: "Alte Mod ausschalten",
  emptyBook: "Noch kein Patch. Beginne einen, oder lade einen, den jemand geteilt hat.",
  autoRepairTitle: "Wege, die der Patch selbst repariert",
  autoRepairNote:
    "Wo dieser Patch etwas ausblendet, über das Figuren laufen, repariert Mod Mallet beim Schreiben des Patches die Wege dort, auf dem Boden darunter. Es ändert einen Weg nur, wo der, den das Spiel nimmt, über ein ausgeblendetes Objekt führt; wo der Weg einer anderen Mod schon darum herumführt, bleibt alles, wie es ist.",
  autoRepairSummary: (places, off) =>
    `Wird beim Schreiben geprüft, an ${places === 1 ? "1 Ort" : `${places} Orten`}, an denen er etwas ausblendet${
      off > 0 ? `; an ${off} abgeschaltet` : ""
    }.`,
  autoRepairTick: "Abhaken lässt die Wege dieses Ortes, wie sie sind.",
  objectsHere: (count) =>
    count === 1 ? "1 ausgeblendetes Objekt" : `${count} ausgeblendete Objekte`,
  chipLabel: "Entscheidungen gehen in",
  chipHint:
    "Alles, was du unter Karten-Kollisionen, Durchgehen, Asset-Kollisionen und in den 3D-Ansichten entscheidest, kommt in diesen Patch. Wähle hier einen anderen, oder beginne einen neuen.",
  duplicate: "Duplizieren",
  duplicateHint:
    "Kopiert diesen Patch in einen neuen, den du dann bearbeitest, und schaltet das Original hier aus – so arbeitest du von ihm aus weiter. In Vortex bleibt das Original, bis du es dort ausschaltest oder die Kopie installierst.",
  copyName: (name) => `${name} (Kopie)`,
  editingBanner:
    "Du bearbeitest diesen Patch: Was du irgendwo in Mod Mallet entscheidest, kommt hinein.",
  edit: "Diesen Patch bearbeiten",
  editHint: "Ab jetzt kommt, was du entscheidest, in diesen Patch.",
  inGameOn: "im Spiel an",
  inGameOff: "im Spiel aus",
  inPatchNote:
    "Diese kommen mit diesem Patch: Installierst du ihn, sind sie im Spiel, ohne eigene Mod.",
  patchLabel: (name) => `Patch „${name}“`,
  open: (spots, pairs) =>
    `Noch nicht entschieden: ${spots === 1 ? "1 Stelle" : `${spots} Stellen`} in ${pairs === 1 ? "1 Mod-Paar" : `${pairs} Mod-Paaren`}.`,
  toCollisions: "Entscheiden",
};

export const patchTexts: Record<Lang, PatchTexts> = { en, de };
