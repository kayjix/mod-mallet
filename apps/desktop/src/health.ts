// How healthy a setup is, as one number and one verdict (ADR 0045).
//
// Nothing here finds anything: the findings and the collisions come from
// the Rust side. This only weighs what is still open, so that someone who
// has never opened a modding tool sees at once whether the game is likely
// to crash, will look wrong in places, or is fine.

import { ADVICE_KINDS, isCollision } from "./arrange";
import type { MapSpot, RenderedFinding, RenderedReport } from "./generated/rendered-report";
import { kindOf, type WorkKind } from "./map-places";
import { decisionAt, hiddenAt, installedKeys, keyOf, type PatchChoices, spotKey } from "./patch";

/** What a kind of finding does to the game, the worst first. */
export type Impact = "crash" | "broken" | "quality" | "note" | "advice" | "scan";

/** How a kind of finding counts: what it does, and how much one of it weighs. */
export interface Weight {
  impact: Impact;
  /** Points one certain finding takes off. */
  points: number;
  /** Whether one open finding alone makes crashes likely. */
  severe?: boolean;
}

/** From this crossing on, in game units (2 m), a touch counts as deep (ADR 0040). */
const DEEP = 140;

/** How much a finding counts for how sure Mod Mallet is. */
const CONFIDENCE: Record<RenderedFinding["finding"]["confidence"], number> = {
  certain: 1,
  likely: 0.8,
  hint: 0.4,
};

/** The most each impact can take off, so that one sort cannot hide the others. */
const CAP: Record<Impact, number> = {
  crash: 60,
  broken: 30,
  quality: 15,
  note: 5,
  advice: 0,
  scan: 0,
};

/** The most the collisions can take off together. */
const COLLISION_CAP = 30;

/** How much one open spot weighs, by its kind in the worklist. */
const SPOT_WEIGHT: Record<WorkKind, number> = {
  duplicate: 1,
  overlap: 0.4,
  deep: 0.15,
  touch: 0.01,
};

/** Turns the weighed spots into points: √ so that hundreds do not count a hundredfold. */
const COLLISION_SCALE = 1.5;

/** Below this score crashes are likely, whatever the reason. */
const CRASH_BELOW = 40;

/** Below this score the game is playable, but the quality suffers. */
const GOOD_FROM = 80;

const numberArg = (rendered: RenderedFinding, name: string): number => {
  const value = rendered.finding.args[name];
  return typeof value === "number" ? value : 0;
};

/** How a finding counts, from its kind; unknown kinds by their severity. */
export function weightOf(rendered: RenderedFinding): Weight {
  const { kind, severity, confidence } = rendered.finding;
  if (ADVICE_KINDS.has(kind)) {
    return { impact: "advice", points: 0 };
  }
  // Crash logs (ADR 0044): the game did crash.
  if (kind.startsWith("crash.") || kind.startsWith("crashes.")) {
    return { impact: "crash", points: 25, severe: severity !== "info" };
  }
  switch (kind) {
    case "plugins.master-missing":
      return { impact: "crash", points: 30, severe: true };
    case "navmeshes.door-cut-off":
      return { impact: "crash", points: 8, severe: true };
    case "navmeshes.changes-undone":
      return { impact: "crash", points: 1.5 };
    case "plugins.needs-cleaning":
      // Deleted navigation meshes are a well-known cause of crashes.
      return numberArg(rendered, "navmeshes") > 0
        ? { impact: "crash", points: 10, severe: true }
        : { impact: "quality", points: 3 };
    case "meshes.made-for-other-game":
      // Skinned meshes of another format crash the game when they are shown.
      return numberArg(rendered, "skinned") > 0
        ? { impact: "crash", points: 12, severe: true }
        : { impact: "broken", points: 6 };
    case "extender.plugin-incompatible":
    case "extender.plugin-not-loaded":
      return { impact: "broken", points: 12 };
    case "plugins.requirement-missing":
    case "plugins.incompatible":
      return { impact: "broken", points: 8 };
    case "npcs.face-mismatch":
    case "meshes.texture-missing":
      return { impact: "broken", points: 6 };
    case "plugins.generated-outdated":
      return { impact: "quality", points: 4 };
    case "meshes.retexture-bypassed":
    case "installer.options-now-recommended":
    case "plugins.loot-warning":
      return { impact: "quality", points: 3 };
    case "plugins.identical-records":
      return { impact: "note", points: 0.5 };
    case "setup.file-unreadable":
    case "setup.load-order-unknown":
      return { impact: "scan", points: 0 };
  }
  if (severity === "info") {
    return { impact: "note", points: 0 };
  }
  if (confidence === "hint") {
    return { impact: "quality", points: 2 };
  }
  return severity === "error" ? { impact: "broken", points: 10 } : { impact: "quality", points: 3 };
}

