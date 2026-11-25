// Doors that mods stack on top of each other (ADR 0067): the doors of a
// spot side by side, each with where it leads, and what merging into one
// of them means. The collisions' page and the guided review show it; the
// choice itself is the patch's, as for any other spot.

import { type ReactNode, useEffect, useState } from "react";
import "./doors.css";
import { useSession } from "./Actions";
import { mergedBy } from "./decisions";
import { type DoorTexts, doorTexts } from "./door-texts";
import type { DoorReach as DoorReachFound } from "./generated/door-reach";
import type {
  DoorGroup,
  DoorMerge,
  DoorRelink,
  MapSpot,
  ObjectMove,
  ObjectRef,
  RepairPlace,
  StackedDoor,
} from "./generated/rendered-report";
import { decisionAt, keptBy, matchKey, type PatchChoices, spotKey } from "./patch";
import { ReachNote, useReach } from "./reach";
import { Note, NoteRow, useNotesInFull } from "./spot-notes";
import { spotNotesTexts } from "./spot-notes-texts";

/** Game units in a centimetre. */
const UNITS_PER_CENTIMETRE = 0.7;

const sameObject = (a: ObjectRef, b: ObjectRef) =>
  a.plugin.toLowerCase() === b.plugin.toLowerCase() && a.localId === b.localId;

/** A plugin's name without its ending. */
const short = (plugin: string) => plugin.replace(/\.(esp|esm|esl)$/i, "");

/** A place as people can find it: an interior by its name, outside by world and cell. */
export function placeName(location: MapSpot["location"]): string {
  return location.type === "interior"
    ? location.cell
    : `${location.world} ${location.x}, ${location.y}`;
}

const centimetres = (units: number) => Math.round(units / UNITS_PER_CENTIMETRE);

/**
 * The door a group is merged into: the one door not hidden while all the
 * others are; `null` while none or more than one stays.
 */
export function mergedInto(
  group: DoorGroup,
  hidden: (object: ObjectRef) => boolean,
): ObjectRef | null {
  const staying = group.doors.filter((door) => !hidden(door.object));
  return staying.length === 1 && group.doors.length > 1 ? (staying[0]?.object ?? null) : null;
}

interface DoorPanelProps {
  group: DoorGroup;
  /** Whether the patch hides a door. */
  hidden: (object: ObjectRef) => boolean;
  /** Keeps the plan's door and hides the others. */
  onMerge: (plan: DoorMerge) => void;
  /** Keeps every door. */
  onKeepAll: () => void;
  /** Whether everything stays: the spot is decided as "keep all". */
  keptAll: boolean;
  /** The door looked at alone in the 3D view, the others as ghosts. */
  looking?: ObjectRef | null;
  /** Looks at one door alone in the 3D view, or at all again with null. */
  onLookAt?: (object: ObjectRef | null) => void;
  /** Makes one door blink for a moment in the 3D view, the camera left where it is. */
  onLocate?: (door: StackedDoor) => void;
  /** The pair of mods whose doors these are, to open all their collisions. */
  pair?: { first: string; second: string; onOpen: () => void } | null;
  /** The worldspace or interior cell the doors are in, to ask where characters still go. */
  place: string | null;
  /** Outside: where the doors stand; null inside. */
  centre: [number, number] | null;
  /** More notes about hiding one door, shown in its card (see `spot-notes.tsx`). */
  notesOf?: (door: StackedDoor) => ReactNode;
}

/**
 * The doors of a spot side by side, and what keeping one of them means:
 * where it then leads in the panel's first line, the rest in each door's
 * card.
 */
