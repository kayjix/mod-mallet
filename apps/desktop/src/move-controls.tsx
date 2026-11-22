// Moving a placed object, or a group the magic wand took, a little to one
// side instead of hiding it (ADR 0060): nudge controls in the object card
// of the 3D views, with the collision check redone where the move puts it,
// what ties the object to its place, and whether characters walk on it. A
// line in the collisions' spot panel, and the plugin of moves on the patch
// page.

import { useEffect, useMemo, useState } from "react";
import { ActionResult, useAction, useSession } from "./Actions";
import type { Api } from "./api";
import { isNature, overlapDepth, ownerOf, type PartTools } from "./area-layer";
import { doorTexts } from "./door-texts";
import { withDoorMoves } from "./doors";
import type { AreaObject } from "./generated/area-objects";
import type { CreatedMoves } from "./generated/created-moves";
import type { ObjectAnchors } from "./generated/object-anchors";
import type { ObjectRef, SpotPart } from "./generated/rendered-report";
import {
  CM_PER_UNIT,
  groupMoves,
  type Move,
  moveOf,
  movesOf,
  objectMovesOf,
  withMoves,
} from "./moves";
import { useMovesTexts } from "./moves-texts";
import { AnchorReasons } from "./object-actions";
import { chooseSpot, decisionAt, keyOf, matchKey, type PatchChoices } from "./patch";
import { Note } from "./spot-notes";
import { spotNotesTexts } from "./spot-notes-texts";
import { walkedOn } from "./walkways";
import "./moves.css";

/** The steps a nudge can take, in game units: about 10 cm, 50 cm, 1 m and 5 m. */
const STEPS = [7, 35, 70, 350];
/** The steps a turn can take, in degrees. */
const TURNS = [1, 5, 15, 90];
/** A step's length for people, rounded to ten centimetres. */
function stepLabel(units: number): string {
  const cm = Math.round((units * CM_PER_UNIT) / 10) * 10;
  return cm >= 100 ? `${cm / 100} m` : `${cm} cm`;
}

/** How far around a moved object others are checked, in game units (about 30 m). */
const AROUND = 2048;

/** What is known of the ties of several objects, asked once per place and set. */
const asked = new Map<string, Promise<ObjectAnchors[] | null>>();

function anchorsOf(api: Api, place: string, objects: ObjectRef[]) {
  const key = `${place}|${objects.map(matchKey).sort().join(",")}`;
  let found = asked.get(key);
  if (found === undefined) {
    found = api.objectAnchors(place, objects).catch(() => null);
    asked.set(key, found);
  }
  return found;
}

/** Why the game may rely on each of `objects`: undefined while asking, null where it cannot be told. */
function useAnchorsOf(place: string | null, objects: AreaObject[]) {
  const { api } = useSession();
  const [found, setFound] = useState<ObjectAnchors[] | null | undefined>(undefined);
  const ids = useMemo(() => objects.map((object) => object.id), [objects]);
  const key = ids.map(matchKey).join(",");
  // biome-ignore lint/correctness/useExhaustiveDependencies: the key stands for the objects.
  useEffect(() => {
    if (place === null) {
      setFound(null);
      return;
    }
    let outdated = false;
    setFound(undefined);
    anchorsOf(api, place, ids).then((result) => {
      if (!outdated) {
        setFound(result);
      }
    });
    return () => {
      outdated = true;
    };
  }, [api, place, key]);
  return found;
}

/** How many pieces of path the game uses lie on `objects`; null while asking. */
function useWalked(place: string | null, interior: boolean, objects: AreaObject[]) {
  const { api } = useSession();
  const [pieces, setPieces] = useState<number | null>(null);
  const key = objects.map((object) => matchKey(object.id)).join(",");
  // biome-ignore lint/correctness/useExhaustiveDependencies: the key stands for the objects.
  useEffect(() => {
    const first = objects[0];
    if (place === null || first === undefined) {
      setPieces(0);
      return;
    }
    let outdated = false;
    setPieces(null);
    walkedOn(
      api,
      place,
      interior ? null : [first.position[0], first.position[1]],
      objects.map((object) => ({ id: object.id, position: object.position })),
    )
      .then((found) => {
        if (!outdated) {
          setPieces([...found.values()].reduce((sum, count) => sum + count, 0));
        }
      })
      .catch(() => setPieces(0));
    return () => {
      outdated = true;
    };
  }, [api, place, interior, key]);
  return pieces;
}

