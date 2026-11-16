// Texts of the app's own controls.
//
// Everything that describes a finding comes from the Rust side already
// translated (crates/mallet-messages), so that the app and the command line
// always say the same thing. Only the words of the window itself live here.

export type Lang = "en" | "de";

export const languages: Record<Lang, string> = {
  en: "English",
  de: "Deutsch",
};

/** What a scan looked at, in numbers. */
export interface ScanStats {
  plugins: number;
  archives: number;
  installers: number;
  meshes: number;
}

export interface Texts {
  scan: string;
  scanning: string;
  scanningNote: string;
  language: string;
  readOnlyNote: string;
  noScanYet: string;
  scanFailed: string;
  versionUnknown: string;
  scriptExtender: string;

  // The views of the sidebar.
  viewTasks: string;
  viewMap: string;
  viewMods: string;
  viewPlaces: string;
  viewCollisions: string;
  viewPatch: string;
  patchNeedsSwapper: string;
  patchCount: (count: number) => string;
  patchInstall: string;
  patchInstallHint: string;
  patchHide: (plugin: string, count: number) => string;
  patchKeepBoth: string;
  patchSuggest: string;
  patchClear: string;
  patchCreate: string;
  patchCreated: string;
  patchStale: string;
  patchSuggested: string;
  patchInstalled: (count: number, mod: string | null) => string;
  patchMatches: string;
  patchDiffers: (more: number, fewer: number) => string;
  patchObsolete: (count: number) => string;
  patchUseInstalled: string;
  patchInPatch: string;
  patchPartly: (hidden: number, of: number) => string;
  patchNew: string;
  openFolder: string;
  openLog: string;
  tagline: string;
  managerRunning: string;
  managerOffline: string;
  managerMissing: string;
  managerLinkReady: (version: string | null) => string;
  managerLinkRestart: string;
  managerLinkOutdated: string;
  managerLinkMissing: string;
  gameLabel: string;
  /** The games of the game menu; only the first can be chosen so far. */
  games: { id: string; name: string; quip?: string }[];
  viewHome: string;
  homeGames: string;
  homeActive: string;
  homeNoArt: string;
  homeOpenTasks: string;
  homeCollisions: string;
  homeMods: string;
  homePatch: string;
  homePatchNone: string;
  homeToTasks: string;
  settingsTabGeneral: string;
  settingsTabPlaces: string;
  settingsTabAbout: string;
  settingsLanguageNote: string;
  settingsTheme: string;
  settingsThemeNote: string;
  themeSystem: string;
  themeLight: string;
  themeDark: string;
  aboutText: string;
  soon: string;
  viewToolbox: string;
  viewSettings: string;
  toolboxReady: string;
  toolboxAttention: (count: number) => string;
  restartVortex: string;
  startVortex: string;
  restartingVortex: string;
  restartedVortex: string;
  restartHint: string;
  updateLink: string;
  getIt: string;
  settingsLanguage: string;
  settingsLive: string;
  settingsLiveLabel: string;
  settingsPlaces: string;
  settingsLog: string;
  settingsLogNote: string;
  collisionsHint: string;

  // Watching the setup.
  live: string;
  liveNote: string;
  changedRescanning: string;

  // The overview at the top of the task list.
  allGood: string;
  allGoodNote: string;
  headline: (errors: number, warnings: number) => string;
  lastScan: string;
  scanStats: (stats: ScanStats) => string;

  // The filters of the task list, which are also its sections.
  filterOpen: string;
  filterFix: string;
  filterCheck: string;
  filterDirect: string;
  filterNotes: string;
  filterIgnored: string;
  fixHint: string;
  checkHint: string;
  directSectionHint: string;
  notesHint: string;
  ignoredHint: string;
  nothingHere: string;