export function DoorPanel({
  group,
  hidden,
  onMerge,
  onKeepAll,
  keptAll,
  looking = null,
  onLookAt,
  onLocate,
  pair = null,
  place,
  centre,
  notesOf,
}: DoorPanelProps) {
  const { lang } = useSession();
  const d = doorTexts[lang];
  const merged = mergedInto(group, hidden);
  const [hovered, setHovered] = useState<number | null>(null);
  const mergedAt = group.doors.findIndex(
    (door) => merged !== null && sameObject(door.object, merged),
  );
  const shown = hovered ?? Math.max(mergedAt, 0);
  const plan = group.plans[shown];
  const kept = group.doors[shown];
  const modOf = (object: ObjectRef) =>
    short(
      group.doors.find((candidate) => sameObject(candidate.object, object))?.plugin ??
        object.plugin,
    );
  const nameOf = (object: ObjectRef) => d.doorOf(modOf(object));
  return (
    <div className="door-panel" data-doors={group.doors.length}>
      <div className="door-head">
        <span className="door-intro">
          {d.intro(group.doors.length)}
          {group.flicker && <> {d.flicker}</>}
        </span>
        {plan !== undefined && kept !== undefined && (
          <span className="door-outcome">
            <strong>{d.ifKept(nameOf(kept.object))}</strong> {outcome(d, group, plan, kept)}
          </span>
        )}
        <span className="door-head-buttons">
          <button type="button" className="small" aria-pressed={keptAll} onClick={onKeepAll}>
            {keptAll ? d.keptAll : d.keepAll}
          </button>
          {pair !== null && (
            <button type="button" className="small quiet" onClick={pair.onOpen}>
              {d.pairLink(short(pair.first), short(pair.second))}
            </button>
          )}
        </span>
      </div>
      {group.apart === true && <p className="door-apart">{d.apart}</p>}
      {group.doors.every((door) => hidden(door.object)) && (
        <p className="door-apart door-lost">{d.wayLost}</p>
      )}
      <div className="door-cards">
        {group.doors.map((door, index) => {
          const itsPlan = group.plans[index];
          return (
            <DoorCard
              key={`${door.object.plugin}:${door.object.localId}`}
              door={door}
              first={index === 0}
              apart={group.apart === true}
              merged={merged !== null && sameObject(merged, door.object)}
              gone={merged !== null && !sameObject(merged, door.object)}
              looking={looking !== null && sameObject(looking, door.object)}
              onHover={(hovering) => setHovered(hovering ? index : null)}
              onLook={
                onLookAt === undefined
                  ? undefined
                  : () =>
                      onLookAt(
                        looking !== null && sameObject(looking, door.object) ? null : door.object,
                      )
              }
              onLocate={onLocate === undefined ? undefined : () => onLocate(door)}
              onMerge={() => {
                if (itsPlan !== undefined) {
                  onMerge(itsPlan);
                }
              }}
              notes={
                <>
                  {itsPlan !== undefined && (
                    <PlanNotes
                      group={group}
                      plan={itsPlan}
                      checked={merged !== null && sameObject(merged, door.object)}
                      nameOf={nameOf}
                      modOf={modOf}
                      place={place}
                      centre={centre}
                    />
                  )}
                  {notesOf?.(door)}
                </>
              }
            />
          );
        })}
      </div>
    </div>
  );
}

