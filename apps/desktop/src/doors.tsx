// Doors that mods stack on top of each other (ADR 0067): the doors of a
// spot side by side, each with where it leads, and what merging into one
// of them means. The collisions' page and the guided review show it; the
// choice itself is the patch's, as for any other spot.

import { useState } from "react";
import "./doors.css";
import { useSession } from "./Actions";
import { doorTexts } from "./door-texts";
import type {
  DoorGroup,
  DoorMerge,
  MapSpot,
  ObjectRef,
  RecordChange,
  StackedDoor,
} from "./generated/rendered-report";
import { type Move, moveOf, withMoves } from "./moves";
import { chooseObject, chooseSpot, matchKey, type PatchChoices } from "./patch";
import { ReachNote, useReach } from "./reach";

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
  /** Looks at the place without a plugin's doors, without deciding. */
  onPreview?: (plugin: string) => void;
  /** The plugins looked at without. */
  previewed?: Set<string>;
  /** The worldspace or interior cell the doors are in, to ask where characters still go. */
  place: string | null;
  /** Outside: where the doors stand; null inside. */
  centre: [number, number] | null;
}

/** The doors of a spot side by side, and what keeping one of them means. */
export function DoorPanel({
  group,
  hidden,
  onMerge,
  onKeepAll,
  keptAll,
  onPreview,
  previewed,
  place,
  centre,
}: DoorPanelProps) {
  const { lang } = useSession();
  const d = doorTexts[lang];
  const merged = mergedInto(group, hidden);
  const [looked, setLooked] = useState<number | null>(null);
  const mergedAt = group.doors.findIndex(
    (door) => merged !== null && sameObject(door.object, merged),
  );
  const shown = looked ?? Math.max(mergedAt, 0);
  const plan = group.plans[shown];
  const kept = group.doors[shown];
  const modOf = (object: ObjectRef) =>
    short(
      group.doors.find((candidate) => sameObject(candidate.object, object))?.plugin ??
        object.plugin,
    );
  const nameOf = (object: ObjectRef) => {
    const door = group.doors.find((candidate) => sameObject(candidate.object, object));
    return d.doorOf(short(door?.plugin ?? object.plugin));
  };
  return (
    <div className="door-panel">
      <p className="door-intro">
        {d.intro(group.doors.length)}
        {group.flicker && <> {d.flicker}</>}
      </p>
      <div className="door-cards">
        {group.doors.map((door, index) => (
          <DoorCard
            key={`${door.object.plugin}:${door.object.localId}`}
            door={door}
            first={index === 0}
            merged={merged !== null && sameObject(merged, door.object)}
            gone={merged !== null && !sameObject(merged, door.object)}
            onLook={(looking) => setLooked(looking ? index : null)}
            onPreview={onPreview === undefined ? undefined : () => onPreview(door.object.plugin)}
            previewed={previewed?.has(door.object.plugin) ?? false}
            onMerge={() => {
              const chosen = group.plans[index];
              if (chosen !== undefined) {
                onMerge(chosen);
              }
            }}
          />
        ))}
      </div>
      <div className="door-plan">
        {plan !== undefined && kept !== undefined && (
          <>
            <strong>{d.ifKept(d.doorOf(short(kept.plugin)))}</strong>
            <ul>
              {plan.hide
                .filter((object) => {
                  const door = group.doors.find((candidate) =>
                    sameObject(candidate.object, object),
                  );
                  return door !== undefined && (door.walkable || door.references > 0);
                })
                .map((object) => (
                  <li key={`users:${object.plugin}:${object.localId}`}>
                    {d.usersMove(nameOf(object), nameOf(plan.keep))}
                  </li>
                ))}
              {plan.changes.map((change, index) => {
                if (change.type === "navmesh-door") {
                  return null;
                }
                const key = `${change.door.plugin}:${change.door.localId}:${index}`;
                if (change.from === undefined || change.from === null) {
                  const leads = kept.leadsTo ?? null;
                  const destination = group.doors.find(
                    (door) =>
                      door.leadsTo?.door != null && sameObject(door.leadsTo.door, change.to),
                  )?.leadsTo;
                  const place = destination ?? leads;
                  return place === null || place === undefined ? null : (
                    <li key={key}>{d.takesWay(modOf(plan.keep), placeName(place.location))}</li>
                  );
                }
                const from = change.from;
                const across = group.doors.find((door) => sameObject(door.object, from));
                const there =
                  across?.leadsTo !== null && across?.leadsTo !== undefined
                    ? placeName(across.leadsTo.location)
                    : change.door.plugin;
                return (
                  <li key={key}>
                    {d.otherSide(there, modOf(plan.keep))}
                    {change.arrival !== null && change.arrival !== undefined && (
                      <> {d.arrivalMoves}</>
                    )}
                  </li>
                );
              })}
              {plan.changes.some((change) => change.type === "navmesh-door") && (
                <li>
                  {d.paths(
                    plan.changes.filter((change) => change.type === "navmesh-door").length,
                    nameOf(plan.keep),
                  )}
                </li>
              )}
              {plan.lost.map((place) => (
                <li key={`lost:${placeName(place.location)}`} className="door-lost">
                  {d.lost(
                    placeName(place.location),
                    nameOf(
                      group.doors.find(
                        (door) =>
                          door.leadsTo !== null &&
                          door.leadsTo !== undefined &&
                          placeName(door.leadsTo.location) === placeName(place.location),
                      )?.object ?? plan.keep,
                    ),
                  )}
                </li>
              ))}
            </ul>
            <p className="door-reached" data-ok={kept.walkable}>
              {kept.walkable ? d.keptReached : d.keptNotOnPaths}
            </p>
            <DoorReach place={place} centre={centre} hide={plan.hide} />
            {plan.changes.length === 0 && <p className="aside">{d.nothingElse}</p>}
            {plan.changes.some((change) => change.type === "teleport") && (
              <p className="aside">{d.viaMoves}</p>
            )}
            {plan.changes.some((change) => change.type === "navmesh-door") && (
              <p className="aside">{d.pathsLater}</p>
            )}
          </>
        )}
        <button type="button" className="small" aria-pressed={keptAll} onClick={onKeepAll}>
          {keptAll ? d.keptAll : d.keepAll}
        </button>
      </div>
    </div>
  );
}

