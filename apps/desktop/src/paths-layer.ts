// Paths in a 3D view, drawn see-through on top of what is there, so that
// paths under the land or a deck show as well: the repair of paths (ADR
// 0058), the pieces taken out in red and the new ones in green, and the
// paths mode (ADR 0071), groups of triangles in colours that also differ
// by pattern, with marks for doors, links and action places.

import { type Mat4, program } from "./gl";
import { CELL } from "./map-places";

/** Triangles of paths, three corners each, in game units. */
export type PathTriangles = number[][][];

/** How the triangles of a group are filled: plain, striped, dotted or cross-hatched; or only outlined. */
export type PathPattern = "solid" | "stripes" | "dots" | "hatch" | "outline";

/** One group of triangles in one style (ADR 0071). */
export interface PathGroup {
  /** The corners, nine numbers (three corners) per triangle, in game units. */
  corners: Float32Array;
  /** Red, green, blue and opacity, from 0 to 1. */
  colour: [number, number, number, number];
  pattern: PathPattern;
  /** Whether the edges of the triangles are drawn as well. */
  edges?: boolean;
}

/** The shape of a mark. */
export type MarkShape = "circle" | "square" | "diamond" | "triangle" | "ring" | "dot";

/** Marks of one kind, as points that keep their size on the screen (ADR 0071). */
export interface PathMarks {
  /** Three numbers per mark, in game units. */
  points: Float32Array;
  colour: [number, number, number, number];
  shape: MarkShape;
  /** In CSS pixels. */
  size: number;
}

/** What to draw. */
export interface PathsOverlay {
  /** Taken out by a repair, drawn on top. */
  removed: PathTriangles;
  /** Laid by a repair, drawn on top. */
  added: PathTriangles;
  /** The paths mode's groups, drawn first (ADR 0071). */
  groups?: PathGroup[];
  /** The paths mode's marks, drawn last (ADR 0071). */
  marks?: PathMarks[];
}

/** Red and green that stay apart for most eyes (Okabe–Ito vermilion and bluish green). */
const REMOVED: [number, number, number, number] = [0.84, 0.37, 0.0, 0.55];
const ADDED: [number, number, number, number] = [0.0, 0.62, 0.45, 0.6];

/** How far above the triangles they are drawn, in game units. */
const LIFT = 6;

/** The length of one stripe and gap of a pattern, in game units. */
const PERIOD = 96;

const PATTERNS: Record<PathPattern, number> = {
  solid: 0,
  stripes: 1,
  dots: 2,
  hatch: 3,
  outline: 4,
};

const SHAPES: Record<MarkShape, number> = {
  circle: 0,
  square: 1,
  diamond: 2,
  triangle: 3,
  ring: 4,
  dot: 5,
};

const VERTEX = `#version 300 es
in vec3 a_position;
uniform mat4 u_matrix;
uniform float u_exaggeration;
uniform float u_size;
out vec2 v_world;
void main() {
  vec3 at = a_position;
  v_world = at.xy;
  gl_Position = u_matrix * vec4(at.xy / ${CELL}.0, (at.z + ${LIFT}.0) / ${CELL}.0 * u_exaggeration, 1.0);
  gl_PointSize = u_size;
}`;

const FRAGMENT = `#version 300 es
precision mediump float;
uniform vec4 u_colour;
uniform int u_pattern;
in vec2 v_world;
out vec4 colour;
void main() {
  vec2 p = v_world / ${PERIOD}.0;
  float strength = 1.0;
  if (u_pattern == 1) {
    strength = fract(p.x + p.y) < 0.5 ? 1.0 : 0.25;
  } else if (u_pattern == 2) {
    strength = length(fract(p) - 0.5) < 0.28 ? 1.0 : 0.25;
  } else if (u_pattern == 3) {
    strength = (fract(p.x + p.y) < 0.35 || fract(p.x - p.y) < 0.35) ? 1.0 : 0.2;
  }
  colour = vec4(u_colour.rgb, u_colour.a * strength);
}`;

