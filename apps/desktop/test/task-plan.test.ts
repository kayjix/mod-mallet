// Tests of how the task page orders and groups findings (ADR 0076), with
// made-up findings of made-up mods.

import assert from "node:assert/strict";
import { test } from "node:test";
import type { RenderedFinding } from "../src/generated/rendered-report";
import type { Impact } from "../src/health";
import { partOf, planTasks, rowModsOf } from "../src/task-plan.ts";

type Severity = RenderedFinding["finding"]["severity"];
type Confidence = RenderedFinding["finding"]["confidence"];

let serial = 0;

/** A finding of `kind` about `mod`, with whatever else a test needs. */
function finding(
  kind: string,
  mod: string,
  {
    severity = "warning",
    confidence = "likely",
    args = {},
    direct = false,
  }: {
    severity?: Severity;
    confidence?: Confidence;
    args?: Record<string, string | number>;
    direct?: boolean;
  } = {},
): RenderedFinding {
  serial += 1;
  return {
    actions: direct
      ? [
          {
            action: { type: "reinstall-mod", mod: { source: "vortex", id: mod, name: mod } },
            label: "Reinstall",
            description: "",
            guide: [],
          } as unknown as RenderedFinding["actions"][number],
        ]
      : [],
    confidenceLabel: "",
    explanation: "",
    finding: {
      id: `${kind}:${serial}`,
      kind,
      severity,
      confidence,
      args,
      subject: { type: "file", path: `data/${mod}.esp` },
      provider: { source: "vortex", id: mod, name: mod },
    } as RenderedFinding["finding"],
    headline: `${mod}: ${kind}`,
    remedy: "",
    severityLabel: "",
    summary: "",
    title: `Title of ${kind}`,
  };
}

/** Doors cut off can crash; faces are wrong in the game; the rest costs quality. */
const impactOf = (rendered: RenderedFinding): Impact => {
  switch (rendered.finding.kind) {
    case "navmeshes.door-cut-off":
    case "plugins.master-missing":
      return "crash";
    case "npcs.face-mismatch":
      return "broken";
    case "mods.collides-widely":
      return "advice";
    default:
      return rendered.finding.severity === "info" ? "note" : "quality";
  }
};

const none = new Set<string>();

test("crash risks come first, whatever the user does about them", () => {
  const doors = finding("navmeshes.door-cut-off", "Town");
  const master = finding("plugins.master-missing", "Patch", { severity: "error", direct: true });
  const faces = finding("npcs.face-mismatch", "Faces");
  const textures = finding("meshes.retexture-bypassed", "Trees");

  const parts = planTasks([textures, faces, doors, master], none, impactOf);

  assert.deepEqual(
    parts.map(({ id, count }) => [id, count]),
    [
      ["crash", 2],
      ["choose", 2],
    ],
  );
  // Within the crash risks, the one with a button for the mod manager first.
  assert.deepEqual(
    parts[0]?.groups.map(({ kind }) => kind),
    ["plugins.master-missing", "navmeshes.door-cut-off"],
  );
  // Within a step, what is wrong in the game before what costs quality.
  assert.deepEqual(
    parts[1]?.groups.map(({ kind, impact }) => [kind, impact]),
    [
      ["npcs.face-mismatch", "broken"],
      ["meshes.retexture-bypassed", "quality"],
    ],
  );
});

test("findings of one kind share a group, the mods named most often first", () => {
  const first = finding("navmeshes.door-cut-off", "Town", { args: { count: 1 } });
  const second = finding("navmeshes.door-cut-off", "Castle", { args: { count: 5 } });
  const third = finding("navmeshes.door-cut-off", "Castle", { args: { count: 2 } });

  const [part] = planTasks([first, second, third], none, impactOf);
  const [group] = part?.groups ?? [];

  assert.equal(part?.groups.length, 1);
  assert.equal(group?.key, "crash/navmeshes.door-cut-off");
  assert.equal(group?.title, "Title of navmeshes.door-cut-off");
  assert.equal(group?.impact, "crash");
  assert.deepEqual(group?.mods, ["Castle", "Town"]);
  // The findings that concern the most things first.
  assert.deepEqual(
    group?.findings.map(({ finding }) => finding.args.count),
    [5, 2, 1],
  );
});

test("set aside, notes and recommendations are never crash risks", () => {
  const aside = finding("navmeshes.door-cut-off", "Town");
  const note = finding("navmeshes.door-cut-off", "Old", { severity: "info" });
  const guess = finding("navmeshes.door-cut-off", "Maybe", { confidence: "hint" });
  const ignored = new Set([aside.finding.id]);

  assert.equal(partOf(aside, ignored, impactOf), "ignored");
  assert.equal(partOf(note, none, impactOf), "notes");
  assert.equal(partOf(guess, none, impactOf), "advice");
  assert.deepEqual(
    planTasks([aside, note, guess], ignored, impactOf).map(({ id }) => id),
    ["advice", "notes", "ignored"],
  );
});

test("collisions are left to their own page", () => {
  const collision = finding("objects.duplicated", "Town");
  assert.equal(partOf(collision, none, impactOf), null);
  assert.deepEqual(planTasks([collision], none, impactOf), []);
});

test("a row names the mods of meshes and textures as well", () => {
  const bypassed = finding("meshes.retexture-bypassed", "Trees", {
    args: { meshMod: "Better Trees", textureMod: "Bark Textures" },
  });
  assert.deepEqual(rowModsOf(bypassed), ["Trees", "Better Trees", "Bark Textures"]);
});

test("a plugin beside the mod that brings it is not named twice", () => {
  const loaded = finding("plugins.switched-off-but-loaded", "Busy Streets", {
    args: { plugin: "Busy Streets.esp", mod: "Busy Streets" },
  });
  assert.deepEqual(rowModsOf(loaded), ["Busy Streets"]);
  // Where only plugins are named, they stand for their mods.
  const stacked = {
    ...finding("doors.stacked", "unused", { args: { first: "A.esm", second: "B.esp" } }),
  };
  stacked.finding = { ...stacked.finding, provider: null };
  assert.deepEqual(rowModsOf(stacked), ["A.esm", "B.esp"]);
});
