// A place up close (ADR 0038): every object placed around a point, by the
// game and by mods, drawn with its own mesh where it stands. The parts of
// a collision are tinted, a picked object is lit up.
//
// Each model is drawn once for all its objects (instancing): their
// matrices and looks are kept in one buffer per model.

import type { Api } from "./api";
import type { AreaObject, AreaObjects } from "./generated/area-objects";
import type { LodMesh } from "./generated/lod-mesh";
import type { Mat4, Vec3 } from "./gl";
import { CELL } from "./map-places";

/** How many meshes are fetched at once. */
const PARALLEL = 6;
/** The largest side of an object's texture. */
const TEXTURE_SIDE = 512;

const VERTEX = `#version 300 es
in vec3 a_position;
in vec3 a_normal;
in vec2 a_uv;
in mat4 a_model;
in float a_look;
uniform mat4 u_matrix;
uniform float u_exaggeration;
out vec2 v_uv;
out vec3 v_normal;
out float v_look;
void main() {
  vec4 world = a_model * vec4(a_position, 1.0);
  v_uv = a_uv;
  v_normal = mat3(a_model) * a_normal;
  v_look = a_look;
  gl_Position = u_matrix * vec4(world.xy / ${CELL}.0, world.z / ${CELL}.0 * u_exaggeration, 1.0);
}`;

const FRAGMENT = `#version 300 es
precision highp float;
in vec2 v_uv;
in vec3 v_normal;
in float v_look;
uniform sampler2D u_texture;
uniform bool u_hasTexture;
uniform bool u_alphaTest;
uniform vec3 u_mark;
uniform vec3 u_focus;
uniform vec3 u_ghost;
out vec4 colour;
void main() {
  vec4 sampled = u_hasTexture ? texture(u_texture, v_uv) : vec4(0.7, 0.7, 0.7, 1.0);
  if (u_alphaTest && sampled.a < 0.5) {
    discard;
  }
  vec3 light = normalize(vec3(-0.6, 0.7, 1.1));
  float lit = 0.55 + 0.6 * abs(dot(normalize(v_normal), light));
  vec3 shaded = sampled.rgb * lit;
  if (v_look > 5.5) {
    // In the chain of the picked object: lit up, half as much.
    shaded = mix(shaded, vec3(1.0), 0.22);
  } else if (v_look > 4.5) {
    // Hidden, and shown as the game would show it: not at all.
    discard;
  }
  if (v_look > 3.5) {
    // Hidden by the patch: half there, every other pixel left out.
    if (mod(floor(gl_FragCoord.x) + floor(gl_FragCoord.y), 2.0) < 1.0) {
      discard;
    }
    shaded = mix(shaded, u_ghost, 0.6);
  } else if (v_look > 2.5) {
    // Part of the collision looked at.
    shaded = mix(shaded, u_focus, 0.6);
  } else if (v_look > 1.5) {
    // Picked: lit up.
    shaded = mix(shaded, vec3(1.0), 0.45);
  } else if (v_look > 0.5) {
    // Part of a collision.
    shaded = mix(shaded, u_mark, 0.55);
  }
  colour = vec4(shaded, 1.0);
}`;

/** One colour per object, to tell which one a point of the screen shows. */
const PICK_FRAGMENT = `#version 300 es
precision highp float;
in vec2 v_uv;
flat in float v_id;
flat in float v_gone;
uniform sampler2D u_texture;
uniform bool u_hasTexture;
uniform bool u_alphaTest;
out vec4 colour;
void main() {
  if (v_gone > 0.5) {
    discard;
  }
  if (u_alphaTest && u_hasTexture && texture(u_texture, v_uv).a < 0.5) {
    discard;
  }
  float id = v_id;
  colour = vec4(mod(id, 256.0), mod(floor(id / 256.0), 256.0), floor(id / 65536.0), 255.0) / 255.0;
}`;

const PICK_VERTEX = `#version 300 es
in vec3 a_position;
in vec2 a_uv;
in mat4 a_model;
in float a_look;
uniform mat4 u_matrix;
uniform float u_exaggeration;
uniform float u_base;
out vec2 v_uv;
flat out float v_id;
flat out float v_gone;
void main() {
  vec4 world = a_model * vec4(a_position, 1.0);
  v_uv = a_uv;
  v_id = u_base + float(gl_InstanceID) + 1.0;
  v_gone = a_look > 4.5 && a_look < 5.5 ? 1.0 : 0.0;
  gl_Position = u_matrix * vec4(world.xy / ${CELL}.0, world.z / ${CELL}.0 * u_exaggeration, 1.0);
}`;

/**
 * The reach of a part in a chain (ADR 0055): its box, turned and placed,
 * grown by the radius on every side, with rounded edges. Drawn from a
 * sphere whose points are pushed out to the box's corners.
 */
const HULL_VERTEX = `#version 300 es
in vec3 a_normal;
in vec3 a_centre;
in vec3 a_axis0;
in vec3 a_axis1;
in vec3 a_axis2;
in float a_strength;
uniform mat4 u_matrix;
uniform float u_exaggeration;
uniform float u_radius;
out vec3 v_normal;
out float v_strength;
void main() {
  vec3 side = sign(a_normal);
  float l0 = max(length(a_axis0), 1e-6);
  float l1 = max(length(a_axis1), 1e-6);
  float l2 = max(length(a_axis2), 1e-6);
  vec3 normal = a_axis0 / l0 * a_normal.x + a_axis1 / l1 * a_normal.y + a_axis2 / l2 * a_normal.z;
  vec3 world = a_centre + a_axis0 * side.x + a_axis1 * side.y + a_axis2 * side.z + normal * u_radius;
  v_normal = normal;
  v_strength = a_strength;
  gl_Position = u_matrix * vec4(world.xy / ${CELL}.0, world.z / ${CELL}.0 * u_exaggeration, 1.0);
}`;

const HULL_FRAGMENT = `#version 300 es
precision highp float;
in vec3 v_normal;
in float v_strength;
uniform vec3 u_colour;
out vec4 colour;
void main() {
  vec3 light = normalize(vec3(-0.6, 0.7, 1.1));
  float lit = 0.75 + 0.35 * abs(dot(normalize(v_normal), light));
  colour = vec4(u_colour * lit, 0.16 * v_strength);
}`;

/** Where the attributes of the hulls are. */
const HULL_LOCATIONS: [string, number][] = [
  ["a_normal", 0],
  ["a_centre", 1],
  ["a_axis0", 2],
  ["a_axis1", 3],
  ["a_axis2", 4],
  ["a_strength", 5],
];