const MARK_FRAGMENT = `#version 300 es
precision mediump float;
uniform vec4 u_colour;
uniform int u_shape;
out vec4 colour;
void main() {
  vec2 p = gl_PointCoord * 2.0 - 1.0;
  float d;
  if (u_shape == 1) {
    d = max(abs(p.x), abs(p.y));
  } else if (u_shape == 2) {
    d = abs(p.x) + abs(p.y);
  } else if (u_shape == 3) {
    // Pointing up (the point's y runs downwards), its base along the bottom.
    d = max(p.y / 0.8, abs(p.x) * 1.8 / max(p.y + 1.0, 0.001));
  } else if (u_shape == 5) {
    d = length(p) * 1.6;
  } else {
    d = length(p);
  }
  if (d > 1.0) {
    discard;
  }
  vec4 dark = vec4(0.08, 0.08, 0.08, 0.9);
  if (u_shape == 4) {
    // A thick ring with dark edges, which stands out on any colour.
    if (d < 0.5) {
      discard;
    }
    colour = d > 0.88 || d < 0.62 ? dark : u_colour;
    return;
  }
  colour = d > 0.72 ? dark : u_colour;
}`;

/** Draws a {@link PathsOverlay} with the matrices of the views. */
export class PathsLayer {
  private readonly gl: WebGL2RenderingContext;
  private readonly shader: WebGLProgram;
  private readonly markShader: WebGLProgram;
  private readonly buffer: WebGLBuffer;
  private readonly vao: WebGLVertexArrayObject;
  private readonly markBuffer: WebGLBuffer;
  private readonly markVao: WebGLVertexArrayObject;
  private removedCount = 0;
  private addedCount = 0;
  /** Where each group's triangles start and how many corners they have, then its edges. */
  private groups: {
    group: PathGroup;
    start: number;
    count: number;
    edgeStart: number;
    edgeCount: number;
  }[] = [];
  private repairStart = 0;
  private marks: { marks: PathMarks; start: number; count: number }[] = [];

  constructor(gl: WebGL2RenderingContext) {
    this.gl = gl;
    this.shader = program(gl, VERTEX, FRAGMENT);
    this.markShader = program(gl, VERTEX, MARK_FRAGMENT);
    const buffer = gl.createBuffer();
    const vao = gl.createVertexArray();
    const markBuffer = gl.createBuffer();
    const markVao = gl.createVertexArray();
    if (buffer === null || vao === null || markBuffer === null || markVao === null) {
      throw new Error("no buffer");
    }
    this.buffer = buffer;
    this.vao = vao;
    this.markBuffer = markBuffer;
    this.markVao = markVao;
    for (const [shader, into, data] of [
      [this.shader, vao, buffer],
      [this.markShader, markVao, markBuffer],
    ] as const) {
      gl.bindVertexArray(into);
      gl.bindBuffer(gl.ARRAY_BUFFER, data);
      const at = gl.getAttribLocation(shader, "a_position");
      gl.enableVertexAttribArray(at);
      gl.vertexAttribPointer(at, 3, gl.FLOAT, false, 0, 0);
    }
    gl.bindVertexArray(null);
  }

  /** Shows `overlay`, or nothing. */
  set(overlay: PathsOverlay | null) {
    const removed = overlay?.removed ?? [];
    const added = overlay?.added ?? [];
    const groups = overlay?.groups ?? [];
    const marks = overlay?.marks ?? [];
    // The groups' triangles, then their edges, then the repair's triangles.
    let floats = (removed.length + added.length) * 9;
    for (const group of groups) {
      floats += group.corners.length * (group.edges || group.pattern === "outline" ? 3 : 1);
    }
    const corners = new Float32Array(floats);
    let at = 0;
    this.groups = [];
    for (const group of groups) {
      const start = at / 3;
      if (group.pattern !== "outline") {
        corners.set(group.corners, at);
        at += group.corners.length;
      }
      const count = at / 3 - start;
      const edgeStart = at / 3;
      if (group.edges || group.pattern === "outline") {
        const c = group.corners;
        for (let triangle = 0; triangle + 8 < c.length; triangle += 9) {
          for (const [from, to] of [
            [0, 3],
            [3, 6],
            [6, 0],
          ] as const) {
            corners[at++] = c[triangle + from] ?? 0;
            corners[at++] = c[triangle + from + 1] ?? 0;
            corners[at++] = c[triangle + from + 2] ?? 0;
            corners[at++] = c[triangle + to] ?? 0;
            corners[at++] = c[triangle + to + 1] ?? 0;
            corners[at++] = c[triangle + to + 2] ?? 0;
          }
        }
      }
      this.groups.push({ group, start, count, edgeStart, edgeCount: at / 3 - edgeStart });
    }
    this.repairStart = at / 3;
    for (const triangle of [...removed, ...added]) {
      for (const corner of triangle.slice(0, 3)) {
        corners.set([corner[0] ?? 0, corner[1] ?? 0, corner[2] ?? 0], at);
        at += 3;
      }
    }
    const { gl } = this;
    gl.bindBuffer(gl.ARRAY_BUFFER, this.buffer);
    gl.bufferData(gl.ARRAY_BUFFER, corners.subarray(0, at), gl.STATIC_DRAW);
    this.removedCount = removed.length * 3;
    this.addedCount = added.length * 3;

    let points = 0;
    for (const kind of marks) {
      points += kind.points.length;
    }
    const all = new Float32Array(points);
    let next = 0;
    this.marks = [];
    for (const kind of marks) {
      all.set(kind.points, next);
      this.marks.push({ marks: kind, start: next / 3, count: Math.floor(kind.points.length / 3) });
      next += kind.points.length;
    }
    gl.bindBuffer(gl.ARRAY_BUFFER, this.markBuffer);
    gl.bufferData(gl.ARRAY_BUFFER, all, gl.STATIC_DRAW);
  }

