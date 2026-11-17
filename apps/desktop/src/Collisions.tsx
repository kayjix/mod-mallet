// The collisions as a worklist (ADR 0040): every spot where objects of two
// mods collide, grouped by the place it is near or by the pair of mods,
// worst first. A spot picked in the list is flown to in 3D, with its two
// objects lit up; the user decides there which of them the patch hides,
// spot by spot or for a pair of mods at once, and goes on to the next.

import { type ReactNode, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { CopyButton, useSession } from "./Actions";
import { objectKey } from "./area-layer";
import { isCollision } from "./arrange";
import type { AreaObject } from "./generated/area-objects";
import type { MapSpot, RenderedReport, SpotPart } from "./generated/rendered-report";
import { PlaceIcon, useWorldPicture } from "./MapView";
import { MapView3D } from "./MapView3D";
import {
  kindOf,
  nearest,
  type Picked,
  SPOT_KINDS,
  type SpotKind,
  spotPosition,
} from "./map-places";
import {
  chooseFinding,
  chooseSpot,
  decisionAt,
  objectsOf,
  type Pair,
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
  kind: SpotKind;
  pair: Pair | null;
  sides: Side[];
  /** The group it is in, by region and by pair of mods. */
  region: Group;
  mods: Group;
}

interface Group {
  key: string;
  label: string;
  aside: string | null;
}

type Grouping = "region" | "mods";

const RANK: Record<SpotKind, number> = { duplicate: 0, overlap: 1, touch: 2 };

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

/** The spots of all collisions not set aside that name their objects, worst first. */
function useItems(report: RenderedReport): Item[] {
  const { t, ignored } = useSession();
  return useMemo(() => {
    const pairs = new Map(pairsOf(report, ignored).map((pair) => [pair.id, pair]));
    const collisions = new Set(
      report.findings
        .filter((rendered) => isCollision(rendered) && !ignored.has(rendered.finding.id))
        .map((rendered) => rendered.finding.id),
    );
    const headlines = new Map(
      report.findings.map((rendered) => [rendered.finding.id, rendered.headline]),
    );
    const worlds = new Map(report.map.worlds.map((world) => [world.name, world]));
    const items: Item[] = [];
    for (const spot of report.map.spots) {
      const key = spotKey(spot);
      if (key === null || !collisions.has(spot.finding)) {
        continue;
      }
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
      const mods: Group = {
        key: spot.finding,
        label:
          plugins.length === 2
            ? `${plugins[0]} · ${plugins[1]}`
            : (headlines.get(spot.finding) ?? spot.finding),
        aside: null,
      };
      items.push({
        key,
        spot,
        kind: kindOf(spot),
        pair,
        sides: sidesOf(spot.parts ?? []),
        region,
        mods,
      });
    }
    return items;
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
      items: [...entry.items].sort(
        (a, b) => RANK[a.kind] - RANK[b.kind] || (b.spot.depth ?? 0) - (a.spot.depth ?? 0),
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

/** How far from a spot the camera comes, in cells: the two objects fill the view. */
const CLOSE = 0.9;

/** Game units in a metre, for the depth of a collision. */
const UNITS_PER_METRE = 70;

/**
 * The collisions as a list of spots to decide one by one, beside the spot
 * looked at in 3D, with what the patch would hide there.
 */
export function Collisions({
  report,
  start,
  children,
}: {
  report: RenderedReport;
  /** The spot to open with, if any. */
  start: MapSpot | null;
  /** Shown below the worklist. */
  children?: ReactNode;
}) {
  const { t, ignored, patchChoices: choices, setPatchChoices, showPatch } = useSession();
  const items = useItems(report);
  const [kinds, setKinds] = useState<Set<SpotKind>>(() =>
    remembered("mallet.workKinds", new Set<SpotKind>(["duplicate", "overlap"]), (value) => {
      const read = value
        .split(",")
        .filter((kind): kind is SpotKind => SPOT_KINDS.includes(kind as SpotKind));
      return new Set(read);
    }),
  );
  const [by, setBy] = useState<Grouping>(() =>
    remembered<Grouping>("mallet.workGroup", "region", (value) =>
      value === "mods" || value === "region" ? value : null,
    ),
  );
  const [hideDecided, setHideDecided] = useState(() =>
    remembered("mallet.workHideDecided", false, (value) => value === "on"),
  );
  const [advance, setAdvance] = useState(() =>
    remembered("mallet.workAdvance", true, (value) => value !== "off"),
  );

  const decision = useCallback(
    (item: Item, from = choices) => decisionAt(from, item.spot.finding, item.key),
    [choices],
  );
  const counts = useMemo(() => {
    const found: Record<SpotKind, number> = { duplicate: 0, overlap: 0, touch: 0 };
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
  const startKey = start === null ? null : spotKey(start);
  const [currentKey, setCurrentKey] = useState<string | null>(startKey);
  // A spot handed over later (from the map) is opened too.
  useEffect(() => {
    if (startKey !== null) {
      setCurrentKey(startKey);
    }
  }, [startKey]);
  const current =
    (currentKey === null ? undefined : byKey.get(currentKey)) ??
    ordered.find((item) => decision(item) === undefined) ??
    ordered[0] ??
    null;

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

  const decide = (item: Item, plugin: string | null) => {
    const next = chooseSpot(choices, item.key, plugin);
    setPatchChoices(next);
    if (advance && item === current) {
      go(step(1, (candidate) => decision(candidate, next) === undefined));
    }
  };
  const undecide = (item: Item) => setPatchChoices(undecideSpot(choices, item.key));
  const decideMods = (pair: Pair, plugin: string | null) => {
    setPatchChoices(chooseFinding(choices, pair, plugin));
  };

  // Keys: arrows go through the spots, digits decide.
  const keys = useRef<(event: KeyboardEvent) => void>(() => {});
  keys.current = (event) => {
    const target = event.target as HTMLElement | null;
    if (
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

  if (items.length === 0) {
    return (
      <>
        <p className="empty">{t.workEmpty}</p>
        {children}
      </>
    );
  }
  return (
    <>
      <section className="work">
        <div className="work-bar">
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
            {SPOT_KINDS.map((kind) => (
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
                    remember("mallet.workKinds", [...next].join(","));
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
                onClick={() => {
                  setBy(grouping);
                  remember("mallet.workGroup", grouping);
                }}
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
          <button type="button" className="primary" onClick={showPatch}>
            {t.workToPatch(hiddenObjects.length)}
          </button>
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
              <>
                <Decision
                  item={current}
                  index={ordered.indexOf(current)}
                  total={ordered.length}
                  decided={decision(current)}
                  ownChoice={current.key in choices}
                  advance={advance}
                  onAdvance={(next) => {
                    setAdvance(next);
                    remember("mallet.workAdvance", next ? "on" : "off");
                  }}
                  onDecide={(plugin) => decide(current, plugin)}
                  onUndecide={() => undecide(current)}
                  onDecideMods={decideMods}
                  onPrevious={() => go(step(-1, shown))}
                  onNext={() => go(step(1, shown))}
                  onNextOpen={() => go(step(1, (item) => decision(item) === undefined))}
                />
                <Stage
                  report={report}
                  item={current}
                  spots={ordered}
                  hidden={hiddenObjects}
                  onPick={(spot) => {
                    const key = spotKey(spot);
                    if (key !== null && byKey.has(key)) {
                      setCurrentKey(key);
                    }
                  }}
                  onHideObject={(plugin) => decide(current, plugin)}
                />
              </>
            )}
          </div>
        </div>
      </section>
      {children}
    </>
  );
}

interface WorkListProps {
  groups: { group: Group; items: Item[] }[];
  by: Grouping;
  current: Item | null;
  shown: (item: Item) => boolean;
  decision: (item: Item) => string | null | undefined;
  onPick: (item: Item) => void;
  onDecideMods: (pair: Pair, plugin: string | null) => void;
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
        const kinds = SPOT_KINDS.map((kind) => ({
          kind,
          count: items.filter((item) => item.kind === kind).length,
        })).filter((entry) => entry.count > 0);
        const pair = by === "mods" ? (items[0]?.pair ?? null) : null;
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
            {isOpen && pair !== null && (
              <div className="work-group-actions">
                {pair.sides.map((side) => (
                  <button
                    type="button"
                    className="small"
                    key={side.plugin}
                    disabled={side.objects.length === 0}
                    onClick={() => onDecideMods(pair, side.plugin)}
                  >
                    {t.workEverywhere(side.plugin)}
                  </button>
                ))}
                <button type="button" className="small" onClick={() => onDecideMods(pair, null)}>
                  {t.workKeepEverywhere}
                </button>
              </div>
            )}
            {isOpen && (
              <ul className="work-items">
                {visible.map((item) => (
                  <li key={item.key}>
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

interface DecisionProps {
  item: Item;
  index: number;
  total: number;
  decided: string | null | undefined;
  /** Whether this spot has a choice of its own rather than its pair's. */
  ownChoice: boolean;
  advance: boolean;
  onAdvance: (advance: boolean) => void;
  onDecide: (plugin: string | null) => void;
  onUndecide: () => void;
  onDecideMods: (pair: Pair, plugin: string | null) => void;
  onPrevious: () => void;
  onNext: () => void;
  onNextOpen: () => void;
}

/** The spot looked at: what it is, and what to do about it. */
function Decision({
  item,
  index,
  total,
  decided,
  ownChoice,
  advance,
  onAdvance,
  onDecide,
  onUndecide,
  onDecideMods,
  onPrevious,
  onNext,
  onNextOpen,
}: DecisionProps) {
  const { t } = useSession();
  const { spot, pair } = item;
  const depth = spot.depth ?? null;
  const pairSpots = pair?.spots.length ?? 0;
  return (
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
        <span className="aside">{index >= 0 ? t.workSpotOf(index + 1, total) : ""}</span>
        <button type="button" className="small" onClick={onPrevious}>
          ‹ {t.workPrevious}
        </button>
        <button type="button" className="small" onClick={onNext}>
          {t.workNext} ›
        </button>
        <button type="button" className="small" onClick={onNextOpen}>
          {t.workNextOpen} »
        </button>
      </div>
      <div className="work-choices">
        {item.sides.map((side, at) => {
          const name = side.parts.map((part) => part.name).join(", ");
          return (
            <button
              type="button"
              key={side.plugin}
              className="work-choice"
              aria-pressed={decided === side.plugin}
              disabled={!side.hideable}
              title={side.hideable ? undefined : t.workCannotHide}
              onClick={() => onDecide(side.plugin)}
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
          );
        })}
        <button
          type="button"
          className="work-choice"
          aria-pressed={decided === null}
          onClick={() => onDecide(null)}
        >
          <kbd>0</kbd>
          <span className="work-choice-text">
            <strong className="work-choice-title">{t.patchKeepBoth}</strong>
          </span>
        </button>
        {ownChoice && (
          <button type="button" className="small quiet" onClick={onUndecide}>
            {t.workReset}
          </button>
        )}
      </div>
      <div className="work-decision-foot">
        {pair !== null && pair.sides.length === 2 && pairSpots > 1 && (
          <span className="work-pair">
            <span className="aside">
              {t.workPair(pair.sides[0]?.plugin ?? "", pair.sides[1]?.plugin ?? "", pairSpots)}
            </span>
            {pair.sides.map((side) => (
              <button
                type="button"
                className="small"
                key={side.plugin}
                disabled={side.objects.length === 0}
                onClick={() => onDecideMods(pair, side.plugin)}
              >
                {t.workEverywhere(side.plugin)}
              </button>
            ))}
          </span>
        )}
        <span className="spacer" />
        <label className="control-check aside">
          <input
            type="checkbox"
            checked={advance}
            onChange={(event) => onAdvance(event.target.checked)}
          />
          {t.workAfterDecide}
        </label>
        <span className="aside work-keys">{t.workKeys}</span>
      </div>
    </div>
  );
}

interface StageProps {
  report: RenderedReport;
  item: Item;
  /** The spots of the list, drawn as pins where they are in the same world. */
  spots: Item[];
  hidden: { plugin: string; localId: number }[];
  onPick: (spot: MapSpot) => void;
  onHideObject: (plugin: string) => void;
}

/** The spot in 3D, up close, with the objects of the patch drawn as ghosts. */
function Stage({ report, item, spots, hidden, onPick, onHideObject }: StageProps) {
  const { t } = useSession();
  const location = item.spot.location;
  const worldName = location.type === "exterior" ? location.world : undefined;
  const world = report.map.worlds.find((candidate) => candidate.name === worldName);
  const picture = useWorldPicture(world?.terrain ? worldName : undefined);
  const [hover, setHover] = useState<Picked | null>(null);
  const worldSpots = useMemo(
    () =>
      spots
        .map((candidate) => candidate.spot)
        .filter((spot) => spot.location.type === "exterior" && spot.location.world === worldName),
    [spots, worldName],
  );
  const markers = useMemo(
    () => (world?.markers ?? []).filter((marker) => marker.origin !== "removed"),
    [world],
  );
  const picked = useMemo<Picked>(() => ({ type: "spot", spot: item.spot }), [item.spot]);
  const focus = useMemo(
    () =>
      new Set(
        (item.spot.parts ?? []).map((part) => objectKey(part.object.plugin, part.object.localId)),
      ),
    [item.spot],
  );
  const ghosts = useMemo(
    () => new Set(hidden.map((object) => objectKey(object.plugin, object.localId))),
    [hidden],
  );
  const extra = (object: AreaObject) => {
    const key = objectKey(object.id.plugin, object.id.localId);
    const part = (item.spot.parts ?? []).find(
      (candidate) => objectKey(candidate.object.plugin, candidate.object.localId) === key,
    );
    if (part === undefined) {
      return null;
    }
    return part.hideable ? (
      <button
        type="button"
        className="small primary relief-object-action"
        onClick={() => onHideObject(part.object.plugin)}
      >
        {t.workHideObject}
      </button>
    ) : (
      <p className="aside relief-object-action">{t.workCannotHide}</p>
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
      markers={markers}
      spots={worldSpots}
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
    >
      <p className="work-legend">
        <span className="work-legend-focus" /> {t.workThisSpot}
        <span className="work-legend-ghost" /> {t.workGhost}
      </p>
    </MapView3D>
  );
}