  // One finding, opened.
  whatToDo: string;
  choicesTitle: string;
  choiceRun: string;
  choiceLeave: string;
  choiceLeaveNote: string;
  choiceBringBackNote: string;
  choiceHide: string;
  choiceHideNote: (chosen: string | null) => string;
  choiceToPatch: string;
  choiceByHand: string;
  moreAbout: string;
  whatItMeans: string;
  file: string;
  mod: string;
  rescan: string;
  installLink: string;
  guideTitle: string;
  ignore: string;
  unignore: string;
  spotsTitle: (shown: number, total: number) => string;
  consoleHint: string;
  copy: string;
  copied: string;
  showOnMap: string;

  // A mod.
  version: string;
  newerVersion: (version: string) => string;
  author: string;
  openPage: string;
  disabled: string;

  // The link to the mod manager.
  linkReady: string;
  linkRestart: string;
  linkNotRunning: string;

  // The map.
  mapWorld: string;
  mapMod: string;
  mapAllMods: string;
  mapLegendOne: string;
  mapLegendTwo: string;
  mapLegendThree: string;
  mapLegendMany: string;
  mapNoMods: string;
  mapLegendSpot: string;
  mapCell: (x: number, y: number) => string;
  mapObjects: (count: number) => string;
  mapZoomIn: string;
  mapZoomOut: string;
  mapFit: string;
  mapEmpty: string;
  mapCollisions: string;
  mapCells: string;
  mapShared: string;

  // The mods.
  modsOnlyProblems: string;
  modsSearch: string;
  modsProblems: (count: number) => string;
  modsNone: string;

  // The places a scan reads.
  places: string;
  placesTitleFirst: string;
  placesTitle: string;
  placesIntro: string;
  placesConfirmFirst: string;
  placesSave: string;
  cancel: string;
  notFound: string;
  placeUnknown: string;
  placeAutomatic: string;
  whatWasRead: string;
}

const plural = (count: number, one: string, other: string) => (count === 1 ? one : other);

