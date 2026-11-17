// The places of the map (ADR 0031): how they are drawn, filtered and
// related to collisions. Shared by the flat map and the 3D relief.

import type { MapImage } from "./generated/map-image";
import type { MapMarker, MapSpot, MapTerrain, MapWorld } from "./generated/rendered-report";
import type { PlaceKind, PlaceOrigin } from "./i18n";

/** What is picked or pointed at on the map. */
export type Picked =
  | { type: "cell"; x: number; y: number }
  | { type: "place"; marker: MapMarker }
  | { type: "spot"; spot: MapSpot };

/** The game's picture of a worldspace, loaded. */
export interface Picture {
  meta: MapImage;
  image: HTMLImageElement;
}

/** Whether two picks are the same thing. */
export function samePick(a: Picked | null, b: Picked | null): boolean {
  if (a === null || b === null || a.type !== b.type) {
    return false;
  }
  if (a.type === "cell" && b.type === "cell") {
    return a.x === b.x && a.y === b.y;
  }
  if (a.type === "place" && b.type === "place") {
    return a.marker === b.marker;
  }
  return a.type === "spot" && b.type === "spot" && a.spot === b.spot;
}

/** A name to write on the map at `x`, `y` (its bottom centre). */
export interface Label {
  x: number;
  y: number;
  text: string;
  strong: boolean;
}

/** Names on the map, strong ones first, without letting them cover each other. */
export function drawLabels(context: CanvasRenderingContext2D, labels: Label[]) {
  const taken: { left: number; top: number; right: number; bottom: number }[] = [];
  const sorted = [...labels].sort((a, b) => Number(b.strong) - Number(a.strong));
  context.textAlign = "center";
  context.textBaseline = "bottom";
  for (const label of sorted) {
    context.font = `${label.strong ? 600 : 400} ${label.strong ? 13 : 11.5}px system-ui, sans-serif`;
    const width = context.measureText(label.text).width;
    const box = {
      left: label.x - width / 2 - 3,
      right: label.x + width / 2 + 3,
      top: label.y - 15,
      bottom: label.y + 1,
    };
    const covered = taken.some(
      (other) =>
        box.left < other.right &&
        box.right > other.left &&
        box.top < other.bottom &&
        box.bottom > other.top,
    );
    if (covered) {
      continue;
    }
    taken.push(box);
    context.lineWidth = 3;
    context.strokeStyle = "rgb(0 0 0 / 55%)";
    context.strokeText(label.text, label.x, label.y);
    context.fillStyle = "#ffffff";
    context.fillText(label.text, label.x, label.y);
  }
}

/** Game units per metre: one unit is about 1.43 centimetres. */
const UNITS_PER_METRE = 70;

/** Game units per cell side. */
export const CELL = 4096;

/** The land of a worldspace, one value per sample, rows from north to south. */
export interface Land {
  /** Samples from west to east. */
  width: number;
  /** Samples from north to south. */
  height: number;
  /** Height of the ground or the water surface, in game units. */
  heights: Float32Array;
  /** 0 where nothing is known, 1 water, 2 land. */
  kinds: Uint8Array;
}

/** The report's heights and water, read for drawing. */
export function readLand(terrain: MapTerrain): Land | null {
  const width = terrain.columns * terrain.samples;
  const height = terrain.rows * terrain.samples;
  const values = atob(terrain.heights);
  const water = atob(terrain.water ?? "");
  if (values.length !== width * height * 2 || water.length !== width * height) {
    return null;
  }
  const span = Math.max(1, terrain.highest - terrain.lowest);
  const heights = new Float32Array(width * height);
  const kinds = new Uint8Array(width * height);
  for (let index = 0; index < width * height; index++) {
    const value = values.charCodeAt(index * 2) | (values.charCodeAt(index * 2 + 1) << 8);
    if (value === 0) {
      heights[index] = terrain.lowest;
      continue;
    }
    heights[index] = terrain.lowest + ((value - 1) / 65534) * span;
    kinds[index] = water.charCodeAt(index) === 1 ? 1 : 2;
  }
  return { width, height, heights, kinds };
}

/** The origins, in the order the filter lists them. */
export const ORIGINS: PlaceOrigin[] = ["game", "added", "changed", "removed"];

/**
 * The outline a place is drawn with: settlements square, dungeons and
 * strongholds as a triangle, nature and landmarks round.
 */
export type Shape = "square" | "triangle" | "circle";

