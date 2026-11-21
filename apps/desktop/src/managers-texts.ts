// The texts of the choice of mod manager on the places page (ADR 0064),
// kept apart from `i18n.ts` so that the choice can be changed on its own.

import type { Lang } from "./i18n";

export interface ManagerTexts {
  /** The heading of the choice. */
  heading: string;
  /** What the choice decides. */
  intro: string;
  /** Beside a manager that is on this computer. */
  found: string;
  /** Beside a manager that is not. */
  notFound: string;
  /** Beside the manager suggested because it was found. */
  suggested: string;
  /** An instance of Mod Organizer, by its folder's name. */
  instance: (name: string) => string;
  /** Beside an instance kept beside its program. */
  portable: string;
  /** Mod Organizer without a found instance: where to name one. */
  noInstance: string;
  /** Why fixes are described rather than made for a manager without a link. */
  byHand: string;
}

export const managerTexts: Record<Lang, ManagerTexts> = {
  en: {
    heading: "Your mod manager",
    intro:
      "Mod Mallet reads the mods where your mod manager keeps them. Pick the one you use for this game.",
    found: "on this computer",
    notFound: "not found on this computer",
    suggested: "suggested",
    instance: (name) => `instance "${name}"`,
    portable: "portable",
    noInstance:
      "No instance was found in the usual place. If yours is a portable one, enter its folder (the one with ModOrganizer.ini) below.",
    byHand:
      "Mod Organizer takes no requests from other programs. Where Mod Mallet would fix something in Vortex, it tells you what to click in Mod Organizer instead. It never changes Mod Organizer's files.",
  },
  de: {
    heading: "Dein Mod-Manager",
    intro:
      "Mod Mallet liest die Mods dort, wo dein Mod-Manager sie aufbewahrt. Wähle den, den du für dieses Spiel nimmst.",
    found: "auf diesem Computer",
    notFound: "auf diesem Computer nicht gefunden",
    suggested: "vorgeschlagen",
    instance: (name) => `Instanz „${name}“`,
    portable: "portabel",
    noInstance:
      "Am üblichen Ort wurde keine Instanz gefunden. Ist deine portabel, trag unten ihren Ordner ein (den mit ModOrganizer.ini).",
    byHand:
      "Mod Organizer nimmt keine Anfragen von anderen Programmen an. Wo Mod Mallet in Vortex etwas beheben würde, sagt es dir stattdessen, was du in Mod Organizer anklicken musst. Mod Organizers Dateien ändert es nie.",
  },
};
