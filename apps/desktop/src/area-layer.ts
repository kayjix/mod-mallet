// A place up close (ADR 0038): every object placed around a point, by the
// game and by mods, drawn with its own mesh where it stands. The parts of
// a collision are tinted.

import type { Api } from "./api";
import type { AreaObject, AreaObjects } from "./generated/area-objects";
import type { LodMesh } from "./generated/lod-mesh";
import { type Mat4, program } from "./gl";
import { CELL } from "./map-places";

/** How many meshes are fetched at once. */
const PARALLEL = 6;
/** The largest side of an object's texture. */
const TEXTURE_SIDE = 512;

const VERTEX = `#version 300 es
in vec3 a_position;
in vec3 a_normal;
in vec2 a_uv;
uniform mat4 u_matrix;
uniform mat4 u_model;
uniform float u_exaggeration;
out vec2 v_uv;
out vec3 v_normal;
void main() {
  vec4 world = u_model * vec4(a_position, 1.0);
  v_uv = a_uv;
  v_normal = mat3(u_model) * a_normal;
  gl_Position = u_matrix * vec4(world.xy / ${CELL}.0, world.z / ${CELL}.0 * u_exaggeration, 1.0);
}`;

const FRAGMENT = `#version 300 es
precision highp float;
in vec2 v_uv;
in vec3 v_normal;
uniform sampler2D u_texture;
uniform bool u_hasTexture;
uniform bool u_alphaTest;
uniform vec4 u_tint;
out vec4 colour;
void main() {
  vec4 sampled = u_hasTexture ? texture(u_texture, v_uv) : vec4(0.7, 0.7, 0.7, 1.0);
  if (u_alphaTest && sampled.a < 0.5) {
    discard;
  }
  vec3 light = normalize(vec3(-0.6, 0.7, 1.1));
  float lit = 0.55 + 0.6 * abs(dot(normalize(v_normal), light));
  vec3 shaded = mix(sampled.rgb * lit, u_tint.rgb, u_tint.a);
  colour = vec4(shaded, 1.0);
}`;

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

export class AreaLayer {
  private readonly program: WebGLProgram;
  private readonly meshes = new Map<string, Surface[] | "loading" | "missing">();
  private readonly textures = new Map<string, Shared>();
  private readonly queue: string[] = [];
  private running = 0;
  private disposed = false;
  private instances: Instance[] = [];
  /** The objects shown, as last set. */
  area: AreaObjects | null = null;

  constructor(
    private readonly gl: WebGL2RenderingContext,
    private readonly api: Api,
    private readonly changed: () => void,
  ) {
    this.program = program(gl, VERTEX, FRAGMENT);
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
      if (!this.meshes.has(model) && !this.queue.includes(model)) {
        this.queue.push(model);
      }
    }
    this.pump();
  }

  /** Whether meshes are still being fetched. */
  get loading(): boolean {
    return this.queue.length > 0 || this.running > 0;
  }

  /** The objects shown with where they stand. */
  get shown(): readonly { object: AreaObject; marked: boolean }[] {
    return this.instances;
  }

  private pump() {
    while (this.running < PARALLEL && this.queue.length > 0) {
      const model = this.queue.shift();
      if (model === undefined) {
        break;
      }
      this.running += 1;
      this.meshes.set(model, "loading");
      this.api
        .areaMesh(model)
        .then((mesh) => {
          this.meshes.set(model, mesh === null ? "missing" : this.upload(mesh));
        })
        .catch(() => this.meshes.set(model, "missing"))
        .finally(() => {
          this.running -= 1;
          if (!this.disposed) {
            this.pump();
            this.changed();
          }
        });
    }
  }

  private upload(mesh: LodMesh): Surface[] {
    const { gl } = this;
    if (this.disposed) {
      return [];
    }
    return mesh.shapes.flatMap((shape) => {
      const positions = new Float32Array(bytesOf(shape.positions));
      const indices = new Uint32Array(bytesOf(shape.indices));
      if (positions.length === 0 || indices.length === 0) {
        return [];
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
    gl.uniform1i(at("u_texture"), 0);
    const model = at("u_model");
    const tint = at("u_tint");
    const hasTexture = at("u_hasTexture");
    const alphaTest = at("u_alphaTest");
    gl.activeTexture(gl.TEXTURE0);
    for (const instance of this.instances) {
      const surfaces = this.meshes.get(instance.object.model);
      if (!Array.isArray(surfaces)) {
        continue;
      }
      gl.uniformMatrix4fv(model, false, instance.matrix);
      gl.uniform4f(tint, ...markColour, instance.marked ? 0.55 : 0);
      for (const surface of surfaces) {
        const texture = surface.texture === null ? null : this.texture(surface.texture);
        if (texture === "pending") {
          continue;
        }
        gl.bindTexture(gl.TEXTURE_2D, texture);
        gl.uniform1i(hasTexture, texture === null ? 0 : 1);
        gl.uniform1i(alphaTest, surface.alphaTest ? 1 : 0);
        gl.bindVertexArray(surface.vao);
        gl.drawElements(gl.TRIANGLES, surface.count, gl.UNSIGNED_INT, 0);
      }
    }
    gl.bindVertexArray(null);
  }

  /** Frees everything; the layer is not used again. */
  dispose() {
    this.disposed = true;
    const { gl } = this;
    for (const surfaces of this.meshes.values()) {
      if (Array.isArray(surfaces)) {
        for (const surface of surfaces) {
          gl.deleteVertexArray(surface.vao);
          for (const buffer of surface.buffers) {
            gl.deleteBuffer(buffer);
          }
        }
      }
    }
    this.meshes.clear();
    for (const texture of this.textures.values()) {
      if (texture !== "loading" && texture !== "missing") {
        gl.deleteTexture(texture);
      }
    }
    this.textures.clear();
    gl.deleteProgram(this.program);
  }
}
