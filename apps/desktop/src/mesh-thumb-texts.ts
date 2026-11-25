// Texts of the small pictures of meshes in lists (ADR 0085): kept apart
// from `i18n.ts`, as every package does.

import { useSession } from "./Actions";
import type { MeshGap } from "./generated/mesh-thumbnail";
import type { Lang } from "./i18n";

export interface MeshThumbTexts {
  /** While the picture is drawn. */
  loading: string;
  /** Why there is no picture, as the slot's tooltip. */
  gaps: Record<MeshGap, string>;
  /** When asking for the picture failed. */
  failed: string;
}

export const meshThumbTexts: Record<Lang, MeshThumbTexts> = {
  en: {
    loading: "Drawing a small picture…",
    gaps: {
      missing: "Not found: the setup has no such version of this file.",
      unreadable: "Cannot be read: the file is damaged or in a format Mod Mallet does not know.",
      skeleton: "Skeleton: bones only, nothing around them to draw. Open it to see the bones.",
      effects: "Only light: glow, mist, flames or sparks, which the game makes as it runs.",
      empty: "Nothing to see: a marker, a camera or an empty frame the game uses but never shows.",
      unsupported: "Its surfaces are of a kind Mod Mallet cannot draw yet.",
    },
    failed: "The picture could not be made.",
  },
  de: {
    loading: "Kleines Bild wird gezeichnet …",
    gaps: {
      missing: "Nicht gefunden: Das Setup hat diese Version der Datei nicht.",
      unreadable:
        "Nicht lesbar: Die Datei ist beschädigt oder in einem Format, das Mod Mallet nicht kennt.",
      skeleton: "Skelett: nur Knochen, nichts darum herum zu zeichnen. Öffnen zeigt die Knochen.",
      effects: "Nur Licht: Schein, Nebel, Flammen oder Funken, die das Spiel erst im Lauf erzeugt.",
      empty:
        "Nichts zu sehen: eine Markierung, eine Kamera oder ein leeres Gerüst, das das Spiel nutzt, aber nie zeigt.",
      unsupported: "Ihre Flächen sind von einer Art, die Mod Mallet noch nicht zeichnen kann.",
    },
    failed: "Das Bild konnte nicht erstellt werden.",
  },
};

export function useMeshThumbTexts(): MeshThumbTexts {
  return meshThumbTexts[useSession().lang];
}