/** An object's name for people. */
const nameOf = (object: AreaObject) =>
  object.name ??
  object.model
    .split(/[\\/]/)
    .pop()
    ?.replace(/\.nif$/i, "") ??
  object.model;

/**
 * What a move does to the collision: the deepest overlap with another
 * mod's object before and after, by the models' boxes.
 */
function collisionAfter(
  tools: PartTools | undefined,
  moved: AreaObject[],
  choices: PatchChoices,
  against: ObjectRef[] | undefined,
): { name: string; before: number; now: number } | "nothing" | null {
  // An invisible marker has no model to collide with (ADR 0069).
  const group = moved.filter((object) => object.model !== "");
  const layer = tools?.layer;
  const first = group[0];
  if (layer === undefined || first === undefined) {
    return null;
  }
  const inGroup = new Set(group.map((object) => matchKey(object.id)));
  const owner = ownerOf(first);
  const wanted = against === undefined ? null : new Set(against.map(matchKey));
  const others = layer
    .objectsNear(first.position, AROUND)
    .filter((other) =>
      wanted !== null
        ? wanted.has(matchKey(other.id))
        : !inGroup.has(matchKey(other.id)) && ownerOf(other) !== owner && !isNature(other.model),
    );
  let worst: { name: string; before: number; now: number } | null = null;
  for (const object of group) {
    const was = layer.boxOf(object, null);
    const is = layer.boxOf(object, moveOf(choices, object.id));
    if (was === null || is === null) {
      continue;
    }
    for (const other of others) {
      const box = layer.boxOf(other, moveOf(choices, other.id));
      if (box === null) {
        continue;
      }
      const before = overlapDepth(was, box);
      const now = overlapDepth(is, box);
      if (before <= 0 && now <= 0) {
        continue;
      }
      // What overlaps now counts first, then what overlapped before.
      const rank = (found: { before: number; now: number }) =>
        found.now > 0 ? 1e9 + found.now : found.before;
      const candidate = { name: nameOf(other), before, now };
      if (worst === null || rank(candidate) > rank(worst)) {
        worst = candidate;
      }
    }
  }
  return worst ?? "nothing";
}

