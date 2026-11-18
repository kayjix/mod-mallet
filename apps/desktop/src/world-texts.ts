// The texts of the map's world picker, kept apart from `i18n.ts`.

import type { Lang } from "./i18n";

export interface WorldTexts {
  /** The picker's groups, with how many worlds each holds. */
  group: Record<"game" | "creationClub" | "mods" | "small", (count: number) => string>;
  /** The mod or Creation Club content a world comes with. */
  from: (mod: string) => string;
}

export const worldTexts: Record<Lang, WorldTexts> = {
  en: {
    group: {
      game: () => "The game",
      creationClub: () => "Creation Club",
      mods: () => "Mods",
      small: (count) => `Small worlds (${count})`,
    },
    from: (mod) => `from ${mod}`,
  },
  de: {
    group: {
      game: () => "Das Spiel",
      creationClub: () => "Creation Club",
      mods: () => "Mods",
      small: (count) => `Kleine Welten (${count})`,
    },
    from: (mod) => `aus ${mod}`,
  },
};
