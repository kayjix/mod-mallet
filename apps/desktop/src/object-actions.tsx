// What can be done with one object picked in a 3D view (ADR 0040, 0046):
// hide it on its own through the patch, with the reasons the game may rely
// on it, or, where it is part of a collision, decide that collision on the
// collisions' page.

import { useEffect, useState } from "react";
import { useSession } from "./Actions";
import type { Api } from "./api";
import { objectKey } from "./area-layer";
import type { AreaObject } from "./generated/area-objects";
import type { Anchor, ObjectAnchors } from "./generated/object-anchors";
import type { MapSpot, ObjectRef } from "./generated/rendered-report";
import type { Texts } from "./i18n";
import { chooseObject, objectChoiceKey } from "./patch";

/** The plugins of the games themselves, whose objects quests may need. */
const GAME_PLUGIN =
  /^(skyrim|update|dawnguard|hearthfires|dragonborn|fallout4|dlc\w*|oblivion|fallout3|falloutnv)\.esm$|^cc\w*\.es[lm]$/i;

/** Whether an object belongs to the game itself rather than to a mod. */
export const isGameObject = (object: { id: ObjectRef }) => GAME_PLUGIN.test(object.id.plugin);

/** What is known of each object's ties, asked once per place and object. */
const asked = new Map<string, Promise<ObjectAnchors | null>>();

function anchorsOf(api: Api, place: string, object: ObjectRef): Promise<ObjectAnchors | null> {
  const key = `${place}|${object.plugin.toLowerCase()}|${object.localId}`;
  let found = asked.get(key);
  if (found === undefined) {
    found = api
      .objectAnchors(place, [object])
      .then((all) => all?.[0] ?? null)
      .catch(() => null);
    asked.set(key, found);
  }
  return found;
}

/**
 * Why the game may rely on an object: `undefined` while asking, `null`
 * where it cannot be told.
 */
export function useAnchors(
  place: string | null,
  object: ObjectRef,
): ObjectAnchors | null | undefined {
  const { api } = useSession();
  const [found, setFound] = useState<ObjectAnchors | null | undefined>(undefined);
  const { plugin, localId } = object;
  useEffect(() => {
    if (place === null) {
      setFound(null);
      return;
    }
    let outdated = false;
    setFound(undefined);
    anchorsOf(api, place, { plugin, localId }).then((result) => {
      if (!outdated) {
        setFound(result);
      }
    });
    return () => {
      outdated = true;
    };
  }, [api, place, plugin, localId]);
  return found;
}

/** One tie, in words. */
function reasonText(t: Texts, anchor: Anchor): string {
  const other = anchor.other;
  const name = other == null ? "" : (other.name ?? `${other.plugin} ${other.localId.toString(16)}`);
  const text = t.anchorKinds[anchor.kind](name, anchor.keyword ?? null);
  if (other?.state === "gone") {
    return `${text} ${t.anchorGone}`;
  }
  return other?.state === "off" ? `${text} ${t.anchorOff}` : text;
}

/** The ties of an object that count, as a short list. */
export function AnchorReasons({ anchors }: { anchors: ObjectAnchors }) {
  const { t } = useSession();
  if (anchors.reasons.length === 0) {
    return null;
  }
  return (
    <ul className="anchor-reasons" data-verdict={anchors.verdict}>
      {anchors.reasons.map((anchor) => {
        const text = reasonText(t, anchor);
        return <li key={text}>{text}</li>;
      })}
    </ul>
  );
}

/**
 * Hiding an object on its own, or showing it again, after what ties it:
 * nothing for an object the game cannot do without, a warning for one it
 * may miss, nothing for one already hidden otherwise (`hiddenOtherwise`).
 */
