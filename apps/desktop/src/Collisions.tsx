// The collisions as a worklist (ADR 0040): every spot where objects of two
// mods collide, grouped by the place it is near or by the pair of mods,
// worst first. A spot picked in the list is flown to in 3D, with its two
// objects lit up; the user looks at it with one side or the other left
// out, decides which of them the patch hides (spot by spot, for a passage
// of spots close together, or for a pair of mods at once) and goes on.

import { type ReactNode, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { CopyButton, useSession } from "./Actions";
import { objectKey } from "./area-layer";
import { isCollision } from "./arrange";
import type { AreaObject, AreaObjects } from "./generated/area-objects";
import type { MapSpot, ObjectRef, RenderedReport, SpotPart } from "./generated/rendered-report";
import { PlaceIcon, useWorldPicture } from "./MapView";
import { MapView3D } from "./MapView3D";
import { kindOf, nearest, type Picked, spotPosition, type WorkKind } from "./map-places";
import {
  chooseFinding,
  chooseObject,
  chooseSpot,
  decisionAt,
  keyOf,
  objectChoiceKey,
  objectsOf,
  type Pair,
  type PatchChoices,
  pairsOf,
  spotKey,
  undecideSpot,
} from "./patch";

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
}

interface Group {
  key: string;
  label: string;
  aside: string | null;
}

type Grouping = "region" | "mods";

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
const WORK_KINDS: WorkKind[] = ["duplicate", "overlap", "deep", "touch"];

const RANK: Record<WorkKind, number> = { duplicate: 0, overlap: 1, deep: 2, touch: 3 };

/** Game units in a metre, for the depth of a collision. */
const UNITS_PER_METRE = 70;

/** From this crossing on, in game units (2 m), a touch counts as deep. */
const DEEP = 140;

/** Spots of the same two mods this close (14 m) belong to one passage. */
const PASSAGE = 1000;

/** How far around a passage's spots its objects count, in game units. */
const PASSAGE_MARGIN = 300;

/** How far from a spot the camera comes, in cells: the two objects fill the view. */
const CLOSE = 0.9;

/** From this many spots on, a pair of mods is better decided as a whole. */
const MANY = 20;

/** The plugins of the games themselves, whose objects quests may need. */
const GAME_PLUGIN =
  /^(skyrim|update|dawnguard|hearthfires|dragonborn|fallout4|dlc\w*|oblivion|fallout3|falloutnv)\.esm$|^cc\w*\.es[lm]$/i;

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
      };
    });
  }, [report, ignored, t]);
}

