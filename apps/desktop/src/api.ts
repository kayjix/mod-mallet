// The frontend's view of the Rust side.
//
// Each function here corresponds to a command in src-tauri/src/lib.rs. The
// result types are generated from the Rust types, so a change on one side
// that the other does not follow fails the type check.

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { AreaLand } from "./generated/area-land";
import type { AreaObjects } from "./generated/area-objects";
import type { AreaPaths } from "./generated/area-paths";
import type { AssetVersions } from "./generated/asset-versions";
import type { Catalogue } from "./generated/catalogue";
import type { CreatedMoves } from "./generated/created-moves";
import type { CreatedPaths } from "./generated/created-paths";
import type { DoorReach } from "./generated/door-reach";
import type { FileComparison } from "./generated/file-comparison";
import type { FilePreview } from "./generated/file-preview";
import type { GameChoice } from "./generated/game-choice";
import type { HiddenWalkways } from "./generated/hidden-walkways";
import type { LoadedPatch } from "./generated/loaded-patch";
import type { LodMesh } from "./generated/lod-mesh";
import type { LodTiles } from "./generated/lod-tiles";
import type { MapImage } from "./generated/map-image";
import type { MeshThumbnail } from "./generated/mesh-thumbnail";
import type { MeshThumbnailRequest } from "./generated/mesh-thumbnail-request";
import type { MeshView } from "./generated/mesh-view";
import type { ObjectAnchors } from "./generated/object-anchors";
import type { PatchBook } from "./generated/patch-book";
import type { RenderedLink } from "./generated/rendered-link";
import type { RenderedOutcome } from "./generated/rendered-outcome";
import type { RenderedPlaces, ScanLocation } from "./generated/rendered-places";
import type {
  Action,
  Moves,
  ObjectRef,
  Patch,
  PathRepairs,
  RecordChange,
  RenderedReport,
} from "./generated/rendered-report";
import type { RenderedToolbox } from "./generated/rendered-toolbox";
import type { ScanProgress } from "./generated/scan-progress";
import type { SpotsReach } from "./generated/spots-reach";
import type { WalkwayReach } from "./generated/walkway-reach";
import type { WalkwayRepair } from "./generated/walkway-repair";
import type { Lang } from "./i18n";

/** The places the user named, by what they are for. */
export type NamedPlaces = Partial<Record<ScanLocation["role"], string>>;

