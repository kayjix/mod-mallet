// The game's own level of detail on the relief (ADR 0037): blocks of land
// and the reduced objects on them, finer where the camera is close.
//
// Blocks are fetched on demand, a few at a time, and kept while they are
// used; until a finer block is there, the coarser one is drawn.

import type { Api } from "./api";
import type { LodMesh } from "./generated/lod-mesh";
import type { LodTile } from "./generated/lod-tiles";
import { type Mat4, program, type Vec3 } from "./gl";
import { CELL } from "./map-places";
import type { RealTopsView } from "./real-tops";

/** How close, in block sizes, the camera has to be for a block to split. */
const SPLIT = 2.4;
/** How many blocks are fetched at once. */
const PARALLEL = 4;
/** How many blocks are kept in graphics memory. */
const KEPT = 360;
/** The largest side of a block's own land texture, and of shared ones. */
const LAND_TEXTURE = 512;
const SHARED_TEXTURE = 2048;
/**
 * How much higher than the real objects below it a reduced object may
 * reach and still be left out, in game units (about 2.3 m): a reduced
 * roof or chimney is never quite where the real one is.
 */
const ABOVE_REAL = 160;
/**
 * How far apart the points are where real objects are looked for around
 * a reduced one, in game units; five by five of them, some 1.8 m across.
 */
const AROUND_REAL = 32;

const VERTEX = `#version 300 es
in vec3 a_position;
in vec3 a_normal;
in vec2 a_uv;
uniform mat4 u_matrix;
uniform float u_exaggeration;
out vec2 v_uv;
out vec3 v_normal;
out vec2 v_xy;
out vec3 v_game;
void main() {
  v_uv = a_uv;
  v_game = a_position;
  v_normal = a_normal;
  v_xy = a_position.xy / ${CELL}.0;
  gl_Position = u_matrix * vec4(v_xy, a_position.z / ${CELL}.0 * u_exaggeration, 1.0);
}`;

const FRAGMENT = `#version 300 es
precision highp float;
in vec2 v_uv;
in vec3 v_normal;
in vec2 v_xy;
in vec3 v_game;
uniform sampler2D u_texture;
uniform bool u_hasTexture;
uniform bool u_alphaTest;
uniform bool u_land;
uniform vec3 u_water;
uniform bool u_showCells;
uniform sampler2D u_cells;
uniform vec4 u_cellsRect;
uniform vec4 u_fine;
uniform bool u_hasTops;
uniform sampler2D u_tops;
uniform vec4 u_topsRect;
uniform vec2 u_topsRange;
out vec4 colour;
void main() {
  // Where the real objects are drawn up close, the reduced ones are left
  // out at their place and up to a little above them (ADR 0074).
  if (!u_land && u_hasTops) {
    vec2 at = (v_game.xy - u_topsRect.xy) / (u_topsRect.zw - u_topsRect.xy);
    if (at.x > 0.0 && at.y > 0.0 && at.x < 1.0 && at.y < 1.0) {
      vec2 spacing = vec2(${AROUND_REAL}.0) / (u_topsRect.zw - u_topsRect.xy);
      float nearest = 1.0;
      for (int i = -2; i <= 2; i++) {
        for (int j = -2; j <= 2; j++) {
          nearest = min(nearest, texture(u_tops, at + vec2(float(i), float(j)) * spacing).r);
        }
      }
      float top = u_topsRange.y - nearest * (u_topsRange.y - u_topsRange.x);
      if (nearest < 1.0 && v_game.z <= top + ${ABOVE_REAL}.0) {
        discard;
      }
    }
  }
  // Where the game's own ground is drawn up close (ADR 0041), its land
  // stays out of the way; its water stays.
  if (u_land && u_hasTexture && v_xy.x > u_fine.x && v_xy.y > u_fine.y &&
      v_xy.x < u_fine.z && v_xy.y < u_fine.w) {
    discard;
  }
  vec4 sampled = u_hasTexture ? texture(u_texture, v_uv) : vec4(u_water, 1.0);
  if (u_alphaTest && sampled.a < 0.5) {
    discard;
  }
  vec3 light = normalize(vec3(-0.6, 0.7, 1.1));
  float facing = max(dot(normalize(v_normal), light), 0.0);
  // The land's texture carries its detail; light only shapes it gently,
  // which also hides where one block meets the next.
  float lit = !u_hasTexture ? 1.0 : u_land ? 0.8 + 0.3 * facing : 0.5 + 0.62 * facing;
  vec3 shaded = sampled.rgb * lit;
  if (u_showCells) {
    vec2 at = vec2((v_xy.x - u_cellsRect.x) / u_cellsRect.z,
                   (u_cellsRect.y + u_cellsRect.w - v_xy.y) / u_cellsRect.w);
    vec4 cell = texture(u_cells, at);
    shaded = mix(shaded, cell.rgb, cell.a);
  }
  colour = vec4(shaded, 1.0);
}`;