  /** Draws with `matrix` (cells, heights as drawn, raised by `exaggeration`). */
  draw(matrix: Mat4, exaggeration: number) {
    if (this.removedCount + this.addedCount === 0 && this.groups.length + this.marks.length === 0) {
      return;
    }
    const { gl } = this;
    // biome-ignore lint/correctness/useHookAtTopLevel: WebGL's useProgram is no React hook.
    gl.useProgram(this.shader);
    gl.uniformMatrix4fv(gl.getUniformLocation(this.shader, "u_matrix"), false, matrix);
    gl.uniform1f(gl.getUniformLocation(this.shader, "u_exaggeration"), exaggeration);
    gl.uniform1f(gl.getUniformLocation(this.shader, "u_size"), 1);
    const colour = gl.getUniformLocation(this.shader, "u_colour");
    const pattern = gl.getUniformLocation(this.shader, "u_pattern");
    gl.enable(gl.BLEND);
    gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
    gl.disable(gl.CULL_FACE);
    // On top of everything: a path under the land or a deck is shown too.
    gl.disable(gl.DEPTH_TEST);
    gl.depthMask(false);
    gl.bindVertexArray(this.vao);
    for (const { group, start, count, edgeStart, edgeCount } of this.groups) {
      if (count > 0) {
        gl.uniform4f(colour, ...group.colour);
        gl.uniform1i(pattern, PATTERNS[group.pattern]);
        gl.drawArrays(gl.TRIANGLES, start, count);
      }
      if (edgeCount > 0) {
        const [r, g, b, a] = group.colour;
        gl.uniform4f(colour, r, g, b, Math.min(1, a + 0.3));
        gl.uniform1i(pattern, 0);
        gl.drawArrays(gl.LINES, edgeStart, edgeCount);
      }
    }
    gl.uniform1i(pattern, 0);
    gl.uniform4f(colour, ...REMOVED);
    gl.drawArrays(gl.TRIANGLES, this.repairStart, this.removedCount);
    gl.uniform4f(colour, ...ADDED);
    gl.drawArrays(gl.TRIANGLES, this.repairStart + this.removedCount, this.addedCount);

    if (this.marks.length > 0) {
      // biome-ignore lint/correctness/useHookAtTopLevel: WebGL's useProgram is no React hook.
      gl.useProgram(this.markShader);
      const uniform = (name: string) => gl.getUniformLocation(this.markShader, name);
      gl.uniformMatrix4fv(uniform("u_matrix"), false, matrix);
      gl.uniform1f(uniform("u_exaggeration"), exaggeration);
      const ratio = window.devicePixelRatio || 1;
      gl.bindVertexArray(this.markVao);
      for (const { marks, start, count } of this.marks) {
        gl.uniform4f(uniform("u_colour"), ...marks.colour);
        gl.uniform1i(uniform("u_shape"), SHAPES[marks.shape]);
        gl.uniform1f(uniform("u_size"), marks.size * ratio);
        gl.drawArrays(gl.POINTS, start, count);
      }
    }
    gl.bindVertexArray(null);
    gl.depthMask(true);
    gl.enable(gl.DEPTH_TEST);
    gl.disable(gl.BLEND);
  }

  dispose() {
    this.gl.deleteBuffer(this.buffer);
    this.gl.deleteVertexArray(this.vao);
    this.gl.deleteBuffer(this.markBuffer);
    this.gl.deleteVertexArray(this.markVao);
    this.gl.deleteProgram(this.shader);
    this.gl.deleteProgram(this.markShader);
  }
}
