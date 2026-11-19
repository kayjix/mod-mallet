// The frontend's view of the Rust side.
//
// Each function here corresponds to a command in src-tauri/src/lib.rs. The
// result types are generated from the Rust types, so a change on one side
// that the other does not follow fails the type check.

import { invoke } from "@tauri-apps/api/core";
import type { AreaLand } from "./generated/area-land";
import type { AreaObjects } from "./generated/area-objects";
import type { FilePreview } from "./generated/file-preview";
import type { GameChoice } from "./generated/game-choice";
import type { HiddenWalkways } from "./generated/hidden-walkways";
import type { LodMesh } from "./generated/lod-mesh";
import type { LodTiles } from "./generated/lod-tiles";
import type { MapImage } from "./generated/map-image";
import type { MeshView } from "./generated/mesh-view";
import type { ObjectAnchors } from "./generated/object-anchors";
import type { RenderedLink } from "./generated/rendered-link";
import type { RenderedOutcome } from "./generated/rendered-outcome";
import type { RenderedPlaces, ScanLocation } from "./generated/rendered-places";
import type { Action, ObjectRef, Patch, RenderedReport } from "./generated/rendered-report";
import type { RenderedToolbox } from "./generated/rendered-toolbox";
import type { Lang } from "./i18n";

/** The places the user named, by what they are for. */
export type NamedPlaces = Partial<Record<ScanLocation["role"], string>>;

export interface Api {
  /** Examines the setup and returns what is wrong with it. */
  scan(lang: Lang): Promise<RenderedReport>;
  /** The result of the last scan, or `null` if there is none yet. */
  lastReport(lang: Lang): Promise<RenderedReport | null>;
  /** Hands an action to the mod manager and says what became of it. */
  runAction(action: Action, lang: Lang): Promise<RenderedOutcome>;
  /** Whether the link to the mod manager is installed and answering. */
  managerLink(lang: Lang): Promise<RenderedLink>;
  /** Installs the link into the mod manager; resolves to a sentence about it. */
  installManagerLink(lang: Lang): Promise<string>;
  /** The places a scan would read, and whether the user has agreed to them. */
  places(lang: Lang): Promise<RenderedPlaces>;
  /** Remembers the places the user named and that they agreed to the rest. */
  confirmPlaces(named: NamedPlaces, lang: Lang): Promise<RenderedPlaces>;
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
  /** Writes the report of what the patch hides next to it, and returns where it is. */
  savePatchReport(text: string): Promise<string>;
  /** Shows one of Mod Mallet's own folders in the file explorer. */
  openFolder(which: "patches" | "logs"): Promise<void>;
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
  /** A version of a mesh, ready to draw; the one in the game unless `version` names another. */
  meshView(path: string, version: number | null): Promise<MeshView | null>;
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
  lastReport: (lang) => invoke<RenderedReport | null>("last_report", { lang }),
  runAction: (action, lang) => invoke<RenderedOutcome>("run_action", { action, lang }),
  managerLink: (lang) => invoke<RenderedLink>("manager_link", { lang }),
  installManagerLink: (lang) => invoke<string>("install_manager_link", { lang }),
  places: (lang) => invoke<RenderedPlaces>("places", { lang }),
  confirmPlaces: (named, lang) => invoke<RenderedPlaces>("confirm_places", { named, lang }),
  ignored: () => invoke<string[]>("ignored"),
  setIgnored: (id, ignored) => invoke<string[]>("set_ignored", { id, ignored }),
  setupFingerprint: () => invoke<string>("setup_fingerprint"),
  openPage: (url) => invoke<void>("open_page", { url }),
  createPatch: (patch) => invoke<string>("create_patch", { patch }),
  savePatchReport: (text) => invoke<string>("save_patch_report", { text }),
  openFolder: (which) => invoke<void>("open_folder", { which }),
  logFile: () => invoke<string | null>("log_file"),
  toolbox: (lang) => invoke<RenderedToolbox>("toolbox", { lang }),
  restartManager: () => invoke<void>("restart_manager"),
  gameArt: (game) => invoke<string | null>("game_art", { game }),
  mapImage: (world) => invoke<MapImage | null>("map_image", { world }),
  filePreview: (path) => invoke<FilePreview | null>("file_preview", { path }),
  meshView: (path, version) => invoke<MeshView | null>("mesh_view", { path, version }),
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
  lodTiles: (world) => invoke<LodTiles>("lod_tiles", { world }),
  lodMesh: (world, level, x, y, objects) =>
    invoke<LodMesh | null>("lod_mesh", { world, level, x, y, objects }),
  lodTexture: (world, path, maxSide) =>
    invoke<string | null>("lod_texture", { world, path, maxSide }),
  ranking: () => invoke<string[]>("ranking"),
  setRanking: (ranking) => invoke<string[]>("set_ranking", { ranking }),
  patchChoices: () => invoke<Record<string, string | null> | null>("patch_choices"),
  setPatchChoices: (choices) => invoke<void>("set_patch_choices", { choices }),
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
