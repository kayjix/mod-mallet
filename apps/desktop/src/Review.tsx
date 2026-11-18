// The guided review (ADR 0050): the setup step by step, the collisions
// structure by structure. It owns nothing: it shows the homes' own rows
// and writes the same choices the homes write.

import { type ReactNode, useEffect, useMemo, useState } from "react";
import { useSession } from "./Actions";
import { objectKey } from "./area-layer";
import { isCollision, stepOf } from "./arrange";
import {
  CompareStage3D,
  type StageGroup,
  type StagePlace,
  type StageSegment,
} from "./CompareStage3D";
import { DuelLine } from "./Duels";
import type { MapSpot, RenderedFinding, RenderedReport } from "./generated/rendered-report";
import { reportSpots } from "./health";
import { CELL } from "./map-places";
import { fileDuels } from "./Overview";
import { isGameObject } from "./object-actions";
import { chooseObject, chooseSpot, hiddenKeysOf, type PatchChoices, revealObjects } from "./patch";
import { type Cluster, clustersOf, type Decided, decidedOf, openIn } from "./review-clusters";
import { type ReviewStep, useReviewTexts } from "./review-texts";
import { FindingRow } from "./Tasks";
import "./review.css";

/** The steps, in their order. */
export const REVIEW_STEPS: ReviewStep[] = [
  "install",
  "order",
  "assets",
  "map",
  "collisions",
  "report",
];

/** Where the review leads for more than it shows. */
export type ReviewTarget = "tasks" | "mods" | "assets" | "map";

/** Kinds whose findings depend on the order of the plugins. */
const ORDER_KINDS = new Set([
  "plugins.loot-note",
  "plugins.loot-warning",
  "plugins.incompatible",
  "plugins.generated-outdated",
  "plugins.needs-cleaning",
  "plugins.identical-records",
  "plugins.contained",
  "setup.load-order-unknown",
  "navmeshes.changes-undone",
  "navmeshes.door-cut-off",
]);

/** Kinds about whose files the game shows: the assets' step, through their duels. */
const ASSET_KINDS = new Set(["meshes.retexture-bypassed", "npcs.face-mismatch"]);

/** How many rows a step shows before it leads to the page itself. */
const LIMIT = 12;

/** What the review remembers per game: the step the user is on, and the steps looked at. */
interface Stored {
  step: ReviewStep;
  seen: ReviewStep[];
}

const storageKey = (game: string) => `mallet.review.${game}`;

/** What the review remembers of a game, if anything. */
export function storedReview(game: string): Stored | null {
  try {
    const text = localStorage.getItem(storageKey(game));
    if (text === null) {
      return null;
    }
    const value = JSON.parse(text) as Partial<Stored>;
    const step = REVIEW_STEPS.find((candidate) => candidate === value.step);
    const seen = (value.seen ?? []).filter((candidate) => REVIEW_STEPS.includes(candidate));
    return step === undefined ? null : { step, seen };
  } catch {
    return null;
  }
}

function remember(game: string, stored: Stored) {
  try {
    localStorage.setItem(storageKey(game), JSON.stringify(stored));
  } catch {
    // Only a convenience: the decisions themselves are kept elsewhere.
  }
}

/** The open findings of a step, set aside ones and notes left out. */
function findingsOf(
  report: RenderedReport,
  ignored: Set<string>,
  step: "install" | "order",
): RenderedFinding[] {
  return report.findings.filter((rendered) => {
    if (isCollision(rendered) || ASSET_KINDS.has(rendered.finding.kind)) {
      return false;
    }
    const what = stepOf(rendered, ignored);
    if (what === null || what === "notes" || what === "ignored") {
      return false;
    }
    return ORDER_KINDS.has(rendered.finding.kind) === (step === "order");
  });
}

interface ReviewProps {
  report: RenderedReport;
  /** The game in use, to remember the review by. */
  game: string;
  onOpen: (view: ReviewTarget) => void;
}

/**
 * The setup step by step (ADR 0050): what is installed, the load order,
 * the assets, the world map and the collisions, the last decided here
 * place by place, and a report at the end. Where the user was is
 * remembered per game.
 */
