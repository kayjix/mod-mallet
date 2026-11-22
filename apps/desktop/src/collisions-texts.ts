// Texts of the collisions' page that came with ADR 0048 (package B): kept
// apart from `i18n.ts`, as every package of that plan does.

import type { Lang } from "./i18n";

export interface CollisionsTexts {
  also: string;
  alsoNote: string;
  takeSuggestions: (count: number) => string;
  openThePair: string;
  showOnMap: string;
  compareFiles: string;
  aroundWaiting: string;
  aroundPreview: (plugin: string) => string;
  aroundCareful: (count: number) => string;
  aroundCarefulHint: string;
  hiddenByItself: string;
  hiddenByDecision: string;
  /** An object hidden by a decision of a patch other than the one being edited (ADR 0070). */
  hiddenInPatch: (patch: string) => string;
  /** An object a decision about a switched-off mod would hide (ADR 0061). */
  hiddenWhenOn: string;
  hideHint: string;
  /** The middle column of a spot: what concerns both mods. */
  common: string;
  /** How many spots the two mods collide at, where they are few. */
  pairSpots: (first: string, second: string, spots: number) => string;
  /** Hiding one mod's part at every spot of the passage. */
  passageHide: (spots: number) => string;
  /** The tool that sets how far around the spot reaches. */
  aroundTool: string;
  aroundHint: string;
  metres: (metres: number) => string;
  /** Hiding one mod's objects around the spot. */
  aroundHideHere: (count: number, metres: number) => string;
  partsShow: (count: number) => string;
  partsHide: (count: number) => string;
  partsSomeTied: (count: number) => string;
}

export const collisionsTexts: Record<Lang, CollisionsTexts> = {
  en: {
    also: "Also",
    alsoNote: "Collisions of these mods that name no spot to look at.",
    takeSuggestions: (count) =>
      count === 1 ? "Take the 1 suggestion" : `Take every suggestion (${count})`,
    openThePair: "Decide this pair as a whole",
    showOnMap: "Show on the map",
    compareFiles: "Compare their files",
    aroundWaiting: "Come close in 3D to see what stands around.",
    aroundPreview: (plugin) =>
      `Look at it without ${plugin}'s objects around here, without deciding`,
    hiddenByItself: "Hidden on its own by the patch.",
    hiddenByDecision:
      "Hidden by the patch, through a decision of a collision. Showing it again keeps both objects at that spot.",
    hiddenInPatch: (patch) =>
      `The patch “${patch}” decides this, not the one you are editing. Showing it again leaves the decision out of that patch.`,
    hiddenWhenOn:
      "A decision about a mod that is switched off would hide it. It waits until that mod is on again.",
    hideHint: "Only this object goes; the patch hides it while the game runs and changes no file.",
    common: "Both",
    pairSpots: (first, second, spots) => `${first} and ${second} collide at ${spots} spots.`,
    passageHide: (spots) => `Hide it in this passage (${spots} spots)`,
    aroundTool: "Around",
    aroundHint:
      "How far around the spot each mod's objects count, for hiding all of one mod's objects here at once.",
    metres: (metres) => `${metres} m`,
    aroundHideHere: (count, metres) =>
      count === 1
        ? `Hide its 1 object within ${metres} m`
        : `Hide its ${count} objects within ${metres} m`,
    partsShow: (count) => `Show these ${count} again`,
    partsHide: (count) => `Hide these ${count}`,
    partsSomeTied: (count) => `${count} of them the game relies on and stay. `,
    aroundCareful: (count) => `${count} of them with care`,
    aroundCarefulHint:
      "The game may miss these: they are drawn from afar, or something refers to them. Pick one in 3D to see why.",
  },
  de: {
    also: "Außerdem",
    alsoNote: "Kollisionen dieser Mods, die keine Stelle zum Ansehen nennen.",
    takeSuggestions: (count) =>
      count === 1 ? "Den 1 Vorschlag übernehmen" : `Alle Vorschläge übernehmen (${count})`,
    openThePair: "Dieses Paar als Ganzes entscheiden",
    showOnMap: "Auf der Karte zeigen",
    compareFiles: "Ihre Dateien vergleichen",
    aroundWaiting: "Komm in 3D näher, um zu sehen, was hier steht.",
    aroundPreview: (plugin) => `Ohne die Objekte von ${plugin} hier ansehen, ohne zu entscheiden`,
    hiddenByItself: "Vom Patch einzeln ausgeblendet.",
    hiddenByDecision:
      "Vom Patch ausgeblendet, über die Entscheidung einer Kollision. Wieder zeigen behält an dieser Stelle beide Objekte.",
    hiddenInPatch: (patch) =>
      `Das entscheidet der Patch „${patch}“, nicht der, den du gerade bearbeitest. Wieder zeigen lässt die Entscheidung in diesem Patch weg.`,
    hiddenWhenOn:
      "Eine Entscheidung zu einer ausgeschalteten Mod würde es ausblenden. Sie wartet, bis die Mod wieder an ist.",
    hideHint:
      "Nur dieses Objekt verschwindet; der Patch blendet es aus, während das Spiel läuft, und ändert keine Datei.",
    common: "Beide",
    pairSpots: (first, second, spots) => `${first} und ${second} kollidieren an ${spots} Stellen.`,
    passageHide: (spots) => `In dieser Passage ausblenden (${spots} Stellen)`,
    aroundTool: "Umkreis",
    aroundHint:
      "Wie weit um die Stelle die Objekte jeder Mod zählen, um alle Objekte einer Mod hier auf einmal auszublenden.",
    metres: (metres) => `${metres} m`,
    aroundHideHere: (count, metres) =>
      count === 1
        ? `Ihr 1 Objekt im Umkreis von ${metres} m ausblenden`
        : `Ihre ${count} Objekte im Umkreis von ${metres} m ausblenden`,
    partsShow: (count) => `Diese ${count} wieder zeigen`,
    partsHide: (count) => `Diese ${count} ausblenden`,
    partsSomeTied: (count) => `${count} davon braucht das Spiel; die bleiben. `,
    aroundCareful: (count) => `davon ${count} mit Vorsicht`,
    aroundCarefulHint:
      "Das Spiel könnte sie vermissen: Sie werden aus der Ferne gezeigt, oder etwas verweist auf sie. Klick eines in 3D an, um zu sehen, warum.",
  },
};
