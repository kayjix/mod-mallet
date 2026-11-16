// The frontend's view of the Rust side.
//
// Each function here corresponds to a command in src-tauri/src/lib.rs. The
// result types are generated from the Rust types, so a change on one side
// that the other does not follow fails the type check.

import { invoke } from "@tauri-apps/api/core";
import type { FilePreview } from "./generated/file-preview";
import type { MapImage } from "./generated/map-image";
import type { MeshView } from "./generated/mesh-view";
import type { RenderedLink } from "./generated/rendered-link";
import type { RenderedOutcome } from "./generated/rendered-outcome";
import type { RenderedPlaces, ScanLocation } from "./generated/rendered-places";
import type { Action, ObjectRef, RenderedReport } from "./generated/rendered-report";
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
  /** Writes the patch that hides `objects`; resolves to where it is. */
  createPatch(objects: ObjectRef[]): Promise<string>;
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
  /** The mods in the order the user likes them, the favourite first. */
  ranking(): Promise<string[]>;
  /** Remembers the order the user likes the mods in. */
  setRanking(ranking: string[]): Promise<string[]>;
  /** The sides the user picked in duels between mods, by duel. */
  decisions(): Promise<Record<string, number>>;
  /** Remembers the side picked in a duel, or forgets it with null. */
  setDecision(id: string, side: number | null): Promise<Record<string, number>>;
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
  createPatch: (objects) => invoke<string>("create_patch", { objects }),
  openFolder: (which) => invoke<void>("open_folder", { which }),
  logFile: () => invoke<string | null>("log_file"),
  toolbox: (lang) => invoke<RenderedToolbox>("toolbox", { lang }),
  restartManager: () => invoke<void>("restart_manager"),
  gameArt: (game) => invoke<string | null>("game_art", { game }),
  mapImage: (world) => invoke<MapImage | null>("map_image", { world }),
  filePreview: (path) => invoke<FilePreview | null>("file_preview", { path }),
  meshView: (path, version) => invoke<MeshView | null>("mesh_view", { path, version }),
  ranking: () => invoke<string[]>("ranking"),
  setRanking: (ranking) => invoke<string[]>("set_ranking", { ranking }),
  decisions: () => invoke<Record<string, number>>("decisions"),
  setDecision: (id, side) => invoke<Record<string, number>>("set_decision", { id, side }),
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
