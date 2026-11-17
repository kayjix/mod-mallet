import {
  type PointerEvent,
  type ReactNode,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { CopyButton, useSession } from "./Actions";
import type {
  MapCell,
  MapMarker,
  MapSpot,
  MapTerrain,
  MapWorld,
  WorldMap,
} from "./generated/rendered-report";
import type { PlaceKind, PlaceOrigin } from "./i18n";
import { MapView3D } from "./MapView3D";
import {
  CELL,
  drawLabels,
  isMajor,
  isShown,
  type Label,
  metres as metresBetween,
  nearest,
  ORIGINS,
  originVariable,
  type Picked,
  type Picture,
  type PlaceFilter,
  readLand,
  type Shape,
  samePick,
  shapeOf,
  spotPosition,
  spotsNear,
  startingFilter,
  tracePlace,
} from "./map-places";

interface MapViewProps {
  map: WorldMap;
  /** The headline of each finding, to say what a spot is about. */
  headlines: Map<string, string>;
  /** A spot to show when the view opens. */
  focus: MapSpot | null;
}

/** How the grid is placed on the canvas: pixels per cell and the offset. */
interface View {
  size: number;
  x: number;
  y: number;
}

const MIN_SIZE = 1;
const MAX_SIZE = 160;

/** How close to a place or spot a click has to land, in pixels. */
const HIT = 9;

/** The key of a cell in lookups. */
const key = (x: number, y: number) => `${x},${y}`;

const cellOf = (coordinate: number) => Math.floor(coordinate / CELL);

/**
 * Where mods place objects in a worldspace, cell by cell, over the shape
 * of the land, with the places of the game's map: which are the game's,
 * which mods added, changed or removed, and where objects of two mods
 * collide. Drawn flat or as a relief in 3D.
 */
export function MapView({ map, headlines, focus }: MapViewProps) {
  const { t, api } = useSession();
  const focusWorld =
    focus?.location.type === "exterior"
      ? map.worlds.findIndex((world) => world.name === (focus.location as { world: string }).world)
      : -1;
  const [worldIndex, setWorldIndex] = useState(Math.max(focusWorld, 0));
  const [owner, setOwner] = useState(-1);
  const [mode, setMode] = useState<"flat" | "relief">("flat");
  // Whether the cells with objects of mods are tinted; remembered.
  const [showCells, setShowCells] = useState(() => {
    try {
      return localStorage.getItem("mallet.mapCells") !== "off";
    } catch {
      return true;
    }
  });
  const changeCells = (next: boolean) => {
    setShowCells(next);
    try {
      localStorage.setItem("mallet.mapCells", next ? "on" : "off");
    } catch {
      // Only a convenience.
    }
  };
  const world = map.worlds[worldIndex];
  const [filter, setFilter] = useState<PlaceFilter>(() => startingFilter(world));
  const [picked, setPicked] = useState<Picked | null>(
    focusWorld >= 0 && focus !== null ? { type: "spot", spot: focus } : null,
  );
  const [hover, setHover] = useState<Picked | null>(null);

  // A new worldspace starts with all its places shown and nothing picked.
  const chooseWorld = (index: number) => {
    setWorldIndex(index);
    setFilter(startingFilter(map.worlds[index]));
    setPicked(null);
    setHover(null);
  };

  // The game's own picture of the land, when its terrain textures are there.
  const [picture, setPicture] = useState<Picture | null>(null);
  const worldName = world?.name;
  useEffect(() => {
    setPicture(null);
    if (worldName === undefined) {
      return;
    }
    let outdated = false;
    api
      .mapImage(worldName)
      .then((meta) => {
        if (meta === null || outdated) {
          return;
        }
        const image = new Image();
        image.onload = () => {
          if (!outdated) {
            setPicture({ meta, image });
          }
        };
        image.src = meta.image;
      })
      .catch(() => {});
    return () => {
      outdated = true;
    };
  }, [api, worldName]);

  const markers = world?.markers ?? [];
  const shownMarkers = useMemo(
    () => markers.filter((marker) => isShown(marker, filter)),
    [markers, filter],
  );
  const worldSpots = useMemo(
    () =>
      map.spots.filter(
        (spot) => spot.location.type === "exterior" && spot.location.world === world?.name,
      ),
    [map.spots, world],
  );
  const shownSpots = filter.collisions ? worldSpots : [];

  if (world === undefined) {
    return <p className="empty">{t.mapEmpty}</p>;
  }
  const ownMap = picture?.meta.world === world.name ? picture : null;
  return (
    <section className="map-view">
      <div className="controls">
        <label className="control">
          {t.mapWorld}
          <select value={worldIndex} onChange={(event) => chooseWorld(Number(event.target.value))}>
            {map.worlds.map((candidate, index) => (
              <option key={candidate.name} value={index}>
                {candidate.name}
              </option>
            ))}
          </select>
        </label>
        <label className="control">
          {t.mapMod}
          <select value={owner} onChange={(event) => setOwner(Number(event.target.value))}>
            <option value={-1}>{t.mapAllMods}</option>
            {map.owners.map((name, index) => (
              <option key={name} value={index}>
                {name}
              </option>
            ))}
          </select>
        </label>
        <fieldset className="segmented" aria-label={t.mapRelief}>
          <button type="button" aria-pressed={mode === "flat"} onClick={() => setMode("flat")}>
            {t.mapFlat}
          </button>
          <button type="button" aria-pressed={mode === "relief"} onClick={() => setMode("relief")}>
            {t.mapRelief}
          </button>
        </fieldset>
        <label className="control control-check">
          <input
            type="checkbox"
            checked={showCells}
            onChange={(event) => changeCells(event.target.checked)}
          />
          {t.mapModCells}
        </label>
        <Legend cells={showCells} />
      </div>
      <div className="map-body map-body-places">
        <PlacesPanel
          markers={markers}
          spots={worldSpots.length}
          filter={filter}
          onChange={setFilter}
        />
        {mode === "flat" ? (
          <Grid
            key={world.name}
            cells={world.cells}
            showCells={showCells}
            terrain={world.terrain ?? null}
            picture={ownMap}
            owner={owner}
            markers={shownMarkers}
            spots={shownSpots}
            picked={picked}
            hover={hover}
            onPick={setPicked}
            onHover={setHover}
            focus={focusWorld === worldIndex ? focus : null}
          />
        ) : world.terrain ? (
          <MapView3D
            key={world.name}
            world={world}
            terrain={world.terrain}
            picture={ownMap}
            owner={owner}
            showCells={showCells}
            markers={shownMarkers}
            spots={shownSpots}
            picked={picked}
            hover={hover}
            onPick={setPicked}
            onHover={setHover}
          />
        ) : (
          <div className="map-frame map-missing">
            <p className="empty">{t.mapNoRelief}</p>
          </div>
        )}
        <Detail
          world={world}
          owners={map.owners}
          owner={owner}
          markers={markers}
          spots={worldSpots}
          headlines={headlines}
          shown={picked ?? hover}
          onPick={setPicked}
        />
      </div>
    </section>
  );
}

function Legend({ cells }: { cells: boolean }) {
  const { t } = useSession();
  return (
    <div className="legend">
      {cells && (
        <>
          <span className="swatch" data-level="1" /> {t.mapLegendOne}
          <span className="swatch" data-level="2" /> {t.mapLegendTwo}
          <span className="swatch" data-level="3" /> {t.mapLegendThree}
          <span className="swatch" data-level="4" /> {t.mapLegendMany}
        </>
      )}
      <span className="swatch swatch-spot" /> {t.mapLegendSpot}
    </div>
  );
}

/** A place's outline as a small picture, for lists and the legend. */
export function PlaceIcon({ shape, origin }: { shape: Shape; origin: PlaceOrigin | "spot" }) {
  const fill = origin === "spot" ? "var(--accent)" : `var(${originVariable(origin)})`;
  const hollow = origin === "removed";
  const common = {
    fill: hollow ? "none" : fill,
    stroke: hollow ? fill : "var(--place-edge)",
    strokeWidth: hollow ? 2 : 1.2,
  };
  return (
    <svg className="place-icon" viewBox="0 0 16 16" aria-hidden="true">
      {shape === "square" ? (
        <rect x="2.5" y="2.5" width="11" height="11" {...common} />
      ) : shape === "triangle" ? (
        <path d="M8 1.8 L14.5 13.5 L1.5 13.5 Z" {...common} />
      ) : (
        <circle cx="8" cy="8" r="6" {...common} />
      )}
    </svg>
  );
}

interface PlacesPanelProps {
  markers: MapMarker[];
  spots: number;
  filter: PlaceFilter;
  onChange: (filter: PlaceFilter) => void;
}

/** Which places are shown: by origin and by kind, with counts. */
function PlacesPanel({ markers, spots, filter, onChange }: PlacesPanelProps) {
  const { t } = useSession();
  const byOrigin = new Map<PlaceOrigin, number>();
  const byKind = new Map<PlaceKind, number>();
  for (const marker of markers) {
    byOrigin.set(marker.origin, (byOrigin.get(marker.origin) ?? 0) + 1);
    if (filter.origins.has(marker.origin)) {
      byKind.set(marker.kind, (byKind.get(marker.kind) ?? 0) + 1);
    }
  }
  const allKinds = [...new Set(markers.map((marker) => marker.kind))];
  const kinds = [...allKinds].sort(
    (a, b) =>
      (byKind.get(b) ?? 0) - (byKind.get(a) ?? 0) || t.placesKind[a].localeCompare(t.placesKind[b]),
  );
  const toggle = <T,>(set: Set<T>, value: T) => {
    const next = new Set(set);
    if (next.has(value)) {
      next.delete(value);
    } else {
      next.add(value);
    }
    return next;
  };

  return (
    <aside className="places-panel" aria-label={t.mapPlaces}>
      <h3>{t.mapPlaces}</h3>
      <ul className="places-list">
        {ORIGINS.map((origin) => (
          <li key={origin}>
            <label className="places-row" title={t.placesOriginHint[origin]}>
              <input
                type="checkbox"
                checked={filter.origins.has(origin)}
                onChange={() => onChange({ ...filter, origins: toggle(filter.origins, origin) })}
              />
              <PlaceIcon shape="circle" origin={origin} />
              <span className="places-name">{t.placesOrigin[origin]}</span>
              <span className="places-count">{byOrigin.get(origin) ?? 0}</span>
            </label>
          </li>
        ))}
        <li>
          <label className="places-row">
            <input
              type="checkbox"
              checked={filter.collisions}
              onChange={() => onChange({ ...filter, collisions: !filter.collisions })}
            />
            <PlaceIcon shape="circle" origin="spot" />
            <span className="places-name">{t.placesCollisions}</span>
            <span className="places-count">{spots}</span>
          </label>
        </li>
      </ul>
      {kinds.length > 0 && (
        <>
          <div className="places-all">
            <button
              type="button"
              className="link-button"
              onClick={() => onChange({ ...filter, kinds: new Set(allKinds) })}
            >
              {t.placesShowAll}
            </button>
            <button
              type="button"
              className="link-button"
              onClick={() => onChange({ ...filter, kinds: new Set() })}
            >
              {t.placesHideAll}
            </button>
          </div>
          <ul className="places-list places-kinds">
            {kinds.map((kind) => (
              <li key={kind}>
                <label className="places-row">
                  <input
                    type="checkbox"
                    checked={filter.kinds.has(kind)}
                    onChange={() => onChange({ ...filter, kinds: toggle(filter.kinds, kind) })}
                  />
                  <PlaceIcon shape={shapeOf(kind)} origin="game" />
                  <span className="places-name">{t.placesKind[kind]}</span>
                  <span className="places-count">{byKind.get(kind) ?? 0}</span>
                </label>
              </li>
            ))}
          </ul>
        </>
      )}
    </aside>
  );
}

/** Colours read from the stylesheet, so that the map follows the theme. */
interface Palette {
  levels: string[];
  dim: string;
  spot: string;
  line: string;
  edge: string;
  places: Record<PlaceOrigin, string>;
  water: [number, number, number];
  low: [number, number, number];
  high: [number, number, number];
}

function rgb(value: string): [number, number, number] {
  const hex = value.trim().replace("#", "");
  const number = Number.parseInt(hex, 16);
  return [(number >> 16) & 255, (number >> 8) & 255, number & 255];
}

function palette(element: Element): Palette {
  const style = getComputedStyle(element);
  const colour = (name: string) => style.getPropertyValue(name).trim();
  return {
    levels: [colour("--heat-1"), colour("--heat-2"), colour("--heat-3"), colour("--heat-4")],
    dim: colour("--map-dim"),
    spot: colour("--accent"),
    line: colour("--text"),
    edge: colour("--place-edge"),
    places: {
      game: colour("--place-game"),
      added: colour("--place-added"),
      changed: colour("--place-changed"),
      removed: colour("--place-removed"),
    },
    water: rgb(colour("--map-water")),
    low: rgb(colour("--map-low")),
    high: rgb(colour("--map-high")),
  };
}

/**
 * The land as an image with one pixel per height sample: water, and land
 * tinted by height and shaded as if lit from the north-west.
 */
function relief(terrain: MapTerrain, colours: Palette): HTMLCanvasElement | null {
  const land = readLand(terrain);
  if (land === null) {
    return null;
  }
  const { width, height, heights, kinds } = land;
  const at = (x: number, y: number) =>
    Math.min(height - 1, Math.max(0, y)) * width + Math.min(width - 1, Math.max(0, x));
  const canvas = document.createElement("canvas");
  canvas.width = width;
  canvas.height = height;
  const context = canvas.getContext("2d");
  if (context === null) {
    return null;
  }
  const image = context.createImageData(width, height);
  const span = Math.max(1, terrain.highest - terrain.lowest);
  // Game units between two samples, and both sides of a sample.
  const spacing = (CELL / terrain.samples) * 2;
  const mix = (share: number, channel: 0 | 1 | 2) =>
    colours.low[channel] + (colours.high[channel] - colours.low[channel]) * share;
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const index = y * width + x;
      const kind = kinds[index] ?? 0;
      if (kind === 0) {
        continue;
      }
      let colour: [number, number, number];
      if (kind === 1) {
        colour = colours.water;
      } else {
        const here = heights[index] ?? terrain.lowest;
        const share = (here - terrain.lowest) / span;
        // Light from the north-west: slopes facing it are brighter.
        const eastward =
          ((heights[at(x + 1, y)] ?? here) - (heights[at(x - 1, y)] ?? here)) / spacing;
        const southward =
          ((heights[at(x, y + 1)] ?? here) - (heights[at(x, y - 1)] ?? here)) / spacing;
        const shade = Math.min(1.35, Math.max(0.55, 1 - (eastward + southward) * 0.9));
        colour = [
          Math.min(255, mix(share, 0) * shade),
          Math.min(255, mix(share, 1) * shade),
          Math.min(255, mix(share, 2) * shade),
        ];
      }
      const pixel = index * 4;
      image.data[pixel] = colour[0];
      image.data[pixel + 1] = colour[1];
      image.data[pixel + 2] = colour[2];
      image.data[pixel + 3] = 255;
    }
  }
  context.putImageData(image, 0, 0);
  return canvas;
}

