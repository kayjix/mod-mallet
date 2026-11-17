import { type PointerEvent, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useSession } from "./Actions";
import type { MapMarker, MapSpot, MapTerrain, MapWorld } from "./generated/rendered-report";
import {
  colour,
  lookAt,
  type Mat4,
  multiply,
  perspective,
  program,
  project,
  type Vec3,
} from "./gl";
import {
  CELL,
  drawLabels,
  isMajor,
  type Label,
  type Picked,
  type Picture,
  readLand,
  samePick,
  shapeOf,
  spotPosition,
} from "./map-places";

interface MapView3DProps {
  world: MapWorld;
  terrain: MapTerrain;
  picture: Picture | null;
  owner: number;
  /** Whether the cells with objects of mods are tinted. */
  showCells: boolean;
  markers: MapMarker[];
  spots: MapSpot[];
  picked: Picked | null;
  hover: Picked | null;
  onPick: (picked: Picked | null) => void;
  onHover: (hover: Picked | null) => void;
}

/**
 * Where the camera is: it looks at `target` (in cells, height in cells as
 * drawn) from `distance` cells away, turned by `yaw` from north and
 * raised by `pitch` above the horizon.
 */
interface Camera {
  target: Vec3;
  distance: number;
  yaw: number;
  pitch: number;
}

const FIELD_OF_VIEW = (45 * Math.PI) / 180;
const MIN_PITCH = 0.2;
const MAX_PITCH = 1.55;
/** How tall a pin stands, in cells. */
const PIN = 0.9;
/** How close to a pin a click has to land, in pixels. */
const HIT = 11;

/** The land as triangles: positions, heights, kinds and indices. */
interface Mesh {
  columns: number;
  rows: number;
  /** East and north of each sample, in cells. */
  xy: Float32Array;
  /** Height of each sample, in game units. */
  heights: Float32Array;
  /** 0 where nothing is known, 1 water, 2 land, 3 the walls of the base. */
  kinds: Float32Array;
  /** The land's triangles first, then the walls'. */
  indices: Uint32Array;
  /** How many indices the land has, before the walls'. */
  landIndices: number;
  /** The normals of the walls' points, which do not change with the height. */
  wallNormals: Float32Array;
}

function buildMesh(terrain: MapTerrain): Mesh | null {
  const land = readLand(terrain);
  if (land === null || land.width < 2 || land.height < 2) {
    return null;
  }
  const columns = land.width;
  const rows = land.height;
  const north = terrain.south + terrain.rows;
  const xy = new Float32Array(columns * rows * 2);
  const heights = land.heights;
  const kinds = Float32Array.from(land.kinds);
  for (let row = 0; row < rows; row++) {
    for (let column = 0; column < columns; column++) {
      const index = row * columns + column;
      xy[index * 2] = terrain.west + (column + 0.5) / terrain.samples;
      xy[index * 2 + 1] = north - (row + 0.5) / terrain.samples;
    }
  }
  const ground = new Uint32Array((columns - 1) * (rows - 1) * 6);
  let at = 0;
  for (let row = 0; row < rows - 1; row++) {
    for (let column = 0; column < columns - 1; column++) {
      const a = row * columns + column;
      const b = a + 1;
      const c = a + columns;
      const d = c + 1;
      ground.set([a, c, b, b, c, d], at);
      at += 6;
    }
  }

  // The base: a wall along each side, from the edge of the land down to a
  // little below the lowest point, like a model on a plinth.
  const span = Math.max(1, terrain.highest - terrain.lowest);
  const floor = terrain.lowest - Math.max(span * 0.06, CELL);
  const sides: { samples: number[]; normal: [number, number] }[] = [
    { samples: range(columns, (column) => column), normal: [0, 1] },
    { samples: range(columns, (column) => (rows - 1) * columns + column), normal: [0, -1] },
    { samples: range(rows, (row) => row * columns), normal: [-1, 0] },
    { samples: range(rows, (row) => row * columns + columns - 1), normal: [1, 0] },
  ];
  const wallXy: number[] = [];
  const wallHeights: number[] = [];
  const wallNormals: number[] = [];
  const walls: number[] = [];
  let next = columns * rows;
  for (const side of sides) {
    side.samples.forEach((sample, index) => {
      for (const height of [heights[sample] ?? terrain.lowest, floor]) {
        wallXy.push(xy[sample * 2] ?? 0, xy[sample * 2 + 1] ?? 0);
        wallHeights.push(height);
        wallNormals.push(side.normal[0], side.normal[1], 0);
      }
      if (index > 0) {
        const [top, bottom] = [next + index * 2, next + index * 2 + 1];
        walls.push(top - 2, bottom - 2, top, top, bottom - 2, bottom);
      }
    });
    next += side.samples.length * 2;
  }
  const join = (a: Float32Array, b: number[]) => {
    const out = new Float32Array(a.length + b.length);
    out.set(a);
    out.set(b, a.length);
    return out;
  };
  return {
    columns,
    rows,
    xy: join(xy, wallXy),
    heights: join(heights, wallHeights),
    kinds: join(
      kinds,
      wallHeights.map(() => 3),
    ),
    indices: joinIndices(ground, walls),
    landIndices: ground.length,
    wallNormals: Float32Array.from(wallNormals),
  };
}

