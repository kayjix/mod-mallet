// The meshes of a place up close in one set of buffers (ADR 0075): every
// model's points and triangles are added to the same buffers, so that the
// objects of many models are drawn with one call per material, and the
// tests that leave out what the camera does not see.

import type { Mat4 } from "./gl";

/** Where the attributes of a pooled mesh are, the same in every program drawing it. */
export const POOL_LOCATIONS: [string, number][] = [
  ["a_position", 0],
  ["a_normal", 1],
  ["a_uv", 2],
];

/** How many points and indices a pool makes room for at first. */
const FIRST_POINTS = 1 << 16;
const FIRST_INDICES = 1 << 17;

/** One attribute's buffer: its location and how many floats a point has. */
interface Attribute {
  buffer: WebGLBuffer;
  location: number;
  size: number;
}

/**
 * Points (positions, normals, texture coordinates) and triangles of many
 * meshes in one vertex array, growing as meshes are added.
 */
export class MeshPool {
  readonly vao: WebGLVertexArrayObject;
  private readonly attributes: Attribute[];
  private elements: WebGLBuffer;
  private points = 0;
  private indices = 0;
  private pointRoom = FIRST_POINTS;
  private indexRoom = FIRST_INDICES;
  private readonly gl: WebGL2RenderingContext;

  constructor(gl: WebGL2RenderingContext) {
    this.gl = gl;
    this.vao = gl.createVertexArray();
    gl.bindVertexArray(this.vao);
    this.attributes = POOL_LOCATIONS.map(([, location], index) => {
      const size = index === 2 ? 2 : 3;
      const buffer = gl.createBuffer();
      gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
      gl.bufferData(gl.ARRAY_BUFFER, this.pointRoom * size * 4, gl.STATIC_DRAW);
      gl.enableVertexAttribArray(location);
      gl.vertexAttribPointer(location, size, gl.FLOAT, false, 0, 0);
      return { buffer, location, size };
    });
    this.elements = gl.createBuffer();
    gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, this.elements);
    gl.bufferData(gl.ELEMENT_ARRAY_BUFFER, this.indexRoom * 4, gl.STATIC_DRAW);
    gl.bindVertexArray(null);
  }

  /**
   * Adds a mesh: its points' positions, normals and texture coordinates,
   * and its triangles by the indices of its own points. Returns where its
   * indices start in the pool, counted in indices.
   */
  add(
    positions: Float32Array,
    normals: Float32Array,
    uvs: Float32Array,
    triangles: Uint32Array,
  ): number {
    const { gl } = this;
    const count = positions.length / 3;
    this.makeRoom(this.points + count, this.indices + triangles.length);
    const shifted = new Uint32Array(triangles.length);
    for (let index = 0; index < triangles.length; index++) {
      shifted[index] = (triangles[index] ?? 0) + this.points;
    }
    const data = [positions, normals, uvs];
    this.attributes.forEach((attribute, index) => {
      gl.bindBuffer(gl.ARRAY_BUFFER, attribute.buffer);
      gl.bufferSubData(
        gl.ARRAY_BUFFER,
        this.points * attribute.size * 4,
        data[index] ?? new Float32Array(),
      );
    });
    gl.bindVertexArray(this.vao);
    gl.bufferSubData(gl.ELEMENT_ARRAY_BUFFER, this.indices * 4, shifted);
    gl.bindVertexArray(null);
    const first = this.indices;
    this.points += count;
    this.indices += triangles.length;
    return first;
  }

  /** Grows the buffers, copying what they hold, until `points` and `indices` fit. */
  private makeRoom(points: number, indices: number) {
    const { gl } = this;
    if (points > this.pointRoom) {
      let room = this.pointRoom;
      while (room < points) {
        room *= 2;
      }
      gl.bindVertexArray(this.vao);
      for (const attribute of this.attributes) {
        const grown = gl.createBuffer();
        gl.bindBuffer(gl.ARRAY_BUFFER, grown);
        gl.bufferData(gl.ARRAY_BUFFER, room * attribute.size * 4, gl.STATIC_DRAW);
        gl.bindBuffer(gl.COPY_READ_BUFFER, attribute.buffer);
        gl.copyBufferSubData(
          gl.COPY_READ_BUFFER,
          gl.ARRAY_BUFFER,
          0,
          0,
          this.points * attribute.size * 4,
        );
        gl.vertexAttribPointer(attribute.location, attribute.size, gl.FLOAT, false, 0, 0);
        gl.bindBuffer(gl.COPY_READ_BUFFER, null);
        gl.deleteBuffer(attribute.buffer);
        attribute.buffer = grown;
      }
      gl.bindVertexArray(null);
      this.pointRoom = room;
    }
    if (indices > this.indexRoom) {
      let room = this.indexRoom;
      while (room < indices) {
        room *= 2;
      }
      gl.bindVertexArray(this.vao);
      const grown = gl.createBuffer();
      gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, grown);
      gl.bufferData(gl.ELEMENT_ARRAY_BUFFER, room * 4, gl.STATIC_DRAW);
      gl.bindBuffer(gl.COPY_READ_BUFFER, this.elements);
      gl.copyBufferSubData(gl.COPY_READ_BUFFER, gl.ELEMENT_ARRAY_BUFFER, 0, 0, this.indices * 4);
      gl.bindBuffer(gl.COPY_READ_BUFFER, null);
      gl.bindVertexArray(null);
      gl.deleteBuffer(this.elements);
      this.elements = grown;
      this.indexRoom = room;
    }
  }

  /** Frees the buffers; the pool is not used again. */
  dispose() {
    const { gl } = this;
    gl.deleteVertexArray(this.vao);
    for (const attribute of this.attributes) {
      gl.deleteBuffer(attribute.buffer);
    }
    gl.deleteBuffer(this.elements);
  }
}

