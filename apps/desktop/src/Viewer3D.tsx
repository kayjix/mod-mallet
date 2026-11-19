import { type PointerEvent, type RefObject, useEffect, useRef, useState } from "react";
import { useSession } from "./Actions";
import type { MeshShape, MeshView } from "./generated/mesh-view";
import { LoadingLine } from "./LoadingMallet";

/** One thing to show: a mesh of the setup, or a texture on a ball. */
export type StageItem =
  | {
      kind: "mesh";
      label: string;
      path: string;
      /** Counted from the version lowest in precedence; absent for the one in the game. */
      version?: number;
      /** A version of a catalogue thing (ADR 0054), by its identity; it wins over `version`. */
      assetVersion?: string;
    }
  | { kind: "texture"; label: string; path: string; image: string | null };

/** One mesh to show: which file, which version of it, and what to call it. */
export interface MeshItem {
  label: string;
  path: string;
  /** Counted from the version lowest in precedence; absent for the one in the game. */
  version?: number;
}

/** Surroundings to set a thing into: a colour behind it and the light around it. */
export const SCENES = ["dark", "white", "meadow", "forest", "cave", "snow"] as const;
export type Scene = (typeof SCENES)[number];

/** The light all around in each scene, as red, green, blue. */
const AMBIENT: Record<Scene, [number, number, number]> = {
  dark: [0.32, 0.36, 0.4],
  white: [0.6, 0.6, 0.62],
  meadow: [0.48, 0.53, 0.42],
  forest: [0.3, 0.4, 0.3],
  cave: [0.3, 0.24, 0.2],
  snow: [0.58, 0.64, 0.72],
};

/** How the stage is lit; the same for both sides. */
export interface Lighting {
  brightness: number;
  /** Where the light stands around the thing, in degrees. */
  azimuth: number;
  /** How high the light stands, in degrees. */
  elevation: number;
  scene: Scene;
}

export const DEFAULT_LIGHTING: Lighting = {
  brightness: 1,
  azimuth: 35,
  elevation: 40,
  scene: "dark",
};

interface Camera {
  yaw: number;
  pitch: number;
  zoom: number;
  spinning: boolean;
  drag: null | { x: number; y: number };
  /** When the camera last turned by itself, so two panes turn it once. */
  turnedAt: number;
}

const START_CAMERA = { yaw: 0.6, pitch: 0.25, zoom: 1, spinning: true };

/**
 * One or two things side by side, seen through one camera: turning one
 * turns both. With controls for the light, the surroundings, turning and
 * going back to the start.
 */
export function Stage({ items }: { items: StageItem[] }) {
  const { t } = useSession();
  const camera = useRef<Camera>({ ...START_CAMERA, drag: null, turnedAt: 0 });
  const [lighting, setLighting] = useState<Lighting>(DEFAULT_LIGHTING);
  const lightingRef = useRef(lighting);
  lightingRef.current = lighting;
  const [spinning, setSpinning] = useState(true);

  const toggleSpin = () => {
    camera.current.spinning = !camera.current.spinning;
    setSpinning(camera.current.spinning);
  };
  const reset = () => {
    Object.assign(camera.current, START_CAMERA);
    setSpinning(true);
    setLighting(DEFAULT_LIGHTING);
  };
  const sceneLabel: Record<Scene, string> = {
    dark: t.sceneDark,
    white: t.sceneWhite,
    meadow: t.sceneMeadow,
    forest: t.sceneForest,
    cave: t.sceneCave,
    snow: t.sceneSnow,
  };
  const slider = (
    label: string,
    key: "brightness" | "azimuth" | "elevation",
    min: number,
    max: number,
    step: number,
  ) => (
    <label className="stage-control">
      <span>{label}</span>
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={lighting[key]}
        onChange={(event) => setLighting({ ...lighting, [key]: Number(event.target.value) })}
      />
    </label>
  );

  return (
    <div className="stage" data-scene={lighting.scene}>
      <div className="stage-panes" data-count={items.length}>
        {items.map((item) => (
          <StagePane
            key={`${item.kind}/${item.path}/${item.kind === "mesh" ? (item.assetVersion ?? item.version ?? "game") : item.label}`}
            item={item}
            camera={camera}
            lighting={lightingRef}
            onTouched={() => setSpinning(false)}
          />
        ))}
      </div>
      <div className="stage-controls">
        <button type="button" onClick={toggleSpin}>
          {spinning ? t.stageStop : t.stageTurn}
        </button>
        <button type="button" onClick={reset}>
          {t.stageReset}
        </button>
        {slider(t.stageBrightness, "brightness", 0.3, 2, 0.05)}
        {slider(t.stageLightAround, "azimuth", 0, 360, 5)}
        {slider(t.stageLightHeight, "elevation", -20, 85, 5)}
        <label className="stage-control">
          <span>{t.stageScene}</span>
          <select
            value={lighting.scene}
            onChange={(event) => setLighting({ ...lighting, scene: event.target.value as Scene })}
          >
            {SCENES.map((scene) => (
              <option key={scene} value={scene}>
                {sceneLabel[scene]}
              </option>
            ))}
          </select>
        </label>
      </div>
    </div>
  );
}

