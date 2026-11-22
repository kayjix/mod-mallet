// The places where characters do something (ADR 0069): furniture to sit,
// lean, sleep or work at and idle markers, many of them invisible in the
// game. The 3D views draw an icon over each by what characters do there;
// the object card says what it is, who uses it and whether characters can
// reach it, where it stands or where a move puts it. Where hiding takes
// away what such places stand on, they can be moved onto the ground
// characters still reach, or hidden too.

import { useEffect, useMemo, useState, useSyncExternalStore } from "react";
import { useSession } from "./Actions";
import { actionPlaceTexts } from "./action-places-texts";
import type { Api } from "./api";
import type { ActionPlace, AreaObject, AreaObjects } from "./generated/area-objects";
import type { Anchor, ObjectAnchors } from "./generated/object-anchors";
import type { ObjectRef } from "./generated/rendered-report";
import type { SpotReach, SpotsReach } from "./generated/spots-reach";
import type { Destination } from "./generated/walkway-reach";
import { type Move, moveOf, withMoves } from "./moves";
import { useMovesTexts } from "./moves-texts";
import { useAnchors } from "./object-actions";
import { chooseObject, hiddenKeysOf, matchKey, revealObjects } from "./patch";
import "./action-places.css";

/** What characters do at a place. */
export type ActionKind = ActionPlace["kind"];

// ---------------------------------------------------------------------
// The places of the areas read, to find one by its object.

const known = new Map<string, { place: ActionPlace; world: string }>();

/** Keeps the places of an area read, so that a card can tell an object is one. */
export function rememberActions(area: AreaObjects) {
  for (const place of area.actions ?? []) {
    known.set(matchKey(place.object), { place, world: area.world });
  }
}

/** The place where characters do something that `object` is, if it is one. */
export function actionPlaceOf(object: ObjectRef): ActionPlace | null {
  return known.get(matchKey(object))?.place ?? null;
}

/** The object of the 3D views a place is, drawn or not, for its card. */
export function asAreaObject(place: ActionPlace, area: AreaObjects | null): AreaObject {
  const key = matchKey(place.object);
  const drawn = area?.objects.find((object) => matchKey(object.id) === key);
  if (drawn !== undefined) {
    return drawn;
  }
  return {
    id: place.object,
    plugin: place.plugin,
    model: "",
    name: place.name ?? place.base ?? null,
    position: place.position,
    rotation: place.rotation,
    scale: 1,
    anchored: false,
  };
}

// ---------------------------------------------------------------------
// Whether the icons are shown, the same in every view.

const SHOWN_KEY = "mallet.actionIcons";
let shownNow = (() => {
  try {
    return window.localStorage.getItem(SHOWN_KEY) !== "off";
  } catch {
    return true;
  }
})();
const listeners = new Set<() => void>();

function setShown(value: boolean) {
  shownNow = value;
  try {
    window.localStorage.setItem(SHOWN_KEY, value ? "on" : "off");
  } catch {
    // Kept for this run only.
  }
  for (const listener of listeners) {
    listener();
  }
}