/** What hiding the doors does to the ways characters take there (ADR 0059). */
function DoorReach(props: {
  place: string | null;
  centre: [number, number] | null;
  hide: ObjectRef[];
}) {
  return <ReachNote reach={useReach(props.place, props.centre, props.hide)} />;
}

interface DoorCardProps {
  door: StackedDoor;
  /** Whether it is the first door, the one others are measured from. */
  first: boolean;
  merged: boolean;
  gone: boolean;
  onLook: (looking: boolean) => void;
  onMerge: () => void;
  onPreview: (() => void) | undefined;
  previewed: boolean;
}

/** One door: whose, where it leads, whether characters walk through it. */
function DoorCard({
  door,
  first,
  merged,
  gone,
  onLook,
  onMerge,
  onPreview,
  previewed,
}: DoorCardProps) {
  const { lang } = useSession();
  const d = doorTexts[lang];
  const check = (ok: boolean, yes: string, no: string) => <li data-ok={ok}>{ok ? yes : no}</li>;
  return (
    <div className="door-card" data-merged={merged || undefined} data-gone={gone || undefined}>
      <div className="door-card-head">
        <strong title={door.plugin}>{short(door.plugin)}</strong>
        {onPreview !== undefined && (
          <button
            type="button"
            className="small quiet door-eye"
            aria-pressed={previewed}
            title={d.lookWithout}
            aria-label={d.lookWithout}
            onClick={onPreview}
          >
            {previewed ? "◌" : "◉"}
          </button>
        )}
        {door.suggested && (
          <span className="suggested-mark" title={d.suggestedHint}>
            {d.suggested}
          </span>
        )}
      </div>
      <div className="aside">
        {door.name}
        {door.front && <> · {d.front}</>}
        {!first && door.offset > 0 && <> · {d.away(centimetres(door.offset))}</>}
      </div>
      <p className="door-leads">
        {door.leadsTo !== null && door.leadsTo !== undefined
          ? d.leadsTo(placeName(door.leadsTo.location))
          : d.opensOnly}
      </p>
      <ul className="door-checks">
        {door.leadsTo !== null &&
          door.leadsTo !== undefined &&
          check(door.leadsBack, d.leadsBack, d.leadsElsewhere)}
        {check(door.walkable, d.walkable, d.notWalkable)}
        {door.leadsTo !== null &&
          door.leadsTo !== undefined &&
          door.walkable &&
          check(door.otherSideWalkable, d.otherSideWalkable, d.otherSideNotWalkable)}
      </ul>
      <button
        type="button"
        className="work-choice"
        aria-pressed={merged}
        onClick={onMerge}
        onMouseEnter={() => onLook(true)}
        onMouseLeave={() => onLook(false)}
        onFocus={() => onLook(true)}
        onBlur={() => onLook(false)}
      >
        <span className="work-choice-text">
          <strong className="work-choice-title">{merged ? d.merged : d.mergeInto}</strong>
        </span>
      </button>
    </div>
  );
}

