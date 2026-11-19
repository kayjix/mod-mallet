// An interior cell in 3D (ADR 0042): everything placed in it, each object
// with its own mesh where the game places it, framed on a point. No land
// and no level of detail: an interior has neither.

import { type PointerEvent, type ReactNode, useCallback, useEffect, useRef, useState } from "react";
import { useSession } from "./Actions";
import { AreaLayer, LOOK, type Look, objectKey, type PartTools } from "./area-layer";
import type { AreaObject, AreaObjects } from "./generated/area-objects";
import { colour, lookAt, type Mat4, multiply, perspective, type Vec3 } from "./gl";
import { InfoTip } from "./info-tip";
import { interiorTexts } from "./interior-texts";
import { LoadingMallet } from "./LoadingMallet";
import { CELL } from "./map-places";
import { useWandLevel, ViewToolbar, WandControl } from "./wand";

export interface InteriorViewProps {
  /** The interior cell, by its editor ID. */
  cell: string;
  /** The point to look at first, in game units; the middle of the cell if `null`. */
  centre: [number, number, number] | null;
  /** The objects of the collision looked at, by {@link objectKey}: drawn lit up. */
  focus: Set<string>;
  /** The objects the patch would hide, by {@link objectKey}: drawn as ghosts. */
  hidden: Set<string>;
  /**
   * Shows the objects as the game would: nothing tinted, and what is
   * hidden left out rather than drawn as a ghost.
   */
  plain: boolean;
  /** More for the card of a picked object, such as what to do with it. */
  objectExtra?: (object: AreaObject, tools: PartTools) => ReactNode;
  /** Told about each object picked in the view, or none. */
  onObject?: (object: AreaObject | null) => void;
  /** Told about the objects of the cell once they are read, `null` if they cannot be. */
  onArea?: (area: AreaObjects | null) => void;
  /** Shown over the view. */
  children?: ReactNode;
  /** More tools for the bar along the top of the view, after the magic wand. */
  tools?: ReactNode;
}

/**
 * Where the camera is: it looks at `target` (in cells) from `distance`
 * cells away, turned by `yaw` from north and raised by `pitch` above the
 * horizon.
 */
interface Camera {
  target: Vec3;
  distance: number;
  yaw: number;
  pitch: number;
}

const FIELD_OF_VIEW = (45 * Math.PI) / 180;
const MIN_PITCH = -0.3;
const MAX_PITCH = 1.55;
/** The closest the camera comes, in cells: about 20 game units. */
const MIN_DISTANCE = 0.005;
/** How far from the point the camera starts, in cells: about 1200 game units. */
const START_DISTANCE = 0.3;

/** The point at the middle of the objects, in cells, at their average height. */
function middle(area: AreaObjects): Vec3 {
  const heights = area.objects.map((object) => object.position[2]);
  const height = heights.length === 0 ? 0 : heights.reduce((a, b) => a + b, 0) / heights.length;
  return [area.centre[0] / CELL, area.centre[1] / CELL, height / CELL];
}

