// The load order as an arc diagram on a canvas (ADR 0068): every active
// plugin on one line from left to right, an arc from each plugin to each
// plugin it needs. What the picked plugins need is drawn above the line
// with solid arcs and triangles, what needs them below with dashed arcs
// and dots, so that the two differ by more than colour.

import { type KeyboardEvent, type MouseEvent, useEffect, useRef, useState } from "react";
import type { DependencyGraph } from "./dependencies";
import { useDependencyTexts } from "./dependency-texts";
import "./dependencies.css";

/** Space left and right of the first and last plugin, in CSS pixels. */
const PAD = 10;

/** Half the thickness of the line the plugins sit on. */
const AXIS = 2;

/** Colours of the diagram, read from the page's style so both themes work. */
interface Palette {
  arc: string;
  needs: string;
  neededBy: string;
  focus: string;
  mark: string;
  missing: string;
  band: Record<string, string>;
}

function readPalette(element: Element): Palette {
  const style = getComputedStyle(element);
  const read = (name: string) => style.getPropertyValue(name).trim() || "gray";
  return {
    arc: read("--dep-arc"),
    needs: read("--dep-needs"),
    neededBy: read("--dep-needed-by"),
    focus: read("--dep-focus"),
    mark: read("--dep-mark"),
    missing: read("--dep-missing"),
    band: {
      game: read("--dep-band-game"),
      creationClub: read("--dep-band-cc"),
      mod: read("--dep-band-mod"),
    },
  };
}

/** A number that changes whenever the theme or the pixel density changes, so the canvas is drawn again. */
function useThemeVersion(): number {
  const [version, setVersion] = useState(0);
  useEffect(() => {
    const bump = () => setVersion((value) => value + 1);
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    media.addEventListener("change", bump);
    // Moving the window to a screen of another pixel density.
    const density = window.matchMedia(`(resolution: ${window.devicePixelRatio}dppx)`);
    density.addEventListener("change", bump);
    const observer = new MutationObserver(bump);
    observer.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ["data-theme"],
    });
    return () => {
      media.removeEventListener("change", bump);
      density.removeEventListener("change", bump);
      observer.disconnect();
    };
  }, []);
  return version;
}

/** The width of an element in CSS pixels, kept current. */
function useWidth(element: HTMLElement | null): number {
  const [width, setWidth] = useState(0);
  useEffect(() => {
    if (element === null) {
      return;
    }
    const observer = new ResizeObserver(([entry]) => {
      if (entry !== undefined) {
        setWidth(Math.floor(entry.contentRect.width));
      }
    });
    observer.observe(element);
    return () => observer.disconnect();
  }, [element]);
  return width;
}

