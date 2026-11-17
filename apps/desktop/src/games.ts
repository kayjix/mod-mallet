// The games Mod Mallet knows and the one the user looks at.

import { useCallback, useEffect, useState } from "react";
import type { Api } from "./api";
import type { GameChoice, KnownGame } from "./generated/game-choice";

/** What the window knows about the games, and how to switch between them. */
export interface Games {
  /** `null` until the Rust side has answered. */
  choice: GameChoice | null;
  /** The game looked at, once known. */
  active: KnownGame | null;
  /** Makes `id` the game to look at and starts the window over for it. */
  select: (id: string) => Promise<void>;
}

/**
 * The games, asked once per window. Switching a game starts the window over:
 * the report, the places, the findings set aside and the ranking all belong
 * to one game, and nothing of the last game may linger.
 */
export function useGames(api: Api): Games {
  const [choice, setChoice] = useState<GameChoice | null>(null);
  useEffect(() => {
    let outdated = false;
    api
      .games()
      .then((games) => {
        if (!outdated) {
          setChoice(games);
        }
      })
      .catch(() => {});
    return () => {
      outdated = true;
    };
  }, [api]);

  const select = useCallback(
    async (id: string) => {
      if (id === choice?.active) {
        return;
      }
      await api.selectGame(id);
      window.location.reload();
    },
    [api, choice],
  );

  const active = choice?.games.find((game) => game.id === choice.active) ?? null;
  return { choice, active, select };
}

/** Whether a game can be looked at: Mod Mallet reads it and it is here. */
export const isPlayable = (game: KnownGame | undefined): boolean =>
  game !== undefined && game.dir != null;