export function Review({ report, game, onOpen }: ReviewProps) {
  const { ignored, decisions, patchChoices } = useSession();
  const r = useReviewTexts();
  const [stored, setStored] = useState<Stored>(
    () => storedReview(game) ?? { step: "install", seen: [] },
  );
  useEffect(() => remember(game, stored), [game, stored]);
  // The place the collisions step shows, kept here so that the report can lead there.
  const [clusterAt, setClusterAt] = useState<string | null>(null);
  const index = REVIEW_STEPS.indexOf(stored.step);
  const previous = REVIEW_STEPS[index - 1];
  const following = REVIEW_STEPS[index + 1];

  const install = useMemo(() => findingsOf(report, ignored, "install"), [report, ignored]);
  const order = useMemo(() => findingsOf(report, ignored, "order"), [report, ignored]);
  const duels = useMemo(
    () => fileDuels(report).filter((rendered) => decisions[rendered.duel.id] === undefined),
    [report, decisions],
  );
  const clusters = useMemo(
    () => clustersOf(reportSpots(report, ignored, patchChoices)),
    [report, ignored, patchChoices],
  );
  const openClusters = clusters.filter((cluster) => openIn(cluster) > 0).length;
  const open: Record<ReviewStep, number | null> = {
    install: install.length,
    order: order.length,
    assets: duels.length,
    map: null,
    collisions: openClusters,
    report: null,
  };
  const go = (step: ReviewStep) => setStored({ ...stored, step });
  const next = () => {
    const seen = stored.seen.includes(stored.step) ? stored.seen : [...stored.seen, stored.step];
    setStored({ step: following ?? stored.step, seen });
  };

  return (
    <section className="review">
      <header className="review-head">
        <h2>{r.title}</h2>
        <p className="aside">{r.intro}</p>
      </header>
      <ol className="review-steps">
        {REVIEW_STEPS.map((step, at) => {
          const count = open[step];
          const done = count === 0 || (count === null && stored.seen.includes(step));
          return (
            <li key={step}>
              <button
                type="button"
                className="review-step"
                aria-current={step === stored.step ? "step" : undefined}
                data-done={done}
                onClick={() => go(step)}
              >
                <span className="review-step-number">{done ? "✓" : at + 1}</span>
                <span className="review-step-title">{r.step[step].title}</span>
                {count !== null && count > 0 && (
                  <span className="review-step-count">{r.open(count)}</span>
                )}
              </button>
            </li>
          );
        })}
      </ol>

      <article className="panel review-body">
        <div className="review-body-head">
          <span className="aside">{r.stepOf(index + 1, REVIEW_STEPS.length)}</span>
          <h3>{r.step[stored.step].title}</h3>
          <p className="aside">{r.step[stored.step].hint}</p>
        </div>
        {stored.step === "install" && (
          <InstallStep report={report} findings={install} onOpen={onOpen} />
        )}
        {stored.step === "order" && (
          <FindingsStep findings={order} more={r.toTasks} onMore={() => onOpen("tasks")} />
        )}
        {stored.step === "assets" && (
          <Rows count={duels.length} more={r.toAssets} onMore={() => onOpen("assets")}>
            {duels.slice(0, LIMIT).map((rendered) => (
              <DuelLine key={rendered.duel.id} rendered={rendered} />
            ))}
          </Rows>
        )}
        {stored.step === "map" && <MapStep report={report} onOpen={onOpen} />}
        {stored.step === "collisions" && (
          <CollisionsStep
            report={report}
            clusters={clusters}
            cursor={clusterAt}
            onCursor={setClusterAt}
          />
        )}
        {stored.step === "report" && (
          <ReportStep
            clusters={clusters}
            open={open}
            onCluster={(key) => {
              setClusterAt(key);
              go("collisions");
            }}
            onStep={go}
          />
        )}
        {/* Between the steps; moving within a step is the step's own. */}
        <footer className="review-foot">
          {previous !== undefined && (
            <button type="button" className="quiet" onClick={() => go(previous)}>
              {r.stepBack(r.step[previous].title)}
            </button>
          )}
          <span className="spacer" />
          {following === undefined ? (
            <button type="button" className="quiet" onClick={() => go("install")}>
              {r.restart}
            </button>
          ) : (
            <button type="button" className="primary" onClick={next}>
              {r.stepNext(r.step[following].title)}
            </button>
          )}
        </footer>
      </article>
    </section>
  );
}

