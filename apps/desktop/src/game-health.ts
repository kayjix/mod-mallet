// The health of every game on the "All games" page (ADR 0048): the game in
// use shows the score it has now, every other game the one it had when the
// user last looked at it, kept with that game's settings. Pure, so that the
// tests cover what a card shows.

import type { KnownGame, LastHealth } from "./generated/game-choice";

/** A health as the window weighs it (ADR 0045), as far as it is kept. */
export interface WeighedHealth {
  score: number;
  verdict: LastHealth["verdict"];
}

/** The health to keep for the game in use, from its score and the scan it was weighed from. */
export function lastHealthOf(health: WeighedHealth, scannedAt: number): LastHealth {
  return {
    score: Math.max(0, Math.min(100, Math.round(health.score))),
    verdict: health.verdict,
    scannedAt: Math.max(0, Math.round(scannedAt)),
  };
}

/** Whether keeping `next` would change nothing, so that the settings are not written again. */
export function sameHealth(kept: LastHealth | null | undefined, next: LastHealth): boolean {
  return (
    kept != null &&
    kept.score === next.score &&
    kept.verdict === next.verdict &&
    kept.scannedAt === next.scannedAt
  );
}

/** What a game's card says about its health. */
export type GameScore =
  /** A score: the one of now for the game in use, else the last one kept. */
  | { type: "score"; health: LastHealth; now: boolean }
  /** Here and readable, but never scanned (or not since scores are kept). */
  | { type: "unscanned" }
  /** Nothing to say: not installed, or not read by Mod Mallet yet. */
  | { type: "none" };

/**
 * What the card of `game` shows: `now` is the health of the game in use
 * as the window weighs it at this moment, null while it has no scan.
 */
export function gameScoreOf(
  game: KnownGame | undefined,
  inUse: boolean,
  now: LastHealth | null,
  playable: boolean,
): GameScore {
  if (inUse && now !== null) {
    return { type: "score", health: now, now: true };
  }
  if (game?.lastHealth != null) {
    return { type: "score", health: game.lastHealth, now: false };
  }
  return playable ? { type: "unscanned" } : { type: "none" };
}
