// Two mods' structures at one place, three times through one camera (ADR
// 0050): the place as it is now, with only the first structure, and with
// only the second. A structure is the parts of one mod that hang together
// (`AreaLayer.connected`), starting from the parts of a collision spot.

import { type PointerEvent, useCallback, useEffect, useRef, useState } from "react";
import { useSession } from "./Actions";
import { AreaLayer, LOOK, type Look, objectKey } from "./area-layer";
import type { AreaObject, AreaObjects, ObjectRef } from "./generated/area-objects";
import { colour, lookAt, type Mat4, multiply, perspective, type Vec3 } from "./gl";
import { LandLayer } from "./land-layer";
import { CELL } from "./map-places";
import { useReviewTexts } from "./review-texts";

/** Where the structures stand: in a worldspace, or in an interior cell. */
export type StagePlace = { type: "exterior"; world: string } | { type: "interior"; cell: string };

export interface CompareStage3DProps {
  place: StagePlace;
  /** The point between the structures, in game units. */
  centre: [number, number, number];
  /** The parts each structure starts from: the first mod's, the second's. */
  seeds: [ObjectRef[], ObjectRef[]];
  /** The two mods' names. */
  names: [string, string];
  /** What the patch hides now, by {@link objectKey}. */
  hidden: Set<string>;
  /** Told about the two structures whenever they are known better. */
  onStructures?: (structures: [AreaObject[], AreaObject[]]) => void;
  /** The height of the stage, in pixels. */
  height?: number;
}

/** How far around the point objects are read, in game units (about two cells across). */
const RADIUS = CELL;
const FIELD_OF_VIEW = (40 * Math.PI) / 180;
const MIN_PITCH = -0.2;
const MAX_PITCH = 1.55;
const MIN_DISTANCE = 0.005;
const START_DISTANCE = 0.25;
/** How close two parts must come to count as one structure, in game units (about 6 cm). */
const STRUCTURE_MARGIN = 4;
/**
 * How far from the spot a structure reaches, in game units (about 30 m):
 * the walls of a whole town touch each other, but the user decides the
 * gate, not the town.
 */
const STRUCTURE_REACH = 2048;
/** The angle the camera looks down at the structures from, above the horizon. */
const FRAMING_PITCH = 0.8;

/** The first mod's colour and the second's: blue and orange (Okabe–Ito). */
const FIRST: Vec3 = [0, 0.447, 0.698];
const SECOND: Vec3 = [0.902, 0.624, 0];

interface Camera {
  target: Vec3;
  distance: number;
  yaw: number;
  pitch: number;
}

/** The objects of `area` that a structure starts from. */
function seedObjects(area: AreaObjects, seeds: ObjectRef[]): AreaObject[] {
  const wanted = new Set(seeds.map((seed) => objectKey(seed.plugin, seed.localId)));
  return area.objects.filter((object) =>
    wanted.has(objectKey(object.id.plugin, object.id.localId)),
  );
}

/** A structure's parts by key, for comparing two findings of it. */
const signature = (parts: AreaObject[]) =>
  parts
    .map((part) => objectKey(part.id.plugin, part.id.localId))
    .sort()
    .join(",");