/**
 * The water as an image with one pixel per height sample, to lay over the
 * game's terrain picture: that shows the ground under the water.
 */
function waterOver(terrain: MapTerrain, colours: Palette): HTMLCanvasElement | null {
  const land = readLand(terrain);
  if (land === null) {
    return null;
  }
  const canvas = document.createElement("canvas");
  canvas.width = land.width;
  canvas.height = land.height;
  const context = canvas.getContext("2d");
  if (context === null) {
    return null;
  }
  const image = context.createImageData(land.width, land.height);
  for (let index = 0; index < land.kinds.length; index++) {
    if (land.kinds[index] === 1) {
      image.data.set([...colours.water, 150], index * 4);
    }
  }
  context.putImageData(image, 0, 0);
  return canvas;
}

interface GridProps {
  cells: MapCell[];
  /** Whether the cells with objects of mods are tinted. */
  showCells: boolean;
  terrain: MapTerrain | null;
  picture: Picture | null;
  owner: number;
  markers: MapMarker[];
  spots: MapSpot[];
  picked: Picked | null;
  hover: Picked | null;
  onPick: (picked: Picked | null) => void;
  onHover: (hover: Picked | null) => void;
  focus: MapSpot | null;
}

function Grid({
  cells,
  showCells,
  terrain,
  picture,
  owner,
  markers,
  spots,
  picked,
  hover,
  onPick,
  onHover,
  focus,
}: GridProps) {
  const { t } = useSession();
  const canvas = useRef<HTMLCanvasElement>(null);
  const frame = useRef<HTMLDivElement>(null);
  const [size, setSize] = useState({ width: 800, height: 600 });
  const { width, height } = size;

  const spotsByCell = useMemo(() => {
    const found = new Map<string, MapSpot[]>();
    for (const spot of spots) {
      if (spot.location.type === "exterior") {
        const at = key(spot.location.x, spot.location.y);
        found.set(at, [...(found.get(at) ?? []), spot]);
      }
    }
    return found;
  }, [spots]);
  // Every cell of the land is on the map, not only those with objects of mods.
  const bounds = useMemo(() => {
    const xs = cells.map((cell) => cell.x);
    const ys = cells.map((cell) => cell.y);
    for (const marker of markers) {
      xs.push(cellOf(marker.position[0]));
      ys.push(cellOf(marker.position[1]));
    }
    if (picture !== null) {
      xs.push(picture.meta.west, picture.meta.west + picture.meta.columns - 1);
      ys.push(picture.meta.south, picture.meta.south + picture.meta.rows - 1);
    } else if (terrain !== null) {
      xs.push(terrain.west, terrain.west + terrain.columns - 1);
      ys.push(terrain.south, terrain.south + terrain.rows - 1);
    }
    if (xs.length === 0) {
      xs.push(0);
      ys.push(0);
    }
    return {
      minX: Math.min(...xs),
      maxX: Math.max(...xs),
      minY: Math.min(...ys),
      maxY: Math.max(...ys),
    };
  }, [cells, markers, picture, terrain]);

  // The land is drawn once per worldspace and then scaled with the view.
  const [land, setLand] = useState<HTMLCanvasElement | null>(null);
  const [water, setWater] = useState<HTMLCanvasElement | null>(null);
  useEffect(() => {
    if (terrain === null || canvas.current === null) {
      setLand(null);
      setWater(null);
      return;
    }
    const colours = palette(canvas.current);
    setLand(relief(terrain, colours));
    setWater(waterOver(terrain, colours));
  }, [terrain]);

  const fit = useCallback((): View => {
    const columns = bounds.maxX - bounds.minX + 1;
    const rows = bounds.maxY - bounds.minY + 1;
    const fitted = Math.max(
      MIN_SIZE,
      Math.min(MAX_SIZE, (width - 24) / columns, (height - 24) / rows),
    );
    return {
      size: fitted,
      x: (width - columns * fitted) / 2,
      y: (height - rows * fitted) / 2,
    };
  }, [bounds, width, height]);

  const [view, setView] = useState<View>(() => fit());
  const drag = useRef<{ x: number; y: number; view: View; moved: boolean } | null>(null);

  // The canvas fills the space the window leaves it.
  useEffect(() => {
    const element = frame.current;
    if (element === null) {
      return;
    }
    const measure = () => {
      const top = element.getBoundingClientRect().top;
      setSize({
        width: Math.max(320, Math.floor(element.clientWidth)),
        height: Math.max(420, Math.floor(window.innerHeight - top - 24)),
      });
    };
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    window.addEventListener("resize", measure);
    measure();
    return () => {
      observer.disconnect();
      window.removeEventListener("resize", measure);
    };
  }, []);

  // Start with everything in view, or close to the spot to show.
  useEffect(() => {
    const whole = fit();
    if (focus?.location.type === "exterior") {
      const zoomed = Math.max(whole.size, 22);
      const column = focus.location.x - bounds.minX;
      const row = bounds.maxY - focus.location.y;
      setView({
        size: zoomed,
        x: width / 2 - (column + 0.5) * zoomed,
        y: height / 2 - (row + 0.5) * zoomed,
      });
    } else {
      setView(whole);
    }
  }, [fit, focus, bounds, width, height]);

  // Where a position of the world lies on the canvas.
  const toScreen = useCallback(
    (position: readonly number[]) => ({
      x: view.x + ((position[0] ?? 0) / CELL - bounds.minX) * view.size,
      y: view.y + (bounds.maxY + 1 - (position[1] ?? 0) / CELL) * view.size,
    }),
    [view, bounds],
  );

  /** The place, spot or cell under the pointer. */
  const pickAt = (clientX: number, clientY: number): Picked | null => {
    const rect = canvas.current?.getBoundingClientRect();
    if (rect === undefined) {
      return null;
    }
    const px = clientX - rect.left;
    const py = clientY - rect.top;
    let best: { picked: Picked; distance: number } | null = null;
    const consider = (picked: Picked, position: readonly number[]) => {
      const at = toScreen(position);
      const distance = Math.hypot(at.x - px, at.y - py);
      if (distance <= HIT && (best === null || distance < best.distance)) {
        best = { picked, distance };
      }
    };
    for (const spot of spots) {
      const at = spotPosition(spot);
      if (at !== null) {
        consider({ type: "spot", spot }, at);
      }
    }
    for (const marker of markers) {
      consider({ type: "place", marker }, marker.position);
    }
    if (best !== null) {
      return (best as { picked: Picked }).picked;
    }
    const column = Math.floor((px - view.x) / view.size);
    const row = Math.floor((py - view.y) / view.size);
    const x = bounds.minX + column;
    const y = bounds.maxY - row;
    const inside = x >= bounds.minX && x <= bounds.maxX && y >= bounds.minY && y <= bounds.maxY;
    return inside ? { type: "cell", x, y } : null;
  };

  // Drawing.
  useEffect(() => {
    const element = canvas.current;
    const context = element?.getContext("2d");
    if (element == null || context == null) {
      return;
    }
    const ratio = window.devicePixelRatio || 1;
    element.width = width * ratio;
    element.height = height * ratio;
    context.setTransform(ratio, 0, 0, ratio, 0, 0);
    const colours = palette(element);

    context.clearRect(0, 0, width, height);
    const ground = picture !== null || land !== null;
    if (picture !== null) {
      context.imageSmoothingEnabled = view.size < 8;
      context.drawImage(
        picture.image,
        view.x + (picture.meta.west - bounds.minX) * view.size,
        view.y + (bounds.maxY - (picture.meta.south + picture.meta.rows - 1)) * view.size,
        picture.meta.columns * view.size,
        picture.meta.rows * view.size,
      );
      if (water !== null && terrain !== null) {
        context.imageSmoothingEnabled = true;
        context.drawImage(
          water,
          view.x + (terrain.west - bounds.minX) * view.size,
          view.y + (bounds.maxY - (terrain.south + terrain.rows - 1)) * view.size,
          terrain.columns * view.size,
          terrain.rows * view.size,
        );
      }
    } else if (land !== null && terrain !== null) {
      context.imageSmoothingEnabled = true;
      context.drawImage(
        land,
        view.x + (terrain.west - bounds.minX) * view.size,
        view.y + (bounds.maxY - (terrain.south + terrain.rows - 1)) * view.size,
        terrain.columns * view.size,
        terrain.rows * view.size,
      );
    }
    // The grid of cells, once they are large enough to tell apart.
    if (view.size >= 6) {
      context.strokeStyle = "rgb(255 255 255 / 14%)";
      context.lineWidth = 1;
      context.beginPath();
      for (let x = bounds.minX; x <= bounds.maxX + 1; x++) {
        const px = Math.round(view.x + (x - bounds.minX) * view.size) + 0.5;
        if (px >= 0 && px <= width) {
          context.moveTo(px, Math.max(0, view.y));
          context.lineTo(
            px,
            Math.min(height, view.y + (bounds.maxY - bounds.minY + 1) * view.size),
          );
        }
      }
      for (let y = bounds.minY; y <= bounds.maxY + 1; y++) {
        const py = Math.round(view.y + (bounds.maxY + 1 - y) * view.size) + 0.5;
        if (py >= 0 && py <= height) {
          context.moveTo(Math.max(0, view.x), py);
          context.lineTo(Math.min(width, view.x + (bounds.maxX - bounds.minX + 1) * view.size), py);
        }
      }
      context.stroke();
    }
    const gap = view.size >= 8 ? 1 : 0;
    context.globalAlpha = ground ? 0.55 : 1;
    for (const cell of showCells ? cells : []) {
      const px = view.x + (cell.x - bounds.minX) * view.size;
      const py = view.y + (bounds.maxY - cell.y) * view.size;
      if (px + view.size < 0 || py + view.size < 0 || px > width || py > height) {
        continue;
      }
      const relevant = owner < 0 || cell.owners.includes(owner);
      if (!relevant && ground) {
        continue;
      }
      const level = Math.min(cell.owners.length, 4) - 1;
      context.fillStyle = relevant ? (colours.levels[level] ?? colours.dim) : colours.dim;
      context.fillRect(px, py, view.size - gap, view.size - gap);
    }
    context.globalAlpha = 1;

    // Collisions: one mark per cell from afar, each at its point up close.
    const exact = view.size >= 24;
    context.lineWidth = 1;
    if (exact) {
      for (const spot of spots) {
        const at = spotPosition(spot);
        if (at === null) {
          continue;
        }
        const { x, y } = toScreen(at);
        context.beginPath();
        context.arc(x, y, 5, 0, Math.PI * 2);
        context.fillStyle = colours.spot;
        context.fill();
        context.strokeStyle = colours.line;
        context.stroke();
      }
    } else {
      for (const [at, found] of spotsByCell) {
        const [x, y] = at.split(",").map(Number) as [number, number];
        if (found.length === 0) {
          continue;
        }
        const px = view.x + (x - bounds.minX + 0.5) * view.size;
        const py = view.y + (bounds.maxY - y + 0.5) * view.size;
        context.beginPath();
        context.arc(px, py, Math.max(3, view.size * 0.3), 0, Math.PI * 2);
        context.fillStyle = colours.spot;
        context.fill();
        context.strokeStyle = colours.line;
        context.stroke();
      }
    }

    // The places, the game's first so that those of mods lie on top.
    const radius = Math.max(3.5, Math.min(7, view.size * 0.35));
    const order = (marker: MapMarker) => (marker.origin === "game" ? 0 : 1);
    const sorted = [...markers].sort((a, b) => order(a) - order(b));
    const labels: Label[] = [];
    for (const marker of sorted) {
      const { x, y } = toScreen(marker.position);
      if (x < -20 || y < -20 || x > width + 20 || y > height + 20) {
        continue;
      }
      // Where the game has a place a mod moved: a thin line from there.
      if (marker.movedFrom) {
        const from = toScreen(marker.movedFrom);
        context.setLineDash([3, 3]);
        context.strokeStyle = colours.places.changed;
        context.lineWidth = 1.5;
        context.beginPath();
        context.moveTo(from.x, from.y);
        context.lineTo(x, y);
        context.stroke();
        context.setLineDash([]);
      }
      tracePlace(context, shapeOf(marker.kind), x, y, radius);
      if (marker.origin === "removed") {
        context.setLineDash([2, 2]);
        context.strokeStyle = colours.places.removed;
        context.lineWidth = 2;
        context.stroke();
        context.setLineDash([]);
      } else {
        context.fillStyle = colours.places[marker.origin];
        context.fill();
        context.strokeStyle = colours.edge;
        context.lineWidth = 1.2;
        context.stroke();
      }
      const named = view.size >= 18 || (view.size >= 5 && isMajor(marker.kind));
      if (named && marker.name !== "") {
        labels.push({ x, y: y - radius - 4, text: marker.name, strong: isMajor(marker.kind) });
      }
    }
    drawLabels(context, labels);

    // What is picked and what the pointer is on.
    for (const [shown, lineWidth] of [
      [hover, 1.5],
      [picked, 3],
    ] as const) {
      if (shown === null) {
        continue;
      }
      context.strokeStyle = ground ? "#ffffff" : colours.line;
      context.lineWidth = lineWidth;
      if (shown.type === "cell") {
        context.strokeRect(
          view.x + (shown.x - bounds.minX) * view.size,
          view.y + (bounds.maxY - shown.y) * view.size,
          view.size,
          view.size,
        );
        continue;
      }
      const position = shown.type === "place" ? shown.marker.position : spotPosition(shown.spot);
      if (position === null) {
        continue;
      }
      const { x, y } = toScreen(position);
      context.beginPath();
      context.arc(x, y, radius + 5, 0, Math.PI * 2);
      context.stroke();
    }
  }, [
    cells,
    showCells,
    terrain,
    land,
    water,
    picture,
    owner,
    view,
    width,
    height,
    bounds,
    spots,
    spotsByCell,
    markers,
    hover,
    picked,
    toScreen,
  ]);

  const zoom = (factor: number, aroundX = width / 2, aroundY = height / 2) => {
    setView((current) => {
      const next = Math.max(MIN_SIZE, Math.min(MAX_SIZE, current.size * factor));
      const scale = next / current.size;
      return {
        size: next,
        x: aroundX - (aroundX - current.x) * scale,
        y: aroundY - (aroundY - current.y) * scale,
      };
    });
  };

  // Something picked elsewhere (in the details) is brought into view.
  useEffect(() => {
    if (picked === null || picked.type === "cell") {
      return;
    }
    const position = picked.type === "place" ? picked.marker.position : spotPosition(picked.spot);
    if (position === null) {
      return;
    }
    setView((current) => {
      const { x, y } = {
        x: current.x + (position[0] / CELL - bounds.minX) * current.size,
        y: current.y + (bounds.maxY + 1 - position[1] / CELL) * current.size,
      };
      if (x > 40 && y > 40 && x < width - 40 && y < height - 40) {
        return current;
      }
      return { ...current, x: current.x + width / 2 - x, y: current.y + height / 2 - y };
    });
  }, [picked, bounds, width, height]);

  const onPointerDown = (event: PointerEvent<HTMLCanvasElement>) => {
    event.currentTarget.setPointerCapture(event.pointerId);
    drag.current = { x: event.clientX, y: event.clientY, view, moved: false };
  };
  const onPointerMove = (event: PointerEvent<HTMLCanvasElement>) => {
    const start = drag.current;
    if (start !== null) {
      const dx = event.clientX - start.x;
      const dy = event.clientY - start.y;
      if (Math.abs(dx) + Math.abs(dy) > 3) {
        start.moved = true;
        setView({ ...start.view, x: start.view.x + dx, y: start.view.y + dy });
      }
      return;
    }
    const found = pickAt(event.clientX, event.clientY);
    if (!samePick(found, hover)) {
      onHover(found);
    }
  };
  const onPointerUp = (event: PointerEvent<HTMLCanvasElement>) => {
    const start = drag.current;
    drag.current = null;
    if (start !== null && !start.moved) {
      onPick(pickAt(event.clientX, event.clientY));
    }
  };

  // The wheel zooms around the pointer. React's own wheel handler cannot
  // stop the page from scrolling, so it is attached directly.
  useEffect(() => {
    const element = canvas.current;
    if (element === null) {
      return;
    }
    const onWheel = (event: WheelEvent) => {
      event.preventDefault();
      const rect = element.getBoundingClientRect();
      zoom(event.deltaY < 0 ? 1.25 : 0.8, event.clientX - rect.left, event.clientY - rect.top);
    };
    element.addEventListener("wheel", onWheel, { passive: false });
    return () => element.removeEventListener("wheel", onWheel);
  });

  return (
    <div className="map-frame" ref={frame}>
      <canvas
        className="map-canvas"
        ref={canvas}
        style={{ width, height }}
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerUp}
        onPointerLeave={() => onHover(null)}
      />
      <div className="map-zoom">
        <button
          type="button"
          className="map-zoom-button"
          onClick={() => zoom(1.5)}
          aria-label={t.mapZoomIn}
          title={t.mapZoomIn}
        >
          +
        </button>
        <button
          type="button"
          className="map-zoom-button"
          onClick={() => zoom(1 / 1.5)}
          aria-label={t.mapZoomOut}
          title={t.mapZoomOut}
        >
          −
        </button>
        <button type="button" className="map-zoom-button" onClick={() => setView(fit())}>
          {t.mapFit}
        </button>
      </div>
    </div>
  );
}