export function FreeObjectActions({
  object,
  place,
  group = [],
  hiddenOtherwise = false,
}: {
  object: AreaObject;
  /** The worldspace or interior cell it is in. */
  place: string | null;
  /** The parts that hang together with it, itself among them. */
  group?: AreaObject[];
  hiddenOtherwise?: boolean;
}) {
  const { t, patchChoices, setPatchChoices } = useSession();
  const anchors = useAnchors(place, object.id);
  // Without an answer, the two flags of the placed object decide as before.
  const locked = anchors === null ? object.anchored : anchors?.verdict === "locked";
  const careful = anchors?.verdict === "careful";
  // The parts of the group that may be hidden, and whether all are.
  const hideable = group.filter((part) => !part.anchored || part === object);
  const allOwn =
    hideable.length > 1 && hideable.every((part) => objectChoiceKey(part.id) in patchChoices);
  const groupButton = hideable.length > 1 && (
    <button
      type="button"
      className="small"
      onClick={() => {
        let next = patchChoices;
        for (const part of hideable) {
          next = chooseObject(next, part.id, !allOwn);
        }
        setPatchChoices(next);
      }}
    >
      {allOwn ? t.workGroupShow(hideable.length) : t.workGroupHide(hideable.length)}
    </button>
  );
  const own = objectChoiceKey(object.id) in patchChoices;
  if (anchors === undefined) {
    return <p className="aside relief-object-action">{t.anchorAsking}</p>;
  }
  if (locked && !own) {
    return (
      <div className="relief-object-action">
        <p className="aside">{t.anchorLocked}</p>
        {anchors !== null && <AnchorReasons anchors={anchors} />}
      </div>
    );
  }
  if (hiddenOtherwise && !own) {
    return null;
  }
  return (
    <div className="relief-object-action">
      {careful && !own && anchors !== null && (
        <>
          <p className="aside relief-object-warning">{t.anchorCareful}</p>
          <AnchorReasons anchors={anchors} />
        </>
      )}
      {isGameObject(object) && !own && (
        <p className="aside relief-object-warning">{t.workObjectGame}</p>
      )}
      <span className="relief-object-buttons">
        <button
          type="button"
          className={own ? "small" : "small primary"}
          onClick={() => setPatchChoices(chooseObject(patchChoices, object.id, !own))}
        >
          {own ? t.workObjectShow : careful ? t.anchorHideAnyway : t.workObjectHide}
        </button>
        {groupButton}
      </span>
    </div>
  );
}

/**
 * A part of a collision that the scan marked as needed by the game: why,
 * and hiding it on its own where the game can do without it.
 */
export function NeededPart({
  place,
  object,
  name,
}: {
  place: string | null;
  object: ObjectRef;
  name: string;
}) {
  const { t, patchChoices, setPatchChoices } = useSession();
  const anchors = useAnchors(place, object);
  const own = objectChoiceKey(object) in patchChoices;
  return (
    <div className="needed-part">
      <strong>{name}</strong>
      {anchors === undefined ? (
        <span className="aside">{t.anchorAsking}</span>
      ) : anchors === null ? null : (
        <AnchorReasons anchors={anchors} />
      )}
      {anchors != null && anchors.verdict !== "locked" && (
        <button
          type="button"
          className="small"
          onClick={() => setPatchChoices(chooseObject(patchChoices, object, !own))}
        >
          {own ? t.workObjectShow : t.anchorHideAnyway}
        </button>
      )}
    </div>
  );
}

/** The spot of a collision an object is part of, if any. */
function spotOf(spots: Map<string, MapSpot[]>, object: AreaObject): MapSpot | null {
  const key = objectKey(object.id.plugin, object.id.localId);
  for (const found of spots.values()) {
    for (const spot of found) {
      if (
        (spot.parts ?? []).some(
          (part) => objectKey(part.object.plugin, part.object.localId) === key,
        )
      ) {
        return spot;
      }
    }
  }
  return null;
}

/** On the map: decide the object's collision on the collisions' page, or hide it on its own. */
export function MapObjectActions({
  object,
  group,
  place,
}: {
  object: AreaObject;
  group: AreaObject[];
  place: string;
}) {
  const { t, spots, decideSpot } = useSession();
  const spot = spotOf(spots, object);
  return (
    <>
      {spot !== null && (
        <button
          type="button"
          className="small primary relief-object-action"
          onClick={() => decideSpot(spot)}
        >
          {t.workDecideHere}
        </button>
      )}
      <FreeObjectActions object={object} group={group} place={place} />
    </>
  );
}
