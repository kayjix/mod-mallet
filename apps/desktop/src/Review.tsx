// The guided review (ADR 0050): the setup step by step, the collisions
// structure by structure. It owns nothing: it shows the homes' own rows
// and writes the same choices the homes write.

import { type ReactNode, useEffect, useMemo, useRef, useState } from "react";
import { useSession } from "./Actions";
import { AreaLayer, isGamePlugin, LOOK, objectKey } from "./area-layer";
import { isCollision, stepOf } from "./arrange";
import {
  CompareStage3D,
  type StageGroup,
  type StagePlace,
  type StageSegment,
} from "./CompareStage3D";
import { DuelLine } from "./Duels";
import { DoorPanel, mergeChoices } from "./doors";
import type { AreaObject } from "./generated/area-objects";
import type { ObjectAnchors } from "./generated/object-anchors";
import type {
  DoorMerge,
  MapSpot,
  RenderedFinding,
  RenderedReport,
} from "./generated/rendered-report";
import { reportSpots } from "./health";
import { CELL } from "./map-places";
import { fileDuels } from "./Overview";
import { AnchorReasons, isGameObject } from "./object-actions";
import { PairReport } from "./PairReport";
import { counted, rememberObjects } from "./pair-report";
import { usePassageTexts } from "./passage-texts";
import { PassageSummary } from "./passages";
import {
  chooseObject,
  chooseSpot,
  freeObjects,
  hiddenKeysOf,
  matchKey,
  type PatchChoices,
  pinsOf,
  revealObjects,
} from "./patch";
import type { PathsOverlay } from "./paths-layer";
import { PinButton, PinIcon, usePinTexts } from "./pins";
import { ReachNote, useReach } from "./reach";
import { RepairPaths } from "./repair-paths";
import {
  type Cluster,
  collisionClusters,
  type Decided,
  decidedOf,
  openIn,
} from "./review-clusters";
import { type ReviewStep, useReviewTexts } from "./review-texts";
import { findSegments } from "./segments";
import { type Suggestion, suggestionOf, suggestionOfPassage } from "./suggest";
import { FindingRow } from "./Tasks";
import {
  PathIcon,
  PathMark,
  useWalkedOn,
  useWalkTexts,
  type WalkQuery,
  walkedOn,
} from "./walkways";
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
  // The segments of the places measured so far, by place, for the suggestions by size.
  const [survey, setSurvey] = useState<Map<string, StageSegment[]>>(new Map());
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
    () => collisionClusters(reportSpots(report, ignored, patchChoices), report.map.passages ?? []),
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
            game={game}
            survey={survey}
            onSurvey={(key, found) => setSurvey((before) => new Map(before).set(key, found))}
            clusters={clusters}
            cursor={clusterAt}
            onCursor={setClusterAt}
          />
        )}
        {stored.step === "report" && (
          <ReportStep
            report={report}
            game={game}
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
export function CollisionsStep({
  report,
  game,
  survey,
  onSurvey,
  clusters,
  cursor,
  onCursor,
}: {
  report: RenderedReport;
  game: string;
  survey: Map<string, StageSegment[]>;
  onSurvey: (key: string, segments: StageSegment[]) => void;
  clusters: Cluster[];
  cursor: string | null;
  onCursor: (key: string) => void;
}) {
  const { api, lang, ignored, patchChoices, setPatchChoices, spots } = useSession();
  const r = useReviewTexts();
  const p = usePassageTexts();
  const w = useWalkTexts();
  const [history, setHistory] = useState<PatchChoices[]>([]);
  // The places the bulk left open because characters walk on what would go.
  const [walkedPlaces, setWalkedPlaces] = useState<string[]>([]);
  // The check of every place in the background: how far, and whether to stop.
  const [measuring, setMeasuring] = useState<{ done: number; total: number } | null>(null);
  const stopMeasuring = useRef(false);
  const [segments, setSegments] = useState<StageSegment[]>([]);
  // The repair of the paths over what a choice would hide, shown on the
  // stage (ADR 0058).
  const [repairShown, setRepairShown] = useState<PathsOverlay | null>(null);
  // Why the game may rely on the parts of the place (ADR 0046), by key, for
  // the parts asked last; null while asking.
  const [anchors, setAnchors] = useState<{
    key: string;
    byPart: Map<string, ObjectAnchors>;
  } | null>(null);
  const [focus, setFocus] = useState<number | null>(null);
  // What the last click in the views did.
  const [note, setNote] = useState<string | null>(null);
  const found = clusters.findIndex((cluster) => cluster.key === cursor);
  const at = Math.max(0, found >= 0 ? found : clusters.findIndex((cluster) => openIn(cluster) > 0));
  const cluster = clusters[at];
  const currentKey = cluster?.key ?? null;
  // biome-ignore lint/correctness/useExhaustiveDependencies: the structures belong to the place.
  useEffect(() => {
    setSegments([]);
    setFocus(null);
    setNote(null);
  }, [currentKey]);
  const hidden = useMemo(() => hiddenKeysOf(spots, patchChoices), [spots, patchChoices]);
  // The parts of the place that a decision might hide, to ask the game's ties of.
  const asking = cluster === undefined ? null : partsToAsk(cluster, segments);
  const askingKey = asking === null ? "" : `${asking.place}|${asking.keys.join(",")}`;
  // biome-ignore lint/correctness/useExhaustiveDependencies: the parts are compared by their keys.
  useEffect(() => {
    if (asking === null || asking.refs.length === 0) {
      setAnchors({ key: askingKey, byPart: new Map() });
      return;
    }
    let outdated = false;
    setAnchors(null);
    api
      .objectAnchors(asking.place, asking.refs)
      .then((found) => {
        if (!outdated) {
          setAnchors({
            key: askingKey,
            byPart: new Map(
              (found ?? []).map((one) => [objectKey(one.object.plugin, one.object.localId), one]),
            ),
          });
        }
      })
      .catch(() => {
        if (!outdated) {
          setAnchors({ key: askingKey, byPart: new Map() });
        }
      });
    return () => {
      outdated = true;
    };
  }, [api, askingKey]);
  // The paths characters walk on over the same parts (ADR 0051), asked as
  // the stage reads the place: outside in the parent worldspace, near the
  // place's middle.
  const stagePlace =
    cluster?.spots[0] === undefined ? null : placeOf(cluster.spots[0].item.spot, report.map.worlds);
  const walked = useWalkedOn(
    askingKey,
    stagePlace === null
      ? null
      : stagePlace.place.type === "exterior"
        ? stagePlace.place.world
        : stagePlace.place.cell,
    stagePlace === null || stagePlace.place.type === "interior" || cluster === undefined
      ? null
      : [cluster.centre[0], cluster.centre[1]],
    asking?.queries ?? [],
  );
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
  const groups = groupsOf(cluster);
  const { place } = placeOf(first.item.spot, report.map.worlds);
  const reach = cluster.radius + 2048;
  const radius = Math.min(2 * CELL, Math.max(CELL, cluster.radius + 2048));
  const bySpot = new Map(cluster.spots.map((spot) => [spot.key, spot]));
  // Until the stage has found them, every spot is a segment of its own.
  const shownSegments: StageSegment[] =
    segments.length > 0
      ? segments
      : cluster.spots.map(({ key }) => ({
          groups: [key],
          parts: [[], []],
          volumes: [0, 0],
          decor: [],
        }));
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
  const partsOf = (chosen: StageSegment[], side: 0 | 1): Part[] => {
    const known = chosen.flatMap((segment) => segment.parts[side]);
    if (known.length > 0) {
      return known.map((object) => ({
        id: object.id,
        game: isGameObject(object) && isGamePlugin(object.plugin),
        name: object.name ?? object.model.split(/[\\/]/).pop() ?? object.model,
      }));
    }
    return chosen.flatMap((segment) =>
      spotsOf(segment).flatMap(({ item }) =>
        (item.spot.parts ?? [])
          .filter((part) => part.object.plugin === sides[side])
          .map((part) => ({
            id: part.object,
            game: isGameObject({ id: part.object }),
            name: part.name,
            hideable: part.hideable,
          })),
      ),
    );
  };
  /**
   * What becomes of a part the patch would hide: it goes; it is tied into
   * the game and goes only when the user says so (careful); the game needs
   * it (locked); or it is the game's own. Unknown while the ties are asked.
   */
  const fateOf = (part: Part): Fate => {
    if (part.game) {
      return "game";
    }
    const key = objectKey(part.id.plugin, part.id.localId);
    const tie = anchors?.byPart.get(key);
    if (tie?.verdict === "locked" || (tie === undefined && part.hideable === false)) {
      return "locked";
    }
    if ((walked?.get(key) ?? 0) > 0) {
      return "walked";
    }
    return tie?.verdict === "careful" ? "careful" : "go";
  };
  /** What keeping `side` in `chosen` does to the other side's parts. */
  const effectOf = (side: 0 | 1, chosen: StageSegment[]) => {
    const parts = partsOf(chosen, side === 0 ? 1 : 0);
    const by = (fate: Fate) => parts.filter((part) => fateOf(part) === fate);
    return {
      parts,
      go: by("go"),
      careful: by("careful"),
      walked: by("walked"),
      locked: by("locked"),
      game: by("game"),
    };
  };
  /** Keeps the choices; deciding the whole place moves on to the next open one. */
  const apply = (next: PatchChoices, moveOn: boolean) => {
    setHistory([...history, patchChoices]);
    setPatchChoices(next);
    if (!moveOn) {
      return;
    }
    const after = collisionClusters(reportSpots(report, ignored, next), report.map.passages ?? []);
    for (let step = 1; step <= clusters.length; step++) {
      const candidate = clusters[(at + step) % clusters.length];
      const now = after.find((other) => other.key === candidate?.key);
      if (candidate !== undefined && now !== undefined && openIn(now) > 0) {
        onCursor(candidate.key);
        return;
      }
    }
  };
  /**
   * Keeps `side` in `chosen`: the other side's parts that may go are
   * hidden, the careful ones and those characters walk on only when
   * `anyway`; what the game needs stays.
   */
  const keep = (side: 0 | 1, chosen: StageSegment[], moveOn: boolean, anyway = false) => {
    const loser = sides[side === 0 ? 1 : 0];
    const effect = effectOf(side, chosen);
    let next = revealObjects(
      patchChoices,
      spots,
      partsOf(chosen, side).map((part) => part.id),
    );
    for (const part of anyway ? [...effect.go, ...effect.careful, ...effect.walked] : effect.go) {
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
  // The suggested side of this place, by size (ADR 0050): from the stage's
  // segments, else from the check of every place.
  const measured = segments.length > 0 ? segments : (survey.get(cluster.key) ?? []);
  const suggested =
    measured.length === 0 && cluster.passage !== undefined
      ? suggestionOfPassage(cluster.passage.passage)
      : suggestionOf(measured);
  const roomText = (cubic: number) =>
    cubic < 10
      ? cubic.toLocaleString(lang, { maximumFractionDigits: cubic < 1 ? 2 : 1 })
      : Math.round(cubic).toLocaleString(lang);
  const suggestReason = (suggestion: Suggestion) => {
    const winner = suggestion.side;
    const loser = winner === 0 ? 1 : 0;
    return r.suggestReason(
      names[winner],
      suggestion.parts[winner],
      roomText(suggestion.volumes[winner]),
      names[loser],
      suggestion.parts[loser],
      roomText(suggestion.volumes[loser]),
      suggestion.single,
    );
  };
  /** The open places with a clear suggestion from the check of every place. */
  const clear = clusters.filter(
    (candidate) => openIn(candidate) > 0 && suggestionOf(survey.get(candidate.key) ?? []) !== null,
  );
  const unmeasured = clusters.filter(
    (candidate) => openIn(candidate) > 0 && !survey.has(candidate.key),
  ).length;
  /** Measures every open place not measured yet, in the background, one after the other. */
  const measureAll = async () => {
    const todo = clusters.filter(
      (candidate) => openIn(candidate) > 0 && !survey.has(candidate.key),
    );
    const gl = document.createElement("canvas").getContext("webgl2");
    if (gl === null || todo.length === 0) {
      return;
    }
    stopMeasuring.current = false;
    setMeasuring({ done: 0, total: todo.length });
    const layer = new AreaLayer(gl, api, () => {});
    try {
      for (const [index, candidate] of todo.entries()) {
        if (stopMeasuring.current) {
          break;
        }
        const found = await measure(layer, candidate);
        if (found !== null) {
          onSurvey(candidate.key, found);
        }
        setMeasuring({ done: index + 1, total: todo.length });
      }
    } finally {
      layer.dispose();
      setMeasuring(null);
    }
  };
  /** One place measured: its area read, its models read, its segments found. */
  const measure = async (layer: AreaLayer, candidate: Cluster) => {
    const spot = candidate.spots[0]?.item.spot;
    if (spot === undefined) {
      return null;
    }
    const where = placeOf(spot, report.map.worlds).place;
    const reachHere = candidate.radius + 2048;
    const radiusHere = Math.min(2 * CELL, Math.max(CELL, candidate.radius + 2048));
    const area = await (where.type === "exterior"
      ? api.areaObjects(where.world, candidate.centre[0], candidate.centre[1], radiusHere)
      : api.interiorObjects(where.cell)
    ).catch(() => null);
    if (area === null) {
      return null;
    }
    layer.show(area, () => LOOK.plain);
    // The models arrive one after the other; at most twenty seconds per place.
    for (let wait = 0; wait < 200; wait++) {
      const progress = layer.progress;
      if (progress.done === progress.total || stopMeasuring.current) {
        break;
      }
      await new Promise((resolve) => window.setTimeout(resolve, 100));
    }
    return findSegments(
      layer,
      area,
      groupsOf(candidate),
      candidate.centre,
      reachHere,
      candidate.passage?.keys ?? null,
    );
  };
  /**
   * Takes every clear suggestion at once, as honestly as the keep buttons:
   * of the side given up only the parts that may go are hidden; what is
   * tied in or needed stays; a place where nothing can go stays open. One
   * undo takes the whole batch back.
   */
  const takeAll = async () => {
    let next = patchChoices;
    let taken = 0;
    let stuck = 0;
    let asking = 0;
    const onWalkways: string[] = [];
    for (const candidate of clear) {
      const found = survey.get(candidate.key) ?? [];
      const suggestion = suggestionOf(found);
      const spot = candidate.spots[0]?.item.spot;
      if (suggestion === null || spot === undefined) {
        continue;
      }
      const losingSide = suggestion.side === 0 ? 1 : 0;
      const loser = candidate.plugins[losingSide];
      const losing = found
        .flatMap((segment) => segment.parts[losingSide])
        .filter((object) => !isGameObject(object) || !isGamePlugin(object.plugin));
      const keeping = found.flatMap((segment) => segment.parts[suggestion.side]);
      const notHideable = new Set(
        candidate.spots.flatMap(({ item }) =>
          (item.spot.parts ?? [])
            .filter((part) => !part.hideable)
            .map((part) => objectKey(part.object.plugin, part.object.localId)),
        ),
      );
      const place = spot.location.type === "interior" ? spot.location.cell : spot.location.world;
      const ties =
        losing.length === 0
          ? []
          : ((await api
              .objectAnchors(
                place,
                losing.map((object) => ({
                  plugin: object.id.plugin,
                  localId: object.id.localId,
                })),
              )
              .catch(() => null)) ?? []);
      const byKey = new Map(
        ties.map((tie) => [objectKey(tie.object.plugin, tie.object.localId), tie]),
      );
      const go = losing.filter((object) => {
        const key = objectKey(object.id.plugin, object.id.localId);
        const tie = byKey.get(key);
        return tie === undefined ? !notHideable.has(key) : tie.verdict === "free";
      });
      // Where characters walk on what would go, the place is the user's
      // to decide (ADR 0051).
      const where = placeOf(spot, report.map.worlds).place;
      const paths = await walkedOn(
        api,
        where.type === "exterior" ? where.world : where.cell,
        where.type === "exterior" ? [candidate.centre[0], candidate.centre[1]] : null,
        go.map((object) => ({ id: object.id, position: object.position })),
      ).catch(() => new Map<string, number>());
      if (paths.size > 0) {
        onWalkways.push(candidate.key);
        continue;
      }
      if (go.length === 0) {
        // Tied-in parts are hidden only when the user says so, place by place.
        const tied = losing.some(
          (object) =>
            byKey.get(objectKey(object.id.plugin, object.id.localId))?.verdict === "careful",
        );
        if (tied) {
          asking += 1;
        } else {
          stuck += 1;
        }
        continue;
      }
      next = revealObjects(
        next,
        spots,
        keeping.map((object) => object.id),
      );
      for (const object of go) {
        next = chooseObject(next, object.id, true);
      }
      for (const { key } of candidate.spots) {
        next = chooseSpot(next, key, loser);
      }
      taken += 1;
    }
    if (taken > 0) {
      setHistory([...history, patchChoices]);
      setPatchChoices(next);
    }
    const open = clusters.filter((candidate) => openIn(candidate) > 0).length;
    setWalkedPlaces(onWalkways);
    setNote(
      r.takeAllDone(
        taken,
        Math.max(0, open - taken - stuck - asking - onWalkways.length),
        stuck,
        asking,
        onWalkways.length,
      ),
    );
  };
  /** What a side brings to the comparison: one item, or parts in segments. */
  const levelOf = (side: 0 | 1) => {
    const parts = partsOf(shownSegments, side);
    const segmentCount = shownSegments.filter((segment) => segment.parts[side].length > 0).length;
    return parts.length === 1
      ? r.levelItem(names[side], parts[0]?.name ?? "")
      : r.levelParts(names[side], parts.length, Math.max(1, segmentCount));
  };
  /** Hides one object on its own, or shows it again (ADR 0040). */
  const toggle = (part: Part) => {
    const key = objectKey(part.id.plugin, part.id.localId);
    apply(
      hidden.has(key)
        ? revealObjects(patchChoices, spots, [part.id])
        : chooseObject(patchChoices, part.id, true),
      false,
    );
  };
  /** One object of a segment: hidden or kept, or why it stays. */
  const partToggle = (part: Part) => {
    const isHidden = hidden.has(objectKey(part.id.plugin, part.id.localId));
    const fate = fateOf(part);
    if (fate === "game") {
      return <span className="aside">{r.partGame}</span>;
    }
    if (fate === "locked" && !isHidden) {
      return <span className="aside">{r.partLocked}</span>;
    }
    return (
      <button type="button" className="small" aria-pressed={isHidden} onClick={() => toggle(part)}>
        {isHidden
          ? r.partShow
          : fate === "careful" || fate === "walked"
            ? r.partHideAnyway
            : r.partHide}
      </button>
    );
  };
  /** An object clicked in the views: of the two mods, it is hidden or shown again. */
  const pick = (object: AreaObject) => {
    const part: Part = {
      id: object.id,
      game: isGameObject(object),
      name: object.name ?? object.model.split(/[\\/]/).pop() ?? object.model,
    };
    const plugin = object.id.plugin.toLowerCase();
    if (!sides.some((side) => side.toLowerCase() === plugin)) {
      setNote(r.pickedOther(part.name, short(object.id.plugin)));
      return;
    }
    const isHidden = hidden.has(objectKey(object.id.plugin, object.id.localId));
    const fate = fateOf(part);
    if (!isHidden && (fate === "game" || fate === "locked")) {
      setNote(r.pickedStays(part.name));
      return;
    }
    toggle(part);
    const triangles = walked?.get(objectKey(object.id.plugin, object.id.localId)) ?? 0;
    setNote(
      isHidden
        ? r.pickedShown(part.name)
        : triangles > 0
          ? w.pickedWalked(part.name, triangles)
          : r.pickedHidden(part.name),
    );
  };
  /**
   * A segment's decision with the objects decided otherwise: of the side
   * hidden, parts that stay though they could go; of the side kept, parts
   * hidden; where nothing is decided, the parts hidden on their own.
   */
  const describeSegment = (segment: StageSegment, decided: Decided) => {
    const isHidden = (part: Part) => hidden.has(objectKey(part.id.plugin, part.id.localId));
    const all = [...partsOf([segment], 0), ...partsOf([segment], 1)];
    let exceptions = 0;
    if (decided.type === "hidden") {
      const loser = decided.plugin.toLowerCase();
      exceptions = all.filter((part) =>
        part.id.plugin.toLowerCase() === loser
          ? !isHidden(part) && fateOf(part) === "go"
          : isHidden(part),
      ).length;
    } else if (decided.type === "kept") {
      exceptions = all.filter(isHidden).length;
    } else if (decided.type === "open") {
      const single = all.filter(isHidden).length;
      return single > 0 ? `${r.stateOpen} · ${r.singlyHidden(single)}` : r.stateOpen;
    }
    return exceptions > 0
      ? `${describe(decided)} ${r.withExceptions(exceptions)}`
      : describe(decided);
  };
  const asked = anchors !== null && anchors.key === askingKey && walked !== null;
  const effects = [effectOf(0, shownSegments), effectOf(1, shownSegments)] as const;
  /** Whether keeping `side` can hide anything of the other side at all. */
  const canGo = (effect: (typeof effects)[number]) =>
    effect.go.length + effect.careful.length + effect.walked.length > 0;

  return (
    <div className="review-collision">
      <div className="review-suggest-bar">
        {measuring !== null ? (
          <>
            <span>{r.measuring(measuring.done, measuring.total)}</span>
            <button
              type="button"
              className="small quiet"
              onClick={() => {
                stopMeasuring.current = true;
              }}
            >
              {r.measureStop}
            </button>
          </>
        ) : (
          <button
            type="button"
            className="small"
            disabled={unmeasured === 0}
            onClick={() => {
              measureAll().catch(() => setMeasuring(null));
            }}
          >
            {r.measureAll(unmeasured)}
          </button>
        )}
        <button
          type="button"
          className="small primary"
          disabled={clear.length === 0 || measuring !== null}
          onClick={() => {
            takeAll().catch(() => {});
          }}
        >
          {r.takeAll(clear.length)}
        </button>
        <span className="aside">{r.suggestHint}</span>
      </div>
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
          <strong>
            {cluster.passage === undefined
              ? r.clusterOf(at + 1, clusters.length)
              : p.passageOf(at + 1, clusters.length)}
          </strong>
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
      {cluster.passage !== undefined && (
        <PassageSummary passage={cluster.passage.passage} worlds={report.map.worlds} />
      )}
      <CompareStage3D
        place={place}
        centre={cluster.centre}
        groups={groups}
        reach={reach}
        within={cluster.passage?.keys ?? null}
        radius={radius}
        focus={focusKeys}
        names={names}
        hidden={hidden}
        walked={walked}
        onSegments={(found) => {
          setSegments(found);
          onSurvey(cluster.key, found);
          // Their names, for the report of parts hidden on their own.
          rememberObjects(
            game,
            worldOfSpot(first.item.spot),
            found.flatMap((segment) => [...segment.parts[0], ...segment.parts[1]]),
          );
        }}
        onPick={pick}
        paths={repairShown}
      />
      {cluster.spots.map(({ key, item }) => {
        const group = item.spot.doors;
        if (group === null || group === undefined) {
          return null;
        }
        const at = placeOf(item.spot, report.map.worlds);
        const merge = (plan: DoorMerge | null) =>
          apply(mergeChoices(patchChoices, key, group, plan, freeObjects(patchChoices)), false);
        return (
          <DoorPanel
            key={key}
            group={group}
            hidden={(object) => hidden.has(matchKey(object))}
            keptAll={patchChoices[key] === null}
            onMerge={merge}
            onKeepAll={() => merge(null)}
            place={at.place.type === "exterior" ? at.place.world : at.place.cell}
            centre={at.place.type === "exterior" ? [at.centre[0], at.centre[1]] : null}
          />
        );
      })}
      {suggested !== null && (
        <p className="review-suggest">
          <span className="suggested-mark">{r.suggestedTag}</span> {suggestReason(suggested)}
        </p>
      )}
      <p className="review-compared">
        <strong>{r.compared}</strong> {levelOf(0)} ⟷ {levelOf(1)}
        <span className="aside"> · {r.pickHint}</span>
      </p>
      {note !== null && (
        <p className="review-note" role="status">
          {note}
        </p>
      )}
      {walkedPlaces.length > 0 && (
        <div className="walk-warning walk-doubtful">
          <PathIcon />
          <span>{w.doubtfulTitle(walkedPlaces.length)}</span>
          {walkedPlaces.map((key) => {
            const index = clusters.findIndex((candidate) => candidate.key === key);
            const other = clusters[index];
            if (other === undefined) {
              return null;
            }
            return (
              <button
                type="button"
                key={key}
                className="small"
                aria-pressed={index === at}
                onClick={() => onCursor(key)}
              >
                {r.clusterOf(index + 1, clusters.length)}: {short(other.plugins[0])} ⟷{" "}
                {short(other.plugins[1])}
              </button>
            );
          })}
        </div>
      )}
      {gameParts > 0 && <p className="aside">{r.gameParts(gameParts)}</p>}
      <div className="review-choices">
        <button
          type="button"
          className="primary"
          aria-pressed={placeKept === 0}
          disabled={!asked || !canGo(effects[0])}
          onClick={() => keep(0, shownSegments, true)}
        >
          {placeKept === 0 ? "✓ " : ""}
          {cluster.passage === undefined ? r.keepHere(names[0]) : p.keepVersion(names[0])}
          {suggested?.side === 0 && ` · ${r.suggestedTag}`}
        </button>
        <button
          type="button"
          className="primary"
          aria-pressed={placeKept === 1}
          disabled={!asked || !canGo(effects[1])}
          onClick={() => keep(1, shownSegments, true)}
        >
          {placeKept === 1 ? "✓ " : ""}
          {cluster.passage === undefined ? r.keepHere(names[1]) : p.keepVersion(names[1])}
          {suggested?.side === 1 && ` · ${r.suggestedTag}`}
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
        <PinButton
          keys={cluster.spots.filter(({ item }) => item.state === "open").map(({ key }) => key)}
        />
        {undo}
      </div>
      {!asked ? (
        <p className="aside">{r.checkingTies}</p>
      ) : (
        <div className="review-effects">
          {([0, 1] as const).map((side) => (
            <KeepEffect
              key={side}
              winner={names[side]}
              loser={names[side === 0 ? 1 : 0]}
              effect={effects[side]}
              anchors={anchors.byPart}
              walked={walked}
              walkPlace={
                stagePlace === null
                  ? null
                  : stagePlace.place.type === "exterior"
                    ? stagePlace.place.world
                    : stagePlace.place.cell
              }
              walkCentre={
                stagePlace === null || stagePlace.place.type === "interior"
                  ? null
                  : [cluster.centre[0], cluster.centre[1]]
              }
              decor={new Set(shownSegments.flatMap((segment) => segment.decor))}
              onAnyway={() => keep(side, shownSegments, true, true)}
              onRepairShown={setRepairShown}
            />
          ))}
        </div>
      )}
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
                  {segment.groups.length === 0 && cluster.passage !== undefined
                    ? p.looseParts(index + 1)
                    : r.segmentName(index + 1, segment.groups.length)}
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
                  {describeSegment(segment, decided)}
                </span>
                {suggestionOf([segment]) !== null && (
                  <span className="suggested-mark">
                    {r.segmentSuggested(names[suggestionOf([segment])?.side ?? 0])}
                  </span>
                )}
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
                  disabled={!asked || !canGo(effectOf(0, [segment]))}
                  onClick={() => keep(0, [segment], false)}
                >
                  {names[0]}
                </button>
                <button
                  type="button"
                  className="small"
                  aria-pressed={kept === 1}
                  disabled={!asked || !canGo(effectOf(1, [segment]))}
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
                <PinButton
                  keys={spotsOf(segment)
                    .filter(({ item }) => item.state === "open")
                    .map(({ key }) => key)}
                />
                <details className="review-segment-parts">
                  <summary className="more">
                    {r.partsTitle(segment.parts[0].length + segment.parts[1].length)}
                  </summary>
                  <div className="review-segment-sides">
                    {([0, 1] as const).map((side) => (
                      <div key={side}>
                        <strong className="stage3-swatch" data-side={side}>
                          {names[side]}
                        </strong>
                        <ul>
                          {partsOf([segment], side).map((part) => (
                            <li key={objectKey(part.id.plugin, part.id.localId)}>
                              <span>{part.name}</span>
                              {(walked?.get(objectKey(part.id.plugin, part.id.localId)) ?? 0) >
                                0 && (
                                <PathMark
                                  triangles={
                                    walked?.get(objectKey(part.id.plugin, part.id.localId)) ?? 0
                                  }
                                />
                              )}
                              {partToggle(part)}
                            </li>
                          ))}
                        </ul>
                      </div>
                    ))}
                  </div>
                </details>
              </li>
            );
          })}
        </ol>
      </section>
    </div>
  );
}