interface DetailProps {
  world: MapWorld;
  owners: string[];
  owner: number;
  markers: MapMarker[];
  spots: MapSpot[];
  headlines: Map<string, string>;
  shown: Picked | null;
  onPick: (picked: Picked) => void;
}

/** What is known about the picked place, collision or cell. */
function Detail({ world, owners, owner, markers, spots, headlines, shown, onPick }: DetailProps) {
  const { t } = useSession();
  const placeName = (marker: MapMarker) => marker.name || t.placesKind[marker.kind];
  const nearby = (position: readonly number[], except?: MapMarker) =>
    nearest(markers, position, 3, except);
  const nearList = (position: readonly number[], except?: MapMarker): ReactNode => {
    const found = nearby(position, except);
    if (found.length === 0) {
      return <p className="aside">{t.placesNone}</p>;
    }
    return (
      <ul className="map-here">
        {found.map(({ marker, metres }) => (
          <li key={`${marker.name}/${marker.position.join(",")}`}>
            <button
              type="button"
              className="place-link"
              onClick={() => onPick({ type: "place", marker })}
            >
              <PlaceIcon shape={shapeOf(marker.kind)} origin={marker.origin} />
              <span>{placeName(marker)}</span>
              <span className="aside">{t.placesDistance(metres)}</span>
            </button>
          </li>
        ))}
      </ul>
    );
  };
  const console = (command: string): ReactNode => (
    <div className="console">
      <code className="command" title={t.consoleHint}>
        {command}
      </code>
      <CopyButton text={command} />
    </div>
  );

  if (shown === null) {
    const counts = new Map<PlaceOrigin, number>();
    for (const marker of markers) {
      counts.set(marker.origin, (counts.get(marker.origin) ?? 0) + 1);
    }
    return (
      <aside className="map-detail">
        <h3>{world.name}</h3>
        <dl className="facts">
          <dt>{t.mapCells}</dt>
          <dd>{world.cells.length}</dd>
          <dt>{t.mapShared}</dt>
          <dd>{world.cells.filter((candidate) => candidate.owners.length > 1).length}</dd>
          <dt>{t.mapLegendSpot}</dt>
          <dd>{spots.length}</dd>
          {ORIGINS.filter((origin) => (counts.get(origin) ?? 0) > 0).map((origin) => (
            <PlaceCount key={origin} origin={origin} count={counts.get(origin) ?? 0} />
          ))}
        </dl>
        <p className="aside">{t.placesPick}</p>
      </aside>
    );
  }

  if (shown.type === "place") {
    const marker = shown.marker;
    const close = spotsNear(spots, world.name, marker.position, 400);
    return (
      <aside className="map-detail">
        <h3 className="place-title">
          <PlaceIcon shape={shapeOf(marker.kind)} origin={marker.origin} />
          {placeName(marker)}
        </h3>
        <p className="aside">
          {t.placesKind[marker.kind]} · {t.placesOrigin[marker.origin]}
          {marker.owner != null && ` ${t.placesBy(owners[marker.owner] ?? "?")}`}
        </p>
        {marker.movedFrom && (
          <p>{t.placesMoved(t.placesDistance(metresBetween(marker.movedFrom, marker.position)))}</p>
        )}
        {marker.renamedFrom && <p>{t.placesRenamed(marker.renamedFrom)}</p>}
        {console(marker.console)}
        {close.length > 0 && (
          <>
            <h3>{t.placesCollisionsNear}</h3>
            <ul className="map-here">
              {close.slice(0, 8).map(({ spot, metres }) => (
                <li key={`${spot.finding}/${spot.objects}/${spot.position?.join(",")}`}>
                  <button
                    type="button"
                    className="place-link"
                    onClick={() => onPick({ type: "spot", spot })}
                  >
                    <PlaceIcon shape="circle" origin="spot" />
                    <span>{spot.objects}</span>
                    <span className="aside">{t.placesDistance(metres)}</span>
                  </button>
                </li>
              ))}
            </ul>
          </>
        )}
        <h3>{t.placesNearby}</h3>
        {nearList(marker.position, marker)}
      </aside>
    );
  }

  if (shown.type === "spot") {
    const spot = shown.spot;
    const position = spotPosition(spot);
    const where = position === null ? [] : nearby(position);
    return (
      <aside className="map-detail">
        <h3 className="place-title">
          <PlaceIcon shape="circle" origin="spot" />
          {spot.objects}
        </h3>
        <p className="aside">{headlines.get(spot.finding) ?? spot.finding}</p>
        {where[0] !== undefined && (
          <p>
            {t.placesNear}{" "}
            <button
              type="button"
              className="link-button"
              onClick={() => where[0] && onPick({ type: "place", marker: where[0].marker })}
            >
              {placeName(where[0].marker)}
            </button>{" "}
            <span className="aside">({t.placesDistance(where[0].metres)})</span>
          </p>
        )}
        <ul className="map-list">
          {(spot.parts ?? []).map((part) => (
            <li key={`${part.object.plugin}/${part.object.localId}`}>
              {part.name} <span className="aside">· {part.object.plugin}</span>
            </li>
          ))}
        </ul>
        {console(spot.console)}
      </aside>
    );
  }

  const cell = world.cells.find((candidate) => candidate.x === shown.x && candidate.y === shown.y);
  const centre = [(shown.x + 0.5) * CELL, (shown.y + 0.5) * CELL];
  const here = spots.filter(
    (spot) =>
      spot.location.type === "exterior" &&
      spot.location.x === shown.x &&
      spot.location.y === shown.y,
  );
  return (
    <aside className="map-detail">
      <h3>{t.mapCell(shown.x, shown.y)}</h3>
      {cell === undefined ? (
        <p className="aside">{t.mapNoMods}</p>
      ) : (
        <>
          <p className="aside">{t.mapObjects(cell.objects)}</p>
          <ul className="map-list">
            {cell.owners.map((index) => (
              <li className="map-list-item" key={index} data-picked={index === owner}>
                {owners[index]}
              </li>
            ))}
          </ul>
        </>
      )}
      {console(`cow ${world.name} ${shown.x} ${shown.y}`)}
      {here.length > 0 && (
        <>
          <h3>{t.mapCollisions}</h3>
          <ul className="map-here">
            {here.map((spot) => (
              <li key={`${spot.finding}/${spot.objects}/${spot.position?.join(",")}`}>
                <button
                  type="button"
                  className="place-link place-link-block"
                  onClick={() => onPick({ type: "spot", spot })}
                >
                  <strong>{spot.objects}</strong>
                  <span className="aside">{headlines.get(spot.finding) ?? spot.finding}</span>
                </button>
              </li>
            ))}
          </ul>
        </>
      )}
      <h3>{t.placesNearby}</h3>
      {nearList(centre)}
    </aside>
  );
}

function PlaceCount({ origin, count }: { origin: PlaceOrigin; count: number }) {
  const { t } = useSession();
  return (
    <>
      <dt className="place-title">
        <PlaceIcon shape="circle" origin={origin} />
        {t.placesOrigin[origin]}
      </dt>
      <dd>{count}</dd>
    </>
  );
}
