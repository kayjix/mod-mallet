import { type PointerEvent, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { CopyButton, useSession } from "./Actions";
import type { MapCell, MapSpot, MapTerrain, WorldMap } from "./generated/rendered-report";

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
const MAX_SIZE = 80;

/** The key of a cell in lookups. */
const key = (x: number, y: number) => `${x},${y}`;

/**
 * Where mods place objects in a worldspace, cell by cell, over the shape
 * of the land: cells coloured by how many mods place objects there, and
 * the spots where objects of two mods collide marked.
 */
export function MapView({ map, headlines, focus }: MapViewProps) {
  const { t } = useSession();
  const focusWorld =
    focus?.location.type === "exterior"
      ? map.worlds.findIndex((world) => world.name === (focus.location as { world: string }).world)
      : -1;
  const [worldIndex, setWorldIndex] = useState(Math.max(focusWorld, 0));
  const [owner, setOwner] = useState(-1);
  const world = map.worlds[worldIndex];

  if (world === undefined) {
    return <p className="empty">{t.mapEmpty}</p>;
  }
  return (
    <section className="map-view">
      <div className="controls">
        <label className="control">
          {t.mapWorld}
          <select
            value={worldIndex}
            onChange={(event) => setWorldIndex(Number(event.target.value))}
          >
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
        <Legend />
      </div>
      <Grid
        key={world.name}
        cells={world.cells}
        terrain={world.terrain ?? null}
        world={world.name}
        owners={map.owners}
        owner={owner}
        spots={map.spots}
        headlines={headlines}
        focus={focusWorld === worldIndex ? focus : null}
      />
    </section>
  );
}

function Legend() {
  const { t } = useSession();
  return (
    <div className="legend">
      <span className="swatch" data-level="1" /> {t.mapLegendOne}
      <span className="swatch" data-level="2" /> {t.mapLegendTwo}
      <span className="swatch" data-level="3" /> {t.mapLegendMany}
      <span className="swatch swatch-spot" /> {t.mapLegendSpot}
    </div>
  );
}

/** Colours read from the stylesheet, so that the map follows the theme. */
interface Palette {
  levels: string[];
  dim: string;
  spot: string;
  line: string;
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
    levels: [colour("--map-1"), colour("--map-2"), colour("--map-3")],
    dim: colour("--map-dim"),
    spot: colour("--accent"),
    line: colour("--text"),
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
  const width = terrain.columns * terrain.samples;
  const height = terrain.rows * terrain.samples;
  const binary = atob(terrain.heights);
  if (binary.length !== width * height) {
    return null;
  }
  const value = (x: number, y: number) =>
    binary.charCodeAt(
      Math.min(height - 1, Math.max(0, y)) * width + Math.min(width - 1, Math.max(0, x)),
    );
  const canvas = document.createElement("canvas");
  canvas.width = width;
  canvas.height = height;
  const context = canvas.getContext("2d");
  if (context === null) {
    return null;
  }
  const image = context.createImageData(width, height);
  const mix = (share: number, channel: 0 | 1 | 2) =>
    colours.low[channel] + (colours.high[channel] - colours.low[channel]) * share;
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const at = (y * width + x) * 4;
      const here = value(x, y);
      if (here === 0) {
        continue;
      }
      let colour: [number, number, number];
      if (here === 1) {
        colour = colours.water;
      } else {
        const share = (here - 2) / 253;
        // Light from the north-west: slopes facing it are brighter.
        const eastward = Math.max(value(x + 1, y), 2) - Math.max(value(x - 1, y), 2);
        const southward = Math.max(value(x, y + 1), 2) - Math.max(value(x, y - 1), 2);
        const shade = Math.min(1.35, Math.max(0.55, 1 - (eastward + southward) * 0.02));
        colour = [
          Math.min(255, mix(share, 0) * shade),
          Math.min(255, mix(share, 1) * shade),
          Math.min(255, mix(share, 2) * shade),
        ];
      }
      image.data[at] = colour[0];
      image.data[at + 1] = colour[1];
      image.data[at + 2] = colour[2];
      image.data[at + 3] = 255;
    }
  }
  context.putImageData(image, 0, 0);
  return canvas;
}

interface GridProps {
  cells: MapCell[];
  terrain: MapTerrain | null;
  world: string;
  owners: string[];
  owner: number;
  spots: MapSpot[];
  headlines: Map<string, string>;
  focus: MapSpot | null;
}

