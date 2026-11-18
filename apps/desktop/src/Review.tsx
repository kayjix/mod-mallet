// The guided review (ADR 0050): the setup step by step, the collisions
// structure by structure. It owns nothing: it shows the homes' own rows
// and writes the same choices the homes write.

import { type ReactNode, useEffect, useMemo, useState } from "react";
import { useSession } from "./Actions";
import { objectKey } from "./area-layer";
import { isCollision, stepOf } from "./arrange";
import { CompareStage3D, type StagePlace } from "./CompareStage3D";
import { DuelLine } from "./Duels";
import type { AreaObject } from "./generated/area-objects";
import type { MapSpot, RenderedFinding, RenderedReport } from "./generated/rendered-report";
import { reportSpots } from "./health";
import { CELL, type WorkKind } from "./map-places";
import { fileDuels } from "./Overview";
import { isGameObject } from "./object-actions";
import {
  chooseObject,
  chooseSpot,
  decisionAt,
  hiddenKeysOf,
  type PatchChoices,
  revealObjects,
  spotKey,
} from "./patch";
import { type ReviewStep, useReviewTexts } from "./review-texts";
import { FindingRow } from "./Tasks";
import "./review.css";

/** The steps, in their order. */
export const REVIEW_STEPS: ReviewStep[] = ["install", "order", "assets", "map", "collisions"];

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

/** The worst collisions first. */
const KIND_ORDER: WorkKind[] = ["duplicate", "overlap", "deep", "touch"];

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
 * the assets, the world map and the collisions, the last decided here one
 * structure at a time. Where the user was is remembered per game.
 */