/** The first few rows of a step, and the way to the page with all of them. */
function Rows({
  count,
  children,
  more,
  onMore,
}: {
  count: number;
  children: ReactNode;
  more: string;
  onMore: () => void;
}) {
  const r = useReviewTexts();
  return (
    <div className="review-rows">
      {count === 0 ? <p className="aside">{r.nothing}</p> : children}
      <p className="review-more">
        {count > LIMIT && <span className="aside">{r.more(count - LIMIT)}</span>}
        <button type="button" className="small" onClick={onMore}>
          {more} →
        </button>
      </p>
    </div>
  );
}

function FindingsStep({
  findings,
  more,
  onMore,
}: {
  findings: RenderedFinding[];
  more: string;
  onMore: () => void;
}) {
  return (
    <Rows count={findings.length} more={more} onMore={onMore}>
      <ul className="review-findings">
        {findings.slice(0, LIMIT).map((rendered) => (
          <FindingRow key={rendered.finding.id} rendered={rendered} />
        ))}
      </ul>
    </Rows>
  );
}

function InstallStep({
  report,
  findings,
  onOpen,
}: {
  report: RenderedReport;
  findings: RenderedFinding[];
  onOpen: (view: ReviewTarget) => void;
}) {
  const r = useReviewTexts();
  const enabled = report.mods.filter((info) => info.enabled).length;
  const newer = report.mods.filter((info) => info.newerVersion != null).length;
  return (
    <>
      <p className="review-line">
        {r.modsLine(enabled, report.mods.length, newer)}{" "}
        <button type="button" className="small quiet" onClick={() => onOpen("mods")}>
          {r.toMods} →
        </button>
      </p>
      <FindingsStep findings={findings} more={r.toTasks} onMore={() => onOpen("tasks")} />
    </>
  );
}

function MapStep({
  report,
  onOpen,
}: {
  report: RenderedReport;
  onOpen: (view: ReviewTarget) => void;
}) {
  const r = useReviewTexts();
  const worlds = report.map.worlds.filter((world) => world.small !== true);
  return (
    <Rows count={worlds.length} more={r.toMap} onMore={() => onOpen("map")}>
      {worlds.slice(0, LIMIT).map((world) => (
        <div key={world.name} className="review-world">
          <strong>{world.title ?? world.name}</strong>
          {world.title != null && world.title !== world.name && (
            <code className="aside">{world.name}</code>
          )}
          <span className="aside">
            {r.worldLine(
              world.cells.length,
              (world.markers ?? []).filter((marker) => marker.origin !== "game").length,
            )}
          </span>
        </div>
      ))}
    </Rows>
  );
}

/** A plugin's name without its extension, to name a side. */
const short = (plugin: string) => plugin.replace(/\.(esp|esm|esl)$/i, "");

/**
 * Where a spot is, for the stage. A city inside its walls (RiftenWorld)
 * stands on the land of its parent world, whose reading takes in the
 * city's objects too: the stage reads the parent, as the collisions' stage
 * does.
 */
function placeOf(
  spot: MapSpot,
  worlds: RenderedReport["map"]["worlds"],
): { place: StagePlace; centre: [number, number, number] } {
  const { location } = spot;
  if (location.type === "interior") {
    return { place: { type: "interior", cell: location.cell }, centre: spot.position ?? [0, 0, 0] };
  }
  return {
    place: {
      type: "exterior",
      world:
        worlds.find((world) => (world.children ?? []).includes(location.world))?.name ??
        location.world,
    },
    centre: spot.position ?? [(location.x + 0.5) * CELL, (location.y + 0.5) * CELL, 0],
  };
}

/**
 * The collisions, place by place (ADR 0050): a cluster of spots of the
 * same two mods, its structures side by side, and below them its
 * segments, each with what is decided there. One choice decides the whole
 * place; each segment can be decided on its own. Moving goes place by
 * place, decided ones included; deciding moves on to the next open place.
 */
