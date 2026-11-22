// The texts of the settings page, kept apart from i18n.ts like the other
// pages' texts (ADR 0077).

import type { Lang } from "./i18n";

export interface SettingsTexts {
  openLog: string;
  tagline: string;
  settingsTabGeneral: string;
  settingsTabPlaces: string;
  settingsTabAbout: string;
  settingsTheme: string;
  themeSystem: string;
  themeLight: string;
  themeDark: string;
  aboutText: string;
  /** The app's version, and which build of it this is (ADR 0057). */
  aboutVersion: (version: string, commit: string | null, date: string | null) => string;
  /** Which graphics card draws the 3D views, and whether it is one at all. */
  aboutGraphics: (renderer: string | null, software: boolean) => string;
  settingsLanguage: string;
  settingsLive: string;
  settingsLiveLabel: string;
  settingsPlaces: string;
  settingsLog: string;
  settingsLogNote: string;
}

export const settingsTexts: Record<Lang, SettingsTexts> = {
  en: {
    openLog: "Open log",
    tagline: "Hammering on",
    settingsTabGeneral: "General",
    settingsTabPlaces: "Places",
    settingsTabAbout: "About",
    settingsTheme: "Appearance",
    themeSystem: "Like Windows",
    themeLight: "Light",
    themeDark: "Dark",
    aboutText:
      "Mod Mallet looks at a modded game, explains what is wrong in plain words and lets the mod manager carry out the fixes. It changes nothing by itself.",
    aboutVersion: (version, commit, date) =>
      commit === null
        ? `Version ${version}, development build`
        : `Version ${version}, build ${commit}${date === null ? "" : ` of ${date}`}`,
    aboutGraphics: (renderer, software) =>
      renderer === null
        ? "3D views: no graphics card found to draw them"
        : software
          ? `3D views drawn without the graphics card, slowly: ${renderer}`
          : `3D views drawn by: ${renderer}`,
    settingsLanguage: "Language",
    settingsLive: "Live",
    settingsLiveLabel: "Scan again by myself when the setup changes",
    settingsPlaces: "Where Mod Mallet reads",
    settingsLog: "Log",
    settingsLogNote:
      "What Mod Mallet did, line by line. Useful when something did not work, for you or for whoever helps you.",
  },
  de: {
    openLog: "Protokoll öffnen",
    tagline: "Hammering on",
    settingsTabGeneral: "Allgemein",
    settingsTabPlaces: "Orte",
    settingsTabAbout: "Über",
    settingsTheme: "Darstellung",
    themeSystem: "Wie Windows",
    themeLight: "Hell",
    themeDark: "Dunkel",
    aboutText:
      "Mod Mallet schaut sich ein gemoddetes Spiel an, erklärt verständlich, was nicht stimmt, und lässt den Mod-Manager die Korrekturen ausführen. Selbst ändert es nichts.",
    aboutVersion: (version, commit, date) =>
      commit === null
        ? `Version ${version}, Entwicklungsstand`
        : `Version ${version}, Build ${commit}${date === null ? "" : ` vom ${date}`}`,
    aboutGraphics: (renderer, software) =>
      renderer === null
        ? "3D-Ansichten: keine Grafikkarte gefunden, die sie zeichnet"
        : software
          ? `3D-Ansichten ohne Grafikkarte gezeichnet, langsam: ${renderer}`
          : `3D-Ansichten gezeichnet von: ${renderer}`,
    settingsLanguage: "Sprache",
    settingsLive: "Live",
    settingsLiveLabel: "Selbst neu prüfen, wenn sich das Setup ändert",
    settingsPlaces: "Wo Mod Mallet liest",
    settingsLog: "Protokoll",
    settingsLogNote:
      "Was Mod Mallet getan hat, Zeile für Zeile. Hilfreich, wenn etwas nicht geklappt hat – für dich oder für wen auch immer dir hilft.",
  },
};