export const texts: Record<Lang, Texts> = {
  en: {
    scan: "Scan",
    scanning: "Scanning…",
    scanningNote: "Reading your setup. This takes a few seconds.",
    language: "Language",
    readOnlyNote: "A scan only reads. Nothing in your game or your mods is changed.",
    noScanYet: "No scan yet. Start one to see what Mod Mallet finds.",
    scanFailed: "The scan could not be completed",
    versionUnknown: "version unknown",
    scriptExtender: "SKSE",

    viewTasks: "Tasks",
    viewMap: "Map",
    viewMods: "Mods",
    viewPlaces: "Places",
    viewCollisions: "Collisions",
    viewPatch: "Patch",
    patchNeedsSwapper:
      "A patch hides objects with Base Object Swapper, a script extender plugin that is not installed.",
    patchCount: (count) => (count === 1 ? "1 object chosen" : `${count} objects chosen`),
    patchInstall: "Install the patch in Vortex",
    patchInstallHint:
      "Mod Mallet writes a small mod that shows an invisible marker in place of these objects while the game runs. Remove the mod in Vortex to bring them back.",
    patchHide: (plugin, count) => `Hide from ${plugin} (${count})`,
    patchKeepBoth: "Keep both",
    patchSuggest: "Use suggestions",
    patchClear: "Clear",
    patchCreate: "Create patch",
    patchCreated: "Created:",
    patchStale: "The choice changed since. Create the patch again before installing it.",
    patchSuggested: "suggested",
    patchInstalled: (count, mod) =>
      `Installed: ${count === 1 ? "1 object is" : `${count} objects are`} hidden${mod === null ? "" : ` by the mod “${mod}”`}.`,
    patchMatches: "Your choice is what is installed.",
    patchDiffers: (more, fewer) =>
      `Your choice differs from what is installed: ${more} more, ${fewer} fewer. Create the patch and install it again to change that.`,
    patchObsolete: (count) =>
      count === 1
        ? "1 object in the installed patch no longer belongs to a collision. It drops out when you install the patch again."
        : `${count} objects in the installed patch no longer belong to a collision. They drop out when you install the patch again.`,
    patchUseInstalled: "Use what is installed",
    patchInPatch: "in the patch",
    patchPartly: (hidden, of) => `${hidden} of ${of} in the patch`,
    patchNew: "not in the patch yet",
    openFolder: "Open folder",
    openLog: "Open log",
    tagline: "Hammering on",
    managerRunning: "running",
    managerOffline: "offline",
    managerMissing: "not installed",
    managerLinkReady: (version) =>
      version === null ? "Link connected" : `Link ${version} connected`,
    managerLinkRestart: "Restart needed for the new link",
    managerLinkOutdated: "Link update available",
    managerLinkMissing: "Link not installed",
    gameLabel: "Game",
    games: [
      { id: "skyrimse", name: "Skyrim Special Edition" },
      { id: "skyrim", name: "Skyrim (2011)", quip: "the mallet is still warming up" },
      { id: "skyrimvr", name: "Skyrim VR", quip: "the mallet needs both hands" },
      { id: "fallout4", name: "Fallout 4", quip: "still in the forge" },
      { id: "falloutnv", name: "Fallout: New Vegas", quip: "still in the forge, too" },
      { id: "oblivion", name: "Oblivion", quip: "closing a few gates first" },
      { id: "morrowind", name: "Morrowind", quip: "waiting for a silt strider" },
      { id: "starfield", name: "Starfield", quip: "the mallet needs a space suit" },
      { id: "tes6", name: "The Elder Scrolls VI", quip: "as soon as it exists" },
      { id: "baldursgate3", name: "Baldur's Gate 3", quip: "rolling for initiative" },
      { id: "cyberpunk2077", name: "Cyberpunk 2077", quip: "waiting for a patch (yes, we know)" },
    ],
    soon: "soon",
    viewHome: "Home",
    homeGames: "Games",
    homeActive: "Active",
    homeNoArt: "Vortex brings a picture of this game along; it is not installed here.",
    homeOpenTasks: "open tasks",
    homeCollisions: "collisions",
    homeMods: "mods",
    homePatch: "objects hidden by the patch",
    homePatchNone: "no patch installed",
    homeToTasks: "To the tasks",
    settingsTabGeneral: "General",
    settingsTabPlaces: "Places",
    settingsTabAbout: "About",
    settingsLanguageNote: "The language of the window and of every finding.",
    settingsTheme: "Appearance",
    settingsThemeNote: "Light or dark, or like Windows.",
    themeSystem: "Like Windows",
    themeLight: "Light",
    themeDark: "Dark",
    aboutText:
      "Mod Mallet looks at a modded game, explains what is wrong in plain words and lets the mod manager carry out the fixes. It changes nothing by itself.",
    viewToolbox: "Toolbox",
    viewSettings: "Settings",
    toolboxReady: "All set",
    toolboxAttention: (count) => (count === 1 ? "1 thing needs you" : `${count} things need you`),
    restartVortex: "Restart Vortex",
    startVortex: "Start Vortex",
    restartingVortex: "Closing Vortex and starting it again…",
    restartedVortex: "Vortex is starting.",
    restartHint:
      "Closes Vortex the way its window's close button does, and starts it again. Vortex saves as usual first.",
    updateLink: "Update the Vortex extension",
    getIt: "Get it",
    settingsLanguage: "Language",
    settingsLive: "Live",
    settingsLiveLabel: "Scan again by myself when the setup changes",
    settingsPlaces: "Where Mod Mallet reads",
    settingsLog: "Log",
    settingsLogNote:
      "What Mod Mallet did, line by line. Useful when something did not work, for you or for whoever helps you.",
    collisionsHint: "Objects of two mods in the same spot.",

    live: "Live",
    liveNote:
      "Mod Mallet scans again by itself when your setup changes, for example after Vortex deploys.",
    changedRescanning: "Your setup changed. Scanning again…",

    allGood: "Everything is in order",
    allGoodNote: "Mod Mallet found nothing that needs your attention.",
    headline: (errors, warnings) => {
      const fix = `${errors} ${plural(errors, "error", "errors")} to fix`;
      const check = `${warnings} ${plural(warnings, "thing", "things")} to check`;
      if (errors > 0 && warnings > 0) {
        return `${fix}, ${check}`;
      }
      return errors > 0 ? fix : check;
    },
    lastScan: "Last scan",
    scanStats: ({ plugins, archives, installers, meshes }) =>
      `${plugins} plugins · ${archives} archives · ${installers} installers · ${meshes} meshes`,

    filterOpen: "Open",
    filterFix: "To fix",
    filterCheck: "To check",
    filterDirect: "Fixable from here",
    filterNotes: "Good to know",
    filterIgnored: "Set aside",
    fixHint: "Something does not work until this is fixed.",
    checkHint: "Probably a problem. Have a look.",
    directSectionHint: "One click hands these to Vortex. You finish them there.",
    notesHint: "Nothing to do. This only tells you which mod wins where.",
    ignoredHint: "You set these aside. They are not counted above.",
    nothingHere: "Nothing here.",

    whatToDo: "What to do",
    choicesTitle: "Your options",
    choiceRun: "Do it",
    choiceLeave: "Leave it as it is",
    choiceLeaveNote:
      "If it does not bother you in the game. Mod Mallet sets it aside and stops counting it; you find it again under “Set aside”.",
    choiceBringBackNote: "You set this aside. Bring it back to decide about it again.",
    choiceHide: "Hide one of the two",
    choiceHideNote: (chosen) =>
      chosen === null
        ? "Pick whose objects disappear. That goes into Mod Mallet's patch; nothing changes until you install it on the patch page."
        : `The objects of ${chosen} go into Mod Mallet's patch. Install the patch on the patch page to apply it.`,
    choiceToPatch: "To the patch",
    choiceByHand: "By hand",
    moreAbout: "More about it",
    whatItMeans: "What this means",
    file: "File",
    mod: "Mod",
    rescan: "Scan again",
    installLink: "Install the Vortex extension",
    guideTitle: "Tick these in the installer",
    ignore: "Set aside",
    unignore: "Bring back",
    spotsTitle: (shown, total) =>
      shown === total ? `Where (${total})` : `Where (${shown} of ${total})`,
    consoleHint:
      "In the game, open the console with the key left of 1, paste this command and press Enter.",
    copy: "Copy",
    copied: "Copied",
    showOnMap: "Show on the map",

    version: "Version",
    newerVersion: (version) => `version ${version} is available`,
    author: "Author",
    openPage: "Open the mod's page",
    disabled: "disabled",

    linkReady: "Vortex connected",
    linkRestart: "Restart Vortex",
    linkNotRunning: "Vortex is not answering",

    mapWorld: "Worldspace",
    mapMod: "Mod",
    mapAllMods: "All mods",
    mapLegendOne: "1 mod",
    mapLegendTwo: "2 mods",
    mapLegendThree: "3 mods",
    mapLegendMany: "4 or more",
    mapNoMods: "No mod places objects here.",
    mapLegendSpot: "collision",
    mapCell: (x, y) => `Cell ${x}, ${y}`,
    mapObjects: (count) => `${count} ${plural(count, "object", "objects")} placed by mods`,
    mapZoomIn: "Zoom in",
    mapZoomOut: "Zoom out",
    mapFit: "Show all",
    mapEmpty: "No mod places objects outside. Scan first, or this setup changes no landscape.",
    mapCollisions: "Collisions here",
    mapCells: "Cells with objects of mods",
    mapShared: "Cells where several mods meet",

    modsOnlyProblems: "Only mods with something open",
    modsSearch: "Search",
    modsProblems: (count) => `${count} open`,
    modsNone: "No mod matches.",

    places: "Places",
    placesTitleFirst: "Before Mod Mallet reads anything",
    placesTitle: "Where Mod Mallet reads",
    placesIntro:
      "Mod Mallet does not search your disk. It reads only these places, the files below the game directory, and the installer description inside each mod archive. It changes nothing.",
    placesConfirmFirst: "Use these and scan",
    placesSave: "Save and scan",
    cancel: "Cancel",
    notFound: "not found",
    placeUnknown: "could not be found; please enter it",
    placeAutomatic: "find by itself",
    whatWasRead: "What was read",
  },
  de: {
    scan: "Scannen",
    scanning: "Scan läuft…",
    scanningNote: "Dein Setup wird gelesen. Das dauert ein paar Sekunden.",
    language: "Sprache",
    readOnlyNote: "Ein Scan liest nur. An deinem Spiel und deinen Mods wird nichts verändert.",
    noScanYet: "Noch kein Scan. Starte einen, um zu sehen, was Mod Mallet findet.",
    scanFailed: "Der Scan konnte nicht abgeschlossen werden",
    versionUnknown: "Version unbekannt",
    scriptExtender: "SKSE",

    viewTasks: "Aufgaben",
    viewMap: "Karte",
    viewMods: "Mods",
    viewPlaces: "Orte",
    viewCollisions: "Kollisionen",
    viewPatch: "Patch",
    patchNeedsSwapper:
      "Ein Patch blendet Objekte mit Base Object Swapper aus, einem Script-Extender-Plugin, das nicht installiert ist.",
    patchCount: (count) => (count === 1 ? "1 Objekt ausgewählt" : `${count} Objekte ausgewählt`),
    patchInstall: "Patch in Vortex installieren",
    patchInstallHint:
      "Mod Mallet schreibt eine kleine Mod, die diese Objekte im Spiel durch einen unsichtbaren Marker ersetzt. Entfernst du die Mod in Vortex, sind sie wieder da.",
    patchHide: (plugin, count) => `Aus ${plugin} ausblenden (${count})`,
    patchKeepBoth: "Beide behalten",
    patchSuggest: "Vorschläge übernehmen",
    patchClear: "Zurücksetzen",
    patchCreate: "Patch erstellen",
    patchCreated: "Erstellt:",
    patchStale:
      "Die Auswahl hat sich seitdem geändert. Erstelle den Patch neu, bevor du ihn installierst.",
    patchSuggested: "Vorschlag",
    patchInstalled: (count, mod) =>
      `Installiert: ${count === 1 ? "1 Objekt ist" : `${count} Objekte sind`} ausgeblendet${mod === null ? "" : ` durch die Mod „${mod}“`}.`,
    patchMatches: "Deine Auswahl entspricht dem installierten Patch.",
    patchDiffers: (more, fewer) =>
      `Deine Auswahl weicht vom installierten Patch ab: ${more} mehr, ${fewer} weniger. Erstelle den Patch und installiere ihn erneut, um das zu übernehmen.`,
    patchObsolete: (count) =>
      count === 1
        ? "1 Objekt im installierten Patch gehört zu keiner Kollision mehr. Es fällt weg, wenn du den Patch neu installierst."
        : `${count} Objekte im installierten Patch gehören zu keiner Kollision mehr. Sie fallen weg, wenn du den Patch neu installierst.`,
    patchUseInstalled: "Installierten Stand übernehmen",
    patchInPatch: "im Patch",
    patchPartly: (hidden, of) => `${hidden} von ${of} im Patch`,
    patchNew: "noch nicht im Patch",
    openFolder: "Ordner öffnen",
    openLog: "Protokoll öffnen",
    tagline: "Hammering on",
    managerRunning: "läuft",
    managerOffline: "offline",
    managerMissing: "nicht installiert",
    managerLinkReady: (version) =>
      version === null ? "Verbindung steht" : `Verbindung ${version} steht`,
    managerLinkRestart: "Neustart nötig für die neue Verbindung",
    managerLinkOutdated: "Update der Verbindung verfügbar",
    managerLinkMissing: "Verbindung nicht installiert",
    gameLabel: "Spiel",
    games: [
      { id: "skyrimse", name: "Skyrim Special Edition" },
      { id: "skyrim", name: "Skyrim (2011)", quip: "der Hammer wärmt sich noch auf" },
      { id: "skyrimvr", name: "Skyrim VR", quip: "der Hammer braucht beide Hände" },
      { id: "fallout4", name: "Fallout 4", quip: "liegt noch in der Schmiede" },
      { id: "falloutnv", name: "Fallout: New Vegas", quip: "liegt auch noch in der Schmiede" },
      { id: "oblivion", name: "Oblivion", quip: "schließt erst noch ein paar Tore" },
      { id: "morrowind", name: "Morrowind", quip: "wartet noch auf einen Schlickschreiter" },
      { id: "starfield", name: "Starfield", quip: "der Hammer braucht noch einen Raumanzug" },
      { id: "tes6", name: "The Elder Scrolls VI", quip: "sobald es das gibt" },
      { id: "baldursgate3", name: "Baldur's Gate 3", quip: "würfelt noch auf Initiative" },
      {
        id: "cyberpunk2077",
        name: "Cyberpunk 2077",
        quip: "wartet auf einen Patch (ja, wissen wir)",
      },
    ],
    soon: "bald",
    viewHome: "Start",
    homeGames: "Spiele",
    homeActive: "Aktiv",
    homeNoArt: "Vortex bringt ein Bild dieses Spiels mit; hier ist es nicht installiert.",
    homeOpenTasks: "offene Aufgaben",
    homeCollisions: "Kollisionen",
    homeMods: "Mods",
    homePatch: "Objekte vom Patch ausgeblendet",
    homePatchNone: "kein Patch installiert",
    homeToTasks: "Zu den Aufgaben",
    settingsTabGeneral: "Allgemein",
    settingsTabPlaces: "Orte",
    settingsTabAbout: "Über",
    settingsLanguageNote: "Die Sprache des Fensters und aller Befunde.",
    settingsTheme: "Darstellung",
    settingsThemeNote: "Hell oder dunkel, oder wie Windows.",
    themeSystem: "Wie Windows",
    themeLight: "Hell",
    themeDark: "Dunkel",
    aboutText:
      "Mod Mallet schaut sich ein gemoddetes Spiel an, erklärt verständlich, was nicht stimmt, und lässt den Mod-Manager die Korrekturen ausführen. Selbst ändert es nichts.",
    viewToolbox: "Werkzeugkasten",
    viewSettings: "Einstellungen",
    toolboxReady: "Alles bereit",
    toolboxAttention: (count) =>
      count === 1 ? "1 Punkt braucht dich" : `${count} Punkte brauchen dich`,
    restartVortex: "Vortex neu starten",
    startVortex: "Vortex starten",
    restartingVortex: "Vortex wird geschlossen und wieder gestartet …",
    restartedVortex: "Vortex startet.",
    restartHint:
      "Schließt Vortex wie über das Schließen-Kreuz des Fensters und startet es wieder. Vortex speichert vorher wie immer.",
    updateLink: "Vortex-Erweiterung aktualisieren",
    getIt: "Holen",
    settingsLanguage: "Sprache",
    settingsLive: "Live",
    settingsLiveLabel: "Selbst neu prüfen, wenn sich das Setup ändert",
    settingsPlaces: "Wo Mod Mallet liest",
    settingsLog: "Protokoll",
    settingsLogNote:
      "Was Mod Mallet getan hat, Zeile für Zeile. Hilfreich, wenn etwas nicht geklappt hat – für dich oder für wen auch immer dir hilft.",
    collisionsHint: "Objekte zweier Mods an derselben Stelle.",

    live: "Live",
    liveNote:
      "Mod Mallet scannt von selbst erneut, wenn sich dein Setup ändert, zum Beispiel nachdem Vortex bereitgestellt hat.",
    changedRescanning: "Dein Setup hat sich geändert. Es wird neu gescannt…",

    allGood: "Alles in Ordnung",
    allGoodNote: "Mod Mallet hat nichts gefunden, um das du dich kümmern musst.",
    headline: (errors, warnings) => {
      const fix = `${errors} Fehler zu beheben`;
      const check = `${warnings} ${plural(warnings, "Punkt", "Punkte")} zu prüfen`;
      if (errors > 0 && warnings > 0) {
        return `${fix}, ${check}`;
      }
      return errors > 0 ? fix : check;
    },
    lastScan: "Letzter Scan",
    scanStats: ({ plugins, archives, installers, meshes }) =>
      `${plugins} Plugins · ${archives} Archive · ${installers} Installer · ${meshes} Modelle`,

    filterOpen: "Offen",
    filterFix: "Zu beheben",
    filterCheck: "Zu prüfen",
    filterDirect: "Direkt behebbar",
    filterNotes: "Gut zu wissen",
    filterIgnored: "Beiseitegelegt",
    fixHint: "Etwas funktioniert nicht, bis das behoben ist.",
    checkHint: "Wahrscheinlich ein Problem. Sieh es dir an.",
    directSectionHint: "Ein Klick übergibt das an Vortex. Dort schließt du es ab.",
    notesHint: "Nichts zu tun. Hier steht nur, welche Mod wo gewinnt.",
    ignoredHint: "Diese hast du beiseitegelegt. Sie werden oben nicht mitgezählt.",
    nothingHere: "Hier ist nichts.",

    whatToDo: "Was tun",
    choicesTitle: "Deine Möglichkeiten",
    choiceRun: "Ausführen",
    choiceLeave: "So lassen",
    choiceLeaveNote:
      "Wenn es dich im Spiel nicht stört. Mod Mallet legt den Befund beiseite und zählt ihn nicht mehr mit; du findest ihn unter „Beiseitegelegt“ wieder.",
    choiceBringBackNote: "Du hast das beiseitegelegt. Hol es zurück, um neu zu entscheiden.",
    choiceHide: "Eines der beiden ausblenden",
    choiceHideNote: (chosen) =>
      chosen === null
        ? "Wähle, wessen Objekte verschwinden. Das kommt in den Patch von Mod Mallet; es ändert sich nichts, bis du ihn auf der Patch-Seite installierst."
        : `Die Objekte aus ${chosen} kommen in den Patch von Mod Mallet. Installiere den Patch auf der Patch-Seite, damit es wirkt.`,
    choiceToPatch: "Zum Patch",
    choiceByHand: "Von Hand",
    moreAbout: "Mehr dazu",
    whatItMeans: "Was das bedeutet",
    file: "Datei",
    mod: "Mod",
    rescan: "Erneut scannen",
    installLink: "Vortex-Erweiterung installieren",
    guideTitle: "Im Installer ankreuzen",
    ignore: "Beiseitelegen",
    unignore: "Zurückholen",
    spotsTitle: (shown, total) =>
      shown === total ? `Wo (${total})` : `Wo (${shown} von ${total})`,
    consoleHint:
      "Im Spiel die Konsole mit der Taste links neben der 1 öffnen, diesen Befehl einfügen und Enter drücken.",
    copy: "Kopieren",
    copied: "Kopiert",
    showOnMap: "Auf der Karte zeigen",

    version: "Version",
    newerVersion: (version) => `Version ${version} ist verfügbar`,
    author: "Autor",
    openPage: "Mod-Seite öffnen",
    disabled: "deaktiviert",

    linkReady: "Vortex verbunden",
    linkRestart: "Vortex neu starten",
    linkNotRunning: "Vortex antwortet nicht",

    mapWorld: "Weltraum",
    mapMod: "Mod",
    mapAllMods: "Alle Mods",
    mapLegendOne: "1 Mod",
    mapLegendTwo: "2 Mods",
    mapLegendThree: "3 Mods",
    mapLegendMany: "4 oder mehr",
    mapNoMods: "Hier platziert keine Mod Objekte.",
    mapLegendSpot: "Zusammenstoß",
    mapCell: (x, y) => `Zelle ${x}, ${y}`,
    mapObjects: (count) => `${count} ${plural(count, "Objekt", "Objekte")} von Mods aufgestellt`,
    mapZoomIn: "Vergrößern",
    mapZoomOut: "Verkleinern",
    mapFit: "Alles zeigen",
    mapEmpty:
      "Keine Mod stellt draußen Objekte auf. Scanne zuerst, oder dieses Setup verändert keine Landschaft.",
    mapCollisions: "Zusammenstöße hier",
    mapCells: "Zellen mit Objekten von Mods",
    mapShared: "Zellen, in denen sich Mods begegnen",

    modsOnlyProblems: "Nur Mods mit offenen Punkten",
    modsSearch: "Suchen",
    modsProblems: (count) => `${count} offen`,
    modsNone: "Keine Mod passt.",

    places: "Orte",
    placesTitleFirst: "Bevor Mod Mallet etwas liest",
    placesTitle: "Wo Mod Mallet liest",
    placesIntro:
      "Mod Mallet durchsucht deine Festplatte nicht. Es liest nur diese Orte, die Dateien unterhalb des Spielordners und die Installer-Beschreibung in jedem Mod-Archiv. Geändert wird nichts.",
    placesConfirmFirst: "So verwenden und scannen",
    placesSave: "Speichern und scannen",
    cancel: "Abbrechen",
    notFound: "nicht gefunden",
    placeUnknown: "nicht gefunden; bitte eintragen",
    placeAutomatic: "selbst finden",
    whatWasRead: "Was gelesen wurde",
  },
};