export function Review({ report, game, onOpen }: ReviewProps) {
  const { ignored, decisions, patchChoices } = useSession();
  const r = useReviewTexts();
  const [stored, setStored] = useState<Stored>(
    () => storedReview(game) ?? { step: "install", seen: [] },
  );
  useEffect(() => remember(game, stored), [game, stored]);
  const index = REVIEW_STEPS.indexOf(stored.step);
  const previous = REVIEW_STEPS[index - 1];
  const following = REVIEW_STEPS[index + 1];

  const install = useMemo(() => findingsOf(report, ignored, "install"), [report, ignored]);
  const order = useMemo(() => findingsOf(report, ignored, "order"), [report, ignored]);
  const duels = useMemo(
    () => fileDuels(report).filter((rendered) => decisions[rendered.duel.id] === undefined),
    [report, decisions],
  );
  const spots = useMemo(
    () => reportSpots(report, ignored, patchChoices).filter((spot) => spot.state === "open"),
    [report, ignored, patchChoices],
  );
  const open: Record<ReviewStep, number | null> = {
    install: install.length,
    order: order.length,
    assets: duels.length,
    map: null,
    collisions: spots.length,
  };
  const go = (step: ReviewStep) => setStored({ ...stored, step });
  const next = () => {
    const seen = stored.seen.includes(stored.step) ? stored.seen : [...stored.seen, stored.step];
    const following = REVIEW_STEPS[index + 1];
    setStored({ step: following ?? stored.step, seen });
  };
  const finished =
    stored.seen.includes("collisions") || (index === REVIEW_STEPS.length - 1 && spots.length === 0);

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
        {stored.step === "collisions" && <CollisionsStep report={report} />}
        {/* Between the steps; moving within a step is the step's own. */}
        <footer className="review-foot">
          {previous !== undefined && (
            <button type="button" className="quiet" onClick={() => go(previous)}>
              {r.stepBack(r.step[previous].title)}
            </button>
          )}
          <span className="spacer" />
          {finished && stored.step === "collisions" ? (
            <p className="aside">{r.finished}</p>
          ) : (
            <button type="button" className="primary" onClick={next}>
              {following === undefined ? r.finish : r.stepNext(r.step[following].title)}
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
 * The collisions, one structure at a time: its two mods' structures side
 * by side, the three choices, and the way to the structure before and
 * after, decided ones included, to see or change what was decided there.
 * Deciding hides a whole structure, which decides every spot it is part
 * of, and moves on to the next spot still open.
 */
function CollisionsStep({ report }: { report: RenderedReport }) {
  const { ignored, patchChoices, setPatchChoices, spots } = useSession();
  const r = useReviewTexts();
  const [cursor, setCursor] = useState<string | null>(null);
  const [history, setHistory] = useState<PatchChoices[]>([]);
  const [structures, setStructures] = useState<[AreaObject[], AreaObject[]]>([[], []]);
  const all = useMemo(
    () => reportSpots(report, ignored, patchChoices),
    [report, ignored, patchChoices],
  );
  // Every spot that names its objects, the worst first; the order does not
  // change as spots are decided, so that moving back finds them again.
  const ordered = useMemo(
    () =>
      all
        .filter((item) => (item.spot.parts ?? []).length > 0)
        .map((item) => ({ item, key: spotKey(item.spot) ?? item.finding }))
        .sort(
          (a, b) =>
            KIND_ORDER.indexOf(a.item.kind) - KIND_ORDER.indexOf(b.item.kind) ||
            a.key.localeCompare(b.key),
        ),
    [all],
  );
  const openCount = ordered.filter(({ item }) => item.state === "open").length;
  const at = Math.max(
    0,
    ordered.findIndex(({ key }) => key === cursor) >= 0
      ? ordered.findIndex(({ key }) => key === cursor)
      : ordered.findIndex(({ item }) => item.state === "open"),
  );
  const entry = ordered[at];
  const currentKey = entry?.key ?? null;
  // biome-ignore lint/correctness/useExhaustiveDependencies: the structures belong to the spot.
  useEffect(() => setStructures([[], []]), [currentKey]);
  const hidden = useMemo(() => hiddenKeysOf(spots, patchChoices), [spots, patchChoices]);
  const decided = all.filter((item) => item.state !== "open").length;
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

  if (entry === undefined) {
    return (
      <div className="review-rows">
        <p>{r.allDecided}</p>
        {history.length > 0 && <div className="review-choices">{undo}</div>}
      </div>
    );
  }
  const current = entry.item;
  const parts = current.spot.parts ?? [];
  const plugins = [...new Set(parts.map((part) => part.object.plugin))];
  const sides: [string, string] = [plugins[0] ?? "", plugins[1] ?? plugins[0] ?? ""];
  const names: [string, string] = [short(sides[0]), short(sides[1])];
  const seeds: [typeof parts, typeof parts] = [
    parts.filter((part) => part.object.plugin === sides[0]),
    parts.filter((part) => part.object.plugin === sides[1]),
  ];
  const { place, centre } = placeOf(current.spot, report.map.worlds);
  const decision = decisionAt(patchChoices, current.finding, spotKey(current.spot), parts);
  const state =
    decision === undefined
      ? r.stateOpen
      : decision === null
        ? r.stateKept
        : r.stateHidden(short(decision));

  const go = (index: number) => {
    const target = ordered[(index + ordered.length) % ordered.length];
    if (target !== undefined) {
      setCursor(target.key);
    }
  };
  /** A side's structure, or the spot's own parts of it until the structure is known. */
  const structureOf = (side: 0 | 1) =>
    structures[side].length > 0
      ? structures[side].map((object) => ({ id: object.id, game: isGameObject(object) }))
      : seeds[side].map((part) => ({ id: part.object, game: !part.hideable }));
  /** Keeps the choices, and moves on to the next spot still open after this one. */
  const apply = (next: PatchChoices) => {
    setHistory([...history, patchChoices]);
    setPatchChoices(next);
    const after = reportSpots(report, ignored, next);
    const open = new Set(
      after
        .filter((item) => item.state === "open")
        .map((item) => spotKey(item.spot) ?? item.finding),
    );
    for (let step = 1; step <= ordered.length; step++) {
      const candidate = ordered[(at + step) % ordered.length];
      if (candidate !== undefined && open.has(candidate.key)) {
        setCursor(candidate.key);
        return;
      }
    }
    setCursor(entry.key);
  };
  const keep = (side: 0 | 1) => {
    const kept = structureOf(side).map((part) => part.id);
    const gone = structureOf(side === 0 ? 1 : 0).filter((part) => !part.game);
    let next = revealObjects(patchChoices, spots, kept);
    for (const part of gone) {
      next = chooseObject(next, part.id, true);
    }
    apply(next);
  };
  // Keeping both keeps every spot whose parts all belong to the two structures.
  const keepBoth = () => {
    const both = [...structureOf(0), ...structureOf(1)].map((part) => part.id);
    const inBoth = new Set(both.map((id) => objectKey(id.plugin, id.localId)));
    let next = revealObjects(patchChoices, spots, both);
    for (const list of spots.values()) {
      for (const spot of list) {
        const key = spotKey(spot);
        const spotParts = spot.parts ?? [];
        if (
          key !== null &&
          spotParts.length > 0 &&
          spotParts.every((part) => inBoth.has(objectKey(part.object.plugin, part.object.localId)))
        ) {
          next = chooseSpot(next, key, null);
        }
      }
    }
    const own = spotKey(current.spot);
    apply(own === null ? next : chooseSpot(next, own, null));
  };
  const gameParts = [...structureOf(0), ...structureOf(1)].filter((part) => part.game).length;
  const keptSide =
    decision === undefined || decision === null
      ? null
      : decision.toLowerCase() === sides[1].toLowerCase()
        ? 0
        : 1;

  return (
    <div className="review-collision">
      <nav className="review-structures" aria-label={r.spotOf(at + 1, ordered.length)}>
        <button type="button" className="small" onClick={() => go(at - 1)}>
          {r.previousStructure}
        </button>
        <span className="review-structures-where">
          <strong>{r.spotOf(at + 1, ordered.length)}</strong>
          <span className="aside">{r.decidedCount(decided, all.length)}</span>
        </span>
        <button type="button" className="small" onClick={() => go(at + 1)}>
          {r.nextStructure}
        </button>
      </nav>
      {openCount === 0 && <p className="aside">{r.allDecided}</p>}
      <div className="review-collision-head">
        <strong>
          {names[0]} ⟷ {names[1]}
        </strong>
        <span className="aside">
          {current.spot.objects} · <code>{current.spot.console}</code>
        </span>
        <span className="spacer" />
        <span className="review-state" data-decided={decision !== undefined}>
          {state}
        </span>
      </div>
      <CompareStage3D
        place={place}
        centre={centre}
        seeds={[seeds[0].map((part) => part.object), seeds[1].map((part) => part.object)]}
        names={names}
        hidden={hidden}
        onStructures={setStructures}
      />
      {gameParts > 0 && <p className="aside">{r.gameParts(gameParts)}</p>}
      <div className="review-choices">
        <button
          type="button"
          className="primary"
          aria-pressed={keptSide === 0}
          onClick={() => keep(0)}
        >
          {keptSide === 0 ? "✓ " : ""}
          {r.keep(names[0])}
        </button>
        <button
          type="button"
          className="primary"
          aria-pressed={keptSide === 1}
          onClick={() => keep(1)}
        >
          {keptSide === 1 ? "✓ " : ""}
          {r.keep(names[1])}
        </button>
        <button type="button" aria-pressed={decision === null} onClick={keepBoth}>
          {decision === null ? "✓ " : ""}
          {r.keepBoth}
        </button>
        <span className="spacer" />
        {undo}
      </div>
    </div>
  );
}