/**
 * The normals of a mesh's points: the sum of the faces' normals around
 * each point, of unit length.
 */
export function normalsOf(positions: Float32Array, indices: Uint32Array): Float32Array {
  const normals = new Float32Array(positions.length);
  const points = positions.length / 3;
  for (let index = 0; index + 2 < indices.length; index += 3) {
    const a = indices[index] ?? 0;
    const b = indices[index + 1] ?? 0;
    const c = indices[index + 2] ?? 0;
    if (a >= points || b >= points || c >= points) {
      continue;
    }
    const ax = positions[a * 3] ?? 0;
    const ay = positions[a * 3 + 1] ?? 0;
    const az = positions[a * 3 + 2] ?? 0;
    const ux = (positions[b * 3] ?? 0) - ax;
    const uy = (positions[b * 3 + 1] ?? 0) - ay;
    const uz = (positions[b * 3 + 2] ?? 0) - az;
    const vx = (positions[c * 3] ?? 0) - ax;
    const vy = (positions[c * 3 + 1] ?? 0) - ay;
    const vz = (positions[c * 3 + 2] ?? 0) - az;
    const nx = uy * vz - uz * vy;
    const ny = uz * vx - ux * vz;
    const nz = ux * vy - uy * vx;
    for (const point of [a, b, c]) {
      normals[point * 3] = (normals[point * 3] ?? 0) + nx;
      normals[point * 3 + 1] = (normals[point * 3 + 1] ?? 0) + ny;
      normals[point * 3 + 2] = (normals[point * 3 + 2] ?? 0) + nz;
    }
  }
  for (let point = 0; point < normals.length; point += 3) {
    const x = normals[point] ?? 0;
    const y = normals[point + 1] ?? 0;
    const z = normals[point + 2] ?? 1;
    const length = Math.hypot(x, y, z) || 1;
    normals[point] = x / length;
    normals[point + 1] = y / length;
    normals[point + 2] = z / length;
  }
  return normals;
}

/**
 * What a camera's matrix sees: its six planes, to test spheres against,
 * and how many pixels a unit across at distance one takes.
 */
export class Sight {
  private readonly planes: Float32Array;
  private readonly row3: [number, number, number, number];
  private readonly across: number;

  constructor(matrix: Mat4, pixels: number) {
    const row = (index: number): [number, number, number, number] => [
      matrix[index] ?? 0,
      matrix[4 + index] ?? 0,
      matrix[8 + index] ?? 0,
      matrix[12 + index] ?? 0,
    ];
    const [r0, r1, r2, r3] = [row(0), row(1), row(2), row(3)];
    this.row3 = r3;
    this.planes = new Float32Array(24);
    const sides = [
      [r3, r0, 1],
      [r3, r0, -1],
      [r3, r1, 1],
      [r3, r1, -1],
      [r3, r2, 1],
      [r3, r2, -1],
    ] as const;
    sides.forEach(([w, axis, sign], index) => {
      const plane = [0, 1, 2, 3].map((at) => (w[at] ?? 0) + sign * (axis[at] ?? 0));
      const length = Math.hypot(plane[0] ?? 0, plane[1] ?? 0, plane[2] ?? 0) || 1;
      for (let at = 0; at < 4; at++) {
        this.planes[index * 4 + at] = (plane[at] ?? 0) / length;
      }
    });
    this.across = Math.hypot(r0[0], r0[1], r0[2]) * (pixels / 2);
  }

  /**
   * Whether a sphere at `x`, `y`, `z` with `radius` (in the matrix's
   * units) is in sight and, where `least` is given, at least that many
   * pixels across.
   */
  sees(x: number, y: number, z: number, radius: number, least = 0): boolean {
    const planes = this.planes;
    for (let index = 0; index < 24; index += 4) {
      const distance =
        (planes[index] ?? 0) * x +
        (planes[index + 1] ?? 0) * y +
        (planes[index + 2] ?? 0) * z +
        (planes[index + 3] ?? 0);
      if (distance < -radius) {
        return false;
      }
    }
    if (least > 0) {
      const [a, b, c, d] = this.row3;
      const depth = a * x + b * y + c * z + d;
      if (depth > radius && ((radius * this.across) / depth) * 2 < least) {
        return false;
      }
    }
    return true;
  }
}