/** The arc diagram of a load order. */
export function DependencyArcs({
  graph,
  focus,
  marked,
  height,
  compact = false,
  onPick,
  onStep,
}: {
  graph: DependencyGraph;
  /** The plugins picked, by place: their arcs are drawn in colour. */
  focus: number[];
  /** Plugins to mark on the line, such as those a search found. */
  marked?: Set<number>;
  /** The height in CSS pixels. */
  height: number;
  /** A thin strip without the names of the stretches below it. */
  compact?: boolean;
  /** Called with a plugin's place when it is clicked. */
  onPick?: (place: number) => void;
  /** Called with -1 or 1 when the arrow keys step through the load order, 0 for Escape. */
  onStep?: (direction: -1 | 0 | 1) => void;
}) {
  const d = useDependencyTexts();
  const [wrapper, setWrapper] = useState<HTMLDivElement | null>(null);
  const canvas = useRef<HTMLCanvasElement>(null);
  const width = useWidth(wrapper);
  const theme = useThemeVersion();
  const [hover, setHover] = useState<{ place: number; x: number } | null>(null);
  const count = graph.nodes.length;
  const step = count > 1 ? (width - 2 * PAD) / (count - 1) : 0;
  const xOf = (place: number) => PAD + place * step;

  // Without a pick, the plugin under the pointer shows its arcs.
  const shown = focus.length > 0 || compact || hover === null ? focus : [hover.place];

  useEffect(() => {
    const element = canvas.current;
    const context = element?.getContext("2d");
    if (element == null || context == null || width === 0) {
      return;
    }
    void theme;
    const ratio = window.devicePixelRatio || 1;
    element.width = Math.round(width * ratio);
    element.height = Math.round(height * ratio);
    context.setTransform(ratio, 0, 0, ratio, 0, 0);
    context.clearRect(0, 0, width, height);
    const palette = readPalette(element);
    // Lines that run straight up and down sit on whole device pixels.
    const snap = (x: number) => Math.round(x * ratio) / ratio;
    const x = (place: number) => PAD + place * step;

    const axis = Math.round(height * (compact ? 0.56 : 0.6));
    const up = axis - AXIS - 1;
    const down = axis + AXIS + 1;
    const roomUp = up - 4;
    const roomDown = height - down - 4;
    // Short links are round, long ones flatten so that they stay inside.
    const rise = (distance: number, radius: number, room: number) =>
      Math.max(
        2,
        Math.min(radius, room * Math.min(1, Math.sqrt(distance / Math.max(1, count)) * 1.3)),
      );

    const arc = (from: number, to: number, below: boolean) => {
      const left = Math.min(x(from), x(to));
      const right = Math.max(x(from), x(to));
      const radius = (right - left) / 2;
      const base = below ? down : up;
      const tall = rise(Math.abs(to - from), radius, below ? roomDown : roomUp);
      context.moveTo(left, base);
      context.ellipse(left + radius, base, Math.max(radius, 0.5), tall, 0, Math.PI, 0, below);
    };

    const inFocus = new Set(shown);
    const picked = shown.length > 0;

    // Every link, faint; those to the game's own plugins, which nearly
    // every plugin needs, fainter still.
    context.strokeStyle = palette.arc;
    context.lineWidth = 1;
    for (const toGame of [true, false]) {
      context.beginPath();
      for (const node of graph.nodes) {
        for (const master of node.masters) {
          if ((graph.nodes[master]?.origin === "game") === toGame) {
            arc(master, node.index, false);
          }
        }
      }
      context.globalAlpha = (picked ? 0.45 : 1) * (toGame ? 0.4 : 1);
      context.stroke();
    }
    context.globalAlpha = 1;

    // The stretches of the game, the Creation Club and the mods.
    for (const band of graph.bands) {
      const left = snap(x(band.from) - step / 2);
      const right = snap(x(band.to) + step / 2);
      context.fillStyle = palette.band[band.origin] ?? palette.arc;
      context.fillRect(Math.max(0, left), axis - AXIS, Math.max(1, right - left), AXIS * 2);
    }

    if (picked) {
      const needs = new Set<number>();
      const neededBy = new Set<number>();
      // Links between the picked plugins themselves.
      context.beginPath();
      for (const place of shown) {
        for (const master of graph.nodes[place]?.masters ?? []) {
          if (inFocus.has(master)) {
            arc(master, place, false);
          }
        }
      }
      context.strokeStyle = palette.focus;
      context.lineWidth = 1.2;
      context.stroke();

      // What they need: solid, above.
      context.beginPath();
      for (const place of shown) {
        for (const master of graph.nodes[place]?.masters ?? []) {
          if (!inFocus.has(master)) {
            needs.add(master);
            arc(master, place, false);
          }
        }
      }
      context.strokeStyle = palette.needs;
      context.lineWidth = 1.6;
      context.stroke();

      // What needs them: dashed, below.
      context.beginPath();
      for (const place of shown) {
        for (const dependent of graph.nodes[place]?.dependents ?? []) {
          if (!inFocus.has(dependent)) {
            neededBy.add(dependent);
            arc(place, dependent, true);
          }
        }
      }
      context.strokeStyle = palette.neededBy;
      context.lineWidth = 1.6;
      context.setLineDash([5, 3]);
      context.stroke();
      context.setLineDash([]);

      // Triangles above for what they need, dots below for what needs them.
      context.fillStyle = palette.needs;
      for (const place of needs) {
        const at = snap(x(place));
        context.beginPath();
        context.moveTo(at, up - 8);
        context.lineTo(at - 4, up - 1);
        context.lineTo(at + 4, up - 1);
        context.closePath();
        context.fill();
      }
      context.fillStyle = palette.neededBy;
      for (const place of neededBy) {
        context.beginPath();
        context.arc(snap(x(place)), down + 4, 3, 0, Math.PI * 2);
        context.fill();
      }
      // The picked plugins: a bar across the line.
      context.fillStyle = palette.focus;
      for (const place of shown) {
        context.fillRect(snap(x(place)) - 1, axis - 9, 2, 18);
        if ((graph.nodes[place]?.missing.length ?? 0) > 0) {
          context.fillStyle = palette.missing;
          context.beginPath();
          context.arc(snap(x(place)), axis - 12, 3, 0, Math.PI * 2);
          context.fill();
          context.fillStyle = palette.focus;
        }
      }
    }

    // Plugins a search found: ticks across the line.
    if (marked !== undefined) {
      context.fillStyle = palette.mark;
      for (const place of marked) {
        context.fillRect(snap(x(place)) - 0.75, axis - 7, 1.5, 14);
      }
    }

    if (hover !== null) {
      context.fillStyle = palette.focus;
      context.globalAlpha = 0.35;
      context.fillRect(snap(x(hover.place)) - 0.5, 0, 1, height);
      context.globalAlpha = 1;
    }
  }, [graph, shown, marked, width, height, compact, step, count, hover, theme]);

  const placeAt = (event: MouseEvent<HTMLCanvasElement>): number | null => {
    if (count === 0 || step <= 0) {
      return count === 1 ? 0 : null;
    }
    const box = event.currentTarget.getBoundingClientRect();
    const place = Math.round((event.clientX - box.left - PAD) / step);
    return Math.min(count - 1, Math.max(0, place));
  };

  const onKey = (event: KeyboardEvent<HTMLCanvasElement>) => {
    if (onStep === undefined) {
      return;
    }
    const direction =
      event.key === "ArrowLeft"
        ? -1
        : event.key === "ArrowRight"
          ? 1
          : event.key === "Escape"
            ? 0
            : null;
    if (direction !== null) {
      event.preventDefault();
      onStep(direction);
    }
  };

  const hovered = hover === null ? undefined : graph.nodes[hover.place];
  return (
    <div className="dep-arcs" data-compact={compact} ref={setWrapper}>
      <canvas
        ref={canvas}
        style={{ width: "100%", height }}
        role="img"
        aria-label={d.diagramLabel(count, graph.links)}
        tabIndex={onStep === undefined ? -1 : 0}
        onKeyDown={onKey}
        onPointerMove={(event) => {
          const place = placeAt(event);
          setHover(place === null ? null : { place, x: xOf(place) });
        }}
        onPointerLeave={() => setHover(null)}
        onClick={(event) => {
          const place = placeAt(event);
          if (place !== null) {
            onPick?.(place);
          }
        }}
        data-pickable={onPick !== undefined}
      />
      {hovered !== undefined && hover !== null && (
        <div
          className="dep-tip"
          style={{ left: Math.min(Math.max(hover.x, 80), Math.max(80, width - 80)) }}
          aria-hidden="true"
        >
          <strong>{hovered.name}</strong>
          {hovered.modName !== null && <span>{d.fromMod(hovered.modName)}</span>}
        </div>
      )}
      {!compact && width > 0 && (
        <div className="dep-bands" aria-hidden="true">
          {bandLabels(graph, d.origins, xOf, step).map((label) => (
            <span key={label.key} data-origin={label.origin} style={{ left: label.left }}>
              {label.text}
            </span>
          ))}
        </div>
      )}
    </div>
  );
}

/** Roughly how wide one letter of a stretch's name is, in CSS pixels. */
const LETTER = 7;

/** The names of the stretches below the line, each where its stretch starts but never over the one before. */
function bandLabels(
  graph: DependencyGraph,
  names: Record<string, string>,
  xOf: (place: number) => number,
  step: number,
): { key: string; origin: string; text: string; left: number }[] {
  let free = 0;
  return graph.bands.map((band) => {
    const text = names[band.origin] ?? band.origin;
    const left = Math.max(free, xOf(band.from) - step / 2);
    free = left + text.length * LETTER + 12;
    return { key: `${band.origin}-${band.from}`, origin: band.origin, text, left };
  });
}
