// Texts of the places where characters do something (ADR 0069): kept apart
// from `i18n.ts`, as every package of that plan does.

import type { ActionPlace } from "./generated/area-objects";
import type { Lang } from "./i18n";

type ActionKind = ActionPlace["kind"];

export interface ActionPlaceTexts {
  /** What a place is for, as a heading: "A place to lean". */
  kind: Record<ActionKind, string>;
  /** What characters stop doing once it is hidden. */
  stop: Record<ActionKind, string>;
  /** The kind of object: furniture that is seen, or a marker that is not. */
  seen: string;
  marker: string;
  idleMarker: string;
  /** The internal name of what is placed. */
  base: (name: string) => string;
  /** The heading of who uses it. */
  usersTitle: string;
  /** A character is linked to it: their routine sends them here. */
  linkedCharacter: (name: string) => string;
  /** Another place is linked to it: a chain of places walked one after the other. */
  linkedPlace: (name: string) => string;
  /** A routine (AI package) names it. */
  routine: (name: string) => string;
  /** A quest needs it. */
  quest: (name: string) => string;
  /** It has a role in a location. */
  locationRole: (name: string, role: string | null) => string;
  /** A script names it. */
  script: (name: string) => string;
  /** Nothing names it: characters find it by its kind. */
  foundByKind: string;
  /** While asking who uses it. */
  asking: string;
  /** Whether characters reach it. */
  reachTitle: string;
  checking: string;
  reached: string;
  reachedMoved: string;
  cutOff: string;
  cutOffMoved: string;
  offPaths: (metres: number | null) => string;
  offPathsMoved: (metres: number | null) => string;
  unknown: string;
  /** That the hidden objects around count. */
  withHidden: (count: number) => string;
  /** What hiding it does. */
  hideNote: (kind: ActionKind) => string;
  /** The toggle of the icons in the 3D views. */
  toggle: string;
  toggleHint: string;
  /** After hiding: places that stood on what is hidden. */
  followTitle: (count: number) => string;
  followHint: string;
  moveToGround: string;
  moveAllToGround: (count: number) => string;
  hideToo: string;
  hideAllToo: (count: number) => string;
  noGround: string;
  hiddenToo: string;
  lockedByQuest: string;
  facingNote: string;
  finding: string;
}

