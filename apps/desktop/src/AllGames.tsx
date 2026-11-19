import { useEffect, useState } from "react";
import { useSession } from "./Actions";
import { openTaskCount } from "./arrange";
import { type Games, isPlayable } from "./games";
import type { RenderedReport } from "./generated/rendered-report";
import type { Health } from "./health";
import { useJourneyTexts } from "./journey-texts";
import { LoadingMallet } from "./LoadingMallet";
import { openFileDuels, ScoreRing } from "./Overview";
import { useOverviewTexts } from "./overview-texts";
import { storedReview } from "./Review";
import { useReviewTexts } from "./review-texts";
import "./journey.css";
import "./overview.css";

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

/** Where the start page leads. */
export type StartTarget = "overview" | "review" | "tasks" | "collisions" | "assets" | "patch";

interface AllGamesProps {
  games: Games;
  /** The last scan of the game in use, if any. */
  report: RenderedReport | null;
  /** Its health, if it was scanned. */
  health: Health | null;
  scanning: boolean;
  onScan: () => void;
  /** Goes to a page of the game in use. */
  onOpen: (view: StartTarget) => void;
}

/**
 * The start page, reached by the name in the sidebar (ADR 0048): the game
 * in use at a glance with the way to what there is to do, and every game
 * Mod Mallet knows, to switch to another. The game's own overview says
 * more; this page says enough to choose.
 */
export function AllGames({ games, report, health, scanning, onScan, onOpen }: AllGamesProps) {
  const { t, lang, ignored, decisions } = useSession();
  const j = useJourneyTexts();
  const o = useOverviewTexts();
  const art = useGameArt(t.games.map((game) => game.id));
  const activeId = games.choice?.active;
  const active = t.games.find((game) => game.id === activeId);
  const hero = activeId === undefined ? undefined : art[activeId];
  const known = new Map((games.choice?.games ?? []).map((game) => [game.id, game]));
  const installed = report?.installedPatch?.objects.length;
  const r = useReviewTexts();
  const review = activeId === undefined ? null : storedReview(activeId);

  const tiles: { target: StartTarget; value: string; label: string }[] =
    report === null || health === null
      ? []
      : [
          {
            target: "tasks",
            value: String(openTaskCount(report.findings, ignored)),
            label: j.startTasks,
          },
          { target: "collisions", value: String(health.spots.open), label: j.startSpots },
          {
            target: "assets",
            value: String(openFileDuels(report, decisions)),
            label: j.startAssets,
          },
          {
            target: "patch",
            value: installed === undefined ? "–" : String(installed),
            label: installed === undefined ? j.patchNone : j.startPatch,
          },
        ];

  return (
    <div className="stack">
      <section
        className="panel home-hero"
        style={hero === undefined ? undefined : { backgroundImage: `url(${hero})` }}
      >
        <div className="home-hero-text">
          <span className="brand-tagline">{t.tagline}</span>
          <div className="home-hero-row">
            {health !== null && <ScoreRing score={health.score} />}
            <div className="home-hero-words">
              <h2>{active?.name}</h2>
              {report !== null && (
                <span>
                  {report.game.version ?? t.versionUnknown}
                  {report.game.extenderVersion != null &&
                    ` · ${games.active?.scriptExtender ?? t.scriptExtender} ${report.game.extenderVersion}`}
                  {" · "}
                  {j.lastScanAt(
                    new Date(report.scannedAt * 1000).toLocaleString(lang, {
                      dateStyle: "short",
                      timeStyle: "short",
                    }),
                  )}
                </span>
              )}
              {health !== null && <strong>{o.verdict[health.verdict]}</strong>}
            </div>
          </div>
          <div className="home-hero-actions">
            {report === null ? (
              <button type="button" className="primary" disabled={scanning} onClick={onScan}>
                {scanning && <LoadingMallet size={26} className="loading-inline" />}
                {scanning ? t.scanning : t.scan}
              </button>
            ) : (
              <>
                <button type="button" className="primary" onClick={() => onOpen("review")}>
                  {review === null ? r.start : r.continueAt(r.step[review.step].title)}
                </button>
                <button type="button" onClick={() => onOpen("overview")}>
                  {j.toOverview}
                </button>
              </>
            )}
          </div>
        </div>
      </section>

      {tiles.length > 0 && (
        <div className="home-stats">
          {tiles.map((tile) => (
            <button
              type="button"
              className="panel home-stat"
              key={tile.target}
              onClick={() => onOpen(tile.target)}
            >
              <strong className="home-stat-value">{tile.value}</strong>
              <span className="home-stat-label">{tile.label}</span>
            </button>
          ))}
        </div>
      )}

      <section className="panel">
        <div className="panel-head">
          <h2>{j.allGamesTitle}</h2>
          <span className="aside">{j.allGamesNote}</span>
        </div>
        <ul className="game-grid">
          {t.games.map((game) => {
            const picture = art[game.id];
            const isActive = game.id === activeId;
            const knownGame = known.get(game.id);
            const playable = isPlayable(knownGame);
            const state = isActive
              ? null
              : knownGame === undefined
                ? `${j.soon}${game.quip === undefined ? "" : ` – ${game.quip}`}`
                : !playable
                  ? t.gameNotInstalled
                  : knownGame.support === "basic"
                    ? t.gameBasic
                    : null;
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
                  {isActive && report === null && <span className="aside">{j.notScanned}</span>}
                  {state !== null && <span className="aside">{state}</span>}
                  {playable && !isActive && (
                    <button
                      type="button"
                      className="game-open"
                      onClick={() => {
                        games.select(game.id).catch(() => {});
                      }}
                    >
                      {j.switchTo}
                    </button>
                  )}
                </div>
              </li>
            );
          })}
        </ul>
      </section>
    </div>
  );
}