/**
 * The patch's choices with a group merged into the door `plan` keeps, or,
 * with `plan` null, with every door kept. The other doors are hidden as
 * the spot's own choice where they are one plugin's, else each on its
 * own; `hiddenObjects` are the objects hidden on their own so far, and the
 * kept door is shown again if it is among them. The doors on the other
 * side are led to the kept door through the plugin of moves (ADR 0060).
 */
export function mergeChoices(
  choices: PatchChoices,
  key: string,
  group: DoorGroup,
  plan: DoorMerge | null,
  hiddenObjects: ObjectRef[],
): PatchChoices {
  let next = withDoorTargets(choices, group, plan);
  if (plan === null) {
    return chooseSpot(next, key, null);
  }
  if (hiddenObjects.some((object) => matchKey(object) === matchKey(plan.keep))) {
    next = chooseObject(next, plan.keep, false);
  }
  const plugins = new Set(plan.hide.map((object) => object.plugin.toLowerCase()));
  const [only] = [...plugins];
  if (plugins.size === 1 && only !== undefined && !plugins.has(plan.keep.plugin.toLowerCase())) {
    const plugin = plan.hide[0]?.plugin ?? only;
    return chooseSpot(next, key, plugin);
  }
  next = chooseSpot(next, key, null);
  for (const object of plan.hide) {
    next = chooseObject(next, object, true);
  }
  return next;
}

/**
 * The choices with the teleports of `plan` among the moves, and those any
 * other plan of the group would set taken out. A move of the same door
 * keeps its offset and turn.
 */
function withDoorTargets(
  choices: PatchChoices,
  group: DoorGroup,
  plan: DoorMerge | null,
): PatchChoices {
  const teleports = (changes: RecordChange[]) =>
    changes.flatMap((change) => (change.type === "teleport" ? [change] : []));
  const cleared = teleports(group.plans.flatMap((each) => each.changes)).flatMap((change) => {
    const before = moveOf(choices, change.door);
    return before?.door ? [{ object: change.door, move: { ...before, door: null } }] : [];
  });
  const next = withMoves(choices, cleared);
  if (plan === null) {
    return next;
  }
  return withMoves(
    next,
    teleports(plan.changes).map((change) => {
      const before: Move = moveOf(next, change.door) ?? {
        dx: 0,
        dy: 0,
        dz: 0,
        rz: 0,
        at: change.at,
        cell: change.cell ?? null,
      };
      return {
        object: change.door,
        move: {
          ...before,
          door: { door: change.to, arrival: change.arrival ?? null, facing: change.facing ?? null },
        },
      };
    }),
  );
}

/** Whether a door is hidden: by the spot's choice or on its own. */
export function doorHidden(
  object: ObjectRef,
  decided: string | null | undefined,
  hiddenObjects: ObjectRef[],
): boolean {
  return (
    (decided !== null &&
      decided !== undefined &&
      decided.toLowerCase() === object.plugin.toLowerCase()) ||
    hiddenObjects.some((hidden) => matchKey(hidden) === matchKey(object))
  );
}