/** One drawable surface of a block. */
interface Surface {
  vao: WebGLVertexArrayObject;
  buffers: WebGLBuffer[];
  count: number;
  texture: string | null;
  alphaTest: boolean;
}

/** What is known of one block: its land, its objects, when last used. */
interface Loaded {
  land: Surface[] | "loading" | "missing";
  objects: Surface[] | "loading" | "missing" | "none";
  used: number;
}

/** A shared texture: being fetched, there, or not to be had. */
type Shared = WebGLTexture | "loading" | "missing";

const key = (level: number, x: number, y: number) => `${level}.${x}.${y}`;

function bytesOf(base64: string): ArrayBuffer {
  const text = atob(base64);
  const bytes = new Uint8Array(text.length);
  for (let index = 0; index < text.length; index++) {
    bytes[index] = text.charCodeAt(index);
  }
  return bytes.buffer;
}

/** Smooth normals, one per point, from the triangles around it. */
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

/** Where the relief is drawn from, and how. */
export interface LodView {
  matrix: Mat4;
  /** The camera, in cells, height as drawn. */
  eye: Vec3;
  exaggeration: number;
  water: Vec3;
  showCells: boolean;
  cells: WebGLTexture;
  /** West, south, columns, rows of the cells texture. */
  cellsRect: [number, number, number, number];
  /** Where and how high the real objects are drawn up close, so that the
   * reduced ones are left out there. */
  realObjects: RealTopsView | null;
  /** Cells (west, south, east, north edges) where the game's own ground
   * is drawn, so that the land of the blocks is left out there. */
  fineLand?: [number, number, number, number] | null;
}

export class LodLayer {
  private readonly program: WebGLProgram;
  private readonly tiles = new Map<string, LodTile>();
  private readonly roots: LodTile[];
  /** The corner the blocks of every level are counted from. */
  private readonly origin: [number, number];
  private readonly loaded = new Map<string, Loaded>();
  private readonly textures = new Map<string, Shared>();
  private readonly queue: { key: string; tile: LodTile; distance: number }[] = [];
  private running = 0;
  private frame = 0;
  private disposed = false;
  /** The blocks drawn last, and a name for that set. */
  drawnTiles: LodTile[] = [];
  drawnKey = "";

  constructor(
    private readonly gl: WebGL2RenderingContext,
    private readonly api: Api,
    private readonly world: string,
    tiles: LodTile[],
    private readonly changed: () => void,
  ) {
    this.program = program(gl, VERTEX, FRAGMENT);
    for (const tile of tiles) {
      this.tiles.set(key(tile.level, tile.x, tile.y), tile);
    }
    const coarsest = Math.max(...tiles.map((tile) => tile.level));
    this.roots = tiles.filter((tile) => tile.level === coarsest && tile.terrain);
    this.origin = [
      Math.min(...this.roots.map((tile) => tile.x)),
      Math.min(...this.roots.map((tile) => tile.y)),
    ];
  }

  /** Whether there is any level of detail to draw. */
  get usable(): boolean {
    return this.roots.length > 0;
  }

  /** The blocks to show from `eye`: finer ones where it is close. */
  private wanted(eye: Vec3): LodTile[] {
    const found: LodTile[] = [];
    const visit = (tile: LodTile) => {
      const half = tile.level / 2;
      const distance = Math.hypot(
        tile.x + half - eye[0],
        tile.y + half - eye[1],
        Math.max(0, eye[2]),
      );
      const children =
        tile.level > 4
          ? [0, 1].flatMap((i) =>
              [0, 1].map((j) => this.tiles.get(key(half, tile.x + i * half, tile.y + j * half))),
            )
          : [];
      const split =
        distance < tile.level * SPLIT &&
        children.length === 4 &&
        children.every((child) => child?.terrain === true);
      if (split) {
        for (const child of children) {
          if (child !== undefined) {
            visit(child);
          }
        }
      } else {
        found.push(tile);
      }
    };
    for (const root of this.roots) {
      visit(root);
    }
    return found;
  }

