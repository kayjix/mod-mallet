// A place up close (ADR 0038): every object placed around a point, by the
// game and by mods, drawn with its own mesh where it stands. The parts of
// a collision are tinted, a picked object is lit up.
//
// The meshes of all models are kept in one pool of buffers, and where the
// objects stand and how they look in two small textures; the objects are
// drawn with one call per material for all models using it, leaving out
// what the camera does not see (ADR 0075).

import { rememberActions } from "./action-places";
import type { Api } from "./api";
import { isGamePlugin } from "./game-plugins";
import type { AreaObject, AreaObjects } from "./generated/area-objects";
import type { LodMesh } from "./generated/lod-mesh";
import type { Mat4, Vec3 } from "./gl";
import { LOCATE_MS } from "./locate-frame";
import { CELL } from "./map-places";
import { MeshPool, normalsOf, POOL_LOCATIONS, Sight } from "./mesh-pool";
import { type Move, movedMatrix } from "./moves";
import { noteMeshes } from "./render-stats";

/** How many meshes are fetched at once. */
const PARALLEL = 6;
/** The largest side of an object's texture. */
const TEXTURE_SIDE = 512;
/** How many texels wide the textures of places and looks are. */
const DATA_WIDTH = 1024;
/** How many draws one call takes at most: their first objects fit in 64 vectors. */
const CHUNK = 256;
/** How many views can draw the same objects with looks of their own. */
const VIEWS = 4;
/** Objects less than this many pixels across are left out. */
const LEAST_PIXELS = 1.5;

/**
 * Which object a point of a draw belongs to, and where it stands and how
 * it looks: read from the textures of places and looks by the number of
 * the draw within its call and of the object within the draw.
 */
const INSTANCE = `
uniform highp sampler2D u_places;
uniform highp sampler2D u_looks;
uniform int u_view;
uniform ivec4 u_bases[${CHUNK / 4}];
int objectIndex() {
  int draw = DRAW_ID;
  return u_bases[draw / 4][draw % 4] + gl_InstanceID;
}
ivec2 dataAt(int texel) {
  return ivec2(texel % ${DATA_WIDTH}, texel / ${DATA_WIDTH});
}
mat4 placeOf(int index) {
  int at = index * 4;
  return mat4(
    texelFetch(u_places, dataAt(at), 0),
    texelFetch(u_places, dataAt(at + 1), 0),
    texelFetch(u_places, dataAt(at + 2), 0),
    texelFetch(u_places, dataAt(at + 3), 0));
}
float lookOf(int index) {
  return texelFetch(u_looks, dataAt(index), 0)[u_view];
}
/** Outside of what is seen: a point that is never drawn. */
const vec4 NOWHERE = vec4(2.0, 2.0, 2.0, 1.0);
`;

/** The start of every vertex shader: with one call for many draws where the browser has it. */
const head = (multi: boolean) =>
  multi
    ? `#version 300 es
#extension GL_ANGLE_multi_draw : require
#define DRAW_ID gl_DrawID
`
    : `#version 300 es
#define DRAW_ID 0
`;

const VERTEX = `
in vec3 a_position;
in vec3 a_normal;
in vec2 a_uv;
uniform mat4 u_matrix;
uniform float u_exaggeration;
out vec2 v_uv;
out vec3 v_normal;
flat out float v_look;
void main() {
  int index = objectIndex();
  float look = lookOf(index);
  v_look = look;
  v_uv = a_uv;
  if (look > 4.5 && look < 5.5) {
    // Hidden, and shown as the game would show it: not at all.
    v_normal = vec3(0.0, 0.0, 1.0);
    gl_Position = NOWHERE;
    return;
  }
  mat4 model = placeOf(index);
  vec4 world = model * vec4(a_position, 1.0);
  v_normal = mat3(model) * a_normal;
  gl_Position = u_matrix * vec4(world.xy / ${CELL}.0, world.z / ${CELL}.0 * u_exaggeration, 1.0);
}`;

const FRAGMENT = `#version 300 es
precision highp float;
in vec2 v_uv;
in vec3 v_normal;
flat in float v_look;
uniform sampler2D u_texture;
uniform bool u_hasTexture;
uniform bool u_alphaTest;
uniform vec3 u_mark;
uniform vec3 u_focus;
uniform vec3 u_ghost;
uniform float u_pulse;
out vec4 colour;
void main() {
  vec4 sampled = u_hasTexture ? texture(u_texture, v_uv) : vec4(0.7, 0.7, 0.7, 1.0);
  if (u_alphaTest && sampled.a < 0.5) {
    discard;
  }
  vec3 light = normalize(vec3(-0.6, 0.7, 1.1));
  float lit = 0.55 + 0.6 * abs(dot(normalize(v_normal), light));
  vec3 shaded = sampled.rgb * lit;
  if (v_look > 7.5) {
    // Located: glows in its own colour, drawn even where it is hidden.
    float glow = 0.5 + 0.5 * sin(u_pulse);
    shaded = mix(shaded, min(shaded * 2.2, vec3(1.0)), glow);
    shaded = mix(shaded, vec3(1.0), 0.35 * glow);
  } else if (v_look > 6.5) {
    // Hidden and in the chain of the picked object: the ghost, lit up.
    if (mod(floor(gl_FragCoord.x) + floor(gl_FragCoord.y), 2.0) < 1.0) {
      discard;
    }
    shaded = mix(mix(shaded, u_ghost, 0.6), vec3(1.0), 0.3);
  } else if (v_look > 5.5) {
    // In the chain of the picked object: lit up, half as much.
    shaded = mix(shaded, vec3(1.0), 0.22);
  } else if (v_look > 4.5) {
    // Hidden, and shown as the game would show it: not at all.
    discard;
  } else if (v_look > 3.5) {
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
uniform sampler2D u_texture;
uniform bool u_hasTexture;
uniform bool u_alphaTest;
out vec4 colour;
void main() {
  if (u_alphaTest && u_hasTexture && texture(u_texture, v_uv).a < 0.5) {
    discard;
  }
  float id = v_id;
  colour = vec4(mod(id, 256.0), mod(floor(id / 256.0), 256.0), floor(id / 65536.0), 255.0) / 255.0;
}`;

