import { useMemo, useState } from "react";
import { ActionResult, useAction, useSession } from "./Actions";
import type { MapSpot, ObjectRef, RenderedReport } from "./generated/rendered-report";
import {
  choicesOf,
  chooseObject,
  decisionsOf,
  freeObjects,
  keyOf,
  matchKey,
  objectsOf,
  openSpots,
  type PairDecision,
  pairsOf,
  signatureOf,
  spotsByKey,
  undoDecision,
} from "./patch";
import { type PatchTexts, patchTexts } from "./patch-texts";

/** Base Object Swapper's page, for when it is missing. */
const SWAPPER_PAGE = "https://www.nexusmods.com/skyrimspecialedition/mods/60805";

/**
 * The patch made for this setup (ADR 0048): what was decided in Collisions,
 * listed by pair of mods with *change* and *undo*, what is in the patch and
 * what is still waiting for it, and its creation and installation. The page
 * makes no decisions of its own.
 */
export function PatchView({ report }: { report: RenderedReport }) {
  const session = useSession();
  const { t, api, ignored, patchChoices: choices, setPatchChoices: setChoices } = session;
  const p = patchTexts[session.lang];
  const pairs = useMemo(() => pairsOf(report, ignored), [report, ignored]);
  const spots = useMemo(() => spotsByKey(report), [report]);
  const installedObjects = report.installedPatch?.objects ?? null;
  const installed = useMemo(
    () => new Set((installedObjects ?? []).map(matchKey)),
    [installedObjects],
  );
  const own = freeObjects(choices);
  const [created, setCreated] = useState<{ path: string; signature: string } | null>(null);
  const [creating, setCreating] = useState(false);
  const [createError, setCreateError] = useState<string | null>(null);
  const action = useAction();

  const objects = objectsOf(pairs, choices);
  const signature = signatureOf(objects);
  const upToDate = created !== null && created.signature === signature;
  const chosen = new Set(objects.map(matchKey));
  const waiting = [...chosen].filter((key) => !installed.has(key)).length;
  const decisions = decisionsOf(pairs, choices);
  const isWaiting = (decision: PairDecision) =>
    decision.hidden.some((object) => !installed.has(matchKey(object)));
  const waitingPairs = decisions.filter(isWaiting).length;
  const open = openSpots(pairs, choices);
  const fromInstalled = useMemo(
    () => choicesOf(pairs, installed, installedObjects ?? []),
    [pairs, installed, installedObjects],
  );
  const sameAsInstalled = signatureOf(objectsOf(pairs, fromInstalled)) === signature;
  const spotOf = (key: string | undefined): MapSpot | undefined =>
    key === undefined ? undefined : spots.get(key);
  const goToSpot = (key: string | undefined) => {
    const spot = spotOf(key);
    if (spot !== undefined) {
      session.decideSpot(spot);
    }
  };

  const create = async () => {
    setCreating(true);
    setCreateError(null);
    try {
      const path = await api.createPatch(objects);
      setCreated({ path, signature });
    } catch (error: unknown) {
      setCreateError(String(error));
    } finally {
      setCreating(false);
    }
  };

  if (!report.objectSwapper) {
    return (
      <section className="patch-missing">
        <p>{p.needsSwapper}</p>
        <button type="button" onClick={() => api.openPage(SWAPPER_PAGE)}>
          {t.openPage}
        </button>
      </section>
    );
  }
  if (pairs.length === 0 && own.length === 0) {
    return <p className="empty">{t.nothingHere}</p>;
  }
  return (
    <section className="patch-view">
      <div className="patch-bar">
        <strong>
          {p.summary(
            chosen.size,
            installedObjects === null ? null : installedObjects.length,
            waiting,
          )}
        </strong>
        <span className="spacer" />
        <button type="button" disabled={objects.length === 0 || creating} onClick={create}>
          {p.create}
        </button>
        <button
          type="button"
          className="primary"
          disabled={!upToDate || action.busy}
          title={p.installHint}
          onClick={() => action.run({ type: "hide-objects", objects })}
        >
          {p.install}
        </button>
        <button
          type="button"
          disabled={Object.keys(choices).length === 0}
          title={p.clearHint}
          onClick={() => setChoices({})}
        >
          {p.clear}
        </button>
      </div>
      {installedObjects !== null && (
        <div
          className={
            waiting === 0 && sameAsInstalled ? "patch-installed" : "patch-installed differs"
          }
        >
          {report.installedPatch?.provider?.name && (
            <p className="patch-line">{p.installedBy(report.installedPatch.provider.name)}</p>
          )}
          {!sameAsInstalled && (
            <button
              type="button"
              title={p.useInstalledHint}
              onClick={() => setChoices(fromInstalled)}
            >
              {p.useInstalled}
            </button>
          )}
        </div>
      )}
      {created && (
        <div className={upToDate ? "patch-file" : "patch-file stale"}>
          <span>
            {upToDate ? p.created : p.stale} {upToDate && <code>{created.path}</code>}
          </span>
          <button type="button" onClick={() => api.openFolder("patches")}>
            {t.openFolder}
          </button>
        </div>
      )}
      {createError && <p className="failure">{createError}</p>}
      <ActionResult state={action} />
      {waiting > 0 && (
        <p className="patch-line patch-waiting">{p.waiting(waiting, waitingPairs)}</p>
      )}
      {open.keys.length > 0 && (
        <p className="patch-line patch-open">
          {p.open(open.keys.length, open.pairs)}{" "}
          <button type="button" className="small" onClick={() => goToSpot(open.keys[0])}>
            {p.toCollisions}
          </button>
        </p>
      )}

      <h3>{p.decided}</h3>
      {decisions.length === 0 ? (
        <p className="aside">{p.nothingDecided}</p>
      ) : (
        <ul className="patch-list">
          {decisions.map((decision) => (
            <DecisionRow
              key={decision.mods.join("\u0000")}
              decision={decision}
              p={p}
              waiting={isWaiting(decision)}
              installedKnown={installedObjects !== null}
              onChange={() => {
                if (decision.wholes.length > 0) {
                  session.openPair(decision.mods[0], decision.mods[1]);
                } else {
                  goToSpot(decision.spots[0]);
                }
              }}
              onChangeSpots={() => goToSpot(decision.spots[0])}
              onUndo={() => setChoices(undoDecision(choices, decision))}
            />
          ))}
        </ul>
      )}

      {own.length > 0 && (
        <OwnObjects
          own={own}
          p={p}
          onShow={(object) => setChoices(chooseObject(choices, object, false))}
        />
      )}
    </section>
  );
}

