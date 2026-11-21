// The texts about games drawn with Unreal Engine (Oblivion Remastered,
// ADR 0065), in every language of the app.

import type { Lang } from "./i18n";

/** A page that does not apply to a game drawn with Unreal Engine. */
export type NotDrawnPage = "collisions" | "assets" | "catalogue";

/** What is said about the Unreal Engine side of a game. */
export interface UnrealTexts {
  /** Why a page shows nothing for this game, one sentence each. */
  notDrawn: Record<NotDrawnPage, string>;
  /** Why, for all of them. */
  why: string;
  paksTitle: string;
  paksNote: string;
  folder: Record<"mods" | "logic-mods", string>;
  noMod: string;
  size: (bytes: number) => string;
}

const megabytes = (bytes: number, lang: Lang) =>
  (bytes / (1024 * 1024)).toLocaleString(lang, { maximumFractionDigits: bytes < 1048576 ? 2 : 1 });

const en: UnrealTexts = {
  notDrawn: {
    collisions:
      "Collisions are checked with the shapes of the objects, and this game does not use those shapes.",
    assets:
      "Comparing meshes and textures does not help in this game: what you see comes from other files.",
    catalogue:
      "The catalogue shows the objects in 3D with their old shapes, and this game draws them differently.",
  },
  why: "Oblivion Remastered runs the old game underneath, but draws everything with Unreal Engine from files of its own. The meshes and textures in its Data folder are not what you see.",
  paksTitle: "Unreal mods",
  paksNote:
    "Mods for the Unreal Engine side of the game: packages in ~mods change pictures, models and menus; packages in LogicMods add game logic through UE4SS.",
  folder: { mods: "~mods", "logic-mods": "LogicMods" },
  noMod: "not known to Vortex",
  size: (bytes) => `${megabytes(bytes, "en")} MB`,
};

const de: UnrealTexts = {
  notDrawn: {
    collisions:
      "Kollisionen werden mit den Formen der Objekte geprüft, und dieses Spiel benutzt diese Formen nicht.",
    assets:
      "Meshes und Texturen zu vergleichen hilft in diesem Spiel nicht: Was du siehst, kommt aus anderen Dateien.",
    catalogue:
      "Der Katalog zeigt die Objekte in 3D mit ihren alten Formen, und dieses Spiel zeichnet sie anders.",
  },
  why: "Oblivion Remastered lässt darunter das alte Spiel laufen, zeichnet aber alles mit der Unreal Engine aus eigenen Dateien. Die Meshes und Texturen in seinem Data-Ordner sind nicht das, was du siehst.",
  paksTitle: "Unreal-Mods",
  paksNote:
    "Mods für die Unreal-Engine-Seite des Spiels: Pakete in ~mods ändern Bilder, Modelle und Menüs; Pakete in LogicMods fügen über UE4SS Spiellogik hinzu.",
  folder: { mods: "~mods", "logic-mods": "LogicMods" },
  noMod: "Vortex unbekannt",
  size: (bytes) => `${megabytes(bytes, "de")} MB`,
};

export const unrealTexts: Record<Lang, UnrealTexts> = { en, de };
