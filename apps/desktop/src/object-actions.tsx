// What can be done with one object picked in a 3D view (ADR 0040, 0046):
// hide it on its own through the patch, with the reasons the game may rely
// on it; show it again however it was hidden; take what the magic wand
// took along with it (ADR 0055); or, where it is part of a
// collision, decide that collision on the collisions' page.

import { useEffect, useMemo, useState } from "react";
import { useSession } from "./Actions";
import { ActionPlaceInfo } from "./action-places";
import type { Api } from "./api";
import { objectKey, type PartTools } from "./area-layer";
import { collisionsTexts } from "./collisions-texts";
import type { AreaObject } from "./generated/area-objects";
import type { Anchor, ObjectAnchors } from "./generated/object-anchors";
import type { MapSpot, ObjectRef } from "./generated/rendered-report";
import type { Texts } from "./i18n";
import { LocateButton } from "./locate";
import { MoveControls } from "./move-controls";
import { ObjectConflicts } from "./object-conflicts";
import {
  chooseObject,
  decisionAt,
  hiddenKeysOf,
  matchKey,
  objectChoiceKey,
  revealObjects,
  spotKey,
} from "./patch";
import { PinButton } from "./pins";
import { useWandTexts } from "./wand";
import { offersFor, splitParts } from "./wand-parts";

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

/** What the current choices hide, by {@link matchKey}. */
function useHidden(): Set<string> {
  const { spots, patchChoices } = useSession();
  return useMemo(() => hiddenKeysOf(spots, patchChoices), [spots, patchChoices]);
}

/**
 * What the magic wand took along with an object (ADR 0055), hidden or shown
 * again together; with the wand off, where to raise it.
 */
export function PartsTogether({ tools }: { tools: PartTools }) {
  const { lang, spots, patchChoices, setPatchChoices } = useSession();
  const own = collisionsTexts[lang];
  const w = useWandTexts();
  const hidden = useHidden();
  const { selection: group, level } = tools;
  if (level === 0) {
    return (
      <div className="parts-together">
        <p className="aside parts-hint">{w.raise}</p>
      </div>
    );
  }
  if (group.length <= 1) {
    return (
      <div className="parts-together">
        <p className="aside wand-took">{w.took(group.length, level)}</p>
        <p className="aside">{w.alone}</p>
      </div>
    );
  }
  // Both ways (ADR 0055): from a hidden object the hidden parts of the
  // selection are shown again, from a visible one the visible parts are
  // hidden; a selection of both states offers both.
  const isHidden = (part: AreaObject) => hidden.has(matchKey(part.id));
  const split = splitParts(group, isHidden);
  const first = group[0];
  const offers = offersFor(split, first !== undefined && isHidden(first));
  return (
    <div className="parts-together">
      <p className="aside wand-took">
        {w.took(group.length, level)} <LocateButton objects={group.map((part) => part.id)} />
      </p>
      {split.hidden.length > 0 && split.visible.length > 0 && (
        <p className="aside parts-hint">{w.mixed(split.hidden.length, split.visible.length)}</p>
      )}
      <span className="relief-object-buttons">
        {offers.map((offer) => (
          <button
            key={offer.direction}
            type="button"
            className={offer.primary ? "small primary" : "small"}
            onClick={() => {
              if (offer.direction === "show") {
                setPatchChoices(
                  revealObjects(
                    patchChoices,
                    spots,
                    split.hidden.map((part) => part.id),
                  ),
                );
                return;
              }
              let next = patchChoices;
              for (const part of split.hideable) {
                next = chooseObject(next, part.id, true);
              }
              setPatchChoices(next);
            }}
          >
            {offer.direction === "show" ? own.partsShow(offer.count) : own.partsHide(offer.count)}
          </button>
        ))}
      </span>
      {group[0] !== undefined && (
        <ObjectConflicts
          objects={group.map((part) => part.id)}
          except={[group[0].id]}
          subject="parts"
        />
      )}
      {split.tied > 0 && <p className="aside parts-hint">{own.partsSomeTied(split.tied)}</p>}
    </div>
  );
}

/**
 * Hiding an object on its own, or showing it again however it was hidden,
 * after what ties it: nothing for an object the game cannot do without, a
 * warning for one it may miss. With the view's tools, what the magic
 * wand took along with it as well (ADR 0055).
 */
