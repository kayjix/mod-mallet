// The paths mode of the 3D views (ADR 0071): where characters walk around
// the point looked at, who made each piece of it, where the current choices
// leave paths over nothing, and where characters sit, lean, sleep or work.
// It draws through the views' layer of paths (`paths-layer.ts`), under the
// repair a spot shows (ADR 0058), and leaves the ways of ADR 0059 to
// `reach.tsx`.

import { type ReactNode, useEffect, useMemo, useState } from "react";
import { useSession } from "./Actions";
import type { ActionPlace, AreaObjects } from "./generated/area-objects";
import type { AreaPaths } from "./generated/area-paths";
import type { MapSpot, ObjectRef } from "./generated/rendered-report";
import { CELL } from "./map-places";
import { objectsOf, type PatchChoices, pairFor } from "./patch";
import type { MarkShape, PathGroup, PathMarks, PathPattern, PathsOverlay } from "./paths-layer";
import { pathsModeTexts } from "./paths-mode-texts";
import { repairKey } from "./repair-paths";
import "./paths-mode.css";

type Colour = [number, number, number, number];

/** One kind of piece of path: its colour and pattern, here and in the legend. */
interface Style {
  colour: Colour;
  pattern: PathPattern;
}

/**
 * The kinds of pieces of path, in colours of the Okabe–Ito palette that
 * stay apart for most eyes, and patterns that tell them apart without
 * colour.
 */
const PATH_STYLES = {
  game: { colour: [0.34, 0.71, 0.91, 0.3], pattern: "solid" },
  mod: { colour: [0.9, 0.62, 0.0, 0.55], pattern: "stripes" },
  repaired: { colour: [0.0, 0.62, 0.45, 0.65], pattern: "dots" },
  takenOut: { colour: [0.84, 0.37, 0.0, 0.85], pattern: "outline" },
  overNothing: { colour: [0.8, 0.47, 0.65, 0.85], pattern: "hatch" },
  highlight: { colour: [0.94, 0.89, 0.26, 0.8], pattern: "solid" },
} satisfies Record<string, Style>;

type StyleName = keyof typeof PATH_STYLES;

/** One kind of mark: its shape, colour and size. */
interface MarkStyle {
  shape: MarkShape;
  colour: Colour;
  size: number;
}

/** The marks, told apart by shape first and colour second. */
const MARK_STYLES = {
  door: { shape: "square", colour: [1, 1, 1, 0.95], size: 11 },
  link: { shape: "dot", colour: [0.75, 0.82, 0.9, 0.8], size: 7 },
  sit: { shape: "circle", colour: [0.94, 0.89, 0.26, 1], size: 12 },
  lean: { shape: "triangle", colour: [0.6, 0.8, 1.0, 1], size: 13 },
  sleep: { shape: "diamond", colour: [0.0, 0.45, 0.7, 1], size: 13 },
  work: { shape: "square", colour: [0.84, 0.37, 0.0, 1], size: 10 },
  other: { shape: "dot", colour: [0.8, 0.8, 0.8, 0.9], size: 9 },
  problem: { shape: "ring", colour: [0.8, 0.47, 0.65, 1], size: 34 },
  problemChosen: { shape: "ring", colour: [0.0, 0.62, 0.45, 1], size: 34 },
} satisfies Record<string, MarkStyle>;

type MarkName = keyof typeof MARK_STYLES;

/** The mark of each kind of action place (ADR 0069): sit, lean and sleep apart, all work alike. */
const MARK_OF_KIND: Record<ActionPlace["kind"], MarkName> = {
  sit: "sit",
  lean: "lean",
  sleep: "sleep",
  "chop-wood": "work",
  mine: "work",
  smith: "work",
  craft: "work",
  cook: "work",
  work: "work",
  pray: "other",
  stand: "other",
  other: "other",
};

/** The farthest around the point that is read, in game units, as the engine reads it. */
const MAX_RADIUS = 2.5 * CELL;

/** Reads a remembered switch; only a convenience. */
function rememberedSwitch(name: string): boolean {
  try {
    return localStorage.getItem(name) === "on";
  } catch {
    return false;
  }
}

/** A switch remembered in this browser; only a convenience. */
function useSwitch(name: string): [boolean, (on: boolean) => void] {
  const [on, setOn] = useState(() => rememberedSwitch(name));
  return [
    on,
    (next) => {
      setOn(next);
      try {
        localStorage.setItem(name, next ? "on" : "off");
      } catch {
        // Only a convenience.
      }
    },
  ];
}

