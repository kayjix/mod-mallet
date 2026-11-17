// The game's own ground up close (ADR 0041): the heights of every cell
// around the point the camera looks at, 33 by 33 points each, so that
// docks, floors and paths stand on the land the game has rather than on
// the coarse level of detail.
//
// The ground is coloured with the land texture of the game's level of
// detail; where a block has none, with the land's own vertex colours.

import type { Api } from "./api";
import type { AreaLand, LandCell } from "./generated/area-land";
import type { LodMesh } from "./generated/lod-mesh";
import { type Mat4, program } from "./gl";
import { CELL } from "./map-places";

/** Points per side of a cell's grid. */
const GRID = 33;
/** Game units between two points of the grid. */
const STEP = CELL / (GRID - 1);
/** The largest side of a block's land texture. */
const TEXTURE_SIDE = 1024;

const VERTEX = `#version 300 es
in vec3 a_position;
in vec3 a_normal;
in vec3 a_colour;
uniform mat4 u_matrix;
uniform float u_exaggeration;
uniform vec3 u_uvU;
uniform vec3 u_uvV;
out vec2 v_uv;
out vec3 v_normal;
out vec3 v_colour;
void main() {
  vec3 at = vec3(a_position.xy / ${CELL}.0, 1.0);
  v_uv = vec2(dot(u_uvU, at), dot(u_uvV, at));
  v_normal = a_normal;
  v_colour = a_colour;
  gl_Position = u_matrix * vec4(at.xy, a_position.z / ${CELL}.0 * u_exaggeration, 1.0);
}`;

const FRAGMENT = `#version 300 es
precision highp float;
in vec2 v_uv;
in vec3 v_normal;
in vec3 v_colour;
uniform sampler2D u_texture;
uniform bool u_hasTexture;
out vec4 colour;
void main() {
  // Without a texture: a plain earth, tinted as the land's own colours say.
  vec3 base = u_hasTexture ? texture(u_texture, v_uv).rgb : vec3(0.42, 0.4, 0.33) * v_colour;
  vec3 light = normalize(vec3(-0.6, 0.7, 1.1));
  float facing = max(dot(normalize(v_normal), light), 0.0);
  // Lit as the level of detail's land is, so that the two meet unseen.
  colour = vec4(base * (0.8 + 0.3 * facing), 1.0);
}`;

/**
 * How a block of level of detail lays its land texture: the texture
 * coordinates as `u = a·x + b·y + c` and `v = d·x + e·y + f`, with `x`
 * and `y` in cells.
 */
interface Mapping {
  texture: string;
  u: [number, number, number];
  v: [number, number, number];
}

/** One cell's ground in graphics memory. */
interface Drawn {
  x: number;
  y: number;
  vao: WebGLVertexArrayObject;
  buffers: WebGLBuffer[];
}