export function FreeObjectActions({
  object,
  place,
  tools,
  interior = false,
}: {
  object: AreaObject;
  /** The worldspace or interior cell it is in. */
  place: string | null;
  tools?: PartTools;
  /** Whether `place` is an interior cell. */
  interior?: boolean;
}) {
  const { t, lang, spots, patchChoices, setPatchChoices } = useSession();
  const own = collisionsTexts[lang];
  const anchors = useAnchors(place, object.id);
  const hidden = useHidden();
  // Without an answer, the two flags of the placed object decide as before.
  const locked = anchors === null ? object.anchored : anchors?.verdict === "locked";
  const careful = anchors?.verdict === "careful";
  const isHidden = hidden.has(matchKey(object.id));
  // A null choice is one shown again on purpose, not one hidden by itself.
  const byItself = (patchChoices[objectChoiceKey(object.id)] ?? null) !== null;
  // The collisions and door spots it is part of, and which hiding it decided.
  const conflicts = <ObjectConflicts objects={[object.id]} />;
  const together = tools === undefined ? null : <PartsTogether tools={tools} />;
  // Or a little to one side instead (ADR 0060).
  const move = <MoveControls object={object} tools={tools} place={place} interior={interior} />;
  // What a place where characters do something is, and who uses it (ADR 0069).
  const action = (
    <ActionPlaceInfo
      object={object}
      place={place}
      interior={interior}
      around={tools?.layer?.area?.objects}
    />
  );
  if (isHidden) {
    return (
      <div className="relief-object-action">
        {action}
        <p className="aside">{byItself ? own.hiddenByItself : own.hiddenByDecision}</p>
        <button
          type="button"
          className="small primary"
          onClick={() => setPatchChoices(revealObjects(patchChoices, spots, [object.id]))}
        >
          {t.workObjectShow}
        </button>
        {conflicts}
        {together}
      </div>
    );
  }
  if (anchors === undefined) {
    return (
      <div className="relief-object-action">
        {action}
        <p className="aside">{t.anchorAsking}</p>
        {conflicts}
        {together}
        {move}
      </div>
    );
  }
  if (locked) {
    return (
      <div className="relief-object-action">
        {action}
        <p className="aside">{t.anchorLocked}</p>
        {anchors !== null && <AnchorReasons anchors={anchors} />}
        {conflicts}
        {together}
        {move}
      </div>
    );
  }
  return (
    <div className="relief-object-action">
      {action}
      {careful && anchors !== null && (
        <>
          <p className="aside relief-object-warning">{t.anchorCareful}</p>
          <AnchorReasons anchors={anchors} />
        </>
      )}
      {isGameObject(object) && <p className="aside relief-object-warning">{t.workObjectGame}</p>}
      <button
        type="button"
        className="small primary"
        onClick={() => setPatchChoices(chooseObject(patchChoices, object.id, true))}
      >
        {careful ? t.anchorHideAnyway : t.workObjectHide}
      </button>
      <p className="aside parts-hint">{own.hideHint}</p>
      {conflicts}
      {together}
      {move}
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
  const { t, spots, patchChoices, setPatchChoices } = useSession();
  const anchors = useAnchors(place, object);
  const hidden = useHidden();
  const isHidden = hidden.has(matchKey(object));
  return (
    <div className="needed-part">
      <strong>{name}</strong>
      <LocateButton objects={[object]} />
      {anchors === undefined ? (
        <span className="aside">{t.anchorAsking}</span>
      ) : anchors === null ? null : (
        <AnchorReasons anchors={anchors} />
      )}
      {anchors != null && (anchors.verdict !== "locked" || isHidden) && (
        <button
          type="button"
          className="small"
          onClick={() =>
            setPatchChoices(
              isHidden
                ? revealObjects(patchChoices, spots, [object])
                : chooseObject(patchChoices, object, true),
            )
          }
        >
          {isHidden ? t.workObjectShow : t.anchorHideAnyway}
        </button>
      )}
    </div>
  );
}

/** The spot of a collision an object is part of, if any. */
export function spotOf(spots: Map<string, MapSpot[]>, object: AreaObject): MapSpot | null {
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
  tools,
  place,
}: {
  object: AreaObject;
  tools: PartTools;
  place: string;
}) {
  const { t, spots, decideSpot, patchChoices } = useSession();
  const spot = spotOf(spots, object);
  const key = spot === null ? null : spotKey(spot);
  // A collision still open can be pinned to solve later (ADR 0050).
  const open =
    spot !== null &&
    key !== null &&
    decisionAt(patchChoices, spot.finding, key, spot.parts ?? []) === undefined;
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
      {open && key !== null && <PinButton keys={[key]} />}
      <FreeObjectActions object={object} tools={tools} place={place} />
    </>
  );
}
