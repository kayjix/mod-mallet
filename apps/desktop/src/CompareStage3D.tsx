// Two mods' structures at one place, three times through one camera (ADR
// 0050): the place as it is now, with only the first structure, and with
// only the second. A structure is the parts of one mod that hang together
// (`AreaLayer.connected`), starting from the parts of a collision spot.

import { type PointerEvent, useCallback, useEffect, useRef, useState } from "react";
import { useSession } from "./Actions";
import { AreaLayer, isFlora, LOOK, type Look, objectKey } from "./area-layer";
import type { AreaObject, AreaObjects, ObjectRef } from "./generated/area-objects";
import { colour, lookAt, type Mat4, multiply, perspective, type Vec3 } from "./gl";
import { LandLayer } from "./land-layer";
import { CELL } from "./map-places";
import { isGameObject } from "./object-actions";
import { useReviewTexts } from "./review-texts";

/** Where the structures stand: in a worldspace, or in an interior cell. */
export type StagePlace = { type: "exterior"; world: string } | { type: "interior"; cell: string };

export interface CompareStage3DProps {
  place: StagePlace;
  /** The point between the structures, in game units. */
  centre: [number, number, number];
  /**
   * The spots shown, each with the parts its structures start from: the
   * first mod's and the second's. One spot, or all spots of a cluster.
   */
  groups: StageGroup[];
  /** How far from the point the structures reach, in game units. */
  reach?: number;
  /** How far around the point objects are read, in game units. */
  radius?: number;
  /** Objects to pick out and frame, by {@link objectKey}, such as one segment's. */
  focus?: string[] | null;
  /** The two mods' names. */
  names: [string, string];
  /** What the patch hides now, by {@link objectKey}. */
  hidden: Set<string>;
  /** Told about the two structures whenever they are known better. */
  onStructures?: (structures: [AreaObject[], AreaObject[]]) => void;
  /** Told about the segments whenever they are known better. */
  onSegments?: (segments: StageSegment[]) => void;
  /** The height of the stage, in pixels. */
  height?: number;
}

/** One spot of the stage, with the parts its structures start from. */
export interface StageGroup {
  key: string;
  seeds: [ObjectRef[], ObjectRef[]];
}

/**
 * A segment: the structures of the two mods that hang together around one
 * or more spots, and those spots (ADR 0050).
 */
export interface StageSegment {
  /** The spots, by their groups' keys. */
  groups: string[];
  parts: [AreaObject[], AreaObject[]];
}