function CollisionsStep({
  report,
  clusters,
  cursor,
  onCursor,
}: {
  report: RenderedReport;
  clusters: Cluster[];
  cursor: string | null;
  onCursor: (key: string) => void;
}) {
  const { ignored, patchChoices, setPatchChoices, spots } = useSession();
  const r = useReviewTexts();
  const [history, setHistory] = useState<PatchChoices[]>([]);
  const [segments, setSegments] = useState<StageSegment[]>([]);
  const [focus, setFocus] = useState<number | null>(null);
  const found = clusters.findIndex((cluster) => cluster.key === cursor);
  const at = Math.max(0, found >= 0 ? found : clusters.findIndex((cluster) => openIn(cluster) > 0));
  const cluster = clusters[at];
  const currentKey = cluster?.key ?? null;
  // biome-ignore lint/correctness/useExhaustiveDependencies: the structures belong to the place.
  useEffect(() => {
    setSegments([]);
    setFocus(null);
  }, [currentKey]);
  const hidden = useMemo(() => hiddenKeysOf(spots, patchChoices), [spots, patchChoices]);
  const decidedPlaces = clusters.filter((candidate) => openIn(candidate) === 0).length;
  const undo = (
    <button
      type="button"
      className="quiet"
      disabled={history.length === 0}
      onClick={() => {
        const previous = history[history.length - 1];
        if (previous !== undefined) {
          setHistory(history.slice(0, -1));
          setPatchChoices(previous);
        }
      }}
    >
      {r.undo}
    </button>
  );

  const first = cluster?.spots[0];
  if (cluster === undefined || first === undefined) {
    return (
      <div className="review-rows">
        <p>{r.allDecided}</p>
        {history.length > 0 && <div className="review-choices">{undo}</div>}
      </div>
    );
  }
  const sides = cluster.plugins;
  const names: [string, string] = [short(sides[0]), short(sides[1])];
  const groups: StageGroup[] = cluster.spots.map(({ item, key }) => {
    const parts = item.spot.parts ?? [];
    return {
      key,
      seeds: [
        parts.filter((part) => part.object.plugin === sides[0]).map((part) => part.object),
        parts.filter((part) => part.object.plugin === sides[1]).map((part) => part.object),
      ],
    };
  });
  const { place } = placeOf(first.item.spot, report.map.worlds);
  const reach = cluster.radius + 2048;
  const radius = Math.min(2 * CELL, Math.max(CELL, cluster.radius + 2048));
  const bySpot = new Map(cluster.spots.map((spot) => [spot.key, spot]));
  // Until the stage has found them, every spot is a segment of its own.
  const shownSegments: StageSegment[] =
    segments.length > 0
      ? segments
      : cluster.spots.map(({ key }) => ({ groups: [key], parts: [[], []] }));
  const spotsOf = (segment: StageSegment) =>
    segment.groups.flatMap((key) => {
      const spot = bySpot.get(key);
      return spot === undefined ? [] : [spot];
    });
  const describe = (decided: Decided) =>
    decided.type === "open"
      ? r.stateOpen
      : decided.type === "kept"
        ? r.stateKept
        : decided.type === "hidden"
          ? r.stateHidden(short(decided.plugin))
          : r.stateMixed(decided.open);
  const keptSideOf = (decided: Decided) =>
    decided.type !== "hidden"
      ? null
      : decided.plugin.toLowerCase() === sides[1].toLowerCase()
        ? 0
        : 1;
  const placeDecided = decidedOf(cluster.spots, patchChoices);

  /** A side's parts in `chosen`, or the spots' own parts until the structures are known. */
  const partsOf = (chosen: StageSegment[], side: 0 | 1) => {
    const known = chosen.flatMap((segment) => segment.parts[side]);
    if (known.length > 0) {
      return known.map((object) => ({ id: object.id, game: isGameObject(object) }));
    }
    return chosen.flatMap((segment) =>
      spotsOf(segment).flatMap(({ item }) =>
        (item.spot.parts ?? [])
          .filter((part) => part.object.plugin === sides[side])
          .map((part) => ({ id: part.object, game: !part.hideable })),
      ),
    );
  };
  /** Keeps the choices; deciding the whole place moves on to the next open one. */
  const apply = (next: PatchChoices, moveOn: boolean) => {
    setHistory([...history, patchChoices]);
    setPatchChoices(next);
    if (!moveOn) {
      return;
    }
    const after = clustersOf(reportSpots(report, ignored, next));
    for (let step = 1; step <= clusters.length; step++) {
      const candidate = clusters[(at + step) % clusters.length];
      const now = after.find((other) => other.key === candidate?.key);
      if (candidate !== undefined && now !== undefined && openIn(now) > 0) {
        onCursor(candidate.key);
        return;
      }
    }
  };
  const keep = (side: 0 | 1, chosen: StageSegment[], moveOn: boolean) => {
    const loser = sides[side === 0 ? 1 : 0];
    let next = revealObjects(
      patchChoices,
      spots,
      partsOf(chosen, side).map((part) => part.id),
    );
    for (const part of partsOf(chosen, side === 0 ? 1 : 0).filter((part) => !part.game)) {
      next = chooseObject(next, part.id, true);
    }
    // Every spot of the segments is decided, also where the losing side's
    // parts there are the game's own and stay.
    for (const segment of chosen) {
      for (const { key } of spotsOf(segment)) {
        next = chooseSpot(next, key, loser);
      }
    }
    apply(next, moveOn);
  };
  const keepBoth = (chosen: StageSegment[], moveOn: boolean) => {
    const both = [...partsOf(chosen, 0), ...partsOf(chosen, 1)].map((part) => part.id);
    let next = revealObjects(patchChoices, spots, both);
    for (const segment of chosen) {
      for (const { key } of spotsOf(segment)) {
        next = chooseSpot(next, key, null);
      }
    }
    apply(next, moveOn);
  };
  const focused = focus === null ? null : shownSegments[focus];
  const focusKeys =
    focused == null
      ? null
      : [...focused.parts[0], ...focused.parts[1]].map((part) =>
          objectKey(part.id.plugin, part.id.localId),
        );
  const gameParts = [...partsOf(shownSegments, 0), ...partsOf(shownSegments, 1)].filter(
    (part) => part.game,
  ).length;
  const placeKept = keptSideOf(placeDecided);

  return (
    <div className="review-collision">
      <nav className="review-structures" aria-label={r.clusterOf(at + 1, clusters.length)}>
        <button
          type="button"
          className="small"
          onClick={() =>
            onCursor(clusters[(at - 1 + clusters.length) % clusters.length]?.key ?? "")
          }
        >
          {r.previousStructure}
        </button>
        <span className="review-structures-where">
          <strong>{r.clusterOf(at + 1, clusters.length)}</strong>
          <span className="aside">{r.reportCollisions(decidedPlaces, clusters.length)}</span>
        </span>
        <button
          type="button"
          className="small"
          onClick={() => onCursor(clusters[(at + 1) % clusters.length]?.key ?? "")}
        >
          {r.nextStructure}
        </button>
      </nav>
      {decidedPlaces === clusters.length && <p className="aside">{r.allDecided}</p>}
      <div className="review-collision-head">
        <strong>
          {names[0]} ⟷ {names[1]}
        </strong>
        <span className="aside">
          {r.clusterSpots(cluster.spots.length, shownSegments.length)} ·{" "}
          <code>{first.item.spot.console}</code>
        </span>
        <span className="spacer" />
        <span className="review-state" data-decided={placeDecided.type !== "open"}>
          {describe(placeDecided)}
        </span>
      </div>
      <CompareStage3D
        place={place}
        centre={cluster.centre}
        groups={groups}
        reach={reach}
        radius={radius}
        focus={focusKeys}
        names={names}
        hidden={hidden}
        onSegments={setSegments}
      />
      {gameParts > 0 && <p className="aside">{r.gameParts(gameParts)}</p>}
      <div className="review-choices">
        <button
          type="button"
          className="primary"
          aria-pressed={placeKept === 0}
          onClick={() => keep(0, shownSegments, true)}
        >
          {placeKept === 0 ? "✓ " : ""}
          {r.keepHere(names[0])}
        </button>
        <button
          type="button"
          className="primary"
          aria-pressed={placeKept === 1}
          onClick={() => keep(1, shownSegments, true)}
        >
          {placeKept === 1 ? "✓ " : ""}
          {r.keepHere(names[1])}
        </button>
        <button
          type="button"
          aria-pressed={placeDecided.type === "kept"}
          onClick={() => keepBoth(shownSegments, true)}
        >
          {placeDecided.type === "kept" ? "✓ " : ""}
          {r.keepBothHere}
        </button>
        <span className="spacer" />
        {undo}
      </div>
      <section className="review-segments">
        <h4>{r.segmentsTitle}</h4>
        <ol>
          {shownSegments.map((segment, index) => {
            const decided = decidedOf(spotsOf(segment), patchChoices);
            const kept = keptSideOf(decided);
            return (
              <li
                key={segment.groups.join(",")}
                className="review-segment"
                data-focus={focus === index}
              >
                <span className="review-segment-name">
                  {r.segmentName(index + 1, segment.groups.length)}
                </span>
                <span className="aside">
                  {r.segmentParts(
                    names[0],
                    segment.parts[0].length,
                    names[1],
                    segment.parts[1].length,
                  )}
                </span>
                <span className="review-state" data-decided={decided.type !== "open"}>
                  {describe(decided)}
                </span>
                <span className="spacer" />
                <button
                  type="button"
                  className="small quiet"
                  aria-pressed={focus === index}
                  onClick={() => setFocus(focus === index ? null : index)}
                >
                  {r.showSegment}
                </button>
                <button
                  type="button"
                  className="small"
                  aria-pressed={kept === 0}
                  onClick={() => keep(0, [segment], false)}
                >
                  {names[0]}
                </button>
                <button
                  type="button"
                  className="small"
                  aria-pressed={kept === 1}
                  onClick={() => keep(1, [segment], false)}
                >
                  {names[1]}
                </button>
                <button
                  type="button"
                  className="small"
                  aria-pressed={decided.type === "kept"}
                  onClick={() => keepBoth([segment], false)}
                >
                  {r.keepBoth}
                </button>
              </li>
            );
          })}
        </ol>
      </section>
    </div>
  );
}