/** A spot's kind in the worklist: a touch that crosses far counts as deep. */
export function workKindOf(spot: MapSpot): WorkKind {
  const kind = kindOf(spot);
  return kind === "touch" && (spot.depth ?? 0) >= DEEP ? "deep" : kind;
}

/** Where a spot of a collision stands. */
export type SpotState = "open" | "kept" | "waiting" | "patched";

/** One spot of a collision with where it stands. */
export interface CollisionSpot {
  spot: MapSpot;
  finding: string;
  kind: WorkKind;
  state: SpotState;
}

/**
 * The spots of the collisions not set aside, once each, with where they
 * stand: nothing decided yet, both kept, one side to be hidden by a patch
 * that is not built yet, or hidden by the patch in the game.
 */
export function collisionSpots(
  findings: RenderedFinding[],
  spots: MapSpot[],
  /** The objects the patch in the game hides, by {@link keyOf}. */
  installed: Set<string>,
  ignored: Set<string>,
  choices: PatchChoices,
): CollisionSpot[] {
  const open = new Set(
    findings
      .filter((rendered) => isCollision(rendered) && !ignored.has(rendered.finding.id))
      .map((rendered) => rendered.finding.id),
  );
  const seen = new Set<string>();
  const found: CollisionSpot[] = [];
  for (const spot of spots) {
    if (!open.has(spot.finding)) {
      continue;
    }
    const key = spotKey(spot);
    const identity = key ?? `${spot.finding}|${spot.console}|${spot.objects}`;
    if (seen.has(identity)) {
      continue;
    }
    seen.add(identity);
    const decided = decisionAt(choices, spot.finding, key);
    let state: SpotState;
    if (decided === undefined) {
      state = "open";
    } else if (decided === null) {
      state = "kept";
    } else {
      const hidden = hiddenAt(spot.parts ?? [], decided);
      state =
        hidden.length > 0 && hidden.every((object) => installed.has(keyOf(object)))
          ? "patched"
          : "waiting";
    }
    found.push({ spot, finding: spot.finding, kind: workKindOf(spot), state });
  }
  return found;
}

/** The collision spots of a report with where they stand (see {@link collisionSpots}). */
export function reportSpots(
  report: RenderedReport,
  ignored: Set<string>,
  choices: PatchChoices,
): CollisionSpot[] {
  return collisionSpots(report.findings, report.map.spots, installedKeys(report), ignored, choices);
}

/** Where a reason can be acted on. */
export type Target = "tasks" | "collisions" | "patch" | "mods";

/** One reason the score is lower than 100. */
export interface Reason {
  key: string;
  impact: Impact | "collisions";
  /** What it is about: a kind's title, or a sort of collision spots. */
  kind: "findings" | "open-spots" | "waiting-spots";
  title: string;
  count: number;
  /** Points it takes off, after the caps. */
  points: number;
  target: Target;
}

/** What the setup is like, in one word. */
export type Verdict = "good" | "suffers" | "crashes";

export interface Health {
  score: number;
  verdict: Verdict;
  /** The reasons, the heaviest first. */
  reasons: Reason[];
  /** Whether Mod Mallet could not read everything, so the score may be too kind. */
  incomplete: boolean;
  /** Findings set aside, which do not count. */
  setAside: number;
  /** Optional recommendations, which do not count either. */
  advice: number;
  /** The collision spots by where they stand. */
  spots: Record<SpotState, number>;
}