  /** Draws the blocks for `view`, fetching what is missing. */
  draw(view: LodView) {
    if (this.disposed) {
      return;
    }
    this.frame += 1;
    const wanted = this.wanted(view.eye);
    // What is drawn: each wanted block, or its nearest coarser block that
    // is there, but never a block and one inside it.
    const drawn = new Map<string, LodTile>();
    for (const tile of wanted) {
      this.request(tile, view.eye);
      let current: LodTile | undefined = tile;
      while (current !== undefined && !this.hasLand(current)) {
        current = this.parent(current);
      }
      if (current !== undefined) {
        drawn.set(key(current.level, current.x, current.y), current);
      }
    }
    for (const [name, tile] of [...drawn]) {
      for (let up = this.parent(tile); up !== undefined; up = this.parent(up)) {
        if (drawn.has(key(up.level, up.x, up.y))) {
          drawn.delete(name);
          break;
        }
      }
    }
    this.pump();

    this.drawnTiles = [...drawn.values()];
    this.drawnKey = [...drawn.keys()].sort().join(" ");

    const { gl } = this;
    // biome-ignore lint/correctness/useHookAtTopLevel: WebGL's useProgram is no React hook.
    gl.useProgram(this.program);
    const at = (name: string) => gl.getUniformLocation(this.program, name);
    gl.uniformMatrix4fv(at("u_matrix"), false, view.matrix);
    gl.uniform1f(at("u_exaggeration"), view.exaggeration);
    gl.uniform3fv(at("u_water"), view.water);
    gl.uniform1i(at("u_showCells"), view.showCells ? 1 : 0);
    gl.uniform4f(at("u_cellsRect"), ...view.cellsRect);
    gl.uniform4f(at("u_fine"), ...(view.fineLand ?? [0, 0, 0, 0]));
    const real = view.realObjects;
    gl.uniform1i(at("u_hasTops"), real === null ? 0 : 1);
    gl.uniform1i(at("u_tops"), 3);
    if (real !== null) {
      gl.activeTexture(gl.TEXTURE3);
      gl.bindTexture(gl.TEXTURE_2D, real.texture);
      gl.uniform4f(at("u_topsRect"), ...real.rect);
      gl.uniform2f(at("u_topsRange"), ...real.range);
    }
    gl.activeTexture(gl.TEXTURE1);
    gl.bindTexture(gl.TEXTURE_2D, view.cells);
    gl.uniform1i(at("u_cells"), 1);
    gl.uniform1i(at("u_texture"), 0);
    gl.enable(gl.POLYGON_OFFSET_FILL);
    gl.polygonOffset(2, 8);
    for (const tile of drawn.values()) {
      const loaded = this.loaded.get(key(tile.level, tile.x, tile.y));
      if (loaded === undefined) {
        continue;
      }
      loaded.used = this.frame;
      if (Array.isArray(loaded.land)) {
        this.drawSurfaces(loaded.land, at, true);
      }
      if (Array.isArray(loaded.objects)) {
        this.drawSurfaces(loaded.objects, at, false);
      }
    }
    gl.bindVertexArray(null);
    gl.disable(gl.POLYGON_OFFSET_FILL);
    this.evict(drawn);
  }

  private drawSurfaces(
    surfaces: Surface[],
    at: (name: string) => WebGLUniformLocation | null,
    land: boolean,
  ) {
    const { gl } = this;
    gl.uniform1i(at("u_land"), land ? 1 : 0);
    for (const surface of surfaces) {
      const texture = surface.texture === null ? null : this.texture(surface.texture, surface);
      if (surface.texture !== null && texture === null) {
        // Its texture is still on its way.
        continue;
      }
      gl.activeTexture(gl.TEXTURE0);
      gl.bindTexture(gl.TEXTURE_2D, texture);
      gl.uniform1i(at("u_hasTexture"), texture === null ? 0 : 1);
      gl.uniform1i(at("u_alphaTest"), surface.alphaTest ? 1 : 0);
      gl.bindVertexArray(surface.vao);
      gl.drawElements(gl.TRIANGLES, surface.count, gl.UNSIGNED_INT, 0);
    }
  }

  private hasLand(tile: LodTile): boolean {
    const loaded = this.loaded.get(key(tile.level, tile.x, tile.y));
    return loaded !== undefined && Array.isArray(loaded.land);
  }

