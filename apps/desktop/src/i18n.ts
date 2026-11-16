// Texts of the app's own controls.
//
// Everything that describes a finding comes from the Rust side already
// translated (crates/mallet-messages), so that the app and the command line
// always say the same thing. Only the words of the window itself live here.

import type { MapMarker } from "./generated/rendered-report";

/** Whose a place of the map is. */
export type PlaceOrigin = MapMarker["origin"];
/** What kind of place it is. */
export type PlaceKind = MapMarker["kind"];

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
  previewTitle: string;
  viewer3d: string;
  viewerHint: string;
  viewerClose: string;
  viewerNothing: string;
  view3d: string;
  view3dBoth: string;
  duelsTitle: string;
  duelsHint: string;
  duelsDecided: (count: number) => string;
  duelsMore: (count: number) => string;
  duelWinner: string;
  duelNow: (name: string) => string;
  duelPick: (name: string) => string;
  duelCompare: string;
  duelCompareClose: string;
  duelUndo: string;
  duelPatchChosen: (count: number) => string;
  duelNoPictures: string;
  duelOriginal: string;
  previewLoading: string;
  previewInGame: string;
  previewLoose: string;
  previewNoPicture: string;
  viewCompare: string;
  compareOpen: string;
  compareBack: string;
  compareDuels: string;
  compareRanking: string;
  compareOpenLabel: string;
  compareDecidedLabel: string;
  compareSearch: string;
  compareAllTopics: string;
  compareShowDecided: string;
  compareFiles: (count: number) => string;
  compareNone: string;
  compareCannotShow: string;
  compareObjects: string;
  compareTechnical: (count: number) => string;
  topicName: Record<"faces" | "textures" | "meshes" | "objects" | "contained", string>;
  objectKinds: Record<string, string>;
  rankedMark: string;
  rankedFor: (name: string) => string;
  rankingHint: string;
  rankingOthers: (count: number) => string;
  rankingUp: string;
  rankingDown: string;
  rankingRemove: string;
  rankingPrefer: string;
  rankingDefer: string;
  rankingUnplaced: string;
  rankingDuels: (count: number) => string;
  rankingApply: (count: number) => string;
  rankingConfirm: (count: number) => string;
  rankingRunning: (done: number, total: number) => string;
  rankingDone: (decided: number, hidden: number, failed: number) => string;
  rankingNothing: string;
  duelCompareFiles: (count: number) => string;
  arrangeLabel: string;
  arrangeCategory: string;
  arrangePairs: string;
  arrangeDuelsAside: (count: number) => string;
  stageStop: string;
  stageTurn: string;
  stageReset: string;
  stageBrightness: string;
  stageLightAround: string;
  stageLightHeight: string;
  stageScene: string;
  sceneDark: string;
  sceneWhite: string;
  sceneMeadow: string;
  sceneForest: string;
  sceneCave: string;
  sceneSnow: string;
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

  // The places of the map and the 3D relief (ADR 0031).
  mapFlat: string;
  mapRelief: string;
  mapNoRelief: string;
  mapReliefHint: string;
  mapHeight: string;
  mapModCells: string;
  mapTopView: string;
  mapPlaces: string;
  placesShowAll: string;
  placesHideAll: string;
  placesOrigin: Record<PlaceOrigin, string>;
  placesOriginHint: Record<PlaceOrigin, string>;
  placesKind: Record<PlaceKind, string>;
  placesCollisions: string;
  placesUnnamed: string;
  placesBy: (owner: string) => string;
  placesMoved: (distance: string) => string;
  placesRenamed: (name: string) => string;
  placesNear: string;
  placesNearby: string;
  placesNone: string;
  placesCollisionsNear: string;
  placesDistance: (metres: number) => string;
  placesPick: string;

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
    previewTitle: "What it looks like",
    viewer3d: "In 3D",
    viewerHint: "Drag to turn, scroll to zoom.",
    viewerClose: "Close",
    viewerNothing: "This mesh cannot be shown.",
    view3d: "View in 3D",
    view3dBoth: "Both in 3D",
    duelsTitle: "Mod against mod",
    duelsHint:
      "Two mods that get in each other's way, with everything they disagree about. Pick the one that should win; it then wins everywhere at once.",
    duelsDecided: (count) => (count === 1 ? "1 decided" : `${count} decided`),
    duelsMore: (count) => `Show ${Math.min(count, 8)} more (${count} left)`,
    duelWinner: "wins",
    duelNow: (name) => `now: ${name}`,
    duelPick: (name) => `${name} wins`,
    duelCompare: "Compare",
    duelCompareClose: "Close comparison",
    duelUndo: "Decide again",
    duelPatchChosen: (count) =>
      count === 1
        ? "1 object of the other mod goes into the patch."
        : `${count} objects of the other mod go into the patch.`,
    duelNoPictures: "There is nothing to show as a picture for these two.",
    duelOriginal: "the game's original",
    previewLoading: "Loading the pictures…",
    previewInGame: "in the game",
    previewLoose: "loose file",
    previewNoPicture: "no picture",
    viewCompare: "Compare mods",
    compareOpen: "Open the comparison",
    compareBack: "Back",
    compareDuels: "Pairs",
    compareRanking: "Ranking",
    compareOpenLabel: "open",
    compareDecidedLabel: "decided",
    compareSearch: "Search mods or objects",
    compareAllTopics: "Everything",
    compareShowDecided: "Show decided ones",
    compareFiles: (count) => (count === 1 ? "1 file" : `${count} files`),
    compareNone: "There are no two mods that get in each other's way.",
    compareCannotShow: "This file cannot be shown as a picture.",
    compareObjects: "What they disagree about",
    compareTechnical: (count) => `Show normal, gloss and glow maps (${count})`,
    topicName: {
      faces: "Faces",
      textures: "Textures",
      meshes: "Meshes",
      objects: "Objects in the world",
      contained: "Files inside each other",
    },
    objectKinds: {
      static: "Static object",
      "movable-static": "Movable object",
      "static-collection": "Group of objects",
      tree: "Tree or plant",
      furniture: "Furniture",
      container: "Container",
      door: "Door",
      activator: "Activator",
    },
    rankedMark: "ranking",
    rankedFor: (name) => `Your ranking: ${name}`,
    rankingHint:
      "Put the mods you like best at the top. In every pair, the one higher up wins. All mods you have not placed stand where the grey line is: move a mod below it, and it loses against all of them.",
    rankingOthers: (count) => `All other mods (${count})`,
    rankingUp: "Move up",
    rankingDown: "Move down",
    rankingRemove: "Take out of the ranking",
    rankingPrefer: "↑ prefer",
    rankingDefer: "↓ put last",
    rankingUnplaced: "Not placed yet",
    rankingDuels: (count) => (count === 1 ? "1 pair" : `${count} pairs`),
    rankingApply: (count) =>
      count === 1
        ? "Decide 1 open pair by the ranking"
        : `Decide ${count} open pairs by the ranking`,
    rankingConfirm: (count) =>
      `This asks Vortex to change the load order for ${count === 1 ? "1 pair" : `${count} pairs`}. Decisions you made yourself stay as they are, and every pair can be decided again later.`,
    rankingRunning: (done, total) => `Deciding… ${done} of ${total}`,
    rankingDone: (decided, hidden, failed) =>
      [
        decided === 1 ? "1 pair decided." : `${decided} pairs decided.`,
        hidden === 1
          ? "1 object goes into the patch."
          : hidden > 1
            ? `${hidden} objects go into the patch.`
            : "",
        failed === 1
          ? "1 change did not work."
          : failed > 1
            ? `${failed} changes did not work.`
            : "",
      ]
        .filter((part) => part !== "")
        .join(" "),
    rankingNothing: "The ranking decides no open pair yet",
    duelCompareFiles: (count) => `Compare (${count === 1 ? "1 file" : `${count} files`})`,
    arrangeLabel: "Arrange the tasks",
    arrangeCategory: "By kind",
    arrangePairs: "Mod against mod",
    arrangeDuelsAside: (count) =>
      `${count === 1 ? "1 pair of mods that disagree is" : `${count} pairs of mods that disagree are`} under "Mod against mod".`,
    stageStop: "Stop turning",
    stageTurn: "Turn",
    stageReset: "Reset",
    stageBrightness: "Brightness",
    stageLightAround: "Light around",
    stageLightHeight: "Light height",
    stageScene: "Scene",
    sceneDark: "Dark room",
    sceneWhite: "White room",
    sceneMeadow: "Meadow",
    sceneForest: "Forest",
    sceneCave: "Cave",
    sceneSnow: "Snow",
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

    mapFlat: "Map",
    mapRelief: "3D relief",
    mapNoRelief: "The land of this worldspace is not known, so there is no relief to show.",
    mapReliefHint: "Drag to move · right-drag to turn · wheel to zoom",
    mapHeight: "Height",
    mapModCells: "Cells of mods",
    mapTopView: "From above",
    mapPlaces: "Places",
    placesShowAll: "Show all",
    placesHideAll: "Hide all",
    placesOrigin: {
      game: "The game's",
      added: "Added by a mod",
      changed: "Changed by a mod",
      removed: "Removed by a mod",
    },
    placesOriginHint: {
      game: "Places as the game has them, untouched by mods.",
      added: "New places that a mod brings.",
      changed: "Places of the game that a mod moved, renamed or turned into something else.",
      removed: "Places that a mod took off the map.",
    },
    placesKind: {
      city: "City",
      town: "Town",
      settlement: "Settlement",
      castle: "Castle",
      cave: "Cave",
      camp: "Camp",
      fort: "Fort",
      "nordic-ruin": "Nordic ruin",
      "dwemer-ruin": "Dwemer ruin",
      shipwreck: "Shipwreck",
      grove: "Grove",
      landmark: "Landmark",
      "dragon-lair": "Dragon lair",
      farm: "Farm",
      "wood-mill": "Wood mill",
      mine: "Mine",
      "imperial-camp": "Imperial camp",
      "stormcloak-camp": "Stormcloak camp",
      doomstone: "Standing stone",
      "wheat-mill": "Wheat mill",
      smelter: "Smelter",
      stable: "Stable",
      "imperial-tower": "Imperial tower",
      clearing: "Clearing",
      pass: "Pass",
      shrine: "Shrine",
      lighthouse: "Lighthouse",
      "orc-stronghold": "Orc stronghold",
      "giant-camp": "Giant camp",
      shack: "Shack",
      "nordic-tower": "Nordic tower",
      "nordic-dwelling": "Nordic dwelling",
      docks: "Docks",
      "daedric-shrine": "Daedric shrine",
      passage: "Passage",
      other: "Other",
    },
    placesCollisions: "Collisions of mods",
    placesUnnamed: "Unnamed place",
    placesBy: (owner) => `by ${owner}`,
    placesMoved: (distance) => `Moved ${distance} away from where the game has it.`,
    placesRenamed: (name) => `The game calls it “${name}”.`,
    placesNear: "Near",
    placesNearby: "Nearest places",
    placesNone: "No place of the map nearby.",
    placesCollisionsNear: "Collisions nearby",
    placesDistance: (metres) =>
      metres >= 1000 ? `${(metres / 1000).toFixed(1)} km` : `${Math.round(metres)} m`,
    placesPick: "Pick a place, a collision or a cell on the map.",

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
    previewTitle: "So sieht es aus",
    viewer3d: "In 3D",
    viewerHint: "Ziehen zum Drehen, Mausrad zum Zoomen.",
    viewerClose: "Schließen",
    viewerNothing: "Dieses Mesh lässt sich nicht zeigen.",
    view3d: "In 3D ansehen",
    view3dBoth: "Beide in 3D",
    duelsTitle: "Mod gegen Mod",
    duelsHint:
      "Zwei Mods, die sich in die Quere kommen, mit allem, worin sie sich uneins sind. Wähle die, die gewinnen soll; sie gewinnt dann überall auf einmal.",
    duelsDecided: (count) => `${count} entschieden`,
    duelsMore: (count) => `${Math.min(count, 8)} weitere zeigen (${count} übrig)`,
    duelWinner: "gewinnt",
    duelNow: (name) => `jetzt: ${name}`,
    duelPick: (name) => `${name} gewinnt`,
    duelCompare: "Vergleichen",
    duelCompareClose: "Vergleich schließen",
    duelUndo: "Neu entscheiden",
    duelPatchChosen: (count) =>
      count === 1
        ? "1 Objekt der anderen Mod kommt in den Patch."
        : `${count} Objekte der anderen Mod kommen in den Patch.`,
    duelNoPictures: "Für diese beiden gibt es nichts als Bild zu zeigen.",
    duelOriginal: "Original des Spiels",
    previewLoading: "Bilder werden geladen …",
    previewInGame: "im Spiel",
    previewLoose: "lose Datei",
    previewNoPicture: "kein Bild",
    viewCompare: "Mods vergleichen",
    compareOpen: "Vergleich öffnen",
    compareBack: "Zurück",
    compareDuels: "Paare",
    compareRanking: "Rangfolge",
    compareOpenLabel: "offen",
    compareDecidedLabel: "entschieden",
    compareSearch: "Mods oder Objekte suchen",
    compareAllTopics: "Alles",
    compareShowDecided: "Entschiedene zeigen",
    compareFiles: (count) => (count === 1 ? "1 Datei" : `${count} Dateien`),
    compareNone: "Es gibt keine zwei Mods, die sich in die Quere kommen.",
    compareCannotShow: "Diese Datei lässt sich nicht als Bild zeigen.",
    compareObjects: "Worum sie streiten",
    compareTechnical: (count) => `Normal-, Glanz- und Leuchtkarten zeigen (${count})`,
    topicName: {
      faces: "Gesichter",
      textures: "Texturen",
      meshes: "Meshes",
      objects: "Objekte in der Welt",
      contained: "Dateien ineinander",
    },
    objectKinds: {
      static: "Festes Objekt",
      "movable-static": "Bewegliches Objekt",
      "static-collection": "Objektgruppe",
      tree: "Baum oder Pflanze",
      furniture: "Möbel",
      container: "Behälter",
      door: "Tür",
      activator: "Aktivator",
    },
    rankedMark: "Rangfolge",
    rankedFor: (name) => `Deine Rangfolge: ${name}`,
    rankingHint:
      "Die Mods, die dir am besten gefallen, nach oben. In jedem Paar gewinnt der weiter oben. Alle Mods, die du nicht einordnest, stehen bei der grauen Linie: Schiebst du einen Mod darunter, verliert er gegen alle.",
    rankingOthers: (count) => `Alle anderen Mods (${count})`,
    rankingUp: "Nach oben",
    rankingDown: "Nach unten",
    rankingRemove: "Aus der Rangfolge nehmen",
    rankingPrefer: "↑ bevorzugen",
    rankingDefer: "↓ ganz nach hinten",
    rankingUnplaced: "Noch nicht eingeordnet",
    rankingDuels: (count) => (count === 1 ? "1 Paar" : `${count} Paare`),
    rankingApply: (count) =>
      count === 1
        ? "1 offenes Paar nach Rangfolge entscheiden"
        : `${count} offene Paare nach Rangfolge entscheiden`,
    rankingConfirm: (count) =>
      `Das bittet Vortex, die Ladereihenfolge für ${count === 1 ? "1 Paar" : `${count} Paare`} zu ändern. Was du selbst entschieden hast, bleibt, und jedes Paar lässt sich später neu entscheiden.`,
    rankingRunning: (done, total) => `Wird entschieden … ${done} von ${total}`,
    rankingDone: (decided, hidden, failed) =>
      [
        decided === 1 ? "1 Paar entschieden." : `${decided} Paare entschieden.`,
        hidden > 0 ? `${hidden} Objekte kommen in den Patch.` : "",
        failed === 1
          ? "1 Änderung hat nicht geklappt."
          : failed > 1
            ? `${failed} Änderungen haben nicht geklappt.`
            : "",
      ]
        .filter((part) => part !== "")
        .join(" "),
    rankingNothing: "Die Rangfolge entscheidet noch kein offenes Paar",
    duelCompareFiles: (count) => `Vergleichen (${count === 1 ? "1 Datei" : `${count} Dateien`})`,
    arrangeLabel: "Aufgaben ordnen",
    arrangeCategory: "Nach Art",
    arrangePairs: "Mod gegen Mod",
    arrangeDuelsAside: (count) =>
      `${count === 1 ? "1 Paar von Mods, die sich widersprechen, steht" : `${count} Paare von Mods, die sich widersprechen, stehen`} unter „Mod gegen Mod“.`,
    stageStop: "Drehen anhalten",
    stageTurn: "Drehen",
    stageReset: "Zurücksetzen",
    stageBrightness: "Helligkeit",
    stageLightAround: "Licht ringsum",
    stageLightHeight: "Lichthöhe",
    stageScene: "Umgebung",
    sceneDark: "Dunkler Raum",
    sceneWhite: "Weißer Raum",
    sceneMeadow: "Wiese",
    sceneForest: "Wald",
    sceneCave: "Höhle",
    sceneSnow: "Schnee",
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

    mapFlat: "Karte",
    mapRelief: "3D-Relief",
    mapNoRelief: "Das Gelände dieses Weltraums ist nicht bekannt, darum gibt es kein Relief.",
    mapReliefHint: "Ziehen verschiebt · rechts ziehen dreht · Mausrad zoomt",
    mapHeight: "Höhe",
    mapModCells: "Zellen von Mods",
    mapTopView: "Von oben",
    mapPlaces: "Orte",
    placesShowAll: "Alle zeigen",
    placesHideAll: "Alle ausblenden",
    placesOrigin: {
      game: "Vom Spiel",
      added: "Von Mod hinzugefügt",
      changed: "Von Mod verändert",
      removed: "Von Mod entfernt",
    },
    placesOriginHint: {
      game: "Orte, wie das Spiel sie hat, von keiner Mod berührt.",
      added: "Neue Orte, die eine Mod mitbringt.",
      changed:
        "Orte des Spiels, die eine Mod verschoben, umbenannt oder zu etwas anderem gemacht hat.",
      removed: "Orte, die eine Mod von der Karte genommen hat.",
    },
    placesKind: {
      city: "Stadt",
      town: "Dorf",
      settlement: "Siedlung",
      castle: "Burg",
      cave: "Höhle",
      camp: "Lager",
      fort: "Festung",
      "nordic-ruin": "Nordische Ruine",
      "dwemer-ruin": "Dwemer-Ruine",
      shipwreck: "Schiffswrack",
      grove: "Hain",
      landmark: "Wahrzeichen",
      "dragon-lair": "Drachenhort",
      farm: "Hof",
      "wood-mill": "Sägewerk",
      mine: "Mine",
      "imperial-camp": "Kaiserliches Lager",
      "stormcloak-camp": "Lager der Sturmmäntel",
      doomstone: "Menhir",
      "wheat-mill": "Mühle",
      smelter: "Schmelze",
      stable: "Stall",
      "imperial-tower": "Kaiserlicher Wachturm",
      clearing: "Lichtung",
      pass: "Pass",
      shrine: "Schrein",
      lighthouse: "Leuchtturm",
      "orc-stronghold": "Orkfestung",
      "giant-camp": "Riesenlager",
      shack: "Hütte",
      "nordic-tower": "Nordischer Turm",
      "nordic-dwelling": "Nordische Behausung",
      docks: "Hafen",
      "daedric-shrine": "Daedra-Schrein",
      passage: "Überfahrt",
      other: "Sonstiges",
    },
    placesCollisions: "Zusammenstöße von Mods",
    placesUnnamed: "Ort ohne Namen",
    placesBy: (owner) => `durch ${owner}`,
    placesMoved: (distance) => `Um ${distance} verschoben gegenüber dem Spiel.`,
    placesRenamed: (name) => `Im Spiel heißt er „${name}“.`,
    placesNear: "Nahe",
    placesNearby: "Nächste Orte",
    placesNone: "Kein Ort der Karte in der Nähe.",
    placesCollisionsNear: "Zusammenstöße in der Nähe",
    placesDistance: (metres) =>
      metres >= 1000
        ? `${(metres / 1000).toFixed(1).replace(".", ",")} km`
        : `${Math.round(metres)} m`,
    placesPick: "Wähle einen Ort, einen Zusammenstoß oder eine Zelle auf der Karte.",

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
