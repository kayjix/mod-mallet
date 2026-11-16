// Small helpers for drawing with WebGL 2 without a library: column-major
// 4×4 matrices as WebGL expects them, and compiling shaders.

/** A 4×4 matrix in column-major order. */
export type Mat4 = Float32Array;

/** A point or direction in three dimensions. */
export type Vec3 = [number, number, number];

/** A perspective projection: `fovy` in radians, `aspect` width by height. */
export function perspective(fovy: number, aspect: number, near: number, far: number): Mat4 {
  const f = 1 / Math.tan(fovy / 2);
  const range = 1 / (near - far);
  // biome-ignore format: a matrix reads best in rows of four.
  return new Float32Array([
    f / aspect, 0, 0, 0,
    0, f, 0, 0,
    0, 0, (near + far) * range, -1,
    0, 0, 2 * near * far * range, 0,
  ]);
}

const subtract = (a: Vec3, b: Vec3): Vec3 => [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
const cross = (a: Vec3, b: Vec3): Vec3 => [
  a[1] * b[2] - a[2] * b[1],
  a[2] * b[0] - a[0] * b[2],
  a[0] * b[1] - a[1] * b[0],
];
const dot = (a: Vec3, b: Vec3) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];

/** The vector scaled to length one; the zero vector stays as it is. */
export function normalize(a: Vec3): Vec3 {
  const length = Math.hypot(a[0], a[1], a[2]);
  return length === 0 ? a : [a[0] / length, a[1] / length, a[2] / length];
}

/** A camera at `eye` looking at `target`, with `up` as the way up. */
export function lookAt(eye: Vec3, target: Vec3, up: Vec3): Mat4 {
  const z = normalize(subtract(eye, target));
  const x = normalize(cross(up, z));
  const y = cross(z, x);
  // biome-ignore format: a matrix reads best in rows of four.
  return new Float32Array([
    x[0], y[0], z[0], 0,
    x[1], y[1], z[1], 0,
    x[2], y[2], z[2], 0,
    -dot(x, eye), -dot(y, eye), -dot(z, eye), 1,
  ]);
}

/** The product `a · b`: first `b`, then `a`. */
export function multiply(a: Mat4, b: Mat4): Mat4 {
  const out = new Float32Array(16);
  for (let column = 0; column < 4; column++) {
    for (let row = 0; row < 4; row++) {
      let sum = 0;
      for (let k = 0; k < 4; k++) {
        sum += (a[k * 4 + row] ?? 0) * (b[column * 4 + k] ?? 0);
      }
      out[column * 4 + row] = sum;
    }
  }
  return out;
}

/**
 * Where a point lands on a canvas of `width` by `height` pixels, and how
 * deep it lies (0 near, 1 far). `null` if it lies behind the camera.
 */
export function project(
  matrix: Mat4,
  point: Vec3,
  width: number,
  height: number,
): { x: number; y: number; depth: number } | null {
  const [px, py, pz] = point;
  const m = (index: number) => matrix[index] ?? 0;
  const x = m(0) * px + m(4) * py + m(8) * pz + m(12);
  const y = m(1) * px + m(5) * py + m(9) * pz + m(13);
  const z = m(2) * px + m(6) * py + m(10) * pz + m(14);
  const w = m(3) * px + m(7) * py + m(11) * pz + m(15);
  if (w <= 1e-6) {
    return null;
  }
  return {
    x: ((x / w + 1) / 2) * width,
    y: ((1 - y / w) / 2) * height,
    depth: (z / w + 1) / 2,
  };
}

/** Compiles and links a program; throws with the driver's message if not. */
export function program(gl: WebGL2RenderingContext, vertex: string, fragment: string) {
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
  const linked = gl.createProgram();
  const parts = [shader(gl.VERTEX_SHADER, vertex), shader(gl.FRAGMENT_SHADER, fragment)];
  for (const part of parts) {
    gl.attachShader(linked, part);
  }
  gl.linkProgram(linked);
  for (const part of parts) {
    gl.deleteShader(part);
  }
  if (!gl.getProgramParameter(linked, gl.LINK_STATUS)) {
    throw new Error(gl.getProgramInfoLog(linked) ?? "program");
  }
  return linked;
}

/** A colour of the stylesheet, `#rrggbb`, as three numbers from 0 to 1. */
export function colour(value: string): Vec3 {
  const hex = value.trim().replace("#", "");
  const number = Number.parseInt(hex.length === 3 ? hex.replace(/./g, "$&$&") : hex, 16);
  if (Number.isNaN(number)) {
    return [0.5, 0.5, 0.5];
  }
  return [((number >> 16) & 255) / 255, ((number >> 8) & 255) / 255, (number & 255) / 255];
}
