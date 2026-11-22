// The collisions as a worklist (ADR 0040): every spot where objects of two
// mods collide, grouped by the place it is near or by the pair of mods,
// worst first. A spot picked in the list is flown to in 3D, with its two
// objects lit up; the user looks at it with one side or the other left
// out, decides which of them the patch hides (spot by spot, for a passage
// of spots close together, or for a pair of mods at once) and goes on.

import {
  type ComponentProps,
  type ReactNode,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import "./work-panel.css";
import { CopyButton, useSession } from "./Actions";
import { ActionPlaceInfo } from "./action-places";
import { objectKey, type PartTools } from "./area-layer";
import { isCollision } from "./arrange";
import { collisionsTexts } from "./collisions-texts";
import { DoorPanel, doorHidden, mergeChoices } from "./doors";
import type { AreaObject, AreaObjects } from "./generated/area-objects";
import type {
  DoorGroup,
  DoorMerge,
  MapSpot,
  ObjectRef,
  RenderedDuel,
  RenderedReport,
  SpotPart,
} from "./generated/rendered-report";
import { InteriorView } from "./InteriorView";
import { LocateButton } from "./locate";
import { PlaceIcon, useWorldPicture } from "./MapView";
import { MapView3D } from "./MapView3D";
import { kindOf, nearest, type Picked, spotPosition, type WorkKind } from "./map-places";
import { MoveControls, SpotMoves } from "./move-controls";
import { FreeObjectActions, NeededPart, PartsTogether, spotOf } from "./object-actions";
import {
  chooseFinding,
  chooseObject,
  chooseSpot,
  decisionAt,
  matchKey,
  objectsOf,
  type Pair,
  type PatchChoices,
  pairsOf,
  pinsOf,
  spotKey,
  undecideSpot,
} from "./patch";
import type { PathsOverlay } from "./paths-layer";
import { PinButton, PinIcon, usePinTexts } from "./pins";
import { ReachNote, usePaintReach, useReach } from "./reach";
import { RepairPaths } from "./repair-paths";
import { DisablePluginsButton, ShowFilesButton, STILL_LOADED, stillFiles } from "./still-loaded";
import { switchedOffTexts } from "./switched-off-texts";
import { TaskList } from "./Tasks";
import { PathIcon, useWalkedOn, useWalkTexts, type WalkQuery } from "./walkways";
import { WideAdvice, wideFindings } from "./wide-advice";

/** One plugin's objects at a spot. */
interface Side {
  plugin: string;
  parts: SpotPart[];
  hideable: boolean;
  suggested: boolean;
}

/** One spot of the worklist. */
interface Item {
  /** Its identity among the patch's choices (see {@link spotKey}). */
  key: string;
  spot: MapSpot;
  kind: WorkKind;
  /** The collision findings of its two mods: one per kind of collision. */
  pairs: Pair[];
  sides: Side[];
  /** The group it is in, by region and by pair of mods. */
  region: Group;
  mods: Group;
  /** The passage it belongs to: its spots of the same two mods close by. */
  passage: string;
  /** What a search finds it by, in lower case. */
  text: string;
}

interface Group {
  key: string;
  label: string;
  aside: string | null;
}

type Grouping = "region" | "mods";

/** How the groups are ordered: by their spots, or by those still open. */
type Sorting = "open" | "spots";

/** The two mods of some findings, with what each could hide in all of them together. */
function sidesOfMods(pairs: Pair[]): { plugin: string; objects: number }[] {
  const sides = new Map<string, { plugin: string; objects: number }>();
  for (const side of pairs.flatMap((pair) => pair.sides)) {
    const key = side.plugin.toLowerCase();
    const entry = sides.get(key) ?? { plugin: side.plugin, objects: 0 };
    entry.objects += side.objects.length;
    sides.set(key, entry);
  }
  return [...sides.values()];
}

/** How many spots some findings have together. */
const spotsOf = (pairs: Pair[]) => pairs.reduce((sum, pair) => sum + pair.spots.length, 0);

/** The kinds of the worklist, the worst first. */
const WORK_KINDS: WorkKind[] = ["duplicate", "door", "overlap", "deep", "touch"];

const RANK: Record<WorkKind, number> = { duplicate: 0, door: 1, overlap: 2, deep: 3, touch: 4 };

/** Game units in a metre, for the depth of a collision. */
const UNITS_PER_METRE = 70;

/** From this crossing on, in game units (2 m), a touch counts as deep. */
const DEEP = 140;

/** Spots of the same two mods this close (14 m) belong to one passage. */
const PASSAGE = 600;

/** The radius around a spot to hide one mod's objects in, at first, in metres. */
const AROUND = 10;

/** How far from a spot the camera comes, in cells: the two objects fill the view. */
const CLOSE = 0.9;

/** From this many spots on, a pair of mods is better decided as a whole. */
const MANY = 20;

/** Reads a remembered setting; only a convenience. */
function remembered<T>(name: string, fallback: T, read: (value: string) => T | null): T {
  try {
    const value = localStorage.getItem(name);
    return value === null ? fallback : (read(value) ?? fallback);
  } catch {
    return fallback;
  }
}

function remember(name: string, value: string) {
  try {
    localStorage.setItem(name, value);
  } catch {
    // Only a convenience.
  }
}

/** A spot's kind in the worklist: a touch that crosses far counts as deep. */
function workKind(spot: MapSpot): WorkKind {
  const kind = kindOf(spot);
  return kind === "touch" && (spot.depth ?? 0) >= DEEP ? "deep" : kind;
}

/** The plugins of a spot with their objects there, in the order the spot names them. */
function sidesOf(parts: SpotPart[]): Side[] {
  const sides = new Map<string, Side>();
  for (const part of parts) {
    const { plugin } = part.object;
    const side = sides.get(plugin) ?? { plugin, parts: [], hideable: false, suggested: false };
    side.parts.push(part);
    side.hideable ||= part.hideable;
    side.suggested ||= part.suggested;
    sides.set(plugin, side);
  }
  return [...sides.values()];
}

/** Where a spot is, to join it with its neighbours; interiors by their cell. */
function placeOf(spot: MapSpot): string {
  return spot.location.type === "interior"
    ? `interior:${spot.location.cell}`
    : `world:${spot.location.world}`;
}

/**
 * The passages of `spots`: spots of the same finding (two mods) in the same
 * place, each within {@link PASSAGE} of another, joined; the key of each
 * spot's passage.
 */
function passagesOf(spots: { key: string; spot: MapSpot }[]): Map<string, string> {
  const parent = new Map<string, string>();
  const root = (key: string): string => {
    let at = key;
    for (let next = parent.get(at); next !== undefined && next !== at; next = parent.get(at)) {
      at = next;
    }
    return at;
  };
  const byPlace = new Map<string, { key: string; at: [number, number, number] }[]>();
  for (const { key, spot } of spots) {
    parent.set(key, key);
    const at = spotPosition(spot) ?? [0, 0, 0];
    const place = `${spot.finding}|${placeOf(spot)}`;
    byPlace.set(place, [...(byPlace.get(place) ?? []), { key, at }]);
  }
  for (const members of byPlace.values()) {
    for (let i = 0; i < members.length; i += 1) {
      for (let j = i + 1; j < members.length; j += 1) {
        const a = members[i];
        const b = members[j];
        if (a === undefined || b === undefined) {
          continue;
        }
        if (Math.hypot(a.at[0] - b.at[0], a.at[1] - b.at[1], a.at[2] - b.at[2]) <= PASSAGE) {
          parent.set(root(a.key), root(b.key));
        }
      }
    }
  }
  return new Map(spots.map(({ key }) => [key, root(key)]));
}

/** The spots of all collisions not set aside that name their objects. */
function useItems(report: RenderedReport): Item[] {
  const { t, ignored } = useSession();
  return useMemo(() => {
    const pairs = new Map(pairsOf(report, ignored).map((pair) => [pair.id, pair]));
    // The findings of each pair of mods, whatever kind of collision.
    const modsKey = (pair: Pair) =>
      pair.sides
        .map((side) => side.plugin.toLowerCase())
        .sort()
        .join("|");
    const byMods = new Map<string, Pair[]>();
    for (const pair of pairs.values()) {
      byMods.set(modsKey(pair), [...(byMods.get(modsKey(pair)) ?? []), pair]);
    }
    const collisions = new Set(
      report.findings
        .filter((rendered) => isCollision(rendered) && !ignored.has(rendered.finding.id))
        .map((rendered) => rendered.finding.id),
    );
    const headlines = new Map(
      report.findings.map((rendered) => [rendered.finding.id, rendered.headline]),
    );
    const worlds = new Map(report.map.worlds.map((world) => [world.name, world]));
    // A spot is named by its objects; the same objects twice are one spot.
    const seen = new Set<string>();
    const keyed = report.map.spots.flatMap((spot) => {
      const key = spotKey(spot);
      if (key === null || seen.has(key) || !collisions.has(spot.finding)) {
        return [];
      }
      seen.add(key);
      return [{ key, spot }];
    });
    const passages = passagesOf(keyed);
    return keyed.map(({ key, spot }) => {
      const pair = pairs.get(spot.finding) ?? null;
      let region: Group;
      if (spot.location.type === "interior") {
        region = {
          key: `interior:${spot.location.cell}`,
          label: t.workInterior(spot.location.cell),
          aside: null,
        };
      } else {
        const world = worlds.get(spot.location.world);
        const position = spotPosition(spot);
        const near =
          position === null ? undefined : nearest(world?.markers ?? [], position, 1)[0]?.marker;
        region =
          near === undefined
            ? { key: `world:${spot.location.world}`, label: spot.location.world, aside: null }
            : {
                key: `world:${spot.location.world}|${near.name}`,
                label: near.name || t.placesKind[near.kind],
                aside: spot.location.world,
              };
      }
      const plugins = pair?.sides.map((side) => side.plugin) ?? [];
      const mods = pair === null ? spot.finding : modsKey(pair);
      return {
        key,
        spot,
        kind: workKind(spot),
        pairs: pair === null ? [] : (byMods.get(mods) ?? [pair]),
        sides: sidesOf(spot.parts ?? []),
        region,
        mods: {
          key: mods,
          label:
            plugins.length === 2
              ? `${plugins[0]} · ${plugins[1]}`
              : (headlines.get(spot.finding) ?? spot.finding),
          aside: null,
        },
        passage: passages.get(key) ?? key,
        text: [
          region.label,
          region.aside ?? "",
          spot.location.type === "interior" ? spot.location.cell : spot.location.world,
          ...(spot.parts ?? []).flatMap((part) => [part.name, part.object.plugin]),
        ]
          .join(" ")
          .toLowerCase(),
      };
    });
  }, [report, ignored, t]);
}

/** Groups of items, the groups with the most serious spots first, each worst first. */
function grouped(
  items: Item[],
  by: Grouping,
  sort: Sorting,
  open: (item: Item) => boolean,
): { group: Group; items: Item[] }[] {
  const groups = new Map<string, { group: Group; items: Item[] }>();
  for (const item of items) {
    const group = item[by === "region" ? "region" : "mods"];
    const entry = groups.get(group.key) ?? { group, items: [] };
    entry.items.push(item);
    groups.set(group.key, entry);
  }
  const serious = (entry: { items: Item[] }) =>
    entry.items.filter((item) => item.kind !== "touch").length;
  const worst = (entry: { items: Item[] }) =>
    Math.min(...entry.items.map((item) => RANK[item.kind]));
  const left = (entry: { items: Item[] }) => entry.items.filter(open).length;
  // Groups with nothing left to decide go last, either way.
  const done = (entry: { items: Item[] }) => (left(entry) === 0 ? 1 : 0);
  return [...groups.values()]
    .map((entry) => ({
      ...entry,
      // Worst first; the spots of one passage stay together.
      items: [...entry.items].sort(
        (a, b) =>
          RANK[a.kind] - RANK[b.kind] ||
          a.passage.localeCompare(b.passage) ||
          (b.spot.depth ?? 0) - (a.spot.depth ?? 0),
      ),
    }))
    .sort(
      (a, b) =>
        done(a) - done(b) ||
        (sort === "open" ? left(b) - left(a) : 0) ||
        worst(a) - worst(b) ||
        serious(b) - serious(a) ||
        b.items.length - a.items.length ||
        a.group.label.localeCompare(b.group.label),
    );
}

/**
 * The collisions: a list of spots to decide one by one beside the spot
 * looked at in 3D, and the pairs of mods to decide as a whole.
 */
export function Collisions({
  report,
  start,
}: {
  report: RenderedReport;
  /** The spot to open with, if any. */
  start: MapSpot | null;
}) {
  const {
    t,
    lang,
    ignored,
    patchChoices: choices,
    setPatchChoices,
    showPatch,
    pairAt,
    pinnedAsked,
    decide: decideDuel,
  } = useSession();
  const own = collisionsTexts[lang];
  const pinText = usePinTexts();
  // Spots pinned to solve later by hand (ADR 0050): a filter of their own.
  const pins = useMemo(() => pinsOf(choices), [choices]);
  const [pinnedOnly, setPinnedOnly] = useState(
    () => pinnedAsked || remembered("mallet.workPinned", false, (value) => value === "on"),
  );
  useEffect(() => {
    if (pinnedAsked) {
      setPinnedOnly(true);
    }
  }, [pinnedAsked]);
  const items = useItems(report);
  // The pair to open with, from the names of its two mods (ADR 0048).
  const focusPair = useMemo(() => {
    if (pairAt === null) {
      return null;
    }
    const wanted = pairAt
      .map((name) => name.toLowerCase())
      .sort()
      .join("|");
    const named = new Map(
      report.findings.map((rendered) => {
        const args = rendered.finding.args as Record<string, unknown>;
        const names = [args.first, args.second].map((name) => String(name ?? "").toLowerCase());
        return [rendered.finding.id, names.sort().join("|")];
      }),
    );
    return items.find((item) => named.get(item.spot.finding) === wanted)?.mods.key ?? null;
  }, [pairAt, report, items]);
  const [tab, setTab] = useState<"spots" | "mods">(() =>
    start !== null
      ? "spots"
      : pairAt !== null
        ? "mods"
        : remembered<"spots" | "mods">("mallet.workTab", "spots", (value) =>
            value === "mods" || value === "spots" ? value : null,
          ),
  );
  const changeTab = (next: "spots" | "mods") => {
    setTab(next);
    remember("mallet.workTab", next);
  };
  const [kinds, setKinds] = useState<Set<WorkKind>>(() =>
    remembered(
      "mallet.workKinds3",
      new Set<WorkKind>(["duplicate", "door", "overlap", "deep"]),
      (value) =>
        new Set(
          value
            .split(",")
            .filter((kind): kind is WorkKind => WORK_KINDS.includes(kind as WorkKind)),
        ),
    ),
  );
  const [by, setBy] = useState<Grouping>(() =>
    remembered<Grouping>("mallet.workGroup", "region", (value) =>
      value === "mods" || value === "region" ? value : null,
    ),
  );
  const [sort, setSort] = useState<Sorting>(() =>
    remembered<Sorting>("mallet.workSort", "open", (value) =>
      value === "open" || value === "spots" ? value : null,
    ),
  );
  const changeSort = (next: Sorting) => {
    setSort(next);
    remember("mallet.workSort", next);
  };
  const changeBy = (next: Grouping) => {
    setBy(next);
    remember("mallet.workGroup", next);
  };
  const [hideDecided, setHideDecided] = useState(() =>
    remembered("mallet.workHideDecided", false, (value) => value === "on"),
  );
  const [advance, setAdvance] = useState(() =>
    remembered("mallet.workAdvance2", false, (value) => value === "on"),
  );
  // Tinted parts and ghosts, or the place as the game would show it.
  const [marks, setMarks] = useState(true);

  const decision = useCallback(
    (item: Item, from: PatchChoices = choices) =>
      decisionAt(from, item.spot.finding, item.key, item.spot.parts ?? []),
    [choices],
  );
  const counts = useMemo(() => {
    const found: Record<WorkKind, number> = {
      duplicate: 0,
      door: 0,
      overlap: 0,
      deep: 0,
      touch: 0,
    };
    for (const item of items) {
      found[item.kind] += 1;
    }
    return found;
  }, [items]);
  const groups = useMemo(
    () =>
      grouped(
        items.filter((item) => kinds.has(item.kind)),
        by,
        sort,
        (item) => decision(item) === undefined,
      ),
    [items, kinds, by, sort, decision],
  );
  /** Every spot of the chosen kinds, in the order of the list. */
  const ordered = useMemo(() => groups.flatMap((entry) => entry.items), [groups]);
  const decidedCount = ordered.filter((item) => decision(item) !== undefined).length;
  // What the search looks for: places, mods, objects.
  const [query, setQuery] = useState("");
  const needle = query.trim().toLowerCase();
  const shown = (item: Item) =>
    (!hideDecided || decision(item) === undefined) &&
    (!pinnedOnly || pins.has(item.key)) &&
    (needle === "" || item.text.includes(needle));

  const byKey = useMemo(() => new Map(items.map((item) => [item.key, item])), [items]);
  const passages = useMemo(() => {
    const found = new Map<string, Item[]>();
    for (const item of items) {
      found.set(item.passage, [...(found.get(item.passage) ?? []), item]);
    }
    return found;
  }, [items]);
  const startKey = start === null ? null : spotKey(start);
  const [currentKey, setCurrentKey] = useState<string | null>(startKey);
  // A spot handed over later (from the map or a comparison) is opened too.
  useEffect(() => {
    if (startKey !== null) {
      setCurrentKey(startKey);
      setTab("spots");
    }
  }, [startKey]);
  // So is a pair handed over later.
  useEffect(() => {
    if (focusPair !== null) {
      setTab("mods");
    }
  }, [focusPair]);
  // The pair opened from a spot, on the tab of the pairs.
  const [openedPair, setOpenedPair] = useState<string | null>(null);
  const current =
    (currentKey === null ? undefined : byKey.get(currentKey)) ??
    ordered.find((item) => decision(item) === undefined) ??
    ordered[0] ??
    null;

  // What is left out to look at, without deciding: plugins at the current spot.
  const [preview, setPreview] = useState<Set<string>>(new Set());
  const currentSpot = current?.key;
  // biome-ignore lint/correctness/useExhaustiveDependencies: a new spot starts without a preview.
  useEffect(() => {
    setPreview(new Set());
  }, [currentSpot]);

  /** The spot after (or before) the current one that `wanted` accepts, round the end. */
  const step = (direction: 1 | -1, wanted: (item: Item) => boolean): Item | null => {
    if (ordered.length === 0) {
      return null;
    }
    const at = current === null ? -1 : ordered.indexOf(current);
    for (let move = 1; move <= ordered.length; move += 1) {
      const index = (at + direction * move + ordered.length * 2) % ordered.length;
      const candidate = ordered[index];
      if (candidate !== undefined && candidate !== current && wanted(candidate)) {
        return candidate;
      }
    }
    return null;
  };
  const go = (item: Item | null) => {
    if (item !== null) {
      setCurrentKey(item.key);
    }
  };
  const goOn = (next: PatchChoices) => {
    if (advance) {
      go(step(1, (candidate) => decision(candidate, next) === undefined));
    }
  };

  const decide = (item: Item, plugin: string | null) => {
    const next = chooseSpot(choices, item.key, plugin);
    setPatchChoices(next);
    setPreview(new Set());
    if (item === current) {
      goOn(next);
    }
  };
  const decidePassage = (passage: Item[], plugin: string, extra: ObjectRef[]) => {
    let next = choices;
    for (const item of passage) {
      next = chooseSpot(next, item.key, plugin);
    }
    for (const object of extra) {
      next = chooseObject(next, object, true);
    }
    setPatchChoices(next);
    setPreview(new Set());
    goOn(next);
  };
  const mergeDoors = (item: Item, group: DoorGroup, plan: DoorMerge | null) => {
    const next = mergeChoices(choices, item.key, group, plan, hiddenObjects);
    setPatchChoices(next);
    setPreview(new Set());
    if (item === current) {
      goOn(next);
    }
  };
  const undecide = (item: Item) => setPatchChoices(undecideSpot(choices, item.key));
  const decideMods = (pairs: Pair[], plugin: string | null) => {
    // The duel of the two mods, if one gathers these findings: the pair's
    // verdict is its verdict (ADR 0048).
    const duel = report.duels.find((rendered) =>
      pairs.some((pair) => rendered.duel.findings.includes(pair.id)),
    );
    if (duel !== undefined && plugin !== null) {
      const loser = duel.duel.sides.findIndex((side) =>
        (side.plugins ?? []).some((name) => name.toLowerCase() === plugin.toLowerCase()),
      );
      if (loser === 0 || loser === 1) {
        decideDuel(duel.duel.id, loser === 0 ? 1 : 0);
      }
    }
    let next = choices;
    for (const pair of pairs) {
      const side =
        plugin === null
          ? null
          : pair.sides.find((candidate) => candidate.plugin.toLowerCase() === plugin.toLowerCase());
      if (side !== undefined) {
        next = chooseFinding(next, pair, side?.plugin ?? null);
      }
    }
    setPatchChoices(next);
  };
  const togglePreview = (plugin: string) => {
    const next = new Set(preview);
    if (next.has(plugin)) {
      next.delete(plugin);
    } else {
      next.add(plugin);
    }
    setPreview(next);
  };

  // Keys: arrows go through the spots, digits decide, V switches the markings.
  const keys = useRef<(event: KeyboardEvent) => void>(() => {});
  keys.current = (event) => {
    const target = event.target as HTMLElement | null;
    if (
      tab !== "spots" ||
      event.altKey ||
      event.ctrlKey ||
      event.metaKey ||
      target?.closest("input, select, textarea, [contenteditable]")
    ) {
      return;
    }
    if (event.key === "ArrowRight" || event.key === "ArrowDown") {
      go(step(1, shown));
    } else if (event.key === "ArrowLeft" || event.key === "ArrowUp") {
      go(step(-1, shown));
    } else if (event.key === "n") {
      go(step(1, (item) => decision(item) === undefined));
    } else if (event.key === "v") {
      setMarks(!marks);
    } else if (current !== null && event.key === "0") {
      decide(current, null);
    } else if (current !== null && (event.key === "1" || event.key === "2")) {
      const side = current.sides[Number(event.key) - 1];
      if (side?.hideable) {
        decide(current, side.plugin);
      }
    } else {
      return;
    }
    event.preventDefault();
  };
  useEffect(() => {
    const listen = (event: KeyboardEvent) => keys.current(event);
    window.addEventListener("keydown", listen);
    return () => window.removeEventListener("keydown", listen);
  }, []);

  const pairs = useMemo(() => pairsOf(report, ignored), [report, ignored]);
  const hiddenObjects = useMemo(() => objectsOf(pairs, choices), [pairs, choices]);
  // The suggestion about a mod that collides widely, when the list is
  // narrowed to that mod or to a pair with it (ADR 0043, 0061).
  const wide = useMemo(() => {
    const names = [...(pairAt ?? []), ...(needle.length >= 3 ? [needle] : [])].map((name) =>
      name.toLowerCase(),
    );
    return wideFindings(report.findings, ignored).filter((rendered) => {
      const mod = String(rendered.finding.args.mod ?? "").toLowerCase();
      return mod !== "" && names.some((name) => mod === name || mod.includes(name));
    });
  }, [report.findings, ignored, pairAt, needle]);
  const wideBanners = wide.map((rendered) => (
    <WideAdvice key={rendered.finding.id} rendered={rendered} />
  ));
  // A plugin of a mod switched off in Vortex that the game still loads.
  const stillLoaded = useMemo(() => {
    const found = new Map<string, string>();
    for (const plugin of report.switchedOffPlugins ?? []) {
      if (plugin.loaded) {
        found.set(plugin.plugin.toLowerCase(), plugin.mod.name);
      }
    }
    return found;
  }, [report.switchedOffPlugins]);
  const offHere = current?.sides.find((side) => stillLoaded.has(side.plugin.toLowerCase()));
  // What takes it out, on the spot as on the mod's page (ADR 0073).
  const offFinding =
    offHere === undefined
      ? undefined
      : report.findings.find(
          (rendered) =>
            rendered.finding.kind === STILL_LOADED &&
            String(rendered.finding.args.plugin ?? "").toLowerCase() ===
              offHere.plugin.toLowerCase(),
        );
  const offNote = offHere !== undefined && (
    <div className="still-loaded-note" role="note">
      <p>
        {switchedOffTexts[lang].stillLoadedSpot(
          stillLoaded.get(offHere.plugin.toLowerCase()) ?? offHere.plugin,
          offHere.plugin,
        )}
      </p>
      {offFinding !== undefined && (
        <div className="still-buttons">
          <DisablePluginsButton rendered={offFinding} />
          <ShowFilesButton files={stillFiles(offFinding)} />
        </div>
      )}
    </div>
  );
  const modCount = useMemo(
    () => new Set(items.filter((item) => item.pairs.length > 0).map((item) => item.mods.key)).size,
    [items],
  );

  const tabs = (
    <fieldset className="segmented work-tabs" aria-label={t.viewCollisions}>
      <button type="button" aria-pressed={tab === "spots"} onClick={() => changeTab("spots")}>
        {t.workTabSpots} <span className="count">{ordered.length}</span>
      </button>
      <button type="button" aria-pressed={tab === "mods"} onClick={() => changeTab("mods")}>
        {t.workMore} <span className="count">{modCount}</span>
      </button>
    </fieldset>
  );
  const toPatch = (
    <button type="button" className="primary" onClick={showPatch}>
      {t.workToPatch(hiddenObjects.length)}
    </button>
  );

  if (tab === "mods" || items.length === 0) {
    const suggested = pairs.filter((pair) => pair.suggested !== null);
    const withSpots = new Set(items.map((item) => item.spot.finding));
    return (
      <section className="work work-pairs-view">
        <div className="work-bar">
          {tabs}
          <span className="spacer" />
          {suggested.length > 0 && (
            <button
              type="button"
              onClick={() => {
                let next = choices;
                for (const pair of suggested) {
                  next = chooseFinding(next, pair, pair.suggested);
                }
                setPatchChoices(next);
              }}
            >
              {own.takeSuggestions(suggested.length)}
            </button>
          )}
          {toPatch}
        </div>
        {items.length === 0 && <p className="empty">{t.workEmpty}</p>}
        {wideBanners}
        <ModPairs
          items={items}
          decision={decision}
          sort={sort}
          duels={report.duels}
          focus={openedPair ?? focusPair}
          onDecideMods={decideMods}
          onShow={(item) => {
            changeBy("mods");
            setCurrentKey(item.key);
            changeTab("spots");
          }}
        />
        {report.findings.some(
          (rendered) => isCollision(rendered) && !withSpots.has(rendered.finding.id),
        ) && (
          <section className="work-also">
            <h3>{own.also}</h3>
            <p className="aside">{own.alsoNote}</p>
            <TaskList findings={report.findings} filter="collisions" exclude={withSpots} />
          </section>
        )}
      </section>
    );
  }
  const passage = current === null ? [] : (passages.get(current.passage) ?? []);
  return (
    <section className="work work-spots">
      <div className="work-bar">
        {tabs}
        <div className="work-progress" title={t.workProgress(decidedCount, ordered.length)}>
          <strong>{t.workProgress(decidedCount, ordered.length)}</strong>
          <span className="work-meter" aria-hidden="true">
            <span
              style={{
                width: `${ordered.length === 0 ? 0 : (decidedCount / ordered.length) * 100}%`,
              }}
            />
          </span>
        </div>
        <span className="spacer" />
        {toPatch}
      </div>
      <div className="work-bench">
        <div className="work-left">
          <div className="work-filters">
            <input
              type="search"
              className="work-search"
              placeholder={t.workSearch}
              aria-label={t.workSearch}
              value={query}
              onChange={(event) => setQuery(event.target.value)}
            />
            <fieldset className="work-kinds" aria-label={t.mapCollisions}>
              {WORK_KINDS.map((kind) => (
                <label key={kind} className="control-check" title={t.spotKindHints[kind]}>
                  <input
                    type="checkbox"
                    checked={kinds.has(kind)}
                    onChange={(event) => {
                      const next = new Set(kinds);
                      if (event.target.checked) {
                        next.add(kind);
                      } else {
                        next.delete(kind);
                      }
                      setKinds(next);
                      remember("mallet.workKinds3", [...next].join(","));
                    }}
                  />
                  <PlaceIcon shape="circle" origin="spot" kind={kind} />
                  {t.spotKinds[kind]} <span className="aside">{counts[kind]}</span>
                </label>
              ))}
            </fieldset>
            <fieldset className="segmented" aria-label={t.workGroupBy}>
              {(["region", "mods"] as const).map((grouping) => (
                <button
                  type="button"
                  key={grouping}
                  aria-pressed={by === grouping}
                  onClick={() => changeBy(grouping)}
                >
                  {grouping === "region" ? t.workByRegion : t.workByMods}
                </button>
              ))}
            </fieldset>
            <fieldset className="segmented" aria-label={t.workSortBy}>
              {(["open", "spots"] as const).map((sorting) => (
                <button
                  type="button"
                  key={sorting}
                  aria-pressed={sort === sorting}
                  onClick={() => changeSort(sorting)}
                >
                  {sorting === "open" ? t.workSortOpen : t.workSortSpots}
                </button>
              ))}
            </fieldset>
            <label className="control-check">
              <input
                type="checkbox"
                checked={hideDecided}
                onChange={(event) => {
                  setHideDecided(event.target.checked);
                  remember("mallet.workHideDecided", event.target.checked ? "on" : "off");
                }}
              />
              {t.workHideDecided}
            </label>
            {(pins.size > 0 || pinnedOnly) && (
              <label className="control-check">
                <input
                  type="checkbox"
                  checked={pinnedOnly}
                  onChange={(event) => {
                    setPinnedOnly(event.target.checked);
                    remember("mallet.workPinned", event.target.checked ? "on" : "off");
                  }}
                />
                <PinIcon /> {pinText.filter(pins.size)}
              </label>
            )}
            {wideBanners}
          </div>
          <WorkList
            groups={groups}
            by={by}
            current={current}
            shown={shown}
            searching={needle !== ""}
            decision={decision}
            pins={pins}
            onPick={go}
          />
        </div>
        <div className="work-stage">
          {offNote}
          {current === null ? (
            <p className="empty">{t.workPickSpot}</p>
          ) : (
            <Spot
              report={report}
              item={current}
              index={ordered.indexOf(current)}
              total={ordered.length}
              ordered={ordered}
              passage={passage}
              decided={decision(current)}
              ownChoice={current.key in choices}
              choices={choices}
              hiddenObjects={hiddenObjects}
              preview={preview}
              marks={marks}
              advance={advance}
              onMarks={setMarks}
              onAdvance={(next) => {
                setAdvance(next);
                remember("mallet.workAdvance2", next ? "on" : "off");
              }}
              onPreview={togglePreview}
              onDecide={(plugin) => decide(current, plugin)}
              onUndecide={() => undecide(current)}
              onDecideMods={decideMods}
              onOpenPair={() => {
                setOpenedPair(current.mods.key);
                changeTab("mods");
              }}
              onDecidePassage={decidePassage}
              onObject={(object, hide) => setPatchChoices(chooseObject(choices, object, hide))}
              onMergeDoors={(group, plan) => mergeDoors(current, group, plan)}
              onPick={(spot) => {
                const key = spotKey(spot);
                if (key !== null && byKey.has(key)) {
                  setCurrentKey(key);
                }
              }}
              onPrevious={() => go(step(-1, shown))}
              onNext={() => go(step(1, shown))}
              onNextOpen={() => go(step(1, (item) => decision(item) === undefined))}
            />
          )}
        </div>
      </div>
    </section>
  );
}

interface WorkListProps {
  groups: { group: Group; items: Item[] }[];
  by: Grouping;
  current: Item | null;
  shown: (item: Item) => boolean;
  /** Whether a search is on: every group with a find is open. */
  searching: boolean;
  decision: (item: Item) => string | null | undefined;
  /** The spots pinned to solve later, by key. */
  pins: Set<string>;
  onPick: (item: Item) => void;
}

/** The spots by group; the group of the current spot is open, others open on a click. */
function WorkList({
  groups,
  by,
  current,
  shown,
  searching,
  decision,
  pins,
  onPick,
}: WorkListProps) {
  const pinText = usePinTexts();
  const [open, setOpen] = useState<Set<string>>(new Set());
  const currentGroup = current === null ? null : current[by === "region" ? "region" : "mods"].key;
  const list = useRef<HTMLDivElement>(null);
  // The current spot stays in sight when it changes.
  const currentKey = current?.key;
  useEffect(() => {
    if (currentKey === undefined) {
      return;
    }
    const row = list.current?.querySelector<HTMLElement>(`[data-key="${CSS.escape(currentKey)}"]`);
    row?.scrollIntoView({ block: "nearest" });
  }, [currentKey]);

  return (
    <div className="work-list" ref={list}>
      {groups.map(({ group, items }) => {
        const visible = items.filter(shown);
        if (visible.length === 0) {
          return null;
        }
        const isOpen = searching || open.has(group.key) || group.key === currentGroup;
        const decided = items.filter((item) => decision(item) !== undefined).length;
        const kinds = WORK_KINDS.map((kind) => ({
          kind,
          count: items.filter((item) => item.kind === kind).length,
        })).filter((entry) => entry.count > 0);
        return (
          <section
            className="work-group"
            key={group.key}
            data-done={decided === items.length || undefined}
          >
            <button
              type="button"
              className="work-group-head"
              aria-expanded={isOpen}
              onClick={() => {
                const next = new Set(open);
                if (isOpen) {
                  next.delete(group.key);
                } else {
                  next.add(group.key);
                }
                setOpen(next);
              }}
            >
              <span className="work-group-name">
                {group.label}
                {group.aside !== null && <span className="aside"> · {group.aside}</span>}
              </span>
              <span className="work-group-counts">
                {kinds.map(({ kind, count }) => (
                  <span key={kind} className="work-count" data-kind={kind}>
                    {count}
                  </span>
                ))}
                <span className="aside">
                  {decided}/{items.length}
                </span>
              </span>
            </button>
            {isOpen && (
              <ul className="work-items">
                {visible.map((item, index) => (
                  <li
                    key={item.key}
                    className={
                      index > 0 && visible[index - 1]?.passage === item.passage
                        ? "work-in-passage"
                        : undefined
                    }
                  >
                    <button
                      type="button"
                      className="work-item"
                      aria-current={item === current ? "true" : undefined}
                      data-key={item.key}
                      onClick={() => onPick(item)}
                    >
                      <PlaceIcon shape="circle" origin="spot" kind={item.kind} />
                      <span className="work-item-text">
                        <span className="work-item-names">
                          {item.sides.map((side) => side.parts[0]?.name ?? side.plugin).join(" · ")}
                        </span>
                        <span className="aside">
                          {by === "region"
                            ? item.sides.map((side) => side.plugin).join(" · ")
                            : item.region.label}
                        </span>
                      </span>
                      {pins.has(item.key) && (
                        <span className="work-pin" title={pinText.pinned}>
                          <PinIcon />
                        </span>
                      )}
                      <Status decided={decision(item)} />
                    </button>
                  </li>
                ))}
              </ul>
            )}
          </section>
        );
      })}
    </div>
  );
}

/** What is decided for a spot, as a small mark. */
function Status({ decided }: { decided: string | null | undefined }) {
  const { t } = useSession();
  if (decided === undefined) {
    return <span className="work-status open">{t.workOpen}</span>;
  }
  if (decided === null) {
    return <span className="work-status kept">{t.workKept}</span>;
  }
  return (
    <span className="work-status hidden" title={t.workHidden(decided)}>
      {t.workHidden(shortName(decided))}
    </span>
  );
}

/** A plugin's name without its extension, to fit a small mark. */
function shortName(plugin: string): string {
  return plugin.replace(/\.(esp|esm|esl)$/i, "");
}

interface SpotProps {
  report: RenderedReport;
  item: Item;
  index: number;
  total: number;
  /** The spots of the list, drawn as pins where they are in the same world. */
  ordered: Item[];
  /** The spots of its passage, itself among them. */
  passage: Item[];
  decided: string | null | undefined;
  /** Whether this spot has a choice of its own rather than its pair's. */
  ownChoice: boolean;
  choices: PatchChoices;
  hiddenObjects: ObjectRef[];
  preview: Set<string>;
  marks: boolean;
  advance: boolean;
  onMarks: (marks: boolean) => void;
  onAdvance: (advance: boolean) => void;
  onPreview: (plugin: string) => void;
  onDecide: (plugin: string | null) => void;
  onUndecide: () => void;
  onDecideMods: (pairs: Pair[], plugin: string | null) => void;
  /** Opens the pair of this spot's two mods on the tab of the pairs. */
  onOpenPair: () => void;
  onDecidePassage: (passage: Item[], plugin: string, extra: ObjectRef[]) => void;
  onObject: (object: ObjectRef, hide: boolean) => void;
  /** Merges stacked doors into the one `plan` keeps (ADR 0067). */
  onMergeDoors: (group: DoorGroup, plan: DoorMerge | null) => void;
  onPick: (spot: MapSpot) => void;
  onPrevious: () => void;
  onNext: () => void;
  onNextOpen: () => void;
}

/** The spot looked at: what it is, what to do about it, and the place in 3D. */
function Spot(props: SpotProps) {
  const { t, lang, showOnMap } = useSession();
  const own = collisionsTexts[lang];
  const { item, passage, preview, decided, marks } = props;
  const { spot, pairs } = item;
  const depth = spot.depth ?? null;
  const pairSpots = spotsOf(pairs);
  const modSides = sidesOfMods(pairs);
  // The objects of the area, to hide a whole passage of one mod.
  const [area, setArea] = useState<AreaObjects | null>(null);
  // One mod's objects around the spot: how far, and which mod is looked at
  // without them, before deciding.
  const [metres, setMetres] = useState(AROUND);
  const [aroundPreview, setAroundPreview] = useState<string | null>(null);
  const nearby = useMemo(() => objectsAround(spot, area, metres), [spot, area, metres]);
  // Of the tied objects around, those the game can do without (ADR 0046).
  const { api } = useSession();
  const place = spot.location.type === "interior" ? spot.location.cell : spot.location.world;
  const tiedList = useMemo(() => [...nearby.tied.values()].flat(), [nearby]);
  const [letGo, setLetGo] = useState<Set<string>>(new Set());
  useEffect(() => {
    setLetGo(new Set());
    if (tiedList.length === 0) {
      return;
    }
    let outdated = false;
    api
      .objectAnchors(place, tiedList)
      .then((answers) => {
        if (!outdated && answers !== null) {
          setLetGo(
            new Set(
              answers
                .filter((answer) => answer.verdict !== "locked")
                .map((answer) => matchKey(answer.object)),
            ),
          );
        }
      })
      .catch(() => {});
    return () => {
      outdated = true;
    };
  }, [api, place, tiedList]);
  const around = useMemo(() => {
    const all = new Map<string, { objects: ObjectRef[]; careful: number }>();
    for (const [plugin, objects] of nearby.free) {
      all.set(plugin, { objects: [...objects], careful: 0 });
    }
    for (const [plugin, objects] of nearby.tied) {
      const allowed = objects.filter((object) => letGo.has(matchKey(object)));
      const entry = all.get(plugin) ?? { objects: [], careful: 0 };
      all.set(plugin, {
        objects: [...entry.objects, ...allowed],
        careful: entry.careful + allowed.length,
      });
    }
    return all;
  }, [nearby, letGo]);
  const previewObjects = aroundPreview === null ? [] : (around.get(aroundPreview)?.objects ?? []);
  const [first, second] = modSides;
  // The paths characters walk on over the parts a choice here would hide
  // (ADR 0051), asked where the stage reads the place: a city inside its
  // walls in the worldspace whose land it stands on.
  const w = useWalkTexts();
  const { location } = spot;
  const walkPlace =
    location.type === "interior"
      ? location.cell
      : (props.report.map.worlds.find(
          (world) =>
            world.name === location.world || (world.children ?? []).includes(location.world),
        )?.name ?? location.world);
  const at = spotPosition(spot);
  const walkQueries = useMemo<WalkQuery[]>(
    () =>
      item.sides
        .filter((side) => side.hideable)
        .flatMap((side) => side.parts.map((part) => ({ id: part.object }))),
    [item],
  );
  // The repair of the paths over what a choice would hide, shown in the
  // view (ADR 0058).
  const [repairShown, setRepairShown] = useState<PathsOverlay | null>(null);
  const walked = useWalkedOn(
    item.key,
    walkPlace,
    location.type === "interior" || at === null ? null : [at[0], at[1]],
    walkQueries,
  );
  /** The parts of a side characters walk on, and how many pieces of path lie on them. */
  const walkedOf = (side: Side) => {
    const parts = side.parts.filter(
      (part) => (walked?.get(objectKey(part.object.plugin, part.object.localId)) ?? 0) > 0,
    );
    const pieces = parts.reduce(
      (sum, part) => sum + (walked?.get(objectKey(part.object.plugin, part.object.localId)) ?? 0),
      0,
    );
    return { parts, pieces };
  };
  // Where characters could no longer go with a side's parts hidden (ADR
  // 0059): the ways are drawn for the side looked at without its parts.
  const walkCentre: [number, number] | null =
    location.type === "interior" || at === null ? null : [at[0], at[1]];
  const leftWalked = item.sides.find(
    (side) =>
      side.hideable &&
      (preview.has(side.plugin) || decided === side.plugin) &&
      walkedOf(side).parts.length > 0,
  );
  const leftReach = useReach(
    walkPlace,
    walkCentre,
    leftWalked === undefined ? [] : walkedOf(leftWalked).parts.map((part) => part.object),
  );
  const paintReach = usePaintReach(leftReach);

  /** One mod's column: hiding its part here, in the passage and around, and what ties it. */
  const sideColumn = (side: Side, at: number) => {
    const name = side.parts.map((part) => part.name).join(", ");
    const left = preview.has(side.plugin) || decided === side.plugin;
    const plugin = side.plugin.toLowerCase();
    const objects = around.get(plugin)?.objects ?? [];
    const careful = around.get(plugin)?.careful ?? 0;
    return (
      <div className="work-column" key={side.plugin}>
        <div className="work-column-head">
          <strong title={side.plugin}>{shortName(side.plugin)}</strong>
          {side.suggested && <span className="suggested-mark">{t.patchSuggested}</span>}
          <LocateButton objects={side.parts.map((part) => part.object)} at={spot.position} />
        </div>
        <div className="work-side" data-left={left || undefined}>
          <button
            type="button"
            className="work-eye"
            aria-pressed={preview.has(side.plugin)}
            title={t.workPreviewWithout(name)}
            aria-label={t.workPreviewWithout(name)}
            onClick={() => props.onPreview(side.plugin)}
          >
            <Eye open={!left} />
          </button>
          <button
            type="button"
            className="work-choice"
            aria-pressed={decided === side.plugin}
            disabled={!side.hideable}
            title={side.hideable ? undefined : t.workCannotHide}
            onClick={() => props.onDecide(side.plugin)}
          >
            <kbd>{at + 1}</kbd>
            <span className="work-choice-text">
              <span className="work-choice-verb">
                {side.hideable ? t.workHideVerb : t.workNeeded}
                {side.hideable && walkedOf(side).parts.length > 0 && (
                  <>
                    {" "}
                    <PathIcon />
                  </>
                )}
              </span>
              <strong className="work-choice-title">{name}</strong>
              <span className="aside">{side.plugin}</span>
            </span>
          </button>
        </div>
        <SpotMoves parts={side.parts} />
        {!side.hideable && (
          <div className="needed-parts">
            <span className="work-column-label">{t.anchorWhy}</span>
            {side.parts.map((part) => (
              <NeededPart
                key={`${part.object.plugin}:${part.object.localId}`}
                place={place}
                object={part.object}
                name={part.name}
              />
            ))}
          </div>
        )}
        {passage.length > 1 && (
          <button
            type="button"
            className="small work-wide"
            disabled={!side.hideable}
            onClick={() => props.onDecidePassage(passage, side.plugin, [])}
          >
            {own.passageHide(passage.length)}
          </button>
        )}
        {spot.location.type === "exterior" && (
          <div className="work-around">
            {area === null ? (
              <span className="aside">{own.aroundWaiting}</span>
            ) : (
              <>
                <button
                  type="button"
                  className="small work-eye-button"
                  aria-pressed={aroundPreview === plugin}
                  disabled={objects.length === 0}
                  title={own.aroundPreview(side.plugin)}
                  aria-label={own.aroundPreview(side.plugin)}
                  onClick={() => setAroundPreview(aroundPreview === plugin ? null : plugin)}
                >
                  <Eye open={aroundPreview !== plugin} />
                </button>
                <button
                  type="button"
                  className="small work-wide"
                  disabled={objects.length === 0}
                  onClick={() => {
                    setAroundPreview(null);
                    props.onDecidePassage([], side.plugin, objects);
                  }}
                >
                  {own.aroundHideHere(objects.length, metres)}
                </button>
                {careful > 0 && (
                  <span className="aside relief-object-warning" title={own.aroundCarefulHint}>
                    {own.aroundCareful(careful)}
                  </span>
                )}
              </>
            )}
          </div>
        )}
      </div>
    );
  };
  const [leftSide, ...rightSides] = item.sides;
  // The tools of this page in the bar over the view: how far "around"
  // reaches, the marks, and what the colours mean.
  const viewTools = (
    <>
      {spot.location.type === "exterior" && (
        <label className="view-tool" title={own.aroundHint}>
          <span className="view-tool-name">{own.aroundTool}</span>
          <input
            type="range"
            min={3}
            max={40}
            step={1}
            value={metres}
            onChange={(event) => setMetres(Number(event.target.value))}
          />
          <span className="view-tool-value">{own.metres(metres)}</span>
        </label>
      )}
      <span className="view-tool">
        <button
          type="button"
          className="small work-marks"
          aria-pressed={marks}
          title={t.workMarksHint}
          onClick={() => props.onMarks(!marks)}
        >
          <Eye open={marks} /> {t.workMarks}
        </button>
        {marks && (
          <span className="view-legend" aria-hidden="true">
            <span className="work-legend-focus" /> {t.workThisSpot}
            <span className="work-legend-ghost" /> {t.workGhost}
          </span>
        )}
      </span>
    </>
  );

  return (
    <>
      <div className="work-decision">
        <div className="work-nav">
          <span className="work-kind" data-kind={item.kind} title={t.spotKindHints[item.kind]}>
            {t.spotKinds[item.kind]}
          </span>
          <strong>{item.region.label}</strong>
          {item.region.aside !== null && <span className="aside">{item.region.aside}</span>}
          {depth !== null && item.kind !== "duplicate" && (
            <span className="aside">· {t.workDepth(depth / UNITS_PER_METRE)}</span>
          )}
          <span className="spacer" />
          <span className="aside" title={t.workKeys}>
            {props.index >= 0 ? t.workSpotOf(props.index + 1, props.total) : ""}
          </span>
          <span className="work-nav-group">
            <button type="button" className="small" onClick={props.onPrevious}>
              ‹ {t.workPrevious}
            </button>
            <button type="button" className="small" onClick={props.onNext}>
              {t.workNext} ›
            </button>
            <button type="button" className="small" onClick={props.onNextOpen}>
              {t.workNextOpen} »
            </button>
          </span>
          <span className="work-nav-more">
            <label className="control-check aside" title={t.workAfterDecideHint}>
              <input
                type="checkbox"
                checked={props.advance}
                onChange={(event) => props.onAdvance(event.target.checked)}
              />
              {t.workAfterDecide}
            </label>
            <PinButton keys={decided === undefined ? [item.key] : []} />
            {spot.location.type === "exterior" && (
              <button type="button" className="small quiet" onClick={() => showOnMap(spot)}>
                {own.showOnMap}
              </button>
            )}
          </span>
        </div>
        {spot.doors !== null && spot.doors !== undefined ? (
          <DoorPanel
            group={spot.doors}
            hidden={(object) => doorHidden(object, decided, props.hiddenObjects)}
            keptAll={decided === null}
            onMerge={(plan) => spot.doors && props.onMergeDoors(spot.doors, plan)}
            onKeepAll={() => spot.doors && props.onMergeDoors(spot.doors, null)}
            place={walkPlace}
            centre={walkCentre}
            onPreview={props.onPreview}
            previewed={preview}
          />
        ) : (
          <div className="work-columns">
            {leftSide !== undefined && sideColumn(leftSide, 0)}
            <div className="work-column work-common">
              <div className="work-column-head">
                <strong>{own.common}</strong>
              </div>
              <div className="work-common-choices">
                <button
                  type="button"
                  className="work-choice"
                  aria-pressed={decided === null}
                  onClick={() => props.onDecide(null)}
                >
                  <kbd>0</kbd>
                  <span className="work-choice-text">
                    <span className="work-choice-verb">{t.workKeepVerb}</span>
                    <strong className="work-choice-title">{t.patchKeepBoth}</strong>
                  </span>
                </button>
                {props.ownChoice && (
                  <button type="button" className="small quiet" onClick={props.onUndecide}>
                    {t.workReset}
                  </button>
                )}
              </div>
              {item.sides.every((side) => !side.hideable) && (
                <p className="work-note">{t.workBothNeeded}</p>
              )}
              {first !== undefined && second !== undefined && pairSpots > 1 && (
                <div className="work-pair-note" data-many={pairSpots >= MANY || undefined}>
                  <span>
                    {pairSpots >= MANY
                      ? t.workPairMany(first.plugin, second.plugin, pairSpots)
                      : own.pairSpots(shortName(first.plugin), shortName(second.plugin), pairSpots)}
                  </span>
                  <button type="button" className="small" onClick={props.onOpenPair}>
                    {own.openThePair}
                  </button>
                </div>
              )}
            </div>
            {rightSides.length > 0 && (
              <div className="work-column-stack">
                {rightSides.map((side, at) => sideColumn(side, at + 1))}
              </div>
            )}
          </div>
        )}
        {item.sides
          .filter((side) => side.hideable && walkedOf(side).parts.length > 0)
          .map((side) => {
            const { parts, pieces } = walkedOf(side);
            const names = [...new Set(parts.map((part) => part.name))].join(", ");
            return (
              <div key={side.plugin}>
                <p className="walk-warning">
                  <PathIcon />
                  <span>{w.spotWarning(names, shortName(side.plugin), pieces)}</span>
                </p>
                <SideReach
                  place={walkPlace}
                  centre={walkCentre}
                  objects={parts.map((part) => part.object)}
                />
                <RepairPaths
                  place={walkPlace}
                  centre={walkCentre}
                  objects={side.parts.map((part) => part.object)}
                  onShow={setRepairShown}
                />
              </div>
            );
          })}
      </div>
      <Stage
        {...props}
        onArea={setArea}
        previewObjects={previewObjects}
        tools={viewTools}
        paint={paintReach}
        paths={repairShown}
      />
    </>
  );
}

/** What hiding one side's parts would break for the characters (ADR 0059). */
function SideReach(props: {
  place: string | null;
  centre: [number, number] | null;
  objects: ObjectRef[];
}) {
  return (
    <ReachNote
      reach={useReach(props.place, props.centre, props.objects)}
      follow={
        props.place === null
          ? null
          : { place: props.place, centre: props.centre, hidden: props.objects }
      }
    />
  );
}

/**
 * The objects of each plugin that placed them (by lower-case name) within
 * `metres` of a spot, as far as the area read so far shows them: those
 * nothing ties (`free`), and those whose flags tie them (`tied`), which
 * ADR 0046 may still let go.
 */
function objectsAround(
  spot: MapSpot,
  area: AreaObjects | null,
  metres: number,
): { free: Map<string, ObjectRef[]>; tied: Map<string, ObjectRef[]> } {
  const free = new Map<string, ObjectRef[]>();
  const tied = new Map<string, ObjectRef[]>();
  const at = spotPosition(spot);
  if (area === null || at === null) {
    return { free, tied };
  }
  const reach = metres * UNITS_PER_METRE;
  for (const object of area.objects) {
    const [x, y, z] = object.position;
    if (Math.hypot(x - at[0], y - at[1], (z - at[2]) / 2) > reach) {
      continue;
    }
    const plugin = object.id.plugin.toLowerCase();
    const into = object.anchored ? tied : free;
    into.set(plugin, [...(into.get(plugin) ?? []), object.id]);
  }
  return { free, tied };
}

/** An eye, open or struck through. */
function Eye({ open }: { open: boolean }) {
  return (
    <svg className="eye" viewBox="0 0 24 24" aria-hidden="true">
      <path d="M2 12s3.6-7 10-7 10 7 10 7-3.6 7-10 7S2 12 2 12z" />
      <circle cx="12" cy="12" r="3" />
      {!open && <path d="M4 4l16 16" />}
    </svg>
  );
}

/** The spot in 3D, up close, with what is hidden drawn as ghosts or left out. */
function Stage({
  report,
  item,
  ordered,
  hiddenObjects,
  preview,
  marks,
  onPick,
  onDecide,
  onArea,
  previewObjects,
  tools,
  paint,
  paths = null,
}: SpotProps & {
  onArea: (area: AreaObjects | null) => void;
  /** Objects left out to look at, without deciding. */
  previewObjects: ObjectRef[];
  /** The page's tools for the bar over the view. */
  tools: ReactNode;
  /** What more to draw over the 3D view, such as the ways characters take. */
  paint?: ComponentProps<typeof MapView3D>["paint"];
  /** The repair of paths to show in the view (ADR 0058), if any. */
  paths?: PathsOverlay | null;
}) {
  const { t } = useSession();
  const location = item.spot.location;
  const worldName = location.type === "exterior" ? location.world : undefined;
  // A city inside its walls is drawn on the map of the world whose land it
  // uses, with its objects.
  const world = report.map.worlds.find(
    (candidate) =>
      candidate.name === worldName ||
      (worldName !== undefined && (candidate.children ?? []).includes(worldName)),
  );
  const picture = useWorldPicture(world?.terrain ? world.name : undefined);
  const [hover, setHover] = useState<Picked | null>(null);
  const worldSpots = useMemo(
    () =>
      ordered
        .map((candidate) => candidate.spot)
        .filter(
          (spot) =>
            spot.location.type === "exterior" &&
            world !== undefined &&
            (spot.location.world === world.name ||
              (world.children ?? []).includes(spot.location.world)),
        ),
    [ordered, world],
  );
  const markers = useMemo(
    () => (world?.markers ?? []).filter((marker) => marker.origin !== "removed"),
    [world],
  );
  const picked = useMemo<Picked>(() => ({ type: "spot", spot: item.spot }), [item.spot]);
  const parts = item.spot.parts ?? [];
  const focus = useMemo(
    () => new Set(parts.map((part) => objectKey(part.object.plugin, part.object.localId))),
    [parts],
  );
  const ghosts = useMemo(() => {
    const keys = new Set(hiddenObjects.map((object) => objectKey(object.plugin, object.localId)));
    for (const part of parts) {
      if (preview.has(part.object.plugin)) {
        keys.add(objectKey(part.object.plugin, part.object.localId));
      }
    }
    for (const object of previewObjects) {
      keys.add(objectKey(object.plugin, object.localId));
    }
    return keys;
  }, [hiddenObjects, parts, preview, previewObjects]);

  // An object of another collision picked in the view opens that collision.
  const { spots: spotsByFinding } = useSession();
  const openSpotOf = (object: AreaObject | null) => {
    if (object === null) {
      return;
    }
    const key = objectKey(object.id.plugin, object.id.localId);
    if (parts.some((part) => objectKey(part.object.plugin, part.object.localId) === key)) {
      return;
    }
    const spot = spotOf(spotsByFinding, object);
    if (spot !== null) {
      onPick(spot);
    }
  };

  // What can be done with an object picked in the view.
  const extra = (object: AreaObject, tools: PartTools) => {
    const key = objectKey(object.id.plugin, object.id.localId);
    const part = parts.find(
      (candidate) => objectKey(candidate.object.plugin, candidate.object.localId) === key,
    );
    if (part !== undefined) {
      return (
        <>
          <ActionPlaceInfo
            object={object}
            place={location.type === "interior" ? location.cell : location.world}
            interior={location.type === "interior"}
            around={tools.layer?.area?.objects}
          />
          {part.hideable ? (
            <button
              type="button"
              className="small primary relief-object-action"
              onClick={() => onDecide(part.object.plugin)}
            >
              {t.workHideObject}
            </button>
          ) : (
            <p className="aside relief-object-action">{t.workCannotHide}</p>
          )}
          <PartsTogether tools={tools} />
          <MoveControls
            object={object}
            tools={tools}
            place={location.type === "interior" ? location.cell : location.world}
            interior={location.type === "interior"}
            against={parts
              .filter((other) => other.object.plugin !== part.object.plugin)
              .map((other) => other.object)}
            spotKey={(() => {
              const key = spotKey(item.spot);
              return key === null ? null : { finding: item.spot.finding, key, parts };
            })()}
          />
        </>
      );
    }
    return (
      <FreeObjectActions
        object={object}
        tools={tools}
        place={location.type === "interior" ? location.cell : location.world}
        interior={location.type === "interior"}
      />
    );
  };

  if (location.type === "interior") {
    return (
      <InteriorView
        key={location.cell}
        cell={location.cell}
        centre={item.spot.position ?? null}
        focus={focus}
        hidden={ghosts}
        plain={!marks}
        objectExtra={extra}
        onObject={openSpotOf}
        onArea={onArea}
        tools={tools}
        paths={paths}
      />
    );
  }
  if (world === undefined || !world.terrain) {
    return (
      <div className="map-frame map-missing work-inside">
        <p className="empty">{t.workInside}</p>
        <div className="console">
          <code className="command" title={t.consoleHint}>
            {item.spot.console}
          </code>
          <CopyButton text={item.spot.console} />
        </div>
      </div>
    );
  }
  return (
    <MapView3D
      key={world.name}
      world={world}
      terrain={world.terrain}
      picture={picture}
      owner={-1}
      showCells={false}
      markers={marks ? markers : []}
      spots={marks ? worldSpots : []}
      picked={picked}
      hover={hover}
      onPick={(next) => {
        if (next?.type === "spot") {
          onPick(next.spot);
        }
      }}
      onHover={setHover}
      focus={focus}
      hidden={ghosts}
      objectExtra={extra}
      onObject={openSpotOf}
      spotDistance={CLOSE}
      plain={!marks}
      onArea={onArea}
      tools={tools}
      paint={paint}
      paths={paths}
    />
  );
}

interface ModPairsProps {
  items: Item[];
  decision: (item: Item) => string | null | undefined;
  sort: Sorting;
  /** The duels of the report, to tell where one was decided. */
  duels: RenderedDuel[];
  /** The pair to show first, by its key, if any. */
  focus: string | null;
  onDecideMods: (pairs: Pair[], plugin: string | null) => void;
  onShow: (item: Item) => void;
}

/**
 * What is decided for a pair of mods: on every spot the same mod hidden,
 * and whether the duel of the two (ADR 0029) says the same.
 */
function PairVerdict({
  findings,
  spots,
  decision,
  duels,
}: {
  findings: Pair[];
  spots: Item[];
  decision: (item: Item) => string | null | undefined;
  duels: RenderedDuel[];
}) {
  const { t, decisions } = useSession();
  const losers = new Set(spots.map((item) => decision(item)));
  const duel = duels.find((rendered) =>
    findings.some((pair) => rendered.duel.findings.includes(pair.id)),
  );
  const duelWinner = duel === undefined ? undefined : decisions[duel.duel.id];
  const [loser] = [...losers];
  if (losers.size !== 1 || loser === undefined || loser === null) {
    return duel !== undefined && duelWinner !== undefined ? (
      <span className="work-verdict">
        {t.workDuelWinner(duel.duel.sides[duelWinner === 0 ? 0 : 1].name)}
      </span>
    ) : null;
  }
  const winner = sidesOfMods(findings).find(
    (side) => side.plugin.toLowerCase() !== loser.toLowerCase(),
  );
  return (
    <span className="work-verdict" data-done="true">
      {t.workWinsEverywhere(winner?.plugin ?? "")}
    </span>
  );
}

/** The pairs of mods whose objects collide, the most first, to decide as a whole. */
function ModPairs({ items, decision, sort, duels, focus, onDecideMods, onShow }: ModPairsProps) {
  const { t, lang, openCompare } = useSession();
  const own = collisionsTexts[lang];
  const list = useRef<HTMLUListElement>(null);
  // The pair opened with comes into sight.
  useEffect(() => {
    if (focus === null) {
      return;
    }
    list.current
      ?.querySelector<HTMLElement>(`[data-key="${CSS.escape(focus)}"]`)
      ?.scrollIntoView({ block: "center" });
  }, [focus]);
  const pairs = useMemo(() => {
    const found = new Map<string, { key: string; pairs: Pair[]; items: Item[] }>();
    for (const item of items) {
      if (item.pairs.length === 0) {
        continue;
      }
      const entry = found.get(item.mods.key) ?? {
        key: item.mods.key,
        pairs: item.pairs,
        items: [],
      };
      entry.items.push(item);
      found.set(item.mods.key, entry);
    }
    const left = (entry: { items: Item[] }) =>
      entry.items.filter((item) => decision(item) === undefined).length;
    const serious = (entry: { items: Item[] }) =>
      entry.items.filter((item) => item.kind !== "touch").length;
    return [...found.values()].sort(
      (a, b) =>
        (left(a) === 0 ? 1 : 0) - (left(b) === 0 ? 1 : 0) ||
        (sort === "open" ? left(b) - left(a) : 0) ||
        serious(b) - serious(a) ||
        b.items.length - a.items.length,
    );
  }, [items, sort, decision]);
  if (pairs.length === 0) {
    return null;
  }
  return (
    <section className="work-pairs">
      <ul className="work-pair-list" ref={list}>
        {pairs.map(({ key, pairs: findings, items: spots }) => {
          const sides = sidesOfMods(findings);
          const decided = spots.filter((item) => decision(item) !== undefined).length;
          const kinds = WORK_KINDS.map((kind) => ({
            kind,
            count: spots.filter((item) => item.kind === kind).length,
          })).filter((entry) => entry.count > 0);
          const first = [...spots].sort((a, b) => RANK[a.kind] - RANK[b.kind])[0];
          return (
            <li
              className="work-pair-row"
              key={key}
              data-key={key}
              data-focus={key === focus || undefined}
              data-done={decided === spots.length || undefined}
            >
              <div className="work-pair-head">
                <strong>{sides.map((side) => side.plugin).join(" × ")}</strong>
                <PairVerdict findings={findings} spots={spots} decision={decision} duels={duels} />
                <span className="work-group-counts">
                  {kinds.map(({ kind, count }) => (
                    <span
                      key={kind}
                      className="work-count"
                      data-kind={kind}
                      title={t.spotKinds[kind]}
                    >
                      {count}
                    </span>
                  ))}
                  <span className="aside">
                    {decided}/{spots.length}
                  </span>
                </span>
              </div>
              <div className="work-pair-actions">
                {sides.map((side) => (
                  <button
                    type="button"
                    className="small"
                    key={side.plugin}
                    disabled={side.objects === 0}
                    onClick={() => onDecideMods(findings, side.plugin)}
                  >
                    {t.workEverywhere(side.plugin)}
                  </button>
                ))}
                <button
                  type="button"
                  className="small"
                  onClick={() => onDecideMods(findings, null)}
                >
                  {t.workKeepEverywhere}
                </button>
                {first !== undefined && (
                  <button type="button" className="small primary" onClick={() => onShow(first)}>
                    {t.workShowSpots}
                  </button>
                )}
                {(() => {
                  // The duel of the two about files is decided among the assets.
                  const duel = duels.find((rendered) =>
                    findings.some((pair) => rendered.duel.findings.includes(pair.id)),
                  );
                  const files = duel?.duel.topics.some(
                    (topic) => topic.topic !== "objects" && topic.topic !== "contained",
                  );
                  return duel !== undefined && files ? (
                    <button
                      type="button"
                      className="small quiet"
                      onClick={() => openCompare(duel.duel.id)}
                    >
                      {own.compareFiles}
                    </button>
                  ) : null;
                })()}
              </div>
            </li>
          );
        })}
      </ul>
    </section>
  );
}
