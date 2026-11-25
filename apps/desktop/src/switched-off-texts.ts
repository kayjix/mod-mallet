// Texts about mods that are switched off (ADR 0061) and about mods that
// collide with very many others (ADR 0043), in every language of the app.

import type { Lang } from "./i18n";

/** Names for a line of text: the first few, and how many more. */
function named(names: string[], more: (count: number) => string, limit = 3): string {
  const shown = names.slice(0, limit).join(", ");
  return names.length > limit ? `${shown} ${more(names.length - limit)}` : shown;
}

export interface SwitchedOffTexts {
  /** Decisions about switched-off mods: kept, not counted, not in the patch. */
  waiting: (decisions: number, mods: string[]) => string;
  waitingNote: string;
  /** The game loads plugins of switched-off mods all the same. */
  stillLoaded: (plugins: string[]) => string;
  /** On a collision with such a plugin. */
  stillLoadedSpot: (mod: string, plugin: string) => string;
  toTasks: string;
  /** The patch in the game hides objects of plugins of switched-off mods. */
  installed: (objects: number, plugins: string[]) => string;
  installedNote: string;
  makePatch: string;
  /** On a mod's page: the mod is off, but its plugin still loaded (ADR 0073). */
  modLead: (mod: string, plugin: string, spots: number) => string;
  /** The files that keep it in the game, which Vortex did not put there. */
  notVortex: (files: number) => string;
  /** Vortex has not deployed since the mod went off. */
  notDeployed: string;
  /** After switching off: what a new scan shows. */
  afterwards: string;
  /** Before switching off a plugin others build on. */
  confirmPlugins: (plugin: string, others: string[]) => string;
  confirmYes: string;
  cancel: string;
  /** Opens the Data folder with the plugin selected. */
  showFiles: string;
  /** What to do with the files: by hand, since Mod Mallet moves nothing. */
  moveOut: (files: number) => string;
  /** On a mod's page: the mod is out of the game, but files are left. */
  leftLead: (mod: string, files: number) => string;
}

export interface WideTexts {
  /** The loud advice: think about removing the mod. */
  remove: (mod: string, places: number, mods: number) => string;
  removeNote: string;
  /** What relies on it, said before any advice to remove it. */
  needs: (mod: string, mods: number, dependents: string) => string;
  needsNothing: (mod: string) => string;
  models: (mod: string, users: string) => string;
  switchOff: string;
  /** Before switching off a mod others build on. */
  confirm: (mod: string, dependents: string) => string;
  confirmYes: string;
  cancel: string;
  /** Quieter: most of its places are small, clear decisions. */
  bulk: (mod: string, places: number, clear: number) => string;
  bulkNote: string;
  toReview: string;
  /** Quietest: it furnishes rooms of its own. */
  keep: (mod: string, places: number, rooms: number) => string;
  showMod: string;
  /** The tag in the list of mods. */
  tagRemove: string;
  tagBulk: string;
  tagKeep: string;
}

const enMore = (count: number) => `and ${count} more`;
const deMore = (count: number) => `und ${count} weitere`;