/** Whether the views draw the icons of places where characters do something. */
export function useActionIconsShown(): boolean {
  return useSyncExternalStore(
    (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    () => shownNow,
  );
}

/** The switch of the icons, for a view's controls. */
export function ActionIconsToggle({ className = "relief-control" }: { className?: string }) {
  const a = actionPlaceTexts[useSession().lang];
  const shown = useActionIconsShown();
  return (
    <label className={className} title={a.toggleHint}>
      <input type="checkbox" checked={shown} onChange={(event) => setShown(event.target.checked)} />
      <span className="action-toggle-icons" aria-hidden="true">
        {ICON.sit}
        {ICON.lean}
      </span>{" "}
      {a.toggle}
    </label>
  );
}

// ---------------------------------------------------------------------
// The icons.

/** The sign of each kind, as drawn over the views and in the card. */
export const ICON: Record<ActionKind, string> = {
  sit: "🪑",
  lean: "🧍",
  sleep: "🛏",
  "chop-wood": "🪓",
  mine: "⛏",
  smith: "⚒",
  craft: "⚗",
  cook: "🍲",
  pray: "🙏",
  work: "🧹",
  stand: "📍",
  other: "◆",
};

/** The colour behind each kind: rest blue, work orange, the rest grey (Okabe–Ito). */
function colourOf(kind: ActionKind): string {
  switch (kind) {
    case "sit":
    case "lean":
    case "sleep":
      return "#0072b2";
    case "chop-wood":
    case "mine":
    case "smith":
    case "craft":
    case "cook":
    case "work":
      return "#e69f00";
    case "pray":
      return "#cc79a7";
    default:
      return "#6b7280";
  }
}

/** One icon drawn, for picking. */
export interface ActionHit {
  place: ActionPlace;
  x: number;
  y: number;
}

/** How close to an icon a click has to land, in pixels. */
const HIT = 13;

/** Where a place stands with its move, and the way it faces, in radians clockwise from north. */
export function placedWith(place: ActionPlace, move: Move | null) {
  const [x, y, z] = place.position;
  return {
    at: [x + (move?.dx ?? 0), y + (move?.dy ?? 0), z + (move?.dz ?? 0)] as [number, number, number],
    facing: (place.rotation[2] ?? 0) + ((move?.rz ?? 0) * Math.PI) / 180,
  };
}

/**
 * Draws an icon over each place: by what characters do there, with a tick
 * the way they face, faint where the patch hides it, ringed where it is
 * picked; a moved place where its move puts it, with a dashed line from
 * where it stood. `toScreen` gives where a point in game units shows.
 * Returns where each icon was drawn.
 */
export function paintActionPlaces(
  context: CanvasRenderingContext2D,
  toScreen: (point: [number, number, number]) => { x: number; y: number } | null,
  places: ActionPlace[],
  options: {
    moves: (object: ObjectRef) => Move | null;
    hidden: (object: ObjectRef) => boolean;
    picked: string | null;
    /** Whether a place is left out, as in a view of only one side. */
    skip?: (object: ObjectRef) => boolean;
  },
): ActionHit[] {
  const hits: ActionHit[] = [];
  context.save();
  context.textAlign = "center";
  context.textBaseline = "middle";
  for (const place of places) {
    if (options.skip?.(place.object)) {
      continue;
    }
    const move = options.moves(place.object);
    const { at, facing } = placedWith(place, move);
    // The icon floats a little above the spot.
    const shown = toScreen([at[0], at[1], at[2] + 40]);
    if (shown === null) {
      continue;
    }
    const hidden = options.hidden(place.object);
    const picked = options.picked === matchKey(place.object);
    if (move !== null) {
      const from = toScreen([place.position[0], place.position[1], place.position[2] + 40]);
      if (from !== null) {
        context.save();
        context.setLineDash([4, 4]);
        context.strokeStyle = "rgb(255 255 255 / 70%)";
        context.lineWidth = 1.5;
        context.beginPath();
        context.moveTo(from.x, from.y);
        context.lineTo(shown.x, shown.y);
        context.stroke();
        context.restore();
      }
    }
    const ahead = toScreen([
      at[0] + Math.sin(facing) * 60,
      at[1] + Math.cos(facing) * 60,
      at[2] + 40,
    ]);
    context.globalAlpha = hidden ? 0.35 : place.drawn ? 0.8 : 1;
    if (ahead !== null) {
      const dx = ahead.x - shown.x;
      const dy = ahead.y - shown.y;
      const length = Math.hypot(dx, dy);
      if (length > 1) {
        context.strokeStyle = "#ffffff";
        context.lineWidth = 2;
        context.beginPath();
        context.moveTo(shown.x + (dx / length) * 10, shown.y + (dy / length) * 10);
        context.lineTo(shown.x + (dx / length) * 17, shown.y + (dy / length) * 17);
        context.stroke();
      }
    }
    const radius = place.drawn ? 9 : 11;
    context.beginPath();
    context.arc(shown.x, shown.y, radius, 0, Math.PI * 2);
    context.fillStyle = colourOf(place.kind);
    context.fill();
    context.lineWidth = picked ? 3 : 1.5;
    context.strokeStyle = picked ? "#ffffff" : "#1b1b1b";
    context.stroke();
    context.font = `${radius + 2}px "Segoe UI Emoji", "Apple Color Emoji", sans-serif`;
    context.fillStyle = "#ffffff";
    context.fillText(ICON[place.kind], shown.x, shown.y + 1);
    if (hidden) {
      context.strokeStyle = "#d55e00";
      context.lineWidth = 2.5;
      context.beginPath();
      context.moveTo(shown.x - radius, shown.y - radius);
      context.lineTo(shown.x + radius, shown.y + radius);
      context.stroke();
    }
    context.globalAlpha = 1;
    hits.push({ place, x: shown.x, y: shown.y });
  }
  context.restore();
  return hits;
}

/** The place whose icon lies under a point, in the same pixels the icons were drawn in. */
export function actionAt(hits: ActionHit[], x: number, y: number): ActionPlace | null {
  let best: { place: ActionPlace; distance: number } | null = null;
  for (const hit of hits) {
    const distance = Math.hypot(hit.x - x, hit.y - y);
    if (distance <= HIT && (best === null || distance < best.distance)) {
      best = { place: hit.place, distance };
    }
  }
  return best?.place ?? null;
}

// ---------------------------------------------------------------------
// Whether characters reach spots.

const askedReach = new Map<string, Promise<SpotsReach | null>>();

function reachOf(
  api: Api,
  place: string,
  centre: [number, number] | null,
  spots: { object: ObjectRef; at: [number, number, number] }[],
  hidden: ObjectRef[],
): Promise<SpotsReach | null> {
  const key = `${place}|${centre?.map(Math.round).join(",") ?? "inside"}|${spots
    .map((spot) => `${matchKey(spot.object)}@${spot.at.join(",")}`)
    .join(";")}|${hidden.map(matchKey).sort().join(";")}`;
  let found = askedReach.get(key);
  if (found === undefined) {
    found = api.spotsReach(place, centre?.[0] ?? null, centre?.[1] ?? null, spots, hidden);
    askedReach.set(key, found);
    found.catch(() => askedReach.delete(key));
  }
  return found;
}

/**
 * Whether characters reach `spots` in the worldspace `place` near `centre`
 * or the interior cell `place` (`centre` null), with the paths over
 * `hidden` cut out: `undefined` while asking, `null` if it cannot be told.
 */
export function useSpotsReach(
  place: string | null,
  centre: [number, number] | null,
  spots: { object: ObjectRef; at: [number, number, number] }[],
  hidden: ObjectRef[],
): SpotReach[] | null | undefined {
  const { api } = useSession();
  const key =
    place === null || spots.length === 0
      ? null
      : `${place}|${spots.map((spot) => `${matchKey(spot.object)}@${spot.at.join(",")}`).join(";")}|${hidden.map(matchKey).sort().join(";")}`;
  const [found, setFound] = useState<{ key: string; spots: SpotReach[] | null } | null>(null);
  // biome-ignore lint/correctness/useExhaustiveDependencies: the spots are compared by `key`.
  useEffect(() => {
    if (key === null || place === null) {
      return;
    }
    let outdated = false;
    reachOf(api, place, centre, spots, hidden)
      .then((reach) => {
        if (!outdated) {
          setFound({ key, spots: reach?.spots ?? null });
        }
      })
      .catch(() => {
        if (!outdated) {
          setFound({ key, spots: null });
        }
      });
    return () => {
      outdated = true;
    };
  }, [api, key]);
  return key !== null && found !== null && found.key === key ? found.spots : undefined;
}

/** A point in whole game units. */
const whole = (point: readonly number[]): [number, number, number] => [
  Math.round(point[0] ?? 0),
  Math.round(point[1] ?? 0),
  Math.round(point[2] ?? 0),
];

// ---------------------------------------------------------------------
// The card.

/** Who uses a place, from its ties (ADR 0046), in words. */
function usersOf(anchors: ObjectAnchors, a: (typeof actionPlaceTexts)["en"]): string[] {
  const lines: string[] = [];
  const name = (anchor: Anchor) =>
    anchor.other?.name ??
    `${anchor.other?.plugin ?? ""} ${anchor.other?.localId.toString(16) ?? ""}`;
  for (const anchor of anchors.reasons) {
    if (anchor.other?.state === "gone") {
      continue;
    }
    switch (anchor.kind) {
      case "linked-ref":
        lines.push(
          anchor.other?.recordType === "ACHR"
            ? a.linkedCharacter(name(anchor))
            : a.linkedPlace(name(anchor)),
        );
        break;
      case "package":
        lines.push(a.routine(name(anchor)));
        break;
      case "quest-alias":
        lines.push(a.quest(name(anchor)));
        break;
      case "location-role":
        lines.push(a.locationRole(name(anchor), anchor.keyword ?? null));
        break;
      case "script":
        lines.push(a.script(name(anchor)));
        break;
      default:
        break;
    }
  }
  return [...new Set(lines)];
}

/** How far around a place hidden objects count for its paths, in game units (about 30 m). */
const HIDDEN_AROUND = 2048;

/**
 * What a place where characters do something is, who uses it, whether
 * characters reach it where it stands or where its move puts it, and what
 * hiding it does. Nothing for other objects.
 */
export function ActionPlaceInfo({
  object,
  place: worldOrCell,
  interior = false,
  around = [],
}: {
  object: AreaObject;
  /** The worldspace or interior cell it is in. */
  place: string | null;
  interior?: boolean;
  /** The other objects of the view, of which those the patch hides count. */
  around?: AreaObject[];
}) {
  const { lang, spots, patchChoices } = useSession();
  const a = actionPlaceTexts[lang];
  const action = actionPlaceOf(object.id);
  const anchors = useAnchors(action === null ? null : worldOrCell, object.id);
  const move = moveOf(patchChoices, object.id);
  const hiddenKeys = useMemo(() => hiddenKeysOf(spots, patchChoices), [spots, patchChoices]);
  const at = action === null ? null : placedWith(action, move).at;
  const hidden = useMemo(
    () =>
      at === null
        ? []
        : around
            .filter(
              (other) =>
                hiddenKeys.has(matchKey(other.id)) &&
                matchKey(other.id) !== matchKey(object.id) &&
                Math.hypot(other.position[0] - at[0], other.position[1] - at[1]) <= HIDDEN_AROUND,
            )
            .slice(0, 40)
            .map((other) => other.id),
    [around, hiddenKeys, at?.[0], at?.[1], object.id],
  );
  const reach = useSpotsReach(
    action === null ? null : worldOrCell,
    at === null || interior ? null : [at[0], at[1]],
    at === null ? [] : [{ object: object.id, at: whole(at) }],
    hidden,
  );
  if (action === null) {
    return null;
  }
  const moved = move !== null;
  const spot = reach?.[0];
  const reachLine =
    reach === undefined
      ? a.checking
      : spot === undefined || reach === null || spot.state === "unknown"
        ? a.unknown
        : spot.state === "reached"
          ? moved
            ? a.reachedMoved
            : a.reached
          : spot.state === "cut-off"
            ? moved
              ? a.cutOffMoved
              : a.cutOff
            : moved
              ? a.offPathsMoved(spot.metresOff ?? null)
              : a.offPaths(spot.metresOff ?? null);
  const users = anchors == null ? [] : usersOf(anchors, a);
  const what = action.drawn ? a.seen : action.idleMarker ? a.idleMarker : a.marker;
  return (
    <div className="action-place-info">
      <p className="action-place-kind">
        <span className="action-place-icon" data-kind={action.kind} aria-hidden="true">
          {ICON[action.kind]}
        </span>
        <strong>{a.kind[action.kind]}</strong> <span className="aside">· {what}</span>
      </p>
      {action.base && <p className="aside">{a.base(action.base)}</p>}
      <p className="action-place-head">{a.usersTitle}</p>
      {anchors === undefined ? (
        <p className="aside">{a.asking}</p>
      ) : (
        <ul className="action-place-users">
          {(users.length > 0 ? users : [a.foundByKind]).map((line) => (
            <li key={line}>{line}</li>
          ))}
        </ul>
      )}
      <p className="action-place-head">{a.reachTitle}</p>
      <p
        className="action-place-reach"
        data-state={reach === undefined ? "checking" : (spot?.state ?? "unknown")}
      >
        {reachLine}
      </p>
      {hidden.length > 0 && <p className="aside">{a.withHidden(hidden.length)}</p>}
      <p className="aside action-place-hide">{a.hideNote(action.kind)}</p>
    </div>
  );
}

// ---------------------------------------------------------------------
// Following what hiding does to places that stood on it.

/** The kinds of destination of ADR 0059 that are places where characters do something. */
const isActionDestination = (destination: Destination) =>
  destination.kind === "furniture" || destination.kind === "idle-marker";

/**
 * Under the note of what hiding breaks (ADR 0059): the places where
 * characters do something that stood on what is hidden, with moving each
 * onto the nearest ground characters still reach, or hiding it too.
 */
export function FollowHiding({
  place,
  centre,
  hidden,
  destinations,
}: {
  /** The worldspace or interior cell. */
  place: string;
  /** Near where, outside; null inside. */
  centre: [number, number] | null;
  /** The objects being hidden. */
  hidden: ObjectRef[];
  /** The destinations that lose what they stood on. */
  destinations: Destination[];
}) {
  const { lang, patchChoices, setPatchChoices, spots } = useSession();
  const a = actionPlaceTexts[lang];
  const m = useMovesTexts();
  const gone = useMemo(() => {
    const seen = new Set<string>();
    return destinations.filter(
      (destination) =>
        isActionDestination(destination) &&
        !seen.has(matchKey(destination.object)) &&
        seen.add(matchKey(destination.object)),
    );
  }, [destinations]);
  const reach = useSpotsReach(
    gone.length === 0 ? null : place,
    centre,
    gone.map((destination) => ({ object: destination.object, at: destination.position })),
    hidden,
  );
  const anchors = useAnchorsOfAll(gone.length === 0 ? null : place, gone);
  const hiddenKeys = useMemo(() => hiddenKeysOf(spots, patchChoices), [spots, patchChoices]);
  if (gone.length === 0) {
    return null;
  }
  const interior = centre === null;
  const groundOf = (destination: Destination) =>
    reach?.find((spot) => matchKey(spot.object) === matchKey(destination.object))?.ground ?? null;
  const locked = (destination: Destination) =>
    anchors?.find((found) => matchKey(found.object) === matchKey(destination.object))?.verdict ===
    "locked";
  const moveTo = (destination: Destination, ground: [number, number, number]): Move => ({
    dx: ground[0] - destination.position[0],
    dy: ground[1] - destination.position[1],
    dz: ground[2] - destination.position[2],
    rz: moveOf(patchChoices, destination.object)?.rz ?? 0,
    at: destination.position,
    cell: interior ? place : null,
  });
  const movable = gone.filter(
    (destination) =>
      !locked(destination) &&
      groundOf(destination) !== null &&
      moveOf(patchChoices, destination.object) === null &&
      !hiddenKeys.has(matchKey(destination.object)),
  );
  const hideable = gone.filter(
    (destination) => !locked(destination) && !hiddenKeys.has(matchKey(destination.object)),
  );
  const moveAll = () =>
    setPatchChoices(
      withMoves(
        patchChoices,
        movable.flatMap((destination) => {
          const ground = groundOf(destination);
          return ground === null
            ? []
            : [{ object: destination.object, move: moveTo(destination, ground) }];
        }),
      ),
    );
  const hideAll = () => {
    let next = patchChoices;
    for (const destination of hideable) {
      next = chooseObject(
        withMoves(next, [{ object: destination.object, move: null }]),
        destination.object,
        true,
      );
    }
    setPatchChoices(next);
  };
  return (
    <div className="follow-hiding">
      <p>
        <strong>{a.followTitle(gone.length)}</strong> <span className="aside">{a.followHint}</span>
      </p>
      <ul>
        {gone.map((destination) => {
          const ground = groundOf(destination);
          const move = moveOf(patchChoices, destination.object);
          const isHidden = hiddenKeys.has(matchKey(destination.object));
          const kind = actionPlaceOf(destination.object)?.kind;
          const state = locked(destination)
            ? a.lockedByQuest
            : isHidden
              ? a.hiddenToo
              : move !== null
                ? m.moved(move.dx, move.dy, move.dz, move.rz)
                : reach === undefined
                  ? a.finding
                  : ground === null
                    ? a.noGround
                    : null;
          return (
            <li key={matchKey(destination.object)}>
              {kind !== undefined && (
                <span className="action-place-icon" data-kind={kind} aria-hidden="true">
                  {ICON[kind]}
                </span>
              )}{" "}
              {destination.name ??
                `${destination.object.plugin} ${destination.object.localId.toString(16)}`}
              {state !== null && <span className="aside"> · {state}</span>}
              {!locked(destination) && !isHidden && move === null && (
                <span className="relief-object-buttons">
                  <button
                    type="button"
                    className="small"
                    disabled={ground === null}
                    onClick={() =>
                      ground !== null &&
                      setPatchChoices(
                        withMoves(patchChoices, [
                          { object: destination.object, move: moveTo(destination, ground) },
                        ]),
                      )
                    }
                  >
                    {a.moveToGround}
                  </button>
                  <button
                    type="button"
                    className="small"
                    onClick={() =>
                      setPatchChoices(
                        chooseObject(
                          withMoves(patchChoices, [{ object: destination.object, move: null }]),
                          destination.object,
                          true,
                        ),
                      )
                    }
                  >
                    {a.hideToo}
                  </button>
                </span>
              )}
              {!locked(destination) && (isHidden || move !== null) && (
                <span className="relief-object-buttons">
                  <button
                    type="button"
                    className="small quiet"
                    onClick={() =>
                      setPatchChoices(
                        revealObjects(
                          withMoves(patchChoices, [{ object: destination.object, move: null }]),
                          spots,
                          [destination.object],
                        ),
                      )
                    }
                  >
                    {m.back}
                  </button>
                </span>
              )}
            </li>
          );
        })}
      </ul>
      {(movable.length > 1 || hideable.length > 1) && (
        <span className="relief-object-buttons">
          {movable.length > 1 && (
            <button type="button" className="small primary" onClick={moveAll}>
              {a.moveAllToGround(movable.length)}
            </button>
          )}
          {hideable.length > 1 && (
            <button type="button" className="small" onClick={hideAll}>
              {a.hideAllToo(hideable.length)}
            </button>
          )}
        </span>
      )}
      <p className="aside">{a.facingNote}</p>
    </div>
  );
}

/** What ties each of `destinations` (ADR 0046): undefined while asking, null where it cannot be told. */
function useAnchorsOfAll(
  place: string | null,
  destinations: Destination[],
): ObjectAnchors[] | null | undefined {
  const { api } = useSession();
  const key = destinations.map((destination) => matchKey(destination.object)).join(",");
  const [found, setFound] = useState<{ key: string; anchors: ObjectAnchors[] | null } | null>(null);
  // biome-ignore lint/correctness/useExhaustiveDependencies: the key stands for the objects.
  useEffect(() => {
    if (place === null || key === "") {
      return;
    }
    let outdated = false;
    api
      .objectAnchors(
        place,
        destinations.map((destination) => destination.object),
      )
      .then((anchors) => !outdated && setFound({ key, anchors }))
      .catch(() => !outdated && setFound({ key, anchors: null }));
    return () => {
      outdated = true;
    };
  }, [api, place, key]);
  return found !== null && found.key === key ? found.anchors : undefined;
}