/** The land's triangles followed by the walls'. */
function joinIndices(ground: Uint32Array, walls: number[]): Uint32Array {
  const out = new Uint32Array(ground.length + walls.length);
  out.set(ground);
  out.set(walls, ground.length);
  return out;
}

/** The numbers `make(0)` to `make(count - 1)`. */
function range(count: number, make: (index: number) => number): number[] {
  return Array.from({ length: count }, (_, index) => make(index));
}

/**
 * The normal of each sample for heights raised `exaggeration` times,
 * followed by those of the walls.
 */
function normals(mesh: Mesh, samples: number, exaggeration: number): Float32Array {
  const { columns, rows, heights } = mesh;
  const out = new Float32Array(columns * rows * 3 + mesh.wallNormals.length);
  out.set(mesh.wallNormals, columns * rows * 3);
  const step = 2 / samples;
  const height = (column: number, row: number) =>
    ((heights[
      Math.min(rows - 1, Math.max(0, row)) * columns + Math.min(columns - 1, Math.max(0, column))
    ] ?? 0) /
      CELL) *
    exaggeration;
  for (let row = 0; row < rows; row++) {
    for (let column = 0; column < columns; column++) {
      // Rows run from north to south.
      const dx = (height(column + 1, row) - height(column - 1, row)) / step;
      const dy = (height(column, row - 1) - height(column, row + 1)) / step;
      const length = Math.hypot(dx, dy, 1);
      const index = (row * columns + column) * 3;
      out[index] = -dx / length;
      out[index + 1] = -dy / length;
      out[index + 2] = 1 / length;
    }
  }
  return out;
}

const TERRAIN_VERTEX = `#version 300 es
in vec2 a_xy;
in float a_height;
in float a_kind;
in vec3 a_normal;
uniform mat4 u_matrix;
uniform float u_exaggeration;
out vec2 v_xy;
out float v_share;
out float v_kind;
out vec3 v_normal;
uniform float u_lowest;
uniform float u_span;
void main() {
  v_xy = a_xy;
  v_share = clamp((a_height - u_lowest) / u_span, 0.0, 1.0);
  v_kind = a_kind;
  v_normal = a_normal;
  gl_Position = u_matrix * vec4(a_xy, a_height / ${CELL}.0 * u_exaggeration, 1.0);
}`;

const TERRAIN_FRAGMENT = `#version 300 es
precision highp float;
in vec2 v_xy;
in float v_share;
in float v_kind;
in vec3 v_normal;
uniform bool u_hasPicture;
uniform sampler2D u_picture;
uniform vec4 u_pictureRect;
uniform bool u_showCells;
uniform sampler2D u_cells;
uniform vec4 u_cellsRect;
uniform vec3 u_low;
uniform vec3 u_high;
uniform vec3 u_water;
uniform bool u_base;
out vec4 colour;
vec2 within(vec4 rect) {
  return vec2((v_xy.x - rect.x) / rect.z, (rect.y + rect.w - v_xy.y) / rect.w);
}
void main() {
  if (v_kind < 0.5 && !u_base) {
    discard;
  }
  vec3 light = normalize(vec3(-0.6, 0.7, 1.1));
  float facing = max(dot(normalize(v_normal), light), 0.0);
  if (v_kind > 2.5) {
    // The walls of the base: a plain, darker earth.
    vec3 wall = mix(u_low, vec3(0.16, 0.15, 0.13), 0.65);
    colour = vec4(wall * (0.55 + 0.5 * abs(dot(normalize(v_normal), vec3(light.xy, 0.0)))), 1.0);
    return;
  }
  vec3 base;
  vec2 uv = within(u_pictureRect);
  bool onPicture = u_hasPicture && uv.x >= 0.0 && uv.y >= 0.0 && uv.x <= 1.0 && uv.y <= 1.0;
  if (onPicture) {
    base = texture(u_picture, uv).rgb;
    // The picture shows the ground under the water.
    if (v_kind < 1.5) {
      base = mix(base, u_water, 0.65);
    }
  } else if (v_kind < 0.5) {
    base = mix(u_low, vec3(0.2), 0.5);
  } else if (v_kind < 1.5) {
    base = u_water;
  } else {
    base = mix(u_low, u_high, v_share);
  }
  float lit = v_kind < 1.5 ? 1.0 : 0.5 + 0.62 * facing;
  vec3 shaded = base * lit;
  if (u_showCells) {
    vec4 cell = texture(u_cells, within(u_cellsRect));
    shaded = mix(shaded, cell.rgb, cell.a);
  }
  colour = vec4(shaded, 1.0);
}`;