/**
 * The stage in a window over the page, closed with Escape or the button.
 * Nothing is loaded before it opens; closing frees the graphics memory.
 */
export function Viewer3D({ items, onClose }: { items: MeshItem[]; onClose: () => void }) {
  const { t } = useSession();
  useEffect(() => {
    const close = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        onClose();
      }
    };
    window.addEventListener("keydown", close);
    return () => window.removeEventListener("keydown", close);
  }, [onClose]);

  return (
    <div className="viewer-backdrop" role="dialog" aria-modal="true" aria-label={t.viewer3d}>
      <div className="viewer">
        <div className="viewer-head">
          <strong>{t.viewer3d}</strong>
          <span className="aside">{t.viewerHint}</span>
          <span className="spacer" />
          <button type="button" onClick={onClose}>
            {t.viewerClose}
          </button>
        </div>
        <Stage items={items.map((item) => ({ kind: "mesh", ...item }))} />
      </div>
    </div>
  );
}

function StagePane({
  item,
  camera,
  lighting,
  onTouched,
}: {
  item: StageItem;
  camera: RefObject<Camera>;
  lighting: RefObject<Lighting>;
  onTouched: () => void;
}) {
  const { t, api } = useSession();
  const [mesh, setMesh] = useState<MeshView | null | undefined>(undefined);
  useEffect(() => {
    if (item.kind === "texture") {
      setMesh(item.image === null ? null : ball(item.path, item.image));
      return;
    }
    let outdated = false;
    setMesh(undefined);
    (item.assetVersion === undefined
      ? api.meshView(item.path, item.version ?? null)
      : api.assetView(item.path, item.assetVersion)
    )
      .then((found) => {
        if (!outdated) {
          setMesh(found);
        }
      })
      .catch(() => {
        if (!outdated) {
          setMesh(null);
        }
      });
    return () => {
      outdated = true;
    };
  }, [api, item]);

  return (
    <figure className="stage-pane">
      {mesh === undefined ? (
        <LoadingLine className="viewer-message" text={t.previewLoading} />
      ) : mesh === null ? (
        <p className="aside viewer-message">{t.viewerNothing}</p>
      ) : (
        <MeshCanvas mesh={mesh} camera={camera} lighting={lighting} onTouched={onTouched} />
      )}
      <figcaption>
        <strong>{item.label}</strong>
        {mesh != null && mesh.shapes.length === 0 && (mesh.bones ?? "") !== "" && (
          <span className="aside stage-note">{t.viewerSkeleton}</span>
        )}
      </figcaption>
    </figure>
  );
}

// ---- a ball for textures ----------------------------------------------------

function base64Of(numbers: Float32Array | Uint32Array): string {
  const bytes = new Uint8Array(numbers.buffer);
  let text = "";
  for (const byte of bytes) {
    text += String.fromCharCode(byte);
  }
  return btoa(text);
}

