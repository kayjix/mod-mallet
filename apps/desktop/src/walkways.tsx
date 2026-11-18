// Paths characters walk on over objects a decision would hide (ADR 0050,
// ADR 0051). Hiding an object takes away what characters stand on, but not
// the paths they follow over it. Before a keep choice, an item toggle or
// the bulk hides objects, the window asks which of them carry paths the
// game uses, once per place and object, and warns.

import { useEffect, useState } from "react";
import { useSession } from "./Actions";
import type { Api } from "./api";
import { objectKey } from "./area-layer";
import type { ObjectRef } from "./generated/rendered-report";
import type { Lang } from "./i18n";
import "./walkways.css";

export interface WalkTexts {
  /** How many pieces of path, as the finding counts them. */
  pieces: (count: number) => string;
  /** The warning on one object or a few. */
  warning: (count: number) => string;
  /** What keeping a side does to the other side's parts characters walk on. */
  keepWarning: (loser: string, names: string, count: number, triangles: number) => string;
  hideAnyway: (loser: string, count: number) => string;
  /** On the collisions page: hiding one side of a spot. */
  spotWarning: (names: string, mod: string, triangles: number) => string;
  pickedWalked: (name: string, triangles: number) => string;
  legend: string;
  doubtfulTitle: (count: number) => string;
}

const en: WalkTexts = {
  pieces: (count) => (count === 1 ? "1 piece of path" : `${count} pieces of path`),
  warning: (count) =>
    `Characters walk on this (${en.pieces(count)}). Hiding it leaves them walking on thin air or stuck.`,
  keepWarning: (loser, names, count, triangles) =>
    `Characters walk on ${names} of ${loser} (${en.pieces(triangles)}). Hidden, they would walk on thin air or get stuck, so ${count === 1 ? "it stays" : "these stay"}. Keep this side or both, or hide ${count === 1 ? "it" : "them"} anyway if nobody needs to walk there.`,
  hideAnyway: (loser, count) =>
    count === 1
      ? `Hide the part of ${loser} anyway, though characters walk on it`
      : `Hide the ${count} parts of ${loser} anyway, though characters walk on some`,
  spotWarning: (names, mod, triangles) =>
    `Characters walk on ${names} of ${mod} (${en.pieces(triangles)}). Hiding it leaves them walking on thin air or stuck. Keep it or both, or hide it knowingly if nobody needs to walk there.`,
  pickedWalked: (name, triangles) =>
    `${name} is hidden now, though characters walk on it (${en.pieces(triangles)}): they walk on thin air there or get stuck. A second click shows it again.`,
  legend: "characters walk on it",
  doubtfulTitle: (count) =>
    count === 1
      ? "Left open because characters walk on what would go:"
      : `${count} places left open because characters walk on what would go:`,
};

const de: WalkTexts = {
  pieces: (count) => (count === 1 ? "1 Wegstück" : `${count} Wegstücke`),
  warning: (count) =>
    `Hier laufen Figuren (${de.pieces(count)}). Versteckt, laufen sie durch die Luft oder bleiben hängen.`,
  keepWarning: (loser, names, count, triangles) =>
    `Figuren laufen über ${names} von ${loser} (${de.pieces(triangles)}). Wäre ${count === 1 ? "es" : "das"} versteckt, liefen sie durch die Luft oder blieben hängen, darum ${count === 1 ? "bleibt es" : "bleiben diese Teile"}. Behalte diese Seite oder beide, oder blende ${count === 1 ? "es" : "sie"} trotzdem aus, wenn dort niemand laufen muss.`,
  hideAnyway: (loser, count) =>
    count === 1
      ? `Das Teil von ${loser} trotzdem ausblenden, obwohl Figuren darauf laufen`
      : `Die ${count} Teile von ${loser} trotzdem ausblenden, obwohl Figuren auf manchen laufen`,
  spotWarning: (names, mod, triangles) =>
    `Figuren laufen über ${names} von ${mod} (${de.pieces(triangles)}). Versteckt, laufen sie durch die Luft oder bleiben hängen. Behalte es oder beide, oder blende es bewusst aus, wenn dort niemand laufen muss.`,
  pickedWalked: (name, triangles) =>
    `${name} ist jetzt ausgeblendet, obwohl Figuren darauf laufen (${de.pieces(triangles)}): Dort laufen sie durch die Luft oder bleiben hängen. Ein zweiter Klick zeigt es wieder.`,
  legend: "darauf laufen Figuren",
  doubtfulTitle: (count) =>
    count === 1
      ? "Offen gelassen, weil Figuren auf dem laufen, was weg würde:"
      : `${count} Bereiche offen gelassen, weil Figuren auf dem laufen, was weg würde:`,
};

export const walkTexts: Record<Lang, WalkTexts> = { en, de };

/** The walkway texts in the window's language. */
export function useWalkTexts(): WalkTexts {
  return walkTexts[useSession().lang];
}

/** An object to ask about, with where it stands if known (game units). */
export interface WalkQuery {
  id: ObjectRef;
  position?: [number, number, number] | null;
}

/**
 * How far apart the points asked from lie, in game units: the engine looks
 * 4096 units around a point, so each object lies well within its square's.
 */
const SQUARE = 4096;

/** What is known of each object, asked once per place and object. */
const asked = new Map<string, Promise<number>>();