/** Groups of items, the groups with the most serious spots first, each worst first. */
function grouped(items: Item[], by: Grouping): { group: Group; items: Item[] }[] {
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
  children,
}: {
  report: RenderedReport;
  /** The spot to open with, if any. */
  start: MapSpot | null;
  /** The duels and the remaining findings, on the tab of the pairs of mods. */
  children?: ReactNode;
}) {
  const { t, ignored, patchChoices: choices, setPatchChoices, showPatch } = useSession();
  const items = useItems(report);
  const [tab, setTab] = useState<"spots" | "mods">(() =>
    start !== null
      ? "spots"
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
      "mallet.workKinds2",
      new Set<WorkKind>(["duplicate", "overlap", "deep"]),
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
    (item: Item, from: PatchChoices = choices) => decisionAt(from, item.spot.finding, item.key),
    [choices],
  );
  const counts = useMemo(() => {
    const found: Record<WorkKind, number> = { duplicate: 0, overlap: 0, deep: 0, touch: 0 };
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
      ),
    [items, kinds, by],
  );
  /** Every spot of the chosen kinds, in the order of the list. */
  const ordered = useMemo(() => groups.flatMap((entry) => entry.items), [groups]);
  const decidedCount = ordered.filter((item) => decision(item) !== undefined).length;
  const shown = (item: Item) => !hideDecided || decision(item) === undefined;

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
  const undecide = (item: Item) => setPatchChoices(undecideSpot(choices, item.key));
  const decideMods = (pairs: Pair[], plugin: string | null) => {
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
    return (
      <section className="work work-pairs-view">
        <div className="work-bar">
          {tabs}
          <span className="spacer" />
          {toPatch}
        </div>
        {items.length === 0 && <p className="empty">{t.workEmpty}</p>}
        <ModPairs
          items={items}
          decision={decision}
          onDecideMods={decideMods}
          onShow={(item) => {
            changeBy("mods");
            setCurrentKey(item.key);
            changeTab("spots");
          }}
        />
        {children}
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
                  remember("mallet.workKinds2", [...next].join(","));
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
        <span className="spacer" />
        {toPatch}
      </div>
      <div className="work-bench">
        <WorkList
          groups={groups}
          by={by}
          current={current}
          shown={shown}
          decision={decision}
          onPick={go}
          onDecideMods={decideMods}
        />
        <div className="work-stage">
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
              onDecidePassage={decidePassage}
              onObject={(object, hide) => setPatchChoices(chooseObject(choices, object, hide))}
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
  decision: (item: Item) => string | null | undefined;
  onPick: (item: Item) => void;
  onDecideMods: (pairs: Pair[], plugin: string | null) => void;
}

/** The spots by group; the group of the current spot is open, others open on a click. */
function WorkList({ groups, by, current, shown, decision, onPick, onDecideMods }: WorkListProps) {
  const { t } = useSession();
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
        const isOpen = open.has(group.key) || group.key === currentGroup;
        const decided = items.filter((item) => decision(item) !== undefined).length;
        const kinds = WORK_KINDS.map((kind) => ({
          kind,
          count: items.filter((item) => item.kind === kind).length,
        })).filter((entry) => entry.count > 0);
        const pairs = by === "mods" ? (items[0]?.pairs ?? []) : [];
        return (
          <section className="work-group" key={group.key}>
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
            {isOpen && pairs.length > 0 && (
              <div className="work-group-actions">
                {sidesOfMods(pairs).map((side) => (
                  <button
                    type="button"
                    className="small"
                    key={side.plugin}
                    disabled={side.objects === 0}
                    onClick={() => onDecideMods(pairs, side.plugin)}
                  >
                    {t.workEverywhere(side.plugin)}
                  </button>
                ))}
                <button type="button" className="small" onClick={() => onDecideMods(pairs, null)}>
                  {t.workKeepEverywhere}
                </button>
              </div>
            )}
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
  onDecidePassage: (passage: Item[], plugin: string, extra: ObjectRef[]) => void;
  onObject: (object: ObjectRef, hide: boolean) => void;
  onPick: (spot: MapSpot) => void;
  onPrevious: () => void;
  onNext: () => void;
  onNextOpen: () => void;
}

/** The spot looked at: what it is, what to do about it, and the place in 3D. */
function Spot(props: SpotProps) {
  const { t } = useSession();
  const { item, passage, preview, decided, marks } = props;
  const { spot, pairs } = item;
  const depth = spot.depth ?? null;
  const pairSpots = spotsOf(pairs);
  const modSides = sidesOfMods(pairs);
  // The objects of the area, to hide a whole passage of one mod.
  const [area, setArea] = useState<AreaObjects | null>(null);
  const around = useMemo(() => passageObjects(passage, area), [passage, area]);
  const [first, second] = modSides;

  return (
    <>
      <div className="work-decision">
        <div className="work-decision-head">
          <span className="work-kind" data-kind={item.kind} title={t.spotKindHints[item.kind]}>
            {t.spotKinds[item.kind]}
          </span>
          <strong>{item.region.label}</strong>
          {item.region.aside !== null && <span className="aside">{item.region.aside}</span>}
          {depth !== null && item.kind !== "duplicate" && (
            <span className="aside">· {t.workDepth(depth / UNITS_PER_METRE)}</span>
          )}
          <span className="spacer" />
          <span className="aside">
            {props.index >= 0 ? t.workSpotOf(props.index + 1, props.total) : ""}
          </span>
          <button type="button" className="small" onClick={props.onPrevious}>
            ‹ {t.workPrevious}
          </button>
          <button type="button" className="small" onClick={props.onNext}>
            {t.workNext} ›
          </button>
          <button type="button" className="small" onClick={props.onNextOpen}>
            {t.workNextOpen} »
          </button>
        </div>
        <div className="work-choices">
          {item.sides.map((side, at) => {
            const name = side.parts.map((part) => part.name).join(", ");
            const left = preview.has(side.plugin) || decided === side.plugin;
            return (
              <div className="work-side" key={side.plugin} data-left={left || undefined}>
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
                    <strong className="work-choice-title">{t.workHide(name)}</strong>
                    <span className="aside">
                      {side.plugin}
                      {side.suggested && <span className="suggested-mark">{t.patchSuggested}</span>}
                    </span>
                  </span>
                </button>
              </div>
            );
          })}
          <button
            type="button"
            className="work-choice"
            aria-pressed={decided === null}
            onClick={() => props.onDecide(null)}
          >
            <kbd>0</kbd>
            <span className="work-choice-text">
              <strong className="work-choice-title">{t.patchKeepBoth}</strong>
            </span>
          </button>
          {props.ownChoice && (
            <button type="button" className="small quiet" onClick={props.onUndecide}>
              {t.workReset}
            </button>
          )}
        </div>
        {passage.length > 1 && (
          <div className="work-line">
            <span className="aside">{t.workPassage(passage.length)}</span>
            {item.sides.map((side) => {
              const extra = around.get(side.plugin.toLowerCase()) ?? [];
              return (
                <span key={side.plugin} className="work-line-group">
                  <button
                    type="button"
                    className="small"
                    disabled={!side.hideable}
                    onClick={() => props.onDecidePassage(passage, side.plugin, [])}
                  >
                    {t.workPassageHide(side.plugin)}
                  </button>
                  {extra.length > 0 && (
                    <button
                      type="button"
                      className="small"
                      onClick={() => props.onDecidePassage(passage, side.plugin, extra)}
                    >
                      {t.workPassageAll(side.plugin, extra.length)}
                    </button>
                  )}
                </span>
              );
            })}
          </div>
        )}
        {first !== undefined && second !== undefined && pairSpots > 1 && (
          <div className="work-line" data-many={pairSpots >= MANY || undefined}>
            <span className="aside">
              {pairSpots >= MANY
                ? t.workPairMany(first.plugin, second.plugin, pairSpots)
                : t.workPair(first.plugin, second.plugin, pairSpots)}
            </span>
            {modSides.map((side) => (
              <button
                type="button"
                className="small"
                key={side.plugin}
                disabled={side.objects === 0}
                onClick={() => props.onDecideMods(pairs, side.plugin)}
              >
                {t.workEverywhere(side.plugin)}
              </button>
            ))}
          </div>
        )}
        <div className="work-decision-foot">
          <button
            type="button"
            className="small work-marks"
            aria-pressed={marks}
            title={t.workMarksHint}
            onClick={() => props.onMarks(!marks)}
          >
            <Eye open={marks} /> {t.workMarks}
          </button>
          <label className="control-check aside">
            <input
              type="checkbox"
              checked={props.advance}
              onChange={(event) => props.onAdvance(event.target.checked)}
            />
            {t.workAfterDecide}
          </label>
          <span className="spacer" />
          <span className="aside work-keys">{t.workKeys}</span>
        </div>
      </div>
      <Stage {...props} onArea={setArea} />
    </>
  );
}

/**
 * The objects of each plugin (by lower-case name) placed around a passage's
 * spots, as far as the area read so far shows them, that may be hidden.
 */
function passageObjects(passage: Item[], area: AreaObjects | null): Map<string, ObjectRef[]> {
  const found = new Map<string, ObjectRef[]>();
  const at = passage.flatMap((item) => {
    const position = spotPosition(item.spot);
    return position === null || item.spot.location.type !== "exterior" ? [] : [position];
  });
  if (area === null || at.length < 2) {
    return found;
  }
  const xs = at.map((point) => point[0]);
  const ys = at.map((point) => point[1]);
  const [west, east] = [Math.min(...xs) - PASSAGE_MARGIN, Math.max(...xs) + PASSAGE_MARGIN];
  const [south, north] = [Math.min(...ys) - PASSAGE_MARGIN, Math.max(...ys) + PASSAGE_MARGIN];
  for (const object of area.objects) {
    const [x, y] = object.position;
    if (object.anchored || x < west || x > east || y < south || y > north) {
      continue;
    }
    const plugin = object.id.plugin.toLowerCase();
    found.set(plugin, [...(found.get(plugin) ?? []), object.id]);
  }
  return found;
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
  choices,
  onPick,
  onDecide,
  onObject,
  onArea,
}: SpotProps & { onArea: (area: AreaObjects | null) => void }) {
  const { t } = useSession();
  const location = item.spot.location;
  const worldName = location.type === "exterior" ? location.world : undefined;
  const world = report.map.worlds.find((candidate) => candidate.name === worldName);
  const picture = useWorldPicture(world?.terrain ? worldName : undefined);
  const [hover, setHover] = useState<Picked | null>(null);
  const worldSpots = useMemo(
    () =>
      ordered
        .map((candidate) => candidate.spot)
        .filter((spot) => spot.location.type === "exterior" && spot.location.world === worldName),
    [ordered, worldName],
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
    return keys;
  }, [hiddenObjects, parts, preview]);
  const hiddenKeys = useMemo(() => new Set(hiddenObjects.map(keyOf)), [hiddenObjects]);

  // What can be done with an object picked in the view.
  const extra = (object: AreaObject) => {
    const key = objectKey(object.id.plugin, object.id.localId);
    const part = parts.find(
      (candidate) => objectKey(candidate.object.plugin, candidate.object.localId) === key,
    );
    if (part !== undefined) {
      return part.hideable ? (
        <button
          type="button"
          className="small primary relief-object-action"
          onClick={() => onDecide(part.object.plugin)}
        >
          {t.workHideObject}
        </button>
      ) : (
        <p className="aside relief-object-action">{t.workCannotHide}</p>
      );
    }
    if (object.anchored) {
      return <p className="aside relief-object-action">{t.workObjectLocked}</p>;
    }
    const own = objectChoiceKey(object.id) in choices;
    const hidden = hiddenKeys.has(keyOf(object.id));
    if (hidden && !own) {
      return null;
    }
    return (
      <div className="relief-object-action">
        {GAME_PLUGIN.test(object.id.plugin) && !own && (
          <p className="aside relief-object-warning">{t.workObjectGame}</p>
        )}
        <button
          type="button"
          className={own ? "small" : "small primary"}
          onClick={() => onObject(object.id, !own)}
        >
          {own ? t.workObjectShow : t.workObjectHide}
        </button>
      </div>
    );
  };

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
      spotDistance={CLOSE}
      plain={!marks}
      onArea={onArea}
    >
      {marks && (
        <p className="work-legend">
          <span className="work-legend-focus" /> {t.workThisSpot}
          <span className="work-legend-ghost" /> {t.workGhost}
        </p>
      )}
    </MapView3D>
  );
}