/** A ball with the texture wrapped around it twice, like a material preview. */
function ball(path: string, image: string): MeshView {
  const positions: number[] = [];
  const uvs: number[] = [];
  const indices: number[] = [];
  const rings = 32;
  const segments = 48;
  for (let ring = 0; ring <= rings; ring++) {
    const theta = (ring / rings) * Math.PI;
    for (let segment = 0; segment <= segments; segment++) {
      const phi = (segment / segments) * Math.PI * 2;
      positions.push(
        50 * Math.sin(theta) * Math.cos(phi),
        50 * Math.sin(theta) * Math.sin(phi),
        50 * Math.cos(theta),
      );
      uvs.push((segment / segments) * 2, ring / rings);
    }
  }
  for (let ring = 0; ring < rings; ring++) {
    for (let segment = 0; segment < segments; segment++) {
      const a = ring * (segments + 1) + segment;
      const b = a + segments + 1;
      indices.push(a, b, a + 1, b, b + 1, a + 1);
    }
  }
  const shape: MeshShape = {
    positions: base64Of(new Float32Array(positions)),
    uvs: base64Of(new Float32Array(uvs)),
    indices: base64Of(new Uint32Array(indices)),
    texture: image,
    tinted: false,
    alphaTest: false,
  };
  return { path, archive: null, tint: null, shapes: [shape], bones: "" };
}

// ---- drawing ----------------------------------------------------------------

type Mat4 = Float32Array;

function perspective(fov: number, aspect: number, near: number, far: number): Mat4 {
  const f = 1 / Math.tan(fov / 2);
  const range = 1 / (near - far);
  return new Float32Array([
    f / aspect,
    0,
    0,
    0,
    0,
    f,
    0,
    0,
    0,
    0,
    (near + far) * range,
    -1,
    0,
    0,
    2 * near * far * range,
    0,
  ]);
}

function lookAt(eye: number[], target: number[], up: number[]): Mat4 {
  const sub = (a: number[], b: number[]) => a.map((value, index) => value - (b[index] ?? 0));
  const norm = (a: number[]) => {
    const length = Math.hypot(...a) || 1;
    return a.map((value) => value / length);
  };
  const cross = (a: number[], b: number[]) => [
    (a[1] ?? 0) * (b[2] ?? 0) - (a[2] ?? 0) * (b[1] ?? 0),
    (a[2] ?? 0) * (b[0] ?? 0) - (a[0] ?? 0) * (b[2] ?? 0),
    (a[0] ?? 0) * (b[1] ?? 0) - (a[1] ?? 0) * (b[0] ?? 0),
  ];
  const dot = (a: number[], b: number[]) =>
    a.reduce((sum, value, index) => sum + value * (b[index] ?? 0), 0);
  const z = norm(sub(eye, target));
  const x = norm(cross(up, z));
  const y = cross(z, x);
  return new Float32Array([
    x[0] ?? 0,
    y[0] ?? 0,
    z[0] ?? 0,
    0,
    x[1] ?? 0,
    y[1] ?? 0,
    z[1] ?? 0,
    0,
    x[2] ?? 0,
    y[2] ?? 0,
    z[2] ?? 0,
    0,
    -dot(x, eye),
    -dot(y, eye),
    -dot(z, eye),
    1,
  ]);
}

function multiply(a: Mat4, b: Mat4): Mat4 {
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
  for (let index = 0; index + 2 < indices.length; index += 3) {
    const [a, b, c] = [indices[index] ?? 0, indices[index + 1] ?? 0, indices[index + 2] ?? 0];
    const p = (point: number, axis: number) => positions[point * 3 + axis] ?? 0;
    const u = [0, 1, 2].map((axis) => p(b, axis) - p(a, axis));
    const v = [0, 1, 2].map((axis) => p(c, axis) - p(a, axis));
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
      Math.hypot(normals[point] ?? 0, normals[point + 1] ?? 0, normals[point + 2] ?? 0) || 1;
    for (let axis = 0; axis < 3; axis++) {
      normals[point + axis] = (normals[point + axis] ?? 0) / length;
    }
  }
  return normals;
}

const VERTEX = `#version 300 es
in vec3 position;
in vec3 normal;
in vec2 uv;
uniform mat4 matrix;
out vec3 vNormal;
out vec2 vUv;
void main() {
  vNormal = normal;
  vUv = uv;
  gl_Position = matrix * vec4(position, 1.0);
}`;