interface DecisionRowProps {
  decision: PairDecision;
  p: PatchTexts;
  /** Whether some of what it hides is not in the installed patch yet. */
  waiting: boolean;
  /** Whether there is an installed patch to compare with. */
  installedKnown: boolean;
  onChange: () => void;
  onChangeSpots: () => void;
  onUndo: () => void;
}

/** What was decided about one pair of mods. */
function DecisionRow({
  decision,
  p,
  waiting,
  installedKnown,
  onChange,
  onChangeSpots,
  onUndo,
}: DecisionRowProps) {
  const verdicts = [...new Set(decision.wholes)];
  const [only] = verdicts;
  const verdict =
    verdicts.length === 0
      ? p.spotsOnly
      : verdicts.length > 1
        ? p.mixed
        : only === null || only === undefined
          ? p.keepBoth
          : p.hide(only);
  const spotsOnTheirOwn = decision.wholes.length > 0 ? decision.spots.length : 0;
  return (
    <li className="patch-pair">
      <span className="patch-headline">
        <strong>
          {decision.mods[0]} ⟷ {decision.mods[1]}
        </strong>{" "}
        {verdict} · {p.objects(decision.hidden.length)}
        {decision.hidden.length > 0 && installedKnown && (
          <span className={waiting ? "patch-mark partly" : "patch-mark done"}>
            {waiting ? p.waitingMark : p.inPatch}
          </span>
        )}
        {spotsOnTheirOwn > 0 && (
          <>
            <br />
            <span className="aside">{p.ownSpots(spotsOnTheirOwn)}</span>{" "}
            <button type="button" className="small" title={p.changeHint} onClick={onChangeSpots}>
              {p.change}
            </button>
          </>
        )}
      </span>
      <button type="button" className="small" title={p.changeHint} onClick={onChange}>
        {p.change}
      </button>
      <button type="button" className="small" onClick={onUndo}>
        {p.undo}
      </button>
    </li>
  );
}

/** The objects hidden on their own, each with *show again*. */
function OwnObjects({
  own,
  p,
  onShow,
}: {
  own: ObjectRef[];
  p: PatchTexts;
  onShow: (object: ObjectRef) => void;
}) {
  return (
    <section className="patch-own">
      <h3>{p.own(own.length)}</h3>
      <p className="aside">{p.ownNote}</p>
      <ul className="patch-own-list">
        {own.map((object) => (
          <li className="patch-own-item" key={keyOf(object)}>
            <span>
              {object.plugin}{" "}
              <code>{object.localId.toString(16).padStart(6, "0").toUpperCase()}</code>
            </span>
            <button type="button" className="small" onClick={() => onShow(object)}>
              {p.showAgain}
            </button>
          </li>
        ))}
      </ul>
    </section>
  );
}
