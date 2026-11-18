// How a report's findings are arranged into a task list.
//
// Nothing here decides anything about a setup: what a finding is, how
// severe it is and what can be done about it comes from the Rust side.
// This only sorts what is there into the sections and groups of the window.

import type { ModInfo, RenderedFinding } from "./generated/rendered-report";

/** What the task list shows. `open` is everything that is not a note. */
export type Filter = "open" | "error" | "warning" | "direct" | "info" | "ignored" | "collisions";

export type Severity = RenderedFinding["finding"]["severity"];

/** How many findings there are of each sort, without those set aside. */
export interface Counts {
  errors: number;
  warnings: number;
  notes: number;
  /** Findings with an action that can be started from the app. */
  direct: number;
  /** Findings the user has set aside. */
  ignored: number;
  /** Findings about objects of two mods in the same spot. */
  collisions: number;
  /** What the task page lists as to do: neither collisions, notes nor recommendations. */
  tasks: number;
}

/** The findings of one kind. */
export interface Group {
  kind: string;
  title: string;
  findings: RenderedFinding[];
}

/** One part of the task list. */
export interface Section {
  id: Exclude<Filter, "open">;
  groups: Group[];
  count: number;
}

/** Whether a finding is about objects of two mods in the same spot. */
export function isCollision(rendered: RenderedFinding): boolean {
  const { kind } = rendered.finding;
  return kind.startsWith("objects.") || kind === "plugins.contained";
}

export function isDirect(rendered: RenderedFinding): boolean {
  return rendered.actions.length > 0;
}

export function countFindings(findings: RenderedFinding[], ignored: Set<string>): Counts {
  const active = findings.filter(({ finding }) => !ignored.has(finding.id));
  const of = (severity: Severity) =>
    active.filter(({ finding }) => finding.severity === severity).length;
  return {
    errors: of("error"),
    warnings: of("warning"),
    notes: of("info"),
    direct: active.filter(isDirect).length,
    ignored: findings.length - active.length,
    collisions: active.filter(isCollision).length,
    tasks: openTaskCount(findings, ignored),
  };
}

/** Findings of the same kind together, kinds in the order of the report. */
function groupByKind(findings: RenderedFinding[]): Group[] {
  const groups = new Map<string, Group>();
  for (const rendered of findings) {
    const { kind } = rendered.finding;
    const group = groups.get(kind);
    if (group === undefined) {
      groups.set(kind, { kind, title: rendered.title, findings: [rendered] });
    } else {
      group.findings.push(rendered);
    }
  }
  for (const group of groups.values()) {
    // Within a group, the findings that concern the most things come first.
    group.findings.sort((a, b) => countOf(b) - countOf(a));
  }
  // What can be done from here comes first within a section.
  return [...groups.values()].sort(
    (a, b) => Number(b.findings.some(isDirect)) - Number(a.findings.some(isDirect)),
  );
}

/** How many things a finding concerns, where its kind counts them. */
function countOf(rendered: RenderedFinding): number {
  const count = rendered.finding.args.count;
  return typeof count === "number" ? count : 0;
}

/** The sections to show for a filter, without the empty ones. */
export function sectionsFor(
  findings: RenderedFinding[],
  filter: Filter,
  ignored: Set<string>,
): Section[] {
  const section = (id: Section["id"], members: RenderedFinding[]): Section => ({
    id,
    groups: groupByKind(members),
    count: members.length,
  });
  if (filter === "collisions") {
    const found = findings.filter(
      (rendered) => isCollision(rendered) && !ignored.has(rendered.finding.id),
    );
    return found.length > 0 ? [section("collisions", found)] : [];
  }
  if (filter === "ignored") {
    const aside = findings.filter(({ finding }) => ignored.has(finding.id));
    return aside.length > 0 ? [section("ignored", aside)] : [];
  }
  const active = findings.filter(({ finding }) => !ignored.has(finding.id));
  const bySeverity = (severity: Severity) =>
    active.filter(({ finding }) => finding.severity === severity);

  const sections =
    filter === "open"
      ? [section("error", bySeverity("error")), section("warning", bySeverity("warning"))]
      : filter === "direct"
        ? [section("direct", active.filter(isDirect))]
        : [section(filter, bySeverity(filter))];
  return sections.filter(({ count }) => count > 0);
}

