import type { ReactNode } from "react";
import { useSession } from "./Actions";
import { useGameArt } from "./AllGames";
import { isTask, openTaskCount } from "./arrange";
import type { Games } from "./games";
import type { RenderedReport } from "./generated/rendered-report";
import type { RenderedToolbox } from "./generated/rendered-toolbox";
import { type Health, type Target, weightOf } from "./health";
import { useJourneyTexts } from "./journey-texts";
import { useOverviewTexts } from "./overview-texts";
import "./journey.css";
import "./overview.css";

/** The pages the overview leads to. */
export type OverviewTarget = Target | "map" | "assets" | "toolbox";

/** The duels about files (textures, meshes, faces), whose home is the assets page. */
export function fileDuels(report: RenderedReport): RenderedReport["duels"] {
  return report.duels.filter((rendered) =>
    rendered.duel.topics.some(({ topic }) => topic !== "objects" && topic !== "contained"),
  );
}

/** How many duels about files have no side picked yet. */
export function openFileDuels(report: RenderedReport, decisions: Record<string, number>): number {
  return fileDuels(report).filter((rendered) => decisions[rendered.duel.id] === undefined).length;
}

interface OverviewProps {
  games: Games;
  report: RenderedReport | null;
  health: Health | null;
  toolbox: RenderedToolbox | null;
  scanning: boolean;
  onScan: () => void;
  onOpen: (view: OverviewTarget) => void;
}

/** How many worldspaces the map's tile names. */
const WORLDS_NAMED = 3;

/**
 * How the game in use stands, and the way to each of its pages (ADR 0048).
 * Nothing is decided here; every tile is the button to its page.
 */