export const switchedOffTexts: Record<Lang, SwitchedOffTexts> = {
  en: {
    waiting: (decisions, mods) =>
      `${decisions === 1 ? "1 decision waits" : `${decisions} decisions wait`} for ${
        mods.length === 1 ? "a mod" : "mods"
      } that ${mods.length === 1 ? "is" : "are"} switched off (${named(mods, enMore)}).`,
    waitingNote:
      "They are kept, but do not count and do not go into the patch. When you switch the mod on again and scan, they are back.",
    stillLoaded: (plugins) =>
      `The game still loads ${named(plugins, enMore)}, though its mod is switched off. Its collisions are real until it is out of the game; the tasks say how to take it out.`,
    stillLoadedSpot: (mod, plugin) =>
      `You switched ${mod} off, but the game still loads ${plugin}: Vortex has not deployed since, or a copy it did not install is still in the Data folder. This collision is real until the plugin is out of the game.`,
    toTasks: "What to do",
    installed: (objects, plugins) =>
      `The patch in the game still hides ${objects === 1 ? "1 object" : `${objects} objects`} of ${named(
        plugins,
        enMore,
      )}, from a mod that is switched off.`,
    installedNote:
      "The next patch leaves them out. Your decisions about them are kept for when the mod is on again.",
    makePatch: "Make the patch without them",
    modLead: (mod, plugin, spots) =>
      `You switched ${mod} off, but the game still loads ${plugin}. ${
        spots === 0
          ? "Until it is out of the game, its objects stay in the world."
          : `Its ${spots === 1 ? "collision" : `${spots} collisions`} below ${
              spots === 1 ? "stays" : "stay"
            } until it is out of the game.`
      }`,
    notVortex: (files) =>
      `${files === 1 ? "This file is" : "These files are"} still in the game's Data folder. Vortex did not put ${
        files === 1 ? "it" : "them"
      } there, so deploying does not take ${files === 1 ? "it" : "them"} away:`,
    notDeployed:
      "Vortex has not deployed since you switched the mod off. Deploy in Vortex, then scan again.",
    afterwards:
      "Once Vortex has switched it off, scan again: its collisions are gone, and what you decided about them waits for the mod.",
    confirmPlugins: (plugin, others) =>
      `Vortex switches off ${plugin} and, with it, ${
        others.length === 1
          ? "the plugin that builds on it"
          : `the ${others.length} plugins that build on it`
      }: ${named(others, enMore, 5)}. Without ${plugin} the game would not start with ${
        others.length === 1 ? "it" : "them"
      }. Nothing is deleted; you can tick them again in Vortex's list of plugins. Go ahead?`,
    confirmYes: "Switch them off",
    cancel: "Cancel",
    showFiles: "Show in the Data folder",
    moveOut: (files) =>
      `Mod Mallet does not move or delete ${files === 1 ? "it" : "them"}. To tidy up, move ${
        files === 1 ? "it" : "them"
      } out of the Data folder yourself, for example into a folder next to it, and scan again.`,
    leftLead: (mod, files) =>
      `${mod} is out of the game, but ${
        files === 1 ? "a file of it is" : `${files} files of it are`
      } still in the Data folder. Vortex did not put ${files === 1 ? "it" : "them"} there.`,
  },
  de: {
    waiting: (decisions, mods) =>
      `${decisions === 1 ? "1 Entscheidung wartet" : `${decisions} Entscheidungen warten`} auf ${
        mods.length === 1 ? "eine Mod, die" : "Mods, die"
      } ausgeschaltet ${mods.length === 1 ? "ist" : "sind"} (${named(mods, deMore)}).`,
    waitingNote:
      "Sie bleiben erhalten, zählen aber nicht und kommen nicht in den Patch. Schaltest du die Mod wieder ein und scannst, sind sie wieder da.",
    stillLoaded: (plugins) =>
      `Das Spiel lädt ${named(plugins, deMore)} noch, obwohl die Mod ausgeschaltet ist. Bis es aus dem Spiel ist, sind seine Kollisionen echt; unter den Aufgaben steht, wie du es herausnimmst.`,
    stillLoadedSpot: (mod, plugin) =>
      `Du hast ${mod} ausgeschaltet, aber das Spiel lädt ${plugin} noch: Vortex hat seitdem nicht bereitgestellt, oder eine Kopie, die Vortex nicht installiert hat, liegt noch im Data-Ordner. Diese Kollision ist echt, bis das Plugin aus dem Spiel ist.`,
    toTasks: "Was zu tun ist",
    installed: (objects, plugins) =>
      `Der Patch im Spiel versteckt noch ${objects === 1 ? "1 Objekt" : `${objects} Objekte`} von ${named(
        plugins,
        deMore,
      )}, aus einer Mod, die ausgeschaltet ist.`,
    installedNote:
      "Der nächste Patch lässt sie weg. Deine Entscheidungen dazu bleiben erhalten, für den Fall, dass du die Mod wieder einschaltest.",
    makePatch: "Patch ohne sie erstellen",
    modLead: (mod, plugin, spots) =>
      `Du hast ${mod} ausgeschaltet, aber das Spiel lädt ${plugin} noch. ${
        spots === 0
          ? "Bis es aus dem Spiel ist, bleiben seine Objekte in der Welt."
          : `${spots === 1 ? "Seine Kollision unten bleibt" : `Seine ${spots} Kollisionen unten bleiben`}, bis es aus dem Spiel ist.`
      }`,
    notVortex: (files) =>
      `${files === 1 ? "Diese Datei liegt" : "Diese Dateien liegen"} noch im Data-Ordner des Spiels. Vortex hat sie nicht dorthin gelegt, darum nimmt Bereitstellen sie auch nicht weg:`,
    notDeployed:
      "Vortex hat seit dem Ausschalten nicht bereitgestellt. Stelle in Vortex bereit und scanne dann erneut.",
    afterwards:
      "Hat Vortex es ausgeschaltet, scanne erneut: Seine Kollisionen sind dann weg, und was du dazu entschieden hast, wartet auf die Mod.",
    confirmPlugins: (plugin, others) =>
      `Vortex schaltet ${plugin} aus und mit ihm ${
        others.length === 1
          ? "das Plugin, das darauf aufbaut"
          : `die ${others.length} Plugins, die darauf aufbauen`
      }: ${named(others, deMore, 5)}. Ohne ${plugin} würde das Spiel mit ${
        others.length === 1 ? "ihm" : "ihnen"
      } nicht starten. Gelöscht wird nichts; in Vortex' Plugin-Liste kannst du die Haken wieder setzen. Fortfahren?`,
    confirmYes: "Ausschalten",
    cancel: "Abbrechen",
    showFiles: "Im Data-Ordner zeigen",
    moveOut: () =>
      "Mod Mallet verschiebt und löscht sie nicht. Zum Aufräumen verschiebe sie selbst aus dem Data-Ordner, zum Beispiel in einen Ordner „_aussortiert“ daneben, und scanne erneut.",
    leftLead: (mod, files) =>
      `${mod} ist aus dem Spiel, aber ${
        files === 1 ? "eine Datei davon liegt" : `${files} Dateien davon liegen`
      } noch im Data-Ordner. Vortex hat sie nicht dorthin gelegt.`,
  },
};

