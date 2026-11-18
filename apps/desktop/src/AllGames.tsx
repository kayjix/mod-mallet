import { useEffect, useState } from "react";
import { useSession } from "./Actions";
import { type Games, isPlayable } from "./games";
import type { RenderedReport } from "./generated/rendered-report";
import type { Health } from "./health";
import { useJourneyTexts } from "./journey-texts";
import "./journey.css";

/**
 * The pictures of the games, by game. They come from the mod manager's
 * installation on this machine; Mod Mallet ships none.
 */
export function useGameArt(ids: string[]): Record<string, string> {
  const { api } = useSession();
  const [art, setArt] = useState<Record<string, string>>({});
  const key = ids.join("|");
  useEffect(() => {
    let outdated = false;
    for (const id of key.split("|").filter((id) => id !== "")) {
      api
        .gameArt(id)
        .then((picture) => {
          if (!outdated && picture !== null) {
            setArt((known) => ({ ...known, [id]: picture }));
          }
        })
        .catch(() => {});
    }
    return () => {
      outdated = true;
    };
  }, [api, key]);
  return art;
}

interface AllGamesProps {
  games: Games;
  /** The last scan of the game in use, if any. */
  report: RenderedReport | null;
  /** Its health, if it was scanned. */
  health: Health | null;
  /** Goes to the overview of the game in use. */
  onOpen: () => void;
}

/**
 * The games Mod Mallet knows: the one place to pick the game to work on
 * (ADR 0048). Opening the game in use goes to its overview; opening
 * another starts the window over for it.
 */
export function AllGames({ games, report, health, onOpen }: AllGamesProps) {
  const { t, lang } = useSession();
  const j = useJourneyTexts();
  const art = useGameArt(t.games.map((game) => game.id));
  const activeId = games.choice?.active;
  const known = new Map((games.choice?.games ?? []).map((game) => [game.id, game]));
  const readable = t.games.filter((game) => known.has(game.id));
  const later = t.games.filter((game) => !known.has(game.id));

  return (
    <div className="stack">
      <section className="panel">
        <div className="panel-head">
          <h2>{j.allGamesTitle}</h2>
          <span className="aside">{j.allGamesNote}</span>
        </div>
        <ul className="game-grid">
          {readable.map((game) => {
            const picture = art[game.id];
            const isActive = game.id === activeId;
            const knownGame = known.get(game.id);
            const playable = isPlayable(knownGame);
            const support = knownGame?.support === "basic" ? t.gameBasic : null;
            let state: string;
            if (!playable) {
              state = t.gameNotInstalled;
            } else if (isActive && report !== null && health !== null) {
              const when = new Date(report.scannedAt * 1000).toLocaleString(lang, {
                dateStyle: "short",
                timeStyle: "short",
              });
              state = j.scoreAt(health.score, when);
            } else if (isActive) {
              state = j.notScanned;
            } else {
              state = "";
            }
            return (
              <li
                className="game-card"
                key={game.id}
                data-active={isActive}
                data-playable={playable}
              >
                <div
                  className="game-art"
                  style={picture === undefined ? undefined : { backgroundImage: `url(${picture})` }}
                  title={picture === undefined ? t.homeNoArt : undefined}
                >
                  {picture === undefined && <span aria-hidden="true">{game.name.slice(0, 1)}</span>}
                </div>
                <div className="game-text">
                  <strong>{game.name}</strong>
                  {isActive && <span className="game-in-use">{j.inUse}</span>}
                  {state !== "" && <span className="aside">{state}</span>}
                  {support !== null && <span className="aside">{support}</span>}
                  {playable && (
                    <button
                      type="button"
                      className={isActive ? "game-open primary" : "game-open"}
                      onClick={() => {
                        if (isActive) {
                          onOpen();
                        } else {
                          games.select(game.id).catch(() => {});
                        }
                      }}
                    >
                      {j.openGame}
                    </button>
                  )}
                </div>
              </li>
            );
          })}
        </ul>
        {later.length > 0 && (
          <p className="aside games-later">
            {j.laterGames}:{" "}
            {later
              .map((game) => (game.quip === undefined ? game.name : `${game.name} – ${game.quip}`))
              .join(" · ")}
          </p>
        )}
      </section>
    </div>
  );
}
