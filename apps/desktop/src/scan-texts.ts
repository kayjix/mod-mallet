// The words of a scan's progress, and the small jokes of the game box and
// the page of all games. Kept apart from `i18n.ts`.

import type { ScanProgress } from "./generated/scan-progress";
import type { Lang } from "./i18n";

type Step = ScanProgress["step"];

export interface ScanTexts {
  /** What each step reads, short and friendly. */
  step: Record<Step, string>;
  /** "37 of 398". */
  count: (done: number, of: number) => string;
  /** "Step 3 of 8". */
  stepOf: (number: number, steps: number) => string;
  /** Above the name of the game in use: "Hammering on" + the game. */
  hammeringOn: string;
  /** A line under the name of a game that Mod Mallet can look at. */
  quips: Record<string, string>;
}

export const scanTexts: Record<Lang, ScanTexts> = {
  en: {
    step: {
      library: "Reading the game library",
      archives: "Reading archives",
      meshes: "Reading meshes",
      plugins: "Reading plugins",
      mods: "Reading your mods",
      models: "Measuring models",
      checking: "Checking collisions",
      saving: "Writing it all down",
    },
    count: (done, of) => `${done} of ${of}`,
    stepOf: (number, steps) => `Step ${number} of ${steps}`,
    hammeringOn: "Hammering on",
    quips: {
      skyrimse: "the mallet feels at home",
      skyrim: "the mallet is warmed up now",
      skyrimvr: "the mallet gets both hands",
      fallout4: "out of the forge, into the Commonwealth",
      fallout4vr: "same Commonwealth, bigger swing",
      falloutnv: "the mallet bets on red",
      fallout3: "the mallet climbs out of the vault",
      oblivion: "the gates are shut, the mallet stays",
    },
  },
  de: {
    step: {
      library: "Lese die Spielbibliothek",
      archives: "Lese Archive",
      meshes: "Lese 3D-Modelle",
      plugins: "Lese Plugins",
      mods: "Lese deine Mods",
      models: "Vermesse Modelle",
      checking: "Prüfe Kollisionen",
      saving: "Schreibe alles auf",
    },
    count: (done, of) => `${done} von ${of}`,
    stepOf: (number, steps) => `Schritt ${number} von ${steps}`,
    hammeringOn: "Hämmert sich durch",
    quips: {
      skyrimse: "hier sitzt jeder Schlag",
      skyrim: "der Hammer ist jetzt warm",
      skyrimvr: "der Hammer hat jetzt beide Hände",
      fallout4: "raus aus der Schmiede, rein ins Commonwealth",
      fallout4vr: "gleiches Commonwealth, größerer Schwung",
      falloutnv: "der Hammer setzt auf Rot",
      fallout3: "der Hammer klettert aus dem Vault",
      oblivion: "die Tore sind zu, der Hammer bleibt",
    },
  },
};
