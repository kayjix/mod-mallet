import { useEffect, useMemo, useState } from "react";
import { useSession } from "./Actions";
import { type Counts, openTaskCount } from "./arrange";
import { type Games, isPlayable } from "./games";
import type { RenderedReport } from "./generated/rendered-report";
import { type Health, healthOf, type Target } from "./health";
import { useOverviewTexts } from "./overview-texts";
import "./overview.css";

interface HomeProps {
  games: Games;
  report: RenderedReport | null;
  counts: Counts | null;
  scanning: boolean;
  onScan: () => void;
  onOpen: (view: Target) => void;
}

/**
 * Where the window starts: the game in use with its state at a glance, and
 * the games Mod Mallet knows, with a button for each one that is installed. Pictures of the games come from the mod
 * manager's installation on this machine; Mod Mallet ships none.
 */
export function Home({ games, report, scanning, onScan, onOpen }: HomeProps) {
  const { t, api, ignored, patchChoices } = useSession();
  const o = useOverviewTexts();
  const [art, setArt] = useState<Record<string, string>>({});

  useEffect(() => {
    let outdated = false;
    for (const game of t.games) {
      api
        .gameArt(game.id)
        .then((picture) => {
          if (!outdated && picture !== null) {
            setArt((known) => ({ ...known, [game.id]: picture }));
          }
        })
        .catch(() => {});
    }
    return () => {
      outdated = true;
    };
  }, [api, t.games]);

  const activeId = games.choice?.active;
  const active = t.games.find((game) => game.id === activeId);
  const hero = activeId === undefined ? undefined : art[activeId];
  const known = new Map((games.choice?.games ?? []).map((game) => [game.id, game]));
  const installed = report?.installedPatch?.objects.length;
  const health = useMemo(
    () => (report === null ? null : healthOf(report, ignored, patchChoices)),
    [report, ignored, patchChoices],
  );
  const stats: { target: Target; value: string; label: string }[] =
    report === null || health === null
      ? []
      : [
          {
            target: "tasks",
            value: String(openTaskCount(report.findings, ignored)),
            label: o.statTasks,
          },
          { target: "collisions", value: String(health.spots.open), label: o.statSpots },
          {
            target: "mods",
            value: `${report.mods.filter((info) => info.enabled).length} / ${report.mods.length}`,
            label: o.statMods,
          },
          {
            target: "patch",
            value: installed === undefined ? "–" : String(installed),
            label: installed === undefined ? t.homePatchNone : t.homePatch,
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
          <h2>{active?.name}</h2>
          {report !== null && (
            <span>
              {report.game.version ?? t.versionUnknown}
              {report.game.extenderVersion != null &&
                ` · ${games.active?.scriptExtender ?? t.scriptExtender} ${report.game.extenderVersion}`}
            </span>
          )}
          <div className="home-hero-actions">
            <button type="button" className="primary" disabled={scanning} onClick={onScan}>
              {scanning ? t.scanning : t.scan}
            </button>
            {report !== null && (
              <button type="button" onClick={() => onOpen("tasks")}>
                {t.homeToTasks}
              </button>
            )}
          </div>
        </div>
      </section>

      {health === null ? (
        <section className="panel health-empty">
          <p className="aside">{o.noScanYet}</p>
        </section>
      ) : (
        <HealthPanel health={health} onOpen={onOpen} />
      )}

      {stats.length > 0 && (
        <div className="home-stats">
          {stats.map((stat) => (
            <button
              type="button"
              className="panel home-stat"
              key={stat.target}
              onClick={() => onOpen(stat.target)}
            >
              <strong className="home-stat-value">{stat.value}</strong>
              <span className="home-stat-label">{stat.label}</span>
            </button>
          ))}
        </div>
      )}

      <section className="panel">
        <div className="panel-head">
          <h2>{t.homeGames}</h2>
        </div>
        <ul className="game-grid">
          {t.games.map((game) => {
            const picture = art[game.id];
            const isActive = game.id === activeId;
            const knownGame = known.get(game.id);
            const playable = isPlayable(knownGame);
            const state = isActive
              ? t.homeActive
              : knownGame === undefined
                ? `${t.soon} – ${game.quip ?? ""}`
                : playable
                  ? knownGame.support === "basic"
                    ? t.gameBasic
                    : ""
                  : t.gameNotInstalled;
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
                  {state !== "" && <span className="aside">{state}</span>}
                  {playable && !isActive && (
                    <button
                      type="button"
                      className="game-switch"
                      onClick={() => {
                        games.select(game.id).catch(() => {});
                      }}
                    >
                      {t.gameSwitch}
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

/** How many reasons the home page names. */
const REASON_LIMIT = 4;

/**
 * The health of the setup: a verdict in words, a score, and the few
 * reasons that lower it most, each with the way to where it is dealt with.
 */
function HealthPanel({ health, onOpen }: { health: Health; onOpen: (view: Target) => void }) {
  const o = useOverviewTexts();
  const reasons = health.reasons.slice(0, REASON_LIMIT);
  const notes = [
    health.incomplete ? o.incomplete : null,
    health.setAside > 0 ? o.setAside(health.setAside) : null,
    health.advice > 0 ? o.adviceNotCounted(health.advice) : null,
  ].filter((note): note is string => note !== null);

  return (
    <section className="panel health" data-verdict={health.verdict}>
      <div className="health-verdict">
        <ScoreRing score={health.score} />
        <div className="health-words">
          <span className="aside">{o.healthTitle}</span>
          <h2>{o.verdict[health.verdict]}</h2>
          <p>{o.verdictNote[health.verdict]}</p>
        </div>
      </div>
      <div className="health-reasons">
        <h3>{o.reasonsTitle}</h3>
        {reasons.length === 0 ? (
          <p className="aside">{o.nothingLowers}</p>
        ) : (
          <ul>
            {reasons.map((reason) => (
              <li key={reason.key} className="health-reason" data-impact={reason.impact}>
                <span className="impact-chip" data-impact={reason.impact}>
                  {o.impact[reason.impact]}
                </span>
                <span className="health-reason-text">
                  {reason.kind === "open-spots"
                    ? o.reasonOpenSpots(reason.count)
                    : reason.kind === "waiting-spots"
                      ? o.reasonWaitingSpots(reason.count)
                      : o.reasonFindings(reason.title, reason.count)}
                </span>
                <span className="health-points">{o.points(Math.round(reason.points))}</span>
                <button type="button" className="small" onClick={() => onOpen(reason.target)}>
                  {o.go[reason.target]}
                </button>
              </li>
            ))}
          </ul>
        )}
        {notes.length > 0 && (
          <p className="aside health-notes">
            {notes.map((note) => (
              <span key={note}>{note}</span>
            ))}
          </p>
        )}
        <details className="health-how">
          <summary className="more">{o.howCounted}</summary>
          <p className="aside">{o.howCountedText}</p>
        </details>
      </div>
    </section>
  );
}

/** The score as a ring that fills up to it. */
function ScoreRing({ score }: { score: number }) {
  const o = useOverviewTexts();
  const radius = 42;
  const length = 2 * Math.PI * radius;
  return (
    <div className="score-ring" role="img" aria-label={`${score} ${o.scoreOf}`}>
      <svg viewBox="0 0 100 100" aria-hidden="true">
        <circle className="score-track" cx="50" cy="50" r={radius} />
        <circle
          className="score-fill"
          cx="50"
          cy="50"
          r={radius}
          strokeDasharray={`${(length * score) / 100} ${length}`}
          transform="rotate(-90 50 50)"
        />
      </svg>
      <span className="score-value">
        <strong>{score}</strong>
        <span>{o.scoreOf}</span>
      </span>
    </div>
  );
}