const en: ActionPlaceTexts = {
  kind: {
    sit: "A place to sit",
    lean: "A place to lean",
    sleep: "A place to sleep",
    "chop-wood": "A block to chop wood",
    mine: "A place to mine ore",
    smith: "A place to smith or tan",
    craft: "A table to mix potions or enchant",
    cook: "A place to cook",
    pray: "A place to pray",
    work: "A place to work",
    stand: "A spot to stand and do something",
    other: "Furniture characters use",
  },
  stop: {
    sit: "Characters stop sitting here.",
    lean: "Characters stop leaning here.",
    sleep: "Characters stop sleeping here.",
    "chop-wood": "Characters stop chopping wood here.",
    mine: "Characters stop mining here.",
    smith: "Characters stop smithing or tanning here.",
    craft: "Characters stop working at this table.",
    cook: "Characters stop cooking here.",
    pray: "Characters stop praying here.",
    work: "Characters stop working here.",
    stand: "Characters stop standing here to do what this spot is for.",
    other: "Characters stop using this.",
  },
  seen: "furniture you can see",
  marker: "an invisible marker",
  idleMarker: "an invisible spot for characters",
  base: (name) => `Placed as “${name}”.`,
  usersTitle: "Who uses it",
  linkedCharacter: (name) =>
    `The character “${name}” is linked to it: their routine sends them here.`,
  linkedPlace: (name) =>
    `Linked with “${name}”: characters go from one such place to the next, as on a patrol.`,
  routine: (name) => `The routine “${name}” sends characters here.`,
  quest: (name) => `The quest “${name}” needs it: it cannot be hidden or moved.`,
  locationRole: (name, role) =>
    role ? `It is the “${role}” of the place “${name}”.` : `It has a role in the place “${name}”.`,
  script: (name) => `A script of “${name}” names it.`,
  foundByKind:
    "Nothing names it. Characters whose routine looks for such a place nearby may use it.",
  asking: "Finding out who uses it…",
  reachTitle: "Can characters get there?",
  checking: "Following the paths characters walk on…",
  reached: "Yes: a path characters walk on leads there.",
  reachedMoved: "Reachable at the new place: a path characters walk on leads there.",
  cutOff: "No: it stands on paths that no way leads to from the rest of the world.",
  cutOffMoved: "Not reachable there: it stands on paths no way leads to.",
  offPaths: (metres) =>
    metres === null
      ? "No: no path characters walk on is near it."
      : `No: the nearest path characters walk on is ${metres} m away.`,
  offPathsMoved: (metres) =>
    metres === null
      ? "Not reachable there: no path characters walk on is near."
      : `Not reachable there: the nearest path characters walk on is ${metres} m away.`,
  unknown: "The paths here could not be read.",
  withHidden: (count) =>
    count === 1
      ? "Counted with the 1 object nearby that the patch hides."
      : `Counted with the ${count} objects nearby that the patch hides.`,
  hideNote: (kind) =>
    `If hidden: ${en.stop[kind]} Their routine goes on elsewhere, or they stand idle.`,
  toggle: "Places to sit, lean, work",
  toggleHint:
    "Shows where characters sit, lean, sleep or work, with an icon by what they do there. Many of these spots are invisible in the game. Click one to see who uses it.",
  followTitle: (count) =>
    count === 1
      ? "1 place where characters do something stands on what would be hidden."
      : `${count} places where characters do something stand on what would be hidden.`,
  followHint:
    "Left as they are, they float where the hidden object was and nobody can use them. Move them onto the nearest ground characters reach, or hide them too.",
  moveToGround: "Onto the ground",
  moveAllToGround: (count) => `Move all ${count} onto the ground nearby`,
  hideToo: "Hide too",
  hideAllToo: (count) => `Hide all ${count} too`,
  noGround: "no ground characters reach within 30 m",
  hiddenToo: "hidden too",
  lockedByQuest: "a quest needs it: it stays",
  facingNote:
    "Moved places keep the way they face; check in the 3D view that a place to lean still faces away from a wall.",
  finding: "Looking for the nearest ground characters reach…",
};