interface ModPairsProps {
  items: Item[];
  decision: (item: Item) => string | null | undefined;
  onDecideMods: (pairs: Pair[], plugin: string | null) => void;
  onShow: (item: Item) => void;
}

/** The pairs of mods whose objects collide, the most first, to decide as a whole. */
function ModPairs({ items, decision, onDecideMods, onShow }: ModPairsProps) {
  const { t } = useSession();
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
    return [...found.values()].sort(
      (a, b) =>
        b.items.filter((item) => item.kind !== "touch").length -
          a.items.filter((item) => item.kind !== "touch").length || b.items.length - a.items.length,
    );
  }, [items]);
  if (pairs.length === 0) {
    return null;
  }
  return (
    <section className="work-pairs">
      <p className="aside">{t.workPairsIntro}</p>
      <ul className="work-pair-list">
        {pairs.map(({ key, pairs: findings, items: spots }) => {
          const sides = sidesOfMods(findings);
          const decided = spots.filter((item) => decision(item) !== undefined).length;
          const kinds = WORK_KINDS.map((kind) => ({
            kind,
            count: spots.filter((item) => item.kind === kind).length,
          })).filter((entry) => entry.count > 0);
          const first = [...spots].sort((a, b) => RANK[a.kind] - RANK[b.kind])[0];
          return (
            <li className="work-pair-row" key={key}>
              <div className="work-pair-head">
                <strong>{sides.map((side) => side.plugin).join(" × ")}</strong>
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
              </div>
            </li>
          );
        })}
      </ul>
    </section>
  );
}
