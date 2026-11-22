// Where the real objects stand up close, and how high, seen from above
// (ADR 0074): the level of detail leaves its reduced objects out there,
// and only there, so that it neither shows through the real ones nor
// leaves a hole where nothing real is drawn.

import type { AreaLayer } from "./area-layer";

/** How far beyond the area the texture reaches, in game units: objects there overhang. */
const PAD = 1024;
/** About how many game units one texel covers (some 20 cm). */
const TEXEL = 16;
/** The texture's largest side. */
const LARGEST = 2048;
/** The heights the texture tells apart, in game units. */
const RANGE: [number, number] = [-60000, 120000];

/** The heights of the real objects, for the level of detail to leave room for. */
export interface RealTopsView {
  /** A depth texture: 1 where nothing is, nearer the higher an object reaches. */
  texture: WebGLTexture;
  /** West, south, east, north, in game units. */
  rect: [number, number, number, number];
  /** The lowest and highest height the depth spans, in game units. */
  range: [number, number];
}

/** Keeps the texture of {@link RealTopsView} up to date with an {@link AreaLayer}. */
export class RealTops {
  private texture: WebGLTexture | null = null;
  private readonly frame: WebGLFramebuffer;
  private side = [0, 0];
  private key = "";

  constructor(private readonly gl: WebGL2RenderingContext) {
    this.frame = gl.createFramebuffer();
  }

  /** The heights of what `layer` draws now, drawn anew when that has changed. */
  update(layer: AreaLayer): RealTopsView | null {
    const area = layer.area;
    if (area === null || area.objects.length === 0) {
      return null;
    }
    const [x, y] = area.centre;
    const reach = area.radius + PAD;
    const rect: [number, number, number, number] = [x - reach, y - reach, x + reach, y + reach];
    const key = `${layer.version} ${rect.join(" ")}`;
    if (key !== this.key || this.texture === null) {
      if (!this.render(layer, rect)) {
        return null;
      }
      this.key = key;
    }
    return this.texture === null ? null : { texture: this.texture, rect, range: RANGE };
  }

  private render(layer: AreaLayer, rect: [number, number, number, number]): boolean {
    const { gl } = this;
    const most = Math.min(LARGEST, gl.getParameter(gl.MAX_TEXTURE_SIZE) as number);
    const side = Math.max(256, Math.min(most, Math.ceil((rect[2] - rect[0]) / TEXEL)));
    if (this.texture === null || this.side[0] !== side) {
      if (this.texture !== null) {
        gl.deleteTexture(this.texture);
      }
      const made = gl.createTexture();
      gl.bindTexture(gl.TEXTURE_2D, made);
      gl.texStorage2D(gl.TEXTURE_2D, 1, gl.DEPTH_COMPONENT32F, side, side);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
      gl.bindFramebuffer(gl.FRAMEBUFFER, this.frame);
      gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.DEPTH_ATTACHMENT, gl.TEXTURE_2D, made, 0);
      gl.bindFramebuffer(gl.FRAMEBUFFER, null);
      this.texture = made;
      this.side = [side, side];
    }
    const viewport = gl.getParameter(gl.VIEWPORT) as Int32Array;
    gl.bindFramebuffer(gl.FRAMEBUFFER, this.frame);
    if (gl.checkFramebufferStatus(gl.FRAMEBUFFER) !== gl.FRAMEBUFFER_COMPLETE) {
      gl.bindFramebuffer(gl.FRAMEBUFFER, null);
      return false;
    }
    gl.viewport(0, 0, side, side);
    gl.enable(gl.DEPTH_TEST);
    gl.depthMask(true);
    gl.clearDepth(1);
    gl.clear(gl.DEPTH_BUFFER_BIT);
    layer.drawTops(rect, RANGE);
    gl.bindFramebuffer(gl.FRAMEBUFFER, null);
    gl.viewport(viewport[0] ?? 0, viewport[1] ?? 0, viewport[2] ?? 0, viewport[3] ?? 0);
    return true;
  }

  /** Frees the texture; the object is not used again. */
  dispose() {
    this.gl.deleteFramebuffer(this.frame);
    if (this.texture !== null) {
      this.gl.deleteTexture(this.texture);
    }
    this.texture = null;
  }
}