const FRAGMENT = `#version 300 es
precision mediump float;
in vec3 vNormal;
in vec2 vUv;
uniform sampler2D colour;
uniform sampler2D tint;
uniform bool hasColour;
uniform bool hasTint;
uniform bool alphaTest;
uniform vec3 lightDirection;
uniform vec3 ambient;
uniform float brightness;
out vec4 result;
void main() {
  vec4 sampled = hasColour ? texture(colour, vUv) : vec4(0.72, 0.72, 0.72, 1.0);
  if (alphaTest && sampled.a < 0.5) {
    discard;
  }
  vec3 base = sampled.rgb;
  if (hasTint) {
    base = clamp(base * texture(tint, vUv).rgb * 2.0, 0.0, 1.0);
  }
  // Normals of meshes do not always face out; light both sides alike.
  float direct = abs(dot(normalize(vNormal), normalize(lightDirection)));
  vec3 light = ambient + vec3(0.8) * direct;
  result = vec4(clamp(base * light * brightness, 0.0, 1.0), 1.0);
}`;

/** Bones of a skeleton: plain lines and dots at the joints, unlit. */
const BONE_VERTEX = `#version 300 es
in vec3 position;
uniform mat4 matrix;
uniform float pointSize;
void main() {
  gl_PointSize = pointSize;
  gl_Position = matrix * vec4(position, 1.0);
}`;

const BONE_FRAGMENT = `#version 300 es
precision mediump float;
uniform vec3 colour;
out vec4 result;
void main() {
  result = vec4(colour, 1.0);
}`;

/** The colour of bones: warm, to stand out on dark and on light scenes alike. */
const BONE_COLOUR: [number, number, number] = [0.96, 0.62, 0.22];

interface Drawn {
  vao: WebGLVertexArrayObject;
  count: number;
  colour: WebGLTexture | null;
  tinted: boolean;
  alphaTest: boolean;
}

