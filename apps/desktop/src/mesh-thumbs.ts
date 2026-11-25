// Small pictures of meshes for lists (ADR 0085): asked for as rows come
// into view, a few at a time and the newest rows first, and kept for a
// while. The engine draws them off the window's thread and keeps them in
// its cache, so asking again after a restart is quick.

import type { Api } from "./api";
import type { MeshThumbnail } from "./generated/mesh-thumbnail";

/** Pixels per side of a picture: sharp in a slot of 48 px on a screen at 200 %. */
export const THUMB_SIDE = 112;
/** How many pictures one call asks for. */
const BATCH = 6;
/** How many calls run at once. */
const PARALLEL = 2;
/** How long a picture is kept here before it is asked for again (the engine's cache answers that at once). */
const KEPT_MS = 5 * 60 * 1000;
/** How many pictures are kept here at most. */
const MOST_KEPT = 600;

interface Waiting {
  key: string;
  path: string;
  version: string | null;
  resolve: (found: MeshThumbnail | null) => void;
}

/** The pictures asked for through one API, and those still waiting. */
export class ThumbQueue {
  private readonly known = new Map<
    string,
    { at: number; found: Promise<MeshThumbnail | null>; wanted: number }
  >();
  private waiting: Waiting[] = [];
  private running = 0;
  private scheduled = false;

  private readonly ask: (
    requests: { path: string; version: string | null }[],
  ) => Promise<MeshThumbnail[]>;
  private readonly now: () => number;

  constructor(
    ask: (requests: { path: string; version: string | null }[]) => Promise<MeshThumbnail[]>,
    now: () => number = Date.now,
  ) {
    this.ask = ask;
    this.now = now;
  }

  /** The picture of `path` in `version` (`null`: the one the game uses); `null` if it cannot be had. */
  get(path: string, version: string | null): Promise<MeshThumbnail | null> {
    const key = keyOf(path, version);
    const kept = this.known.get(key);
    if (kept !== undefined && this.now() - kept.at < KEPT_MS) {
      kept.wanted += 1;
      return kept.found;
    }
    const found = new Promise<MeshThumbnail | null>((resolve) => {
      this.waiting.push({ key, path, version, resolve });
    });
    this.known.set(key, { at: this.now(), found, wanted: 1 });
    this.trim();
    this.schedule();
    return found;
  }

  /** Takes back a picture no longer wanted, such as of a row scrolled out of view, if it was not asked for yet. */
  forget(path: string, version: string | null) {
    const key = keyOf(path, version);
    const kept = this.known.get(key);
    if (kept === undefined) {
      return;
    }
    kept.wanted -= 1;
    if (kept.wanted > 0) {
      return;
    }
    const at = this.waiting.findIndex((waiting) => waiting.key === key);
    if (at >= 0) {
      this.waiting.splice(at, 1);
      this.known.delete(key);
    }
  }

  /** How many pictures wait to be asked for. */
  get pending(): number {
    return this.waiting.length;
  }

  private schedule() {
    if (this.scheduled) {
      return;
    }
    this.scheduled = true;
    // Rows that come into view together are asked for together.
    queueMicrotask(() => {
      this.scheduled = false;
      this.pump();
    });
  }

  private pump() {
    while (this.running < PARALLEL && this.waiting.length > 0) {
      // The rows that came into view last are the ones looked at now.
      const batch = this.waiting.splice(Math.max(0, this.waiting.length - BATCH));
      this.running += 1;
      this.ask(batch.map(({ path, version }) => ({ path, version })))
        .then((found) => {
          batch.forEach((waiting, index) => {
            waiting.resolve(found[index] ?? null);
          });
        })
        .catch(() => {
          for (const waiting of batch) {
            // Asked again next time, rather than kept as failed.
            this.known.delete(waiting.key);
            waiting.resolve(null);
          }
        })
        .finally(() => {
          this.running -= 1;
          this.pump();
        });
    }
  }

  private trim() {
    while (this.known.size > MOST_KEPT) {
      const oldest = this.known.keys().next().value;
      if (oldest === undefined) {
        return;
      }
      this.known.delete(oldest);
    }
  }
}

function keyOf(path: string, version: string | null): string {
  return `${path.toLowerCase()}|${version ?? ""}`;
}

const queues = new WeakMap<Api, ThumbQueue>();

/** The queue of pictures of `api`, one for the whole window. */
export function thumbQueue(api: Api): ThumbQueue {
  let queue = queues.get(api);
  if (queue === undefined) {
    queue = new ThumbQueue((requests) => api.meshThumbnails(requests, THUMB_SIDE));
    queues.set(api, queue);
  }
  return queue;
}