/** An object by plugin and number, as the keys of the analyses spell it. */
function refKey(object: ObjectRef): string {
  return `${object.plugin.toLowerCase()}:${object.localId}`;
}

/** What is asked of the engine for the area shown, or why nothing is. */
type Asked = { key: string; paths: AreaPaths | null } | null;

/**
 * The paths of the area shown: `undefined` while reading, `null` if they
 * cannot be read, `"far"` where no area is shown.
 */
function useAreaPaths(
  on: boolean,
  area: AreaObjects | null,
  interior: boolean,
  hidden: ObjectRef[],
): AreaPaths | null | undefined | "far" {
  const { api } = useSession();
  const hiddenKey = useMemo(() => hidden.map(refKey).sort().join(";"), [hidden]);
  let key: string | null = null;
  if (on && area !== null) {
    if (interior) {
      key = `${area.world}|inside|${hiddenKey}`;
    } else {
      const radius = Math.min(area.radius, MAX_RADIUS);
      const [x, y] = area.centre;
      const cell = (value: number) => Math.floor(value / CELL);
      key = `${area.world}|${cell(x - radius)},${cell(x + radius)},${cell(y - radius)},${cell(
        y + radius,
      )}|${hiddenKey}`;
    }
  }
  const [asked, setAsked] = useState<Asked>(null);
  // biome-ignore lint/correctness/useExhaustiveDependencies: the area and the objects are compared by `key`.
  useEffect(() => {
    if (key === null || area === null) {
      return;
    }
    let outdated = false;
    const centre = interior ? null : area.centre;
    api
      .areaPaths(
        area.world,
        centre?.[0] ?? null,
        centre?.[1] ?? null,
        Math.min(area.radius, MAX_RADIUS),
        hidden,
      )
      .then((paths) => {
        if (!outdated) {
          setAsked({ key, paths });
        }
      })
      .catch(() => {
        if (!outdated) {
          setAsked({ key, paths: null });
        }
      });
    return () => {
      outdated = true;
    };
  }, [api, key]);
  if (!on) {
    return undefined;
  }
  if (area === null) {
    return "far";
  }
  return asked !== null && asked.key === key ? asked.paths : undefined;
}

/** A spot where the current choices leave paths over nothing. */
interface PathsProblem {
  key: string;
  /** The collision it belongs to, if one is listed. */
  spot: MapSpot | null;
  /** The names of the objects that carry the paths. */
  names: string;
  /** How many pieces of path lie over them. */
  pieces: number;
  /** Whether the repair of every mesh concerned is chosen (ADR 0058). */
  repairChosen: boolean;
  /** Where to mark it: the middle of the pieces, in game units. */
  at: [number, number, number];
}

/** The middle of triangle `triangle` of a mesh, in game units. */
function middleOf(
  vertices: number[],
  triangles: number[],
  triangle: number,
): [number, number, number] {
  const at: [number, number, number] = [0, 0, 0];
  for (let corner = 0; corner < 3; corner++) {
    const vertex = triangles[triangle * 3 + corner] ?? 0;
    at[0] += (vertices[vertex * 3] ?? 0) / 3;
    at[1] += (vertices[vertex * 3 + 1] ?? 0) / 3;
    at[2] += (vertices[vertex * 3 + 2] ?? 0) / 3;
  }
  return at;
}

/**
 * The spots of `spots` whose parts carry paths over nothing in `paths`,
 * and how many other objects asked about do.
 */
function pathsProblems(
  paths: AreaPaths,
  spots: MapSpot[],
  choices: PatchChoices,
): { problems: PathsProblem[]; others: number } {
  const spotOf = new Map<string, MapSpot>();
  for (const spot of spots) {
    for (const part of spot.parts ?? []) {
      spotOf.set(refKey(part.object), spot);
    }
  }
  const meshes = new Map(paths.navmeshes.map((mesh) => [mesh.navmesh, mesh]));
  const found = new Map<
    string,
    {
      spot: MapSpot | null;
      names: Set<string>;
      pieces: number;
      navmeshes: Set<string>;
      sum: [number, number, number];
    }
  >();
  const others = new Set<string>();
  for (const over of paths.overHidden) {
    const mesh = meshes.get(over.navmesh);
    if (mesh === undefined) {
      continue;
    }
    const spot = spotOf.get(refKey(over.object)) ?? null;
    if (spot === null) {
      others.add(refKey(over.object));
      continue;
    }
    const key = `${spot.finding}|${spot.position?.join(",") ?? ""}`;
    const entry = found.get(key) ?? {
      spot,
      names: new Set<string>(),
      pieces: 0,
      navmeshes: new Set<string>(),
      sum: [0, 0, 0] as [number, number, number],
    };
    const part = (spot.parts ?? []).find(
      (candidate) => refKey(candidate.object) === refKey(over.object),
    );
    entry.names.add(part?.name ?? `${over.object.plugin} ${over.object.localId.toString(16)}`);
    entry.navmeshes.add(over.navmesh);
    for (const triangle of over.triangles) {
      const middle = middleOf(mesh.vertices, mesh.triangles, triangle);
      entry.sum[0] += middle[0];
      entry.sum[1] += middle[1];
      entry.sum[2] += middle[2];
    }
    entry.pieces += over.triangles.length;
    found.set(key, entry);
  }
  const problems = [...found.entries()].map(([key, entry]) => ({
    key,
    spot: entry.spot,
    names: [...entry.names].join(", "),
    pieces: entry.pieces,
    repairChosen: [...entry.navmeshes].every(
      (navmesh) => (choices[repairKey(navmesh)] ?? null) !== null,
    ),
    at: entry.sum.map((value) => value / Math.max(1, entry.pieces)) as [number, number, number],
  }));
  problems.sort((a, b) => b.pieces - a.pieces);
  return { problems, others: others.size };
}