/** The spots of a place, each with the parts its structures start from. */
function groupsOf(cluster: Cluster): StageGroup[] {
  const sides = cluster.plugins;
  return cluster.spots.map(({ item, key }): StageGroup => {
    const parts = item.spot.parts ?? [];
    return {
      key,
      seeds: [
        parts.filter((part) => part.object.plugin === sides[0]).map((part) => part.object),
        parts.filter((part) => part.object.plugin === sides[1]).map((part) => part.object),
      ],
    };
  });
}

/** The worldspace or cell of a spot, as the report remembers objects by. */
function worldOfSpot(spot: MapSpot): string {
  return spot.location.type === "interior" ? spot.location.cell : spot.location.world;
}

/** A part a decision might hide, with what is known of it. */
interface Part {
  id: { plugin: string; localId: number };
  game: boolean;
  name: string;
  /** What the scan says of the spot's own parts: whether the patch may hide it. */
  hideable?: boolean;
}

/**
 * What becomes of a part the patch would hide; `walked`: characters walk on
 * it, and it goes only when the user says so (ADR 0051).
 */
type Fate = "go" | "careful" | "walked" | "locked" | "game";

/**
 * The parts of a place that a decision might hide, not the game's own, to
 * ask the game's ties of (ADR 0046), in the world or cell of its spots.
 */