export const wideTexts: Record<Lang, WideTexts> = {
  en: {
    remove: (mod, places, mods) =>
      `${mod} collides with very many others: ${places} places, with ${
        mods === 1 ? "1 other mod" : `${mods} other mods`
      }. Think about removing it.`,
    removeNote:
      "Deciding each place takes long, and a patch rarely leaves such a mod looking right. If you can do without it, switching it off is the quicker way; you can switch it on again in Vortex.",
    needs: (mod, mods, dependents) =>
      `${mods === 1 ? "1 other mod needs" : `${mods} other mods need`} ${mod}: ${dependents}. Removing it breaks ${mods === 1 ? "it" : "them"} unless you remove ${mods === 1 ? "it" : "them"} too.`,
    needsNothing: (mod) => `No other mod's plugin builds on ${mod}.`,
    models: (mod, users) =>
      `Objects of other mods show models only ${mod} brings (${users}); without it they would be missing.`,
    switchOff: "Switch it off in Vortex",
    confirm: (mod, dependents) =>
      `Switching ${mod} off leaves these plugins without the plugin they build on: ${dependents}. The game will not start until you switch them off too. Switch it off anyway?`,
    confirmYes: "Switch it off",
    cancel: "Cancel",
    bulk: (mod, places, clear) =>
      `${mod} collides in ${places} places, but in ${clear} of them one object is far larger than the other, so the decision is clear.`,
    bulkNote:
      "Go through them step by step and take every clear suggestion at once; only the rest needs a look.",
    toReview: "Step by step",
    keep: (mod, places, rooms) =>
      `${mod} collides in ${places} places, but furnishes ${
        rooms === 1 ? "a room" : `${rooms} rooms`
      } of its own: keep it and decide only these places.`,
    showMod: "Its collisions",
    tagRemove: "Think about removing it",
    tagBulk: "Many clear places",
    tagKeep: "Has rooms of its own",
  },
  de: {
    remove: (mod, places, mods) =>
      `${mod} kollidiert mit sehr vielen anderen: an ${places} Orten, mit ${
        mods === 1 ? "1 anderen Mod" : `${mods} anderen Mods`
      }. Überleg, sie zu entfernen.`,
    removeNote:
      "Jeden Ort zu entscheiden dauert lange, und ein Patch macht so eine Mod selten stimmig. Kannst du auf sie verzichten, ist Ausschalten der schnellere Weg; in Vortex kannst du sie wieder einschalten.",
    needs: (mod, mods, dependents) =>
      `${mods === 1 ? "1 andere Mod braucht" : `${mods} andere Mods brauchen`} ${mod}: ${dependents}. Entfernst du sie, ${mods === 1 ? "geht diese" : "gehen diese"} kaputt, außer du entfernst sie mit.`,
    needsNothing: (mod) => `Kein Plugin einer anderen Mod baut auf ${mod} auf.`,
    models: (mod, users) =>
      `Objekte anderer Mods zeigen Modelle, die nur ${mod} mitbringt (${users}); ohne sie würden sie fehlen.`,
    switchOff: "In Vortex ausschalten",
    confirm: (mod, dependents) =>
      `Schaltest du ${mod} aus, fehlt diesen Plugins das Plugin, auf dem sie aufbauen: ${dependents}. Das Spiel startet erst wieder, wenn du sie auch ausschaltest. Trotzdem ausschalten?`,
    confirmYes: "Ausschalten",
    cancel: "Abbrechen",
    bulk: (mod, places, clear) =>
      `${mod} kollidiert an ${places} Orten, aber an ${clear} davon ist ein Objekt viel größer als das andere, die Entscheidung also klar.`,
    bulkNote:
      "Geh sie durch und übernimm alle klaren Vorschläge auf einmal; nur der Rest braucht einen Blick.",
    toReview: "Schritt für Schritt",
    keep: (mod, places, rooms) =>
      `${mod} kollidiert an ${places} Orten, richtet aber ${
        rooms === 1 ? "einen eigenen Raum" : `${rooms} eigene Räume`
      } ein: Behalte sie und entscheide nur diese Orte.`,
    showMod: "Ihre Kollisionen",
    tagRemove: "Überleg, sie zu entfernen",
    tagBulk: "Viele klare Orte",
    tagKeep: "Hat eigene Räume",
  },
};
