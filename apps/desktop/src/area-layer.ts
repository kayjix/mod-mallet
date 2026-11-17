// A place up close (ADR 0038): every object placed around a point, by the
// game and by mods, drawn with its own mesh where it stands. The parts of
// a collision are tinted, a picked object is lit up.
//
// Each model is drawn once for all its objects (instancing): their
// matrices and looks are kept in one buffer per model.

import type { Api } from "./api";
import type { AreaObject, AreaObjects } from "./generated/area-objects";
import type { LodMesh } from "./generated/lod-mesh";
import type { Mat4 } from "./gl";
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
out vec4 colour;
void main() {
  vec4 sampled = u_hasTexture ? texture(u_texture, v_uv) : vec4(0.7, 0.7, 0.7, 1.0);
  if (u_alphaTest && sampled.a < 0.5) {
    discard;
  }
  vec3 light = normalize(vec3(-0.6, 0.7, 1.1));
  float lit = 0.55 + 0.6 * abs(dot(normalize(v_normal), light));
  vec3 shaded = sampled.rgb * lit;
  if (v_look > 1.5) {
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

const PICK_VERTEX = `#version 300 es
in vec3 a_position;
in vec2 a_uv;
in mat4 a_model;
uniform mat4 u_matrix;
uniform float u_exaggeration;
uniform float u_base;
out vec2 v_uv;
flat out float v_id;
void main() {
  vec4 world = a_model * vec4(a_position, 1.0);
  v_uv = a_uv;
  v_id = u_base + float(gl_InstanceID) + 1.0;
  gl_Position = u_matrix * vec4(world.xy / ${CELL}.0, world.z / ${CELL}.0 * u_exaggeration, 1.0);
}`;

/** Where every attribute is, the same in both programs. */
const LOCATIONS: [string, number][] = [
  ["a_position", 0],
  ["a_normal", 1],
  ["a_uv", 2],
  ["a_model", 3],
  ["a_look", 7],
];

/** A program with the attributes at {@link LOCATIONS}. */
function linked(gl: WebGL2RenderingContext, vertex: string, fragment: string): WebGLProgram {
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
  for (const [name, location] of LOCATIONS) {
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
 * Where an object stands, as the game places it: scaled, turned about x,
 * then y, then z (the game turns clockwise), and moved; column by column.
 */
export function placement(object: AreaObject): Float32Array {
  const [rx, ry, rz] = object.rotation.map((angle) => -angle) as [number, number, number];
  const [cx, sx] = [Math.cos(rx), Math.sin(rx)];
  const [cy, sy] = [Math.cos(ry), Math.sin(ry)];
  const [cz, sz] = [Math.cos(rz), Math.sin(rz)];
  // Rz · Ry · Rx, row by row.
  const r = [
    [cz * cy, cz * sy * sx - sz * cx, cz * sy * cx + sz * sx],
    [sz * cy, sz * sy * sx + cz * cx, sz * sy * cx - cz * sx],
    [-sy, cy * sx, cy * cx],
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

/** How an object is drawn: plainly, as part of a collision, or picked. */
const PLAIN = 0;
const MARKED = 1;
const SELECTED = 2;

/** What is known of a model: being fetched, its surfaces, or nothing to draw. */
interface Model {
  surfaces: Surface[] | "loading" | "missing";
  /** The middle of its points, in its own units, to pick it by. */
  centre: [number, number, number];
  /** Its objects: one matrix and one look each, in one buffer. */
  instances: WebGLBuffer | null;
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
  private selected: string | null = null;
  /** The objects shown, as last set. */
  area: AreaObjects | null = null;

  constructor(
    private readonly gl: WebGL2RenderingContext,
    private readonly api: Api,
    private readonly changed: () => void,
  ) {
    this.program = linked(gl, VERTEX, FRAGMENT);
    this.picking = linked(gl, PICK_VERTEX, PICK_FRAGMENT);
  }

  /** Shows `area`; objects whose key is in `marked` are tinted. */
  show(area: AreaObjects, marked: Set<string>) {
    this.area = area;
    this.instances = area.objects.map((object) => ({
      object,
      matrix: placement(object),
      marked: marked.has(objectKey(object.id.plugin, object.id.localId)),
    }));
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

  /** Picks the object with `key` (see {@link objectKey}), or none. */
  select(key: string | null) {
    this.selected = key;
    for (const model of this.models.keys()) {
      this.place(model);
    }
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
  get shown(): { object: AreaObject; marked: boolean; centre: [number, number, number] }[] {
    return this.instances.map((instance) => {
      const [x, y, z] = this.models.get(instance.object.model)?.centre ?? [0, 0, 0];
      const m = instance.matrix;
      const at = (row: number) =>
        (m[row] ?? 0) * x + (m[4 + row] ?? 0) * y + (m[8 + row] ?? 0) * z + (m[12 + row] ?? 0);
      return { object: instance.object, marked: instance.marked, centre: [at(0), at(1), at(2)] };
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
        instances: null,
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
      data[index * 17 + 16] = key === this.selected ? SELECTED : instance.marked ? MARKED : PLAIN;
    });
    if (model.instances === null) {
      model.instances = gl.createBuffer();
    }
    gl.bindBuffer(gl.ARRAY_BUFFER, model.instances);
    gl.bufferData(gl.ARRAY_BUFFER, data, gl.DYNAMIC_DRAW);
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
  draw(matrix: Mat4, exaggeration: number, markColour: [number, number, number]) {
    if (this.disposed || this.instances.length === 0) {
      return;
    }
    const { gl } = this;
    // biome-ignore lint/correctness/useHookAtTopLevel: WebGL's useProgram is no React hook.
    gl.useProgram(this.program);
    const at = (name: string) => gl.getUniformLocation(this.program, name);
    gl.uniformMatrix4fv(at("u_matrix"), false, matrix);
    gl.uniform1f(at("u_exaggeration"), exaggeration);
    gl.uniform3f(at("u_mark"), ...markColour);
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
  }
}