/** How many floats a hull takes in its buffer: centre, three half axes, strength. */
const HULL_STRIDE = 13;

/** The colour of the hulls of a chain. */
const HULL_COLOUR: Vec3 = [0.75, 0.92, 1.0];

/**
 * The points and triangles of a sphere of radius one, none of whose points
 * lies on a plane of the axes: pushed out to a box's corners, the triangles
 * across those planes span the box's flat sides.
 */
function hullSphere(): { points: Float32Array; triangles: Uint16Array } {
  const rings = 12;
  const around = 24;
  const points: number[] = [0, 0, 1];
  for (let ring = 0; ring < rings; ring += 1) {
    const polar = ((ring + 0.5) / rings) * Math.PI;
    for (let step = 0; step < around; step += 1) {
      const azimuth = ((step + 0.5) / around) * Math.PI * 2;
      points.push(
        Math.sin(polar) * Math.cos(azimuth),
        Math.sin(polar) * Math.sin(azimuth),
        Math.cos(polar),
      );
    }
  }
  points.push(0, 0, -1);
  const last = 1 + rings * around;
  const at = (ring: number, step: number) => 1 + ring * around + (step % around);
  const triangles: number[] = [];
  for (let step = 0; step < around; step += 1) {
    triangles.push(0, at(0, step), at(0, step + 1));
    triangles.push(last, at(rings - 1, step + 1), at(rings - 1, step));
  }
  for (let ring = 0; ring + 1 < rings; ring += 1) {
    for (let step = 0; step < around; step += 1) {
      triangles.push(at(ring, step), at(ring + 1, step), at(ring + 1, step + 1));
      triangles.push(at(ring, step), at(ring + 1, step + 1), at(ring, step + 1));
    }
  }
  return { points: new Float32Array(points), triangles: new Uint16Array(triangles) };
}

/** Where every attribute is, the same in both programs. */
const LOCATIONS: [string, number][] = [
  ["a_position", 0],
  ["a_normal", 1],
  ["a_uv", 2],
  ["a_model", 3],
  ["a_look", 7],
];

/** A program with the attributes at `locations`. */
function linked(
  gl: WebGL2RenderingContext,
  vertex: string,
  fragment: string,
  locations: [string, number][] = LOCATIONS,
): WebGLProgram {
  const shader = (type: number, source: string) => {
    const made = gl.createShader(type);
    if (made === null) {
      throw new Error("no shader");
    }
    gl.shaderSource(made, source);
    gl.compileShader(made);
    if (!gl.getShaderParameter(made, gl.COMPILE_STATUS)) {
      throw new Error(gl.getShaderInfoLog(made) ?? "shader");
    }
    return made;
  };
  const made = gl.createProgram();
  const parts = [shader(gl.VERTEX_SHADER, vertex), shader(gl.FRAGMENT_SHADER, fragment)];
  for (const part of parts) {
    gl.attachShader(made, part);
  }
  for (const [name, location] of locations) {
    gl.bindAttribLocation(made, location, name);
  }
  gl.linkProgram(made);
  for (const part of parts) {
    gl.deleteShader(part);
  }
  if (!gl.getProgramParameter(made, gl.LINK_STATUS)) {
    throw new Error(gl.getProgramInfoLog(made) ?? "program");
  }
  return made;
}

interface Surface {
  vao: WebGLVertexArrayObject;
  buffers: WebGLBuffer[];
  count: number;
  texture: string | null;
  alphaTest: boolean;
}

type Shared = WebGLTexture | "loading" | "missing";

/** An object as drawn: its model matrix and whether it is part of a collision. */
interface Instance {
  object: AreaObject;
  matrix: Float32Array;
  marked: boolean;
  look: Look;
}

/** The key by which collisions name an object. */
export const objectKey = (plugin: string, localId: number) => `${plugin.toLowerCase()}:${localId}`;

function bytesOf(base64: string): ArrayBuffer {
  const text = atob(base64);
  const bytes = new Uint8Array(text.length);
  for (let index = 0; index < text.length; index++) {
    bytes[index] = text.charCodeAt(index);
  }
  return bytes.buffer;
}

function normalsOf(positions: Float32Array, indices: Uint32Array): Float32Array {
  const normals = new Float32Array(positions.length);
  const p = (point: number, axis: number) => positions[point * 3 + axis] ?? 0;
  for (let index = 0; index + 2 < indices.length; index += 3) {
    const a = indices[index] ?? 0;
    const b = indices[index + 1] ?? 0;
    const c = indices[index + 2] ?? 0;
    const u = [p(b, 0) - p(a, 0), p(b, 1) - p(a, 1), p(b, 2) - p(a, 2)];
    const v = [p(c, 0) - p(a, 0), p(c, 1) - p(a, 1), p(c, 2) - p(a, 2)];
    const n = [
      (u[1] ?? 0) * (v[2] ?? 0) - (u[2] ?? 0) * (v[1] ?? 0),
      (u[2] ?? 0) * (v[0] ?? 0) - (u[0] ?? 0) * (v[2] ?? 0),
      (u[0] ?? 0) * (v[1] ?? 0) - (u[1] ?? 0) * (v[0] ?? 0),
    ];
    for (const point of [a, b, c]) {
      for (let axis = 0; axis < 3; axis++) {
        normals[point * 3 + axis] = (normals[point * 3 + axis] ?? 0) + (n[axis] ?? 0);
      }
    }
  }
  for (let point = 0; point < normals.length; point += 3) {
    const length =
      Math.hypot(normals[point] ?? 0, normals[point + 1] ?? 0, normals[point + 2] ?? 1) || 1;
    for (let axis = 0; axis < 3; axis++) {
      normals[point + axis] = (normals[point + axis] ?? 0) / length;
    }
  }
  return normals;
}

/**
 * Where an object stands, as the game places it: scaled, turned about the
 * world's z, then y, then x (the game turns clockwise), and moved; column
 * by column. The order is the one that draws tilted rocks and beams of real
 * setups as the game does (ADR 0040).
 */
