// On the card of an object picked in a 3D view: which open collisions and
// door spots it is part of, with a link to decide each, and which
// collisions hiding it decided (ADR 0055). Deciding doors is the door
// panel's business; this only shows and links.

import "./object-conflicts.css";
import { useMemo } from "react";
import { useDecisions, useSession } from "./Actions";
import { collidingWith, conflictsOf } from "./conflicts";
import type { MapSpot } from "./generated/rendered-report";
import type { Lang } from "./i18n";
import { decisionAt, matchKey, spotKey } from "./patch";

interface ConflictTexts {
  /** "This object is in 2 open collisions and 1 open door spot:" */
  open: (subject: "object" | "parts", collisions: number, doors: number) => string;
  decide: string;
  /** "This decided the collision with X." */
  decided: (names: string) => string;
  decidedMore: (count: number) => string;
  door: string;
}

const plural = (count: number, one: string, many: string) =>
  count === 1 ? `1 ${one}` : `${count} ${many}`;

const texts: Record<Lang, ConflictTexts> = {
  en: {
    open: (subject, collisions, doors) => {
      const what = [
        collisions > 0 ? plural(collisions, "open collision", "open collisions") : "",
        doors > 0 ? plural(doors, "open door spot", "open door spots") : "",
      ]
        .filter((text) => text !== "")
        .join(" and ");
      return `${subject === "object" ? "This object is" : "These parts are"} in ${what}:`;
    },
    decide: "Decide",
    decided: (names) => `This decided the collision with ${names}.`,
    decidedMore: (count) => `And ${count} more collisions decided the same way.`,
    door: "door",
  },
  de: {
    open: (subject, collisions, doors) => {
      const what = [
        collisions > 0 ? plural(collisions, "offenen Kollision", "offenen Kollisionen") : "",
        doors > 0 ? plural(doors, "offenen Türstelle", "offenen Türstellen") : "",
      ]
        .filter((text) => text !== "")
        .join(" und ");
      return `${subject === "object" ? "Dieses Objekt gehört" : "Diese Teile gehören"} zu ${what}:`;
    },
    decide: "Entscheiden",
    decided: (names) => `Damit ist die Kollision mit ${names} entschieden.`,
    decidedMore: (count) => `Und ${count} weitere Kollisionen auf dieselbe Art.`,
    door: "Tür",
  },
};

/** How many decided collisions are named in full before the rest is counted. */
const NAMED = 3;

const isDoor = (spot: MapSpot) => (spot.doors ?? null) !== null || spot.kind === "door";

/**
 * The open collisions and door spots `objects` are part of, each with a
 * link to decide it, and the collisions their being hidden decided. With
 * `except`, not those that one of these objects is part of: the list of a
 * wand's selection beside the one of the picked object.
 */
export function ObjectConflicts({
  objects,
  except,
  subject = "object",
}: {
  objects: readonly { plugin: string; localId: number }[];
  except?: readonly { plugin: string; localId: number }[];
  subject?: "object" | "parts";
}) {
  const { lang, spots, patchChoices, decideSpot } = useSession();
  const t = texts[lang];
  const found = useMemo(
    () =>
      conflictsOf(
        [...spots.values()].flat(),
        objects,
        (spot) => decisionAt(patchChoices, spot.finding, spotKey(spot), spot.parts ?? []),
        (spot) => {
          const key = spotKey(spot);
          return (key !== null && key in patchChoices) || spot.finding in patchChoices;
        },
        except,
      ),
    [spots, patchChoices, objects, except],
  );
  // Door spots are decided in the door panel: they are listed, not counted as decided.
  const hidden = useDecisions().decisions.hiddenKeys;
  const mine = new Set(objects.map(matchKey));
  const decided = found.decidedByHiding.filter(
    (spot) =>
      !isDoor(spot) &&
      (spot.parts ?? []).some(
        (part) => mine.has(matchKey(part.object)) && hidden.has(matchKey(part.object)),
      ),
  );
  if (found.open.length === 0 && decided.length === 0) {
    return null;
  }
  const doors = found.open.filter(isDoor).length;
  return (
    <div className="object-conflicts">
      {found.open.length > 0 && (
        <>
          <p className="aside">{t.open(subject, found.open.length - doors, doors)}</p>
          <ul className="object-conflict-list">
            {found.open.map((spot) => (
              <li key={`${spot.finding}|${spotKey(spot)}`}>
                <span className="object-conflict-name">
                  {isDoor(spot) && <span className="aside">{t.door} · </span>}
                  {spot.objects}
                </span>
                <button type="button" className="small" onClick={() => decideSpot(spot)}>
                  {t.decide}
                </button>
              </li>
            ))}
          </ul>
        </>
      )}
      {decided.slice(0, NAMED).map((spot) => (
        <p className="aside" key={`${spot.finding}|${spotKey(spot)}`}>
          {t.decided(collidingWith(spot, objects).join(", ") || spot.objects)}
        </p>
      ))}
      {decided.length > NAMED && <p className="aside">{t.decidedMore(decided.length - NAMED)}</p>}
    </div>
  );
}
