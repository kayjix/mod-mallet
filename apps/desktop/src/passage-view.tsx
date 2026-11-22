// A passage (ADR 0072) in Map collisions: picked by its name in the
// worklist, it is framed in the page's one 3D view. A switch in the view
// shows the place now or with one mod's version only, and the panel above
// keeps one version for the whole passage, or both. The side-by-side
// comparison of three views stays in the guided review.

import { useEffect, useRef, useState } from "react";
import type { Api } from "./api";
import { type AreaLayer, isGamePlugin, objectKey } from "./area-layer";
import type { AreaObjects } from "./generated/area-objects";
import type { ObjectRef, RenderedReport } from "./generated/rendered-report";
import { LocateButton } from "./locate";
import { usePassageTexts } from "./passage-texts";
import { usePassageTitle } from "./passages";
import { matchKey } from "./patch";
import { groupsOf } from "./Review";
import type { Cluster, Decided } from "./review-clusters";
import { useReviewTexts } from "./review-texts";
import { findSegments, type StageSegment } from "./segments";
import { Note, NoteRow } from "./spot-notes";
import "./passage-view.css";
import { UndoButton } from "./ui/kit";

/** What the view shows of a passage: the place now, or one side's version only. */
export type PassageVersion = "now" | 0 | 1;

/** A plugin's name without its extension. */
const short = (plugin: string) => plugin.replace(/\.(esp|esm|esl)$/i, "");

/** How long the models of the place may take to arrive before measuring anyway, in steps of 400 ms. */
const PATIENCE = 75;

/**
 * The sections of `cluster` in what the view's `layer` shows of `area`
 * (see `segments.ts`), once its models are in; null until then.
 */
export function usePassageSegments(
  cluster: Cluster | null,
  layer: AreaLayer | null,
  area: AreaObjects | null,
): StageSegment[] | null {
  const [found, setFound] = useState<{ key: string; segments: StageSegment[] } | null>(null);
  const now = useRef(cluster);
  now.current = cluster;
  const key = cluster?.key ?? null;
  // biome-ignore lint/correctness/useExhaustiveDependencies: the passage is compared by its key.
  useEffect(() => {
    const measured = now.current;
    if (measured === null || layer === null || area === null) {
      return;
    }
    let tries = 0;
    let timer: number | null = null;
    const tick = () => {
      const progress = layer.progress;
      if ((progress.total > 0 && progress.done >= progress.total) || tries >= PATIENCE) {
        setFound({
          key: measured.key,
          segments: findSegments(
            layer,
            area,
            groupsOf(measured),
            measured.centre,
            measured.radius + 2048,
            measured.passage?.keys ?? null,
          ),
        });
        return;
      }
      tries += 1;
      timer = window.setTimeout(tick, 400);
    };
    tick();
    return () => {
      if (timer !== null) {
        window.clearTimeout(timer);
      }
    };
  }, [key, layer, area]);
  return found !== null && found.key === key ? found.segments : null;
}

/**
 * Of `objects` in the world or cell `place`, those the game ties in (ADR
 * 0046) and that stay when a whole passage is decided, by
 * {@link objectKey}. None where the ties cannot be read.
 */
export async function tiedIn(api: Api, place: string, objects: ObjectRef[]): Promise<Set<string>> {
  const asked = objects.filter((object) => !isGamePlugin(object.plugin));
  if (asked.length === 0) {
    return new Set();
  }
  const found = await api.objectAnchors(place, asked).catch(() => null);
  return new Set(
    (found ?? []).filter((one) => one.verdict !== "free").map((one) => matchKey(one.object)),
  );
}

/** What the last decision of a whole passage did, said right under its head. */
export interface PassageOutcome {
  /** The passage, by key. */
  key: string;
  /** The side kept, or null for both. */
  side: 0 | 1 | null;
  /** How many parts of the other side it hid. */
  hidden: number;
  /** How many of them stay, as the game ties them in. */
  held: number;
  /** The kept side's parts hidden by hand that it showed again. */
  byHand: ObjectRef[];
}

/** The switch in the 3D view's bar: the place now, or with one side's version only. */
export function VersionSwitch({
  plugins,
  version,
  onVersion,
}: {
  plugins: [string, string];
  version: PassageVersion;
  onVersion: (version: PassageVersion) => void;
}) {
  const p = usePassageTexts();
  const options: { value: PassageVersion; label: string }[] = [
    { value: "now", label: p.versionNow },
    { value: 0, label: p.version(short(plugins[0])) },
    { value: 1, label: p.version(short(plugins[1])) },
  ];
  return (
    <fieldset
      className="segmented view-tool passage-versions"
      data-size="small"
      title={p.versionHint}
      aria-label={p.versionHint}
    >
      {options.map((option) => (
        <button
          type="button"
          key={String(option.value)}
          aria-pressed={version === option.value}
          onClick={() => onVersion(option.value)}
        >
          {option.label}
        </button>
      ))}
    </fieldset>
  );
}