/** How many pieces of each kind are in view, and of each mod. */
interface Counts {
  styles: Record<StyleName, number>;
  /** By the maker's place in {@link AreaPaths.makers}. */
  makers: Map<number, number>;
}

/** What the paths mode draws: groups of triangles and marks. */
function pathsOverlay(
  paths: AreaPaths,
  options: {
    highlight: number | null;
    actions: ActionPlace[] | null;
    problems: PathsProblem[];
  },
): { groups: PathGroup[]; marks: PathMarks[]; counts: Counts } {
  const corners: Record<StyleName, number[]> = {
    game: [],
    mod: [],
    repaired: [],
    takenOut: [],
    overNothing: [],
    highlight: [],
  };
  const counts: Counts = {
    styles: { game: 0, mod: 0, repaired: 0, takenOut: 0, overNothing: 0, highlight: 0 },
    makers: new Map(),
  };
  const push = (into: number[], vertices: number[], triangles: number[], triangle: number) => {
    for (let corner = 0; corner < 3; corner++) {
      const vertex = triangles[triangle * 3 + corner] ?? 0;
      into.push(
        vertices[vertex * 3] ?? 0,
        vertices[vertex * 3 + 1] ?? 0,
        vertices[vertex * 3 + 2] ?? 0,
      );
    }
  };
  const marks: Record<MarkName, number[]> = {
    door: [],
    link: [],
    sit: [],
    lean: [],
    sleep: [],
    work: [],
    other: [],
    problem: [],
    problemChosen: [],
  };
  for (const mesh of paths.navmeshes) {
    const out = new Set(mesh.takenOut);
    mesh.madeBy.forEach((maker, triangle) => {
      let style: StyleName;
      if (out.has(triangle)) {
        style = "takenOut";
      } else {
        const kind = paths.makers[maker]?.kind ?? "mod";
        style = kind === "game" ? "game" : kind === "repair" ? "repaired" : "mod";
        counts.makers.set(maker, (counts.makers.get(maker) ?? 0) + 1);
      }
      counts.styles[style] += 1;
      push(
        corners[options.highlight === maker ? "highlight" : style],
        mesh.vertices,
        mesh.triangles,
        triangle,
      );
    });
    for (const door of mesh.doors) {
      marks.door.push(...door);
    }
    for (const link of mesh.links) {
      marks.link.push(...link);
    }
  }
  const meshes = new Map(paths.navmeshes.map((mesh) => [mesh.navmesh, mesh]));
  for (const over of paths.overHidden) {
    const mesh = meshes.get(over.navmesh);
    if (mesh === undefined) {
      continue;
    }
    for (const triangle of over.triangles) {
      push(corners.overNothing, mesh.vertices, mesh.triangles, triangle);
      counts.styles.overNothing += 1;
    }
  }
  for (const place of options.actions ?? []) {
    marks[MARK_OF_KIND[place.kind]].push(
      place.position[0] ?? 0,
      place.position[1] ?? 0,
      place.position[2] ?? 0,
    );
  }
  for (const problem of options.problems) {
    marks[problem.repairChosen ? "problemChosen" : "problem"].push(...problem.at);
  }
  // Lit up, one mod's paths stand out: the others step back.
  const dim = options.highlight !== null;
  const groups: PathGroup[] = (
    ["game", "mod", "repaired", "takenOut", "highlight", "overNothing"] as StyleName[]
  )
    .filter((name) => corners[name].length > 0)
    .map((name) => {
      const { colour, pattern } = PATH_STYLES[name];
      const [r, g, b, a] = colour;
      const faded = dim && name !== "highlight" && name !== "overNothing";
      return {
        corners: new Float32Array(corners[name]),
        colour: [r, g, b, faded ? a * 0.35 : a] as Colour,
        pattern,
        edges: !faded,
      };
    });
  const drawn: PathMarks[] = (Object.keys(marks) as MarkName[])
    .filter((name) => marks[name].length > 0)
    .map((name) => ({ ...MARK_STYLES[name], points: new Float32Array(marks[name]) }));
  return { groups, marks: drawn, counts };
}

