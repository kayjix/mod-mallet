// Texts of the paths mode of the 3D views (ADR 0071).

import type { Lang } from "./i18n";

export interface PathsModeTexts {
  /** The switch in the bar over the view. */
  toggle: string;
  toggleHint: string;
  /** The switch for the places where characters sit, lean, sleep or work. */
  actions: string;
  title: string;
  reading: string;
  failed: string;
  /** Where the view is too far away to read paths. */
  tooFar: string;
  nothing: string;
  game: string;
  mods: string;
  repaired: string;
  takenOut: string;
  overNothing: string;
  /** A number of triangles. */
  pieces: (count: number) => string;
  /** A mod whose paths are all replaced by a later one here. */
  replaced: string;
  /** Says how to see one mod's paths alone. */
  modsHint: string;
  doors: string;
  links: string;
  kinds: { sit: string; lean: string; sleep: string; work: string; other: string };
  problemsTitle: (count: number) => string;
  /** One spot where a choice leaves paths over nothing. */
  problem: (names: string, pieces: number) => string;
  /** The same, when the repair of all its paths is chosen. */
  problemRepairChosen: string;
  open: string;
  /** Objects of choices here that are not part of a collision listed. */
  otherObjects: (count: number) => string;
}

export const pathsModeTexts: Record<Lang, PathsModeTexts> = {
  en: {
    toggle: "Paths",
    toggleHint:
      "Shows where characters walk: the game's paths, those of mods, the repairs of Mod Mallet, and where your choices leave paths over nothing.",
    actions: "Sit, lean, sleep, work",
    title: "Where characters walk",
    reading: "Reading the paths…",
    failed: "The paths here could not be read.",
    tooFar: "Come closer to see the paths.",
    nothing: "No paths here.",
    game: "The game's paths",
    mods: "Paths added or changed by mods",
    repaired: "Repaired by Mod Mallet",
    takenOut: "Taken out by the repair",
    overNothing: "Over nothing with your choices",
    pieces: (count) => `${count.toLocaleString("en")} ${count === 1 ? "piece" : "pieces"}`,
    replaced: "replaced by a later mod here",
    modsHint: "Point at a mod to see its paths alone; click to keep them lit.",
    doors: "Through a door",
    links: "On into the next piece",
    kinds: {
      sit: "Sit",
      lean: "Lean",
      sleep: "Sleep",
      work: "Work",
      other: "Stand or something else",
    },
    problemsTitle: (count) =>
      count === 1 ? "1 place where paths break" : `${count} places where paths break`,
    problem: (names, pieces) =>
      `${names}: ${pieces.toLocaleString("en")} ${pieces === 1 ? "piece" : "pieces"} of path over nothing`,
    problemRepairChosen: "repair chosen",
    open: "Open",
    otherObjects: (count) =>
      count === 1
        ? "1 more object you chose to hide carries paths."
        : `${count} more objects you chose to hide carry paths.`,
  },
  de: {
    toggle: "Wege",
    toggleHint:
      "Zeigt, wo Figuren laufen: die Wege des Spiels, die von Mods, die Reparaturen von Mod Mallet und wo deine Entscheidungen Wege über dem Nichts lassen.",
    actions: "Sitzen, lehnen, schlafen, arbeiten",
    title: "Wo Figuren laufen",
    reading: "Wege werden gelesen …",
    failed: "Die Wege hier konnten nicht gelesen werden.",
    tooFar: "Komm näher heran, um die Wege zu sehen.",
    nothing: "Hier gibt es keine Wege.",
    game: "Wege des Spiels",
    mods: "Von Mods hinzugefügte oder geänderte Wege",
    repaired: "Von Mod Mallet repariert",
    takenOut: "Von der Reparatur herausgenommen",
    overNothing: "Mit deinen Entscheidungen über dem Nichts",
    pieces: (count) => `${count.toLocaleString("de")} ${count === 1 ? "Stück" : "Stücke"}`,
    replaced: "hier von einem späteren Mod ersetzt",
    modsHint:
      "Zeig auf einen Mod, um nur seine Wege zu sehen; klick, damit sie hervorgehoben bleiben.",
    doors: "Durch eine Tür",
    links: "Weiter ins nächste Stück",
    kinds: {
      sit: "Sitzen",
      lean: "Lehnen",
      sleep: "Schlafen",
      work: "Arbeiten",
      other: "Stehen oder anderes",
    },
    problemsTitle: (count) =>
      count === 1 ? "1 Stelle, an der Wege brechen" : `${count} Stellen, an denen Wege brechen`,
    problem: (names, pieces) =>
      `${names}: ${pieces.toLocaleString("de")} ${pieces === 1 ? "Stück" : "Stücke"} Weg über dem Nichts`,
    problemRepairChosen: "Reparatur gewählt",
    open: "Öffnen",
    otherObjects: (count) =>
      count === 1
        ? "1 weiteres Objekt, das du ausblenden willst, trägt Wege."
        : `${count} weitere Objekte, die du ausblenden willst, tragen Wege.`,
  },
};