/** A block of the level of detail by its finest level and corner. */
interface Block {
  level: number;
  x: number;
  y: number;
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
 * The affine map that best takes the points `xy` (in cells) to `values`,
 * by least squares; `null` if the points do not span a plane.
 */
export function fitAffine(
  xy: [number, number][],
  values: number[],
): [number, number, number] | null {
  // The normal equations, A·[a, b, c] = r.
  let [sxx, sxy, sx, syy, sy, n, rx, ry, r] = [0, 0, 0, 0, 0, 0, 0, 0, 0];
  xy.forEach(([x, y], index) => {
    const value = values[index] ?? 0;
    sxx += x * x;
    sxy += x * y;
    sx += x;
    syy += y * y;
    sy += y;
    n += 1;
    rx += x * value;
    ry += y * value;
    r += value;
  });
  const det = (m: number[][]) => {
    const [a = [], b = [], c = []] = m;
    const at = (row: number[], column: number) => row[column] ?? 0;
    return (
      at(a, 0) * (at(b, 1) * at(c, 2) - at(b, 2) * at(c, 1)) -
      at(a, 1) * (at(b, 0) * at(c, 2) - at(b, 2) * at(c, 0)) +
      at(a, 2) * (at(b, 0) * at(c, 1) - at(b, 1) * at(c, 0))
    );
  };
  const matrix = [
    [sxx, sxy, sx],
    [sxy, syy, sy],
    [sx, sy, n],
  ];
  const whole = det(matrix);
  if (!Number.isFinite(whole) || Math.abs(whole) < 1e-9) {
    return null;
  }
  const right = [rx, ry, r];
  const solved = [0, 1, 2].map(
    (column) =>
      det(
        matrix.map((row, index) =>
          row.map((cell, at) => (at === column ? (right[index] ?? 0) : cell)),
        ),
      ) / whole,
  );
  return [solved[0] ?? 0, solved[1] ?? 0, solved[2] ?? 0];
}

/** The mapping of a block's land texture, read from its mesh. */
export function mappingOf(mesh: LodMesh): Mapping | null {
  for (const shape of mesh.shapes) {
    if (shape.texture === null || shape.texture === undefined || shape.uvs === "") {
      continue;
    }
    const positions = new Float32Array(bytesOf(shape.positions));
    const uvs = new Float32Array(bytesOf(shape.uvs));
    const xy: [number, number][] = [];
    const us: number[] = [];
    const vs: number[] = [];
    for (let point = 0; point * 3 + 1 < positions.length && point * 2 + 1 < uvs.length; point++) {
      xy.push([(positions[point * 3] ?? 0) / CELL, (positions[point * 3 + 1] ?? 0) / CELL]);
      us.push(uvs[point * 2] ?? 0);
      vs.push(uvs[point * 2 + 1] ?? 0);
    }
    const u = fitAffine(xy, us);
    const v = fitAffine(xy, vs);
    if (u !== null && v !== null) {
      return { texture: shape.texture, u, v };
    }
  }
  return null;
}

/** The two triangles of every square of a cell's grid. */
function gridIndices(): Uint16Array {
  const indices = new Uint16Array((GRID - 1) * (GRID - 1) * 6);
  let at = 0;
  for (let row = 0; row + 1 < GRID; row++) {
    for (let column = 0; column + 1 < GRID; column++) {
      const a = row * GRID + column;
      const b = a + 1;
      const c = a + GRID;
      const d = c + 1;
      indices.set([a, b, d, a, d, c], at);
      at += 6;
    }
  }
  return indices;
}

export class LandLayer {
  private readonly program: WebGLProgram;
  private readonly indices: WebGLBuffer;
  private readonly count = (GRID - 1) * (GRID - 1) * 6;
  private drawn: Drawn[] = [];
  /** The cells (west, south, east, north edges) the ground covers. */
  private rect: [number, number, number, number] | null = null;
  private blocks: Promise<Block[]> | null = null;
  private readonly mappings = new Map<string, Mapping | "loading" | "missing">();
  private readonly textures = new Map<string, WebGLTexture | "loading" | "missing">();
  private asked = 0;
  private disposed = false;

