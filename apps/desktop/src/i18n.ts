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
  noScanYet: string;
  scanFailed: string;
  versionUnknown: string;
  scriptExtender: string;

  // The views of the sidebar.
  viewTasks: string;
  viewMap: string;
  viewMods: string;
  viewCollisions: string;
  viewPatch: string;
  patchKeepBoth: string;
  patchSuggested: string;
  compareSpots: (count: number) => string;
  compareToSpots: string;
  openFolder: string;
  managerRunning: string;
  managerOffline: string;
  managerMissing: string;
  managerLinkReady: (version: string | null) => string;
  managerLinkRestart: string;
  managerLinkOutdated: string;
  managerLinkMissing: string;
  /**
   * The games of the game menu. Those Mod Mallet reads have no quip; which
   * of them are installed comes from the Rust side. The others are a promise.
   */
  games: { id: string; name: string; quip?: string }[];
  /** A game Mod Mallet reads, but that is not on this machine. */
  gameNotInstalled: string;
  /** A game Mod Mallet reads, but not yet with every check. */
  gameBasic: string;
  homeNoArt: string;
  viewToolbox: string;
  viewSettings: string;
  collisionsHint: string;

  liveNote: string;
  liveShort: string;
  changedRescanning: string;

  lastScan: string;
  scanStats: (stats: ScanStats) => string;

  // The filters of the task list, which are also its sections.
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
  choiceLeave: string;
  choiceToPatch: string;
  moreAbout: string;
  previewTitle: string;
  viewer3d: string;
  viewerHint: string;
  viewerClose: string;
  viewerNothing: string;
  viewerSkeleton: string;
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
  duelUndo: string;
  duelPatchChosen: (count: number) => string;
  duelNoPictures: string;
  duelOriginal: string;
  previewLoading: string;
  previewInGame: string;
  previewLoose: string;
  previewNoPicture: string;
  compareOpen: string;
  compareDuels: string;
  compareRanking: string;
  compareOpenLabel: string;
  compareDecidedLabel: string;
  compareSearch: string;
  compareShowDecided: string;
  compareFiles: (count: number) => string;
  compareNone: string;
  compareCannotShow: string;
  compareObjects: string;
  compareTechnical: (count: number) => string;
  topicName: Record<"faces" | "textures" | "meshes" | "objects" | "contained" | "files", string>;
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
  rescan: string;
  installLink: string;
  guideTitle: string;
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

  // The map.
  mapWorld: string;
  mapMod: string;
  mapAllMods: string;
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
  mapBase: string;
  mapLod: string;
  mapNear: string;
  mapNearFar: string;
  mapNearLoading: string;
  mapNearCount: (count: number) => string;
  mapNearProgress: (count: number, done: number, total: number) => string;
  mapObjectFrom: string;
  mapObjectPlacedBy: string;
  mapObjectId: string;
  mapObjectModel: string;
  // The collisions as a worklist, decided spot by spot (ADR 0040).
  spotKinds: Record<"duplicate" | "door" | "overlap" | "deep" | "touch", string>;
  spotKindHints: Record<"duplicate" | "door" | "overlap" | "deep" | "touch", string>;
  workTabSpots: string;
  workPreviewWithout: (name: string) => string;
  workPairMany: (first: string, second: string, spots: number) => string;
  workShowSpots: string;
  workObjectGame: string;
  workObjectHide: string;
  workObjectShow: string;
  workProgress: (decided: number, total: number) => string;
  workGroupBy: string;
  workByRegion: string;
  workByMods: string;
  workHideDecided: string;
  workInterior: (cell: string) => string;
  workDepth: (metres: number) => string;
  workHidden: (plugin: string) => string;
  workKept: string;
  workOpen: string;
  workSpotOf: (index: number, total: number) => string;
  workCannotHide: string;
  workReset: string;
  workPrevious: string;
  workNext: string;
  workNextOpen: string;
  workEverywhere: (plugin: string) => string;
  workKeepEverywhere: string;
  workKeys: string;
  workToPatch: (objects: number) => string;
  workPickSpot: string;
  workInside: string;
  workEmpty: string;
  workMore: string;
  workGhost: string;
  workThisSpot: string;
  workSearch: string;
  workHideVerb: string;
  workKeepVerb: string;
  workNeeded: string;
  workBothNeeded: string;
  anchorAsking: string;
  anchorLocked: string;
  anchorCareful: string;
  anchorHideAnyway: string;
  anchorWhy: string;
  anchorGone: string;
  anchorOff: string;
  anchorKinds: Record<
    | "teleport-door"
    | "quest-alias"
    | "large-reference"
    | "activate-parent"
    | "enable-parent"
    | "linked-ref"
    | "location-role"
    | "package"
    | "script"
    | "visible-when-distant"
    | "persistent",
    (other: string, keyword: string | null) => string
  >;
  workSortBy: string;
  workSortOpen: string;
  workSortSpots: string;
  workWinsEverywhere: (mod: string) => string;
  workDuelWinner: (mod: string) => string;
  duelFromCollisions: (mod: string) => string;
  duelTakeOver: string;
  workHideObject: string;
  workDecideHere: string;
  workAfterDecide: string;
  /** What the fast forward checkbox does, as its tooltip. */
  workAfterDecideHint: string;
  mapPlaces: string;
  placesShowAll: string;
  placesHideAll: string;
  placesOrigin: Record<PlaceOrigin, string>;
  placesOriginHint: Record<PlaceOrigin, string>;
  placesKind: Record<PlaceKind, string>;
  placesCollisions: string;
  placesBy: (owner: string) => string;
  placesMoved: (distance: string) => string;
  placesRenamed: (name: string) => string;
  placesNear: string;
  placesNearby: string;
  placesNone: string;
  placesCollisionsNear: string;
  placesDistance: (metres: number) => string;
  placesPick: string;

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
    noScanYet: "No scan yet. Start one to see what Mod Mallet finds.",
    scanFailed: "The scan could not be completed",
    versionUnknown: "version unknown",
    scriptExtender: "SKSE",

    viewTasks: "Tasks",
    viewMap: "Map",
    viewMods: "Mods",
    viewCollisions: "Collisions",
    viewPatch: "Patch",
    patchKeepBoth: "Keep both",
    patchSuggested: "suggested",
    compareSpots: (count) =>
      count === 1
        ? "Objects of these two mods collide at 1 spot. Look at it in 3D and decide there."
        : `Objects of these two mods collide at ${count} spots. Look at them in 3D and decide there.`,
    compareToSpots: "Show the spots in 3D",
    openFolder: "Open folder",
    managerRunning: "running",
    managerOffline: "offline",
    managerMissing: "not installed",
    managerLinkReady: (version) =>
      version === null ? "Link connected" : `Link ${version} connected`,
    managerLinkRestart: "Restart needed for the new link",
    managerLinkOutdated: "Link update available",
    managerLinkMissing: "Link not installed",
    games: [
      { id: "skyrimse", name: "Skyrim Special Edition" },
      { id: "skyrimvr", name: "Skyrim VR" },
      { id: "skyrim", name: "Skyrim (2011)" },
      { id: "enderalspecialedition", name: "Enderal Special Edition" },
      { id: "fallout4", name: "Fallout 4" },
      { id: "fallout4vr", name: "Fallout 4 VR" },
      { id: "falloutnv", name: "Fallout: New Vegas" },
      { id: "fallout3", name: "Fallout 3" },
      { id: "oblivion", name: "Oblivion" },
      { id: "oblivionremastered", name: "Oblivion Remastered" },
      { id: "morrowind", name: "Morrowind", quip: "waiting for a silt strider" },
      { id: "starfield", name: "Starfield", quip: "the mallet needs a space suit" },
      { id: "tes6", name: "The Elder Scrolls VI", quip: "as soon as it exists" },
      { id: "baldursgate3", name: "Baldur's Gate 3", quip: "rolling for initiative" },
      { id: "cyberpunk2077", name: "Cyberpunk 2077", quip: "waiting for a patch (yes, we know)" },
    ],
    gameNotInstalled: "not found on this computer",
    gameBasic: "the basics: load order, files, archives, map, toolbox",
    homeNoArt: "Vortex brings a picture of this game along; it is not installed here.",
    viewToolbox: "Toolbox",
    viewSettings: "Settings",
    collisionsHint: "Objects of two mods in the same spot.",

    liveNote:
      "Mod Mallet scans again by itself when your setup changes, for example after Vortex deploys.",
    liveShort: "Scans again by itself",
    changedRescanning: "Your setup changed. Scanning again…",

    lastScan: "Last scan",
    scanStats: ({ plugins, archives, installers, meshes }) =>
      `${plugins} plugins · ${archives} archives · ${installers} installers · ${meshes} meshes`,

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
    choiceLeave: "Leave it as it is",
    choiceToPatch: "To the patch",
    moreAbout: "More about it",
    previewTitle: "What it looks like",
    viewer3d: "In 3D",
    viewerHint: "Drag to turn, scroll to zoom.",
    viewerClose: "Close",
    viewerNothing: "This mesh could not be read, or has nothing to draw.",
    viewerSkeleton: "A skeleton: only bones, drawn as lines.",
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
    compareOpen: "Open the comparison",
    compareDuels: "Pairs",
    compareRanking: "Ranking",
    compareOpenLabel: "open",
    compareDecidedLabel: "decided",
    compareSearch: "Search mods or objects",
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
      files: "Other files",
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
    rescan: "Scan again",
    installLink: "Install the Vortex extension",
    guideTitle: "Tick these in the installer",
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

    mapWorld: "Worldspace",
    mapMod: "Mod",
    mapAllMods: "All mods",
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
    mapBase: "On a base",
    mapLod: "Game detail",
    mapNear: "Objects up close",
    mapNearFar: "come closer to see them",
    mapNearLoading: "loading…",
    mapNearCount: (count) => (count === 1 ? "1 object" : `${count} objects`),
    mapNearProgress: (count, done, total) => `${count} objects · models ${done} of ${total}`,
    mapObjectFrom: "Version from",
    mapObjectPlacedBy: "Placed by",
    mapObjectId: "Number in that plugin",
    mapObjectModel: "Model",
    spotKinds: {
      duplicate: "duplicate",
      door: "doors",
      overlap: "inside each other",
      deep: "cross deeply",
      touch: "touch",
    },
    spotKindHints: {
      duplicate: "The same object twice in the same place.",
      door: "Doors standing on top of each other, often leading to different places.",
      overlap: "One object stands largely inside the other.",
      deep: "Two objects run into each other over 2 m or more; often two mods building the same thing.",
      touch: "Two objects reach into each other a little; often harmless.",
    },
    workTabSpots: "Spots",
    workPreviewWithout: (name) => `Look at it without ${name}, without deciding`,
    workPairMany: (first, second, spots) =>
      `${first} and ${second} collide at ${spots} spots. Usually it is better to choose one of them:`,
    workShowSpots: "Show the spots",
    workObjectGame:
      "This object belongs to the game itself. Hide it only if you are sure: a quest may need it.",
    workObjectHide: "Hide this object",
    workObjectShow: "Show it again",
    workProgress: (decided, total) => `${decided} of ${total} spots decided`,
    workGroupBy: "Group by",
    workByRegion: "Region",
    workByMods: "Mod pair",
    workHideDecided: "Hide decided spots",
    workInterior: (cell) => `Inside: ${cell}`,
    workDepth: (metres) =>
      `${metres.toLocaleString("en", { maximumFractionDigits: 1 })} m inside each other`,
    workHidden: (plugin) => `${plugin} hidden`,
    workKept: "both kept",
    workOpen: "open",
    workSpotOf: (index, total) => `Spot ${index} of ${total}`,
    workCannotHide: "Other things refer to it, so it cannot be hidden safely.",
    workReset: "Undecide",
    workPrevious: "Previous",
    workNext: "Next",
    workNextOpen: "Next open",
    workEverywhere: (plugin) => `Hide ${plugin} everywhere`,
    workKeepEverywhere: "Keep both everywhere",
    workKeys: "Keys: ← → spots · 1, 2 hide · 0 keep both · V markings",
    workToPatch: (objects) =>
      objects === 1 ? "To the patch (1 object)" : `To the patch (${objects} objects)`,
    workPickSpot: "Pick a spot from the list.",
    workInside: "This spot is inside a building or cave; there is no 3D view of it yet.",
    workEmpty: "No collisions to decide.",
    workMore: "Mod against mod",
    workGhost: "Red and see-through: what the patch would hide.",
    workThisSpot: "this spot",
    workSearch: "Search places, mods, objects…",
    workHideVerb: "Hide",
    workKeepVerb: "Keep",
    anchorAsking: "Looking at what ties it…",
    anchorLocked: "The game cannot do without this object:",
    anchorCareful: "The game may miss it. Hide it only if what follows does not matter to you:",
    anchorHideAnyway: "Hide anyway",
    anchorWhy: "Why it is needed",
    anchorGone: "(that one is gone, so this does not count)",
    anchorOff: "(that one is switched off)",
    anchorKinds: {
      "teleport-door": (other) => `A door with a way through${other ? ` to ${other}` : ""}`,
      "quest-alias": (other) => `A quest needs it${other ? ` (${other})` : ""}`,
      "large-reference": () => "Part of the world's distant view",
      "activate-parent": (other) => `It sets off ${other || "other objects"}`,
      "enable-parent": (other) =>
        `It switches other objects on and off${other ? `, such as ${other}` : ""}; they stay as they are`,
      "linked-ref": (other, keyword) =>
        `${other || "Another object"} is linked to it${keyword ? ` (${keyword})` : ""}`,
      "location-role": (other) => `It has a role in a location${other ? ` (${other})` : ""}`,
      package: (other) => `A character's routine uses it${other ? ` (${other})` : ""}`,
      script: (other) => `A script names it${other ? ` (${other})` : ""}`,
      "visible-when-distant": () =>
        "It is drawn from afar; the distant view keeps showing it until it is generated again",
      persistent: () => "The game keeps it loaded; nothing was found that still uses it",
    },
    workSortBy: "Order",
    workSortOpen: "Most open",
    workSortSpots: "Most spots",
    workWinsEverywhere: (mod) => `✓ ${mod} wins everywhere`,
    workDuelWinner: (mod) => `Duel: ${mod} wins`,
    duelFromCollisions: (mod) =>
      `In the collisions you let ${mod} win at every spot. Take that over here, for its files too?`,
    duelTakeOver: "Take it over",
    workNeeded: "Needed by the game",
    workBothNeeded:
      "The game needs both objects (for example doors with a loading screen, or things scripts refer to). Nothing here can be hidden safely; keeping both is usually right.",
    workHideObject: "Hide this object here",
    workDecideHere: "Decide in the worklist",
    workAfterDecide: "Fast forward",
    workAfterDecideHint: "After deciding, go on to the next open spot",
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
      vault: "Vault",
      ruin: "Ruins",
      factory: "Factory",
      station: "Station",
      building: "Building",
      "ayleid-ruin": "Ayleid ruin",
      "oblivion-gate": "Oblivion gate",
      inn: "Inn",
      other: "Other",
    },
    placesCollisions: "Collisions of mods",
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
    noScanYet: "Noch kein Scan. Starte einen, um zu sehen, was Mod Mallet findet.",
    scanFailed: "Der Scan konnte nicht abgeschlossen werden",
    versionUnknown: "Version unbekannt",
    scriptExtender: "SKSE",

    viewTasks: "Aufgaben",
    viewMap: "Karte",
    viewMods: "Mods",
    viewCollisions: "Kollisionen",
    viewPatch: "Patch",
    patchKeepBoth: "Beide behalten",
    patchSuggested: "Vorschlag",
    compareSpots: (count) =>
      count === 1
        ? "Objekte dieser beiden Mods stoßen an 1 Stelle zusammen. Sieh sie dir in 3D an und entscheide dort."
        : `Objekte dieser beiden Mods stoßen an ${count} Stellen zusammen. Sieh sie dir in 3D an und entscheide dort.`,
    compareToSpots: "Stellen in 3D ansehen",
    openFolder: "Ordner öffnen",
    managerRunning: "läuft",
    managerOffline: "offline",
    managerMissing: "nicht installiert",
    managerLinkReady: (version) =>
      version === null ? "Verbindung steht" : `Verbindung ${version} steht`,
    managerLinkRestart: "Neustart nötig für die neue Verbindung",
    managerLinkOutdated: "Update der Verbindung verfügbar",
    managerLinkMissing: "Verbindung nicht installiert",
    games: [
      { id: "skyrimse", name: "Skyrim Special Edition" },
      { id: "skyrimvr", name: "Skyrim VR" },
      { id: "skyrim", name: "Skyrim (2011)" },
      { id: "enderalspecialedition", name: "Enderal Special Edition" },
      { id: "fallout4", name: "Fallout 4" },
      { id: "fallout4vr", name: "Fallout 4 VR" },
      { id: "falloutnv", name: "Fallout: New Vegas" },
      { id: "fallout3", name: "Fallout 3" },
      { id: "oblivion", name: "Oblivion" },
      { id: "oblivionremastered", name: "Oblivion Remastered" },
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
    gameNotInstalled: "auf diesem Computer nicht gefunden",
    gameBasic: "das Grundlegende: Ladereihenfolge, Dateien, Archive, Karte, Werkzeuge",
    homeNoArt: "Vortex bringt ein Bild dieses Spiels mit; hier ist es nicht installiert.",
    viewToolbox: "Werkzeugkasten",
    viewSettings: "Einstellungen",
    collisionsHint: "Objekte zweier Mods an derselben Stelle.",

    liveNote:
      "Mod Mallet scannt von selbst erneut, wenn sich dein Setup ändert, zum Beispiel nachdem Vortex bereitgestellt hat.",
    liveShort: "Scannt von selbst neu",
    changedRescanning: "Dein Setup hat sich geändert. Es wird neu gescannt…",

    lastScan: "Letzter Scan",
    scanStats: ({ plugins, archives, installers, meshes }) =>
      `${plugins} Plugins · ${archives} Archive · ${installers} Installer · ${meshes} Modelle`,

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
    choiceLeave: "So lassen",
    choiceToPatch: "Zum Patch",
    moreAbout: "Mehr dazu",
    previewTitle: "So sieht es aus",
    viewer3d: "In 3D",
    viewerHint: "Ziehen zum Drehen, Mausrad zum Zoomen.",
    viewerClose: "Schließen",
    viewerNothing: "Dieses Mesh ließ sich nicht lesen oder hat nichts zu zeichnen.",
    viewerSkeleton: "Ein Skelett: nur Knochen, als Linien gezeichnet.",
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
    compareOpen: "Vergleich öffnen",
    compareDuels: "Paare",
    compareRanking: "Rangfolge",
    compareOpenLabel: "offen",
    compareDecidedLabel: "entschieden",
    compareSearch: "Mods oder Objekte suchen",
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
      files: "Andere Dateien",
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
    rescan: "Erneut scannen",
    installLink: "Vortex-Erweiterung installieren",
    guideTitle: "Im Installer ankreuzen",
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

    mapWorld: "Weltraum",
    mapMod: "Mod",
    mapAllMods: "Alle Mods",
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
    mapBase: "Auf Sockel",
    mapLod: "Spieldetail",
    mapNear: "Objekte aus der Nähe",
    mapNearFar: "zum Sehen näher heran",
    mapNearLoading: "wird geladen …",
    mapNearCount: (count) => (count === 1 ? "1 Objekt" : `${count} Objekte`),
    mapNearProgress: (count, done, total) => `${count} Objekte · Modelle ${done} von ${total}`,
    mapObjectFrom: "Version aus",
    mapObjectPlacedBy: "Aufgestellt von",
    mapObjectId: "Nummer in diesem Plugin",
    mapObjectModel: "Modell",
    spotKinds: {
      duplicate: "doppelt",
      door: "Türen",
      overlap: "steckt ineinander",
      deep: "kreuzt tief",
      touch: "berührt sich",
    },
    spotKindHints: {
      duplicate: "Dasselbe Objekt zweimal an derselben Stelle.",
      door: "Türen übereinander, oft mit verschiedenen Zielen.",
      overlap: "Ein Objekt steckt großteils im anderen.",
      deep: "Zwei Objekte laufen über 2 m oder mehr ineinander; oft bauen zwei Mods dasselbe.",
      touch: "Zwei Objekte ragen ein wenig ineinander; oft harmlos.",
    },
    workTabSpots: "Stellen",
    workPreviewWithout: (name) => `Ohne ${name} ansehen, ohne zu entscheiden`,
    workPairMany: (first, second, spots) =>
      `${first} und ${second} stoßen an ${spots} Stellen zusammen. Meist ist es besser, dich für eine zu entscheiden:`,
    workShowSpots: "Stellen ansehen",
    workObjectGame:
      "Dieses Objekt gehört zum Spiel selbst. Blende es nur aus, wenn du sicher bist: Eine Quest könnte es brauchen.",
    workObjectHide: "Dieses Objekt ausblenden",
    workObjectShow: "Wieder zeigen",
    workProgress: (decided, total) => `${decided} von ${total} Stellen entschieden`,
    workGroupBy: "Ordnen nach",
    workByRegion: "Region",
    workByMods: "Mod-Paar",
    workHideDecided: "Entschiedene ausblenden",
    workInterior: (cell) => `Innen: ${cell}`,
    workDepth: (metres) =>
      `${metres.toLocaleString("de", { maximumFractionDigits: 1 })} m ineinander`,
    workHidden: (plugin) => `${plugin} ausgeblendet`,
    workKept: "beide bleiben",
    workOpen: "offen",
    workSpotOf: (index, total) => `Stelle ${index} von ${total}`,
    workCannotHide: "Anderes verweist darauf; es lässt sich nicht gefahrlos ausblenden.",
    workReset: "Zurücksetzen",
    workPrevious: "Zurück",
    workNext: "Weiter",
    workNextOpen: "Nächste offene",
    workEverywhere: (plugin) => `Überall ${plugin} ausblenden`,
    workKeepEverywhere: "Überall beide behalten",
    workKeys: "Tasten: ← → Stellen · 1, 2 ausblenden · 0 beide behalten · V Markierungen",
    workToPatch: (objects) =>
      objects === 1 ? "Zum Patch (1 Objekt)" : `Zum Patch (${objects} Objekte)`,
    workPickSpot: "Wähle links eine Stelle.",
    workInside:
      "Diese Stelle liegt in einem Gebäude oder einer Höhle; dafür gibt es noch keine 3D-Ansicht.",
    workEmpty: "Keine Kollisionen zu entscheiden.",
    workMore: "Mod gegen Mod",
    workGhost: "Rot und durchscheinend: was der Patch ausblenden würde.",
    workThisSpot: "diese Stelle",
    workSearch: "Orte, Mods, Objekte suchen…",
    workHideVerb: "Ausblenden",
    workKeepVerb: "Behalten",
    anchorAsking: "Prüfe, woran es hängt…",
    anchorLocked: "Ohne dieses Objekt geht es im Spiel nicht:",
    anchorCareful:
      "Das Spiel könnte es vermissen. Blende es nur aus, wenn dir das Folgende egal ist:",
    anchorHideAnyway: "Trotzdem ausblenden",
    anchorWhy: "Warum gebraucht",
    anchorGone: "(das ist selbst weg, zählt also nicht)",
    anchorOff: "(das ist ausgeschaltet)",
    anchorKinds: {
      "teleport-door": (other) => `Eine Tür mit Durchgang${other ? ` nach ${other}` : ""}`,
      "quest-alias": (other) => `Eine Quest braucht es${other ? ` (${other})` : ""}`,
      "large-reference": () => "Teil der Fernansicht der Welt",
      "activate-parent": (other) => `Es löst ${other || "andere Objekte"} aus`,
      "enable-parent": (other) =>
        `Es schaltet andere Objekte ein und aus${other ? `, etwa ${other}` : ""}; die bleiben, wie sie sind`,
      "linked-ref": (other, keyword) =>
        `${other || "Ein anderes Objekt"} ist damit verknüpft${keyword ? ` (${keyword})` : ""}`,
      "location-role": (other) => `Es hat eine Rolle an einem Ort${other ? ` (${other})` : ""}`,
      package: (other) => `Ein Tagesablauf einer Figur nutzt es${other ? ` (${other})` : ""}`,
      script: (other) => `Ein Skript nennt es${other ? ` (${other})` : ""}`,
      "visible-when-distant": () =>
        "Wird aus der Ferne gezeigt; die Fernansicht zeigt es weiter, bis sie neu erzeugt wird",
      persistent: () => "Das Spiel hält es geladen; nichts gefunden, das es noch nutzt",
    },
    workSortBy: "Reihenfolge",
    workSortOpen: "Meiste offene",
    workSortSpots: "Meiste Stellen",
    workWinsEverywhere: (mod) => `✓ ${mod} gewinnt überall`,
    workDuelWinner: (mod) => `Duell: ${mod} gewinnt`,
    duelFromCollisions: (mod) =>
      `In den Kollisionen hast du ${mod} an jeder Stelle gewinnen lassen. Hier übernehmen, auch für die Dateien?`,
    duelTakeOver: "Übernehmen",
    workNeeded: "Vom Spiel gebraucht",
    workBothNeeded:
      "Das Spiel braucht beide Objekte (zum Beispiel Türen mit Ladebildschirm oder Dinge, auf die Skripte verweisen). Hier lässt sich nichts gefahrlos ausblenden; beide zu behalten ist meist richtig.",
    workHideObject: "Dieses Objekt hier ausblenden",
    workDecideHere: "In der Arbeitsliste entscheiden",
    workAfterDecide: "Vorspulen",
    workAfterDecideHint: "Nach dem Entscheiden zur nächsten offenen Stelle",
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
      vault: "Vault",
      ruin: "Ruinen",
      factory: "Fabrik",
      station: "Bahnhof",
      building: "Gebäude",
      "ayleid-ruin": "Ayleiden-Ruine",
      "oblivion-gate": "Oblivion-Tor",
      inn: "Gasthaus",
      other: "Sonstiges",
    },
    placesCollisions: "Zusammenstöße von Mods",
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