const PIN_VERTEX = `#version 300 es
in vec3 a_position;
in float a_lift;
in vec3 a_colour;
in vec2 a_style;
in float a_size;
uniform mat4 u_matrix;
uniform float u_exaggeration;
uniform float u_pin;
out vec3 v_colour;
out vec2 v_style;
void main() {
  v_colour = a_colour;
  v_style = a_style;
  vec3 at = vec3(a_position.xy, a_position.z / ${CELL}.0 * u_exaggeration + a_lift * u_pin);
  gl_Position = u_matrix * vec4(at, 1.0);
  gl_PointSize = a_size;
}`;

const PIN_FRAGMENT = `#version 300 es
precision highp float;
in vec3 v_colour;
in vec2 v_style;
uniform bool u_points;
uniform vec3 u_edge;
out vec4 colour;
void main() {
  if (!u_points) {
    colour = vec4(v_colour * 0.8, 0.9);
    return;
  }
  vec2 p = gl_PointCoord * 2.0 - 1.0;
  float shape = v_style.x;
  float inside;
  float edge;
  if (shape < 0.5) {
    float d = max(abs(p.x), abs(p.y));
    inside = step(d, 0.8);
    edge = step(0.6, d);
  } else if (shape < 1.5) {
    // A triangle pointing up, apex at y = -0.9, base at y = 0.75; y grows
    // downwards in point space.
    float d = max(abs(p.x) - 0.6 * (p.y + 0.9), p.y - 0.75);
    inside = step(d, 0.0);
    edge = step(-0.18, d);
  } else {
    float d = length(p);
    inside = step(d, 0.85);
    edge = step(0.62, d);
  }
  if (inside < 0.5) {
    discard;
  }
  bool hollow = v_style.y > 0.5;
  if (hollow && edge < 0.5) {
    discard;
  }
  vec3 tint = hollow ? v_colour : mix(v_colour, u_edge, edge);
  colour = vec4(tint, 1.0);
}`;

/** The resources of one WebGL context. */
interface Scene {
  gl: WebGL2RenderingContext;
  terrain: WebGLProgram;
  pins: WebGLProgram;
  terrainVao: WebGLVertexArrayObject;
  normalBuffer: WebGLBuffer;
  indexCount: number;
  landIndices: number;
  picture: WebGLTexture | null;
  cells: WebGLTexture;
  pinVao: WebGLVertexArrayObject;
  pinBuffer: WebGLBuffer;
  pinCount: number;
  headVao: WebGLVertexArrayObject;
  headBuffer: WebGLBuffer;
  headCount: number;
}

/** One pin: what it stands for, where, and how it looks. */
interface Pin {
  picked: Picked;
  position: Vec3;
  colour: Vec3;
  shape: number;
  hollow: boolean;
  size: number;
  label: string;
  major: boolean;
}

/** The floats per pin vertex: position 3, lift 1, colour 3, style 2, size 1. */
const STRIDE = 10;

/**
 * A worldspace as a relief in 3D: the land from the report's heights, with
 * the game's terrain picture over it where there is one, the cells of mods
 * tinted, and places and collisions standing on it as pins (ADR 0031).
 */