export function placement(object: AreaObject): Float32Array {
  const [rx, ry, rz] = object.rotation.map((angle) => -angle) as [number, number, number];
  const [cx, sx] = [Math.cos(rx), Math.sin(rx)];
  const [cy, sy] = [Math.cos(ry), Math.sin(ry)];
  const [cz, sz] = [Math.cos(rz), Math.sin(rz)];
  // Rx · Ry · Rz, row by row.
  const r = [
    [cy * cz, -cy * sz, sy],
    [sx * sy * cz + cx * sz, cx * cz - sx * sy * sz, -sx * cy],
    [sx * sz - cx * sy * cz, cx * sy * sz + sx * cz, cx * cy],
  ];
  const s = object.scale;
  const at = (row: number, column: number) => (r[row]?.[column] ?? 0) * s;
  const [x, y, z] = object.position;
  return new Float32Array([
    at(0, 0),
    at(1, 0),
    at(2, 0),
    0,
    at(0, 1),
    at(1, 1),
    at(2, 1),
    0,
    at(0, 2),
    at(1, 2),
    at(2, 2),
    0,
    x,
    y,
    z,
    1,
  ]);
}

/** How close two parts' boxes must come to hang together, in game units (about 20 cm). */
export const TOUCHING = 14;

/**
 * Folders of models of nature and the like: trees, plants, grass, rocks and
 * the rest of the landscape, effects and lights. They stand next to what is
 * built and touch it, but never belong to it.
 */
const NATURE =
  /(^|[\\/])(landscape|trees|plants|flora|grass|rocks?|effects|fx|lights?|sky|water|actors|creatures)[\\/]/i;

/** The plugins of the games themselves. */
const GAME_PLUGIN =
  /^(skyrim|update|dawnguard|hearthfires|dragonborn|fallout4|dlc\w*|oblivion|fallout3|falloutnv)\.esm$|^cc\w*\.es[lm]$/i;

/** Whether a plugin is one of the game's own. */
export function isGamePlugin(plugin: string): boolean {
  return GAME_PLUGIN.test(plugin);
}

/**
 * The mod an object belongs to, by plugin: the one that placed it, or,
 * for an object of the game, the one whose version of it wins (a mod that
 * moves a vanilla book onto its shelf makes the book its own, ADR 0032).
 */
export function ownerOf(object: AreaObject): string {
  return (isGamePlugin(object.id.plugin) ? object.plugin : object.id.plugin).toLowerCase();
}

/** Folders of trees, which stay out of structures even as leaves. */
const TREES = /(^|[\\/])trees[\\/]/i;

/** Folders of small plants: decor a mod puts on what it builds. */
const PLANTS = /(^|[\\/])(plants|flora|grass)[\\/]/i;

/** The folder of clutter: crates, barrels, sacks and whatever else lies about. */
const CLUTTER = /(^|[\\/])clutter[\\/]/i;

/** Folders of trees, plants and grass. */
const FLORA = /(^|[\\/])(trees|plants|flora|grass)[\\/]/i;

/** Whether a model is a tree, a plant or grass. */
export function isFlora(model: string): boolean {
  return FLORA.test(model);
}

/** Whether a model is nature rather than something placed by hand: trees, rocks, landscape. */
export function isNature(model: string): boolean {
  return NATURE.test(model);
}

/** Below this longest side, in game units (about 35 cm), a part is clutter, not building. */
const SMALL = 24;

/**
 * Whether a part is decor that a built structure takes along where it
 * touches it: clutter, or small plants; never trees, rocks or landscape.
 */
function isDecor(model: string, kind: "nature" | "clutter" | "built"): boolean {
  if (TREES.test(model)) {
    return false;
  }
  return kind === "clutter" || PLANTS.test(model);
}

/**
 * What a part is, for growing a structure: nature, clutter lying about
 * (by its folder or its size), or something built.
 */
function kindOf(model: string, size: number): "nature" | "clutter" | "built" {
  if (NATURE.test(model)) {
    return "nature";
  }
  return CLUTTER.test(model) || size < SMALL ? "clutter" : "built";
}

/** The longest side of a model's box at `scale`, in game units. */
function sizeOf(box: [[number, number, number], [number, number, number]], scale: number): number {
  const [low, high] = box;
  return Math.max(high[0] - low[0], high[1] - low[1], high[2] - low[2]) * scale;
}

/** What the card of a picked object can do with the view it was picked in. */
export interface PartTools {
  /** Its chain: the parts of the same mod linked to it, each within `radius` game units of the next. */
  chain: (radius: number) => AreaObject[];
  /**
   * Lights `chain` up faintly next to the picked object and, with `hulls`,
   * draws how far each part reaches; an empty chain clears it.
   */
  showChain: (chain: AreaObject[], radius: number, hulls: boolean) => void;
}

/** A box turned and placed in the world: its middle, its three axes (unit length) and half sizes. */
export interface PlacedBox {
  centre: [number, number, number];
  axes: [[number, number, number], [number, number, number], [number, number, number]];
  half: [number, number, number];
}

/** A model's box as `matrix` (column by column, with scale) places it. */
export function placedBox(
  box: [[number, number, number], [number, number, number]],
  matrix: Float32Array,
): PlacedBox {
  const [low, high] = box;
  const middle = [0, 1, 2].map((axis) => ((low[axis] ?? 0) + (high[axis] ?? 0)) / 2);
  const at = (row: number) =>
    (matrix[row] ?? 0) * (middle[0] ?? 0) +
    (matrix[4 + row] ?? 0) * (middle[1] ?? 0) +
    (matrix[8 + row] ?? 0) * (middle[2] ?? 0) +
    (matrix[12 + row] ?? 0);
  const column = (index: number): [number, number, number] => [
    matrix[index * 4] ?? 0,
    matrix[index * 4 + 1] ?? 0,
    matrix[index * 4 + 2] ?? 0,
  ];
  const axes = [column(0), column(1), column(2)];
  const lengths = axes.map((axis) => Math.hypot(...axis) || 1);
  return {
    centre: [at(0), at(1), at(2)],
    axes: axes.map((axis, index) => axis.map((value) => value / (lengths[index] ?? 1))) as [
      [number, number, number],
      [number, number, number],
      [number, number, number],
    ],
    half: [0, 1, 2].map(
      (axis) => (((high[axis] ?? 0) - (low[axis] ?? 0)) / 2) * (lengths[axis] ?? 1),
    ) as [number, number, number],
  };
}

const dot = (a: readonly number[], b: readonly number[]) =>
  (a[0] ?? 0) * (b[0] ?? 0) + (a[1] ?? 0) * (b[1] ?? 0) + (a[2] ?? 0) * (b[2] ?? 0);

