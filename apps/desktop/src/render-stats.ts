// What drawing the 3D views costs, to find what makes them slow (ADR 0075).
// In a development build, or in a built app where the local storage holds
// `mallet.renderStats` = "1", every view's drawing context counts its
// calls, and `__malletBench()` in the console draws each open view a
// number of times and tells how long a frame takes in JavaScript and in
// all, with the graphics card's work. Otherwise nothing is counted.

/** What one view's frames cost, on average over a bench. */
export interface FrameCost {
  view: string;
  /** Draw calls per frame. */
  draws: number;
  /** Buffer and texture uploads per frame. */
  uploads: number;
  /** State calls per frame: programs, textures, arrays and uniforms bound. */
  binds: number;
  /** Milliseconds per frame spent in JavaScript, until the frame is handed on. */
  script: number;
  /** Milliseconds per frame until the graphics card has drawn it. */
  total: number;
}

interface Watched {
  view: string;
  gl: WebGL2RenderingContext;
  draw: () => void;
  counts: { draws: number; uploads: number; binds: number };
}

const watched = new Set<Watched>();

/** How many meshes were read into the views, and how many milliseconds that took in all. */
const meshes = { count: 0, ms: 0 };

/** How often the segments of a comparison were found, how long that took in all and at most. */
const segments = { count: 0, ms: 0, longest: 0 };

/** Counts one finding of a comparison's segments, which took `ms` milliseconds. */
export function noteSegments(ms: number) {
  segments.count += 1;
  segments.ms += ms;
  segments.longest = Math.max(segments.longest, ms);
}

/** Counts one mesh read into a view, which took `ms` milliseconds. */
export function noteMeshes(ms: number) {
  meshes.count += 1;
  meshes.ms += ms;
}

/** Whether the views are watched: in a development build, or when asked for. */
const WATCHING = (() => {
  if (import.meta.env.DEV) {
    return true;
  }
  try {
    return (
      typeof window !== "undefined" && window.localStorage.getItem("mallet.renderStats") === "1"
    );
  } catch {
    return false;
  }
})();

const DRAWS = ["drawElements", "drawElementsInstanced", "drawArrays", "drawArraysInstanced"];
const UPLOADS = ["bufferData", "bufferSubData", "texImage2D", "texSubImage2D"];
const BINDS = [
  "useProgram",
  "bindTexture",
  "bindVertexArray",
  "bindBuffer",
  "uniform1i",
  "uniform1f",
  "uniform3f",
  "uniform4f",
  "uniformMatrix4fv",
  "getUniformLocation",
];

/** Counts the calls of `names` on `gl` into `counts[key]`. */
function count(
  gl: WebGL2RenderingContext,
  names: string[],
  counts: Watched["counts"],
  key: keyof Watched["counts"],
) {
  const target = gl as unknown as Record<string, unknown>;
  for (const name of names) {
    const original = target[name];
    if (typeof original !== "function") {
      continue;
    }
    target[name] = (...args: unknown[]) => {
      counts[key] += 1;
      return (original as (...args: unknown[]) => unknown).apply(gl, args);
    };
  }
}

/**
 * Watches the view `view` drawn into `gl` by `draw`, in a development
 * build only; returns what ends the watch.
 */
export function watchView(view: string, gl: WebGL2RenderingContext, draw: () => void): () => void {
  if (!WATCHING) {
    return () => {};
  }
  const known = [...watched].find((entry) => entry.gl === gl);
  const counts = known?.counts ?? { draws: 0, uploads: 0, binds: 0 };
  if (known === undefined) {
    count(gl, DRAWS, counts, "draws");
    count(gl, UPLOADS, counts, "uploads");
    count(gl, BINDS, counts, "binds");
    // One call for many draws counts as one draw call.
    const multi = gl.getExtension("WEBGL_multi_draw");
    if (multi !== null) {
      count(
        multi as unknown as WebGL2RenderingContext,
        ["multiDrawElementsInstancedWEBGL", "multiDrawElementsWEBGL"],
        counts,
        "draws",
      );
    }
  }
  const entry: Watched = { view, gl, draw, counts };
  watched.add(entry);
  return () => watched.delete(entry);
}

/** Names of renderers that draw without a graphics card. */
const SOFTWARE = /swiftshader|software|llvmpipe|basic render|warp/i;

let card: { renderer: string | null; software: boolean } | null = null;

/**
 * The graphics card the 3D views are drawn with, as the browser names
 * it, and whether that is no card at all but the processor; none where
 * no drawing context can be made.
 */
