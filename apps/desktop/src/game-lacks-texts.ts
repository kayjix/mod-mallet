// What the window says where a part of Mod Mallet does not apply to the
// game looked at (ADR 0078), in every language of the app.

import type { GameLack } from "./generated/rendered-report";
import type { Lang } from "./i18n";

/** One sentence per part that does not apply, naming the game. */
export type GameLackTexts = Record<GameLack, (game: string) => string>;

export const gameLackTexts: Record<Lang, GameLackTexts> = {
  en: {
    catalogue: (game) =>
      `The catalogue knows the weapons, armour and clothes of the Skyrim games only. It cannot list those of ${game} yet.`,
    patches: (game) =>
      `Mod Mallet's patches need Base Object Swapper, a small helper that exists only for Skyrim. ${game} has none, so no patch can be built for it.`,
    "written-plugins": (game) =>
      `Mod Mallet writes plugins only for Skyrim Special Edition and Enderal so far, not for ${game}.`,
    walkways: (game) =>
      `${game} keeps the paths characters walk in a form Mod Mallet cannot read yet, so they cannot be shown here.`,
  },
  de: {
    catalogue: (game) =>
      `Der Katalog kennt bisher nur die Waffen, Rüstungen und Kleidung der Skyrim-Spiele. Die von ${game} kann er noch nicht zeigen.`,
    patches: (game) =>
      `Die Patches von Mod Mallet brauchen Base Object Swapper, ein kleines Hilfsprogramm, das es nur für Skyrim gibt. ${game} hat keins, deshalb lässt sich dafür kein Patch bauen.`,
    "written-plugins": (game) =>
      `Mod Mallet schreibt bisher nur Plugins für Skyrim Special Edition und Enderal, nicht für ${game}.`,
    walkways: (game) =>
      `${game} speichert die Wege, auf denen Figuren laufen, in einer Form, die Mod Mallet noch nicht lesen kann. Deshalb können sie hier nicht gezeigt werden.`,
  },
};
