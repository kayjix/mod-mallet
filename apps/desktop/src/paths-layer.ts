// The repair of paths in a 3D view (ADR 0058): the pieces of path taken
// out in red, the new ones in green, drawn see-through on top of what is
// there, so that paths under the land or a deck show as well.

import { type Mat4, program } from "./gl";
import { CELL } from "./map-places";

/** Triangles of paths, three corners each, in game units. */
export type PathTriangles = number[][][];

/** What the repair takes out and what it lays instead. */
export interface PathsOverlay {
  removed: PathTriangles;
  added: PathTriangles;
}

/** Red and green that stay apart for most eyes (Okabe–Ito vermilion and bluish green). */
const REMOVED: [number, number, number, number] = [0.84, 0.37, 0.0, 0.55];
const ADDED: [number, number, number, number] = [0.0, 0.62, 0.45, 0.6];

/** How far above the triangles they are drawn, in game units. */
const LIFT = 6;

const VERTEX = `#version 300 es
in vec3 a_position;
uniform mat4 u_matrix;
uniform float u_exaggeration;
void main() {
  vec3 at = a_position;
  gl_Position = u_matrix * vec4(at.xy / ${CELL}.0, (at.z + ${LIFT}.0) / ${CELL}.0 * u_exaggeration, 1.0);
}`;

const FRAGMENT = `#version 300 es
precision mediump float;
uniform vec4 u_colour;
out vec4 colour;
void main() {
  colour = u_colour;
}`;

/** Draws a {@link PathsOverlay} with the matrices of the views. */
export class PathsLayer {
  private readonly gl: WebGL2RenderingContext;
  private readonly shader: WebGLProgram;
  private readonly buffer: WebGLBuffer;
  private readonly vao: WebGLVertexArrayObject;
  private removedCount = 0;
  private addedCount = 0;

  constructor(gl: WebGL2RenderingContext) {
    this.gl = gl;
    this.shader = program(gl, VERTEX, FRAGMENT);
    const buffer = gl.createBuffer();
    const vao = gl.createVertexArray();
    if (buffer === null || vao === null) {
      throw new Error("no buffer");
    }
    this.buffer = buffer;
    this.vao = vao;
    gl.bindVertexArray(vao);
    gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
    const at = gl.getAttribLocation(this.shader, "a_position");
    gl.enableVertexAttribArray(at);
    gl.vertexAttribPointer(at, 3, gl.FLOAT, false, 0, 0);
    gl.bindVertexArray(null);
  }

  /** Shows `overlay`, or nothing. */
  set(overlay: PathsOverlay | null) {
    const removed = overlay?.removed ?? [];
    const added = overlay?.added ?? [];
    const corners = new Float32Array((removed.length + added.length) * 9);
    let at = 0;
    for (const triangle of [...removed, ...added]) {
      for (const corner of triangle.slice(0, 3)) {
        corners.set([corner[0] ?? 0, corner[1] ?? 0, corner[2] ?? 0], at);
        at += 3;
      }
    }
    const { gl } = this;
    gl.bindBuffer(gl.ARRAY_BUFFER, this.buffer);
    gl.bufferData(gl.ARRAY_BUFFER, corners, gl.STATIC_DRAW);
    this.removedCount = removed.length * 3;
    this.addedCount = added.length * 3;
  }

  /** Draws with `matrix` (cells, heights as drawn, raised by `exaggeration`). */
  draw(matrix: Mat4, exaggeration: number) {
    if (this.removedCount + this.addedCount === 0) {
      return;
    }
    const { gl } = this;
    // biome-ignore lint/correctness/useHookAtTopLevel: WebGL's useProgram is no React hook.
    gl.useProgram(this.shader);
    gl.uniformMatrix4fv(gl.getUniformLocation(this.shader, "u_matrix"), false, matrix);
    gl.uniform1f(gl.getUniformLocation(this.shader, "u_exaggeration"), exaggeration);
    const colour = gl.getUniformLocation(this.shader, "u_colour");
    gl.enable(gl.BLEND);
    gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
    gl.disable(gl.CULL_FACE);
    // On top of everything: a path under the land or a deck is shown too.
    gl.disable(gl.DEPTH_TEST);
    gl.depthMask(false);
    gl.bindVertexArray(this.vao);
    gl.uniform4f(colour, ...REMOVED);
    gl.drawArrays(gl.TRIANGLES, 0, this.removedCount);
    gl.uniform4f(colour, ...ADDED);
    gl.drawArrays(gl.TRIANGLES, this.removedCount, this.addedCount);
    gl.bindVertexArray(null);
    gl.depthMask(true);
    gl.enable(gl.DEPTH_TEST);
    gl.disable(gl.BLEND);
  }

  dispose() {
    this.gl.deleteBuffer(this.buffer);
    this.gl.deleteVertexArray(this.vao);
    this.gl.deleteProgram(this.shader);
  }
}