/**
 * The report at the end (ADR 0050): the places decided and how, the mods
 * involved, and what is left open, in this step and in the others.
 */
function ReportStep({
  clusters,
  open,
  onCluster,
  onStep,
}: {
  clusters: Cluster[];
  open: Record<ReviewStep, number | null>;
  onCluster: (key: string) => void;
  onStep: (step: ReviewStep) => void;
}) {
  const { patchChoices } = useSession();
  const r = useReviewTexts();
  const decided = clusters.filter((cluster) => openIn(cluster) === 0);
  const pending = clusters.filter((cluster) => openIn(cluster) > 0);
  const mods = new Map<string, number>();
  for (const cluster of clusters) {
    for (const plugin of cluster.plugins) {
      mods.set(short(plugin), (mods.get(short(plugin)) ?? 0) + 1);
    }
  }
  const line = (cluster: Cluster) => {
    const state = decidedOf(cluster.spots, patchChoices);
    const what =
      state.type === "open"
        ? r.stateOpen
        : state.type === "kept"
          ? r.stateKept
          : state.type === "hidden"
            ? r.stateHidden(short(state.plugin))
            : r.stateMixed(state.open);
    return (
      <li key={cluster.key} className="review-report-line">
        <strong>
          {short(cluster.plugins[0])} ⟷ {short(cluster.plugins[1])}
        </strong>
        <code className="aside">{cluster.spots[0]?.item.spot.console}</code>
        <span className="aside">{cluster.spots.length}×</span>
        <span className="review-state" data-decided={state.type !== "open"}>
          {what}
        </span>
        <span className="spacer" />
        <button type="button" className="small quiet" onClick={() => onCluster(cluster.key)}>
          {r.goThere}
        </button>
      </li>
    );
  };
  const steps: ReviewStep[] = ["install", "order", "assets"];
  return (
    <div className="review-report">
      <p>
        <strong>{r.reportCollisions(decided.length, clusters.length)}</strong>
      </p>
      {pending.length > 0 && (
        <section>
          <h4>{r.reportOpenTitle}</h4>
          <ul className="review-report-list">{pending.map(line)}</ul>
        </section>
      )}
      {decided.length > 0 && (
        <section>
          <h4>{r.reportDecidedTitle}</h4>
          <ul className="review-report-list">{decided.map(line)}</ul>
        </section>
      )}
      {mods.size > 0 && (
        <section>
          <h4>{r.reportModsTitle}</h4>
          <p className="aside">
            {[...mods.entries()]
              .sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]))
              .map(([name, count]) => `${name} (${count})`)
              .join(" · ")}
          </p>
        </section>
      )}
      <section>
        <h4>{r.reportOtherSteps}</h4>
        <ul className="review-report-list">
          {steps.map((step) => (
            <li key={step} className="review-report-line">
              <span>{r.reportLine(r.step[step].title, open[step] ?? 0)}</span>
              <span className="spacer" />
              {(open[step] ?? 0) > 0 && (
                <button type="button" className="small quiet" onClick={() => onStep(step)}>
                  {r.goThere}
                </button>
              )}
            </li>
          ))}
        </ul>
      </section>
    </div>
  );
}