function partsToAsk(
  cluster: Cluster,
  segments: StageSegment[],
): {
  place: string;
  refs: { plugin: string; localId: number }[];
  keys: string[];
  /** The same parts with where they stand, where the stage knows it. */
  queries: WalkQuery[];
} | null {
  const first = cluster.spots[0];
  if (first === undefined) {
    return null;
  }
  const { location } = first.item.spot;
  const place = location.type === "interior" ? location.cell : location.world;
  const found = new Map<string, WalkQuery>();
  const add = (id: { plugin: string; localId: number }) => {
    const key = objectKey(id.plugin, id.localId);
    if (!isGameObject({ id }) && !found.has(key)) {
      found.set(key, { id: { plugin: id.plugin, localId: id.localId } });
    }
  };
  for (const segment of segments) {
    for (const part of [...segment.parts[0], ...segment.parts[1]]) {
      // Objects of the game a mod moved are asked about too (ADR 0032).
      if (!isGameObject(part) || !isGamePlugin(part.plugin)) {
        found.set(objectKey(part.id.plugin, part.id.localId), {
          id: { plugin: part.id.plugin, localId: part.id.localId },
          position: part.position,
        });
      }
    }
  }
  for (const { item } of cluster.spots) {
    for (const part of item.spot.parts ?? []) {
      add(part.object);
    }
  }
  const keys = [...found.keys()].sort();
  const queries = keys.flatMap((key) => found.get(key) ?? []);
  return { place, refs: queries.map((query) => query.id), keys, queries };
}