/** The arrows, steps and turns of a move, and what it does. */
export function MoveControls({
  object,
  tools,
  place,
  interior = false,
  against,
  spotKey,
}: {
  object: AreaObject;
  tools?: PartTools;
  /** The worldspace or interior cell it is in. */
  place: string | null;
  /** Whether `place` is an interior cell. */
  interior?: boolean;
  /** The other mod's parts at the spot looked at, checked instead of everything around. */
  against?: ObjectRef[];
  /** The spot looked at: moving a part of it keeps both there. */
  spotKey?: { finding: string; key: string; parts: SpotPart[] } | null;
}) {
  const { patchChoices: choices, setPatchChoices } = useSession();
  const m = useMovesTexts();
  const [open, setOpen] = useState(false);
  const [together, setTogether] = useState(false);
  const [step, setStep] = useState(STEPS[0] ?? 7);
  const [turn, setTurn] = useState(TURNS[1] ?? 5);
  /** The moves of the group before each nudge, for undo. */
  const [history, setHistory] = useState<{ object: ObjectRef; move: Move | null }[][]>([]);
  const selection = tools?.selection ?? [object];
  const canTogether = selection.length > 1;
  const group = together && canTogether ? selection : [object];
  const anchors = useAnchorsOf(open ? place : null, group);
  const walked = useWalked(open ? place : null, interior, group);

  // Without an answer, the placed objects' own flags decide.
  const locked = group.filter((part, index) =>
    anchors === null || anchors === undefined
      ? part.anchored === true
      : anchors[index]?.verdict === "locked",
  );
  const careful = (anchors ?? []).filter((found) => found.verdict === "careful");
  const moved = group.filter((part) => moveOf(choices, part.id) !== null);
  const own = moveOf(choices, object.id);

  const nudge = (offset: [number, number, number], degrees: number) => {
    const before = group.map((part) => ({ object: part.id, move: moveOf(choices, part.id) }));
    // The group turns about its middle as it stands now.
    const at = group.map((part) => {
      const move = moveOf(choices, part.id);
      return [part.position[0] + (move?.dx ?? 0), part.position[1] + (move?.dy ?? 0)];
    });
    const pivot: [number, number] = [
      at.reduce((sum, point) => sum + (point[0] ?? 0), 0) / Math.max(1, at.length),
      at.reduce((sum, point) => sum + (point[1] ?? 0), 0) / Math.max(1, at.length),
    ];
    const changes = groupMoves(group, pivot, offset, degrees, interior ? place : null, (part) =>
      moveOf(choices, part.id),
    );
    let next = withMoves(choices, changes);
    // Moving a part of an open spot keeps both there.
    if (
      spotKey != null &&
      decisionAt(next, spotKey.finding, spotKey.key, spotKey.parts) === undefined
    ) {
      next = chooseSpot(next, spotKey.key, null);
    }
    setPatchChoices(next);
    setHistory([...history, before]);
  };
  const undo = () => {
    const last = history[history.length - 1];
    if (last !== undefined) {
      setPatchChoices(withMoves(choices, last));
      setHistory(history.slice(0, -1));
    }
  };
  const back = () => {
    setHistory([
      ...history,
      group.map((part) => ({ object: part.id, move: moveOf(choices, part.id) })),
    ]);
    setPatchChoices(
      withMoves(
        choices,
        group.map((part) => ({ object: part.id, move: null })),
      ),
    );
  };

  const collision = useMemo(
    () => (moved.length === 0 ? null : collisionAfter(tools, group, choices, against)),
    [moved.length, tools, group, choices, against],
  );

  if (!open) {
    return (
      <div className="move-controls">
        <span className="relief-object-buttons">
          <button type="button" className="small" title={m.moveHint} onClick={() => setOpen(true)}>
            {m.move}
          </button>
        </span>
        {own !== null && <p className="aside">{m.moved(own.dx, own.dy, own.dz, own.rz)}</p>}
      </div>
    );
  }

  const arrows: { label: string; text: string; offset: [number, number, number] }[] = [
    { label: m.north, text: "↑", offset: [0, step, 0] },
    { label: m.west, text: "←", offset: [-step, 0, 0] },
    { label: m.east, text: "→", offset: [step, 0, 0] },
    { label: m.south, text: "↓", offset: [0, -step, 0] },
  ];
  const stuck = locked.length > 0;
  const shownMove = moveOf(choices, (group[0] ?? object).id);
  return (
    <div className="move-controls" data-open>
      <div className="move-head">
        <strong>{together && canTogether ? m.moveTogether(group.length) : m.move}</strong>
        <button type="button" className="small quiet" onClick={() => setOpen(false)}>
          {m.close}
        </button>
      </div>
      {canTogether && (
        <label className="move-together">
          <input
            type="checkbox"
            checked={together}
            onChange={(event) => setTogether(event.target.checked)}
          />
          {m.moveTogether(selection.length)}
        </label>
      )}
      {stuck ? (
        <>
          <p className="aside relief-object-warning">
            {group.length === 1 ? m.locked : m.someLocked(locked.length)}
          </p>
          {group.length === 1 && anchors?.[0] != null && <AnchorReasons anchors={anchors[0]} />}
        </>
      ) : (
        <>
          {careful.length > 0 && (
            <>
              <p className="aside relief-object-warning">{m.careful}</p>
              {careful.map((found) => (
                <AnchorReasons key={matchKey(found.object)} anchors={found} />
              ))}
            </>
          )}
          {walked !== null && walked > 0 && (
            <p className="aside relief-object-warning move-walked">{m.walked(walked)}</p>
          )}
          <div className="move-pad">
            <fieldset className="move-arrows">
              {arrows.map((arrow) => (
                <button
                  key={arrow.label}
                  type="button"
                  className="small move-arrow"
                  data-direction={arrow.text}
                  title={arrow.label}
                  aria-label={arrow.label}
                  onClick={() => nudge(arrow.offset, 0)}
                >
                  {arrow.text}
                </button>
              ))}
            </fieldset>
            <fieldset className="move-column">
              <button
                type="button"
                className="small"
                title={m.up}
                aria-label={m.up}
                onClick={() => nudge([0, 0, step], 0)}
              >
                ⤒
              </button>
              <button
                type="button"
                className="small"
                title={m.down}
                aria-label={m.down}
                onClick={() => nudge([0, 0, -step], 0)}
              >
                ⤓
              </button>
            </fieldset>
            <fieldset className="move-column">
              <button
                type="button"
                className="small"
                title={m.turnLeft}
                aria-label={m.turnLeft}
                onClick={() => nudge([0, 0, 0], -turn)}
              >
                ⟲
              </button>
              <button
                type="button"
                className="small"
                title={m.turnRight}
                aria-label={m.turnRight}
                onClick={() => nudge([0, 0, 0], turn)}
              >
                ⟳
              </button>
            </fieldset>
          </div>
          <div className="move-steps">
            <label>
              {m.step}{" "}
              <select value={step} onChange={(event) => setStep(Number(event.target.value))}>
                {STEPS.map((units) => (
                  <option key={units} value={units}>
                    {stepLabel(units)}
                  </option>
                ))}
              </select>
            </label>
            <label>
              {m.turnStep}{" "}
              <select value={turn} onChange={(event) => setTurn(Number(event.target.value))}>
                {TURNS.map((degrees) => (
                  <option key={degrees} value={degrees}>
                    {degrees}°
                  </option>
                ))}
              </select>
            </label>
          </div>
        </>
      )}
      <p className="aside move-state">
        {shownMove === null
          ? m.notMoved
          : m.moved(shownMove.dx, shownMove.dy, shownMove.dz, shownMove.rz)}
      </p>
      {collision !== null && (
        <p
          className="move-collision"
          data-clear={collision === "nothing" || collision.now <= 0 || undefined}
        >
          {collision === "nothing"
            ? m.touchesNothing
            : collision.now <= 0
              ? m.noLongerTouches(collision.name)
              : collision.before > 0
                ? m.stillOverlaps(collision.name, Math.round(collision.now * CM_PER_UNIT))
                : m.nowOverlaps(collision.name, Math.round(collision.now * CM_PER_UNIT))}{" "}
          <span className="aside">{m.boxesNote}</span>
        </p>
      )}
      <span className="relief-object-buttons">
        <button type="button" className="small" disabled={history.length === 0} onClick={undo}>
          {m.undo}
        </button>
        <button type="button" className="small" disabled={moved.length === 0} onClick={back}>
          {m.back}
        </button>
      </span>
      <p className="aside parts-hint">{m.intoPlugin}</p>
    </div>
  );
}

