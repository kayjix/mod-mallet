// Texts of the collisions' page that came with ADR 0048 (package B): kept
// apart from `i18n.ts`, as every package of that plan does.

import type { Lang } from "./i18n";

/** A length given in centimetres, in metres from one metre on, with `point` as decimal mark. */
function length(centimetres: number, point: string): string {
  if (centimetres < 100) {
    return `${centimetres} cm`;
  }
  return `${(centimetres / 100).toFixed(1).replace(".", point)} m`;
}

export interface CollisionsTexts {
  also: string;
  alsoNote: string;
  takeSuggestions: (count: number) => string;
  openThePair: string;
  showOnMap: string;
  compareFiles: string;
  around: (metres: number) => string;
  aroundWaiting: string;
  aroundPreview: (plugin: string) => string;
  aroundPreviewShort: string;
  aroundHide: (plugin: string, count: number) => string;
  aroundCareful: (count: number) => string;
  aroundCarefulHint: string;
  hiddenByItself: string;
  hiddenByDecision: string;
  hideHint: string;
  partsAlone: string;
  partsTogether: (count: number) => string;
  partsReach: (centimetres: number) => string;
  partsHulls: string;
  partsShow: (count: number) => string;
  partsHide: (count: number) => string;
  partsSomeTied: (count: number) => string;
  partsHint: string;
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
    around: (metres) => `Around · ${metres} m`,
    aroundWaiting: "Come close in 3D to see what stands around.",
    aroundPreview: (plugin) =>
      `Look at it without ${plugin}'s objects around here, without deciding`,
    aroundPreviewShort: "Preview",
    hiddenByItself: "Hidden on its own by the patch.",
    hiddenByDecision:
      "Hidden by the patch, through a decision of a collision. Showing it again keeps both objects at that spot.",
    hideHint: "Only this object goes; the patch hides it while the game runs and changes no file.",
    partsAlone: "No other part of this mod lies within its reach.",
    partsTogether: (count) =>
      `Linked by their reach with other parts of the same mod: ${count} in all`,
    partsReach: (centimetres) => `Each part reaches ${length(centimetres, ".")} beyond itself`,
    partsHulls: "Show how far each part reaches",
    partsShow: (count) => `Show all ${count} again`,
    partsHide: (count) => `Hide all ${count}`,
    partsSomeTied: (count) => `${count} of them the game relies on and stay. `,
    partsHint:
      "The chain goes from part to part of this mod while each lies within the reach of one before; parts of other mods are passed over. More reach: also parts a little apart, such as the posts under a roof.",
    aroundCareful: (count) => `${count} of them with care`,
    aroundCarefulHint:
      "The game may miss these: they are drawn from afar, or something refers to them. Pick one in 3D to see why.",
    aroundHide: (plugin, count) =>
      count === 1 ? `Hide 1 object of ${plugin}` : `Hide ${count} objects of ${plugin}`,
  },
  de: {
    also: "Außerdem",
    alsoNote: "Kollisionen dieser Mods, die keine Stelle zum Ansehen nennen.",
    takeSuggestions: (count) =>
      count === 1 ? "Den 1 Vorschlag übernehmen" : `Alle Vorschläge übernehmen (${count})`,
    openThePair: "Dieses Paar als Ganzes entscheiden",
    showOnMap: "Auf der Karte zeigen",
    compareFiles: "Ihre Dateien vergleichen",
    around: (metres) => `Im Umkreis · ${metres} m`,
    aroundWaiting: "Komm in 3D näher, um zu sehen, was hier steht.",
    aroundPreview: (plugin) => `Ohne die Objekte von ${plugin} hier ansehen, ohne zu entscheiden`,
    aroundPreviewShort: "Vorschau",
    hiddenByItself: "Vom Patch einzeln ausgeblendet.",
    hiddenByDecision:
      "Vom Patch ausgeblendet, über die Entscheidung einer Kollision. Wieder zeigen behält an dieser Stelle beide Objekte.",
    hideHint:
      "Nur dieses Objekt verschwindet; der Patch blendet es aus, während das Spiel läuft, und ändert keine Datei.",
    partsAlone: "In seiner Reichweite liegt kein anderes Teil dieser Mod.",
    partsTogether: (count) =>
      `Über die Reichweite mit anderen Teilen derselben Mod verbunden: ${count} insgesamt`,
    partsReach: (centimetres) => `Jedes Teil reicht ${length(centimetres, ",")} über sich hinaus`,
    partsHulls: "Reichweite jedes Teils zeigen",
    partsShow: (count) => `Alle ${count} wieder zeigen`,
    partsHide: (count) => `Alle ${count} ausblenden`,
    partsSomeTied: (count) => `${count} davon braucht das Spiel; die bleiben. `,
    partsHint:
      "Die Kette geht von Teil zu Teil dieser Mod, solange jedes in der Reichweite eines vorigen liegt; Teile anderer Mods zählen nicht mit. Mehr Reichweite: auch Teile mit etwas Abstand, etwa die Stützen unter einem Dach.",
    aroundCareful: (count) => `davon ${count} mit Vorsicht`,
    aroundCarefulHint:
      "Das Spiel könnte sie vermissen: Sie werden aus der Ferne gezeigt, oder etwas verweist auf sie. Klick eines in 3D an, um zu sehen, warum.",
    aroundHide: (plugin, count) =>
      count === 1
        ? `1 Objekt von ${plugin} ausblenden`
        : `${count} Objekte von ${plugin} ausblenden`,
  },
};