/**
 * How many pieces of path the game uses lie on each of `objects`, by
 * `objectKey`, in the worldspace `place` (outside, `centre` for objects
 * without a position) or the interior cell `place` (`centre` null). Objects
 * without paths, or not found, count 0. Asked once per place and object for
 * the window's life, in as few calls as the objects' squares allow.
 */
export async function walkedOn(
  api: Api,
  place: string,
  centre: [number, number] | null,
  objects: WalkQuery[],
): Promise<Map<string, number>> {
  const keyOf = (object: WalkQuery) => `${place}|${objectKey(object.id.plugin, object.id.localId)}`;
  // The objects not asked about yet, by the square their point lies in.
  const squares = new Map<string, { at: [number, number] | null; objects: WalkQuery[] }>();
  for (const object of objects) {
    if (asked.has(keyOf(object))) {
      continue;
    }
    const point: [number, number] | null =
      centre === null
        ? null
        : object.position != null
          ? [object.position[0], object.position[1]]
          : centre;
    const square: [number, number] | null =
      point === null ? null : [Math.floor(point[0] / SQUARE), Math.floor(point[1] / SQUARE)];
    const name = square === null ? "inside" : square.join(",");
    const entry = squares.get(name) ?? {
      at: square === null ? null : [(square[0] + 0.5) * SQUARE, (square[1] + 0.5) * SQUARE],
      objects: [],
    };
    if (!entry.objects.some((other) => keyOf(other) === keyOf(object))) {
      entry.objects.push(object);
    }
    squares.set(name, entry);
  }
  for (const { at, objects: these } of squares.values()) {
    const answer = api
      .hiddenWalkways(
        place,
        at?.[0] ?? null,
        at?.[1] ?? null,
        these.map(({ id }) => ({ plugin: id.plugin, localId: id.localId })),
      )
      .then((found) => {
        const triangles = new Map<string, number>();
        for (const walkway of found?.walkways ?? []) {
          // A version another plugin replaces does no harm.
          if (walkway.used) {
            const key = objectKey(walkway.object.plugin, walkway.object.localId);
            triangles.set(key, (triangles.get(key) ?? 0) + walkway.triangles);
          }
        }
        return triangles;
      });
    for (const object of these) {
      const key = keyOf(object);
      asked.set(
        key,
        answer
          .then((triangles) => triangles.get(objectKey(object.id.plugin, object.id.localId)) ?? 0)
          .catch(() => {
            // Not known: asked again next time.
            asked.delete(key);
            return 0;
          }),
      );
    }
  }
  const result = new Map<string, number>();
  for (const object of objects) {
    const triangles = await (asked.get(keyOf(object)) ?? Promise.resolve(0));
    if (triangles > 0) {
      result.set(objectKey(object.id.plugin, object.id.localId), triangles);
    }
  }
  return result;
}

/**
 * The pieces of path on `objects` by key, for the parts named by `key`;
 * `null` while asking.
 */
export function useWalkedOn(
  key: string,
  place: string | null,
  centre: [number, number] | null,
  objects: WalkQuery[],
): Map<string, number> | null {
  const { api } = useSession();
  const [found, setFound] = useState<{ key: string; walked: Map<string, number> } | null>(null);
  const objectsNow = objects;
  // biome-ignore lint/correctness/useExhaustiveDependencies: the objects are compared by `key`.
  useEffect(() => {
    if (place === null || objectsNow.length === 0) {
      setFound({ key, walked: new Map() });
      return;
    }
    let outdated = false;
    walkedOn(api, place, centre, objectsNow)
      .then((walked) => {
        if (!outdated) {
          setFound({ key, walked });
        }
      })
      .catch(() => {
        if (!outdated) {
          setFound({ key, walked: new Map() });
        }
      });
    return () => {
      outdated = true;
    };
  }, [api, key, place]);
  return found !== null && found.key === key ? found.walked : null;
}

/** The outline of the path sign, in a box 24 units wide: a dashed path. */
export const PATH_TRACK = "M8 19 C8 13 16 12 16 5";

/** The sign's colour: yellow (Okabe–Ito), with a dark rim and path. */
export const PATH_COLOUR = "#f0e442";

/** A path sign: a dashed winding path on a yellow disc, told apart by its shape. */
export function PathIcon() {
  return (
    <svg className="path-icon" viewBox="0 0 24 24" aria-hidden="true">
      <circle cx="12" cy="12" r="10.5" />
      <path d={PATH_TRACK} />
    </svg>
  );
}

/** The path sign beside one object's name, with how many pieces of path lie on it. */
export function PathMark({ triangles }: { triangles: number }) {
  const w = useWalkTexts();
  const text = w.warning(triangles);
  return (
    <span className="path-mark" title={text}>
      <PathIcon />
      {triangles}
      <span className="visually-hidden">{text}</span>
    </span>
  );
}

/** Draws the path sign centred on `x`, `y` in a 2D context, `size` pixels wide. */
export function drawPathSign(
  context: CanvasRenderingContext2D,
  x: number,
  y: number,
  size: number,
) {
  const scale = size / 24;
  context.save();
  context.translate(x - size / 2, y - size / 2);
  context.scale(scale, scale);
  context.beginPath();
  context.arc(12, 12, 10.5, 0, Math.PI * 2);
  context.fillStyle = PATH_COLOUR;
  context.fill();
  context.lineWidth = 2;
  context.strokeStyle = "#1b1b1b";
  context.stroke();
  context.setLineDash([3, 2.5]);
  context.lineWidth = 2.4;
  context.lineCap = "round";
  context.stroke(new Path2D(PATH_TRACK));
  context.restore();
}