function MeshCanvas({
  mesh,
  camera,
  lighting,
  onTouched,
}: {
  mesh: MeshView;
  camera: RefObject<Camera>;
  lighting: RefObject<Lighting>;
  onTouched: () => void;
}) {
  const canvas = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const element = canvas.current;
    const gl = element?.getContext("webgl2", { antialias: true, premultipliedAlpha: false });
    if (element == null || gl == null) {
      return;
    }
    const compile = (type: number, source: string) => {
      const shader = gl.createShader(type);
      if (shader === null) {
        return null;
      }
      gl.shaderSource(shader, source);
      gl.compileShader(shader);
      return shader;
    };
    const link = (vertexSource: string, fragmentSource: string) => {
      const linked = gl.createProgram();
      const vertex = compile(gl.VERTEX_SHADER, vertexSource);
      const fragment = compile(gl.FRAGMENT_SHADER, fragmentSource);
      if (linked === null || vertex === null || fragment === null) {
        return null;
      }
      gl.attachShader(linked, vertex);
      gl.attachShader(linked, fragment);
      gl.linkProgram(linked);
      return linked;
    };
    const program = link(VERTEX, FRAGMENT);
    if (program === null) {
      return;
    }
    // WebGL's "useProgram" is no React hook; the linter only sees the name.
    const activate = gl.useProgram.bind(gl);
    activate(program);

    const texture = (url: string | null | undefined): WebGLTexture | null => {
      if (url == null) {
        return null;
      }
      const made = gl.createTexture();
      gl.bindTexture(gl.TEXTURE_2D, made);
      gl.texImage2D(
        gl.TEXTURE_2D,
        0,
        gl.RGBA,
        1,
        1,
        0,
        gl.RGBA,
        gl.UNSIGNED_BYTE,
        new Uint8Array([180, 180, 180, 255]),
      );
      const image = new Image();
      image.onload = () => {
        gl.bindTexture(gl.TEXTURE_2D, made);
        gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, image);
        gl.generateMipmap(gl.TEXTURE_2D);
        gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR_MIPMAP_LINEAR);
      };
      image.src = url;
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.REPEAT);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.REPEAT);
      return made;
    };
    const tint = texture(mesh.tint);

    // Everything in one box, to aim the camera at.
    const low = [Infinity, Infinity, Infinity];
    const high = [-Infinity, -Infinity, -Infinity];
    const drawn: Drawn[] = [];
    for (const shape of mesh.shapes) {
      const positions = new Float32Array(bytesOf(shape.positions));
      const indices = new Uint32Array(bytesOf(shape.indices));
      const uvs =
        shape.uvs === ""
          ? new Float32Array((positions.length / 3) * 2)
          : new Float32Array(bytesOf(shape.uvs));
      for (let index = 0; index < positions.length; index++) {
        const axis = index % 3;
        low[axis] = Math.min(low[axis] ?? 0, positions[index] ?? 0);
        high[axis] = Math.max(high[axis] ?? 0, positions[index] ?? 0);
      }
      const vao = gl.createVertexArray();
      gl.bindVertexArray(vao);
      const attribute = (name: string, data: Float32Array, size: number) => {
        const location = gl.getAttribLocation(program, name);
        if (location < 0) {
          return;
        }
        const buffer = gl.createBuffer();
        gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
        gl.bufferData(gl.ARRAY_BUFFER, data, gl.STATIC_DRAW);
        gl.enableVertexAttribArray(location);
        gl.vertexAttribPointer(location, size, gl.FLOAT, false, 0, 0);
      };
      attribute("position", positions, 3);
      attribute("normal", normalsOf(positions, indices), 3);
      attribute("uv", uvs, 2);
      const elements = gl.createBuffer();
      gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, elements);
      gl.bufferData(gl.ELEMENT_ARRAY_BUFFER, indices, gl.STATIC_DRAW);
      if (vao !== null) {
        drawn.push({
          vao,
          count: indices.length,
          colour: texture(shape.texture),
          tinted: shape.tinted,
          alphaTest: shape.alphaTest ?? false,
        });
      }
    }

    // A skeleton's bones, two points each, as lines with dots at the joints.
    const bonePoints =
      (mesh.bones ?? "") === "" ? new Float32Array(0) : new Float32Array(bytesOf(mesh.bones ?? ""));
    const boneCount = Math.floor(bonePoints.length / 6) * 2;
    const boneProgram = boneCount > 0 ? link(BONE_VERTEX, BONE_FRAGMENT) : null;
    let boneVao: WebGLVertexArrayObject | null = null;
    if (boneProgram !== null) {
      for (let index = 0; index < boneCount * 3; index++) {
        const axis = index % 3;
        low[axis] = Math.min(low[axis] ?? 0, bonePoints[index] ?? 0);
        high[axis] = Math.max(high[axis] ?? 0, bonePoints[index] ?? 0);
      }
      boneVao = gl.createVertexArray();
      gl.bindVertexArray(boneVao);
      const location = gl.getAttribLocation(boneProgram, "position");
      if (location >= 0) {
        const buffer = gl.createBuffer();
        gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
        gl.bufferData(gl.ARRAY_BUFFER, bonePoints.subarray(0, boneCount * 3), gl.STATIC_DRAW);
        gl.enableVertexAttribArray(location);
        gl.vertexAttribPointer(location, 3, gl.FLOAT, false, 0, 0);
      }
    }
    const boneMatrixAt = boneProgram === null ? null : gl.getUniformLocation(boneProgram, "matrix");
    const boneColourAt = boneProgram === null ? null : gl.getUniformLocation(boneProgram, "colour");
    const pointSizeAt =
      boneProgram === null ? null : gl.getUniformLocation(boneProgram, "pointSize");

    const centre = [0, 1, 2].map((axis) => ((low[axis] ?? 0) + (high[axis] ?? 0)) / 2);
    const radius = Math.max(
      1,
      Math.hypot(...[0, 1, 2].map((axis) => (high[axis] ?? 0) - (low[axis] ?? 0))) / 2,
    );

    gl.enable(gl.DEPTH_TEST);
    const at = (name: string) => gl.getUniformLocation(program, name);
    const matrixAt = at("matrix");
    const hasColourAt = at("hasColour");
    const hasTintAt = at("hasTint");
    const alphaTestAt = at("alphaTest");
    const lightAt = at("lightDirection");
    const ambientAt = at("ambient");
    const brightnessAt = at("brightness");
    gl.uniform1i(at("colour"), 0);
    gl.uniform1i(at("tint"), 1);

    let frame = 0;
    const draw = (now: number) => {
      const view = camera.current;
      const light = lighting.current;
      // Two panes share the camera; whichever draws first turns it.
      if (view.spinning && view.turnedAt !== 0 && now > view.turnedAt) {
        view.yaw += (Math.min(100, now - view.turnedAt) / 1000) * 0.5;
      }
      if (now > view.turnedAt) {
        view.turnedAt = now;
      }
      const ratio = window.devicePixelRatio || 1;
      const width = Math.floor(element.clientWidth * ratio);
      const height = Math.floor(element.clientHeight * ratio);
      if (element.width !== width || element.height !== height) {
        element.width = width;
        element.height = height;
      }
      gl.viewport(0, 0, width, height);
      // Clear: the scene behind is drawn by the page.
      gl.clearColor(0, 0, 0, 0);
      gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
      const distance = radius * 2.4 * view.zoom;
      // Skyrim's up is z.
      const eye = [
        (centre[0] ?? 0) + distance * Math.cos(view.pitch) * Math.cos(view.yaw),
        (centre[1] ?? 0) + distance * Math.cos(view.pitch) * Math.sin(view.yaw),
        (centre[2] ?? 0) + distance * Math.sin(view.pitch),
      ];
      const matrix = multiply(
        perspective(0.7, width / Math.max(1, height), radius * 0.05, radius * 20),
        lookAt(eye, centre, [0, 0, 1]),
      );
      // The light stays where it was set as seen from the camera.
      const around = (light.azimuth * Math.PI) / 180 + view.yaw;
      const up = (light.elevation * Math.PI) / 180;
      gl.uniformMatrix4fv(matrixAt, false, matrix);
      gl.uniform3f(
        lightAt,
        Math.cos(up) * Math.cos(around),
        Math.cos(up) * Math.sin(around),
        Math.sin(up),
      );
      const [red, green, blue] = AMBIENT[light.scene];
      gl.uniform3f(ambientAt, red, green, blue);
      gl.uniform1f(brightnessAt, light.brightness);
      for (const shape of drawn) {
        gl.activeTexture(gl.TEXTURE0);
        gl.bindTexture(gl.TEXTURE_2D, shape.colour);
        gl.uniform1i(hasColourAt, shape.colour === null ? 0 : 1);
        gl.activeTexture(gl.TEXTURE1);
        gl.bindTexture(gl.TEXTURE_2D, tint);
        gl.uniform1i(hasTintAt, shape.tinted && tint !== null ? 1 : 0);
        gl.uniform1i(alphaTestAt, shape.alphaTest ? 1 : 0);
        gl.bindVertexArray(shape.vao);
        gl.drawElements(gl.TRIANGLES, shape.count, gl.UNSIGNED_INT, 0);
      }
      if (boneProgram !== null && boneVao !== null) {
        activate(boneProgram);
        gl.uniformMatrix4fv(boneMatrixAt, false, matrix);
        gl.uniform3f(boneColourAt, ...BONE_COLOUR);
        gl.uniform1f(pointSizeAt, 5 * ratio);
        gl.bindVertexArray(boneVao);
        gl.drawArrays(gl.LINES, 0, boneCount);
        gl.drawArrays(gl.POINTS, 0, boneCount);
        activate(program);
      }
      frame = requestAnimationFrame(draw);
    };
    frame = requestAnimationFrame(draw);
    return () => {
      cancelAnimationFrame(frame);
      // The graphics memory goes back at once, not when the page is closed.
      gl.getExtension("WEBGL_lose_context")?.loseContext();
    };
  }, [mesh, camera, lighting]);

  const down = (event: PointerEvent<HTMLCanvasElement>) => {
    event.currentTarget.setPointerCapture(event.pointerId);
    camera.current.spinning = false;
    camera.current.drag = { x: event.clientX, y: event.clientY };
    onTouched();
  };
  const move = (event: PointerEvent<HTMLCanvasElement>) => {
    const view = camera.current;
    if (view.drag === null) {
      return;
    }
    view.yaw -= (event.clientX - view.drag.x) * 0.01;
    view.pitch = Math.max(-1.4, Math.min(1.4, view.pitch + (event.clientY - view.drag.y) * 0.01));
    view.drag = { x: event.clientX, y: event.clientY };
  };
  const up = () => {
    camera.current.drag = null;
  };
  useEffect(() => {
    const element = canvas.current;
    if (element === null) {
      return;
    }
    const wheel = (event: WheelEvent) => {
      event.preventDefault();
      const view = camera.current;
      view.zoom = Math.max(0.2, Math.min(5, view.zoom * (event.deltaY > 0 ? 1.12 : 0.89)));
    };
    element.addEventListener("wheel", wheel, { passive: false });
    return () => element.removeEventListener("wheel", wheel);
  }, [camera]);

  return (
    <canvas
      ref={canvas}
      className="viewer-canvas"
      onPointerDown={down}
      onPointerMove={move}
      onPointerUp={up}
      onPointerCancel={up}
    />
  );
}