/** In a collision's spot panel: which of its parts are moved, and how to move one. */
export function SpotMoves({ parts }: { parts: SpotPart[] }) {
  const { patchChoices: choices, setPatchChoices, lang } = useSession();
  const m = useMovesTexts();
  const moved = parts.filter((part) => moveOf(choices, part.object) !== null);
  if (moved.length === 0) {
    return (
      <Note tone="info" summary={spotNotesTexts[lang].moveInstead} hint={m.spotHint}>
        <p className="aside move-spot">{m.spotHint}</p>
      </Note>
    );
  }
  return (
    <p className="aside move-spot" data-moved>
      {m.spotMoved(moved.length)}{" "}
      <button
        type="button"
        className="small"
        onClick={() =>
          setPatchChoices(
            withMoves(
              choices,
              moved.map((part) => ({ object: part.object, move: null })),
            ),
          )
        }
      >
        {m.back}
      </button>
    </p>
  );
}

/** On the patch page: the moved objects, and the plugin that carries them into the game. */
/**
 * The moves, and the doors that merged doors lead elsewhere. On the patch
 * page (ADR 0070) it lists those of one patch, given as `inPatch`, whose
 * archive brings them: it then writes and installs nothing of its own.
 */
export function MovesPanel({
  inPatch,
}: {
  inPatch?: { choices: PatchChoices; note: string };
} = {}) {
  const session = useSession();
  const { t, api, lang, setPatchChoices, spots } = session;
  const everything = session.patchChoices;
  const choices = inPatch?.choices ?? everything;
  const d = doorTexts[lang];
  const hex = (object: ObjectRef) => object.localId.toString(16).padStart(6, "0").toUpperCase();
  const m = useMovesTexts();
  const action = useAction();
  const moves = movesOf(choices);
  // With the doors that merged doors lead elsewhere (ADR 0067), which the
  // door decisions say.
  const all = withDoorMoves(objectMovesOf(choices), choices, spots);
  const signature = JSON.stringify(all);
  const [written, setWritten] = useState<{ result: CreatedMoves; signature: string } | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  if (all.length === 0 && written === null) {
    return null;
  }
  const upToDate = written !== null && written.signature === signature;
  const create = async () => {
    setBusy(true);
    setError(null);
    try {
      const result = await api.createMoves({ moves: all });
      setWritten({ result, signature });
    } catch (failure: unknown) {
      setError(String(failure));
    } finally {
      setBusy(false);
    }
  };
  const reason = (refused: CreatedMoves["refused"][number]) =>
    refused.reason.type === "unknown-field"
      ? m.reasons["unknown-field"](refused.reason.field)
      : m.reasons[refused.reason.type];
  return (
    <section className="moves-panel">
      <h3>{m.title}</h3>
      <div className="patch-bar">
        <strong>{m.summary(all.length)}</strong>
        <span className="spacer" />
        {inPatch === undefined && (
          <>
            <button type="button" disabled={all.length === 0 || busy} onClick={create}>
              {busy ? m.creating : m.create}
            </button>
            <button
              type="button"
              className="primary"
              disabled={!upToDate || action.busy || (written?.result.moved ?? 0) === 0}
              title={m.installHint}
              onClick={() => action.run({ type: "move-objects", moves: { moves: all } })}
            >
              {m.install}
            </button>
          </>
        )}
        <button
          type="button"
          disabled={moves.size === 0}
          onClick={() =>
            setPatchChoices(
              withMoves(
                everything,
                [...moves.values()].map(({ object }) => ({ object, move: null })),
              ),
            )
          }
        >
          {m.putAllBack}
        </button>
      </div>
      <ul className="moves-list">
        {all.map(({ object, offset, turn, teleport }) => (
          <li key={keyOf(object)}>
            <span>
              {object.plugin}{" "}
              <code>{object.localId.toString(16).padStart(6, "0").toUpperCase()}</code>
            </span>{" "}
            <span className="aside">
              {offset.every((value) => value === 0) && turn % 360 === 0
                ? ""
                : m.moved(offset[0], offset[1], offset[2], turn)}
              {teleport ? ` ${d.ledTo(`${teleport.door.plugin} ${hex(teleport.door)}`)}` : ""}
            </span>
          </li>
        ))}
      </ul>
      {written !== null && (
        <div className={upToDate ? "patch-file" : "patch-file stale"}>
          {written.result.moved === 0 ? (
            <span>{m.nothingWritten}</span>
          ) : (
            <span>
              {upToDate
                ? m.written(written.result.moved, written.result.masters.join(", "))
                : m.outdated}{" "}
              {upToDate && written.result.light && m.writtenLight}{" "}
              {upToDate && <code>{written.result.path}</code>}
            </span>
          )}
          <button type="button" onClick={() => api.openFolder("patches")}>
            {t.openFolder}
          </button>
        </div>
      )}
      {written !== null && written.result.refused.length > 0 && (
        <div className="moves-refused">
          <p>{m.refusedTitle(written.result.refused.length)}</p>
          <ul>
            {written.result.refused.map((refused) => (
              <li key={keyOf(refused.object)}>
                {refused.object.plugin}{" "}
                <code>{refused.object.localId.toString(16).padStart(6, "0").toUpperCase()}</code>:{" "}
                {reason(refused)}
              </li>
            ))}
          </ul>
        </div>
      )}
      {error !== null && <p className="failure">{error}</p>}
      <ActionResult state={action} />
      {inPatch !== undefined && <p className="aside">{inPatch.note}</p>}
      <p className="aside">{m.limits}</p>
    </section>
  );
}