export function InteriorView({
  cell,
  centre,
  focus,
  hidden,
  plain,
  objectExtra,
  onObject,
  onArea,
  children,
  tools,
}: InteriorViewProps) {
  const { t, api, lang } = useSession();
  const own = interiorTexts[lang];
  const frame = useRef<HTMLDivElement>(null);
  const canvas = useRef<HTMLCanvasElement>(null);
  const layer = useRef<AreaLayer | null>(null);
  const [height, setHeight] = useState(520);
  const [state, setState] = useState<"loading" | "missing" | "shown">("loading");
  const [area, setArea] = useState<AreaObjects | null>(null);
  const [progress, setProgress] = useState({ done: 0, total: 0 });
  const [picked, setPicked] = useState<AreaObject | null>(null);
  // What the magic wand takes along with the picked object (ADR 0055).
  const [level, setLevel] = useWandLevel();
  const [selection, setSelection] = useState<AreaObject[]>([]);
  useEffect(() => {
    const found = picked === null ? [] : (layer.current?.wand(picked, level) ?? [picked]);
    setSelection(found);
    layer.current?.showChain(found.slice(1));
    redrawNow.current();
  }, [picked, level]);
  const [seeInside, setSeeInside] = useState(true);
  const [failed, setFailed] = useState(false);
  const camera = useRef<Camera>({
    target: [0, 0, 0],
    distance: START_DISTANCE,
    yaw: 0,
    pitch: 0.7,
  });
  const lastMatrix = useRef<Mat4 | null>(null);
  const pending = useRef<number | null>(null);
  const told = useRef(onArea);
  told.current = onArea;
  const seeInsideNow = useRef(seeInside);
  seeInsideNow.current = seeInside;

  const lookOf = useRef<(key: string) => Look>(() => LOOK.plain);
  lookOf.current = (key) =>
    hidden.has(key)
      ? plain
        ? LOOK.gone
        : LOOK.ghost
      : !plain && focus.has(key)
        ? LOOK.focus
        : LOOK.plain;

  // The point by its value, so that a new array with the same point does
  // not move the camera.
  const centreKey = centre === null ? "" : centre.join(",");
  /** The camera framed on `centre`, or else on the middle of the cell. */
  const framed = useCallback(
    (shown: AreaObjects | null): Camera => {
      const point = centreKey === "" ? null : centreKey.split(",").map(Number);
      const target: Vec3 =
        point !== null
          ? [(point[0] ?? 0) / CELL, (point[1] ?? 0) / CELL, (point[2] ?? 0) / CELL]
          : shown !== null
            ? middle(shown)
            : [0, 0, 0];
      const distance =
        point !== null || shown === null
          ? START_DISTANCE
          : Math.max(START_DISTANCE, (shown.radius / CELL) * 2.2);
      return { target, distance, yaw: 0, pitch: 0.7 };
    },
    [centreKey],
  );

  const draw = useCallback(() => {
    pending.current = null;
    const element = canvas.current;
    const current = layer.current;
    const gl = element?.getContext("webgl2") ?? null;
    if (element === null || gl === null) {
      return;
    }
    const ratio = window.devicePixelRatio || 1;
    const width = Math.max(1, element.clientWidth);
    const tall = Math.max(1, element.clientHeight);
    const wanted = [Math.round(width * ratio), Math.round(tall * ratio)];
    if (element.width !== wanted[0] || element.height !== wanted[1]) {
      element.width = wanted[0] ?? element.width;
      element.height = wanted[1] ?? element.height;
    }
    gl.viewport(0, 0, gl.drawingBufferWidth, gl.drawingBufferHeight);
    gl.clearColor(0.05, 0.06, 0.07, 1);
    gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
    const view = camera.current;
    const level = Math.cos(view.pitch) * view.distance;
    const eye: Vec3 = [
      view.target[0] - Math.sin(view.yaw) * level,
      view.target[1] - Math.cos(view.yaw) * level,
      view.target[2] + Math.sin(view.pitch) * view.distance,
    ];
    const reach = Math.max(1, (area?.radius ?? 0) / CELL) * 4;
    const matrix = multiply(
      perspective(FIELD_OF_VIEW, width / tall, view.distance * 0.02, view.distance * 8 + reach),
      lookAt(eye, view.target, [0, 0, 1]),
    );
    lastMatrix.current = matrix;
    if (current === null) {
      return;
    }
    gl.enable(gl.DEPTH_TEST);
    cull(gl, seeInsideNow.current);
    const style = getComputedStyle(element);
    const css = (name: string) => colour(style.getPropertyValue(name));
    current.draw(matrix, 1, {
      mark: css("--accent"),
      focus: css("--spot-focus"),
      ghost: css("--spot-ghost"),
    });
    gl.disable(gl.CULL_FACE);
    const now = current.progress;
    setProgress((before) =>
      before.done === now.done && before.total === now.total ? before : now,
    );
  }, [area]);

  const redraw = useCallback(() => {
    if (pending.current === null) {
      pending.current = window.requestAnimationFrame(draw);
    }
  }, [draw]);
  const redrawNow = useRef(redraw);
  redrawNow.current = redraw;

  // The canvas fills the width and the height the window leaves it.
  useEffect(() => {
    const element = frame.current;
    if (element === null) {
      return;
    }
    const measure = () => {
      const top = element.getBoundingClientRect().top;
      setHeight(Math.max(360, Math.floor(window.innerHeight - top - 24)));
      redrawNow.current();
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

  // One drawing context and one layer of objects for the life of the view.
  useEffect(() => {
    const gl = canvas.current?.getContext("webgl2", { antialias: true }) ?? null;
    if (gl === null) {
      setFailed(true);
      return;
    }
    try {
      layer.current = new AreaLayer(gl, api, () => redrawNow.current());
    } catch {
      setFailed(true);
    }
    return () => {
      layer.current?.dispose();
      layer.current = null;
      if (pending.current !== null) {
        window.cancelAnimationFrame(pending.current);
        pending.current = null;
      }
    };
  }, [api]);

  // The cell's objects, read anew when another cell is looked at.
  useEffect(() => {
    let gone = false;
    setState("loading");
    setArea(null);
    setPicked(null);
    api
      .interiorObjects(cell)
      .then((found) => {
        if (gone) {
          return;
        }
        const current = layer.current;
        if (found === null || current === null) {
          setState("missing");
          told.current?.(null);
          return;
        }
        current.show(found, lookOf.current);
        setArea(found);
        setState("shown");
        told.current?.(found);
      })
      .catch(() => {
        if (!gone) {
          setState("missing");
          told.current?.(null);
        }
      });
    return () => {
      gone = true;
    };
  }, [api, cell]);

  // The camera goes to the point whenever it or the cell changes.
  useEffect(() => {
    camera.current = framed(area);
    redraw();
  }, [area, framed, redraw]);

  // The objects are drawn anew when what is looked at or hidden changes.
  // biome-ignore lint/correctness/useExhaustiveDependencies: these change how objects look.
  useEffect(() => {
    layer.current?.restyle(lookOf.current);
    redraw();
  }, [focus, hidden, plain, redraw]);

  // biome-ignore lint/correctness/useExhaustiveDependencies: these change the drawing.
  useEffect(() => {
    redraw();
  }, [height, seeInside, redraw]);

  /** The object drawn at a point of the screen, if any. */
  const objectAt = (clientX: number, clientY: number): AreaObject | null => {
    const element = canvas.current;
    const matrix = lastMatrix.current;
    const current = layer.current;
    const gl = element?.getContext("webgl2") ?? null;
    if (element === null || matrix === null || current === null || gl === null) {
      return null;
    }
    const rect = element.getBoundingClientRect();
    const x = Math.floor((clientX - rect.left) * (element.width / Math.max(1, rect.width)));
    const y = Math.floor(
      element.height - (clientY - rect.top) * (element.height / Math.max(1, rect.height)) - 1,
    );
    cull(gl, seeInsideNow.current);
    const found = current.pick(matrix, 1, x, y, element.width, element.height);
    gl.disable(gl.CULL_FACE);
    redraw();
    return found;
  };

  const drag = useRef<{
    x: number;
    y: number;
    turn: boolean;
    moved: boolean;
    camera: Camera;
  } | null>(null);
  const onPointerDown = (event: PointerEvent<HTMLCanvasElement>) => {
    event.currentTarget.setPointerCapture(event.pointerId);
    drag.current = {
      x: event.clientX,
      y: event.clientY,
      turn: event.button === 2 || event.shiftKey || event.ctrlKey,
      moved: false,
      camera: { ...camera.current, target: [...camera.current.target] as Vec3 },
    };
  };
  const onPointerMove = (event: PointerEvent<HTMLCanvasElement>) => {
    const start = drag.current;
    if (start === null) {
      return;
    }
    const dx = event.clientX - start.x;
    const dy = event.clientY - start.y;
    if (Math.abs(dx) + Math.abs(dy) > 3) {
      start.moved = true;
    }
    const from = start.camera;
    if (start.turn) {
      camera.current = {
        ...from,
        yaw: from.yaw - dx * 0.008,
        pitch: Math.min(MAX_PITCH, Math.max(MIN_PITCH, from.pitch + dy * 0.006)),
      };
    } else {
      // One pixel moves the view by as much as it covers at the target.
      const perPixel = (2 * Math.tan(FIELD_OF_VIEW / 2) * from.distance) / Math.max(1, height);
      const forward = [Math.sin(from.yaw), Math.cos(from.yaw)];
      const right = [Math.cos(from.yaw), -Math.sin(from.yaw)];
      const along = dy / Math.max(0.35, Math.abs(Math.sin(from.pitch)));
      camera.current = {
        ...from,
        target: [
          from.target[0] - ((right[0] ?? 0) * dx - (forward[0] ?? 0) * along) * perPixel,
          from.target[1] - ((right[1] ?? 0) * dx - (forward[1] ?? 0) * along) * perPixel,
          from.target[2],
        ],
      };
    }
    redraw();
  };
  const onPointerUp = () => {
    const start = drag.current;
    drag.current = null;
    if (start !== null && !start.moved && !start.turn) {
      const object = objectAt(start.x, start.y);
      setPicked(object);
      onObject?.(object);
      layer.current?.select(
        object === null ? [] : [objectKey(object.id.plugin, object.id.localId)],
      );
      redraw();
    }
  };

  // The wheel zooms; attached directly so that the page does not scroll.
  useEffect(() => {
    const element = canvas.current;
    if (element === null) {
      return;
    }
    const onWheel = (event: WheelEvent) => {
      event.preventDefault();
      camera.current = {
        ...camera.current,
        distance: Math.min(
          8,
          Math.max(MIN_DISTANCE, camera.current.distance * Math.exp(event.deltaY * 0.0012)),
        ),
      };
      redrawNow.current();
    };
    element.addEventListener("wheel", onWheel, { passive: false });
    return () => element.removeEventListener("wheel", onWheel);
  }, []);

  const zoom = (factor: number) => {
    camera.current = {
      ...camera.current,
      distance: Math.min(8, Math.max(MIN_DISTANCE, camera.current.distance * factor)),
    };
    redraw();
  };

  return (
    <div className="map-frame relief-frame" ref={frame} style={{ background: "#0d0f12" }}>
      <canvas
        className="map-canvas relief-canvas"
        ref={canvas}
        style={{ width: "100%", height }}
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerUp}
        onContextMenu={(event) => event.preventDefault()}
      />
      {(failed || state !== "shown") && (
        <p
          className="empty"
          style={{
            position: "absolute",
            inset: 0,
            margin: "auto",
            height: "fit-content",
            textAlign: "center",
          }}
        >
          {failed || state === "missing" ? (
            own.missing(cell)
          ) : (
            <>
              <LoadingMallet size={96} className="loading-centred" />
              {own.loading(cell)}
            </>
          )}
        </p>
      )}
      <div className="view-top">
        <ViewToolbar
          tools={
            <>
              <WandControl level={level} onLevel={setLevel} />
              {tools}
            </>
          }
          camera={
            <>
              <button
                type="button"
                className="map-zoom-button"
                onClick={() => zoom(1 / 1.5)}
                aria-label={t.mapZoomIn}
                title={t.mapZoomIn}
              >
                +
              </button>
              <button
                type="button"
                className="map-zoom-button"
                onClick={() => zoom(1.5)}
                aria-label={t.mapZoomOut}
                title={t.mapZoomOut}
              >
                −
              </button>
              <button
                type="button"
                className="map-zoom-button"
                onClick={() => {
                  camera.current = { ...camera.current, yaw: 0, pitch: MAX_PITCH };
                  redraw();
                }}
              >
                {t.mapTopView}
              </button>
              <button
                type="button"
                className="map-zoom-button"
                onClick={() => {
                  camera.current = framed(area);
                  redraw();
                }}
              >
                {own.fit}
              </button>
            </>
          }
        />
        {picked !== null && (
          <div className="relief-object" role="status">
            <div className="relief-object-head">
              <strong>{picked.name ?? picked.model.split("/").pop()}</strong>
              <button
                type="button"
                className="small quiet"
                aria-label={t.viewerClose}
                onClick={() => {
                  setPicked(null);
                  layer.current?.select([]);
                  redraw();
                }}
              >
                ✕
              </button>
            </div>
            <dl>
              <dt>{t.mapObjectFrom}</dt>
              <dd>{picked.plugin}</dd>
              {picked.id.plugin.toLowerCase() !== picked.plugin.toLowerCase() && (
                <>
                  <dt>{t.mapObjectPlacedBy}</dt>
                  <dd>{picked.id.plugin}</dd>
                </>
              )}
              <dt>{t.mapObjectId}</dt>
              <dd>
                <code>{picked.id.localId.toString(16).padStart(6, "0").toUpperCase()}</code>
              </dd>
              <dt>{t.mapObjectModel}</dt>
              <dd>
                <code>{picked.model}</code>
              </dd>
            </dl>
            {objectExtra?.(picked, {
              selection: selection[0] === picked ? selection : [picked],
              level,
            })}
          </div>
        )}
      </div>
      {children}
      <div className="relief-controls">
        <label className="relief-control" title={own.seeInsideHint}>
          <input
            type="checkbox"
            checked={seeInside}
            onChange={(event) => setSeeInside(event.target.checked)}
          />
          {own.seeInside}
        </label>
        {area !== null && (
          <span className="relief-hint">
            {own.count(area.objects.length, progress.done, progress.total)}
          </span>
        )}
        <InfoTip text={own.hint} above />
      </div>
    </div>
  );
}

/**
 * Leaves out the sides of surfaces that face away from the camera, or
 * draws both sides. Seen from outside, the walls and ceilings of a room
 * face away and are left out, so that one looks in.
 */
function cull(gl: WebGL2RenderingContext, seeInside: boolean) {
  if (seeInside) {
    gl.enable(gl.CULL_FACE);
    gl.cullFace(gl.BACK);
    gl.frontFace(gl.CCW);
  } else {
    gl.disable(gl.CULL_FACE);
  }
}
