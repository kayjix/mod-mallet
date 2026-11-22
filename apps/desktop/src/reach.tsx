// Whether characters still reach where they go once objects are hidden and
// the paths over them cut out (ADR 0059): the sentences that say what
// breaks, and the ways drawn in the 3D view, grey before and green after,
// with a red mark where no way is left.

import { useEffect, useMemo, useState } from "react";
import { useSession } from "./Actions";
import { FollowHiding } from "./action-places";
import type { ObjectRef } from "./generated/rendered-report";
import type { ReachChange, Route, WalkwayReach } from "./generated/walkway-reach";
import { reachTexts } from "./reach-texts";
import "./reach.css";

/** How many changes a note lists before it counts the rest. */
const LISTED = 4;

/** What is known for each place, centre and set of objects, asked once. */
const asked = new Map<string, Promise<WalkwayReach | null>>();

/**
 * What hiding `objects` does to the ways characters take near them, in the
 * worldspace `place` near `centre`, or the interior cell `place` (`centre`
 * null). `undefined` while asking or with nothing to ask, `null` if the
 * setup cannot be read.
 */
export function useReach(
  place: string | null,
  centre: [number, number] | null,
  objects: ObjectRef[],
): WalkwayReach | null | undefined {
  const { api } = useSession();
  const key =
    place === null || objects.length === 0
      ? null
      : `${place}|${centre?.map(Math.round).join(",") ?? "inside"}|${objects
          .map((object) => `${object.plugin.toLowerCase()}:${object.localId}`)
          .sort()
          .join(";")}`;
  const [found, setFound] = useState<{ key: string; reach: WalkwayReach | null } | null>(null);
  // biome-ignore lint/correctness/useExhaustiveDependencies: the objects are compared by `key`.
  useEffect(() => {
    if (key === null || place === null) {
      return;
    }
    let outdated = false;
    let answer = asked.get(key);
    if (answer === undefined) {
      answer = api.walkwayReach(place, centre?.[0] ?? null, centre?.[1] ?? null, objects);
      asked.set(key, answer);
      // Not known: asked again next time.
      answer.catch(() => asked.delete(key));
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

/** The sentence that says what happens to the way to one destination. */
export function useReachSentence(): (change: ReachChange) => string {
  const r = reachTexts[useSession().lang];
  return (change) => {
    const what = r.what(change.destination);
    switch (change.outcome) {
      case "gone":
        return r.gone(what);
      case "cut-off":
        return r.cutOff(what);
      case "longer":
        return r.longer(
          what,
          change.from ? r.what(change.from) : "",
          change.before?.metres ?? 0,
          change.after?.metres ?? null,
        );
    }
  };
}

/**
 * Says plainly what hiding breaks for the characters, or that nothing
 * does. Nothing while there is nothing to tell.
 */
export function ReachNote({
  reach,
  follow,
}: {
  reach: WalkwayReach | null | undefined;
  /**
   * Where the objects are hidden, to offer moving or hiding the places
   * where characters do something that stood on them (ADR 0069).
   */
  follow?: { place: string; centre: [number, number] | null; hidden: ObjectRef[] } | null;
}) {
  const r = reachTexts[useSession().lang];
  const sentence = useReachSentence();
  if (reach === undefined || reach === null || reach.checked === 0) {
    return null;
  }
  const gone = reach.changes
    .filter((change) => change.outcome === "gone")
    .map((change) => change.destination);
  const offers =
    follow == null || gone.length === 0 ? null : (
      <FollowHiding
        place={follow.place}
        centre={follow.centre}
        hidden={follow.hidden}
        destinations={gone}
      />
    );
  if (reach.changes.length === 0) {
    return <p className="reach-note reach-fine">{r.allReached(reach.checked)}</p>;
  }
  const listed = reach.changes.slice(0, LISTED);
  const rest = reach.changes.length - listed.length;
  return (
    <>
      <div className="reach-note">
        <ul>
          {listed.map((change) => (
            <li
              key={`${change.outcome}|${change.destination.object.plugin}:${change.destination.object.localId}`}
              data-outcome={change.outcome}
            >
              {sentence(change)}
            </li>
          ))}
        </ul>
        {rest > 0 && <p className="reach-more">{r.more(rest)}</p>}
        <p className="reach-legend">{r.legend}</p>
      </div>
      {offers}
    </>
  );
}

/** The colours of the ways: grey before, green after (Okabe–Ito), red for none. */
const BEFORE = "rgb(200 200 200 / 85%)";
const AFTER = "#009e73";
const NONE = "#d55e00";

/** Draws the ways of `reach` over the 3D view, for `MapView3D`'s `paint`. */
export function usePaintReach(reach: WalkwayReach | null | undefined) {
  return useMemo(() => {
    if (reach === undefined || reach === null || reach.changes.length === 0) {
      return undefined;
    }
    return (
      context: CanvasRenderingContext2D,
      toScreen: (point: [number, number, number]) => { x: number; y: number } | null,
    ) => {
      const line = (route: Route | null | undefined, colour: string, dashed: boolean) => {
        if (!route || route.points.length < 2) {
          return;
        }
        context.save();
        context.strokeStyle = colour;
        context.lineWidth = 3;
        context.lineJoin = "round";
        context.lineCap = "round";
        context.setLineDash(dashed ? [7, 5] : []);
        context.beginPath();
        let started = false;
        for (const point of route.points) {
          const shown = toScreen(point);
          if (shown === null) {
            started = false;
            continue;
          }
          if (started) {
            context.lineTo(shown.x, shown.y);
          } else {
            context.moveTo(shown.x, shown.y);
            started = true;
          }
        }
        context.stroke();
        context.restore();
      };
      const mark = (point: [number, number, number], colour: string) => {
        const shown = toScreen(point);
        if (shown === null) {
          return;
        }
        context.save();
        context.beginPath();
        context.arc(shown.x, shown.y, 7, 0, Math.PI * 2);
        context.fillStyle = colour;
        context.fill();
        context.lineWidth = 2;
        context.strokeStyle = "#1b1b1b";
        context.stroke();
        context.restore();
      };
      for (const change of reach.changes) {
        line(change.before, BEFORE, true);
        line(change.after, AFTER, false);
      }
      for (const change of reach.changes) {
        mark(change.destination.position, change.after ? AFTER : NONE);
      }
    };
  }, [reach]);
}
