// What can be done with one object picked in a 3D view (ADR 0040, 0046):
// hide it on its own through the patch, with the reasons the game may rely
// on it; show it again however it was hidden; take the parts of the same
// mod that hang together with it along; or, where it is part of a
// collision, decide that collision on the collisions' page.

import { useEffect, useMemo, useState } from "react";
import { useSession } from "./Actions";
import type { Api } from "./api";
import { objectKey, type PartTools, TOUCHING } from "./area-layer";
import { collisionsTexts } from "./collisions-texts";
import type { AreaObject } from "./generated/area-objects";
import type { Anchor, ObjectAnchors } from "./generated/object-anchors";
import type { MapSpot, ObjectRef } from "./generated/rendered-report";
import type { Texts } from "./i18n";
import { chooseObject, hiddenKeysOf, matchKey, objectChoiceKey, revealObjects } from "./patch";

/** The plugins of the games themselves, whose objects quests may need. */
const GAME_PLUGIN =
  /^(skyrim|update|dawnguard|hearthfires|dragonborn|fallout4|dlc\w*|oblivion|fallout3|falloutnv)\.esm$|^cc\w*\.es[lm]$/i;

/** Game units in a centimetre, for how close parts must come to hang together. */
const UNITS_PER_CENTIMETRE = 0.7;

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
 * The parts of the same mod that hang together with an object, as far as
 * the view shows them: how close they must come, marked in the view, and
 * hidden or shown again together.
 */
function PartsTogether({ object, tools }: { object: AreaObject; tools: PartTools }) {
  const { lang, spots, patchChoices, setPatchChoices } = useSession();
  const own = collisionsTexts[lang];
  const hidden = useHidden();
  // How close parts must come, in centimetres.
  const [gap, setGap] = useState(Math.round(TOUCHING / UNITS_PER_CENTIMETRE));
  const { connected, mark } = tools;
  const group = useMemo(() => connected(gap * UNITS_PER_CENTIMETRE), [connected, gap]);
  const others = group.filter((part) => part !== object);
  if (others.length === 0 && gap <= 20) {
    return (
      <div className="parts-together">
        <p className="aside">{own.partsAlone}</p>
        <GapSlider gap={gap} onGap={setGap} />
      </div>
    );
  }
  const allHidden = group.every((part) => hidden.has(matchKey(part.id)));
  const hideable = group.filter((part) => !part.anchored);
  return (
    <div className="parts-together">
      <p className="aside">{own.partsTogether(group.length)}</p>
      <GapSlider gap={gap} onGap={setGap} />
      <span className="relief-object-buttons">
        <button type="button" className="small" onClick={() => mark(group)}>
          {own.partsMark}
        </button>
        {allHidden ? (
          <button
            type="button"
            className="small primary"
            onClick={() =>
              setPatchChoices(
                revealObjects(
                  patchChoices,
                  spots,
                  group.map((part) => part.id),
                ),
              )
            }
          >
            {own.partsShow(group.length)}
          </button>
        ) : (
          <button
            type="button"
            className="small"
            disabled={hideable.length === 0}
            onClick={() => {
              let next = patchChoices;
              for (const part of hideable) {
                next = chooseObject(next, part.id, true);
              }
              setPatchChoices(next);
            }}
          >
            {own.partsHide(hideable.length)}
          </button>
        )}
      </span>
      <p className="aside parts-hint">
        {hideable.length < group.length ? own.partsSomeTied(group.length - hideable.length) : ""}
        {own.partsHint}
      </p>
    </div>
  );
}

function GapSlider({ gap, onGap }: { gap: number; onGap: (gap: number) => void }) {
  const { lang } = useSession();
  const own = collisionsTexts[lang];
  return (
    <label className="parts-gap">
      {own.partsGap(gap)}
      <input
        type="range"
        min={0}
        max={100}
        step={5}
        value={gap}
        onChange={(event) => onGap(Number(event.target.value))}
      />
    </label>
  );
}

/**
 * Hiding an object on its own, or showing it again however it was hidden,
 * after what ties it: nothing for an object the game cannot do without, a
 * warning for one it may miss. With the view's tools, the parts that hang
 * together with it as well.
 */
export function FreeObjectActions({
  object,
  place,
  tools,
}: {
  object: AreaObject;
  /** The worldspace or interior cell it is in. */
  place: string | null;
  tools?: PartTools;
}) {
  const { t, lang, spots, patchChoices, setPatchChoices } = useSession();
  const own = collisionsTexts[lang];
  const anchors = useAnchors(place, object.id);
  const hidden = useHidden();
  // Without an answer, the two flags of the placed object decide as before.
  const locked = anchors === null ? object.anchored : anchors?.verdict === "locked";
  const careful = anchors?.verdict === "careful";
  const isHidden = hidden.has(matchKey(object.id));
  const byItself = objectChoiceKey(object.id) in patchChoices;
  const together = tools === undefined ? null : <PartsTogether object={object} tools={tools} />;
  if (isHidden) {
    return (
      <div className="relief-object-action">
        <p className="aside">{byItself ? own.hiddenByItself : own.hiddenByDecision}</p>
        <button
          type="button"
          className="small primary"
          onClick={() => setPatchChoices(revealObjects(patchChoices, spots, [object.id]))}
        >
          {t.workObjectShow}
        </button>
        {together}
      </div>
    );
  }
  if (anchors === undefined) {
    return (
      <div className="relief-object-action">
        <p className="aside">{t.anchorAsking}</p>
        {together}
      </div>
    );
  }
  if (locked) {
    return (
      <div className="relief-object-action">
        <p className="aside">{t.anchorLocked}</p>
        {anchors !== null && <AnchorReasons anchors={anchors} />}
        {together}
      </div>
    );
  }
  return (
    <div className="relief-object-action">
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
      {together}
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
      <FreeObjectActions object={object} tools={tools} place={place} />
    </>
  );
}
