// What the comparison of files other than textures and meshes works out
// (ADR 0066): the kind of a file by its name, which version each side of a
// duel brings, whether they are the same, and how two texts differ.

import type { FileKind } from "./compare-files-texts";
import type { ComparedVersion, FileComparison, ScriptFunction } from "./generated/file-comparison";
import type { ModDuel } from "./generated/rendered-report";

/** Whether a file is shown on the stage as a picture or a 3D model. */
export function isPictured(path: string): boolean {
  return /\.(dds|nif)$/i.test(path);
}

/** The kind of a file below the data directory, by its name and folder. */
export function kindOf(path: string): FileKind {
  const lower = path.toLowerCase().replace(/\\/g, "/");
  const extension = lower.includes(".") ? (lower.split(".").pop() ?? "") : "";
  const inside = (folder: string) => lower.startsWith(folder);
  switch (extension) {
    case "pex":
      return "script";
    case "psc":
      return "source";
    case "swf":
      return "menu";
    case "seq":
      return "questStart";
    case "hkx":
      return "animation";
    case "dll":
      return "extender";
    case "esp":
    case "esm":
    case "esl":
      return "plugin";
    case "wav":
    case "xwm":
    case "mp3":
    case "ogg":
      return inside("sound/voice/") ? "voice" : "sound";
    case "fuz":
      return "voice";
    case "lip":
      return "lip";
    case "tri":
      return "faceShapes";
    case "bgsm":
    case "bgem":
      return "material";
    case "png":
    case "jpg":
    case "jpeg":
    case "tga":
    case "bmp":
      return "picture";
    case "json":
    case "ini":
    case "toml":
    case "yaml":
    case "yml":
    case "cfg":
    case "xml":
      return inside("skse/") ? "extenderSettings" : "settings";
    case "txt":
      if (inside("interface/translations/")) {
        return "menuText";
      }
      return inside("skse/") ? "extenderSettings" : "text";
    case "md":
    case "log":
    case "html":
    case "csv":
      return "text";
    default:
      return "other";
  }
}

/** The version of a file that one side of a duel brings, and how it came to be chosen. */
export interface SideVersion {
  index: number;
  version: ComparedVersion;
  /** Whether it is the side's own, rather than the game's original it leaves in place. */
  own: boolean;
}

/** Whether `version` belongs to side `side` of `duel`. */
function belongs(duel: ModDuel, side: 0 | 1, version: ComparedVersion): boolean {
  const owner = duel.sides[side];
  if (owner.mod != null) {
    return version.provider?.id === owner.mod.id;
  }
  // A side without a mod manager's mod is an archive, named by its file.
  return version.provider == null && version.archive?.toLowerCase() === owner.name.toLowerCase();
}

/**
 * The version each side brings: its own, the last one if it has several,
 * as the game would take it; or else the game's original, which is what
 * the side leaves in place.
 */
export function sideVersions(
  duel: ModDuel,
  comparison: FileComparison,
): [SideVersion | null, SideVersion | null] {
  const pick = (side: 0 | 1): SideVersion | null => {
    let found: SideVersion | null = null;
    comparison.versions.forEach((version, index) => {
      if (belongs(duel, side, version)) {
        found = { index, version, own: true };
      }
    });
    if (found !== null) {
      return found;
    }
    const original = comparison.versions.findIndex(
      (version) => version.provider == null && !belongs(duel, side === 0 ? 1 : 0, version),
    );
    const version = comparison.versions[original];
    return version === undefined ? null : { index: original, version, own: false };
  };
  return [pick(0), pick(1)];
}

/** The first version with the same bytes as the version at `index`. */
function group(comparison: FileComparison, index: number): number {
  return comparison.versions[index]?.sameAs ?? index;
}

/** Whether the two sides bring the same bytes; `null` where a side has none. */
export function sameOnBothSides(duel: ModDuel, comparison: FileComparison): boolean | null {
  const [a, b] = sideVersions(duel, comparison);
  if (a === null || b === null) {
    return null;
  }
  return group(comparison, a.index) === group(comparison, b.index);
}

/** A function's identity within a script. */
function functionKey(function_: ScriptFunction): string {
  return `${function_.state}\u0000${function_.name}\u0000${function_.kind}`;
}

/** How the functions of two versions of a script differ. */
export interface FunctionDifference {
  differ: ScriptFunction[];
  onlyA: ScriptFunction[];
  onlyB: ScriptFunction[];
  same: number;
}

export function compareFunctions(a: ScriptFunction[], b: ScriptFunction[]): FunctionDifference {
  const inB = new Map(b.map((function_) => [functionKey(function_), function_]));
  const inA = new Set(a.map(functionKey));
  const result: FunctionDifference = { differ: [], onlyA: [], onlyB: [], same: 0 };
  for (const function_ of a) {
    const other = inB.get(functionKey(function_));
    if (other === undefined) {
      result.onlyA.push(function_);
    } else if (other.fingerprint === function_.fingerprint) {
      result.same += 1;
    } else {
      result.differ.push(function_);
    }
  }
  result.onlyB = b.filter((function_) => !inA.has(functionKey(function_)));
  return result;
}

/** One row of two texts set side by side. */
export type DiffRow =
  | { kind: "same"; left: number; right: number; text: string }
  | { kind: "changed"; left: number | null; right: number | null; a: string; b: string }
  | { kind: "skipped"; count: number };

/** Above this many cells, the middle of two texts is not matched line by line. */
const MAX_CELLS = 4_000_000;