function Grid({ cells, terrain, world, owners, owner, spots, headlines, focus }: GridProps) {
  const { t } = useSession();
  const canvas = useRef<HTMLCanvasElement>(null);
  const frame = useRef<HTMLDivElement>(null);
  const [size, setSize] = useState({ width: 800, height: 600 });
  const { width, height } = size;

  const byCell = useMemo(() => new Map(cells.map((cell) => [key(cell.x, cell.y), cell])), [cells]);
  const spotsByCell = useMemo(() => {
    const found = new Map<string, MapSpot[]>();
    for (const spot of spots) {
      if (spot.location.type === "exterior" && spot.location.world === world) {
        const at = key(spot.location.x, spot.location.y);
        found.set(at, [...(found.get(at) ?? []), spot]);
      }
    }
    return found;
  }, [spots, world]);
  const bounds = useMemo(() => {
    const xs = cells.map((cell) => cell.x);
    const ys = cells.map((cell) => cell.y);
    return {
      minX: Math.min(...xs),
      maxX: Math.max(...xs),
      minY: Math.min(...ys),
      maxY: Math.max(...ys),
    };
  }, [cells]);

  // The land is drawn once per worldspace and then scaled with the view.
  const [land, setLand] = useState<HTMLCanvasElement | null>(null);
  useEffect(() => {
    if (terrain === null || canvas.current === null) {
      setLand(null);
      return;
    }
    setLand(relief(terrain, palette(canvas.current)));
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
  const [hover, setHover] = useState<string | null>(null);
  const [picked, setPicked] = useState<string | null>(() =>
    focus?.location.type === "exterior" ? key(focus.location.x, focus.location.y) : null,
  );
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

  const cellAt = (clientX: number, clientY: number): string | null => {
    const rect = canvas.current?.getBoundingClientRect();
    if (rect === undefined) {
      return null;
    }
    const column = Math.floor((clientX - rect.left - view.x) / view.size);
    const row = Math.floor((clientY - rect.top - view.y) / view.size);
    const at = key(bounds.minX + column, bounds.maxY - row);
    return byCell.has(at) ? at : null;
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
    if (land !== null && terrain !== null) {
      context.imageSmoothingEnabled = true;
      context.drawImage(
        land,
        view.x + (terrain.west - bounds.minX) * view.size,
        view.y + (bounds.maxY - (terrain.south + terrain.rows - 1)) * view.size,
        terrain.columns * view.size,
        terrain.rows * view.size,
      );
    }
    const gap = view.size >= 8 ? 1 : 0;
    context.globalAlpha = land === null ? 1 : 0.62;
    for (const cell of cells) {
      const px = view.x + (cell.x - bounds.minX) * view.size;
      const py = view.y + (bounds.maxY - cell.y) * view.size;
      if (px + view.size < 0 || py + view.size < 0 || px > width || py > height) {
        continue;
      }
      const relevant = owner < 0 || cell.owners.includes(owner);
      if (!relevant && land !== null) {
        continue;
      }
      const level = Math.min(cell.owners.length, 3) - 1;
      context.fillStyle = relevant ? (colours.levels[level] ?? colours.dim) : colours.dim;
      context.fillRect(px, py, view.size - gap, view.size - gap);
    }
    context.globalAlpha = 1;
    for (const [at, found] of spotsByCell) {
      const cell = byCell.get(at);
      if (cell === undefined || found.length === 0) {
        continue;
      }
      const px = view.x + (cell.x - bounds.minX + 0.5) * view.size;
      const py = view.y + (bounds.maxY - cell.y + 0.5) * view.size;
      context.beginPath();
      context.arc(px, py, Math.max(3, view.size * 0.3), 0, Math.PI * 2);
      context.fillStyle = colours.spot;
      context.fill();
      context.lineWidth = 1;
      context.strokeStyle = colours.line;
      context.stroke();
    }
    for (const [at, lineWidth] of [
      [hover, 1],
      [picked, 2.5],
    ] as const) {
      const cell = at === null ? undefined : byCell.get(at);
      if (cell !== undefined) {
        context.strokeStyle = colours.line;
        context.lineWidth = lineWidth;
        context.strokeRect(
          view.x + (cell.x - bounds.minX) * view.size,
          view.y + (bounds.maxY - cell.y) * view.size,
          view.size,
          view.size,
        );
      }
    }
  }, [
    cells,
    terrain,
    land,
    owner,
    view,
    width,
    height,
    bounds,
    spotsByCell,
    byCell,
    hover,
    picked,
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
    setHover(cellAt(event.clientX, event.clientY));
  };
  const onPointerUp = (event: PointerEvent<HTMLCanvasElement>) => {
    const start = drag.current;
    drag.current = null;
    if (start !== null && !start.moved) {
      setPicked(cellAt(event.clientX, event.clientY));
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

  const shown = picked ?? hover;
  const cell = shown === null ? undefined : byCell.get(shown);
  const here = shown === null ? [] : (spotsByCell.get(shown) ?? []);
  const collisions = [...spotsByCell.values()].reduce((sum, found) => sum + found.length, 0);

  return (
    <div className="map-body">
      <div className="map-frame" ref={frame}>
        <canvas
          className="map-canvas"
          ref={canvas}
          style={{ width, height }}
          onPointerDown={onPointerDown}
          onPointerMove={onPointerMove}
          onPointerUp={onPointerUp}
          onPointerLeave={() => setHover(null)}
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
      <aside className="map-detail">
        {cell === undefined ? (
          <>
            <h3>{world}</h3>
            <dl className="facts">
              <dt>{t.mapCells}</dt>
              <dd>{cells.length}</dd>
              <dt>{t.mapShared}</dt>
              <dd>{cells.filter((candidate) => candidate.owners.length > 1).length}</dd>
              <dt>{t.mapLegendSpot}</dt>
              <dd>{collisions}</dd>
            </dl>
          </>
        ) : (
          <>
            <h3>{t.mapCell(cell.x, cell.y)}</h3>
            <p className="aside">{t.mapObjects(cell.objects)}</p>
            <ul className="map-list">
              {cell.owners.map((index) => (
                <li className="map-list-item" key={index} data-picked={index === owner}>
                  {owners[index]}
                </li>
              ))}
            </ul>
            <div className="console">
              <code className="command" title={t.consoleHint}>
                {`cow ${world} ${cell.x} ${cell.y}`}
              </code>
              <CopyButton text={`cow ${world} ${cell.x} ${cell.y}`} />
            </div>
            {here.length > 0 && (
              <>
                <h3>{t.mapCollisions}</h3>
                <ul className="map-here">
                  {here.map((spot) => (
                    <li className="map-here-item" key={`${spot.finding}/${spot.objects}`}>
                      <strong>{spot.objects}</strong>
                      <span className="aside">{headlines.get(spot.finding) ?? spot.finding}</span>
                    </li>
                  ))}
                </ul>
              </>
            )}
          </>
        )}
      </aside>
    </div>
  );
}
