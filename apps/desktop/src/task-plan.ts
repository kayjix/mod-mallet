// How the task page orders and groups what there is to do (ADR 0076).
//
// Nothing here decides anything about a setup: what a finding is, how
// severe it is and what can be done about it comes from the Rust side, and
// what it does to the game from the weights of the health score. This only
// sorts the findings into the parts of the page, crash risks first, and
// puts findings of one kind behind one row. Pure, so that a script can test
// it without a window.

import { isDirect, modsOf, STEPS, type Step, stepOf } from "./arrange.ts";
import type { RenderedFinding } from "./generated/rendered-report";
import type { Impact } from "./health";

/** A part of the task page: the crash risks, then the steps of `arrange.ts`. */
export type PartId = "crash" | Step;

/** The parts in the order the page shows them. */
export const PARTS: PartId[] = ["crash", ...STEPS];

/** Steps whose findings are not things to do: optional, only to know, or set aside. */
const NOT_TO_DO = new Set<Step>(["advice", "notes", "ignored"]);

/** What a finding does to the game, the worst first. */
const IMPACT_RANK: Record<Impact, number> = {
  crash: 0,
  broken: 1,
  quality: 2,
  scan: 3,
  note: 4,
  advice: 5,
};

/** Findings of one kind in one part, behind one row when there are several. */
export interface PlannedGroup {
  /** Unique on the page: the part and the kind. */
  key: string;
  kind: string;
  /** The kind's title, for the row that holds several findings. */
  title: string;
  findings: RenderedFinding[];
  /** The mods the findings are about, the one named most often first. */
  mods: string[];
  /** The worst that any of the findings does to the game. */
  impact: Impact;
}

/** One part of the page with its groups. */
export interface PlannedPart {
  id: PartId;
  groups: PlannedGroup[];
  /** How many findings the part holds. */
  count: number;
}

/** File endings that make a plugin's or archive's name, not a mod's. */
const FILE_ENDING = /\.(esp|esm|esl|bsa|ba2|dll)$/i;

/**
 * The names a finding's row shows as its mods: those `modsOf` knows, and
 * the mods that findings about meshes, textures and archives name. Names
 * of plugins and archives only where no mod is named.
 */
export function rowModsOf(rendered: RenderedFinding): string[] {
  const names = new Set(modsOf(rendered));
  for (const key of ["meshMod", "textureMod", "archiveMod", "winnerMod"]) {
    const value = rendered.finding.args[key];
    if (typeof value === "string" && value !== "") {
      names.add(value);
    }
  }
  // A plugin or archive beside the mod that brings it says nothing more.
  const all = [...names];
  const mods = all.filter((name) => !FILE_ENDING.test(name));
  return mods.length > 0 ? mods : all;
}

/** How many things a finding concerns, where its kind counts them. */
function countOf(rendered: RenderedFinding): number {
  const count = rendered.finding.args.count;
  return typeof count === "number" ? count : 0;
}

/**
 * The part a finding belongs to; null for collisions, which have a page of
 * their own. A thing to do that can crash the game comes first, whatever
 * the user does about it.
 */
export function partOf(
  rendered: RenderedFinding,
  ignored: Set<string>,
  impactOf: (rendered: RenderedFinding) => Impact,
): PartId | null {
  const step = stepOf(rendered, ignored);
  if (step === null) {
    return null;
  }
  return !NOT_TO_DO.has(step) && impactOf(rendered) === "crash" ? "crash" : step;
}

/**
 * The parts of the task page with their groups, without the empty ones.
 * Within a part, the groups that do the worst come first, then those with
 * a button for the mod manager, then the larger ones; within a group, the
 * findings with such a button, then those that concern the most things.
 */
export function planTasks(
  findings: RenderedFinding[],
  ignored: Set<string>,
  impactOf: (rendered: RenderedFinding) => Impact,
): PlannedPart[] {
  const byPart = new Map<PartId, RenderedFinding[]>();
  for (const rendered of findings) {
    const part = partOf(rendered, ignored, impactOf);
    if (part !== null) {
      byPart.set(part, [...(byPart.get(part) ?? []), rendered]);
    }
  }
  return PARTS.flatMap((id) => {
    const members = byPart.get(id) ?? [];
    return members.length === 0
      ? []
      : [{ id, groups: groupsOf(id, members, impactOf), count: members.length }];
  });
}

function groupsOf(
  part: PartId,
  findings: RenderedFinding[],
  impactOf: (rendered: RenderedFinding) => Impact,
): PlannedGroup[] {
  const byKind = new Map<string, RenderedFinding[]>();
  for (const rendered of findings) {
    const { kind } = rendered.finding;
    byKind.set(kind, [...(byKind.get(kind) ?? []), rendered]);
  }
  const groups = [...byKind].map(([kind, members]): PlannedGroup => {
    const sorted = members
      .map((rendered, index) => ({ rendered, index }))
      .sort(
        (a, b) =>
          Number(isDirect(b.rendered)) - Number(isDirect(a.rendered)) ||
          countOf(b.rendered) - countOf(a.rendered) ||
          a.index - b.index,
      )
      .map(({ rendered }) => rendered);
    return {
      key: `${part}/${kind}`,
      kind,
      title: sorted[0]?.title ?? kind,
      findings: sorted,
      mods: modsByMention(sorted),
      impact: worstOf(sorted, impactOf),
    };
  });
  return groups
    .map((group, index) => ({ group, index }))
    .sort(
      (a, b) =>
        IMPACT_RANK[a.group.impact] - IMPACT_RANK[b.group.impact] ||
        Number(b.group.findings.some(isDirect)) - Number(a.group.findings.some(isDirect)) ||
        b.group.findings.length - a.group.findings.length ||
        a.index - b.index,
    )
    .map(({ group }) => group);
}

/** The worst impact among findings; a note where there are none. */
function worstOf(
  findings: RenderedFinding[],
  impactOf: (rendered: RenderedFinding) => Impact,
): Impact {
  let worst: Impact = "note";
  let rank = Number.POSITIVE_INFINITY;
  for (const rendered of findings) {
    const impact = impactOf(rendered);
    if (IMPACT_RANK[impact] < rank) {
      worst = impact;
      rank = IMPACT_RANK[impact];
    }
  }
  return worst;
}

/** The mods findings name, the one named most often first, ties by first mention. */
function modsByMention(findings: RenderedFinding[]): string[] {
  const counts = new Map<string, number>();
  for (const rendered of findings) {
    for (const name of rowModsOf(rendered)) {
      counts.set(name, (counts.get(name) ?? 0) + 1);
    }
  }
  // A Map keeps the order of first mention, and the sort is stable.
  return [...counts].sort((a, b) => b[1] - a[1]).map(([name]) => name);
}