  private parent(tile: LodTile): LodTile | undefined {
    const level = tile.level * 2;
    const [west, south] = this.origin;
    const x = Math.floor((tile.x - west) / level) * level + west;
    const y = Math.floor((tile.y - south) / level) * level + south;
    return this.tiles.get(key(level, x, y));
  }

  /** Puts a block in line to be fetched, closest first. */
  private request(tile: LodTile, eye: Vec3) {
    const name = key(tile.level, tile.x, tile.y);
    if (this.loaded.has(name) || this.queue.some((waiting) => waiting.key === name)) {
      return;
    }
    const distance = Math.hypot(tile.x + tile.level / 2 - eye[0], tile.y + tile.level / 2 - eye[1]);
    this.queue.push({ key: name, tile, distance });
  }

  /** Fetches the next blocks, a few at a time. */
  private pump() {
    this.queue.sort((a, b) => a.distance - b.distance);
    while (this.running < PARALLEL && this.queue.length > 0) {
      const next = this.queue.shift();
      if (next === undefined) {
        break;
      }
      this.running += 1;
      const entry: Loaded = {
        land: "loading",
        objects: next.tile.objects ? "loading" : "none",
        used: this.frame,
      };
      this.loaded.set(next.key, entry);
      const { level, x, y } = next.tile;
      const land = this.api
        .lodMesh(this.world, level, x, y, false)
        .then((mesh) => {
          entry.land = mesh === null ? "missing" : this.upload(mesh);
        })
        .catch(() => {
          entry.land = "missing";
        });
      const objects = next.tile.objects
        ? this.api
            .lodMesh(this.world, level, x, y, true)
            .then((mesh) => {
              entry.objects = mesh === null ? "missing" : this.upload(mesh);
            })
            .catch(() => {
              entry.objects = "missing";
            })
        : Promise.resolve();
      Promise.all([land, objects]).finally(() => {
        this.running -= 1;
        if (!this.disposed) {
          this.changed();
        }
      });
    }
  }

  /** Puts a block's surfaces into graphics memory. */
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

  /** A texture by its path, fetched the first time it is asked for. */
  private texture(path: string, surface: Surface): WebGLTexture | null {
    const known = this.textures.get(path);
    if (known === "missing") {
      // Drawn plain rather than not at all.
      surface.texture = null;
      return null;
    }
    if (known !== undefined && known !== "loading") {
      return known;
    }
    if (known === undefined) {
      this.textures.set(path, "loading");
      // A block's own land texture is used by that block alone; the atlas
      // of objects by all.
      const side =
        path.includes("/objects/") || !path.includes("/terrain/") ? SHARED_TEXTURE : LAND_TEXTURE;
      this.api
        .lodTexture(this.world, path, side)
        .then((url) => {
          if (url === null || this.disposed) {
            this.textures.set(path, "missing");
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
            gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
            gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
            this.textures.set(path, made);
            this.changed();
          };
          image.onerror = () => this.textures.set(path, "missing");
          image.src = url;
        })
        .catch(() => this.textures.set(path, "missing"));
    }
    return null;
  }

  /** Frees the blocks used longest ago when too many are kept. */
  private evict(drawn: Map<string, LodTile>) {
    if (this.loaded.size <= KEPT) {
      return;
    }
    const old = [...this.loaded.entries()]
      .filter(
        ([name, entry]) =>
          !drawn.has(name) && entry.land !== "loading" && entry.objects !== "loading",
      )
      .sort((a, b) => a[1].used - b[1].used)
      .slice(0, this.loaded.size - KEPT);
    for (const [name, entry] of old) {
      this.free(entry);
      this.loaded.delete(name);
    }
  }

  private free(entry: Loaded) {
    const { gl } = this;
    for (const surfaces of [entry.land, entry.objects]) {
      if (Array.isArray(surfaces)) {
        for (const surface of surfaces) {
          gl.deleteVertexArray(surface.vao);
          for (const buffer of surface.buffers) {
            gl.deleteBuffer(buffer);
          }
        }
      }
    }
  }

  /** Frees everything; the layer is not used again. */
  dispose() {
    this.disposed = true;
    for (const entry of this.loaded.values()) {
      this.free(entry);
    }
    this.loaded.clear();
    for (const texture of this.textures.values()) {
      if (texture !== "loading" && texture !== "missing") {
        this.gl.deleteTexture(texture);
      }
    }
    this.textures.clear();
    this.gl.deleteProgram(this.program);
  }
}