/** The names of the mods a finding is about. */
export function modsOf(rendered: RenderedFinding): string[] {
  const names = new Set<string>();
  if (rendered.finding.provider != null) {
    names.add(rendered.finding.provider.name);
  }
  // Findings about two mods name them as their first and second argument,
  // or as the mod contained in another and the one containing it.
  for (const key of ["first", "second", "plugin", "container", "mod"]) {
    const value = rendered.finding.args[key];
    if (typeof value === "string") {
      names.add(value);
    }
  }
  return [...names];
}

/** The mods by their name, to look up what a finding names. */
export function modsByName(mods: ModInfo[]): Map<string, ModInfo> {
  return new Map(mods.map((info) => [info.mod.name, info]));
}

/**
 * What the user does about a finding, for the task page: the steps in the
 * order the page shows them. Collisions are not among them; they have a
 * page of their own.
 */
export type Step =
  | "one-click"
  | "install"
  | "update"
  | "choose"
  | "scan"
  | "check"
  | "advice"
  | "notes"
  | "ignored";

export const STEPS: Step[] = [
  "one-click",
  "install",
  "update",
  "choose",
  "scan",
  "check",
  "advice",
  "notes",
  "ignored",
];

/** Kinds of mod-level recommendations (ADR 0043): optional, never required. */
export const ADVICE_KINDS = new Set([
  "mods.collides-widely",
  "mods.mostly-overridden",
  "mods.duplicates-another",
]);

/** The step of the kinds whose remedy is the same for every finding of them. */
const STEP_OF_KIND: Record<string, Step> = {
  "plugins.master-missing": "install",
  "plugins.requirement-missing": "install",
  "meshes.texture-missing": "install",
  "extender.plugin-incompatible": "update",
  "extender.plugin-not-loaded": "update",
  "meshes.made-for-other-game": "update",
  "plugins.generated-outdated": "update",
  "plugins.needs-cleaning": "update",
  "installer.options-now-recommended": "update",
  "plugins.incompatible": "choose",
  "npcs.face-mismatch": "choose",
  "meshes.retexture-bypassed": "choose",
  "navmeshes.door-cut-off": "choose",
  "navmeshes.changes-undone": "choose",
  "setup.file-unreadable": "scan",
  "setup.load-order-unknown": "scan",
};

/**
 * The step a finding belongs to; null for collisions. What Mod Mallet can
 * hand to the mod manager comes first, recommendations and guesses are
 * optional, notes are only to know.
 */
export function stepOf(rendered: RenderedFinding, ignored: Set<string>): Step | null {
  const { id, kind, severity, confidence } = rendered.finding;
  if (isCollision(rendered)) {
    return null;
  }
  if (ignored.has(id)) {
    return "ignored";
  }
  if (ADVICE_KINDS.has(kind)) {
    return "advice";
  }
  if (severity === "info") {
    return "notes";
  }
  if (confidence === "hint") {
    return "advice";
  }
  if (isDirect(rendered)) {
    return "one-click";
  }
  return STEP_OF_KIND[kind] ?? "check";
}

/** How many things there are to do outside the collisions: the steps before the optional ones. */
export function openTaskCount(findings: RenderedFinding[], ignored: Set<string>): number {
  return findings.filter((rendered) => {
    const step = stepOf(rendered, ignored);
    return step !== null && step !== "advice" && step !== "notes" && step !== "ignored";
  }).length;
}

/** The findings of each step, grouped by kind, the errors first. */
export function stepsFor(
  findings: RenderedFinding[],
  ignored: Set<string>,
): { step: Step; groups: Group[]; count: number }[] {
  const byStep = new Map<Step, RenderedFinding[]>();
  for (const rendered of findings) {
    const step = stepOf(rendered, ignored);
    if (step !== null) {
      byStep.set(step, [...(byStep.get(step) ?? []), rendered]);
    }
  }
  const rank = (group: Group) =>
    Math.min(
      ...group.findings.map(({ finding }) =>
        finding.severity === "error" ? 0 : finding.severity === "warning" ? 1 : 2,
      ),
    );
  return STEPS.flatMap((step) => {
    const members = byStep.get(step) ?? [];
    if (members.length === 0) {
      return [];
    }
    const groups = groupByKind(members).sort((a, b) => rank(a) - rank(b));
    return [{ step, groups, count: members.length }];
  });
}