/** The repair a spot shows (ADR 0058) on top of the paths mode, or either alone. */
export function withPaths(
  mode: Pick<PathsOverlay, "groups" | "marks"> | null,
  repair: PathsOverlay | null,
): PathsOverlay | null {
  if (mode === null) {
    return repair;
  }
  return { removed: repair?.removed ?? [], added: repair?.added ?? [], ...mode };
}

/** What a view needs from the paths mode. */
export interface PathsMode {
  /** Whether the mode is on. */
  on: boolean;
  /** The switches for the bar over the view. */
  tool: ReactNode;
  /** The legend and the problems in view, to show over the view. */
  legend: ReactNode;
  /** What to draw, without a repair; null while off. */
  overlay: Pick<PathsOverlay, "groups" | "marks"> | null;
}

/**
 * The paths mode for a view: `area` as the view tells it (`onArea`),
 * `interior` whether it is an interior cell, `hidden` the objects the
 * choices hide, `spots` the collisions listed, and `onOpen` opens one.
 */
export function usePathsMode(input: {
  area: AreaObjects | null;
  interior: boolean;
  hidden: ObjectRef[];
  spots: MapSpot[];
  onOpen?: (spot: MapSpot) => void;
}): PathsMode {
  const { lang, patchChoices } = useSession();
  const p = pathsModeTexts[lang];
  const [on, setOn] = useSwitch("mod-mallet.paths-mode");
  const [actions, setActions] = useSwitch("mod-mallet.paths-mode.actions");
  const [pointed, setPointed] = useState<number | null>(null);
  const [pinned, setPinned] = useState<number | null>(null);
  const highlight = pointed ?? pinned;
  const paths = useAreaPaths(on, input.area, input.interior, input.hidden);
  const shown = paths !== undefined && paths !== null && paths !== "far" ? paths : null;
  const found = useMemo(
    () =>
      shown === null
        ? { problems: [], others: 0 }
        : pathsProblems(shown, input.spots, patchChoices),
    [shown, input.spots, patchChoices],
  );
  const drawn = useMemo(
    () =>
      shown === null
        ? null
        : pathsOverlay(shown, {
            highlight,
            actions: actions ? (input.area?.actions ?? []) : null,
            problems: found.problems,
          }),
    [shown, highlight, actions, found, input.area],
  );
  const overlay = useMemo(
    () => (drawn === null ? null : { groups: drawn.groups, marks: drawn.marks }),
    [drawn],
  );

  const tool = (
    <span className="view-tool" title={p.toggleHint}>
      <button
        type="button"
        className="small paths-mode-toggle"
        aria-pressed={on}
        onClick={() => setOn(!on)}
      >
        <PathsIcon />
        {p.toggle}
      </button>
      {on && (
        <label className="paths-mode-actions">
          <input
            type="checkbox"
            checked={actions}
            onChange={(event) => setActions(event.target.checked)}
          />
          {p.actions}
        </label>
      )}
    </span>
  );

  let legend: ReactNode = null;
  if (on) {
    let body: ReactNode;
    if (paths === "far") {
      body = <p className="aside">{p.tooFar}</p>;
    } else if (paths === undefined) {
      body = <p className="aside">{p.reading}</p>;
    } else if (paths === null) {
      body = <p className="aside">{p.failed}</p>;
    } else if (drawn === null || paths.navmeshes.length === 0) {
      body = <p className="aside">{p.nothing}</p>;
    } else {
      const { counts } = drawn;
      const mods = paths.makers
        .map((maker, index) => ({ maker, index, count: counts.makers.get(index) ?? 0 }))
        .filter(({ maker }) => maker.kind === "mod")
        .sort((a, b) => b.count - a.count);
      body = (
        <>
          <ul className="paths-legend-list">
            <Row style="game" label={p.game} count={counts.styles.game} p={p} />
            <Row style="mod" label={p.mods} count={counts.styles.mod} p={p} />
            {mods.length > 0 && (
              <li className="paths-legend-mods">
                <ul onMouseLeave={() => setPointed(null)}>
                  {mods.map(({ maker, index, count }) => (
                    <li key={maker.plugin}>
                      <button
                        type="button"
                        className="paths-legend-mod"
                        aria-pressed={pinned === index}
                        disabled={count === 0}
                        onMouseEnter={() => setPointed(count === 0 ? null : index)}
                        onFocus={() => setPointed(count === 0 ? null : index)}
                        onBlur={() => setPointed(null)}
                        onClick={() => setPinned(pinned === index ? null : index)}
                      >
                        <span className="paths-legend-mod-name">{maker.plugin}</span>
                        <span className="paths-legend-count">
                          {count === 0 ? p.replaced : p.pieces(count)}
                        </span>
                      </button>
                    </li>
                  ))}
                </ul>
                <p className="aside">{p.modsHint}</p>
              </li>
            )}
            <Row style="repaired" label={p.repaired} count={counts.styles.repaired} p={p} />
            <Row style="takenOut" label={p.takenOut} count={counts.styles.takenOut} p={p} />
            <Row
              style="overNothing"
              label={p.overNothing}
              count={counts.styles.overNothing}
              p={p}
            />
          </ul>
          <ul className="paths-legend-marks">
            <MarkRow mark="door" label={p.doors} />
            <MarkRow mark="link" label={p.links} />
            {actions &&
              (["sit", "lean", "sleep", "work", "other"] as const).map((kind) => (
                <MarkRow key={kind} mark={kind} label={p.kinds[kind]} />
              ))}
          </ul>
          {found.problems.length > 0 && (
            <div className="paths-problems">
              <strong>{p.problemsTitle(found.problems.length)}</strong>
              <ul>
                {found.problems.map((problem) => (
                  <li key={problem.key}>
                    <span
                      className="paths-problem-mark"
                      data-chosen={problem.repairChosen || undefined}
                    />
                    <span>
                      {p.problem(problem.names, problem.pieces)}
                      {problem.repairChosen && (
                        <span className="paths-problem-chosen"> · {p.problemRepairChosen}</span>
                      )}
                    </span>
                    {problem.spot !== null && input.onOpen !== undefined && (
                      <button
                        type="button"
                        className="small"
                        onClick={() => problem.spot !== null && input.onOpen?.(problem.spot)}
                      >
                        {p.open}
                      </button>
                    )}
                  </li>
                ))}
              </ul>
            </div>
          )}
          {found.others > 0 && <p className="aside">{p.otherObjects(found.others)}</p>}
        </>
      );
    }
    legend = (
      <section className="paths-legend" aria-label={p.title}>
        <strong className="paths-legend-title">{p.title}</strong>
        {body}
      </section>
    );
  }
  return { on, tool, legend, overlay };
}