const cross = (a: readonly number[], b: readonly number[]): [number, number, number] => [
  (a[1] ?? 0) * (b[2] ?? 0) - (a[2] ?? 0) * (b[1] ?? 0),
  (a[2] ?? 0) * (b[0] ?? 0) - (a[0] ?? 0) * (b[2] ?? 0),
  (a[0] ?? 0) * (b[1] ?? 0) - (a[1] ?? 0) * (b[0] ?? 0),
];

/**
 * Whether two placed boxes overlap or come within `margin` of each other:
 * no axis separates them (the separating axis test, fifteen axes).
 */
export function boxesMeet(a: PlacedBox, b: PlacedBox, margin: number): boolean {
  const between = [0, 1, 2].map((axis) => (b.centre[axis] ?? 0) - (a.centre[axis] ?? 0));
  const reach = (box: PlacedBox, axis: readonly number[]) =>
    box.axes.reduce(
      (sum, own, index) => sum + Math.abs(dot(own, axis)) * (box.half[index] ?? 0),
      0,
    );
  const axes: [number, number, number][] = [...a.axes, ...b.axes];
  for (const own of a.axes) {
    for (const other of b.axes) {
      const axis = cross(own, other);
      const length = Math.hypot(...axis);
      if (length > 1e-6) {
        axes.push(axis.map((value) => value / length) as [number, number, number]);
      }
    }
  }
  return axes.every(
    (axis) => Math.abs(dot(between, axis)) <= reach(a, axis) + reach(b, axis) + margin,
  );
}

/** Half the diagonal of a placed box: no point of it is farther from its middle. */
function outerRadius(box: PlacedBox): number {
  return Math.hypot(...box.half);
}

/**
 * The smart selection (ADR 0055): the parts reached from the part at
 * `start` link by link, each part reaching `radius` game units beyond its
 * box. Two parts link where one's box comes within the other's reach. The
 * caller passes only parts of one mod; the chain grows through them alone.
 * Returns the indices of the chain's parts, the start first.
 */
export function chainFrom(boxes: PlacedBox[], start: number, radius: number): number[] {
  if (boxes[start] === undefined) {
    return [];
  }
  const outer = boxes.map(outerRadius);
  const found = [start];
  const left = new Set(boxes.map((_, index) => index));
  left.delete(start);
  for (let at = 0; at < found.length; at += 1) {
    const from = found[at] ?? start;
    const box = boxes[from];
    if (box === undefined) {
      continue;
    }
    for (const other of [...left]) {
      const candidate = boxes[other];
      if (candidate === undefined) {
        continue;
      }
      // Far apart by their spheres already: no axis needed.
      const apart = Math.hypot(
        candidate.centre[0] - box.centre[0],
        candidate.centre[1] - box.centre[1],
        candidate.centre[2] - box.centre[2],
      );
      if (apart > (outer[from] ?? 0) + (outer[other] ?? 0) + radius) {
        continue;
      }
      if (boxesMeet(box, candidate, radius)) {
        found.push(other);
        left.delete(other);
      }
    }
  }
  return found;
}

/**
 * How an object is drawn: plainly, as part of a collision, picked, as part
 * of the collision looked at, as hidden by the patch, not at all, or in
 * the chain of the picked object.
 */
export const LOOK = {
  plain: 0,
  marked: 1,
  selected: 2,
  focus: 3,
  ghost: 4,
  gone: 5,
  chained: 6,
} as const;
export type Look = (typeof LOOK)[keyof typeof LOOK];

/** What is known of a model: being fetched, its surfaces, or nothing to draw. */
interface Model {
  surfaces: Surface[] | "loading" | "missing";
  /** The middle of its points, in its own units, to pick it by. */
  centre: [number, number, number];
  /** Half the diagonal of the box around its points, in its own units. */
  radius: number;
  /** The box around its points, in its own units: lowest and highest corner. */
  box: [[number, number, number], [number, number, number]];
  /** Its objects: one matrix and one look each, in one buffer. */
  instances: WebGLBuffer | null;
  /** What its buffer holds, to change the looks in place. */
  data: Float32Array | null;
  count: number;
  /** Its objects, in the order of the buffer. */
  own: Instance[];
}

export class AreaLayer {
  private readonly program: WebGLProgram;
  private readonly picking: WebGLProgram;
  private readonly models = new Map<string, Model>();
  private readonly textures = new Map<string, Shared>();
  private readonly queue: string[] = [];
  private running = 0;
  private disposed = false;
  private instances: Instance[] = [];
  private selected = new Set<string>();
  /** The chain of the picked object, lit faintly (ADR 0055). */
  private chained = new Set<string>();
  private readonly hullProgram: WebGLProgram;
  /** The sphere the hulls are drawn from, with its points and triangles. */
  private readonly hullShape: {
    vao: WebGLVertexArrayObject;
    buffers: WebGLBuffer[];
    count: number;
  };
  /** The hulls of the chain, one per part, and how far they reach. */
  private hulls: { buffer: WebGLBuffer | null; count: number; radius: number } = {
    buffer: null,
    count: 0,
    radius: 0,
  };
  /** The objects shown, as last set. */
  area: AreaObjects | null = null;