export function CompareStage3D({
  place,
  centre,
  seeds,
  names,
  hidden,
  onStructures,
  height = 420,
}: CompareStage3DProps) {
  const { api } = useSession();
  const r = useReviewTexts();
  const canvas = useRef<HTMLCanvasElement>(null);
  const layer = useRef<AreaLayer | null>(null);
  const land = useRef<LandLayer | null>(null);
  const pending = useRef<number | null>(null);
  const [state, setState] = useState<"loading" | "missing" | "shown">("loading");
  const [markings, setMarkings] = useState(true);
  const [failed, setFailed] = useState(false);
  const structures = useRef<[Set<string>, Set<string>]>([new Set(), new Set()]);
  const [counts, setCounts] = useState<[number, number]>([0, 0]);
  // What is known of the structures, and whether the camera framed them
  // yet: roughly while models are still read, or for good.
  const known = useRef<{ signature: string; framed: "no" | "rough" | "done"; done: number }>({
    signature: "",
    framed: "no",
    done: -1,
  });
  /** Whether the user moved the camera, which framing then leaves alone. */
  const moved = useRef(false);
  const centreNow = useRef(centre);
  centreNow.current = centre;
  /** The width of one of the three views over its height. */
  const aspectNow = useRef(0.7);
  const camera = useRef<Camera>({
    target: [centre[0] / CELL, centre[1] / CELL, centre[2] / CELL],
    distance: START_DISTANCE,
    yaw: 0.6,
    pitch: 0.55,
  });
  const told = useRef(onStructures);
  told.current = onStructures;
  const markingsNow = useRef(markings);
  markingsNow.current = markings;
  const hiddenNow = useRef(hidden);
  hiddenNow.current = hidden;
  const seedsNow = useRef(seeds);
  seedsNow.current = seeds;

  /** How an object looks in view `index`: now, only the first, only the second. */
  const lookIn = (index: 0 | 1 | 2) =>
    lookFor(index, structures.current, markingsNow.current, hiddenNow.current);
  const lookNow = useRef(lookIn);
  lookNow.current = lookIn;

  /** Finds the two structures anew once more of the area's models are read. */
  const findStructures = useCallback(() => {
    const current = layer.current;
    const area = current?.area;
    if (current === null || area == null) {
      return;
    }
    const progress = current.progress;
    if (progress.done === known.current.done) {
      return;
    }
    known.current.done = progress.done;
    const [firstSeeds, secondSeeds] = seedsNow.current;
    const grow = (from: AreaObject[]) => {
      const found = new Map<string, AreaObject>();
      for (const seed of from) {
        const [x, y] = centreNow.current;
        const near = { x, y, radius: STRUCTURE_REACH };
        for (const part of current.connected(seed, STRUCTURE_MARGIN, near)) {
          found.set(objectKey(part.id.plugin, part.id.localId), part);
        }
      }
      return [...found.values()];
    };
    const first = grow(seedObjects(area, firstSeeds));
    const second = grow(seedObjects(area, secondSeeds));
    const now = `${signature(first)}|${signature(second)}`;
    if (now !== known.current.signature) {
      known.current.signature = now;
      structures.current = [
        new Set(first.map((part) => objectKey(part.id.plugin, part.id.localId))),
        new Set(second.map((part) => objectKey(part.id.plugin, part.id.localId))),
      ];
      setCounts([first.length, second.length]);
      told.current?.([first, second]);
    }
    // The camera frames both structures roughly as soon as they are known,
    // and once more when every model is read, unless the user moved it.
    const wanted = progress.done === progress.total ? "done" : "rough";
    if (!moved.current && known.current.framed !== "done" && known.current.framed !== wanted) {
      const box = boxOf(current, structures.current);
      if (box !== null) {
        known.current.framed = wanted;
        camera.current = framing(box, camera.current, aspectNow.current);
      }
    }
  }, []);

  const draw = useCallback(() => {
    pending.current = null;
    const element = canvas.current;
    const current = layer.current;
    const gl = element?.getContext("webgl2") ?? null;
    if (element === null || gl === null) {
      return;
    }
    findStructures();
    const ratio = window.devicePixelRatio || 1;
    const width = Math.max(3, element.clientWidth);
    const tall = Math.max(1, element.clientHeight);
    const wanted = [Math.round(width * ratio), Math.round(tall * ratio)];
    if (element.width !== wanted[0] || element.height !== wanted[1]) {
      element.width = wanted[0] ?? element.width;
      element.height = wanted[1] ?? element.height;
    }
    const view = camera.current;
    const level = Math.cos(view.pitch) * view.distance;
    const eye: Vec3 = [
      view.target[0] - Math.sin(view.yaw) * level,
      view.target[1] - Math.cos(view.yaw) * level,
      view.target[2] + Math.sin(view.pitch) * view.distance,
    ];
    const third = gl.drawingBufferWidth / 3;
    aspectNow.current = width / 3 / tall;
    const matrix: Mat4 = multiply(
      perspective(FIELD_OF_VIEW, width / 3 / tall, view.distance * 0.02, view.distance * 8 + 2),
      lookAt(eye, view.target, [0, 0, 1]),
    );
    const style = getComputedStyle(element);
    const ghost = colour(style.getPropertyValue("--spot-ghost"));
    gl.enable(gl.SCISSOR_TEST);
    gl.enable(gl.DEPTH_TEST);
    for (const index of [0, 1, 2] as const) {
      const left = Math.round(third * index);
      const span = Math.round(third * (index + 1)) - left;
      gl.viewport(left, 0, span, gl.drawingBufferHeight);
      gl.scissor(left, 0, span, gl.drawingBufferHeight);
      const shade = index === 0 ? 0.07 : 0.055;
      gl.clearColor(shade, shade + 0.01, shade + 0.02, 1);
      gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
      land.current?.draw(matrix, 1);
      if (current !== null) {
        current.restyle(lookNow.current(index));
        current.draw(matrix, 1, { mark: SECOND, focus: FIRST, ghost });
      }
    }
    gl.disable(gl.SCISSOR_TEST);
  }, [findStructures]);

  const redraw = useCallback(() => {
    if (pending.current === null) {
      pending.current = window.requestAnimationFrame(draw);
    }
  }, [draw]);
  const redrawNow = useRef(redraw);
  redrawNow.current = redraw;
  // The structures are found as the models arrive, whether or not the
  // stage is drawn just then (a hidden window draws nothing).
  const findNow = useRef(findStructures);
  findNow.current = findStructures;

  // One drawing context with its layers for the life of the stage.
  useEffect(() => {
    const gl = canvas.current?.getContext("webgl2", { antialias: true }) ?? null;
    if (gl === null) {
      setFailed(true);
      return;
    }
    try {
      layer.current = new AreaLayer(gl, api, () => {
        findNow.current();
        redrawNow.current();
      });
    } catch {
      setFailed(true);
    }
    return () => {
      layer.current?.dispose();
      layer.current = null;
      land.current?.dispose();
      land.current = null;
      if (pending.current !== null) {
        window.cancelAnimationFrame(pending.current);
        pending.current = null;
      }
    };
  }, [api]);

  // What stands around the point, read anew for another place.
  const placeKey = place.type === "exterior" ? `w:${place.world}` : `i:${place.cell}`;
  const [cx, cy, cz] = centre;
  // biome-ignore lint/correctness/useExhaustiveDependencies: the place is compared by its key.
  useEffect(() => {
    let gone = false;
    const current = layer.current;
    const gl = canvas.current?.getContext("webgl2") ?? null;
    setState("loading");
    structures.current = [new Set(), new Set()];
    known.current = { signature: "", framed: "no", done: -1 };
    moved.current = false;
    setCounts([0, 0]);
    camera.current = {
      target: [cx / CELL, cy / CELL, cz / CELL],
      distance: START_DISTANCE,
      yaw: 0.6,
      pitch: 0.55,
    };
    land.current?.dispose();
    land.current = null;
    if (place.type === "exterior" && gl !== null) {
      try {
        land.current = new LandLayer(gl, api, place.world, () => redrawNow.current());
        land.current.load(cx, cy, RADIUS);
      } catch {
        land.current = null;
      }
    }
    const read =
      place.type === "exterior"
        ? api.areaObjects(place.world, cx, cy, RADIUS)
        : api.interiorObjects(place.cell);
    read
      .then((found) => {
        if (gone || current === null) {
          return;
        }
        if (found === null) {
          setState("missing");
          return;
        }
        current.show(found, lookNow.current(0));
        findNow.current();
        setState("shown");
        redrawNow.current();
      })
      .catch(() => {
        if (!gone) {
          setState("missing");
        }
      });
    return () => {
      gone = true;
    };
  }, [api, placeKey, cx, cy, cz]);

  // biome-ignore lint/correctness/useExhaustiveDependencies: these change how objects look.
  useEffect(() => {
    redraw();
  }, [markings, hidden, height, redraw]);

  // The canvas follows the width of its frame.
  useEffect(() => {
    const element = canvas.current;
    if (element === null) {
      return;
    }
    const observer = new ResizeObserver(() => redrawNow.current());
    observer.observe(element);
    return () => observer.disconnect();
  }, []);

  const drag = useRef<{ x: number; y: number; move: boolean; camera: Camera } | null>(null);
  const onPointerDown = (event: PointerEvent<HTMLCanvasElement>) => {
    event.currentTarget.setPointerCapture(event.pointerId);
    drag.current = {
      x: event.clientX,
      y: event.clientY,
      move: event.button === 2 || event.shiftKey,
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
    moved.current = true;
    const from = start.camera;
    if (!start.move) {
      camera.current = {
        ...from,
        yaw: from.yaw - dx * 0.008,
        pitch: Math.min(MAX_PITCH, Math.max(MIN_PITCH, from.pitch + dy * 0.006)),
      };
    } else {
      const perPixel = (2 * Math.tan(FIELD_OF_VIEW / 2) * from.distance) / Math.max(1, height);
      const forward = [Math.sin(from.yaw), Math.cos(from.yaw)];
      const right = [Math.cos(from.yaw), -Math.sin(from.yaw)];
      camera.current = {
        ...from,
        target: [
          from.target[0] - ((right[0] ?? 0) * dx - (forward[0] ?? 0) * dy) * perPixel,
          from.target[1] - ((right[1] ?? 0) * dx - (forward[1] ?? 0) * dy) * perPixel,
          from.target[2],
        ],
      };
    }
    redraw();
  };
  const onPointerUp = () => {
    drag.current = null;
  };

  // The wheel zooms; attached directly so that the page does not scroll.
  useEffect(() => {
    const element = canvas.current;
    if (element === null) {
      return;
    }
    const onWheel = (event: WheelEvent) => {
      event.preventDefault();
      moved.current = true;
      camera.current = {
        ...camera.current,
        distance: Math.min(
          4,
          Math.max(MIN_DISTANCE, camera.current.distance * Math.exp(event.deltaY * 0.0012)),
        ),
      };
      redrawNow.current();
    };
    element.addEventListener("wheel", onWheel, { passive: false });
    return () => element.removeEventListener("wheel", onWheel);
  }, []);

  const fit = () => {
    const current = layer.current;
    const box = current === null ? null : boxOf(current, structures.current);
    if (box !== null) {
      camera.current = framing(box, camera.current, aspectNow.current);
      redraw();
    }
  };

  const labels = [r.viewNow, r.viewOnly(names[0]), r.viewOnly(names[1])];
  return (
    <div className="stage3">
      <div className="stage3-frame">
        <canvas
          ref={canvas}
          className="stage3-canvas"
          style={{ height }}
          onPointerDown={onPointerDown}
          onPointerMove={onPointerMove}
          onPointerUp={onPointerUp}
          onContextMenu={(event) => event.preventDefault()}
        />
        <div className="stage3-labels" aria-hidden="true">
          {labels.map((label, index) => (
            <span key={label} className="stage3-label" data-view={index}>
              {label}
            </span>
          ))}
        </div>
        {(failed || state !== "shown") && (
          <p className="stage3-state">{failed || state === "missing" ? r.missing : r.loading}</p>
        )}
      </div>
      <div className="stage3-controls">
        <label className="control-check">
          <input
            type="checkbox"
            checked={markings}
            onChange={(event) => setMarkings(event.target.checked)}
          />
          {r.markings}
        </label>
        <span className="stage3-swatch" data-side="0">
          {r.parts(names[0], counts[0])}
        </span>
        <span className="stage3-swatch" data-side="1">
          {r.parts(names[1], counts[1])}
        </span>
        <span className="spacer" />
        <button type="button" className="small quiet" onClick={fit}>
          {r.fit}
        </button>
        <button
          type="button"
          className="small quiet"
          onClick={() => {
            camera.current = { ...camera.current, yaw: 0, pitch: MAX_PITCH };
            redraw();
          }}
        >
          {r.topView}
        </button>
        <span className="aside stage3-hint">{r.stageHint}</span>
      </div>
    </div>
  );
}

/**
 * How an object looks in view `index` (now, only the first structure,
 * only the second): the other structure left out, what the patch hides as
 * a ghost or left out, the two structures in their colours.
 */
function lookFor(
  index: 0 | 1 | 2,
  [first, second]: [Set<string>, Set<string>],
  marks: boolean,
  hidden: Set<string>,
): (key: string) => Look {
  return (key) => {
    const inFirst = first.has(key);
    const inSecond = second.has(key) && !inFirst;
    if ((index === 1 && inSecond) || (index === 2 && inFirst)) {
      return LOOK.gone;
    }
    const shownAnyway = (index === 1 && inFirst) || (index === 2 && inSecond);
    if (hidden.has(key) && !shownAnyway) {
      return marks ? LOOK.ghost : LOOK.gone;
    }
    if (!marks) {
      return LOOK.plain;
    }
    return inFirst ? LOOK.focus : inSecond ? LOOK.marked : LOOK.plain;
  };
}

/** A box in game units: lowest and highest corner. */
type Box = [Vec3, Vec3];

/**
 * The box around both structures' parts, from where their models' middles
 * stand, widened a little for the models' size; none before they are known.
 */
function boxOf(layer: AreaLayer, [first, second]: [Set<string>, Set<string>]): Box | null {
  const low: Vec3 = [Infinity, Infinity, Infinity];
  const high: Vec3 = [-Infinity, -Infinity, -Infinity];
  for (const { object, centre } of layer.shown) {
    const key = objectKey(object.id.plugin, object.id.localId);
    if (!first.has(key) && !second.has(key)) {
      continue;
    }
    for (const axis of [0, 1, 2] as const) {
      low[axis] = Math.min(low[axis], centre[axis]);
      high[axis] = Math.max(high[axis], centre[axis]);
    }
  }
  if (!Number.isFinite(low[0])) {
    return null;
  }
  const pad = 120;
  return [
    [low[0] - pad, low[1] - pad, low[2] - pad],
    [high[0] + pad, high[1] + pad, high[2] + pad],
  ];
}

/**
 * The camera turned as before, looking down at an angle at the middle of
 * `box` from far enough that all of it fits into a view `aspect` wide
 * for each unit high.
 */
function framing([low, high]: Box, from: Camera, aspect: number): Camera {
  const middle: Vec3 = [(low[0] + high[0]) / 2, (low[1] + high[1]) / 2, (low[2] + high[2]) / 2];
  const radius = Math.hypot(high[0] - low[0], high[1] - low[1], high[2] - low[2]) / 2;
  const half = FIELD_OF_VIEW / 2;
  const across = Math.atan(Math.tan(half) * Math.max(0.2, aspect));
  const distance = (radius / Math.sin(Math.min(half, across))) * 1.05;
  return {
    ...from,
    target: [middle[0] / CELL, middle[1] / CELL, middle[2] / CELL],
    distance: Math.max(MIN_DISTANCE, distance / CELL),
    pitch: FRAMING_PITCH,
  };
}