/**
 * Parts with the same ties together, so that forty planks tied alike read
 * as one line: their names with how many of each, and the ties once.
 */
function tiedTogether(
  parts: Part[],
  anchors: Map<string, ObjectAnchors>,
): { key: string; names: string; tie: ObjectAnchors | undefined }[] {
  const groups = new Map<string, { tie: ObjectAnchors | undefined; names: Map<string, number> }>();
  for (const part of parts) {
    const tie = anchors.get(objectKey(part.id.plugin, part.id.localId));
    const key =
      tie === undefined
        ? "needed"
        : `${tie.verdict}|${tie.reasons
            .map(
              (reason) => `${reason.kind}:${reason.other?.name ?? ""}:${reason.other?.state ?? ""}`,
            )
            .join(",")}`;
    const group = groups.get(key) ?? { tie, names: new Map() };
    group.names.set(part.name, (group.names.get(part.name) ?? 0) + 1);
    groups.set(key, group);
  }
  return [...groups.entries()].map(([key, group]) => ({
    key,
    tie: group.tie,
    names: [...group.names.entries()]
      .map(([name, count]) => (count === 1 ? name : `${name} ×${count}`))
      .join(" · "),
  }));
}

/**
 * What keeping one side does to the other: how many of its parts can go,
 * which the game needs and why, with "hide anyway" for the careful ones,
 * and keeping both suggested where nothing can go.
 */