  constructor(
    private readonly gl: WebGL2RenderingContext,
    private readonly api: Api,
    private readonly world: string,
    private readonly changed: () => void,
  ) {
    this.program = program(gl, VERTEX, FRAGMENT);
    this.indices = gl.createBuffer();
    gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, this.indices);
    gl.bufferData(gl.ELEMENT_ARRAY_BUFFER, gridIndices(), gl.STATIC_DRAW);
    gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, null);
  }

  /**
   * The cells the ground is drawn on, as west, south, east and north
   * edges in cells; the coarser land is to be left out there.
   */
  get covers(): [number, number, number, number] | null {
    return this.rect;
  }

  /** Fetches the ground within `radius` game units of `x`, `y`. */
  load(x: number, y: number, radius: number) {
    const asked = ++this.asked;
    this.api
      .areaLand(this.world, x, y, radius)
      .then((land) => {
        if (land !== null && asked === this.asked && !this.disposed) {
          this.show(land);
        }
      })
      .catch(() => {});
  }

  private show(land: AreaLand) {
    this.free();
    const heights = new Map<string, Float32Array>();
    for (const cell of land.cells) {
      const values = new Float32Array(bytesOf(cell.heights));
      if (values.length === GRID * GRID) {
        heights.set(`${cell.x}.${cell.y}`, values);
      }
    }
    // A height anywhere in the area, by point of the whole grid.
    const height = (gx: number, gy: number, fallback: number) => {
      const cx = Math.floor(gx / (GRID - 1));
      const cy = Math.floor(gy / (GRID - 1));
      const own = heights.get(`${cx}.${cy}`);
      return own?.[(gy - cy * (GRID - 1)) * GRID + (gx - cx * (GRID - 1))] ?? fallback;
    };
    for (const cell of land.cells) {
      const values = heights.get(`${cell.x}.${cell.y}`);
      if (values !== undefined) {
        this.drawn.push(this.upload(cell, values, height));
      }
    }
    const xs = this.drawn.map((cell) => cell.x);
    const ys = this.drawn.map((cell) => cell.y);
    this.rect =
      this.drawn.length === 0
        ? null
        : [Math.min(...xs), Math.min(...ys), Math.max(...xs) + 1, Math.max(...ys) + 1];
    for (const cell of this.drawn) {
      void this.mapping(cell.x, cell.y);
    }
    this.changed();
  }

  private upload(
    cell: LandCell,
    values: Float32Array,
    height: (gx: number, gy: number, fallback: number) => number,
  ): Drawn {
    const { gl } = this;
    const positions = new Float32Array(GRID * GRID * 3);
    const normals = new Float32Array(GRID * GRID * 3);
    for (let row = 0; row < GRID; row++) {
      for (let column = 0; column < GRID; column++) {
        const point = row * GRID + column;
        const z = values[point] ?? 0;
        positions.set([cell.x * CELL + column * STEP, cell.y * CELL + row * STEP, z], point * 3);
        // The slope across the point, reaching into the neighbouring
        // cells, so that their edges are lit alike.
        const gx = cell.x * (GRID - 1) + column;
        const gy = cell.y * (GRID - 1) + row;
        const dx = (height(gx + 1, gy, z) - height(gx - 1, gy, z)) / (2 * STEP);
        const dy = (height(gx, gy + 1, z) - height(gx, gy - 1, z)) / (2 * STEP);
        const length = Math.hypot(dx, dy, 1);
        normals.set([-dx / length, -dy / length, 1 / length], point * 3);
      }
    }
    const vao = gl.createVertexArray();
    gl.bindVertexArray(vao);
    const buffers: WebGLBuffer[] = [];
    const attribute = (name: string, data: ArrayBufferView, size: number, type: number) => {
      const location = gl.getAttribLocation(this.program, name);
      const buffer = gl.createBuffer();
      buffers.push(buffer);
      gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
      gl.bufferData(gl.ARRAY_BUFFER, data, gl.STATIC_DRAW);
      if (location >= 0) {
        gl.enableVertexAttribArray(location);
        gl.vertexAttribPointer(location, size, type, type === gl.UNSIGNED_BYTE, 0, 0);
      }
    };
    attribute("a_position", positions, 3, gl.FLOAT);
    attribute("a_normal", normals, 3, gl.FLOAT);
    const colours = cell.colours === "" ? null : new Uint8Array(bytesOf(cell.colours));
    if (colours !== null && colours.length === GRID * GRID * 3) {
      attribute("a_colour", colours, 3, gl.UNSIGNED_BYTE);
    } else {
      const location = gl.getAttribLocation(this.program, "a_colour");
      if (location >= 0) {
        gl.disableVertexAttribArray(location);
      }
    }
    gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, this.indices);
    gl.bindVertexArray(null);
    return { x: cell.x, y: cell.y, vao, buffers };
  }

  /** The finest block of level of detail with land around each cell. */
  private block(x: number, y: number): Promise<Block | null> {
    if (this.blocks === null) {
      this.blocks = this.api
        .lodTiles(this.world)
        .then((found) =>
          found.tiles
            .filter((tile) => tile.terrain)
            .map(({ level, x, y }) => ({ level, x, y }))
            .sort((a, b) => a.level - b.level),
        )
        .catch(() => []);
    }
    return this.blocks.then(
      (blocks) =>
        blocks.find(
          (block) =>
            x >= block.x && x < block.x + block.level && y >= block.y && y < block.y + block.level,
        ) ?? null,
    );
  }

  /** The texture mapping of the block a cell lies in, once it is known. */
  private async mapping(x: number, y: number): Promise<void> {
    const block = await this.block(x, y);
    if (block === null || this.disposed) {
      return;
    }
    const name = `${block.level}.${block.x}.${block.y}`;
    if (this.mappings.has(name)) {
      return;
    }
    this.mappings.set(name, "loading");
    try {
      const mesh = await this.api.lodMesh(this.world, block.level, block.x, block.y, false);
      const found = mesh === null ? null : mappingOf(mesh);
      this.mappings.set(name, found ?? "missing");
      if (found !== null) {
        this.texture(found.texture);
      }
    } catch {
      this.mappings.set(name, "missing");
    }
  }

  private texture(path: string): WebGLTexture | null {
    const known = this.textures.get(path);
    if (known !== undefined) {
      return known === "loading" || known === "missing" ? null : known;
    }
    this.textures.set(path, "loading");
    this.api
      .lodTexture(this.world, path, TEXTURE_SIDE)
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
    return null;
  }

  /** The mapping already known for the block a cell lies in. */
  private knownMapping(x: number, y: number): Mapping | null {
    for (const [name, mapping] of this.mappings) {
      if (typeof mapping === "string") {
        continue;
      }
      const [level = 0, bx = 0, by = 0] = name.split(".").map(Number);
      if (x >= bx && x < bx + level && y >= by && y < by + level) {
        return mapping;
      }
    }
    return null;
  }

  /** Draws the ground with `matrix` (cells, heights as drawn). */
  draw(matrix: Mat4, exaggeration: number) {
    if (this.disposed || this.drawn.length === 0) {
      return;
    }
    const { gl } = this;
    // biome-ignore lint/correctness/useHookAtTopLevel: WebGL's useProgram is no React hook.
    gl.useProgram(this.program);
    const at = (name: string) => gl.getUniformLocation(this.program, name);
    gl.uniformMatrix4fv(at("u_matrix"), false, matrix);
    gl.uniform1f(at("u_exaggeration"), exaggeration);
    gl.uniform1i(at("u_texture"), 0);
    // Without colours of its own, the land is untinted.
    const colour = gl.getAttribLocation(this.program, "a_colour");
    if (colour >= 0) {
      gl.vertexAttrib3f(colour, 1, 1, 1);
    }
    gl.activeTexture(gl.TEXTURE0);
    for (const cell of this.drawn) {
      const mapping = this.knownMapping(cell.x, cell.y);
      const texture = mapping === null ? null : this.texture(mapping.texture);
      gl.bindTexture(gl.TEXTURE_2D, texture);
      gl.uniform1i(at("u_hasTexture"), texture === null ? 0 : 1);
      gl.uniform3fv(at("u_uvU"), mapping?.u ?? [0, 0, 0]);
      gl.uniform3fv(at("u_uvV"), mapping?.v ?? [0, 0, 0]);
      gl.bindVertexArray(cell.vao);
      gl.drawElements(gl.TRIANGLES, this.count, gl.UNSIGNED_SHORT, 0);
    }
    gl.bindVertexArray(null);
  }

  private free() {
    const { gl } = this;
    for (const cell of this.drawn) {
      gl.deleteVertexArray(cell.vao);
      for (const buffer of cell.buffers) {
        gl.deleteBuffer(buffer);
      }
    }
    this.drawn = [];
    this.rect = null;
  }

  /** Frees everything; the layer is not used again. */
  dispose() {
    this.disposed = true;
    this.free();
    const { gl } = this;
    for (const texture of this.textures.values()) {
      if (texture !== "loading" && texture !== "missing") {
        gl.deleteTexture(texture);
      }
    }
    this.textures.clear();
    gl.deleteBuffer(this.indices);
    gl.deleteProgram(this.program);
  }
}