/** One kind of piece of path in the legend. */
function Row(props: {
  style: StyleName;
  label: string;
  count: number;
  p: (typeof pathsModeTexts)["en"];
}) {
  if (props.count === 0 && props.style !== "game") {
    return null;
  }
  return (
    <li>
      <span className="paths-swatch" data-style={props.style} aria-hidden="true" />
      <span>{props.label}</span>
      <span className="paths-legend-count">{props.p.pieces(props.count)}</span>
    </li>
  );
}

/** One kind of mark in the legend. */
function MarkRow(props: { mark: MarkName; label: string }) {
  return (
    <li>
      <span className="paths-mark" data-mark={props.mark} aria-hidden="true" />
      <span>{props.label}</span>
    </li>
  );
}

/** A footpath: two rows of steps. */
function PathsIcon() {
  return (
    <svg className="paths-mode-icon" viewBox="0 0 24 24" aria-hidden="true">
      <path d="M7 21l2-6M11 11l2-6M13 21l2-6M17 11l2-6" />
    </svg>
  );
}

/**
 * The objects the current choices hide among the collisions `spots`, for
 * a page that has the spots but not the pairs, such as the world map.
 */
export function useHiddenBySpots(spots: MapSpot[]): ObjectRef[] {
  const { patchChoices, ignored } = useSession();
  return useMemo(() => {
    const byFinding = new Map<string, MapSpot[]>();
    for (const spot of spots) {
      if (!ignored.has(spot.finding)) {
        byFinding.set(spot.finding, [...(byFinding.get(spot.finding) ?? []), spot]);
      }
    }
    const pairs = [...byFinding.entries()].flatMap(([id, found]) => pairFor(id, "", found) ?? []);
    return objectsOf(pairs, patchChoices);
  }, [spots, ignored, patchChoices]);
}
