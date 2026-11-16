import { useEffect, useState } from "react";
import { useSession } from "./Actions";
import type { Counts } from "./arrange";
import type { RenderedReport } from "./generated/rendered-report";

/** The game Mod Mallet works on; the others are on the list as a promise. */
const ACTIVE_GAME = "skyrimse";

type Target = "tasks" | "collisions" | "mods" | "patch";

interface HomeProps {
  report: RenderedReport | null;
  counts: Counts | null;
  scanning: boolean;
  onScan: () => void;
  onOpen: (view: Target) => void;
}

/**
 * Where the window starts: the game in use with its state at a glance, and
 * the games Mod Mallet knows. Pictures of the games come from the mod
 * manager's installation on this machine; Mod Mallet ships none.
 */
export function Home({ report, counts, scanning, onScan, onOpen }: HomeProps) {
  const { t, api } = useSession();
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

  const active = t.games.find((game) => game.id === ACTIVE_GAME);
  const hero = art[ACTIVE_GAME];
  const installed = report?.installedPatch?.objects.length;
  const stats: { target: Target; value: string; label: string }[] =
    counts === null || report === null
      ? []
      : [
          {
            target: "tasks",
            value: String(counts.errors + counts.warnings),
            label: t.homeOpenTasks,
          },
          { target: "collisions", value: String(counts.collisions), label: t.homeCollisions },
          { target: "mods", value: String(report.mods.length), label: t.homeMods },
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
                ` · ${t.scriptExtender} ${report.game.extenderVersion}`}
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
            const isActive = game.id === ACTIVE_GAME;
            return (
              <li className="game-card" key={game.id} data-active={isActive}>
                <div
                  className="game-art"
                  style={picture === undefined ? undefined : { backgroundImage: `url(${picture})` }}
                  title={picture === undefined ? t.homeNoArt : undefined}
                >
                  {picture === undefined && <span aria-hidden="true">{game.name.slice(0, 1)}</span>}
                </div>
                <div className="game-text">
                  <strong>{game.name}</strong>
                  <span className="aside">
                    {isActive ? t.homeActive : `${t.soon} – ${game.quip ?? ""}`}
                  </span>
                </div>
              </li>
            );
          })}
        </ul>
      </section>
    </div>
  );
}