function KeepEffect({
  winner,
  loser,
  effect,
  anchors,
  walked,
  walkPlace,
  walkCentre,
  decor,
  onAnyway,
  onRepairShown,
}: {
  winner: string;
  loser: string;
  effect: {
    parts: Part[];
    go: Part[];
    careful: Part[];
    walked: Part[];
    locked: Part[];
    game: Part[];
  };
  anchors: Map<string, ObjectAnchors>;
  /** The pieces of path characters walk on, by part. */
  walked: Map<string, number> | null;
  /** Where the paths are asked about, as for `walked`. */
  walkPlace: string | null;
  walkCentre: [number, number] | null;
  /** The parts that are decor resting on others, by key. */
  decor: Set<string>;
  onAnyway: () => void;
  /** Told what the repair of the paths shows in the view (ADR 0058). */
  onRepairShown: (overlay: PathsOverlay | null) => void;
}) {
  const isDecor = (part: Part) => decor.has(objectKey(part.id.plugin, part.id.localId));
  const resting = effect.go.filter(isDecor);
  const restingStays = [
    ...effect.careful,
    ...effect.walked,
    ...effect.locked,
    ...effect.game,
  ].filter(isDecor);
  const r = useReviewTexts();
  const w = useWalkTexts();
  const pieces = effect.walked.reduce(
    (sum, part) => sum + (walked?.get(objectKey(part.id.plugin, part.id.localId)) ?? 0),
    0,
  );
  const anyway = effect.careful.length + effect.walked.length;
  const staying = [...effect.careful, ...effect.locked];
  // Where characters could no longer go with those parts hidden (ADR 0059).
  const reach = useReach(
    walkPlace,
    walkCentre,
    effect.walked.map((part) => part.id),
  );
  if (effect.parts.length === 0) {
    return null;
  }
  const nothing = effect.go.length + anyway === 0;
  return (
    <div className="review-effect" data-nothing={nothing}>
      <p>
        <strong>{r.keepHere(winner)}:</strong>{" "}
        {nothing
          ? r.nothingCanGo(loser)
          : r.keepEffect(loser, effect.go.length, effect.parts.length)}
        {resting.length > 0 && ` ${r.restingGoes(counted(resting.map((part) => part.name)))}`}
        {restingStays.length > 0 &&
          ` ${r.restingStays(counted(restingStays.map((part) => part.name)))}`}
        {effect.careful.length > 0 && ` ${r.carefulNote(effect.careful.length)}`}
        {effect.locked.length > 0 && ` ${r.lockedNote(effect.locked.length)}`}
        {effect.game.length > 0 && ` ${r.gameParts(effect.game.length)}`}
      </p>
      {staying.length > 0 && (
        <details className="review-ties">
          <summary className="more">{r.whyTheyStay(staying.length)}</summary>
          <ul>
            {tiedTogether(staying, anchors).map(({ key, names, tie }) => (
              <li key={key}>
                <strong>{names}</strong>
                {tie === undefined ? (
                  <span className="aside"> {r.neededByGame}</span>
                ) : (
                  <AnchorReasons anchors={tie} />
                )}
              </li>
            ))}
          </ul>
        </details>
      )}
      {effect.walked.length > 0 && (
        <p className="walk-warning">
          <PathIcon />
          <span>
            {w.keepWarning(
              loser,
              counted(effect.walked.map((part) => part.name)),
              effect.walked.length,
              pieces,
            )}
          </span>
        </p>
      )}
      {effect.walked.length > 0 && (
        <ReachNote
          reach={reach}
          follow={
            walkPlace === null
              ? null
              : {
                  place: walkPlace,
                  centre: walkCentre,
                  hidden: effect.walked.map((part) => part.id),
                }
          }
        />
      )}
      {effect.walked.length > 0 && walkPlace !== null && (
        <RepairPaths
          place={walkPlace}
          centre={walkCentre}
          objects={[...effect.go, ...effect.walked].map((part) => part.id)}
          onShow={onRepairShown}
        />
      )}
      {anyway > 0 && (
        <button type="button" className="small" onClick={onAnyway}>
          {effect.walked.length > 0
            ? w.hideAnyway(loser, anyway)
            : r.hideAnyway(loser, effect.careful.length)}
        </button>
      )}
    </div>
  );
}

/**
 * The report at the end (ADR 0050): the places decided and how, the mods
 * involved, and what is left open, in this step and in the others.
 */
function ReportStep({
  report,
  game,
  clusters,
  open,
  onCluster,
  onStep,
}: {
  report: RenderedReport;
  game: string;
  clusters: Cluster[];
  open: Record<ReviewStep, number | null>;
  onCluster: (key: string) => void;
  onStep: (step: ReviewStep) => void;
}) {
  const { patchChoices, showPinned } = useSession();
  const r = useReviewTexts();
  const pinText = usePinTexts();
  const pinned = pinsOf(patchChoices).size;
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
      {pinned > 0 && (
        <p className="review-report-line">
          <PinIcon /> <strong>{pinText.reportPinned(pinned)}</strong>
          <span className="spacer" />
          <button type="button" className="small" onClick={showPinned}>
            {pinText.showPinned}
          </button>
        </p>
      )}
      <PairReport report={report} game={game} />
    </div>
  );
}