  constructor(
    private readonly gl: WebGL2RenderingContext,
    private readonly api: Api,
    private readonly changed: () => void,
  ) {
    this.program = linked(gl, VERTEX, FRAGMENT);
    this.picking = linked(gl, PICK_VERTEX, PICK_FRAGMENT);
    this.hullProgram = linked(gl, HULL_VERTEX, HULL_FRAGMENT, HULL_LOCATIONS);
    const sphere = hullSphere();
    const vao = gl.createVertexArray();
    gl.bindVertexArray(vao);
    const points = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, points);
    gl.bufferData(gl.ARRAY_BUFFER, sphere.points, gl.STATIC_DRAW);
    gl.enableVertexAttribArray(0);
    gl.vertexAttribPointer(0, 3, gl.FLOAT, false, 0, 0);
    const triangles = gl.createBuffer();
    gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, triangles);
    gl.bufferData(gl.ELEMENT_ARRAY_BUFFER, sphere.triangles, gl.STATIC_DRAW);
    gl.bindVertexArray(null);
    this.hullShape = { vao, buffers: [points, triangles], count: sphere.triangles.length };
  }

  /** How an object is drawn now: picked, in the chain, or as its look says. */
  private lookOf(key: string, look: Look): Look {
    if (this.selected.has(key)) {
      return LOOK.selected;
    }
    if (this.chained.has(key) && look !== LOOK.ghost && look !== LOOK.gone) {
      return LOOK.chained;
    }
    return look;
  }

  /** Shows `area`, each object drawn as `look` says by its key (see {@link objectKey}). */
  show(area: AreaObjects, look: (key: string) => Look) {
    this.area = area;
    this.instances = area.objects.map((object) => {
      const shown = look(objectKey(object.id.plugin, object.id.localId));
      return { object, matrix: placement(object), marked: shown !== LOOK.plain, look: shown };
    });
    for (const model of new Set(area.objects.map((object) => object.model))) {
      if (!this.models.has(model) && !this.queue.includes(model)) {
        this.queue.push(model);
      }
    }
    // The models are fetched closest to the middle first.
    const [cx, cy] = area.centre;
    const distance = new Map<string, number>();
    for (const { position, model } of area.objects) {
      const away = Math.hypot(position[0] - cx, position[1] - cy);
      distance.set(model, Math.min(distance.get(model) ?? Infinity, away));
    }
    this.queue.sort((a, b) => (distance.get(a) ?? 0) - (distance.get(b) ?? 0));
    for (const model of this.models.keys()) {
      this.place(model);
    }
    this.pump();
  }

  /** Draws the objects anew as `look` says. */
  restyle(look: (key: string) => Look) {
    for (const instance of this.instances) {
      instance.look = look(objectKey(instance.object.id.plugin, instance.object.id.localId));
      instance.marked = instance.look !== LOOK.plain;
    }
    this.relookAll();
  }

  /**
   * Writes the looks anew into every model's buffer, in place where the
   * model holds the same objects as when it was placed (a view drawn
   * several times with other looks, many times a second).
   */
  private relookAll() {
    const { gl } = this;
    for (const [name, model] of this.models) {
      const { data, instances } = model;
      if (data === null || instances === null || data.length !== model.own.length * 17) {
        this.place(name);
        continue;
      }
      model.own.forEach((instance, index) => {
        const key = objectKey(instance.object.id.plugin, instance.object.id.localId);
        data[index * 17 + 16] = this.lookOf(key, instance.look);
      });
      gl.bindBuffer(gl.ARRAY_BUFFER, instances);
      gl.bufferSubData(gl.ARRAY_BUFFER, 0, data);
    }
  }

  /**
   * Picks the objects with `keys` (see {@link objectKey}), or none; a chain
   * shown goes unless the same objects are picked again.
   */
  select(keys: string[]) {
    const same = keys.length === this.selected.size && keys.every((key) => this.selected.has(key));
    this.selected = new Set(keys);
    if (!same) {
      this.chained = new Set();
      this.hulls.count = 0;
    }
    this.relookAll();
  }

  /**
   * The chain of `object` (ADR 0055): the parts of its mod linked to it,
   * each reaching `radius` game units beyond its box, as far as the area
   * shows them and their models are read. Parts of other mods and of the
   * game are passed over; they neither join the chain nor break it.
   */
  chain(object: AreaObject, radius: number): AreaObject[] {
    const owner = ownerOf(object);
    const isStart = (other: AreaObject) =>
      other.id.plugin === object.id.plugin && other.id.localId === object.id.localId;
    const parts = this.instances.flatMap((instance) => {
      const other = instance.object;
      const ours =
        isStart(other) || ownerOf(other) === owner || other.plugin.toLowerCase() === owner;
      const model = this.models.get(other.model);
      if (!ours || model === undefined || !Array.isArray(model.surfaces)) {
        return [];
      }
      return [{ object: other, box: placedBox(model.box, instance.matrix) }];
    });
    const start = parts.findIndex((part) => isStart(part.object));
    if (start < 0) {
      return [object];
    }
    return chainFrom(
      parts.map((part) => part.box),
      start,
      radius,
    ).flatMap((index) => {
      const part = parts[index];
      return part === undefined ? [] : [part.object];
    });
  }

  /**
   * Lights `chain` faintly besides what is picked and, with `hulls`, draws
   * each part's reach of `radius` game units around it: the picked ones'
   * stronger. An empty chain clears it.
   */
  showChain(chain: AreaObject[], radius: number, hulls: boolean) {
    const { gl } = this;
    const keys = new Set(chain.map((part) => objectKey(part.id.plugin, part.id.localId)));
    this.chained = keys;
    const placed = hulls
      ? this.instances.flatMap((instance) => {
          const key = objectKey(instance.object.id.plugin, instance.object.id.localId);
          const model = this.models.get(instance.object.model);
          if (
            !(keys.has(key) || this.selected.has(key)) ||
            model === undefined ||
            !Array.isArray(model.surfaces)
          ) {
            return [];
          }
          return [{ box: placedBox(model.box, instance.matrix), main: this.selected.has(key) }];
        })
      : [];
    const data = new Float32Array(placed.length * HULL_STRIDE);
    placed.forEach(({ box, main }, index) => {
      const at = index * HULL_STRIDE;
      data.set(box.centre, at);
      box.axes.forEach((axis, which) => {
        const half = box.half[which] ?? 0;
        data.set(
          axis.map((value) => value * half),
          at + 3 + which * 3,
        );
      });
      data[at + 12] = main ? 1 : 0.5;
    });
    if (this.hulls.buffer === null) {
      this.hulls.buffer = gl.createBuffer();
    }
    gl.bindBuffer(gl.ARRAY_BUFFER, this.hulls.buffer);
    gl.bufferData(gl.ARRAY_BUFFER, data, gl.DYNAMIC_DRAW);
    gl.bindVertexArray(this.hullShape.vao);
    const stride = HULL_STRIDE * 4;
    for (let location = 1; location <= 5; location += 1) {
      gl.enableVertexAttribArray(location);
      gl.vertexAttribPointer(
        location,
        location === 5 ? 1 : 3,
        gl.FLOAT,
        false,
        stride,
        (location - 1) * 12,
      );
      gl.vertexAttribDivisor(location, 1);
    }
    gl.bindVertexArray(null);
    this.hulls.count = placed.length;
    this.hulls.radius = radius;
    this.relookAll();
  }

  /**
   * The objects placed by the same plugin as `object` that hang together
   * with it: each touches another of them, their models' boxes, turned and
   * placed as the game places them, meeting within a hand's breadth, as far
   * as the area shows them. A bridge or a scaffold built of parts.
   */
  connected(
    object: AreaObject,
    margin = TOUCHING,
    /** Only parts placed within `radius` game units of `x`, `y`, if given. */
    near?: { x: number; y: number; radius: number },
  ): AreaObject[] {
    const owner = ownerOf(object);
    const isStart = (other: AreaObject) =>
      other.id.plugin === object.id.plugin && other.id.localId === object.id.localId;
    const isNear = (other: AreaObject) =>
      near === undefined ||
      isStart(other) ||
      Math.hypot(other.position[0] - near.x, other.position[1] - near.y) <= near.radius;
    // What a structure grows through depends on what it starts from: from
    // something built, through what is built (a gate and its walls, not the
    // crates in front of it); from clutter, through clutter (a pile of
    // crates). Never through trees, rocks or the landscape. The part
    // started from always belongs to it.
    const startModel = this.models.get(object.model);
    const startKind =
      startModel === undefined || !Array.isArray(startModel.surfaces)
        ? "built"
        : kindOf(object.model, sizeOf(startModel.box, object.scale));
    const grows = startKind === "clutter" ? "clutter" : "built";
    // Decor of the same mod on what it builds (plants in a planter, a
    // banner, a basket) is taken along as leaves: it belongs to the
    // structure it touches, but the structure does not grow on from it.
    // Decor counts also where the mod's version of another mod's object
    // wins (a book it moved onto its shelf); what it grows through is its own.
    const leaves: { object: AreaObject; box: PlacedBox }[] = [];
    const candidates = this.instances
      .filter((instance) => {
        const other = instance.object;
        return isNear(other) && (ownerOf(other) === owner || other.plugin.toLowerCase() === owner);
      })
      .flatMap((instance) => {
        const model = this.models.get(instance.object.model);
        if (model === undefined || !Array.isArray(model.surfaces)) {
          return [];
        }
        const kind = kindOf(instance.object.model, sizeOf(model.box, instance.object.scale));
        const box = placedBox(model.box, instance.matrix);
        const own = ownerOf(instance.object) === owner;
        if (!isStart(instance.object) && (kind !== grows || !own)) {
          if (grows === "built" && isDecor(instance.object.model, kind)) {
            leaves.push({ object: instance.object, box });
          }
          return [];
        }
        return [{ object: instance.object, box }];
      });
    const start = candidates.find(
      (candidate) =>
        candidate.object.id.plugin === object.id.plugin &&
        candidate.object.id.localId === object.id.localId,
    );
    if (start === undefined) {
      return [object];
    }
    const found = [start];
    const left = new Set(candidates.filter((candidate) => candidate !== start));
    for (let index = 0; index < found.length; index += 1) {
      const from = found[index];
      if (from === undefined) {
        break;
      }
      for (const other of [...left]) {
        if (boxesMeet(from.box, other.box, margin)) {
          found.push(other);
          left.delete(other);
        }
      }
    }
    const touching = leaves.filter((leaf) =>
      found.some((part) => boxesMeet(part.box, leaf.box, margin)),
    );
    return [...found, ...touching].map((candidate) => candidate.object);
  }

  /**
   * The room an object's model box takes as placed, in cubic game units
   * (each side at least one unit, so that flat parts count); null until
   * its model is read.
   */
  volumeOf(object: AreaObject): number | null {
    const model = this.models.get(object.model);
    if (model === undefined || !Array.isArray(model.surfaces)) {
      return null;
    }
    const [low, high] = model.box;
    const side = (axis: 0 | 1 | 2) => Math.max(1, (high[axis] - low[axis]) * object.scale);
    return side(0) * side(1) * side(2);
  }

  /**
   * Whether an object is decor (clutter, small plants) that rests on what
   * it touches; null until its model is read.
   */
  isDecorPart(object: AreaObject): boolean | null {
    const model = this.models.get(object.model);
    if (model === undefined || !Array.isArray(model.surfaces)) {
      return null;
    }
    return isDecor(object.model, kindOf(object.model, sizeOf(model.box, object.scale)));
  }

  /** How many of the area's models are there, of how many. */
  get progress(): { done: number; total: number } {
    const wanted = new Set(this.instances.map((instance) => instance.object.model));
    let done = 0;
    for (const model of wanted) {
      const known = this.models.get(model);
      if (known !== undefined && known.surfaces !== "loading") {
        done += 1;
      }
    }
    return { done, total: wanted.size };
  }

  /** The objects shown with where their middle stands, in game units. */
  get shown(): {
    object: AreaObject;
    marked: boolean;
    look: Look;
    centre: [number, number, number];
  }[] {
    return this.instances.map((instance) => {
      const [x, y, z] = this.models.get(instance.object.model)?.centre ?? [0, 0, 0];
      const m = instance.matrix;
      const at = (row: number) =>
        (m[row] ?? 0) * x + (m[4 + row] ?? 0) * y + (m[8 + row] ?? 0) * z + (m[12 + row] ?? 0);
      return {
        object: instance.object,
        marked: instance.marked,
        look: instance.look,
        centre: [at(0), at(1), at(2)],
      };
    });
  }

  private pump() {
    while (this.running < PARALLEL && this.queue.length > 0) {
      const name = this.queue.shift();
      if (name === undefined) {
        break;
      }
      this.running += 1;
      const model: Model = {
        surfaces: "loading",
        centre: [0, 0, 0],
        radius: 0,
        box: [
          [0, 0, 0],
          [0, 0, 0],
        ],
        instances: null,
        data: null,
        count: 0,
        own: [],
      };
      this.models.set(name, model);
      this.api
        .areaMesh(name)
        .then((mesh) => {
          if (mesh === null || this.disposed) {
            model.surfaces = "missing";
            return;
          }
          this.upload(mesh, model);
          this.place(name);
        })
        .catch(() => {
          model.surfaces = "missing";
        })
        .finally(() => {
          this.running -= 1;
          if (!this.disposed) {
            this.pump();
            this.changed();
          }
        });
    }
  }

  /** Puts the matrices and looks of a model's objects into its buffer. */
  private place(name: string) {
    const model = this.models.get(name);
    if (model === undefined || !Array.isArray(model.surfaces) || this.disposed) {
      return;
    }
    const { gl } = this;
    const own = this.instances.filter((instance) => instance.object.model === name);
    const data = new Float32Array(own.length * 17);
    own.forEach((instance, index) => {
      data.set(instance.matrix, index * 17);
      const key = objectKey(instance.object.id.plugin, instance.object.id.localId);
      data[index * 17 + 16] = this.lookOf(key, instance.look);
    });
    if (model.instances === null) {
      model.instances = gl.createBuffer();
    }
    gl.bindBuffer(gl.ARRAY_BUFFER, model.instances);
    gl.bufferData(gl.ARRAY_BUFFER, data, gl.DYNAMIC_DRAW);
    model.data = data;
    model.count = own.length;
    model.own = own;
    const matrixAt = gl.getAttribLocation(this.program, "a_model");
    const lookAt = gl.getAttribLocation(this.program, "a_look");
    const stride = 17 * 4;
    for (const surface of model.surfaces) {
      gl.bindVertexArray(surface.vao);
      gl.bindBuffer(gl.ARRAY_BUFFER, model.instances);
      for (let column = 0; column < 4 && matrixAt >= 0; column++) {
        gl.enableVertexAttribArray(matrixAt + column);
        gl.vertexAttribPointer(matrixAt + column, 4, gl.FLOAT, false, stride, column * 16);
        gl.vertexAttribDivisor(matrixAt + column, 1);
      }
      if (lookAt >= 0) {
        gl.enableVertexAttribArray(lookAt);
        gl.vertexAttribPointer(lookAt, 1, gl.FLOAT, false, stride, 64);
        gl.vertexAttribDivisor(lookAt, 1);
      }
    }
    gl.bindVertexArray(null);
  }

  private upload(mesh: LodMesh, model: Model) {
    const { gl } = this;
    let sum = [0, 0, 0];
    let points = 0;
    const low = [Infinity, Infinity, Infinity];
    const high = [-Infinity, -Infinity, -Infinity];
    model.surfaces = mesh.shapes.flatMap((shape) => {
      const positions = new Float32Array(bytesOf(shape.positions));
      const indices = new Uint32Array(bytesOf(shape.indices));
      if (positions.length === 0 || indices.length === 0) {
        return [];
      }
      for (let index = 0; index + 2 < positions.length; index += 3) {
        sum = [
          (sum[0] ?? 0) + (positions[index] ?? 0),
          (sum[1] ?? 0) + (positions[index + 1] ?? 0),
          (sum[2] ?? 0) + (positions[index + 2] ?? 0),
        ];
        points += 1;
        for (let axis = 0; axis < 3; axis += 1) {
          const value = positions[index + axis] ?? 0;
          low[axis] = Math.min(low[axis] ?? 0, value);
          high[axis] = Math.max(high[axis] ?? 0, value);
        }
      }
      const uvs =
        shape.uvs === ""
          ? new Float32Array((positions.length / 3) * 2)
          : new Float32Array(bytesOf(shape.uvs));
      const vao = gl.createVertexArray();
      gl.bindVertexArray(vao);
      const buffers: WebGLBuffer[] = [];
      const attribute = (name: string, data: Float32Array, size: number) => {
        const location = gl.getAttribLocation(this.program, name);
        const buffer = gl.createBuffer();
        buffers.push(buffer);
        gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
        gl.bufferData(gl.ARRAY_BUFFER, data, gl.STATIC_DRAW);
        if (location >= 0) {
          gl.enableVertexAttribArray(location);
          gl.vertexAttribPointer(location, size, gl.FLOAT, false, 0, 0);
        }
      };
      attribute("a_position", positions, 3);
      attribute("a_normal", normalsOf(positions, indices), 3);
      attribute("a_uv", uvs, 2);
      const elements = gl.createBuffer();
      buffers.push(elements);
      gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, elements);
      gl.bufferData(gl.ELEMENT_ARRAY_BUFFER, indices, gl.STATIC_DRAW);
      gl.bindVertexArray(null);
      return [
        {
          vao,
          buffers,
          count: indices.length,
          texture: shape.texture ?? null,
          alphaTest: shape.alphaTest,
        },
      ];
    });
    if (points > 0) {
      model.centre = [(sum[0] ?? 0) / points, (sum[1] ?? 0) / points, (sum[2] ?? 0) / points];
      model.radius =
        Math.hypot(
          (high[0] ?? 0) - (low[0] ?? 0),
          (high[1] ?? 0) - (low[1] ?? 0),
          (high[2] ?? 0) - (low[2] ?? 0),
        ) / 2;
      model.box = [
        [low[0] ?? 0, low[1] ?? 0, low[2] ?? 0],
        [high[0] ?? 0, high[1] ?? 0, high[2] ?? 0],
      ];
    }
  }

  private texture(path: string): WebGLTexture | null | "pending" {
    const known = this.textures.get(path);
    if (known === "missing") {
      return null;
    }
    if (known !== undefined && known !== "loading") {
      return known;
    }
    if (known === undefined) {
      this.textures.set(path, "loading");
      this.api
        .areaTexture(path, TEXTURE_SIDE)
        .then((url) => {
          if (url === null || this.disposed) {
            this.textures.set(path, "missing");
            this.changed();
            return;
          }
          const image = new Image();
          image.onload = () => {
            if (this.disposed) {
              return;
            }
            const { gl } = this;
            const made = gl.createTexture();
            gl.bindTexture(gl.TEXTURE_2D, made);
            gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, image);
            gl.generateMipmap(gl.TEXTURE_2D);
            gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR_MIPMAP_LINEAR);
            gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.REPEAT);
            gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.REPEAT);
            this.textures.set(path, made);
            this.changed();
          };
          image.onerror = () => this.textures.set(path, "missing");
          image.src = url;
        })
        .catch(() => this.textures.set(path, "missing"));
    }
    return "pending";
  }

  /** Draws the objects with `matrix` (cells, heights as drawn). */
  draw(matrix: Mat4, exaggeration: number, colours: { mark: Vec3; focus: Vec3; ghost: Vec3 }) {
    if (this.disposed || this.instances.length === 0) {
      return;
    }
    const { gl } = this;
    // biome-ignore lint/correctness/useHookAtTopLevel: WebGL's useProgram is no React hook.
    gl.useProgram(this.program);
    const at = (name: string) => gl.getUniformLocation(this.program, name);
    gl.uniformMatrix4fv(at("u_matrix"), false, matrix);
    gl.uniform1f(at("u_exaggeration"), exaggeration);
    gl.uniform3f(at("u_mark"), ...colours.mark);
    gl.uniform3f(at("u_focus"), ...colours.focus);
    gl.uniform3f(at("u_ghost"), ...colours.ghost);
    gl.uniform1i(at("u_texture"), 0);
    const hasTexture = at("u_hasTexture");
    const alphaTest = at("u_alphaTest");
    gl.activeTexture(gl.TEXTURE0);
    for (const model of this.models.values()) {
      if (!Array.isArray(model.surfaces) || model.count === 0) {
        continue;
      }
      for (const surface of model.surfaces) {
        const texture = surface.texture === null ? null : this.texture(surface.texture);
        if (texture === "pending") {
          continue;
        }
        gl.bindTexture(gl.TEXTURE_2D, texture);
        gl.uniform1i(hasTexture, texture === null ? 0 : 1);
        gl.uniform1i(alphaTest, surface.alphaTest ? 1 : 0);
        gl.bindVertexArray(surface.vao);
        gl.drawElementsInstanced(gl.TRIANGLES, surface.count, gl.UNSIGNED_INT, 0, model.count);
      }
    }
    gl.bindVertexArray(null);
    this.drawHulls(matrix, exaggeration);
  }

  /** Draws the reach of the chain's parts, see-through, over what is drawn. */
  private drawHulls(matrix: Mat4, exaggeration: number) {
    if (this.hulls.count === 0) {
      return;
    }
    const { gl } = this;
    // biome-ignore lint/correctness/useHookAtTopLevel: WebGL's useProgram is no React hook.
    gl.useProgram(this.hullProgram);
    const at = (name: string) => gl.getUniformLocation(this.hullProgram, name);
    gl.uniformMatrix4fv(at("u_matrix"), false, matrix);
    gl.uniform1f(at("u_exaggeration"), exaggeration);
    gl.uniform1f(at("u_radius"), this.hulls.radius);
    gl.uniform3f(at("u_colour"), ...HULL_COLOUR);
    const blending = gl.isEnabled(gl.BLEND);
    const culling = gl.isEnabled(gl.CULL_FACE);
    gl.enable(gl.BLEND);
    gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
    gl.disable(gl.CULL_FACE);
    gl.depthMask(false);
    gl.bindVertexArray(this.hullShape.vao);
    gl.drawElementsInstanced(
      gl.TRIANGLES,
      this.hullShape.count,
      gl.UNSIGNED_SHORT,
      0,
      this.hulls.count,
    );
    gl.bindVertexArray(null);
    gl.depthMask(true);
    if (!blending) {
      gl.disable(gl.BLEND);
    }
    if (culling) {
      gl.enable(gl.CULL_FACE);
    }
  }

  /**
   * The object drawn at the point `x`, `y` of the drawing buffer (from
   * the bottom left), as seen with `matrix`; none where no object is.
   */
  pick(
    matrix: Mat4,
    exaggeration: number,
    x: number,
    y: number,
    width: number,
    height: number,
  ): AreaObject | null {
    if (this.disposed || this.instances.length === 0) {
      return null;
    }
    const { gl } = this;
    const frame = gl.createFramebuffer();
    const colour = gl.createRenderbuffer();
    const depth = gl.createRenderbuffer();
    gl.bindRenderbuffer(gl.RENDERBUFFER, colour);
    gl.renderbufferStorage(gl.RENDERBUFFER, gl.RGBA8, width, height);
    gl.bindRenderbuffer(gl.RENDERBUFFER, depth);
    gl.renderbufferStorage(gl.RENDERBUFFER, gl.DEPTH_COMPONENT24, width, height);
    gl.bindFramebuffer(gl.FRAMEBUFFER, frame);
    gl.framebufferRenderbuffer(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.RENDERBUFFER, colour);
    gl.framebufferRenderbuffer(gl.FRAMEBUFFER, gl.DEPTH_ATTACHMENT, gl.RENDERBUFFER, depth);
    gl.viewport(0, 0, width, height);
    gl.clearColor(0, 0, 0, 0);
    gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
    gl.enable(gl.DEPTH_TEST);
    // biome-ignore lint/correctness/useHookAtTopLevel: WebGL's useProgram is no React hook.
    gl.useProgram(this.picking);
    const at = (name: string) => gl.getUniformLocation(this.picking, name);
    gl.uniformMatrix4fv(at("u_matrix"), false, matrix);
    gl.uniform1f(at("u_exaggeration"), exaggeration);
    gl.uniform1i(at("u_texture"), 0);
    gl.activeTexture(gl.TEXTURE0);
    const order: Instance[] = [];
    for (const model of this.models.values()) {
      if (!Array.isArray(model.surfaces) || model.count === 0) {
        continue;
      }
      gl.uniform1f(at("u_base"), order.length);
      order.push(...model.own);
      for (const surface of model.surfaces) {
        const texture = surface.texture === null ? null : this.textures.get(surface.texture);
        const ready = texture !== undefined && texture !== "loading" && texture !== "missing";
        gl.bindTexture(gl.TEXTURE_2D, ready ? texture : null);
        gl.uniform1i(at("u_hasTexture"), ready ? 1 : 0);
        gl.uniform1i(at("u_alphaTest"), surface.alphaTest ? 1 : 0);
        gl.bindVertexArray(surface.vao);
        gl.drawElementsInstanced(gl.TRIANGLES, surface.count, gl.UNSIGNED_INT, 0, model.count);
      }
    }
    gl.bindVertexArray(null);
    const pixel = new Uint8Array(4);
    gl.readPixels(x, y, 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, pixel);
    gl.bindFramebuffer(gl.FRAMEBUFFER, null);
    gl.deleteFramebuffer(frame);
    gl.deleteRenderbuffer(colour);
    gl.deleteRenderbuffer(depth);
    const id = (pixel[0] ?? 0) + (pixel[1] ?? 0) * 256 + (pixel[2] ?? 0) * 65536;
    return id === 0 ? null : (order[id - 1]?.object ?? null);
  }

  /** Frees everything; the layer is not used again. */
  dispose() {
    this.disposed = true;
    const { gl } = this;
    for (const model of this.models.values()) {
      if (Array.isArray(model.surfaces)) {
        for (const surface of model.surfaces) {
          gl.deleteVertexArray(surface.vao);
          for (const buffer of surface.buffers) {
            gl.deleteBuffer(buffer);
          }
        }
      }
      gl.deleteBuffer(model.instances);
    }
    this.models.clear();
    for (const texture of this.textures.values()) {
      if (texture !== "loading" && texture !== "missing") {
        gl.deleteTexture(texture);
      }
    }
    this.textures.clear();
    gl.deleteProgram(this.program);
    gl.deleteProgram(this.picking);
    gl.deleteProgram(this.hullProgram);
    gl.deleteVertexArray(this.hullShape.vao);
    for (const buffer of this.hullShape.buffers) {
      gl.deleteBuffer(buffer);
    }
    gl.deleteBuffer(this.hulls.buffer);
  }
}