export function Overview({
  games,
  report,
  health,
  toolbox,
  scanning,
  onScan,
  onOpen,
}: OverviewProps) {
  const { t, ignored, decisions, ranking } = useSession();
  const j = useJourneyTexts();
  const activeId = games.choice?.active;
  const art = useGameArt(activeId === undefined ? [] : [activeId]);
  const name = t.games.find((game) => game.id === activeId)?.name ?? games.active?.id ?? "";
  const picture = activeId === undefined ? undefined : art[activeId];

  const banner = (
    <section
      className="panel overview-banner"
      style={picture === undefined ? undefined : { backgroundImage: `url(${picture})` }}
    >
      <div className="overview-banner-text">
        <h2>{name}</h2>
        {report !== null && (
          <span>
            {report.game.version ?? t.versionUnknown}
            {report.game.extenderVersion != null &&
              ` · ${games.active?.scriptExtender ?? t.scriptExtender} ${report.game.extenderVersion}`}
          </span>
        )}
      </div>
    </section>
  );

  if (report === null || health === null) {
    return (
      <div className="stack">
        {banner}
        {!scanning && (
          <section className="panel welcome">
            <p>{j.scanFirst}</p>
            <button type="button" className="primary" onClick={onScan}>
              {t.scan}
            </button>
          </section>
        )}
      </div>
    );
  }

  const open = openTaskCount(report.findings, ignored);
  // Of the things to do, those that can crash the game: counted alike, so
  // that the part is never more than the whole.
  const crashes = report.findings.filter(
    (rendered) => isTask(rendered, ignored) && weightOf(rendered).impact === "crash",
  ).length;
  const installed = report.installedPatch?.objects.length;
  const enabled = report.mods.filter((info) => info.enabled).length;
  const newer = report.mods.filter((info) => info.newerVersion != null).length;
  const worlds = report.map.worlds.map((world) => world.name);
  const attention = toolbox?.attention ?? 0;
  const duels = fileDuels(report).length;
  const openDuels = openFileDuels(report, decisions);
  const spotsTotal =
    health.spots.open + health.spots.kept + health.spots.waiting + health.spots.patched;

  const tiles: {
    target: OverviewTarget;
    title: string;
    value: string;
    lines: ReactNode[];
    tone?: string;
  }[] = [
    {
      target: "tasks",
      title: t.viewTasks,
      value: String(open),
      lines: [
        open === 0 ? j.tasksNone : j.tasksOpen(open),
        crashes > 0 ? j.tasksCrash(crashes) : null,
      ],
      tone: crashes > 0 ? "error" : open > 0 ? "warning" : "ok",
    },
    {
      target: "collisions",
      title: t.viewCollisions,
      value: String(health.spots.open),
      lines: [
        spotsTotal === 0
          ? j.spotsNone
          : health.spots.open === 0
            ? j.spotsAllDecided
            : j.spotsOpen(health.spots.open),
        health.spots.waiting > 0 ? j.spotsWaiting(health.spots.waiting) : null,
      ],
      tone: health.spots.open > 0 ? "warning" : "ok",
    },
    {
      target: "assets",
      title: j.navAssets,
      value: String(openDuels),
      lines: [
        duels === 0 ? j.assetsNone : openDuels === 0 ? j.assetsAllDecided : j.assetsOpen(openDuels),
        ranking.length > 0 ? j.assetsRanked(ranking.length) : null,
      ],
      tone: openDuels > 0 ? "warning" : duels > 0 ? "ok" : undefined,
    },
    {
      target: "patch",
      title: t.viewPatch,
      value: installed === undefined ? "–" : String(installed),
      lines: [
        installed === undefined ? j.patchNone : j.patchHidden(installed),
        health.spots.waiting > 0
          ? j.patchNotCurrent
          : installed === undefined
            ? null
            : j.patchCurrent,
      ],
      tone: health.spots.waiting > 0 ? "warning" : undefined,
    },
    {
      target: "mods",
      title: t.viewMods,
      value: `${enabled}/${report.mods.length}`,
      lines: [j.modsEnabled(enabled, report.mods.length), newer > 0 ? j.modsNewer(newer) : null],
    },
    {
      target: "map",
      title: t.viewMap,
      value: String(worlds.length),
      lines: [
        j.mapWorlds(worlds.length),
        worlds.length === 0
          ? null
          : `${worlds.slice(0, WORLDS_NAMED).join(", ")}${
              worlds.length > WORLDS_NAMED ? ` ${j.mapMore(worlds.length - WORLDS_NAMED)}` : ""
            }`,
      ],
    },
    {
      target: "toolbox",
      title: t.viewToolbox,
      value: toolbox === null ? "–" : String(attention),
      lines: [
        toolbox === null
          ? j.toolsUnknown
          : attention > 0
            ? j.toolsAttention(attention)
            : j.toolsReady,
      ],
      tone: attention > 0 ? "warning" : toolbox === null ? undefined : "ok",
    },
  ];

  return (
    <div className="stack">
      {banner}
      <HealthPanel health={health} onOpen={onOpen} />
      <div className="overview-tiles">
        {tiles.map((tile) => (
          <button
            type="button"
            className="panel overview-tile"
            key={tile.target}
            data-tone={tile.tone}
            onClick={() => onOpen(tile.target)}
          >
            <span className="overview-tile-title">{tile.title}</span>
            <strong className="overview-tile-value">{tile.value}</strong>
            {tile.lines
              .filter((line) => line !== null)
              .map((line, index) => (
                // biome-ignore lint/suspicious/noArrayIndexKey: the lines of a tile never move.
                <span className="overview-tile-line" key={index}>
                  {line}
                </span>
              ))}
          </button>
        ))}
      </div>
      <section className="panel overview-sources">
        <p className="meta">
          {t.scanStats({
            plugins: report.pluginsChecked,
            archives: report.archivesChecked,
            installers: report.installersChecked,
            meshes: report.meshesChecked,
          })}
        </p>
        <details className="sources">
          <summary className="more">{t.whatWasRead}</summary>
          <dl>
            {report.locations.map(({ location, roleLabel, originLabel }) => (
              <div key={location.role}>
                <dt>{roleLabel}</dt>
                <dd>
                  <code>{location.path}</code>
                  <span className="origin">{location.found ? originLabel : t.notFound}</span>
                </dd>
              </div>
            ))}
          </dl>
        </details>
      </section>
    </div>
  );
}

/** How many reasons the overview names. */
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
export function ScoreRing({ score }: { score: number }) {
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
