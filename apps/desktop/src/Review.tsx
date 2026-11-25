// The guided review (ADR 0050): the setup step by step, the collisions
// structure by structure. It owns nothing: it shows the homes' own rows
// and writes the same choices the homes write.

import { type ReactNode, useEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { useSession } from "./Actions";
import { AreaLayer, isGamePlugin, LOOK, objectKey } from "./area-layer";
import { isCollision, stepOf } from "./arrange";
import {
  CompareStage3D,
  type StageGroup,
  type StagePlace,
  type StageSegment,
} from "./CompareStage3D";
import { useCollisionStepTexts } from "./collision-step-texts";
import { keepSide, leftoversOf, mergeDoors } from "./decisions";
import { DoorPanel } from "./doors";
import type { AreaObject } from "./generated/area-objects";
import type { ObjectAnchors } from "./generated/object-anchors";
import type {
  DoorMerge,
  MapSpot,
  ObjectRef,
  RenderedFinding,
  RenderedReport,
} from "./generated/rendered-report";
import { reportSpots } from "./health";
import { InfoTip } from "./info-tip";
import { CELL, type WorkKind } from "./map-places";
import { fileDuels } from "./Overview";
import { AnchorReasons, isGameObject } from "./object-actions";
import { counted, rememberObjects } from "./pair-report";
import { partKind } from "./part-groups";
import { type ListedPart, PartList, type PartNote } from "./part-list";
import { usePassageTexts } from "./passage-texts";
import { PassageSummary } from "./passages";
import { matchKey, type PatchChoices } from "./patch";
import type { PathsOverlay } from "./paths-layer";
import { PinButton } from "./pins";
import { ReachNote, useReach } from "./reach";
import { RepairPaths } from "./repair-paths";
import {
  type Cluster,
  collisionClusters,
  type Decided,
  decidedOf,
  openIn,
} from "./review-clusters";
import { ReportStep } from "./review-report";
import { AssetsStep, InstallStep, MapStep } from "./review-steps";
import { type ReviewStep, useReviewTexts } from "./review-texts";
import { findSegments } from "./segments";
import { type Suggestion, suggestionOf, suggestionOfPassage } from "./suggest";
import { Eye } from "./view-tools";
import { PathIcon, useWalkedOn, useWalkTexts, type WalkQuery, walkedOn } from "./walkways";
import "./review.css";
import { Chip, Note, UndoButton } from "./ui/kit";

/** The steps, in their order. */
export const REVIEW_STEPS: ReviewStep[] = ["install", "assets", "map", "collisions", "report"];

/** Where the review leads for more than it shows. */
export type ReviewTarget = "tasks" | "mods" | "assets" | "map";

/** Kinds about plugins and their order: a group of their own in the installation step (ADR 0083). */
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

/** What the review remembers per game: the step the user is on, and the steps looked at. */
interface Stored {
  step: ReviewStep;
  seen: ReviewStep[];
  /** The place the collisions step shows, so that coming back finds it again. */
  place?: string | null;
  /**
   * Parts of a mod given up that the user said to leave as they are
   * where they still collide (ADR 0080), by {@link objectKey}.
   */
  accepted?: string[];
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
    // The load order was a step of its own until it joined the installation (ADR 0083).
    const was = (value.step as string | undefined) === "order" ? "install" : value.step;
    const step = REVIEW_STEPS.find((candidate) => candidate === was);
    const seen = (value.seen ?? []).filter((candidate) => REVIEW_STEPS.includes(candidate));
    const place = typeof value.place === "string" ? value.place : null;
    const accepted = Array.isArray(value.accepted)
      ? value.accepted.filter((key): key is string => typeof key === "string")
      : [];
    return step === undefined ? null : { step, seen, place, accepted };
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
  group: "mods" | "plugins",
): RenderedFinding[] {
  return report.findings.filter((rendered) => {
    if (isCollision(rendered) || ASSET_KINDS.has(rendered.finding.kind)) {
      return false;
    }
    const what = stepOf(rendered, ignored);
    if (what === null || what === "notes" || what === "ignored") {
      return false;
    }
    return ORDER_KINDS.has(rendered.finding.kind) === (group === "plugins");
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
  // The place the collisions step shows, kept here so that the report can
  // lead there, and remembered so that coming back from another page finds it.
  const clusterAt = stored.place ?? null;
  const setClusterAt = (place: string) => setStored((before) => ({ ...before, place }));
  const accepted = useMemo(() => new Set(stored.accepted ?? []), [stored.accepted]);
  // Where the collisions step puts its buttons for every place: in the bar of the steps.
  const [bar, setBar] = useState<HTMLElement | null>(null);
  // The segments of the places measured so far, by place, for the suggestions by size.
  const [survey, setSurvey] = useState<Map<string, StageSegment[]>>(new Map());
  const index = REVIEW_STEPS.indexOf(stored.step);
  const previous = REVIEW_STEPS[index - 1];
  const following = REVIEW_STEPS[index + 1];

  const mods = useMemo(() => findingsOf(report, ignored, "mods"), [report, ignored]);
  const plugins = useMemo(() => findingsOf(report, ignored, "plugins"), [report, ignored]);
  const allDuels = useMemo(() => fileDuels(report), [report]);
  const duels = allDuels.filter((rendered) => decisions[rendered.duel.id] === undefined);
  const clusters = useMemo(
    () => collisionClusters(reportSpots(report, ignored, patchChoices), report.map.passages ?? []),
    [report, ignored, patchChoices],
  );
  const openClusters = clusters.filter((cluster) => openIn(cluster) > 0).length;
  const open: Record<ReviewStep, number | null> = {
    install: mods.length + plugins.length,
    assets: duels.length,
    map: null,
    collisions: openClusters,
    report: null,
  };
  const go = (step: ReviewStep) => setStored((before) => ({ ...before, step }));
  const next = () => {
    const seen = stored.seen.includes(stored.step) ? stored.seen : [...stored.seen, stored.step];
    setStored({ ...stored, step: following ?? stored.step, seen });
  };

  return (
    <section className="review">
      <div className="review-steps-bar">
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
        {stored.step === "collisions" && <div ref={setBar} className="review-steps-tools" />}
      </div>

      <article className="panel review-body" aria-label={r.step[stored.step].title}>
        {stored.step === "install" && (
          <InstallStep
            report={report}
            mods={mods}
            plugins={plugins}
            onTasks={() => onOpen("tasks")}
            onMods={() => onOpen("mods")}
          />
        )}
        {stored.step === "assets" && (
          <AssetsStep duels={allDuels} onOpen={() => onOpen("assets")} />
        )}
        {stored.step === "map" && (
          <MapStep
            report={report}
            clusters={clusters}
            onPlace={(key) => {
              setClusterAt(key);
              go("collisions");
            }}
            onMap={() => onOpen("map")}
          />
        )}
        {stored.step === "collisions" && (
          <CollisionsStep
            report={report}
            game={game}
            bar={bar}
            accepted={accepted}
            onAccept={(keys) =>
              setStored((before) => ({
                ...before,
                accepted: [...new Set([...(before.accepted ?? []), ...keys])],
              }))
            }
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

/** Game units in a cubic metre: 70 to the metre. */
const ROOM_UNITS = 70 ** 3;

/** How many times the room of the other side makes a section's side the bigger one. */
const BIGGER = 1.5;

/** The kinds of collisions, the worst first, to name a section by its worst. */
const SPOT_ORDER: WorkKind[] = ["duplicate", "door", "overlap", "deep", "touch"];

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
  bar,
  accepted,
  onAccept,
  survey,
  onSurvey,
  clusters,
  cursor,
  onCursor,
}: {
  report: RenderedReport;
  game: string;
  /** Where the buttons for every place go, if not above the place. */
  bar: HTMLElement | null;
  /** Parts left where they still collide on purpose, by {@link objectKey}. */
  accepted: Set<string>;
  /** Leaves these parts as they are where they still collide. */
  onAccept: (keys: string[]) => void;
  survey: Map<string, StageSegment[]>;
  onSurvey: (key: string, segments: StageSegment[]) => void;
  clusters: Cluster[];
  cursor: string | null;
  onCursor: (key: string) => void;
}) {
  const {
    api,
    lang,
    ignored,
    patchChoices,
    setPatchChoices,
    spots,
    decidePassage,
    decideSpot,
    decided: shared,
  } = useSession();
  const r = useReviewTexts();
  const c = useCollisionStepTexts();
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
  // The section whose other choices are open, if any.
  const [sectionChoices, setSectionChoices] = useState<number | null>(null);
  // What the last click in the views did.
  const [note, setNote] = useState<string | null>(null);
  // Parts shown in the views with the rest dimmed (ADR 0080): what a
  // button names, such as the parts a choice hides.
  const [shownApart, setShownApart] = useState<{
    id: string;
    keys: string[];
    label: string;
  } | null>(null);
  // The parts the pointer is on in a list, lit up in the views.
  const [pointed, setPointed] = useState<string[] | null>(null);
  // The parts picked in the lists, by key.
  const [picked, setPicked] = useState<Set<string>>(new Set());
  // Whether each side's list of single parts is open.
  const [singles, setSingles] = useState<[boolean, boolean]>([false, false]);
  const found = clusters.findIndex((cluster) => cluster.key === cursor);
  const at = Math.max(0, found >= 0 ? found : clusters.findIndex((cluster) => openIn(cluster) > 0));
  const cluster = clusters[at];
  const currentKey = cluster?.key ?? null;
  // The place shown is held on to, so that deciding parts of it by hand
  // does not move on to the next open one; only deciding it all does.
  useEffect(() => {
    if (currentKey !== null && currentKey !== cursor) {
      onCursor(currentKey);
    }
  }, [currentKey, cursor, onCursor]);
  // biome-ignore lint/correctness/useExhaustiveDependencies: the structures belong to the place.
  useEffect(() => {
    setSegments([]);
    setSectionChoices(null);
    setNote(null);
    setShownApart(null);
    setPointed(null);
    setPicked(new Set());
    setSingles([false, false]);
  }, [currentKey]);
  // What is hidden, the same on every page (ADR 0081).
  const hidden = shared.hiddenKeys;
  // One door of stacked doors looked at alone (ADR 0067): the others as ghosts.
  const [doorLook, setDoorLook] = useState<{ object: ObjectRef; others: ObjectRef[] } | null>(null);
  const stageHidden = useMemo(
    () => (doorLook === null ? hidden : new Set([...hidden, ...doorLook.others.map(matchKey)])),
    [hidden, doorLook],
  );
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
    <UndoButton
      disabled={history.length === 0}
      onClick={() => {
        const previous = history[history.length - 1];
        if (previous !== undefined) {
          setHistory(history.slice(0, -1));
          setPatchChoices(previous);
        }
      }}
    />
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
  const placeDecided = decidedOf(cluster.spots, patchChoices, cluster.passage?.passage);

  /** A side's parts in `chosen`, or the spots' own parts until the structures are known. */
  const partsOf = (chosen: StageSegment[], side: 0 | 1): Part[] => {
    const known = chosen.flatMap((segment) => segment.parts[side]);
    if (known.length > 0) {
      return known.map((object) => ({
        id: object.id,
        game: isGameObject(object) && isGamePlugin(object.plugin),
        name: object.name ?? object.model.split(/[\\/]/).pop() ?? object.model,
        model: object.model,
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
  const keyOfPart = (part: Part) => objectKey(part.id.plugin, part.id.localId);
  /**
   * What of a mod given up still collides with the other here after
   * `choices` (ADR 0080): parts the game needs or characters walk on,
   * tied ones and parts kept by hand. At its spots, where a part of the
   * other mod is left too; in a passage, every part left in the sections
   * given up and among the passage's other parts. Parts the user said to
   * leave as they are no longer count.
   */
  const leftoversIn = (choices: PatchChoices) =>
    leftoversOf(shared.whatIf(choices), {
      sides,
      spots: cluster.spots.map(({ key, item }) => ({
        key,
        spot: item.spot,
        finding: item.finding,
      })),
      partOf: (part): Part => ({
        id: part.object,
        game: isGameObject({ id: part.object }),
        name: part.name,
        hideable: part.hideable,
      }),
      // A section whose spots are not known is not counted as given up.
      sections:
        cluster.passage === undefined
          ? undefined
          : shownSegments
              .filter((segment) => segment.groups.length === 0 || spotsOf(segment).length > 0)
              .map((segment) => ({
                spots: spotsOf(segment).map(({ item }) => ({
                  spot: item.spot,
                  finding: item.finding,
                })),
                parts: [partsOf([segment], 0), partsOf([segment], 1)] as const,
              })),
      accepted,
    });
  /**
   * Keeps the choices; deciding the whole place moves on to the next open
   * one, unless parts of the mod given up still collide here.
   */
  const apply = (next: PatchChoices, moveOn: boolean) => {
    setHistory([...history, patchChoices]);
    setPatchChoices(next);
    if (!moveOn || leftoversIn(next) !== null) {
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
    const effect = effectOf(side, chosen);
    const gone = anyway ? [...effect.go, ...effect.careful, ...effect.walked] : effect.go;
    // Every spot of the segments is decided, also where the losing side's
    // parts there are the game's own and stay.
    apply(
      shared.keepSide({
        kept: partsOf(chosen, side).map((part) => part.id),
        gone: gone.map((part) => part.id),
        spots: chosen.flatMap((segment) => spotsOf(segment).map(({ key }) => key)),
        loser: sides[side === 0 ? 1 : 0],
      }),
      moveOn,
    );
  };
  const keepBoth = (chosen: StageSegment[], moveOn: boolean) =>
    apply(
      shared.keepSide({
        kept: [...partsOf(chosen, 0), ...partsOf(chosen, 1)].map((part) => part.id),
        gone: [],
        spots: chosen.flatMap((segment) => spotsOf(segment).map(({ key }) => key)),
        loser: null,
      }),
      moveOn,
    );
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
      next = keepSide(next, shared.spots, {
        kept: keeping.map((object) => object.id),
        gone: go.map((object) => object.id),
        spots: candidate.spots.map(({ key }) => key),
        loser,
      });
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
  /** Hides one object on its own, or shows it again (ADR 0040). */
  const toggle = (part: Part) => {
    const key = objectKey(part.id.plugin, part.id.localId);
    apply(hidden.has(key) ? shared.show([part.id]) : shared.hide([part.id]), false);
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
  /** Shows `parts` in the views with the rest dimmed, or everything again for the same button. */
  const showApart = (id: string, parts: Part[], label: string) =>
    setShownApart((before) =>
      before?.id === id || parts.length === 0 ? null : { id, keys: parts.map(keyOfPart), label },
    );
  const decor = new Set(shownSegments.flatMap((segment) => segment.decor));
  /** A part as the lists show it. */
  const listed = (part: Part): ListedPart => {
    const key = keyOfPart(part);
    const fate = fateOf(part);
    const isHidden = hidden.has(key);
    const notes: PartNote[] = [
      ...(fate === "careful" ? (["tied"] as const) : []),
      ...(fate === "locked" ? (["needed"] as const) : []),
      ...(fate === "walked" ? (["walked"] as const) : []),
      ...(fate === "game" ? (["game"] as const) : []),
      ...(decor.has(key) ? (["rests"] as const) : []),
    ];
    return {
      key,
      id: part.id,
      name: part.name,
      side: part.id.plugin.toLowerCase() === sides[0].toLowerCase() ? 0 : 1,
      kind: partKind(part.model ?? ""),
      notes,
      hidden: isHidden,
      stays: fate === "game" ? "game" : fate === "locked" && !isHidden ? "locked" : null,
      walked: walked?.get(key) ?? 0,
    };
  };
  // Hidden and shown again as on every page (ADR 0081).
  const hideParts = (chosen: ListedPart[]) =>
    apply(shared.hide(chosen.map((part) => part.id)), false);
  const showParts = (chosen: ListedPart[]) =>
    apply(shared.show(chosen.map((part) => part.id)), false);
  const partList = (parts: Part[]) => (
    <PartList
      parts={parts.map(listed)}
      names={names}
      picked={picked}
      onPicked={setPicked}
      onHide={hideParts}
      onShow={showParts}
      onHover={setPointed}
    />
  );
  const leftovers = leftoversIn(patchChoices);
  const stagePlaceName =
    stagePlace === null
      ? null
      : stagePlace.place.type === "exterior"
        ? stagePlace.place.world
        : stagePlace.place.cell;
  const stageCentre: [number, number] | null =
    stagePlace === null || stagePlace.place.type === "interior"
      ? null
      : [cluster.centre[0], cluster.centre[1]];
  const openInMap = () => {
    if (cluster.passage !== undefined) {
      decidePassage(first.item.spot, cluster.key);
    } else {
      decideSpot(first.item.spot);
    }
  };

  const barTools = (
    <div className="review-bar-tools">
      {measuring !== null ? (
        <>
          <span className="aside">{r.measuring(measuring.done, measuring.total)}</span>
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
          title={c.measureTip}
          disabled={unmeasured === 0}
          onClick={() => {
            measureAll().catch(() => setMeasuring(null));
          }}
        >
          {c.measure(unmeasured)}
        </button>
      )}
      <button
        type="button"
        className={clear.length > 0 ? "small primary" : "small"}
        title={c.takeTip}
        disabled={clear.length === 0 || measuring !== null}
        onClick={() => {
          takeAll().catch(() => {});
        }}
      >
        {c.take(clear.length)}
      </button>
      <InfoTip text={`${c.measureTip} ${c.takeTip}`} />
    </div>
  );

  return (
    <div className="review-collision">
      {bar !== null ? createPortal(barTools, bar) : barTools}
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
          <span className="aside">· {r.reportCollisions(decidedPlaces, clusters.length)}</span>
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
        <span className="review-state" data-decided={placeDecided.type !== "open"}>
          {describe(placeDecided)}
        </span>
        <span className="spacer" />
        <button type="button" className="small quiet" title={c.openInMapTip} onClick={openInMap}>
          {c.openInMap} ↗
        </button>
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
        focus={doorLook === null ? null : [matchKey(doorLook.object)]}
        names={names}
        hidden={stageHidden}
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
        highlight={shownApart}
        onClearHighlight={() => setShownApart(null)}
        hover={pointed ?? (picked.size > 0 ? [...picked] : null)}
        tools={
          <>
            <PinButton
              keys={cluster.spots.filter(({ item }) => item.state === "open").map(({ key }) => key)}
            />
            {undo}
          </>
        }
      />
      {cluster.spots.map(({ key, item }) => {
        const group = item.spot.doors;
        if (group === null || group === undefined) {
          return null;
        }
        const at = placeOf(item.spot, report.map.worlds);
        const merge = (plan: DoorMerge | null) =>
          apply(mergeDoors(patchChoices, key, group, plan, spots), false);
        return (
          <DoorPanel
            key={key}
            group={group}
            hidden={(object) => hidden.has(matchKey(object))}
            keptAll={patchChoices[key] === null}
            onMerge={merge}
            onKeepAll={() => merge(null)}
            looking={doorLook?.object ?? null}
            onLookAt={(object) =>
              setDoorLook(
                object === null
                  ? null
                  : {
                      object,
                      others: group.doors
                        .map((door) => door.object)
                        .filter((other) => matchKey(other) !== matchKey(object)),
                    },
              )
            }
            place={at.place.type === "exterior" ? at.place.world : at.place.cell}
            centre={at.place.type === "exterior" ? [at.centre[0], at.centre[1]] : null}
          />
        );
      })}
      {note !== null && (
        <Note role="status" compact>
          <p>{note}</p>
        </Note>
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
      <div className="review-sides">
        {([0, 1] as const).map((side) => (
          <SideColumn
            key={`${cluster.key}:${side}`}
            side={side}
            name={names[side]}
            other={names[side === 0 ? 1 : 0]}
            keepLabel={
              cluster.passage === undefined ? r.keepHere(names[side]) : p.keepVersion(names[side])
            }
            kept={placeKept === side}
            suggested={suggested?.side === side ? suggestReason(suggested) : null}
            asked={asked}
            canKeep={asked && canGo(effects[side])}
            onKeep={() => keep(side, shownSegments, true)}
            effect={effects[side]}
            anchors={anchors?.byPart ?? new Map()}
            walked={walked}
            walkPlace={stagePlaceName}
            walkCentre={stageCentre}
            decor={decor}
            onAnyway={() => keep(side, shownSegments, true, true)}
            onRepairShown={setRepairShown}
            shownApart={shownApart?.id ?? null}
            onShowApart={showApart}
            singles={singles[side]}
            onSingles={() =>
              setSingles(side === 0 ? [!singles[0], singles[1]] : [singles[0], !singles[1]])
            }
            list={partList(partsOf(shownSegments, side))}
          />
        ))}
      </div>
      <div className="review-choices">
        <button
          type="button"
          aria-pressed={placeDecided.type === "kept"}
          onClick={() => keepBoth(shownSegments, true)}
        >
          {placeDecided.type === "kept" ? "✓ " : ""}
          {r.keepBothHere}
        </button>
        {leftovers !== null && (
          <Note
            tone="warning"
            compact
            className="review-left"
            actions={
              <>
                <button
                  type="button"
                  className="small"
                  title={c.leftKeepBothTip}
                  onClick={() => onAccept(leftovers.parts.map(keyOfPart))}
                >
                  {c.leftKeepBoth}
                </button>
                {leftovers.parts.some((part) => {
                  const fate = fateOf(part);
                  return fate !== "game" && fate !== "locked";
                }) ? (
                  <button
                    type="button"
                    className="small"
                    onClick={() =>
                      hideParts(
                        leftovers.parts
                          .filter((part) => {
                            const fate = fateOf(part);
                            return fate !== "game" && fate !== "locked";
                          })
                          .map(listed),
                      )
                    }
                  >
                    {c.leftHide}
                  </button>
                ) : (
                  <span className="aside">{c.leftNeeded}</span>
                )}
                <button
                  type="button"
                  className="small quiet"
                  aria-pressed={shownApart?.id === "left"}
                  onClick={() => {
                    showApart(
                      "left",
                      leftovers.parts,
                      c.leftParts(leftovers.parts.length, short(leftovers.loser)),
                    );
                    setPicked(new Set(leftovers.parts.map(keyOfPart)));
                    setSingles(leftovers.side === 0 ? [true, singles[1]] : [singles[0], true]);
                  }}
                >
                  <Eye open /> {c.leftOpen}
                </button>
              </>
            }
          >
            <p>{c.left(leftovers.parts.length, short(leftovers.loser))}</p>
          </Note>
        )}
      </div>
      <section className="review-segments">
        <h4>{c.sectionsTitle}</h4>
        <ol>
          {shownSegments.map((segment, index) => {
            const here = spotsOf(segment);
            const decided = decidedOf(here, patchChoices);
            const kept = keptSideOf(decided);
            const counts = [segment.parts[0].length, segment.parts[1].length] as const;
            const kinds = new Set(here.map(({ item }) => item.kind));
            const worst = SPOT_ORDER.find((kind) => kinds.has(kind));
            const suggestion = suggestionOf([segment]);
            // In m³; one side counts as bigger with half again the room.
            const room = [
              segment.volumes[0] / ROOM_UNITS,
              segment.volumes[1] / ROOM_UNITS,
            ] as const;
            const larger: 0 | 1 = room[0] >= room[1] ? 0 : 1;
            const bigger: 0 | 1 | null =
              suggestion?.side ??
              (counts[0] === 0 ||
              counts[1] === 0 ||
              room[larger] < BIGGER * room[larger === 0 ? 1 : 0]
                ? null
                : larger);
            const what =
              counts[0] === 0 || counts[1] === 0
                ? c.sectionOnly(names[counts[0] === 0 ? 1 : 0], counts[0] + counts[1])
                : bigger !== null
                  ? c.sectionBigger(
                      names[bigger],
                      counts[bigger],
                      roomText(room[bigger]),
                      names[bigger === 0 ? 1 : 0],
                      counts[bigger === 0 ? 1 : 0],
                      roomText(room[bigger === 0 ? 1 : 0]),
                    )
                  : c.sectionAlike(names[0], counts[0], names[1], counts[1]);
            // One thing to do: keep the bigger version, or what the whole
            // place keeps; where neither can hide anything, keep both.
            // Without a reason, the two versions are offered alike.
            const wanted = bigger ?? placeKept;
            const canKeep = (side: 0 | 1) => asked && canGo(effectOf(side, [segment]));
            const recommended: 0 | 1 | "both" | null =
              wanted !== null && canKeep(wanted)
                ? wanted
                : asked && !canKeep(0) && !canKeep(1)
                  ? "both"
                  : null;
            const offered: (0 | 1 | "both")[] = recommended === null ? [0, 1] : [recommended];
            const choose = (choice: 0 | 1 | "both") =>
              choice === "both" ? keepBoth([segment], false) : keep(choice, [segment], false);
            const label = (choice: 0 | 1 | "both") =>
              choice === "both" ? r.keepBothHere : r.keepHere(names[choice]);
            const others = ([0, 1, "both"] as const).filter(
              (choice) => decided.type !== "open" || !offered.includes(choice),
            );
            const parts = [...partsOf([segment], 0), ...partsOf([segment], 1)];
            const showId = `section:${index}`;
            const choicesOpen = sectionChoices === index;
            return (
              <li
                key={segment.groups.join(",")}
                className="review-section"
                data-decided={decided.type !== "open" ? "" : undefined}
              >
                <button
                  type="button"
                  className="review-section-show small"
                  aria-pressed={shownApart?.id === showId}
                  title={c.sectionShowTip}
                  disabled={parts.length === 0}
                  onClick={() =>
                    showApart(
                      showId,
                      parts,
                      segment.groups.length === 0 && cluster.passage !== undefined
                        ? p.looseParts(index + 1)
                        : r.segmentName(index + 1, segment.groups.length),
                    )
                  }
                >
                  <Eye open /> {c.sectionShow}
                </button>
                <div className="review-section-body">
                  <div className="review-section-head">
                    <strong>
                      {segment.groups.length === 0 && cluster.passage !== undefined
                        ? p.looseParts(index + 1)
                        : r.segmentName(index + 1, segment.groups.length)}
                    </strong>
                    {worst !== undefined && <Chip>{c.spotKind[worst]}</Chip>}
                    <span className="review-state" data-decided={decided.type !== "open"}>
                      {decided.type === "open"
                        ? r.stateOpen
                        : `✓ ${describeSegment(segment, decided)}`}
                    </span>
                  </div>
                  <p className="review-section-what">{what}</p>
                  <div className="review-section-actions">
                    {decided.type === "open" ? (
                      <>
                        {recommended !== null && <span className="aside">{c.recommended}</span>}
                        {offered.map((choice) => (
                          <button
                            type="button"
                            key={choice}
                            className={recommended === null ? "small" : "small primary"}
                            disabled={choice !== "both" && !canKeep(choice)}
                            title={recommended === null ? undefined : c.recommendedTip}
                            onClick={() => choose(choice)}
                          >
                            {label(choice)}
                          </button>
                        ))}
                      </>
                    ) : null}
                    <button
                      type="button"
                      className="small quiet"
                      aria-expanded={choicesOpen}
                      onClick={() => setSectionChoices(choicesOpen ? null : index)}
                    >
                      {decided.type === "open" ? c.otherChoices : c.change}{" "}
                      <span aria-hidden="true">{choicesOpen ? "▾" : "▸"}</span>
                    </button>
                    {choicesOpen && (
                      <>
                        {others.map((choice) => (
                          <button
                            type="button"
                            key={choice}
                            className="small"
                            aria-pressed={
                              choice === "both" ? decided.type === "kept" : kept === choice
                            }
                            disabled={
                              choice !== "both" && (!asked || !canGo(effectOf(choice, [segment])))
                            }
                            onClick={() => choose(choice)}
                          >
                            {label(choice)}
                          </button>
                        ))}
                        <PinButton
                          keys={here
                            .filter(({ item }) => item.state === "open")
                            .map(({ key }) => key)}
                        />
                      </>
                    )}
                  </div>
                  {parts.length > 0 && (
                    <details className="review-section-parts">
                      <summary className="more">{c.sectionParts(parts.length)}</summary>
                      {partList(parts)}
                    </details>
                  )}
                </div>
              </li>
            );
          })}
        </ol>
      </section>
    </div>
  );
}

/** The spots of a place, each with the parts its structures start from. */
export function groupsOf(cluster: Cluster): StageGroup[] {
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
  /** Its model below the data directory, where the stage knows it. */
  model?: string;
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

/** What keeping one side does to the other's parts, grouped by their fate. */
interface Effect {
  parts: Part[];
  go: Part[];
  careful: Part[];
  walked: Part[];
  locked: Part[];
  game: Part[];
}

/** A note of a side's column that opens to say more: tied parts, parts that stay, and so on. */
type Badge = "tied" | "walked" | "stay" | "resting" | "restingStays" | "game";

/**
 * One mod's column (ADR 0080), in the mod's colour: keeping its version
 * here, what that does to the other mod's parts in a few words with the
 * way to see them, its notes as badges that open to say more, and its
 * own parts to hide one by one.
 */
function SideColumn({
  side,
  name,
  other,
  keepLabel,
  kept,
  suggested,
  asked,
  canKeep,
  onKeep,
  effect,
  anchors,
  walked,
  walkPlace,
  walkCentre,
  decor,
  onAnyway,
  onRepairShown,
  shownApart,
  onShowApart,
  singles,
  onSingles,
  list,
}: {
  side: 0 | 1;
  name: string;
  /** The other mod, whose parts keeping this one hides. */
  other: string;
  keepLabel: string;
  kept: boolean;
  /** Why keeping this side is suggested, if it is. */
  suggested: string | null;
  /** Whether what the game needs here is known. */
  asked: boolean;
  canKeep: boolean;
  onKeep: () => void;
  effect: Effect;
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
  /** Which parts the views show apart now, by the button's id. */
  shownApart: string | null;
  onShowApart: (id: string, parts: Part[], label: string) => void;
  /** Whether the list of this side's parts is open. */
  singles: boolean;
  onSingles: () => void;
  list: ReactNode;
}) {
  const r = useReviewTexts();
  const c = useCollisionStepTexts();
  const w = useWalkTexts();
  const [open, setOpen] = useState<Badge | null>(null);
  const isDecor = (part: Part) => decor.has(objectKey(part.id.plugin, part.id.localId));
  const resting = effect.go.filter(isDecor);
  const restingStays = [
    ...effect.careful,
    ...effect.walked,
    ...effect.locked,
    ...effect.game,
  ].filter(isDecor);
  const pieces = effect.walked.reduce(
    (sum, part) => sum + (walked?.get(objectKey(part.id.plugin, part.id.localId)) ?? 0),
    0,
  );
  const anyway = effect.careful.length + effect.walked.length;
  // Where characters could no longer go with those parts hidden (ADR 0059).
  const reach = useReach(
    walkPlace,
    walkCentre,
    effect.walked.map((part) => part.id),
  );
  const nothing = effect.go.length + anyway === 0;
  const badges: { id: Badge; parts: Part[]; text: string; tone: "warning" | "neutral" }[] = [
    {
      id: "tied" as const,
      parts: effect.careful,
      text: c.badgeTied(effect.careful.length),
      tone: "warning" as const,
    },
    {
      id: "walked" as const,
      parts: effect.walked,
      text: c.badgeWalked(effect.walked.length),
      tone: "warning" as const,
    },
    {
      id: "stay" as const,
      parts: effect.locked,
      text: c.badgeStay(effect.locked.length),
      tone: "neutral" as const,
    },
    {
      id: "resting" as const,
      parts: resting,
      text: c.badgeResting(resting.length),
      tone: "neutral" as const,
    },
    {
      id: "restingStays" as const,
      parts: restingStays,
      text: c.badgeRestingStays(restingStays.length),
      tone: "neutral" as const,
    },
    {
      id: "game" as const,
      parts: effect.game,
      text: c.badgeGame(effect.game.length),
      tone: "neutral" as const,
    },
  ].filter((badge) => badge.parts.length > 0);
  const opened = badges.find((badge) => badge.id === open);
  const labelOf = (badge: Badge, parts: Part[]) =>
    badge === "tied"
      ? c.tiedParts(parts.length, other)
      : badge === "walked"
        ? c.walkedParts(parts.length, other)
        : badge === "stay" || badge === "game" || badge === "restingStays"
          ? c.stayingParts(parts.length, other)
          : c.goingParts(parts.length, other);
  const showWhich = (badge: Badge, parts: Part[]) => (
    <button
      type="button"
      className="small quiet"
      aria-pressed={shownApart === `${side}:${badge}`}
      onClick={() => onShowApart(`${side}:${badge}`, parts, labelOf(badge, parts))}
    >
      <Eye open /> {c.showWhich}
    </button>
  );
  const reasons = (parts: Part[]) => (
    <ul className="review-side-reasons">
      {tiedTogether(parts, anchors).map(({ key, names, tie }) => (
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
  );
  const hideAnywayButton = anyway > 0 && (
    <button type="button" className="small" onClick={onAnyway}>
      {effect.walked.length > 0
        ? w.hideAnyway(other, anyway)
        : r.hideAnyway(other, effect.careful.length)}
    </button>
  );

  return (
    <section className="review-side" data-side={side} data-kept={kept ? "" : undefined}>
      <header className="review-side-head">
        <strong className="stage3-swatch" data-side={side}>
          {name}
        </strong>
        {suggested !== null && (
          <Chip tone="info" title={suggested}>
            {r.suggestedTag}
          </Chip>
        )}
        {kept && (
          <Chip tone="ok" icon>
            {c.keptHere}
          </Chip>
        )}
      </header>
      <button
        type="button"
        className="primary review-side-keep"
        aria-pressed={kept}
        disabled={!canKeep}
        title={suggested ?? undefined}
        onClick={onKeep}
      >
        {kept ? "✓ " : ""}
        {keepLabel}
      </button>
      {!asked ? (
        <p className="aside">{r.checkingTies}</p>
      ) : (
        effect.parts.length > 0 && (
          <>
            <p className="review-side-effect">
              {nothing ? (
                <>
                  <span>{c.effectNothing(other)}</span>
                  <InfoTip text={c.effectNothingTip} />
                </>
              ) : (
                <span>{c.effectGo(other, effect.go.length, effect.parts.length)}</span>
              )}
              {effect.go.length > 0 && (
                <button
                  type="button"
                  className="small quiet"
                  aria-pressed={shownApart === `${side}:go`}
                  onClick={() =>
                    onShowApart(`${side}:go`, effect.go, c.goingParts(effect.go.length, other))
                  }
                >
                  <Eye open /> {c.showThese}
                </button>
              )}
            </p>
            {badges.length > 0 && (
              <div className="review-side-badges">
                {badges.map((badge) => (
                  <button
                    type="button"
                    key={badge.id}
                    className="review-side-badge"
                    data-tone={badge.tone}
                    aria-expanded={open === badge.id}
                    onClick={() => setOpen(open === badge.id ? null : badge.id)}
                  >
                    {badge.id === "walked" && <PathIcon />}
                    {badge.text}
                    <span aria-hidden="true">{open === badge.id ? "▾" : "▸"}</span>
                  </button>
                ))}
              </div>
            )}
            {opened !== undefined && (
              <div className="review-side-more">
                {opened.id === "tied" && (
                  <>
                    <p>{c.tiedExplain}</p>
                    {reasons(opened.parts)}
                    <div className="review-side-actions">
                      {showWhich("tied", opened.parts)}
                      {hideAnywayButton}
                    </div>
                  </>
                )}
                {opened.id === "walked" && (
                  <>
                    <p className="walk-warning">
                      <PathIcon />
                      <span>
                        {w.keepWarning(
                          other,
                          counted(effect.walked.map((part) => part.name)),
                          effect.walked.length,
                          pieces,
                        )}
                      </span>
                    </p>
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
                    {walkPlace !== null && (
                      <RepairPaths
                        place={walkPlace}
                        centre={walkCentre}
                        objects={[...effect.go, ...effect.walked].map((part) => part.id)}
                        onShow={onRepairShown}
                      />
                    )}
                    <div className="review-side-actions">
                      {showWhich("walked", opened.parts)}
                      {hideAnywayButton}
                    </div>
                  </>
                )}
                {opened.id === "stay" && (
                  <>
                    <p>{c.stayExplain}</p>
                    {reasons(opened.parts)}
                    <div className="review-side-actions">{showWhich("stay", opened.parts)}</div>
                  </>
                )}
                {(opened.id === "resting" || opened.id === "restingStays") && (
                  <>
                    <p>
                      {opened.id === "resting" ? c.restingExplain : c.restingStaysExplain}{" "}
                      {counted(opened.parts.map((part) => part.name))}
                    </p>
                    <div className="review-side-actions">{showWhich(opened.id, opened.parts)}</div>
                  </>
                )}
                {opened.id === "game" && (
                  <>
                    <p>{c.gameExplain}</p>
                    <div className="review-side-actions">{showWhich("game", opened.parts)}</div>
                  </>
                )}
              </div>
            )}
          </>
        )
      )}
      <button
        type="button"
        className="small quiet review-side-singles"
        aria-expanded={singles}
        onClick={onSingles}
      >
        <span aria-hidden="true">{singles ? "▾" : "▸"}</span> {c.hideSingles(name)}
      </button>
      {singles && list}
    </section>
  );
}