/**
 * The health of a setup: 100 minus what the open findings and the
 * undecided collisions take off, each sort up to its cap (ADR 0045).
 */
export function healthOf(
  report: RenderedReport,
  ignored: Set<string>,
  choices: PatchChoices,
): Health {
  const reasons: Reason[] = [];
  let incomplete = false;
  let advice = 0;
  let severe = false;

  // Findings of one kind count together: √n of them, so that dozens of
  // notes do not count as much as dozens of crashes.
  const byKind = new Map<string, { title: string; weight: Weight; sum: number; count: number }>();
  for (const rendered of report.findings) {
    if (ignored.has(rendered.finding.id) || isCollision(rendered)) {
      continue;
    }
    const weight = weightOf(rendered);
    if (weight.impact === "advice") {
      advice += 1;
      continue;
    }
    if (weight.impact === "scan") {
      incomplete = true;
    }
    severe ||= weight.severe === true && rendered.finding.confidence !== "hint";
    const points = weight.points * CONFIDENCE[rendered.finding.confidence];
    const key = `${weight.impact}|${rendered.finding.kind}`;
    const entry = byKind.get(key) ?? { title: rendered.title, weight, sum: 0, count: 0 };
    entry.sum += points;
    entry.count += 1;
    byKind.set(key, entry);
  }
  const raw: Reason[] = [...byKind.entries()].map(([key, entry]) => ({
    key,
    impact: entry.weight.impact,
    kind: "findings",
    title: entry.title,
    count: entry.count,
    points: (entry.sum / entry.count) * Math.sqrt(entry.count),
    target: "tasks",
  }));
  // Each impact up to its cap; its reasons shrink alike.
  for (const impact of Object.keys(CAP) as Impact[]) {
    const members = raw.filter((reason) => reason.impact === impact);
    const total = members.reduce((sum, reason) => sum + reason.points, 0);
    const factor = total > CAP[impact] ? CAP[impact] / total : 1;
    for (const reason of members) {
      reasons.push({ ...reason, points: reason.points * factor });
    }
  }

  const spots = reportSpots(report, ignored, choices);
  const states: Record<SpotState, number> = { open: 0, kept: 0, waiting: 0, patched: 0 };
  let openWeight = 0;
  let waitingWeight = 0;
  for (const spot of spots) {
    states[spot.state] += 1;
    if (spot.state === "open") {
      openWeight += SPOT_WEIGHT[spot.kind];
    } else if (spot.state === "waiting") {
      // Decided, but the game does not have it until the patch is built.
      waitingWeight += SPOT_WEIGHT[spot.kind] / 2;
    }
  }
  const collisionPoints = Math.min(
    COLLISION_CAP,
    COLLISION_SCALE * Math.sqrt(openWeight + waitingWeight),
  );
  const share = (part: number) =>
    openWeight + waitingWeight === 0 ? 0 : (collisionPoints * part) / (openWeight + waitingWeight);
  if (states.open > 0) {
    reasons.push({
      key: "collisions|open",
      impact: "collisions",
      kind: "open-spots",
      title: "",
      count: states.open,
      points: share(openWeight),
      target: "collisions",
    });
  }
  if (states.waiting > 0) {
    reasons.push({
      key: "collisions|waiting",
      impact: "collisions",
      kind: "waiting-spots",
      title: "",
      count: states.waiting,
      points: share(waitingWeight),
      target: "patch",
    });
  }

  const lost = reasons.reduce((sum, reason) => sum + reason.points, 0);
  const score = Math.max(0, Math.min(100, Math.round(100 - lost)));
  const broken = reasons.some((reason) => reason.impact === "broken" && reason.points > 0);
  const verdict: Verdict =
    severe || score < CRASH_BELOW ? "crashes" : broken || score < GOOD_FROM ? "suffers" : "good";
  return {
    score,
    verdict,
    reasons: reasons
      .filter((reason) => reason.points >= 0.5)
      .sort((a, b) => b.points - a.points || a.title.localeCompare(b.title)),
    incomplete,
    setAside: report.findings.filter(({ finding }) => ignored.has(finding.id)).length,
    advice,
    spots: states,
  };
}
