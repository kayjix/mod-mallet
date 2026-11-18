// The texts of the view of an interior cell (ADR 0042), kept apart from
// `i18n.ts` so that the view can be changed on its own.

import type { Lang } from "./i18n";

export interface InteriorTexts {
  /** While the cell is read. */
  loading: (cell: string) => string;
  /** When the cell cannot be read or has nothing to draw. */
  missing: (cell: string) => string;
  /** How many objects are drawn, and how many models are still to come. */
  count: (objects: number, done: number, total: number) => string;
  /** The switch that leaves out walls and ceilings seen from behind. */
  seeInside: string;
  seeInsideHint: string;
  /** How to move the camera. */
  hint: string;
  /** Back to the point the view was opened on. */
  fit: string;
}

export const interiorTexts: Record<Lang, InteriorTexts> = {
  en: {
    loading: (cell) => `Reading ${cell} …`,
    missing: (cell) =>
      `${cell} cannot be shown: no active plugin has this cell, or nothing in it can be drawn.`,
    count: (objects, done, total) =>
      done < total ? `${objects} objects · ${done} of ${total} models` : `${objects} objects`,
    seeInside: "See into rooms",
    seeInsideHint:
      "Walls and ceilings seen from behind are left out, so that you can look in from outside.",
    hint: "Drag to move · right-drag to turn · wheel to zoom",
    fit: "Back to the spot",
  },
  de: {
    loading: (cell) => `${cell} wird gelesen …`,
    missing: (cell) =>
      `${cell} lässt sich nicht zeigen: Kein aktives Plugin hat diese Zelle, oder darin ist nichts zu zeichnen.`,
    count: (objects, done, total) =>
      done < total ? `${objects} Objekte · ${done} von ${total} Modellen` : `${objects} Objekte`,
    seeInside: "In Räume hineinsehen",
    seeInsideHint:
      "Wände und Decken, die man von hinten sieht, werden weggelassen, damit man von außen hineinschauen kann.",
    hint: "Ziehen verschiebt · rechts ziehen dreht · Mausrad zoomt",
    fit: "Zurück zur Stelle",
  },
};