const SHAPES: Record<PlaceKind, Shape> = {
  city: "square",
  town: "square",
  settlement: "square",
  castle: "square",
  farm: "square",
  "wood-mill": "square",
  "wheat-mill": "square",
  smelter: "square",
  stable: "square",
  shack: "square",
  "nordic-dwelling": "square",
  docks: "square",
  lighthouse: "square",
  "orc-stronghold": "square",
  building: "square",
  inn: "square",
  station: "square",
  cave: "triangle",
  camp: "triangle",
  fort: "triangle",
  "nordic-ruin": "triangle",
  "dwemer-ruin": "triangle",
  shipwreck: "triangle",
  "dragon-lair": "triangle",
  mine: "triangle",
  "imperial-camp": "triangle",
  "stormcloak-camp": "triangle",
  "imperial-tower": "triangle",
  "nordic-tower": "triangle",
  "giant-camp": "triangle",
  vault: "triangle",
  ruin: "triangle",
  factory: "triangle",
  "ayleid-ruin": "triangle",
  "oblivion-gate": "triangle",
  grove: "circle",
  landmark: "circle",
  doomstone: "circle",
  clearing: "circle",
  pass: "circle",
  shrine: "circle",
  "daedric-shrine": "circle",
  passage: "circle",
  other: "circle",
};

export const shapeOf = (kind: PlaceKind): Shape => SHAPES[kind];

/** Places big enough to be named on the map from afar. */
export const isMajor = (kind: PlaceKind) => kind === "city" || kind === "town" || kind === "castle";

/** The stylesheet variable that colours places of an origin. */
export const originVariable = (origin: PlaceOrigin) => `--place-${origin}`;

/** Which places are shown. */
export interface PlaceFilter {
  origins: Set<PlaceOrigin>;
  kinds: Set<PlaceKind>;
  /** Whether collisions of mods are shown. */
  collisions: boolean;
}

/** Every place and every collision of `world` shown. */
export function startingFilter(world: MapWorld | undefined): PlaceFilter {
  const kinds = new Set<PlaceKind>((world?.markers ?? []).map((marker) => marker.kind));
  return { origins: new Set(ORIGINS), kinds, collisions: true };
}

export const isShown = (marker: MapMarker, filter: PlaceFilter) =>
  filter.origins.has(marker.origin) && filter.kinds.has(marker.kind);

/** The distance between two positions on the map, in metres. */
export function metres(a: readonly number[], b: readonly number[]): number {
  return Math.hypot((a[0] ?? 0) - (b[0] ?? 0), (a[1] ?? 0) - (b[1] ?? 0)) / UNITS_PER_METRE;
}

/** Where a collision is: its exact point, or the middle of its cell. */
export function spotPosition(spot: MapSpot): [number, number, number] | null {
  if (spot.position) {
    return spot.position;
  }
  if (spot.location.type !== "exterior") {
    return null;
  }
  return [(spot.location.x + 0.5) * CELL, (spot.location.y + 0.5) * CELL, 0];
}

/** The `count` places nearest to `position`, with their distance in metres. */
export function nearest(
  markers: MapMarker[],
  position: readonly number[],
  count: number,
  except?: MapMarker,
): { marker: MapMarker; metres: number }[] {
  return markers
    .filter((marker) => marker !== except && marker.origin !== "removed")
    .map((marker) => ({ marker, metres: metres(marker.position, position) }))
    .sort((a, b) => a.metres - b.metres)
    .slice(0, count);
}

/** The collisions of `world` within `radius` metres of `position`, nearest first. */
export function spotsNear(
  spots: MapSpot[],
  world: string,
  position: readonly number[],
  radius: number,
): { spot: MapSpot; metres: number }[] {
  return spots
    .filter((spot) => spot.location.type === "exterior" && spot.location.world === world)
    .flatMap((spot) => {
      const at = spotPosition(spot);
      return at === null ? [] : [{ spot, metres: metres(at, position) }];
    })
    .filter((found) => found.metres <= radius)
    .sort((a, b) => a.metres - b.metres);
}

/** Draws a place's outline centred on `x`, `y` with radius `r`. */
export function tracePlace(
  context: CanvasRenderingContext2D,
  shape: Shape,
  x: number,
  y: number,
  r: number,
) {
  context.beginPath();
  if (shape === "square") {
    context.rect(x - r * 0.85, y - r * 0.85, r * 1.7, r * 1.7);
  } else if (shape === "triangle") {
    context.moveTo(x, y - r * 1.1);
    context.lineTo(x + r, y + r * 0.75);
    context.lineTo(x - r, y + r * 0.75);
    context.closePath();
  } else {
    context.arc(x, y, r, 0, Math.PI * 2);
  }
}

/** What kind a collision is (ADR 0039). */
export type SpotKind = NonNullable<MapSpot["kind"]>;

/** The kinds of collisions, the worst first. */
export const SPOT_KINDS: SpotKind[] = ["duplicate", "overlap", "touch"];

/** A spot's kind; spots of older scans overlap. */
export const kindOf = (spot: MapSpot): SpotKind => spot.kind ?? "overlap";

/**
 * The kinds of the collisions' worklist: a touch that crosses far counts
 * as deep (ADR 0040).
 */
export type WorkKind = SpotKind | "deep";

/** The colour of a kind of collision, as a CSS variable. */
export const kindVariable = (kind: WorkKind) => `--spot-${kind}`;

/** The worst kind among `spots`. */
export function worstKind(spots: MapSpot[]): SpotKind {
  return SPOT_KINDS.find((kind) => spots.some((spot) => kindOf(spot) === kind)) ?? "overlap";
}