export function MapView3D({
  world,
  terrain,
  picture,
  owner,
  showCells,
  markers,
  spots,
  picked,
  hover,
  onPick,
  onHover,
}: MapView3DProps) {
  const { t } = useSession();
  const frame = useRef<HTMLDivElement>(null);
  const canvas = useRef<HTMLCanvasElement>(null);
  const overlay = useRef<HTMLCanvasElement>(null);
  const scene = useRef<Scene | null>(null);
  const [size, setSize] = useState({ width: 800, height: 600 });
  // The true height of the land; raised only when the user asks.
  const [exaggeration, setExaggeration] = useState(1);
  // The land on a block with walls, so that it cannot be seen from below.
  const [withBase, setWithBase] = useState(true);
  const [failed, setFailed] = useState(false);
  const mesh = useMemo(() => buildMesh(terrain), [terrain]);

  const fitted = useCallback(
    (): Camera => ({
      target: [
        terrain.west + terrain.columns / 2,
        terrain.south + terrain.rows / 2,
        (((terrain.lowest + terrain.highest) / 2) * exaggeration) / CELL,
      ],
      distance: Math.max(terrain.columns, terrain.rows) * 1.05,
      yaw: 0,
      pitch: 0.95,
    }),
    [terrain, exaggeration],
  );
  const camera = useRef<Camera>(fitted());
  const animation = useRef<number | null>(null);
  const pending = useRef<number | null>(null);
  /** Where each pin's head was drawn last, for picking. */
  const heads = useRef<{ pin: Pin; x: number; y: number }[]>([]);
  const state = useRef({
    pins: [] as Pin[],
    picked,
    hover,
    exaggeration,
    showCells,
    withBase,
    size,
    picture,
  });
  state.current = {
    ...state.current,
    picked,
    hover,
    exaggeration,
    showCells,
    withBase,
    size,
    picture,
  };

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

  const draw = useCallback(() => {
    const current = scene.current;
    const target = overlay.current;
    if (current === null || target === null) {
      return;
    }
    const { gl } = current;
    const ratio = window.devicePixelRatio || 1;
    // The canvas fills the frame; its size is measured here, so that a
    // drawing never uses a size the window has left behind.
    const element = gl.canvas as HTMLCanvasElement;
    const width = Math.max(1, element.clientWidth || state.current.size.width);
    const height = Math.max(1, element.clientHeight || state.current.size.height);
    const wanted = [Math.round(width * ratio), Math.round(height * ratio)];
    if (element.width !== wanted[0] || element.height !== wanted[1]) {
      element.width = wanted[0] ?? element.width;
      element.height = wanted[1] ?? element.height;
    }
    gl.viewport(0, 0, gl.drawingBufferWidth, gl.drawingBufferHeight);
    gl.clearColor(0, 0, 0, 0);
    gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);

    const view = camera.current;
    const level = Math.cos(view.pitch) * view.distance;
    const eye: Vec3 = [
      view.target[0] - Math.sin(view.yaw) * level,
      view.target[1] - Math.cos(view.yaw) * level,
      view.target[2] + Math.sin(view.pitch) * view.distance,
    ];
    const matrix: Mat4 = multiply(
      perspective(FIELD_OF_VIEW, width / height, view.distance * 0.02, view.distance * 6 + 400),
      lookAt(eye, view.target, [0, 0, 1]),
    );
    const { exaggeration: raise, showCells: tinted, withBase } = state.current;
    const span = Math.max(1, terrain.highest - terrain.lowest);

    gl.enable(gl.DEPTH_TEST);
    // biome-ignore lint/correctness/useHookAtTopLevel: WebGL's useProgram is no React hook.
    gl.useProgram(current.terrain);
    const terrainUniform = (name: string) => gl.getUniformLocation(current.terrain, name);
    gl.uniformMatrix4fv(terrainUniform("u_matrix"), false, matrix);
    gl.uniform1f(terrainUniform("u_exaggeration"), raise);
    gl.uniform1f(terrainUniform("u_lowest"), terrain.lowest);
    gl.uniform1f(terrainUniform("u_span"), span);
    const style = getComputedStyle(target);
    const css = (name: string) => colour(style.getPropertyValue(name));
    gl.uniform3fv(terrainUniform("u_low"), css("--map-low"));
    gl.uniform3fv(terrainUniform("u_high"), css("--map-high"));
    gl.uniform3fv(terrainUniform("u_water"), css("--map-water"));
    gl.uniform1i(terrainUniform("u_hasPicture"), current.picture === null ? 0 : 1);
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, current.picture ?? current.cells);
    gl.uniform1i(terrainUniform("u_picture"), 0);
    const picture = state.current.picture;
    if (picture !== null) {
      gl.uniform4f(
        terrainUniform("u_pictureRect"),
        picture.meta.west,
        picture.meta.south,
        picture.meta.columns,
        picture.meta.rows,
      );
    }
    gl.activeTexture(gl.TEXTURE1);
    gl.bindTexture(gl.TEXTURE_2D, current.cells);
    gl.uniform1i(terrainUniform("u_cells"), 1);
    gl.uniform1i(terrainUniform("u_showCells"), tinted ? 1 : 0);
    gl.uniform4f(
      terrainUniform("u_cellsRect"),
      terrain.west,
      terrain.south,
      terrain.columns,
      terrain.rows,
    );
    gl.uniform1i(terrainUniform("u_base"), withBase ? 1 : 0);
    gl.bindVertexArray(current.terrainVao);
    gl.drawElements(
      gl.TRIANGLES,
      withBase ? current.indexCount : current.landIndices,
      gl.UNSIGNED_INT,
      0,
    );

    // biome-ignore lint/correctness/useHookAtTopLevel: WebGL's useProgram is no React hook.
    gl.useProgram(current.pins);
    const pinUniform = (name: string) => gl.getUniformLocation(current.pins, name);
    gl.uniformMatrix4fv(pinUniform("u_matrix"), false, matrix);
    gl.uniform1f(pinUniform("u_exaggeration"), raise);
    gl.uniform1f(pinUniform("u_pin"), PIN * Math.max(1, view.distance / 40));
    gl.uniform3fv(pinUniform("u_edge"), css("--place-edge"));
    gl.uniform1i(pinUniform("u_points"), 0);
    gl.bindVertexArray(current.pinVao);
    gl.drawArrays(gl.LINES, 0, current.pinCount);
    gl.uniform1i(pinUniform("u_points"), 1);
    gl.bindVertexArray(current.headVao);
    gl.drawArrays(gl.POINTS, 0, current.headCount);
    gl.bindVertexArray(null);

    // Names and highlights are drawn flat on top.
    target.width = width * ratio;
    target.height = height * ratio;
    const context = target.getContext("2d");
    if (context === null) {
      return;
    }
    context.setTransform(ratio, 0, 0, ratio, 0, 0);
    context.clearRect(0, 0, width, height);
    const lift = PIN * Math.max(1, view.distance / 40);
    const placed: { pin: Pin; x: number; y: number }[] = [];
    const labels: Label[] = [];
    const close = view.distance < 45;
    for (const pin of state.current.pins) {
      const top = project(
        matrix,
        [pin.position[0], pin.position[1], (pin.position[2] / CELL) * raise + lift],
        width,
        height,
      );
      if (top === null || top.depth > 1 || top.x < -40 || top.y < -40) {
        continue;
      }
      if (top.x > width + 40 || top.y > height + 40) {
        continue;
      }
      placed.push({ pin, x: top.x, y: top.y });
      const shown =
        samePick(pin.picked, state.current.picked) || samePick(pin.picked, state.current.hover);
      if (pin.label !== "" && (shown || pin.major || close)) {
        labels.push({ x: top.x, y: top.y - pin.size / 2 - 4, text: pin.label, strong: shown });
      }
    }
    heads.current = placed;
    drawLabels(context, labels);
    for (const [shown, lineWidth] of [
      [state.current.hover, 1.5],
      [state.current.picked, 3],
    ] as const) {
      const found = placed.find((head) => samePick(head.pin.picked, shown));
      if (found !== undefined) {
        context.strokeStyle = "#ffffff";
        context.lineWidth = lineWidth;
        context.beginPath();
        context.arc(found.x, found.y, found.pin.size / 2 + 5, 0, Math.PI * 2);
        context.stroke();
      }
    }
  }, [terrain]);

  const redraw = useCallback(() => {
    if (pending.current !== null) {
      return;
    }
    pending.current = requestAnimationFrame(() => {
      pending.current = null;
      draw();
    });
  }, [draw]);

  // The context and the land are made once per worldspace.
  useEffect(() => {
    const element = canvas.current;
    if (element === null || mesh === null) {
      return;
    }
    const gl = element.getContext("webgl2", { antialias: true });
    if (gl === null) {
      setFailed(true);
      return;
    }
    let made: Scene;
    // Everything made here, to be deleted again when the view closes.
    const buffers: WebGLBuffer[] = [];
    const arrays: WebGLVertexArrayObject[] = [];
    try {
      const terrainProgram = program(gl, TERRAIN_VERTEX, TERRAIN_FRAGMENT);
      const pinProgram = program(gl, PIN_VERTEX, PIN_FRAGMENT);
      const buffer = (
        data: Float32Array,
        attribute: string,
        count: number,
        shader: WebGLProgram,
      ) => {
        const made = gl.createBuffer();
        buffers.push(made);
        gl.bindBuffer(gl.ARRAY_BUFFER, made);
        gl.bufferData(gl.ARRAY_BUFFER, data, gl.STATIC_DRAW);
        const location = gl.getAttribLocation(shader, attribute);
        if (location >= 0) {
          gl.enableVertexAttribArray(location);
          gl.vertexAttribPointer(location, count, gl.FLOAT, false, 0, 0);
        }
        return made;
      };
      const terrainVao = gl.createVertexArray();
      arrays.push(terrainVao);
      gl.bindVertexArray(terrainVao);
      buffer(mesh.xy, "a_xy", 2, terrainProgram);
      buffer(mesh.heights, "a_height", 1, terrainProgram);
      buffer(mesh.kinds, "a_kind", 1, terrainProgram);
      const normalBuffer = buffer(
        normals(mesh, terrain.samples, state.current.exaggeration),
        "a_normal",
        3,
        terrainProgram,
      );
      const indexBuffer = gl.createBuffer();
      buffers.push(indexBuffer);
      gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, indexBuffer);
      gl.bufferData(gl.ELEMENT_ARRAY_BUFFER, mesh.indices, gl.STATIC_DRAW);
      gl.bindVertexArray(null);

      const pinLayout = (vao: WebGLVertexArrayObject, data: WebGLBuffer) => {
        gl.bindVertexArray(vao);
        gl.bindBuffer(gl.ARRAY_BUFFER, data);
        const bytes = Float32Array.BYTES_PER_ELEMENT;
        for (const [name, count, offset] of [
          ["a_position", 3, 0],
          ["a_lift", 1, 3],
          ["a_colour", 3, 4],
          ["a_style", 2, 7],
          ["a_size", 1, 9],
        ] as const) {
          const location = gl.getAttribLocation(pinProgram, name);
          if (location >= 0) {
            gl.enableVertexAttribArray(location);
            gl.vertexAttribPointer(
              location,
              count,
              gl.FLOAT,
              false,
              STRIDE * bytes,
              offset * bytes,
            );
          }
        }
        gl.bindVertexArray(null);
      };
      const pinVao = gl.createVertexArray();
      const pinBuffer = gl.createBuffer();
      pinLayout(pinVao, pinBuffer);
      arrays.push(pinVao);
      buffers.push(pinBuffer);
      const headVao = gl.createVertexArray();
      const headBuffer = gl.createBuffer();
      pinLayout(headVao, headBuffer);
      arrays.push(headVao);
      buffers.push(headBuffer);

      const cells = gl.createTexture();
      gl.bindTexture(gl.TEXTURE_2D, cells);
      gl.texImage2D(
        gl.TEXTURE_2D,
        0,
        gl.RGBA,
        1,
        1,
        0,
        gl.RGBA,
        gl.UNSIGNED_BYTE,
        new Uint8Array([0, 0, 0, 0]),
      );
      made = {
        gl,
        terrain: terrainProgram,
        pins: pinProgram,
        terrainVao,
        normalBuffer,
        indexCount: mesh.indices.length,
        landIndices: mesh.landIndices,
        picture: null,
        cells,
        pinVao,
        pinBuffer,
        pinCount: 0,
        headVao,
        headBuffer,
        headCount: 0,
      };
    } catch {
      setFailed(true);
      return;
    }
    scene.current = made;
    redraw();
    return () => {
      // Frees the graphics memory at once rather than when collected.
      for (const made of buffers) {
        gl.deleteBuffer(made);
      }
      for (const made of arrays) {
        gl.deleteVertexArray(made);
      }
      gl.deleteTexture(made.cells);
      gl.deleteTexture(made.picture);
      gl.deleteProgram(made.terrain);
      gl.deleteProgram(made.pins);
      scene.current = null;
    };
  }, [mesh, terrain, redraw]);

  // The game's picture of the land, laid over it.
  useEffect(() => {
    const current = scene.current;
    if (current === null) {
      return;
    }
    const { gl } = current;
    if (current.picture !== null) {
      gl.deleteTexture(current.picture);
      current.picture = null;
    }
    if (picture !== null) {
      const texture = gl.createTexture();
      gl.bindTexture(gl.TEXTURE_2D, texture);
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, picture.image);
      gl.generateMipmap(gl.TEXTURE_2D);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR_MIPMAP_LINEAR);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
      current.picture = texture;
    }
    redraw();
  }, [picture, redraw]);

  // The cells of mods, one pixel per cell, tinted by how many meet.
  useEffect(() => {
    const current = scene.current;
    const element = overlay.current;
    if (current === null || element === null) {
      return;
    }
    const { gl } = current;
    const style = getComputedStyle(element);
    const heat = [1, 2, 3, 4].map((level) =>
      colour(style.getPropertyValue(`--heat-${level}`)).map((value) => value * 255),
    );
    const pixels = new Uint8Array(terrain.columns * terrain.rows * 4);
    const north = terrain.south + terrain.rows - 1;
    for (const cell of world.cells) {
      if (owner >= 0 && !cell.owners.includes(owner)) {
        continue;
      }
      const column = cell.x - terrain.west;
      const row = north - cell.y;
      if (column < 0 || row < 0 || column >= terrain.columns || row >= terrain.rows) {
        continue;
      }
      const tint = heat[Math.min(cell.owners.length, 4) - 1] ?? [255, 255, 255];
      pixels.set(
        [tint[0] ?? 0, tint[1] ?? 0, tint[2] ?? 0, 95],
        (row * terrain.columns + column) * 4,
      );
    }
    gl.bindTexture(gl.TEXTURE_2D, current.cells);
    gl.texImage2D(
      gl.TEXTURE_2D,
      0,
      gl.RGBA,
      terrain.columns,
      terrain.rows,
      0,
      gl.RGBA,
      gl.UNSIGNED_BYTE,
      pixels,
    );
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    redraw();
  }, [world.cells, owner, terrain, redraw]);

  // The normals follow how far the land is raised.
  useEffect(() => {
    const current = scene.current;
    if (current === null || mesh === null) {
      return;
    }
    const { gl } = current;
    gl.bindBuffer(gl.ARRAY_BUFFER, current.normalBuffer);
    gl.bufferData(gl.ARRAY_BUFFER, normals(mesh, terrain.samples, exaggeration), gl.STATIC_DRAW);
    redraw();
  }, [exaggeration, mesh, terrain.samples, redraw]);

  // The pins of places and collisions.
  useEffect(() => {
    const current = scene.current;
    const element = overlay.current;
    if (current === null || element === null) {
      return;
    }
    const style = getComputedStyle(element);
    const css = (name: string) => colour(style.getPropertyValue(name));
    const ground = groundHeight(mesh, terrain);
    const pins: Pin[] = [];
    for (const marker of markers) {
      const [x, y, z] = marker.position;
      pins.push({
        picked: { type: "place", marker },
        position: [x / CELL, y / CELL, z],
        colour: css(`--place-${marker.origin}`),
        shape: { square: 0, triangle: 1, circle: 2 }[shapeOf(marker.kind)],
        hollow: marker.origin === "removed",
        size: isMajor(marker.kind) ? 15 : 11,
        label: marker.name,
        major: isMajor(marker.kind),
      });
    }
    for (const spot of spots) {
      const at = spotPosition(spot);
      if (at === null) {
        continue;
      }
      const z = spot.position ? at[2] : ground(at[0] / CELL, at[1] / CELL);
      pins.push({
        picked: { type: "spot", spot },
        position: [at[0] / CELL, at[1] / CELL, z],
        colour: css("--accent"),
        shape: 2,
        hollow: false,
        size: 13,
        label: "",
        major: false,
      });
    }
    state.current.pins = pins;
    const lines = new Float32Array(pins.length * 2 * STRIDE);
    const points = new Float32Array(pins.length * STRIDE);
    pins.forEach((pin, index) => {
      const vertex = (lift: number) => [
        ...pin.position,
        lift,
        ...pin.colour,
        pin.shape,
        pin.hollow ? 1 : 0,
        pin.size,
      ];
      lines.set([...vertex(0), ...vertex(1)], index * 2 * STRIDE);
      points.set(vertex(1), index * STRIDE);
    });
    const { gl } = current;
    gl.bindBuffer(gl.ARRAY_BUFFER, current.pinBuffer);
    gl.bufferData(gl.ARRAY_BUFFER, lines, gl.DYNAMIC_DRAW);
    gl.bindBuffer(gl.ARRAY_BUFFER, current.headBuffer);
    gl.bufferData(gl.ARRAY_BUFFER, points, gl.DYNAMIC_DRAW);
    current.pinCount = pins.length * 2;
    current.headCount = pins.length;
    redraw();
  }, [markers, spots, mesh, terrain, redraw]);

  // The drawing follows size, highlights and the controls, which it reads
  // from a ref so that drawing need not be made anew for each.
  // biome-ignore lint/correctness/useExhaustiveDependencies: these trigger a redraw.
  useEffect(() => {
    redraw();
  }, [size, picked, hover, showCells, withBase, exaggeration, redraw]);

  // Whatever changes the canvas's size, it is drawn anew at its new size.
  useEffect(() => {
    const element = canvas.current;
    if (element === null) {
      return;
    }
    const observer = new ResizeObserver(() => redraw());
    observer.observe(element);
    return () => observer.disconnect();
  }, [redraw]);

  /** Moves the camera smoothly to `goal`. */
  const fly = useCallback(
    (goal: Camera) => {
      if (animation.current !== null) {
        cancelAnimationFrame(animation.current);
      }
      const start = { ...camera.current, target: [...camera.current.target] as Vec3 };
      const began = performance.now();
      const step = (now: number) => {
        const share = Math.min(1, (now - began) / 450);
        const ease = share * (2 - share);
        const mix = (a: number, b: number) => a + (b - a) * ease;
        camera.current = {
          target: [
            mix(start.target[0], goal.target[0]),
            mix(start.target[1], goal.target[1]),
            mix(start.target[2], goal.target[2]),
          ],
          distance: mix(start.distance, goal.distance),
          yaw: mix(start.yaw, goal.yaw),
          pitch: mix(start.pitch, goal.pitch),
        };
        draw();
        animation.current = share < 1 ? requestAnimationFrame(step) : null;
      };
      animation.current = requestAnimationFrame(step);
    },
    [draw],
  );
  useEffect(
    () => () => {
      if (animation.current !== null) {
        cancelAnimationFrame(animation.current);
        animation.current = null;
      }
      if (pending.current !== null) {
        cancelAnimationFrame(pending.current);
        pending.current = null;
      }
    },
    [],
  );

  // A place or collision picked elsewhere is flown to.
  useEffect(() => {
    if (picked === null || picked.type === "cell") {
      return;
    }
    const position = picked.type === "place" ? picked.marker.position : spotPosition(picked.spot);
    if (position === null) {
      return;
    }
    const raise = state.current.exaggeration;
    fly({
      ...camera.current,
      target: [position[0] / CELL, position[1] / CELL, (position[2] / CELL) * raise],
      distance: Math.min(camera.current.distance, 28),
    });
  }, [picked, fly]);

  const pickAt = (clientX: number, clientY: number): Picked | null => {
    const rect = overlay.current?.getBoundingClientRect();
    if (rect === undefined) {
      return null;
    }
    const x = clientX - rect.left;
    const y = clientY - rect.top;
    let best: { picked: Picked; distance: number } | null = null;
    for (const head of heads.current) {
      const distance = Math.hypot(head.x - x, head.y - y);
      if (distance <= HIT && (best === null || distance < best.distance)) {
        best = { picked: head.pin.picked, distance };
      }
    }
    return best?.picked ?? null;
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
      const found = pickAt(event.clientX, event.clientY);
      if (!samePick(found, hover)) {
        onHover(found);
      }
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
      // One pixel moves the land by as much as it covers at the target.
      const perPixel = (2 * Math.tan(FIELD_OF_VIEW / 2) * from.distance) / size.height;
      const forward = [Math.sin(from.yaw), Math.cos(from.yaw)];
      const right = [Math.cos(from.yaw), -Math.sin(from.yaw)];
      const along = dy / Math.max(0.35, Math.sin(from.pitch));
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
  const onPointerUp = (event: PointerEvent<HTMLCanvasElement>) => {
    const start = drag.current;
    drag.current = null;
    if (start !== null && !start.moved) {
      const found = pickAt(event.clientX, event.clientY);
      if (found !== null || !start.turn) {
        onPick(found);
      }
    }
  };

  // The wheel zooms; attached directly so that the page does not scroll.
  useEffect(() => {
    const element = overlay.current;
    if (element === null) {
      return;
    }
    const onWheel = (event: WheelEvent) => {
      event.preventDefault();
      const limit = Math.max(terrain.columns, terrain.rows) * 2.5;
      camera.current = {
        ...camera.current,
        distance: Math.min(
          limit,
          Math.max(1.5, camera.current.distance * Math.exp(event.deltaY * 0.0012)),
        ),
      };
      redraw();
    };
    element.addEventListener("wheel", onWheel, { passive: false });
    return () => element.removeEventListener("wheel", onWheel);
  }, [terrain, redraw]);

  if (mesh === null || failed) {
    return (
      <div className="map-frame map-missing">
        <p className="empty">{t.mapNoRelief}</p>
      </div>
    );
  }
  const zoom = (factor: number) => {
    fly({ ...camera.current, distance: Math.max(1.5, camera.current.distance * factor) });
  };
  return (
    <div className="map-frame relief-frame" ref={frame}>
      <canvas
        className="relief-canvas"
        ref={canvas}
        style={{ width: "100%", height: size.height }}
      />
      <canvas
        className="map-canvas relief-overlay"
        ref={overlay}
        style={{ width: "100%", height: "100%" }}
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerUp}
        onPointerLeave={() => onHover(null)}
        onContextMenu={(event) => event.preventDefault()}
      />
      <div className="map-zoom">
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
          onClick={() => fly({ ...camera.current, yaw: 0, pitch: MAX_PITCH })}
        >
          {t.mapTopView}
        </button>
        <button type="button" className="map-zoom-button" onClick={() => fly(fitted())}>
          {t.mapFit}
        </button>
      </div>
      <div className="relief-controls">
        <label className="relief-control">
          {t.mapHeight}
          <input
            type="range"
            min={0.5}
            max={4}
            step={0.25}
            value={exaggeration}
            onChange={(event) => {
              const next = Number(event.target.value);
              const view = camera.current;
              // Keep looking at the same spot of the land.
              camera.current = {
                ...view,
                target: [view.target[0], view.target[1], (view.target[2] / exaggeration) * next],
              };
              setExaggeration(next);
            }}
          />
          <span className="relief-value">{t.mapHeightValue(exaggeration)}</span>
        </label>
        <label className="relief-control">
          <input
            type="checkbox"
            checked={withBase}
            onChange={(event) => setWithBase(event.target.checked)}
          />
          {t.mapBase}
        </label>
        <span className="relief-hint">{t.mapReliefHint}</span>
      </div>
    </div>
  );
}

/** The height of the land at a point (in cells), in game units. */
function groundHeight(mesh: Mesh | null, terrain: MapTerrain) {
  return (x: number, y: number) => {
    if (mesh === null) {
      return terrain.lowest;
    }
    const column = Math.round((x - terrain.west) * terrain.samples - 0.5);
    const row = Math.round((terrain.south + terrain.rows - y) * terrain.samples - 0.5);
    const clampedColumn = Math.min(mesh.columns - 1, Math.max(0, column));
    const clampedRow = Math.min(mesh.rows - 1, Math.max(0, row));
    return mesh.heights[clampedRow * mesh.columns + clampedColumn] ?? terrain.lowest;
  };
}