const STORAGE_KEY = "mod-mallet.lang";

function isLang(value: unknown): value is Lang {
  return value === "en" || value === "de";
}

/** The language to start in: the one chosen last time, else the system's. */
export function initialLang(): Lang {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (isLang(stored)) {
      return stored;
    }
  } catch {
    // Storage can be unavailable; the system language is a fine default.
  }
  return navigator.language.toLowerCase().startsWith("de") ? "de" : "en";
}

const LIVE_KEY = "mod-mallet.live";
const THEME_KEY = "mod-mallet.theme";

/** Light, dark, or whatever Windows uses. */
export type Theme = "system" | "light" | "dark";

/** The theme chosen last time; like Windows unless chosen otherwise. */
export function initialTheme(): Theme {
  try {
    const stored = localStorage.getItem(THEME_KEY);
    if (stored === "light" || stored === "dark") {
      return stored;
    }
  } catch {
    // Storage can be unavailable; following Windows is a fine default.
  }
  return "system";
}

/** Shows the window in `theme` and remembers it. */
export function applyTheme(theme: Theme): void {
  if (theme === "system") {
    delete document.documentElement.dataset.theme;
  } else {
    document.documentElement.dataset.theme = theme;
  }
  try {
    localStorage.setItem(THEME_KEY, theme);
  } catch {
    // Not being able to remember the choice is not worth an error.
  }
}

/** Whether the app scans again by itself; on unless switched off. */
export function initialLive(): boolean {
  try {
    return localStorage.getItem(LIVE_KEY) !== "off";
  } catch {
    return true;
  }
}

export function rememberLive(live: boolean): void {
  try {
    localStorage.setItem(LIVE_KEY, live ? "on" : "off");
  } catch {
    // Not being able to remember the choice is not worth an error.
  }
}

export function rememberLang(lang: Lang): void {
  try {
    localStorage.setItem(STORAGE_KEY, lang);
  } catch {
    // Not being able to remember the choice is not worth an error.
  }
}
