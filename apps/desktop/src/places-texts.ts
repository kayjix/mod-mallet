// The texts of the places' fields that are not the places themselves: the
// button that opens a dialog for a place (ADR 0018).

import type { Lang } from "./i18n";
import type { PlaceKind } from "./place-kinds";

export interface PlacesTexts {
  /** The button beside a place that opens the dialog. */
  browse: string;
  /** Its tooltip, by whether the place is a file or a folder. */
  browseHint: Record<PlaceKind, string>;
  /** When the dialog could not be opened. */
  browseFailed: (reason: string) => string;
}

export const placesTexts: Record<Lang, PlacesTexts> = {
  en: {
    browse: "Browse…",
    browseHint: {
      file: "Pick the file on your computer instead of typing its path",
      folder: "Pick the folder on your computer instead of typing its path",
    },
    browseFailed: (reason) => `The window to pick a place did not open: ${reason}`,
  },
  de: {
    browse: "Durchsuchen…",
    browseHint: {
      file: "Die Datei auf deinem Computer auswählen, statt den Pfad einzutippen",
      folder: "Den Ordner auf deinem Computer auswählen, statt den Pfad einzutippen",
    },
    browseFailed: (reason) => `Das Fenster zum Auswählen ging nicht auf: ${reason}`,
  },
};