export interface Api {
  /** Examines the setup and returns what is wrong with it. */
  scan(lang: Lang): Promise<RenderedReport>;
  /**
   * Calls `listener` while a scan runs, with the step it is in and how far it
   * is. Returns the function that stops listening.
   */
  onScanProgress(listener: (progress: ScanProgress) => void): () => void;
  /** The result of the last scan, or `null` if there is none yet. */
  lastReport(lang: Lang): Promise<RenderedReport | null>;
  /**
   * Every thing of the setup worth looking at (ADR 0054): weapons, armour,
   * characters and other things, with the outfits they form. The first call
   * for a setup reads every plugin and takes a few seconds. Null if the setup
   * cannot be read.
   */
  catalogue(lang: Lang): Promise<Catalogue | null>;
  /** Every version of a thing of the catalogue: what the game and each mod make it look like. */
  assetVersions(models: string[]): Promise<AssetVersions | null>;
  /** The mesh at `path` as one version of a thing makes it look. */
  assetView(path: string, version: string): Promise<MeshView | null>;
  /** The versions of things the user put into the game, by thing. */
  assetChoices(): Promise<Record<string, string>>;
  /** Remembers the version of a thing put into the game, or forgets it with null. */
  setAssetChoice(id: string, version: string | null): Promise<Record<string, string>>;
  /** Hands an action to the mod manager and says what became of it. */
  runAction(action: Action, lang: Lang): Promise<RenderedOutcome>;
  /** Whether the link to the mod manager is installed and answering. */
  managerLink(lang: Lang): Promise<RenderedLink>;
  /** Installs the link into the mod manager; resolves to a sentence about it. */
  installManagerLink(lang: Lang): Promise<string>;
  /** Asks Vortex to list Mod Mallet among its tools, or to take it off the list. */
  setVortexTool(add: boolean, lang: Lang): Promise<RenderedOutcome>;
  /** The places a scan would read, and whether the user has agreed to them. */
  places(lang: Lang): Promise<RenderedPlaces>;
  /**
   * The places a scan would read with another mod manager, and its instance
   * if one was picked. Nothing is kept until the places are confirmed.
   */
  placesForManager(manager: string, instance: string | null, lang: Lang): Promise<RenderedPlaces>;
  /**
   * Remembers the places the user named, the mod manager they chose (null
   * keeps the one before), and that they agreed to the rest.
   */
  confirmPlaces(named: NamedPlaces, manager: string | null, lang: Lang): Promise<RenderedPlaces>;
  /** The identities of the findings the user has set aside. */
  ignored(): Promise<string[]>;
  /** Sets a finding aside or brings it back; resolves to all set aside. */
  setIgnored(id: string, ignored: boolean): Promise<string[]>;
  /** A short text that changes whenever the setup changes. */
  setupFingerprint(): Promise<string>;
  /** Opens a mod's page in the browser. */
  openPage(url: string): Promise<void>;
  /** Writes `patch`, with an installer for its parts (ADR 0053); resolves to where it is. */
  createPatch(patch: Patch): Promise<string>;
  /** Writes the plugin that moves objects a little (ADR 0060); says what it moves and what not. */
  createMoves(moves: Moves): Promise<CreatedMoves>;
  /** Writes the plugin of repaired paths (ADR 0058) into Mod Mallet's own folder. */
  createPaths(repairs: PathRepairs): Promise<CreatedPaths>;
  /** Writes the report of what the patch hides next to it, and returns where it is. */
  savePatchReport(text: string): Promise<string>;
  /** Shows one of Mod Mallet's own folders in the file explorer. */
  openFolder(which: "patches" | "logs"): Promise<void>;
  /** Shows a plugin or an archive in the game's Data folder in the file explorer, selected (ADR 0073). */
  showDataFile(name: string): Promise<void>;
  /** Where the app writes its log. */
  logFile(): Promise<string | null>;
  /** What the setup needs besides its mods, and how each of those stands. */
  toolbox(lang: Lang): Promise<RenderedToolbox>;
  /** Closes Vortex and starts it again. */
  restartManager(): Promise<void>;
  /** The picture of a game as a data address, if the mod manager has one. */
  gameArt(game: string): Promise<string | null>;
  /** A picture of a worldspace from the game's terrain textures, if it has them. */
  mapImage(world: string): Promise<MapImage | null>;
  /** Every version of a file of the setup, as pictures where it is a texture. */
  filePreview(path: string): Promise<FilePreview | null>;
  /**
   * Every version of files other than textures and meshes, compared, in the
   * order of `paths`; `mods` names the mod manager's folders of the mods
   * concerned, whose loose copies kept back are read too.
   */
  compareFiles(paths: string[], mods: string[]): Promise<FileComparison[] | null>;
  /** A version of a mesh, ready to draw; the one in the game unless `version` names another. */
  meshView(path: string, version: number | null): Promise<MeshView | null>;
  /** Small pictures of mesh versions for lists, `side` pixels square, in the order asked. */
  meshThumbnails(requests: MeshThumbnailRequest[], side: number): Promise<MeshThumbnail[]>;
  /** Everything placed within `radius` game units of a point. */
  areaObjects(world: string, x: number, y: number, radius: number): Promise<AreaObjects | null>;
  /** The game's ground of the cells within `radius` game units of a point. */
  areaLand(world: string, x: number, y: number, radius: number): Promise<AreaLand | null>;
  /** The game's ground of every cell of a small world within its bounds, for a world without level of detail. */
  worldLand(
    world: string,
    bounds: { west: number; south: number; east: number; north: number },
  ): Promise<AreaLand | null>;
  /** Everything placed in an interior cell, by its editor ID. */
  interiorObjects(cell: string): Promise<AreaObjects | null>;
  /** A mesh of the setup in its own units, textures named by path. */
  areaMesh(path: string): Promise<LodMesh | null>;
  /** A texture of the setup, no larger than `maxSide`. */
  areaTexture(path: string, maxSide: number): Promise<string | null>;
  /**
   * Why the game relies on each placed object, and whether it can be hidden
   * (ADR 0046), in the same order. `worldOrCell` is the worldspace by internal
   * name, or the interior cell. The first call for a setup reads every plugin
   * and takes a few seconds; calling with no objects does only that. Null if
   * the setup cannot be read.
   */
  objectAnchors(worldOrCell: string, objects: ObjectRef[]): Promise<ObjectAnchors[] | null>;
  /**
   * The paths characters walk on that run over `objects`, which are about to
   * be hidden: in the worldspace `worldOrCell` near the point `x`, `y`, or,
   * with `x` and `y` null, in that interior cell. Every plugin's version of
   * each path is given, with whether the game uses it. Null if the setup
   * cannot be read.
   */
  hiddenWalkways(
    worldOrCell: string,
    x: number | null,
    y: number | null,
    objects: ObjectRef[],
  ): Promise<HiddenWalkways | null>;
  /**
   * Whether characters still reach where they go once `objects` are hidden
   * and the paths over them cut out (ADR 0059), asked as `hiddenWalkways`.
   * `null` if the setup cannot be read.
   */
  walkwayReach(
    worldOrCell: string,
    x: number | null,
    y: number | null,
    objects: ObjectRef[],
  ): Promise<WalkwayReach | null>;
  /**
   * Whether characters reach the door `keep`, kept where stacked doors are
   * merged, and walk through it once the patch hides `hidden` and makes the
   * plan's `changes` (ADR 0088). `null` if the setup cannot be read.
   */
  doorReach(
    worldOrCell: string,
    x: number | null,
    y: number | null,
    hidden: ObjectRef[],
    keep: ObjectRef,
    changes: RecordChange[],
  ): Promise<DoorReach | null>;
  /** What repairing the paths over objects about to be hidden does (ADR 0058). */
  walkwayRepair(
    worldOrCell: string,
    x: number | null,
    y: number | null,
    objects: ObjectRef[],
  ): Promise<WalkwayRepair | null>;
  /**
   * Whether characters reach where they go once the paths over `objects`
   * are repaired (ADR 0058, 0059), asked as `walkwayRepair`.
   */
  walkwayRepairReach(
    worldOrCell: string,
    x: number | null,
    y: number | null,
    objects: ObjectRef[],
  ): Promise<WalkwayReach | null>;
  /**
   * Whether characters can reach `spots`, places where they do something,
   * where they stand or would stand after a move (ADR 0069), with the
   * paths over `hidden` cut out. `null` if the setup cannot be read.
   */
  spotsReach(
    worldOrCell: string,
    x: number | null,
    y: number | null,
    spots: { object: ObjectRef; at: [number, number, number] }[],
    hidden: ObjectRef[],
  ): Promise<SpotsReach | null>;
  /**
   * Where characters walk in the worldspace `worldOrCell` within `radius`
   * game units of `x`, `y`, or, with `x` and `y` null, in that interior cell
   * (ADR 0071): who made each piece of the paths, and where they run over
   * `hidden`. `null` if the setup cannot be read.
   */
  areaPaths(
    worldOrCell: string,
    x: number | null,
    y: number | null,
    radius: number,
    hidden: ObjectRef[],
  ): Promise<AreaPaths | null>;
  /** The blocks of the game's level of detail a worldspace has. */
  lodTiles(world: string): Promise<LodTiles>;
  /** The land of a block of level of detail, or with `objects` its objects. */
  lodMesh(
    world: string,
    level: number,
    x: number,
    y: number,
    objects: boolean,
  ): Promise<LodMesh | null>;
  /** A texture of the level of detail, no larger than `maxSide`. */
  lodTexture(world: string, path: string, maxSide: number): Promise<string | null>;
  /** The mods in the order the user likes them, the favourite first. */
  ranking(): Promise<string[]>;
  /** Remembers the order the user likes the mods in. */
  setRanking(ranking: string[]): Promise<string[]>;
  /**
   * What the user chose for the patch that hides objects, by finding, spot
   * or object: the plugin to hide, or null to keep both. Null as a whole
   * while nothing has been chosen.
   */
  patchChoices(): Promise<Record<string, string | null> | null>;
  /** Remembers what the user chose for the patch that hides objects. */
  setPatchChoices(choices: Record<string, string | null>): Promise<void>;
  /** The user's patches for the game in use (ADR 0070); null until made. */
  patchBook(): Promise<PatchBook | null>;
  /** Remembers the user's patches. */
  setPatchBook(book: PatchBook): Promise<void>;
  /** Reads the patch in an archive the user picked; nothing is installed. */
  loadPatchArchive(name: string, bytes: number[]): Promise<LoadedPatch>;
  /** The sides the user picked in duels between mods, by duel. */
  decisions(): Promise<Record<string, number>>;
  /** Remembers the side picked in a duel, or forgets it with null. */
  setDecision(id: string, side: number | null): Promise<Record<string, number>>;
  /** The games Mod Mallet knows, where each is, and which one is looked at. */
  games(): Promise<GameChoice>;
  /** Makes a game the one to look at; each game keeps its own settings. */
  selectGame(game: string): Promise<GameChoice>;
}