/** What else keeping one door changes, and what hiding the others does to the ways. */
function PlanNotes({
  group,
  plan,
  checked,
  nameOf,
  modOf,
  place,
  centre,
}: {
  group: DoorGroup;
  plan: DoorMerge;
  /** Whether the spot is merged into this plan's door: the ways to it are then checked. */
  checked: boolean;
  nameOf: (object: ObjectRef) => string;
  modOf: (object: ObjectRef) => string;
  place: string | null;
  centre: [number, number] | null;
}) {
  const { lang } = useSession();
  const d = doorTexts[lang];
  const n = spotNotesTexts[lang];
  const full = useNotesInFull();
  const changes: { key: string; text: string; lost?: boolean }[] = [];
  if (group.apart !== true) {
    for (const object of plan.hide) {
      const door = group.doors.find((candidate) => sameObject(candidate.object, object));
      if (door !== undefined && (door.walkable || door.references > 0)) {
        changes.push({
          key: `users:${object.plugin}:${object.localId}`,
          text: d.usersMove(nameOf(object), nameOf(plan.keep)),
        });
      }
    }
  }
  for (const change of plan.changes) {
    if (change.type !== "teleport" || change.from === undefined || change.from === null) {
      continue;
    }
    const from = change.from;
    const across = group.doors.find((door) => sameObject(door.object, from));
    const there =
      across?.leadsTo !== null && across?.leadsTo !== undefined
        ? placeName(across.leadsTo.location)
        : change.door.plugin;
    const arrival = change.arrival !== null && change.arrival !== undefined;
    changes.push({
      key: `${change.door.plugin}:${change.door.localId}`,
      text: `${d.otherSide(there, modOf(plan.keep))}${arrival ? ` ${d.arrivalMoves}` : ""}`,
    });
  }
  const navmeshDoors = plan.changes.filter((change) => change.type === "navmesh-door").length;
  if (navmeshDoors > 0) {
    changes.push({ key: "paths", text: d.paths(navmeshDoors, nameOf(plan.keep)) });
  }
  for (const lost of plan.lost) {
    changes.push({
      key: `lost:${placeName(lost.location)}`,
      lost: true,
      text: d.lost(
        placeName(lost.location),
        nameOf(
          group.doors.find(
            (door) =>
              door.leadsTo !== null &&
              door.leadsTo !== undefined &&
              placeName(door.leadsTo.location) === placeName(lost.location),
          )?.object ?? plan.keep,
        ),
      ),
    });
  }
  const lost = changes.filter((change) => change.lost).length;
  return (
    <>
      {changes.length > 0 && (
        <Note
          tone={lost > 0 ? "bad" : "info"}
          summary={lost > 0 ? n.doorLost(lost) : n.doorChanges(changes.length)}
          hint={changes.map((change) => change.text).join("\n")}
        >
          <div className="door-plan">
            <ul>
              {changes.map((change) => (
                <li key={change.key} className={change.lost ? "door-lost" : undefined}>
                  {change.text}
                </li>
              ))}
            </ul>
            {plan.changes.some((change) => change.type === "teleport") && (
              <p className="aside">{d.viaMoves}</p>
            )}
            {navmeshDoors > 0 && <p className="aside">{d.viaPaths}</p>}
          </div>
        </Note>
      )}
      {full && changes.length === 0 && <p className="aside">{d.nothingElse}</p>}
      <DoorReach place={place} centre={centre} hide={plan.hide} />
      {checked && group.apart !== true && (
        <KeptDoorReach
          place={place}
          centre={centre}
          plan={plan}
          name={nameOf(plan.keep)}
          anyWalked={group.doors.some((door) => door.walkable)}
        />
      )}
    </>
  );
}

/**
 * Where the kept door leads once the others are gone, and whether the door
 * there leads back: its own way, or the one it takes over.
 */
function outcome(d: DoorTexts, group: DoorGroup, plan: DoorMerge, kept: StackedDoor): string {
  const taken = plan.changes.find(
    (change) =>
      change.type === "teleport" &&
      (change.from === undefined || change.from === null) &&
      sameObject(change.door, plan.keep),
  );
  const destination =
    kept.leadsTo ??
    (taken === undefined
      ? null
      : (group.doors.find(
          (door) =>
            door.leadsTo?.door !== undefined &&
            door.leadsTo.door !== null &&
            sameObject(door.leadsTo.door, taken.to),
        )?.leadsTo ?? null));
  if (destination === null) {
    return d.outcomeOpensOnly;
  }
  const there = destination.door ?? null;
  const back =
    (kept.leadsBack && taken === undefined) ||
    plan.changes.some(
      (change) =>
        change.type === "teleport" &&
        sameObject(change.to, plan.keep) &&
        there !== null &&
        sameObject(change.door, there),
    );
  return back
    ? d.outcomeLeadsBack(placeName(destination.location))
    : d.outcomeLeads(placeName(destination.location));
}