export function graphicsCard(): { renderer: string | null; software: boolean } {
  if (card !== null) {
    return card;
  }
  try {
    const gl = document.createElement("canvas").getContext("webgl2");
    const renderer = gl === null ? null : rendererOf(gl);
    gl?.getExtension("WEBGL_lose_context")?.loseContext();
    card = { renderer, software: renderer !== null && SOFTWARE.test(renderer) };
  } catch {
    card = { renderer: null, software: false };
  }
  return card;
}

/** The graphics card a context draws with, as the browser names it. */
export function rendererOf(gl: WebGL2RenderingContext): string {
  const info = gl.getExtension("WEBGL_debug_renderer_info");
  const name = info === null ? null : gl.getParameter(info.UNMASKED_RENDERER_WEBGL);
  return typeof name === "string" ? name : String(gl.getParameter(gl.RENDERER));
}

/**
 * Draws every watched view `frames` times and tells what a frame costs;
 * where the browser can time the graphics card, also how long it drew.
 */
async function bench(frames = 30): Promise<(FrameCost & { card: number | null })[]> {
  const pixel = new Uint8Array(4);
  const results: (FrameCost & { card: number | null })[] = [];
  for (const { view, gl, draw, counts } of [...watched]) {
    // One frame first, so that what a first frame sets up is not counted.
    draw();
    gl.readPixels(0, 0, 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, pixel);
    const timer = gl.getExtension("EXT_disjoint_timer_query_webgl2") as {
      TIME_ELAPSED_EXT: number;
    } | null;
    const queries: WebGLQuery[] = [];
    const before = { ...counts };
    let script = 0;
    let total = 0;
    for (let frame = 0; frame < frames; frame++) {
      const query = timer === null ? null : gl.createQuery();
      if (query !== null && timer !== null) {
        gl.beginQuery(timer.TIME_ELAPSED_EXT, query);
        queries.push(query);
      }
      const start = performance.now();
      draw();
      const handed = performance.now();
      if (timer !== null) {
        gl.endQuery(timer.TIME_ELAPSED_EXT);
      }
      // Reading a pixel waits until the card has drawn the frame.
      gl.readPixels(0, 0, 1, 1, gl.RGBA, gl.UNSIGNED_BYTE, pixel);
      const done = performance.now();
      script += handed - start;
      total += done - start;
    }
    const per = (value: number) => Math.round((value / frames) * 100) / 100;
    // The card's times arrive a few frames later.
    let card: number | null = null;
    for (let wait = 0; wait < 20 && queries.length > 0; wait++) {
      await new Promise((resolve) => requestAnimationFrame(resolve));
      const last = queries[queries.length - 1];
      if (last !== undefined && gl.getQueryParameter(last, gl.QUERY_RESULT_AVAILABLE) === true) {
        let sum = 0;
        for (const query of queries) {
          sum += Number(gl.getQueryParameter(query, gl.QUERY_RESULT));
        }
        card = Math.round((sum / queries.length / 1e6) * 100) / 100;
        break;
      }
    }
    for (const query of queries) {
      gl.deleteQuery(query);
    }
    results.push({
      view,
      draws: per(counts.draws - before.draws),
      uploads: per(counts.uploads - before.uploads),
      binds: per(counts.binds - before.binds),
      script: per(script),
      total: per(total),
      card,
    });
  }
  return results;
}

if (WATCHING && typeof window !== "undefined") {
  const global = window as unknown as Record<string, unknown>;
  global.__malletBench = bench;
  global.__malletRenderer = () => [...watched].map(({ view, gl }) => `${view}: ${rendererOf(gl)}`);
  // The calls counted so far, to see whether a view draws while nothing changes.
  global.__malletCounts = () => ({
    views: [...watched].map(({ view, counts }) => ({ view, ...counts })),
    meshes: { ...meshes, ms: Math.round(meshes.ms) },
    segments: { ...segments, ms: Math.round(segments.ms), longest: Math.round(segments.longest) },
  });
  // A picture of each view as a PNG address, to compare two ways of drawing pixel by pixel.
  global.__malletSnapshot = () =>
    [...watched].map(({ view, gl, draw }) => {
      draw();
      const width = gl.drawingBufferWidth;
      const height = gl.drawingBufferHeight;
      const pixels = new Uint8ClampedArray(width * height * 4);
      gl.readPixels(0, 0, width, height, gl.RGBA, gl.UNSIGNED_BYTE, pixels);
      const canvas = document.createElement("canvas");
      canvas.width = width;
      canvas.height = height;
      canvas.getContext("2d")?.putImageData(new ImageData(pixels, width, height), 0, 0);
      return { view, url: canvas.toDataURL("image/png") };
    });
}