/** How long finding the segments rests between two tries while models arrive, in milliseconds. */
const FIND_EVERY = 400;

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
  groups,
  reach = STRUCTURE_REACH,
  radius = RADIUS,
  focus = null,
  names,
  hidden,
  onStructures,
  onSegments,
  height = 420,
}: CompareStage3DProps) {
  const seeds: [ObjectRef[], ObjectRef[]] = [
    groups.flatMap((group) => group.seeds[0]),
    groups.flatMap((group) => group.seeds[1]),
  ];
  const { api } = useSession();
  const r = useReviewTexts();
  const canvas = useRef<HTMLCanvasElement>(null);
  const layer = useRef<AreaLayer | null>(null);
  const land = useRef<LandLayer | null>(null);
  const pending = useRef<number | null>(null);
  const [state, setState] = useState<"loading" | "missing" | "shown">("loading");
  const [markings, setMarkings] = useState(true);
  // Trees around a building hide it; they are left out unless wanted.
  const [trees, setTrees] = useState(false);
  const [failed, setFailed] = useState(false);
  const structures = useRef<[Set<string>, Set<string>]>([new Set(), new Set()]);
  const [counts, setCounts] = useState<[number, number]>([0, 0]);
  /** The other mods with objects among the structures, with their objects' keys. */
  const [others, setOthers] = useState<{ plugin: string; keys: string[] }[]>([]);
  /** The other mod whose objects are picked out, if any. */
  const [shown, setShown] = useState<string | null>(null);
  /** Whether the camera is being moved: drawn at a lower resolution meanwhile. */
  const moving = useRef(false);
  const settle = useRef<number | undefined>(undefined);
  // What is known of the structures, and whether the camera framed them
  // yet: roughly while models are still read, or for good.
  const known = useRef<{
    signature: string;
    framed: "no" | "rough" | "done";
    done: number;
    at: number;
  }>({
    signature: "",
    framed: "no",
    done: -1,
    at: 0,
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
  const toldSegments = useRef(onSegments);
  toldSegments.current = onSegments;
  const reachNow = useRef(reach);
  reachNow.current = reach;
  const groupsNow = useRef(groups);
  groupsNow.current = groups;
  const markingsNow = useRef(markings);
  markingsNow.current = markings;
  const treesNow = useRef(trees);
  treesNow.current = trees;
  /** The trees, plants and grass of the place, by key. */
  const flora = useRef<Set<string>>(new Set());
  const hiddenNow = useRef(hidden);
  hiddenNow.current = hidden;
  const seedsNow = useRef(seeds);
  seedsNow.current = seeds;

  /** How an object looks in view `index`: now, only the first, only the second. */
  const lookIn = (index: 0 | 1 | 2) =>
    lookFor(
      index,
      structures.current,
      markingsNow.current,
      hiddenNow.current,
      treesNow.current ? null : flora.current,
    );
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
    const complete = progress.done === progress.total;
    // Finding the segments costs; while models still arrive it rests a
    // moment between two tries.
    if (
      progress.done === known.current.done ||
      (!complete && known.current.done >= 0 && Date.now() - known.current.at < FIND_EVERY)
    ) {
      return;
    }
    known.current.done = progress.done;
    known.current.at = Date.now();
    const [x, y] = centreNow.current;
    const near = { x, y, radius: reachNow.current };
    // What grows from each seed, once per seed.
    const grown = new Map<string, AreaObject[]>();
    const growFrom = (seed: AreaObject) => {
      const key = objectKey(seed.id.plugin, seed.id.localId);
      let parts = grown.get(key);
      if (parts === undefined) {
        parts = current.connected(seed, STRUCTURE_MARGIN, near);
        for (const part of parts) {
          // A part reached from another seed of the same kind grows the same.
          grown.set(objectKey(part.id.plugin, part.id.localId), parts);
        }
        grown.set(key, parts);
      }
      return parts;
    };
    const grow = (from: AreaObject[]) => {
      const found = new Map<string, AreaObject>();
      for (const seed of from) {
        for (const part of growFrom(seed)) {
          found.set(objectKey(part.id.plugin, part.id.localId), part);
        }
      }
      return [...found.values()];
    };
    const perGroup = groupsNow.current.map((group) => ({
      key: group.key,
      parts: [grow(seedObjects(area, group.seeds[0])), grow(seedObjects(area, group.seeds[1]))] as [
        AreaObject[],
        AreaObject[],
      ],
    }));
    const segments = segmentsOf(perGroup);
    const first = [
      ...new Map(
        segments
          .flatMap((segment) => segment.parts[0])
          .map((part) => [objectKey(part.id.plugin, part.id.localId), part]),
      ).values(),
    ];
    const second = [
      ...new Map(
        segments
          .flatMap((segment) => segment.parts[1])
          .map((part) => [objectKey(part.id.plugin, part.id.localId), part]),
      ).values(),
    ];
    const now = `${signature(first)}|${signature(second)}`;
    if (now !== known.current.signature) {
      toldSegments.current?.(segments);
      known.current.signature = now;
      structures.current = [
        new Set(first.map((part) => objectKey(part.id.plugin, part.id.localId))),
        new Set(second.map((part) => objectKey(part.id.plugin, part.id.localId))),
      ];
      setCounts([first.length, second.length]);
      told.current?.([first, second]);
      setOthers(othersAmong(area, structures.current, seedsNow.current));
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
    const ratio = Math.min(1.5, window.devicePixelRatio || 1) * (moving.current ? 0.5 : 1);
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
    known.current = { signature: "", framed: "no", done: -1, at: 0 };
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
        land.current.load(cx, cy, radius);
      } catch {
        land.current = null;
      }
    }
    const read =
      place.type === "exterior"
        ? api.areaObjects(place.world, cx, cy, radius)
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
        flora.current = new Set(
          found.objects
            .filter((object) => isFlora(object.model))
            .map((object) => objectKey(object.id.plugin, object.id.localId)),
        );
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
  }, [markings, trees, hidden, height, redraw]);

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
    moving.current = true;
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
    rest();
  };
  /** Back to the full resolution a moment after the camera stops. */
  const rest = () => {
    window.clearTimeout(settle.current);
    settle.current = window.setTimeout(() => {
      moving.current = false;
      redrawNow.current();
    }, 180);
  };
  const restNow = useRef(rest);
  restNow.current = rest;

  // The wheel zooms; attached directly so that the page does not scroll.
  useEffect(() => {
    const element = canvas.current;
    if (element === null) {
      return;
    }
    const onWheel = (event: WheelEvent) => {
      event.preventDefault();
      moved.current = true;
      moving.current = true;
      restNow.current();
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

  const focusKey = focus === null ? "" : focus.join(",");
  // biome-ignore lint/correctness/useExhaustiveDependencies: the keys belong to the mod shown.
  useEffect(() => {
    const picked = focusKey === "" ? null : focusKey.split(",");
    layer.current?.select(picked ?? others.find((other) => other.plugin === shown)?.keys ?? []);
    const current = layer.current;
    if (picked !== null && current !== null) {
      const box = boxOf(current, [new Set(picked), new Set()]);
      if (box !== null) {
        camera.current = framing(box, camera.current, aspectNow.current);
      }
    }
    redraw();
  }, [shown, focusKey, redraw]);

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
        <button
          type="button"
          className="small stage3-eye"
          aria-pressed={markings}
          title={r.markingsHint}
          onClick={() => setMarkings(!markings)}
        >
          <Eye open={markings} /> {r.markings}
        </button>
        <label className="control-check">
          <input
            type="checkbox"
            checked={trees}
            onChange={(event) => setTrees(event.target.checked)}
          />
          {r.showTrees}
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
      {markings && (
        <p className="stage3-legend aside">
          <span className="stage3-key" data-key="hidden" /> {r.legendHidden}
        </p>
      )}
      {others.length > 0 && (
        <div className="stage3-others">
          <span className="aside">{r.othersHere(others.length)}</span>
          {others.map((other) => (
            <button
              type="button"
              key={other.plugin}
              className="small stage3-other"
              aria-pressed={shown === other.plugin}
              title={r.othersShow(shortName(other.plugin))}
              onClick={() => setShown(shown === other.plugin ? null : other.plugin)}
            >
              {r.othersCount(shortName(other.plugin), other.keys.length)}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

/**
 * The spots' structures joined where they share a part: each join is one
 * segment, with the spots it covers and the parts of both mods.
 */
function segmentsOf(
  perGroup: { key: string; parts: [AreaObject[], AreaObject[]] }[],
): StageSegment[] {
  const owner = new Map<string, number>();
  const root = perGroup.map((_, index) => index);
  const find = (index: number): number => {
    let at = index;
    while (root[at] !== at) {
      at = root[at] ?? at;
    }
    return at;
  };
  perGroup.forEach((group, index) => {
    for (const part of [...group.parts[0], ...group.parts[1]]) {
      const key = objectKey(part.id.plugin, part.id.localId);
      const before = owner.get(key);
      if (before === undefined) {
        owner.set(key, index);
      } else {
        root[find(index)] = find(before);
      }
    }
  });
  const joined = new Map<number, StageSegment>();
  perGroup.forEach((group, index) => {
    const at = find(index);
    const segment = joined.get(at) ?? { groups: [], parts: [[], []] };
    segment.groups.push(group.key);
    for (const side of [0, 1] as const) {
      const known = new Set(
        segment.parts[side].map((part) => objectKey(part.id.plugin, part.id.localId)),
      );
      for (const part of group.parts[side]) {
        if (!known.has(objectKey(part.id.plugin, part.id.localId))) {
          segment.parts[side].push(part);
        }
      }
    }
    joined.set(at, segment);
  });
  return [...joined.values()];
}

/** A plugin's name without its extension. */
const shortName = (plugin: string) => plugin.replace(/\.(esp|esm|esl)$/i, "");

/** How far around the structures other mods' objects count as being there, in game units (4 m). */
const AROUND = 280;

/**
 * The mods besides the two whose objects stand among the structures (in
 * the box around them, a little wider), the most first: at a town's gate
 * a third and fourth mod may have built there too. The game's own objects
 * and trees do not count.
 */
function othersAmong(
  area: AreaObjects,
  [first, second]: [Set<string>, Set<string>],
  seeds: [ObjectRef[], ObjectRef[]],
): { plugin: string; keys: string[] }[] {
  const inside = area.objects.filter((object) =>
    [first, second].some((set) => set.has(objectKey(object.id.plugin, object.id.localId))),
  );
  if (inside.length === 0) {
    return [];
  }
  const low = [0, 1, 2].map((axis) => Math.min(...inside.map((o) => o.position[axis] ?? 0)));
  const high = [0, 1, 2].map((axis) => Math.max(...inside.map((o) => o.position[axis] ?? 0)));
  const sides = new Set(seeds.flat().map((seed) => seed.plugin.toLowerCase()));
  const found = new Map<string, string[]>();
  for (const object of area.objects) {
    const plugin = object.id.plugin;
    if (sides.has(plugin.toLowerCase()) || isGameObject(object) || isFlora(object.model)) {
      continue;
    }
    const within = [0, 1, 2].every(
      (axis) =>
        (object.position[axis] ?? 0) >= (low[axis] ?? 0) - AROUND &&
        (object.position[axis] ?? 0) <= (high[axis] ?? 0) + AROUND,
    );
    if (within) {
      found.set(plugin, [...(found.get(plugin) ?? []), objectKey(plugin, object.id.localId)]);
    }
  }
  return [...found.entries()]
    .map(([plugin, keys]) => ({ plugin, keys }))
    .sort((a, b) => b.keys.length - a.keys.length || a.plugin.localeCompare(b.plugin));
}

/** An eye, open or struck through, as on the collisions page. */
function Eye({ open }: { open: boolean }) {
  return (
    <svg className="eye" viewBox="0 0 24 24" aria-hidden="true">
      <path d="M2 12s3.6-7 10-7 10 7 10 7-3.6 7-10 7S2 12 2 12z" />
      <circle cx="12" cy="12" r="3" />
      {!open && <path d="M4 4l16 16" />}
    </svg>
  );
}

/**
 * How an object looks in view `index` (now, only the first structure,
 * only the second): the other structure left out, what the patch hides as
 * a ghost or left out, the two structures in their colours, and the trees
 * in `flora` left out (unless they are part of a structure).
 */
function lookFor(
  index: 0 | 1 | 2,
  [first, second]: [Set<string>, Set<string>],
  marks: boolean,
  hidden: Set<string>,
  flora: Set<string> | null,
): (key: string) => Look {
  return (key) => {
    const inFirst = first.has(key);
    const inSecond = second.has(key) && !inFirst;
    if (flora?.has(key) && !inFirst && !inSecond) {
      return LOOK.gone;
    }
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
 * How much room the camera leaves around the structures, in game units
 * (about 12 m): enough to see where they stand, not only themselves.
 */
const ROOM = 850;

/**
 * The box around both structures' parts, from where their models' middles
 * stand, widened by some room around them; none before they are known.
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
  return [
    [low[0] - ROOM, low[1] - ROOM, low[2] - 120],
    [high[0] + ROOM, high[1] + ROOM, high[2] + 120],
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