/** What hiding the doors does to the ways characters take there (ADR 0059). */
function DoorReach(props: {
  place: string | null;
  centre: [number, number] | null;
  hide: ObjectRef[];
}) {
  return <ReachNote reach={useReach(props.place, props.centre, props.hide)} />;
}

/** The answers of the engine about kept doors, by what was asked. */
const doorsAsked = new Map<string, Promise<DoorReachFound | null>>();

/**
 * Whether characters reach the kept door of `plan` and walk through it on
 * the paths as the patch leaves them (ADR 0088). `undefined` while asking,
 * `null` if the setup cannot be read.
 */
function useDoorReach(
  place: string | null,
  centre: [number, number] | null,
  plan: DoorMerge,
): DoorReachFound | null | undefined {
  const { api } = useSession();
  const key =
    place === null
      ? null
      : JSON.stringify([
          place,
          centre?.map(Math.round) ?? null,
          plan.keep,
          plan.hide,
          plan.changes,
        ]);
  const [found, setFound] = useState<{ key: string; reach: DoorReachFound | null } | null>(null);
  // biome-ignore lint/correctness/useExhaustiveDependencies: the plan is compared by `key`.
  useEffect(() => {
    if (key === null || place === null) {
      return;
    }
    let outdated = false;
    let answer = doorsAsked.get(key);
    if (answer === undefined) {
      answer = api.doorReach(
        place,
        centre?.[0] ?? null,
        centre?.[1] ?? null,
        plan.hide,
        plan.keep,
        plan.changes,
      );
      doorsAsked.set(key, answer);
      answer.catch(() => doorsAsked.delete(key));
    }
    answer
      .then((reach) => {
        if (!outdated) {
          setFound({ key, reach });
        }
      })
      .catch(() => {
        if (!outdated) {
          setFound({ key, reach: null });
        }
      });
    return () => {
      outdated = true;
    };
  }, [api, key]);
  return key !== null && found !== null && found.key === key ? found.reach : undefined;
}

/** Says whether characters reach the kept door and walk through it. */
function KeptDoorReach({
  place,
  centre,
  plan,
  name,
  anyWalked,
}: {
  place: string | null;
  centre: [number, number] | null;
  plan: DoorMerge;
  name: string;
  /** Whether characters walked through any door of the group before. */
  anyWalked: boolean;
}) {
  const { lang } = useSession();
  const d = doorTexts[lang];
  const reach = useDoorReach(place, centre, plan);
  if (reach === undefined || reach === null || !reach.known) {
    return null;
  }
  if (!reach.reached && !anyWalked) {
    return (
      <Note tone="info" summary={d.reachOtherShort} hint={d.reachNoneAll}>
        <p className="door-reach">{d.reachNoneAll}</p>
      </Note>
    );
  }
  const ok = reach.reached && reach.through;
  const text = ok ? d.reachOk(name) : reach.reached ? d.reachNotThrough(name) : d.reachNone(name);
  return (
    <Note tone={ok ? "ok" : "bad"} summary={ok ? d.reachOkShort : d.reachBadShort} hint={text}>
      <p className="door-reach" data-ok={ok}>
        {text}
      </p>
    </Note>
  );
}

/** A record's number in its plugin, as xEdit shows it without the load order's byte. */
const hex = (object: ObjectRef) => object.localId.toString(16).toUpperCase().padStart(6, "0");

interface DoorCardProps {
  door: StackedDoor;
  /** Whether it is the first door, the one others are measured from. */
  first: boolean;
  /** Whether the doors lead to places apart, so keeping one is no merge. */
  apart: boolean;
  merged: boolean;
  gone: boolean;
  /** Whether it is looked at alone in the 3D view. */
  looking: boolean;
  onHover: (hovering: boolean) => void;
  /** Looks at it alone in the 3D view, or at all again. */
  onLook: (() => void) | undefined;
  /** Makes it blink in the 3D view, the camera left where it is. */
  onLocate: (() => void) | undefined;
  onMerge: () => void;
  /** What keeping it, or hiding it, means besides: badges, or in full. */
  notes: ReactNode;
}