/** Unchanged lines kept around each change. */
const CONTEXT = 3;

/**
 * Two texts line by line, side by side: the lines both have, and where
 * they differ. Long runs of the same lines are folded.
 */
export function diffLines(a: string, b: string): DiffRow[] {
  const left = a.replace(/\r\n?/g, "\n").split("\n");
  const right = b.replace(/\r\n?/g, "\n").split("\n");
  let start = 0;
  while (start < left.length && start < right.length && left[start] === right[start]) {
    start += 1;
  }
  let endA = left.length;
  let endB = right.length;
  while (endA > start && endB > start && left[endA - 1] === right[endB - 1]) {
    endA -= 1;
    endB -= 1;
  }
  const middleA = left.slice(start, endA);
  const middleB = right.slice(start, endB);

  // Each line as one of: both, only left, only right.
  type Step =
    | { both: true; a: number; b: number }
    | { both: false; a: number | null; b: number | null };
  const steps: Step[] = [];
  for (let index = 0; index < start; index += 1) {
    steps.push({ both: true, a: index, b: index });
  }
  const n = middleA.length;
  const m = middleB.length;
  if (n * m <= MAX_CELLS && n > 0 && m > 0) {
    // The longest common run of lines, from the end, so that walking
    // forward reads it off.
    const width = m + 1;
    const table = new Uint32Array((n + 1) * width);
    for (let i = n - 1; i >= 0; i -= 1) {
      for (let j = m - 1; j >= 0; j -= 1) {
        table[i * width + j] =
          middleA[i] === middleB[j]
            ? (table[(i + 1) * width + j + 1] ?? 0) + 1
            : Math.max(table[(i + 1) * width + j] ?? 0, table[i * width + j + 1] ?? 0);
      }
    }
    let i = 0;
    let j = 0;
    while (i < n || j < m) {
      if (i < n && j < m && middleA[i] === middleB[j]) {
        steps.push({ both: true, a: start + i, b: start + j });
        i += 1;
        j += 1;
      } else if (
        j >= m ||
        (i < n && (table[(i + 1) * width + j] ?? 0) >= (table[i * width + j + 1] ?? 0))
      ) {
        steps.push({ both: false, a: start + i, b: null });
        i += 1;
      } else {
        steps.push({ both: false, a: null, b: start + j });
        j += 1;
      }
    }
  } else {
    for (let index = 0; index < Math.max(n, m); index += 1) {
      steps.push({
        both: false,
        a: index < n ? start + index : null,
        b: index < m ? start + index : null,
      });
    }
  }
  for (let offset = 0; offset < left.length - endA; offset += 1) {
    steps.push({ both: true, a: endA + offset, b: endB + offset });
  }

  // Lines only on one side next to each other become one changed row
  // each, pairing a removed line with an added one.
  const rows: DiffRow[] = [];
  let pendingA: number[] = [];
  let pendingB: number[] = [];
  const flush = () => {
    for (let index = 0; index < Math.max(pendingA.length, pendingB.length); index += 1) {
      const lineA = pendingA[index] ?? null;
      const lineB = pendingB[index] ?? null;
      rows.push({
        kind: "changed",
        left: lineA === null ? null : lineA + 1,
        right: lineB === null ? null : lineB + 1,
        a: lineA === null ? "" : (left[lineA] ?? ""),
        b: lineB === null ? "" : (right[lineB] ?? ""),
      });
    }
    pendingA = [];
    pendingB = [];
  };
  for (const step of steps) {
    if (step.both) {
      flush();
      rows.push({ kind: "same", left: step.a + 1, right: step.b + 1, text: left[step.a] ?? "" });
    } else {
      if (step.a !== null) {
        pendingA.push(step.a);
      }
      if (step.b !== null) {
        pendingB.push(step.b);
      }
    }
  }
  flush();
  return fold(rows);
}

/** Folds runs of unchanged rows away from any change. */
function fold(rows: DiffRow[]): DiffRow[] {
  const near = rows.map(() => false);
  rows.forEach((row, index) => {
    if (row.kind === "changed") {
      for (
        let at = Math.max(0, index - CONTEXT);
        at <= Math.min(rows.length - 1, index + CONTEXT);
        at += 1
      ) {
        near[at] = true;
      }
    }
  });
  const folded: DiffRow[] = [];
  let skipped = 0;
  rows.forEach((row, index) => {
    if (row.kind === "same" && !near[index]) {
      skipped += 1;
      return;
    }
    if (skipped > 0) {
      folded.push({ kind: "skipped", count: skipped });
      skipped = 0;
    }
    folded.push(row);
  });
  if (skipped > 0) {
    folded.push({ kind: "skipped", count: skipped });
  }
  return folded;
}

/** A size in bytes, as people read it. */
export function formatSize(bytes: number, lang: string): string {
  const format = (value: number, unit: string) =>
    `${value.toLocaleString(lang, { maximumFractionDigits: 1 })} ${unit}`;
  if (bytes < 1024) {
    return format(bytes, "B");
  }
  if (bytes < 1024 * 1024) {
    return format(bytes / 1024, "KB");
  }
  return format(bytes / (1024 * 1024), "MB");
}

/** A moment in seconds since 1970, as a date. */
export function formatDate(seconds: number, lang: string): string {
  return new Date(seconds * 1000).toLocaleDateString(lang, {
    year: "numeric",
    month: "short",
    day: "numeric",
  });
}