const PICK_VERTEX = `
in vec3 a_position;
in vec2 a_uv;
uniform mat4 u_matrix;
uniform float u_exaggeration;
out vec2 v_uv;
flat out float v_id;
void main() {
  int index = objectIndex();
  float look = lookOf(index);
  v_uv = a_uv;
  v_id = float(index + 1);
  if (look > 4.5 && look < 5.5) {
    gl_Position = NOWHERE;
    return;
  }
  vec4 world = placeOf(index) * vec4(a_position, 1.0);
  gl_Position = u_matrix * vec4(world.xy / ${CELL}.0, world.z / ${CELL}.0 * u_exaggeration, 1.0);
}`;

/**
 * The objects seen from straight above, into a depth texture: the higher
 * a point, the nearer (ADR 0074). Hidden objects count too: where the
 * game has loaded a place it draws no level of detail, whether an object
 * there is shown or not.
 */
const TOPS_VERTEX = `
in vec3 a_position;
in vec2 a_uv;
uniform vec4 u_rect;
uniform vec2 u_range;
out vec2 v_uv;
void main() {
  vec4 world = placeOf(objectIndex()) * vec4(a_position, 1.0);
  v_uv = a_uv;
  vec2 at = (world.xy - u_rect.xy) / (u_rect.zw - u_rect.xy) * 2.0 - 1.0;
  float depth = clamp((u_range.y - world.z) / (u_range.y - u_range.x), 0.0, 1.0);
  gl_Position = vec4(at, depth * 2.0 - 1.0, 1.0);
}`;

const TOPS_FRAGMENT = `#version 300 es
precision highp float;
in vec2 v_uv;
uniform sampler2D u_texture;
uniform bool u_hasTexture;
uniform bool u_alphaTest;
out vec4 colour;
void main() {
  if (u_alphaTest && u_hasTexture && texture(u_texture, v_uv).a < 0.5) {
    discard;
  }
  colour = vec4(1.0);
}`;

/** A program drawing from the pool, with its uniforms' locations looked up once. */
interface Program {
  program: WebGLProgram;
  at: (name: string) => WebGLUniformLocation | null;
}

/** A program with the attributes where the pool has them. */
function linked(
  gl: WebGL2RenderingContext,
  multi: boolean,
  vertex: string,
  fragment: string,
): Program {
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
  const parts = [
    shader(gl.VERTEX_SHADER, head(multi) + INSTANCE + vertex),
    shader(gl.FRAGMENT_SHADER, fragment),
  ];
  for (const part of parts) {
    gl.attachShader(made, part);
  }
  for (const [name, location] of POOL_LOCATIONS) {
    gl.bindAttribLocation(made, location, name);
  }
  gl.linkProgram(made);
  for (const part of parts) {
    gl.deleteShader(part);
  }
  if (!gl.getProgramParameter(made, gl.LINK_STATUS)) {
    throw new Error(gl.getProgramInfoLog(made) ?? "program");
  }
  const known = new Map<string, WebGLUniformLocation | null>();
  return {
    program: made,
    at: (name) => {
      if (!known.has(name)) {
        known.set(name, gl.getUniformLocation(made, name));
      }
      return known.get(name) ?? null;
    },
  };
}

/** The browser's way to make many draws with one call (`WEBGL_multi_draw`). */
interface MultiDraw {
  multiDrawElementsInstancedWEBGL(
    mode: number,
    counts: Int32Array,
    countsOffset: number,
    type: number,
    offsets: Int32Array,
    offsetsOffset: number,
    instanceCounts: Int32Array,
    instanceCountsOffset: number,
    drawCount: number,
  ): void;
}

/** A part of a model drawn with one material: a range of the pool's indices. */
interface Surface {
  /** Where its indices start in the pool, and how many there are. */
  first: number;
  count: number;
  texture: string | null;
  alphaTest: boolean;
}

/** The surfaces of all models drawn with one material, in the order drawn. */
interface Batch {
  texture: string | null;
  alphaTest: boolean;
  items: { surface: Surface; model: Model }[];
}

type Shared = WebGLTexture | "loading" | "missing";

/** An object as drawn: its model matrix and how it looks in each view. */
interface Instance {
  object: AreaObject;
  /** Its key, see {@link objectKey}. */
  key: string;
  matrix: Float32Array;
  /** Whether it is drawn other than plainly in the first view. */
  marked: boolean;
  /** How it looks in the first view. */
  look: Look;
  /** How it looks in each view, the first one first. */
  looks: Look[];
}

/** The key by which collisions name an object. */
export const objectKey = (plugin: string, localId: number) => `${plugin.toLowerCase()}:${localId}`;

/**
 * A picture given by its address, decoded away from the page's own
 * thread where the browser can (a PNG in a `data:` address, as the
 * backend sends textures), else through an image element.
 */
async function picture(url: string): Promise<{ source: TexImageSource; close?: () => void }> {
  const comma = url.indexOf(",");
  const head = comma < 0 ? "" : url.slice(0, comma);
  if (typeof createImageBitmap === "function" && /^data:image\/[\w+.-]+;base64$/.test(head))
    try {
      const type = head.slice(5, head.indexOf(";"));
      const blob = new Blob([bytesOf(url.slice(comma + 1))], { type });
      // As WebGL takes a picture by default: colours not multiplied by their alpha.
      const bitmap = await createImageBitmap(blob, { premultiplyAlpha: "none" });
      return { source: bitmap, close: () => bitmap.close() };
    } catch {
      // Read the usual way below.
    }
  const image = new Image();
  await new Promise<void>((resolve, reject) => {
    image.onload = () => resolve();
    image.onerror = () => reject(new Error("picture"));
    image.src = url;
  });
  return { source: image };
}