const tauriApi: Api = {
  scan: (lang) => invoke<RenderedReport>("scan", { lang }),
  onScanProgress: (listener) => {
    const stopped = listen<ScanProgress>("scan-progress", (event) => listener(event.payload));
    return () => {
      void stopped.then((stop) => stop());
    };
  },
  lastReport: (lang) => invoke<RenderedReport | null>("last_report", { lang }),
  runAction: (action, lang) => invoke<RenderedOutcome>("run_action", { action, lang }),
  managerLink: (lang) => invoke<RenderedLink>("manager_link", { lang }),
  installManagerLink: (lang) => invoke<string>("install_manager_link", { lang }),
  setVortexTool: (add, lang) => invoke<RenderedOutcome>("set_vortex_tool", { add, lang }),
  places: (lang) => invoke<RenderedPlaces>("places", { lang }),
  placesForManager: (manager, instance, lang) =>
    invoke<RenderedPlaces>("places_for_manager", { manager, instance, lang }),
  confirmPlaces: (named, manager, lang) =>
    invoke<RenderedPlaces>("confirm_places", { named, manager, lang }),
  ignored: () => invoke<string[]>("ignored"),
  setIgnored: (id, ignored) => invoke<string[]>("set_ignored", { id, ignored }),
  setupFingerprint: () => invoke<string>("setup_fingerprint"),
  openPage: (url) => invoke<void>("open_page", { url }),
  createPatch: (patch) => invoke<string>("create_patch", { patch }),
  createMoves: (moves) => invoke<CreatedMoves>("create_moves", { moves }),
  createPaths: (repairs) => invoke<CreatedPaths>("create_paths", { repairs }),
  savePatchReport: (text) => invoke<string>("save_patch_report", { text }),
  openFolder: (which) => invoke<void>("open_folder", { which }),
  showDataFile: (name) => invoke<void>("show_data_file", { name }),
  logFile: () => invoke<string | null>("log_file"),
  toolbox: (lang) => invoke<RenderedToolbox>("toolbox", { lang }),
  restartManager: () => invoke<void>("restart_manager"),
  gameArt: (game) => invoke<string | null>("game_art", { game }),
  mapImage: (world) => invoke<MapImage | null>("map_image", { world }),
  filePreview: (path) => invoke<FilePreview | null>("file_preview", { path }),
  compareFiles: (paths, mods) => invoke<FileComparison[] | null>("compare_files", { paths, mods }),
  meshView: (path, version) => invoke<MeshView | null>("mesh_view", { path, version }),
  meshThumbnails: (requests, side) =>
    invoke<MeshThumbnail[]>("mesh_thumbnails", { requests, side }),
  catalogue: (lang) => invoke<Catalogue | null>("catalogue", { lang }),
  assetVersions: (models) => invoke<AssetVersions | null>("asset_versions", { models }),
  assetView: (path, version) => invoke<MeshView | null>("asset_view", { path, version }),
  assetChoices: () => invoke<Record<string, string>>("asset_choices"),
  setAssetChoice: (id, version) =>
    invoke<Record<string, string>>("set_asset_choice", { id, version }),
  areaObjects: (world, x, y, radius) =>
    invoke<AreaObjects | null>("area_objects", { world, x, y, radius }),
  interiorObjects: (cell) => invoke<AreaObjects | null>("interior_objects", { cell }),
  worldLand: (world, { west, south, east, north }) =>
    invoke<AreaLand | null>("world_land", { world, west, south, east, north }),
  areaLand: (world, x, y, radius) => invoke<AreaLand | null>("area_land", { world, x, y, radius }),
  areaMesh: (path) => invoke<LodMesh | null>("area_mesh", { path }),
  areaTexture: (path, maxSide) => invoke<string | null>("area_texture", { path, maxSide }),
  objectAnchors: (worldOrCell, objects) =>
    invoke<ObjectAnchors[] | null>("object_anchors", { worldOrCell, objects }),
  hiddenWalkways: (worldOrCell, x, y, objects) =>
    invoke<HiddenWalkways | null>("hidden_walkways", { worldOrCell, x, y, objects }),
  walkwayReach: (worldOrCell, x, y, objects) =>
    invoke<WalkwayReach | null>("walkway_reach", { worldOrCell, x, y, objects }),
  doorReach: (worldOrCell, x, y, hidden, keep, changes) =>
    invoke<DoorReach | null>("door_reach", { worldOrCell, x, y, hidden, keep, changes }),
  walkwayRepair: (worldOrCell, x, y, objects) =>
    invoke<WalkwayRepair | null>("walkway_repair", { worldOrCell, x, y, objects }),
  walkwayRepairReach: (worldOrCell, x, y, objects) =>
    invoke<WalkwayReach | null>("walkway_repair_reach", { worldOrCell, x, y, objects }),
  spotsReach: (worldOrCell, x, y, spots, hidden) =>
    invoke<SpotsReach | null>("spots_reach", { worldOrCell, x, y, spots, hidden }),
  areaPaths: (worldOrCell, x, y, radius, hidden) =>
    invoke<AreaPaths | null>("area_paths", { worldOrCell, x, y, radius, hidden }),
  lodTiles: (world) => invoke<LodTiles>("lod_tiles", { world }),
  lodMesh: (world, level, x, y, objects) =>
    invoke<LodMesh | null>("lod_mesh", { world, level, x, y, objects }),
  lodTexture: (world, path, maxSide) =>
    invoke<string | null>("lod_texture", { world, path, maxSide }),
  ranking: () => invoke<string[]>("ranking"),
  setRanking: (ranking) => invoke<string[]>("set_ranking", { ranking }),
  patchChoices: () => invoke<Record<string, string | null> | null>("patch_choices"),
  setPatchChoices: (choices) => invoke<void>("set_patch_choices", { choices }),
  patchBook: () => invoke<PatchBook | null>("patch_book"),
  setPatchBook: (book) => invoke<void>("set_patch_book", { book }),
  loadPatchArchive: (name, bytes) => invoke<LoadedPatch>("load_patch_archive", { name, bytes }),
  decisions: () => invoke<Record<string, number>>("decisions"),
  setDecision: (id, side) => invoke<Record<string, number>>("set_decision", { id, side }),
  games: () => invoke<GameChoice>("games"),
  selectGame: (game) => invoke<GameChoice>("select_game", { game }),
};

function runsInsideTauri(): boolean {
  return "__TAURI_INTERNALS__" in window;
}

/**
 * The app's backend.
 *
 * Opened in a plain browser with `npm run dev`, the frontend has no Rust
 * side to talk to. It then shows a made-up report, which is enough to work
 * on layout and wording without building the app.
 */
export async function loadApi(): Promise<Api> {
  if (import.meta.env.DEV && !runsInsideTauri()) {
    const { sampleApi } = await import("./dev/sample-api");
    return sampleApi;
  }
  return tauriApi;
}