/** One door: which, where it leads, whether characters walk through it. */
function DoorCard({
  door,
  first,
  apart,
  merged,
  gone,
  looking,
  onHover,
  onLook,
  onLocate,
  onMerge,
  notes,
}: DoorCardProps) {
  const { lang } = useSession();
  const d = doorTexts[lang];
  const n = spotNotesTexts[lang];
  const full = useNotesInFull();
  // In full, a sentence a line; otherwise short words, the sentence in the tooltip.
  const check = (ok: boolean, yes: string, no: string, words: [string, string]) => (
    <li data-ok={ok} title={full ? undefined : ok ? yes : no}>
      {full ? (ok ? yes : no) : ok ? words[0] : words[1]}
    </li>
  );
  const leadsTo = door.leadsTo ?? null;
  const changedBy = door.plugin.toLowerCase() !== door.object.plugin.toLowerCase();
  return (
    <div
      className="door-card"
      data-merged={merged || undefined}
      data-gone={gone || undefined}
      data-looking={looking || undefined}
    >
      <div className="door-card-head">
        <strong title={door.object.plugin}>{short(door.object.plugin)}</strong>
        <code className="door-id">{hex(door.object)}</code>
        {door.suggested && (
          <span className="suggested-mark" title={d.suggestedHint}>
            {d.suggested}
          </span>
        )}
        {onLocate !== undefined && (
          <button
            type="button"
            className="small quiet door-locate"
            title={d.locate}
            aria-label={d.locate}
            onClick={onLocate}
          >
            <svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true">
              <circle cx="12" cy="12" r="7" />
              <path d="M12 2v4M12 18v4M2 12h4M18 12h4" />
            </svg>
          </button>
        )}
        {onLook !== undefined && (
          <button
            type="button"
            className="small quiet door-eye"
            aria-pressed={looking}
            title={d.lookAlone}
            aria-label={d.lookAlone}
            onClick={onLook}
          >
            <svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true">
              <path d="M2 12s3.6-6 10-6 10 6 10 6-3.6 6-10 6S2 12 2 12z" />
              <circle cx="12" cy="12" r="3" />
            </svg>
          </button>
        )}
      </div>
      <div className="aside door-what">
        {door.name}
        {changedBy && <> · {d.changedBy(short(door.plugin))}</>}
        {door.front && <> · {d.front}</>}
        {!first && door.offset > 0 && <> · {d.away(centimetres(door.offset))}</>}
      </div>
      <p className="door-leads">
        {leadsTo !== null
          ? d.leadsTo(placeName(leadsTo.location))
          : door.broken
            ? d.broken
            : d.opensOnly}
      </p>
      <ul className="door-checks" data-full={full || undefined}>
        {leadsTo !== null &&
          check(door.leadsBack, d.leadsBack, d.leadsElsewhere, [n.leadsBack, n.leadsElsewhere])}
        {check(door.walkable, d.walkable, `${d.notWalkable}. ${d.keptNotOnPaths}`, [
          n.walkable,
          n.notWalkable,
        ])}
        {leadsTo !== null &&
          door.walkable &&
          check(door.otherSideWalkable, d.otherSideWalkable, d.otherSideNotWalkable, [
            n.otherSide,
            n.otherSideNot,
          ])}
      </ul>
      <NoteRow>{notes}</NoteRow>
      <button
        type="button"
        className="work-choice"
        aria-pressed={merged}
        onClick={onMerge}
        onMouseEnter={() => onHover(true)}
        onMouseLeave={() => onHover(false)}
        onFocus={() => onHover(true)}
        onBlur={() => onHover(false)}
      >
        <span className="work-choice-text">
          <strong className="work-choice-title">
            {apart ? (merged ? d.keptOnly : d.keepOnly) : merged ? d.merged : d.mergeInto}
          </strong>
        </span>
      </button>
    </div>
  );
}