/**
 * The head of a passage in the spot panel: what it is, where, how far it
 * is decided, and the buttons that decide it as a whole; its sections
 * below, folded.
 */
export function PassageHead({
  cluster,
  worlds,
  decided,
  segments,
  outcome,
  pending,
  onKeep,
  onUndo,
  onHideAgain,
}: {
  cluster: Cluster;
  worlds: RenderedReport["map"]["worlds"];
  decided: Decided;
  segments: StageSegment[] | null;
  /** What the last decision of this passage did, if one was made here. */
  outcome: PassageOutcome | null;
  /** Whether a decision waits for what the game ties in. */
  pending: boolean;
  /** Keeps one side's version (0 or 1), or both (null). */
  onKeep: (side: 0 | 1 | null) => void;
  /** Takes the last decision back. */
  onUndo: () => void;
  /** Hides again the parts hidden by hand that the decision showed. */
  onHideAgain: () => void;
}) {
  const p = usePassageTexts();
  const r = useReviewTexts();
  const title = usePassageTitle();
  const passage = cluster.passage?.passage;
  if (passage === undefined) {
    return null;
  }
  const names = [short(passage.plugins[0]), short(passage.plugins[1])] as const;
  const kept =
    decided.type === "kept"
      ? null
      : decided.type === "hidden"
        ? decided.plugin.toLowerCase() === passage.plugins[1].toLowerCase()
          ? 0
          : 1
        : undefined;
  const facts = [
    p.sides(names[0], passage.parts[0].length, names[1], passage.parts[1].length),
    p.spots(cluster.spots.length),
    ...(passage.duplicates > 0 ? [p.twins(passage.duplicates)] : []),
  ];
  const state =
    decided.type === "open"
      ? r.stateOpen
      : decided.type === "kept"
        ? r.stateKept
        : decided.type === "hidden"
          ? r.stateHidden(short(decided.plugin))
          : decided.open === 0
            ? p.withExceptions
            : r.stateMixed(decided.open);
  return (
    <div className="passage-head">
      <div className="work-nav passage-nav">
        <span className="work-kind" data-kind={cluster.kind}>
          {p.kind[passage.severity]}
        </span>
        <strong>{title(passage, worlds)}</strong>
        <span className="aside">{facts.join(" · ")}</span>
        <NoteRow>
          <Note tone="info" summary={p.whyShort} hint={`${p.why[passage.severity]} ${p.bounded}`}>
            <p className="aside">
              {p.why[passage.severity]} {p.bounded}
            </p>
          </Note>
          {outcome !== null && outcome.held > 0 && (
            <Note
              tone="warn"
              summary={p.held(outcome.held)}
              hint={`${p.held(outcome.held)}. ${p.heldHint}`}
            >
              <p className="aside">
                {p.held(outcome.held)}. {p.heldHint}
              </p>
            </Note>
          )}
        </NoteRow>
        <span className="spacer" />
        <span
          className={`work-status ${decided.type === "open" ? "open" : decided.type === "kept" ? "kept" : "hidden"}`}
        >
          {state}
        </span>
        <span className="work-nav-group passage-keep">
          {([0, 1] as const).map((side) => (
            <button
              type="button"
              key={side}
              className="small"
              aria-pressed={kept === side}
              disabled={pending}
              onClick={() => onKeep(side)}
            >
              {p.keepVersion(names[side])}
            </button>
          ))}
          <button
            type="button"
            className="small"
            aria-pressed={kept === null}
            disabled={pending}
            onClick={() => onKeep(null)}
          >
            {r.keepBoth}
          </button>
        </span>
      </div>
      {pending && (
        <p className="passage-done" role="status">
          {p.deciding}
        </p>
      )}
      {!pending && outcome !== null && (
        <p className="passage-done" role="status">
          <span>
            {outcome.side === null
              ? p.keptBoth
              : p.keptDone(names[outcome.side], outcome.hidden, names[outcome.side === 0 ? 1 : 0])}
          </span>
          {outcome.side !== null && outcome.byHand.length > 0 && (
            <span>
              {p.byHand(outcome.byHand.length, names[outcome.side])}{" "}
              <button type="button" className="small quiet" onClick={onHideAgain}>
                {p.hideAgain}
              </button>
            </span>
          )}
          <UndoButton onClick={onUndo} />
        </p>
      )}
      <details className="passage-sections">
        <summary>{segments === null ? p.measuring : p.sections(segments.length)}</summary>
        {segments !== null && (
          <ul>
            {segments.map((segment, index) => (
              <li key={segment.groups.join("|") || index}>
                <span>
                  {p.section(
                    index + 1,
                    names[0],
                    segment.parts[0].length,
                    names[1],
                    segment.parts[1].length,
                  )}
                </span>
                <LocateButton
                  objects={[...segment.parts[0], ...segment.parts[1]].map((part) => part.id)}
                />
              </li>
            ))}
          </ul>
        )}
      </details>
    </div>
  );
}