function bytesOf(base64: string): ArrayBuffer {
  const text = atob(base64);
  const bytes = new Uint8Array(text.length);
  for (let index = 0; index < text.length; index++) {
    bytes[index] = text.charCodeAt(index);
  }
  return bytes.buffer;
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

/** What the card of a picked object knows of the view it was picked in. */
export interface PartTools {
  /** What the magic wand took along with the object (ADR 0055), the object first. */
  selection: AreaObject[];
  /** The magic wand's level the selection was made at, 0 to {@link WAND_TOP}. */
  level: number;
  /** The view's objects, to see where moved objects would stand (ADR 0060). */
  layer?: AreaLayer;
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

/**
 * How far two placed boxes reach into each other, in game units: the
 * least overlap along any separating axis. Zero or less where they do not
 * meet; then minus the gap between them.
 */
export function overlapDepth(a: PlacedBox, b: PlacedBox): number {
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
  return Math.min(
    ...axes.map((axis) => reach(a, axis) + reach(b, axis) - Math.abs(dot(between, axis))),
  );
}

/**
 * Whether two placed boxes meet within `margin`, exactly as
 * {@link boxesMeet} tells, after a quick test that rules out boxes too far
 * apart for it: most parts of a place are far from each other.
 *
 * Where no axis of `a` separates them, the middles are at most
 * `ra + √3·rb + √3·margin` apart (`ra`, `rb` half their diagonals): the
 * reach of `b` along three axes at right angles is at most `√3·rb` in
 * all; and the same the other way round.
 */
export function boxesTouch(a: PlacedBox, b: PlacedBox, margin: number): boolean {
  const dx = b.centre[0] - a.centre[0];
  const dy = b.centre[1] - a.centre[1];
  const dz = b.centre[2] - a.centre[2];
  const ra = outerRadius(a);
  const rb = outerRadius(b);
  // A little over √3, for rounding.
  const root3 = 1.7321;
  const reach = Math.min(ra + root3 * rb, root3 * ra + rb) + root3 * margin;
  return dx * dx + dy * dy + dz * dz <= reach * reach && boxesMeet(a, b, margin);
}

/** Half the diagonal of a placed box: no point of it is farther from its middle. */
function outerRadius(box: PlacedBox): number {
  return Math.hypot(...box.half);
}

/** Whether the point `at` lies inside the placed box `box`. */
function holds(box: PlacedBox, at: readonly number[]): boolean {
  const between = [0, 1, 2].map((axis) => (at[axis] ?? 0) - (box.centre[axis] ?? 0));
  return box.axes.every((axis, index) => Math.abs(dot(between, axis)) <= (box.half[index] ?? 0));
}

/**
 * The parts reached from the part at `start` link by link, at most `most`:
 * two parts link where their boxes come within `gap` game units of each
 * other. The caller passes only the parts that may link. Returns the
 * indices of the chain's parts, the start first.
 */
export function chainFrom(
  boxes: PlacedBox[],
  start: number,
  gap: number,
  most = Number.POSITIVE_INFINITY,
): number[] {
  if (boxes[start] === undefined) {
    return [];
  }
  const outer = boxes.map(outerRadius);
  const found = [start];
  const left = new Set(boxes.map((_, index) => index));
  left.delete(start);
  for (let at = 0; at < found.length && found.length < most; at += 1) {
    const from = found[at] ?? start;
    const box = boxes[from];
    if (box === undefined) {
      continue;
    }
    for (const other of [...left]) {
      const candidate = boxes[other];
      if (candidate === undefined || found.length >= most) {
        continue;
      }
      // Far apart by their spheres already: no axis needed.
      const apart = Math.hypot(
        candidate.centre[0] - box.centre[0],
        candidate.centre[1] - box.centre[1],
        candidate.centre[2] - box.centre[2],
      );
      if (apart > (outer[from] ?? 0) + (outer[other] ?? 0) + gap) {
        continue;
      }
      if (boxesMeet(box, candidate, gap)) {
        found.push(other);
        left.delete(other);
      }
    }
  }
  return found;
}

/**
 * A model's family: its file name without the number of the variant, so
 * that `UpperShelf01` and `UpperShelf02`, or every piece of a stone wall,
 * are alike.
 */
export function modelFamily(model: string): string {
  const name = (model.split(/[\\/]/).pop() ?? model).toLowerCase().replace(/\.nif$/, "");
  return name.replace(/[\d_ -]+[a-z]?$/, "") || name;
}

/** The highest level of the magic wand (ADR 0055); level 0 takes the object alone. */
export const WAND_TOP = 5;

/**
 * What the magic wand chains at each level from 2 on: which parts are
 * alike (the same model, the same family of models, or the same kind of
 * thing: built, clutter or nature), and how far apart two links may be, in
 * game units. Level 1 takes only what lies on or in the object.
 */
const WAND_STEPS: Record<number, { alike: "model" | "family" | "kind"; gap: number }> = {
  2: { alike: "model", gap: 14 },
  3: { alike: "family", gap: 140 },
  4: { alike: "kind", gap: 14 },
  5: { alike: "kind", gap: 140 },
};

/** How far from the picked object the magic wand reaches at most, in game units (about 30 m). */
const WAND_REACH = 2048;

/** How long a located object glows, in milliseconds: as long as its arrow points. */
const PULSE_MS = LOCATE_MS;

/** How many parts the magic wand chains at most. */
const WAND_MOST = 300;

/** How close decor must come to a part to lie on it, in game units. */
const CONTENT_MARGIN = 4;

/**
 * At the levels of a whole structure, how many times longer than the
 * picked part a link may be: a road or a slab of ground of the same mod
 * touches half a town, and hiding it tears holes.
 */
const WAND_LARGER = 4;

/** A link may always be this long, in game units (about 7 m). */
const WAND_LARGE = 512;

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
  /** Hidden by the patch and in the chain of the picked object: the ghost, lit up. */
  chainedGhost: 7,
  /** Located for a moment: glowing, even where it is hidden. */
  pulse: 8,
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
  /** Where its objects start in the textures of places and looks, and how many there are. */
  base: number;
  count: number;
  /** The sphere around all its objects as placed, in game units: x, y, z and radius. */
  bounds: [number, number, number, number];
}

/** The camera a frame is drawn with, and what it sees; reused while the matrix stays. */
interface Seen {
  matrix: Mat4;
  exaggeration: number;
  pixels: number;
  least: number;
  sight: Sight;
  /** Whether each model is seen, by the frame it was last tested for. */
  models: Map<Model, boolean>;
}

/** What growing a structure asks of an object, see `AreaLayer.factsOf`. */
interface Facts {
  owner: string;
  plugin: string;
  kind: "nature" | "clutter" | "built";
  box: PlacedBox;
  decor: boolean;
}

/** The longest side of a placed object's model, as its matrix scales it. */
function scaleOf(matrix: Float32Array): number {
  const column = (index: number) =>
    Math.hypot(matrix[index * 4] ?? 0, matrix[index * 4 + 1] ?? 0, matrix[index * 4 + 2] ?? 0);
  return Math.max(column(0), column(1), column(2));
}

export class AreaLayer {
  private readonly multi: MultiDraw | null;
  private readonly program: Program;
  private readonly picking: Program;
  private readonly tops: Program;
  private readonly pool: MeshPool;
  /** Where each object stands: four texels of its matrix each, in the order of {@link order}. */
  private readonly places: WebGLTexture;
  /** How each object looks in each view: one texel each. */
  private readonly looks: WebGLTexture;
  private looksData = new Float32Array(0);
  /** The objects in the order of the textures: those of one model one after the other. */
  private order: Instance[] = [];
  /** Where each model's objects start in {@link order}, and how many there are, by model. */
  private spans = new Map<string, [number, number]>();
  /** The surfaces by material, made anew when models arrive or the objects change. */
  private batches: Batch[] | null = null;
  private seen: Seen | null = null;
  /** What {@link shown} last told, until something changes. */
  private shownNow: ReturnType<AreaLayer["shownOf"]> | null = null;
  // The lists handed to one call for many draws, reused from frame to frame.
  private readonly counts = new Int32Array(CHUNK);
  private readonly offsets = new Int32Array(CHUNK);
  private readonly instanceCounts = new Int32Array(CHUNK);
  private readonly bases = new Int32Array(CHUNK);
  /** Counts the changes to what is drawn where, see {@link version}. */
  private changes = 0;
  private readonly models = new Map<string, Model>();
  private readonly textures = new Map<string, Shared>();
  private readonly queue: string[] = [];
  private running = 0;
  private disposed = false;
  private instances: Instance[] = [];
  private selected = new Set<string>();
  /** The chain of the picked object, lit faintly (ADR 0055). */
  private chained = new Set<string>();
  /** The objects located for a moment, by key (see {@link AreaLayer.pulse}). */
  private pulsing = new Set<string>();
  private pulseStart = 0;
  private pulseTimer: ReturnType<typeof setInterval> | null = null;
  /** Where moved objects stand now, by key (ADR 0060). */
  private moves = new Map<string, Move>();
  /** What growing a structure asks of each object, until the objects change. */
  private facts: { version: number; known: Map<Instance, Facts> } = {
    version: -1,
    known: new Map(),
  };
  /** The boxes of objects where they stand unmoved, until the objects or models change. */
  private unmoved: { version: number; known: Map<AreaObject, PlacedBox> } = {
    version: -1,
    known: new Map(),
  };
  /** The objects shown, as last set. */
  area: AreaObjects | null = null;

  constructor(
    private readonly gl: WebGL2RenderingContext,
    private readonly api: Api,
    private readonly changed: () => void,
  ) {
    // One call for many draws where the browser has it; one call each where not.
    let multi = gl.getExtension("WEBGL_multi_draw") as MultiDraw | null;
    const programs = (withMulti: boolean) => [
      linked(gl, withMulti, VERTEX, FRAGMENT),
      linked(gl, withMulti, PICK_VERTEX, PICK_FRAGMENT),
      linked(gl, withMulti, TOPS_VERTEX, TOPS_FRAGMENT),
    ];
    let made: Program[];
    try {
      made = programs(multi !== null);
    } catch (error) {
      if (multi === null) {
        throw error;
      }
      multi = null;
      made = programs(false);
    }
    const [program, picking, tops] = made;
    if (program === undefined || picking === undefined || tops === undefined) {
      throw new Error("program");
    }
    this.multi = multi;
    this.program = program;
    this.picking = picking;
    this.tops = tops;
    this.pool = new MeshPool(gl);
    const data = () => {
      const texture = gl.createTexture();
      gl.bindTexture(gl.TEXTURE_2D, texture);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA32F, 1, 1, 0, gl.RGBA, gl.FLOAT, new Float32Array(4));
      return texture;
    };
    this.places = data();
    this.looks = data();
    gl.bindTexture(gl.TEXTURE_2D, null);
  }

  /** Changes whenever an object is drawn elsewhere or a model or texture arrives. */
  get version(): number {
    return this.changes;
  }

  /** How an object is drawn now: picked, in the chain, or as its look says. */
  private lookOf(key: string, look: Look): Look {
    if (this.pulsing.has(key)) {
      return LOOK.pulse;
    }
    if (this.selected.has(key)) {
      return LOOK.selected;
    }
    if (this.chained.has(key)) {
      // Hidden parts the wand took along are shown too, as lit ghosts even
      // where the view leaves hidden parts out, so that they can be seen
      // before they are shown again (ADR 0055).
      return look === LOOK.ghost || look === LOOK.gone ? LOOK.chainedGhost : LOOK.chained;
    }
    return look;
  }

  /**
   * Shows `area`, each object drawn as `look` says by its key (see
   * {@link objectKey}) in every view, until a view is restyled.
   */
  show(area: AreaObjects, look: (key: string) => Look) {
    this.area = area;
    rememberActions(area);
    this.instances = area.objects.map((object) => {
      const key = objectKey(object.id.plugin, object.id.localId);
      const shown = look(key);
      const matrix = movedMatrix(placement(object), this.moves.get(key) ?? null);
      return {
        object,
        key,
        matrix,
        marked: shown !== LOOK.plain,
        look: shown,
        looks: new Array<Look>(VIEWS).fill(shown),
      };
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
    this.arrange();
    this.pump();
  }

  /**
   * Draws each object where its move puts it, by key (see
   * {@link objectKey}); objects without one where they stand (ADR 0060).
   */
  setMoves(moves: Map<string, Move>) {
    const changed = new Set<string>();
    for (const key of new Set([...this.moves.keys(), ...moves.keys()])) {
      if (JSON.stringify(this.moves.get(key)) !== JSON.stringify(moves.get(key))) {
        changed.add(key);
      }
    }
    this.moves = new Map(moves);
    if (changed.size === 0) {
      return;
    }
    let moved = false;
    for (const instance of this.instances) {
      if (changed.has(instance.key)) {
        instance.matrix = movedMatrix(
          placement(instance.object),
          this.moves.get(instance.key) ?? null,
        );
        moved = true;
      }
    }
    if (moved) {
      this.writePlaces();
    }
  }

  /**
   * The box of an object's model where `move` would put it (where it
   * stands with no move); null until its model is read.
   */
  boxOf(object: AreaObject, move: Move | null): PlacedBox | null {
    const model = this.models.get(object.model);
    if (model === undefined || !Array.isArray(model.surfaces)) {
      return null;
    }
    if (move !== null) {
      return placedBox(model.box, movedMatrix(placement(object), move));
    }
    // Where an object stands unmoved is asked often while structures grow.
    if (this.unmoved.version !== this.changes) {
      this.unmoved = { version: this.changes, known: new Map() };
    }
    let box = this.unmoved.known.get(object);
    if (box === undefined) {
      box = placedBox(model.box, placement(object));
      this.unmoved.known.set(object, box);
    }
    return box;
  }

  /** The objects shown within `radius` game units of `point`, as their positions say. */
  objectsNear(point: readonly number[], radius: number): AreaObject[] {
    return this.instances
      .map((instance) => instance.object)
      .filter(
        (object) =>
          Math.hypot(
            object.position[0] - (point[0] ?? 0),
            object.position[1] - (point[1] ?? 0),
            object.position[2] - (point[2] ?? 0),
          ) <= radius,
      );
  }

  /** Draws the objects anew as `look` says, in view `view` (the first one unless given). */
  restyle(look: (key: string) => Look, view = 0) {
    const slot = Math.min(VIEWS - 1, Math.max(0, Math.floor(view)));
    for (const instance of this.instances) {
      const next = look(instance.key);
      instance.looks[slot] = next;
      if (slot === 0 && next !== instance.look) {
        instance.look = next;
        instance.marked = next !== LOOK.plain;
        this.shownNow = null;
      }
    }
    this.relook(slot);
  }

  /**
   * Writes the looks anew into their texture: of view `only`, or of
   * every view; the texture is sent to the card only where a look changed
   * (a view drawn several times with other looks, many times a second).
   */
  private relook(only: number | null = null) {
    const { gl } = this;
    const data = this.looksData;
    let changed = false;
    for (let index = 0; index < this.order.length; index++) {
      const instance = this.order[index];
      if (instance === undefined) {
        continue;
      }
      for (let slot = 0; slot < VIEWS; slot++) {
        if (only !== null && slot !== only) {
          continue;
        }
        const look = this.lookOf(instance.key, instance.looks[slot] ?? instance.look);
        if (data[index * 4 + slot] !== look) {
          data[index * 4 + slot] = look;
          changed = true;
        }
      }
    }
    if (changed && this.order.length > 0) {
      gl.bindTexture(gl.TEXTURE_2D, this.looks);
      const rows = data.length / 4 / DATA_WIDTH;
      gl.texSubImage2D(gl.TEXTURE_2D, 0, 0, 0, DATA_WIDTH, rows, gl.RGBA, gl.FLOAT, data);
      gl.bindTexture(gl.TEXTURE_2D, null);
    }
  }

  /**
   * Puts the objects in the order of their models, and their places and
   * looks into the textures (when the objects shown change).
   */
  private arrange() {
    const { gl } = this;
    this.changes += 1;
    this.shownNow = null;
    this.batches = null;
    const byModel = new Map<string, Instance[]>();
    for (const instance of this.instances) {
      const list = byModel.get(instance.object.model);
      if (list === undefined) {
        byModel.set(instance.object.model, [instance]);
      } else {
        list.push(instance);
      }
    }
    for (const model of this.models.values()) {
      model.count = 0;
    }
    this.order = [];
    this.spans = new Map();
    for (const [name, own] of byModel) {
      this.spans.set(name, [this.order.length, own.length]);
      const model = this.models.get(name);
      if (model !== undefined) {
        model.base = this.order.length;
        model.count = own.length;
      }
      this.order.push(...own);
    }
    const rows = Math.max(1, Math.ceil(this.order.length / DATA_WIDTH));
    this.looksData = new Float32Array(rows * DATA_WIDTH * 4).fill(-1);
    gl.bindTexture(gl.TEXTURE_2D, this.looks);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA32F, DATA_WIDTH, rows, 0, gl.RGBA, gl.FLOAT, null);
    gl.bindTexture(gl.TEXTURE_2D, null);
    this.relook();
    this.writePlaces();
  }

  /** Writes where every object stands into its texture, and each model's sphere around them. */
  private writePlaces() {
    const { gl } = this;
    this.changes += 1;
    this.shownNow = null;
    this.seen = null;
    const rows = Math.max(1, Math.ceil((this.order.length * 4) / DATA_WIDTH));
    const data = new Float32Array(rows * DATA_WIDTH * 4);
    this.order.forEach((instance, index) => {
      data.set(instance.matrix, index * 16);
    });
    gl.bindTexture(gl.TEXTURE_2D, this.places);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA32F, DATA_WIDTH, rows, 0, gl.RGBA, gl.FLOAT, data);
    gl.bindTexture(gl.TEXTURE_2D, null);
    for (const model of this.models.values()) {
      this.bound(model);
    }
  }

  /** The sphere around all objects of a model as placed. */
  private bound(model: Model) {
    const [x, y, z] = model.centre;
    const low = [Infinity, Infinity, Infinity];
    const high = [-Infinity, -Infinity, -Infinity];
    let reach = 0;
    for (let index = model.base; index < model.base + model.count; index++) {
      const m = this.order[index]?.matrix;
      if (m === undefined) {
        continue;
      }
      for (let row = 0; row < 3; row++) {
        const at =
          (m[row] ?? 0) * x + (m[4 + row] ?? 0) * y + (m[8 + row] ?? 0) * z + (m[12 + row] ?? 0);
        low[row] = Math.min(low[row] ?? 0, at);
        high[row] = Math.max(high[row] ?? 0, at);
      }
      reach = Math.max(reach, model.radius * scaleOf(m));
    }
    if (!Number.isFinite(low[0])) {
      model.bounds = [0, 0, 0, 0];
      return;
    }
    const middle = [0, 1, 2].map((axis) => ((low[axis] ?? 0) + (high[axis] ?? 0)) / 2);
    const spread = Math.hypot(
      (high[0] ?? 0) - (low[0] ?? 0),
      (high[1] ?? 0) - (low[1] ?? 0),
      (high[2] ?? 0) - (low[2] ?? 0),
    );
    model.bounds = [middle[0] ?? 0, middle[1] ?? 0, middle[2] ?? 0, spread / 2 + reach];
  }

  /**
   * Picks the objects with `keys` (see {@link objectKey}), or none; a
   * selection of the magic wand goes unless the same objects are picked again.
   */
  select(keys: string[]) {
    const same = keys.length === this.selected.size && keys.every((key) => this.selected.has(key));
    this.selected = new Set(keys);
    if (!same) {
      this.chained = new Set();
    }
    this.relook();
  }

  /**
   * What the magic wand takes along with `object` at `level` (ADR 0055),
   * the object first: from level 1 what lies on or in it, from level 2 on
   * the parts of its mod alike enough to it in a chain, with what lies on
   * or in them. Parts of other mods and of the game are passed over. Only
   * parts within {@link WAND_REACH} of the object count, and their models
   * must be read.
   */
  wand(object: AreaObject, level: number): AreaObject[] {
    if (level <= 0) {
      return [object];
    }
    const owner = ownerOf(object);
    const isStart = (other: AreaObject) =>
      other.id.plugin === object.id.plugin && other.id.localId === object.id.localId;
    const [x, y, z] = object.position;
    const parts = this.instances.flatMap((instance) => {
      const other = instance.object;
      const ours =
        isStart(other) || ownerOf(other) === owner || other.plugin.toLowerCase() === owner;
      const model = this.models.get(other.model);
      const [ox, oy, oz] = other.position;
      if (
        !ours ||
        model === undefined ||
        !Array.isArray(model.surfaces) ||
        Math.hypot(ox - x, oy - y, oz - z) > WAND_REACH
      ) {
        return [];
      }
      return [
        {
          object: other,
          box: placedBox(model.box, instance.matrix),
          kind: kindOf(other.model, sizeOf(model.box, other.scale)),
          size: sizeOf(model.box, other.scale),
        },
      ];
    });
    const start = parts.findIndex((part) => isStart(part.object));
    const first = parts[start];
    if (first === undefined) {
      return [object];
    }
    let chain = [start];
    const step = WAND_STEPS[level];
    if (step !== undefined) {
      const family = modelFamily(first.object.model);
      const alike = (part: (typeof parts)[number]) =>
        step.alike === "model"
          ? part.object.model.toLowerCase() === first.object.model.toLowerCase()
          : step.alike === "family"
            ? modelFamily(part.object.model) === family
            : part.kind === first.kind &&
              part.size <= Math.max(first.size * WAND_LARGER, WAND_LARGE);
      const links = parts.flatMap((part, index) =>
        index === start || (ownerOf(part.object) === owner && alike(part)) ? [index] : [],
      );
      chain = chainFrom(
        links.map((index) => parts[index]?.box ?? first.box),
        links.indexOf(start),
        step.gap,
        WAND_MOST,
      ).map((at) => links[at] ?? start);
    }
    const inChain = new Set(chain);
    const holders = chain.flatMap((index) => {
      const part = parts[index];
      return part === undefined ? [] : [part.box];
    });
    // What lies on or in the chain: decor touching it, or anything whose
    // middle is inside one of its parts. The chain does not grow from it.
    const contents = parts.flatMap((part, index) => {
      if (inChain.has(index)) {
        return [];
      }
      const decor = isDecor(part.object.model, part.kind);
      const held = holders.some((box) =>
        decor ? boxesTouch(box, part.box, CONTENT_MARGIN) : holds(box, part.box.centre),
      );
      return held ? [part.object] : [];
    });
    return [
      ...chain.flatMap((index) => {
        const part = parts[index];
        return part === undefined ? [] : [part.object];
      }),
      ...contents,
    ];
  }

  /**
   * Makes the objects with these ids glow in the view for `ms` milliseconds,
   * so that one can see which ones are meant, also where they are hidden.
   * Returns those the view has, with where they stand; none where they
   * are not shown (yet).
   */
  pulse(objects: readonly { plugin: string; localId: number }[], ms = PULSE_MS): AreaObject[] {
    const wanted = new Set(objects.map((object) => objectKey(object.plugin, object.localId)));
    const found = this.instances
      .map((instance) => instance.object)
      .filter((object) => wanted.has(objectKey(object.id.plugin, object.id.localId)));
    const before = this.pulsing.size;
    this.stopPulse();
    if (found.length === 0) {
      if (before > 0) {
        this.relook();
        this.changed();
      }
      return [];
    }
    this.pulsing = new Set(found.map((object) => objectKey(object.id.plugin, object.id.localId)));
    this.pulseStart = performance.now();
    this.relook();
    this.changed();
    // The glow is drawn by the clock, so the view is drawn anew meanwhile.
    this.pulseTimer = setInterval(() => {
      if (this.disposed || performance.now() - this.pulseStart >= ms) {
        this.stopPulse();
        if (!this.disposed) {
          this.relook();
        }
      }
      this.changed();
    }, 40);
    return found;
  }

  private stopPulse() {
    if (this.pulseTimer !== null) {
      clearInterval(this.pulseTimer);
      this.pulseTimer = null;
    }
    this.pulsing = new Set();
  }

  /** Lights `objects` faintly besides what is picked: what the magic wand took along. */
  showChain(objects: AreaObject[]) {
    this.chained = new Set(objects.map((part) => objectKey(part.id.plugin, part.id.localId)));
    this.relook();
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
    const candidates: { object: AreaObject; box: PlacedBox }[] = [];
    for (const instance of this.instances) {
      const other = instance.object;
      const facts = this.factsOf(instance);
      if (facts === null || !isNear(other) || (facts.owner !== owner && facts.plugin !== owner)) {
        continue;
      }
      const { kind, box } = facts;
      if (!isStart(other) && (kind !== grows || facts.owner !== owner)) {
        if (grows === "built" && facts.decor) {
          leaves.push({ object: other, box });
        }
        continue;
      }
      candidates.push({ object: other, box });
    }
    const start = candidates.find(
      (candidate) =>
        candidate.object.id.plugin === object.id.plugin &&
        candidate.object.id.localId === object.id.localId,
    );
    if (start === undefined) {
      return [object];
    }
    const found = [start];
    let left = candidates.filter((candidate) => candidate !== start);
    for (let index = 0; index < found.length && left.length > 0; index += 1) {
      const from = found[index];
      if (from === undefined) {
        break;
      }
      const still: typeof left = [];
      for (const other of left) {
        if (boxesTouch(from.box, other.box, margin)) {
          found.push(other);
        } else {
          still.push(other);
        }
      }
      left = still;
    }
    const touching = leaves.filter((leaf) =>
      found.some((part) => boxesTouch(part.box, leaf.box, margin)),
    );
    return [...found, ...touching].map((candidate) => candidate.object);
  }

  /**
   * What growing a structure asks of an object, worked out once while
   * the objects stay where they are: the mod it belongs to, its plugin,
   * what kind of part it is, its box as placed and whether it is decor;
   * null until its model is read.
   */
  private factsOf(instance: Instance): Facts | null {
    if (this.facts.version !== this.changes) {
      this.facts = { version: this.changes, known: new Map() };
    }
    const known = this.facts.known.get(instance);
    if (known !== undefined) {
      return known;
    }
    const { object } = instance;
    const model = this.models.get(object.model);
    if (model === undefined || !Array.isArray(model.surfaces)) {
      return null;
    }
    const kind = kindOf(object.model, sizeOf(model.box, object.scale));
    const made: Facts = {
      owner: ownerOf(object),
      plugin: object.plugin.toLowerCase(),
      kind,
      box: placedBox(model.box, instance.matrix),
      decor: isDecor(object.model, kind),
    };
    this.facts.known.set(instance, made);
    return made;
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
    this.shownNow ??= this.shownOf();
    return this.shownNow;
  }

  private shownOf() {
    return this.instances.map((instance) => {
      const [x, y, z] = this.models.get(instance.object.model)?.centre ?? [0, 0, 0];
      const m = instance.matrix;
      const at = (row: number) =>
        (m[row] ?? 0) * x + (m[4 + row] ?? 0) * y + (m[8 + row] ?? 0) * z + (m[12 + row] ?? 0);
      return {
        object: instance.object,
        marked: instance.marked,
        look: instance.look,
        centre: [at(0), at(1), at(2)] as [number, number, number],
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
      const [base, count] = this.spans.get(name) ?? [0, 0];
      const model: Model = {
        surfaces: "loading",
        centre: [0, 0, 0],
        radius: 0,
        box: [
          [0, 0, 0],
          [0, 0, 0],
        ],
        base,
        count,
        bounds: [0, 0, 0, 0],
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
          this.bound(model);
          this.changes += 1;
          this.batches = null;
          this.shownNow = null;
          this.seen = null;
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

  /**
   * Adds a model's mesh to the pool: its shapes ordered by material, so
   * that shapes of the same material make one surface.
   */
  private upload(mesh: LodMesh, model: Model) {
    const started = performance.now();
    let sum = [0, 0, 0];
    let points = 0;
    const low = [Infinity, Infinity, Infinity];
    const high = [-Infinity, -Infinity, -Infinity];
    const materialOf = (shape: LodMesh["shapes"][number]) =>
      `${shape.alphaTest ? 1 : 0}${shape.texture ?? ""}`;
    const shapes = [...mesh.shapes].sort((a, b) => materialOf(a).localeCompare(materialOf(b)));
    const surfaces: Surface[] = [];
    for (const shape of shapes) {
      const positions = new Float32Array(bytesOf(shape.positions));
      const indices = new Uint32Array(bytesOf(shape.indices));
      if (positions.length === 0 || indices.length === 0) {
        continue;
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
      const fitting =
        uvs.length === (positions.length / 3) * 2
          ? uvs
          : new Float32Array((positions.length / 3) * 2);
      const first = this.pool.add(positions, normalsOf(positions, indices), fitting, indices);
      const texture = shape.texture ?? null;
      const last = surfaces[surfaces.length - 1];
      if (
        last !== undefined &&
        last.texture === texture &&
        last.alphaTest === shape.alphaTest &&
        last.first + last.count === first
      ) {
        last.count += indices.length;
      } else {
        surfaces.push({ first, count: indices.length, texture, alphaTest: shape.alphaTest });
      }
    }
    model.surfaces = surfaces;
    noteMeshes(performance.now() - started);
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
            this.changes += 1;
            this.changed();
            return;
          }
          return picture(url).then((image) => {
            if (this.disposed) {
              image.close?.();
              return;
            }
            const { gl } = this;
            const made = gl.createTexture();
            gl.bindTexture(gl.TEXTURE_2D, made);
            gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, image.source);
            gl.generateMipmap(gl.TEXTURE_2D);
            gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR_MIPMAP_LINEAR);
            gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.REPEAT);
            gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.REPEAT);
            image.close?.();
            this.textures.set(path, made);
            this.changes += 1;
            this.changed();
          });
        })
        .catch(() => this.textures.set(path, "missing"));
    }
    return "pending";
  }

  /** The surfaces of the models read, by material, in an order that binds each texture once. */
  private batchesNow(): Batch[] {
    if (this.batches !== null) {
      return this.batches;
    }
    const byMaterial = new Map<string, Batch>();
    for (const model of this.models.values()) {
      if (!Array.isArray(model.surfaces) || model.count === 0) {
        continue;
      }
      for (const surface of model.surfaces) {
        const key = `${surface.alphaTest ? 1 : 0}${surface.texture ?? ""}`;
        let batch = byMaterial.get(key);
        if (batch === undefined) {
          batch = { texture: surface.texture, alphaTest: surface.alphaTest, items: [] };
          byMaterial.set(key, batch);
        }
        batch.items.push({ surface, model });
      }
    }
    // Opaque surfaces first: the alpha tested ones then fail the depth test more often.
    this.batches = [...byMaterial.values()].sort(
      (a, b) => Number(a.alphaTest) - Number(b.alphaTest),
    );
    return this.batches;
  }

  /**
   * What `matrix` sees, kept while the matrix stays (the views of a
   * comparison draw the same objects with the same camera).
   */
  private seenWith(matrix: Mat4, exaggeration: number, least: number): Seen {
    const pixels = this.gl.drawingBufferWidth;
    const known = this.seen;
    if (
      known !== null &&
      known.exaggeration === exaggeration &&
      known.pixels === pixels &&
      known.least === least &&
      known.matrix.every((value, index) => value === matrix[index])
    ) {
      return known;
    }
    this.seen = {
      matrix: new Float32Array(matrix),
      exaggeration,
      pixels,
      least,
      sight: new Sight(matrix, pixels),
      models: new Map(),
    };
    return this.seen;
  }

  /** Whether any object of `model` is seen, as `seen` tells. */
  private sees(seen: Seen, model: Model): boolean {
    const known = seen.models.get(model);
    if (known !== undefined) {
      return known;
    }
    const [x, y, z, radius] = model.bounds;
    const raise = seen.exaggeration;
    const result = seen.sight.sees(
      x / CELL,
      y / CELL,
      (z / CELL) * raise,
      (radius / CELL) * Math.max(1, raise),
      seen.least,
    );
    seen.models.set(model, result);
    return result;
  }

  /**
   * Draws the batches with `program` (bound, its uniforms set): every
   * surface of a seen model, one call per material and up to {@link CHUNK}
   * surfaces. Surfaces whose texture is still on its way are left out
   * unless `waiting` is set, which draws them without it.
   */
  private drawBatches(program: Program, seen: Seen | null, waiting: boolean) {
    const { gl, counts, offsets, instanceCounts, bases, multi } = this;
    const baseAt = program.at("u_bases");
    const hasTexture = program.at("u_hasTexture");
    const alphaTest = program.at("u_alphaTest");
    gl.activeTexture(gl.TEXTURE1);
    gl.bindTexture(gl.TEXTURE_2D, this.places);
    gl.uniform1i(program.at("u_places"), 1);
    gl.activeTexture(gl.TEXTURE2);
    gl.bindTexture(gl.TEXTURE_2D, this.looks);
    gl.uniform1i(program.at("u_looks"), 2);
    gl.activeTexture(gl.TEXTURE0);
    gl.uniform1i(program.at("u_texture"), 0);
    gl.bindVertexArray(this.pool.vao);
    for (const batch of this.batchesNow()) {
      let texture: WebGLTexture | null = null;
      if (batch.texture !== null) {
        const found = waiting ? this.textures.get(batch.texture) : this.texture(batch.texture);
        if (found === "pending") {
          continue;
        }
        texture = found === undefined || found === "loading" || found === "missing" ? null : found;
      }
      let drawn = 0;
      const flush = () => {
        if (drawn === 0) {
          return;
        }
        if (multi !== null) {
          gl.uniform4iv(baseAt, bases);
          multi.multiDrawElementsInstancedWEBGL(
            gl.TRIANGLES,
            counts,
            0,
            gl.UNSIGNED_INT,
            offsets,
            0,
            instanceCounts,
            0,
            drawn,
          );
        }
        drawn = 0;
      };
      let bound = false;
      for (const { surface, model } of batch.items) {
        if (model.count === 0 || (seen !== null && !this.sees(seen, model))) {
          continue;
        }
        if (!bound) {
          gl.bindTexture(gl.TEXTURE_2D, texture);
          gl.uniform1i(hasTexture, texture === null ? 0 : 1);
          gl.uniform1i(alphaTest, batch.alphaTest ? 1 : 0);
          bound = true;
        }
        if (multi === null) {
          gl.uniform4i(baseAt, model.base, 0, 0, 0);
          gl.drawElementsInstanced(
            gl.TRIANGLES,
            surface.count,
            gl.UNSIGNED_INT,
            surface.first * 4,
            model.count,
          );
          continue;
        }
        counts[drawn] = surface.count;
        offsets[drawn] = surface.first * 4;
        instanceCounts[drawn] = model.count;
        bases[drawn] = model.base;
        drawn += 1;
        if (drawn === CHUNK) {
          flush();
        }
      }
      flush();
    }
    gl.bindVertexArray(null);
  }

  /**
   * Draws the objects with `matrix` (cells, heights as drawn), as they
   * look in view `view` (see {@link restyle}).
   */
  draw(
    matrix: Mat4,
    exaggeration: number,
    colours: { mark: Vec3; focus: Vec3; ghost: Vec3 },
    view = 0,
  ) {
    if (this.disposed || this.instances.length === 0) {
      return;
    }
    const { gl } = this;
    const { program, at } = this.program;
    // biome-ignore lint/correctness/useHookAtTopLevel: WebGL's useProgram is no React hook.
    gl.useProgram(program);
    gl.uniformMatrix4fv(at("u_matrix"), false, matrix);
    gl.uniform1f(at("u_exaggeration"), exaggeration);
    gl.uniform1i(at("u_view"), Math.min(VIEWS - 1, Math.max(0, Math.floor(view))));
    gl.uniform3f(at("u_mark"), ...colours.mark);
    gl.uniform3f(at("u_focus"), ...colours.focus);
    gl.uniform3f(at("u_ghost"), ...colours.ghost);
    // Two glows a second, from the moment the pulse began.
    gl.uniform1f(at("u_pulse"), ((performance.now() - this.pulseStart) / 1000) * Math.PI * 4);
    this.drawBatches(this.program, this.seenWith(matrix, exaggeration, LEAST_PIXELS), false);
  }

  /**
   * Draws the objects seen from straight above into the bound frame's
   * depth: `rect` (west, south, east, north) and `range` (lowest and
   * highest height) in game units span it, the highest point nearest.
   * Surfaces whose texture is still on its way are left out, as in
   * {@link draw}.
   */
  drawTops(rect: [number, number, number, number], range: [number, number]) {
    if (this.disposed || this.instances.length === 0) {
      return;
    }
    const { gl } = this;
    const { program, at } = this.tops;
    // biome-ignore lint/correctness/useHookAtTopLevel: WebGL's useProgram is no React hook.
    gl.useProgram(program);
    gl.uniform4f(at("u_rect"), ...rect);
    gl.uniform2f(at("u_range"), ...range);
    this.drawBatches(this.tops, null, false);
  }

  /**
   * The object drawn at the point `x`, `y` of the drawing buffer (from
   * the bottom left), as seen with `matrix` in view `view`; none where no
   * object is.
   */
  pick(
    matrix: Mat4,
    exaggeration: number,
    x: number,
    y: number,
    width: number,
    height: number,
    view = 0,
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
    const { program, at } = this.picking;
    // biome-ignore lint/correctness/useHookAtTopLevel: WebGL's useProgram is no React hook.
    gl.useProgram(program);
    gl.uniformMatrix4fv(at("u_matrix"), false, matrix);
    gl.uniform1f(at("u_exaggeration"), exaggeration);
    gl.uniform1i(at("u_view"), Math.min(VIEWS - 1, Math.max(0, Math.floor(view))));
    this.drawBatches(this.picking, null, true);
    const pixel = new Uint8Array(4);
    gl.readPixels(x, y, 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, pixel);
    gl.bindFramebuffer(gl.FRAMEBUFFER, null);
    gl.deleteFramebuffer(frame);
    gl.deleteRenderbuffer(colour);
    gl.deleteRenderbuffer(depth);
    const id = (pixel[0] ?? 0) + (pixel[1] ?? 0) * 256 + (pixel[2] ?? 0) * 65536;
    return id === 0 ? null : (this.order[id - 1]?.object ?? null);
  }

  /** Frees everything; the layer is not used again. */
  dispose() {
    this.disposed = true;
    this.stopPulse();
    const { gl } = this;
    this.pool.dispose();
    this.models.clear();
    for (const texture of this.textures.values()) {
      if (texture !== "loading" && texture !== "missing") {
        gl.deleteTexture(texture);
      }
    }
    this.textures.clear();
    gl.deleteTexture(this.places);
    gl.deleteTexture(this.looks);
    gl.deleteProgram(this.program.program);
    gl.deleteProgram(this.picking.program);
    gl.deleteProgram(this.tops.program);
  }
}

export { isGamePlugin };