/**
 * The moves of `moves` with the teleports of the merged doors added: for
 * each spot of doors whose choice keeps one of them, the doors its plan
 * leads elsewhere (ADR 0067). A door that is moved as well keeps its move.
 * They go into the plugin of moves (ADR 0060).
 */
export function withDoorMoves(
  moves: ObjectMove[],
  choices: PatchChoices,
  spots: Map<string, MapSpot[]>,
): ObjectMove[] {
  const all = new Map(moves.map((each) => [matchKey(each.object), each]));
  const seen = new Set<string>();
  for (const [finding, list] of spots) {
    for (const spot of list) {
      const key = spotKey(spot);
      if (spot.doors === null || spot.doors === undefined || key === null || seen.has(key)) {
        continue;
      }
      seen.add(key);
      const plan = mergedBy(spot.doors, decisionAt(choices, finding, key, spot.parts ?? []));
      for (const change of plan?.changes ?? []) {
        if (change.type !== "teleport") {
          continue;
        }
        const before = all.get(matchKey(change.door));
        all.set(matchKey(change.door), {
          object: change.door,
          cell: before?.cell ?? change.cell ?? null,
          from: before?.from ?? change.at,
          offset: before?.offset ?? [0, 0, 0],
          turn: before?.turn ?? 0,
          teleport: {
            door: change.to,
            arrival: change.arrival ?? null,
            facing: change.facing ?? null,
          },
        });
      }
    }
  }
  return [...all.values()].sort((a, b) => matchKey(a.object).localeCompare(matchKey(b.object)));
}

/**
 * The door triangles of the merged doors to re-attach to the kept ones,
 * place by place (ADR 0088): for each spot of doors whose choice keeps one
 * of them, the paths its plan points at the kept door. They go into the
 * plugin of paths with the repairs (ADR 0058, 0070).
 */
export function doorRelinksOf(choices: PatchChoices, spots: Map<string, MapSpot[]>): RepairPlace[] {
  const places = new Map<string, RepairPlace>();
  const seen = new Set<string>();
  for (const [finding, list] of spots) {
    for (const spot of list) {
      const key = spotKey(spot);
      if (spot.doors === null || spot.doors === undefined || key === null || seen.has(key)) {
        continue;
      }
      seen.add(key);
      const plan = mergedBy(spot.doors, decisionAt(choices, finding, key, spot.parts ?? []));
      const doors: DoorRelink[] = (plan?.changes ?? []).flatMap((change) =>
        change.type === "navmesh-door"
          ? [
              {
                navmesh: change.navmesh,
                triangle: change.triangle,
                from: change.from,
                to: change.to,
              },
            ]
          : [],
      );
      if (plan === null || plan === undefined || doors.length === 0) {
        continue;
      }
      const kept = spot.doors.doors.find((door) => sameObject(door.object, plan.keep));
      const location = spot.location;
      const place = location.type === "interior" ? location.cell : location.world;
      const at = kept?.position;
      const centre: [number, number] | null =
        location.type === "interior" || at === undefined ? null : [at[0], at[1]];
      if (location.type !== "interior" && centre === null) {
        continue;
      }
      const where = `${place}|${centre?.join(",") ?? ""}`;
      const known = places.get(where) ?? { place, centre, navmeshes: [], every: false, doors: [] };
      known.doors = [...(known.doors ?? []), ...doors];
      places.set(where, known);
    }
  }
  return [...places.values()];
}

/** How the worklist names a spot's choice that keeps one door, if it is one. */
export function keptLabel(decided: string | null | undefined): string | null {
  const kept = keptBy(decided);
  return kept === null
    ? null
    : `${short(kept.plugin)} ${kept.localId.toString(16).toUpperCase().padStart(6, "0")}`;
}