const de: ActionPlaceTexts = {
  kind: {
    sit: "Ein Platz zum Sitzen",
    lean: "Ein Platz zum Anlehnen",
    sleep: "Ein Platz zum Schlafen",
    "chop-wood": "Ein Hackklotz zum Holzhacken",
    mine: "Ein Platz zum Erzabbau",
    smith: "Ein Platz zum Schmieden oder Gerben",
    craft: "Ein Tisch für Tränke oder Verzauberungen",
    cook: "Ein Platz zum Kochen",
    pray: "Ein Platz zum Beten",
    work: "Ein Arbeitsplatz",
    stand: "Eine Stelle zum Stehen und Tun",
    other: "Ein Möbel, das Figuren benutzen",
  },
  stop: {
    sit: "Figuren sitzen hier nicht mehr.",
    lean: "Figuren lehnen sich hier nicht mehr an.",
    sleep: "Figuren schlafen hier nicht mehr.",
    "chop-wood": "Figuren hacken hier kein Holz mehr.",
    mine: "Figuren bauen hier kein Erz mehr ab.",
    smith: "Figuren arbeiten hier nicht mehr.",
    craft: "Figuren arbeiten nicht mehr an diesem Tisch.",
    cook: "Figuren kochen hier nicht mehr.",
    pray: "Figuren beten hier nicht mehr.",
    work: "Figuren arbeiten hier nicht mehr.",
    stand: "Figuren stehen hier nicht mehr, um zu tun, wofür die Stelle da ist.",
    other: "Figuren benutzen das nicht mehr.",
  },
  seen: "ein sichtbares Möbel",
  marker: "eine unsichtbare Markierung",
  idleMarker: "eine unsichtbare Stelle für Figuren",
  base: (name) => `Platziert als „${name}“.`,
  usersTitle: "Wer es benutzt",
  linkedCharacter: (name) =>
    `Die Figur „${name}“ ist damit verknüpft: ihr Tagesablauf schickt sie hierher.`,
  linkedPlace: (name) =>
    `Verknüpft mit „${name}“: Figuren gehen von einem solchen Platz zum nächsten, wie auf einer Runde.`,
  routine: (name) => `Der Tagesablauf „${name}“ schickt Figuren hierher.`,
  quest: (name) =>
    `Die Quest „${name}“ braucht es: es kann weder versteckt noch verschoben werden.`,
  locationRole: (name, role) =>
    role ? `Es ist „${role}“ des Ortes „${name}“.` : `Es hat eine Rolle im Ort „${name}“.`,
  script: (name) => `Ein Skript von „${name}“ nennt es.`,
  foundByKind:
    "Nichts nennt es. Figuren, deren Tagesablauf in der Nähe einen solchen Platz sucht, können es benutzen.",
  asking: "Es wird geschaut, wer es benutzt …",
  reachTitle: "Kommen Figuren dorthin?",
  checking: "Die Wege der Figuren werden verfolgt …",
  reached: "Ja: ein Weg, auf dem Figuren gehen, führt dorthin.",
  reachedMoved: "An der neuen Stelle erreichbar: ein Weg, auf dem Figuren gehen, führt dorthin.",
  cutOff: "Nein: es steht auf Wegen, zu denen vom Rest der Welt kein Weg führt.",
  cutOffMoved: "Dort nicht erreichbar: es steht auf Wegen, zu denen kein Weg führt.",
  offPaths: (metres) =>
    metres === null
      ? "Nein: kein Weg, auf dem Figuren gehen, ist in der Nähe."
      : `Nein: der nächste Weg, auf dem Figuren gehen, ist ${metres} m entfernt.`,
  offPathsMoved: (metres) =>
    metres === null
      ? "Dort nicht erreichbar: kein Weg, auf dem Figuren gehen, ist in der Nähe."
      : `Dort nicht erreichbar: der nächste Weg, auf dem Figuren gehen, ist ${metres} m entfernt.`,
  unknown: "Die Wege hier ließen sich nicht lesen.",
  withHidden: (count) =>
    count === 1
      ? "Gerechnet mit dem 1 Objekt in der Nähe, das der Patch versteckt."
      : `Gerechnet mit den ${count} Objekten in der Nähe, die der Patch versteckt.`,
  hideNote: (kind) =>
    `Wenn versteckt: ${de.stop[kind]} Ihr Tagesablauf geht anderswo weiter, oder sie stehen herum.`,
  toggle: "Plätze zum Sitzen, Lehnen, Arbeiten",
  toggleHint:
    "Zeigt, wo Figuren sitzen, sich anlehnen, schlafen oder arbeiten, mit einem Symbol für das, was sie dort tun. Viele dieser Stellen sind im Spiel unsichtbar. Ein Klick zeigt, wer sie benutzt.",
  followTitle: (count) =>
    count === 1
      ? "1 Platz, an dem Figuren etwas tun, steht auf dem, was versteckt würde."
      : `${count} Plätze, an denen Figuren etwas tun, stehen auf dem, was versteckt würde.`,
  followHint:
    "So belassen, schweben sie dort, wo das versteckte Objekt war, und niemand kann sie benutzen. Verschiebe sie auf den nächsten Boden, zu dem Figuren kommen, oder verstecke sie auch.",
  moveToGround: "Auf den Boden",
  moveAllToGround: (count) => `Alle ${count} auf den Boden daneben verschieben`,
  hideToo: "Auch verstecken",
  hideAllToo: (count) => `Alle ${count} auch verstecken`,
  noGround: "kein erreichbarer Boden innerhalb von 30 m",
  hiddenToo: "auch versteckt",
  lockedByQuest: "eine Quest braucht es: es bleibt",
  facingNote:
    "Verschobene Plätze behalten ihre Blickrichtung; schau in der 3D-Ansicht nach, ob ein Platz zum Anlehnen noch von der Wand weg zeigt.",
  finding: "Der nächste Boden, zu dem Figuren kommen, wird gesucht …",
};

export const actionPlaceTexts: Record<Lang, ActionPlaceTexts> = { en, de };
