import { useMemo, useState } from "react";
import { ActionResult, useAction, useSession } from "./Actions";
import type { RenderedReport } from "./generated/rendered-report";
import { choicesOf, keyOf, objectsOf, type Pair, pairsOf, signatureOf } from "./patch";

/** Base Object Swapper's page, for when it is missing. */
const SWAPPER_PAGE = "https://www.nexusmods.com/skyrimspecialedition/mods/60805";

/**
 * A patch made for this setup: for each collision, the user picks which
 * mod's objects disappear, creates the patch, and has Vortex install it as
 * a small mod. Nothing is chosen until the user chooses it.
 */
export function PatchView({ report }: { report: RenderedReport }) {
  const { t, api, ignored, patchChoices: choices, setPatchChoices: setChoices } = useSession();
  const pairs = useMemo(() => pairsOf(report, ignored), [report, ignored]);
  const installedObjects = report.installedPatch?.objects ?? null;
  const installed = useMemo(() => new Set((installedObjects ?? []).map(keyOf)), [installedObjects]);
  // Objects of the installed patch that no collision of this scan contains,
  // set aside or not: their collision is gone.
  const obsolete = useMemo(() => {
    const inCollisions = new Set(
      report.map.spots.flatMap((spot) => (spot.parts ?? []).map((part) => keyOf(part.object))),
    );
    return [...installed].filter((key) => !inCollisions.has(key)).length;
  }, [report, installed]);
  const [created, setCreated] = useState<{ path: string; signature: string } | null>(null);
  const [creating, setCreating] = useState(false);
  const [createError, setCreateError] = useState<string | null>(null);
  const action = useAction();

  const choice = (pair: Pair) => choices[pair.id] ?? null;
  const objects = objectsOf(pairs, choices);
  const signature = signatureOf(objects);
  const upToDate = created !== null && created.signature === signature;
  const chosen = new Set(objects.map(keyOf));
  const more = [...chosen].filter((key) => !installed.has(key)).length;
  const fewer = [...installed].filter((key) => !chosen.has(key)).length;
  const fromInstalled = useMemo(() => choicesOf(pairs, installed), [pairs, installed]);
  const sameAsInstalled = pairs.every((pair) => choice(pair) === (fromInstalled[pair.id] ?? null));

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
        <p>{t.patchNeedsSwapper}</p>
        <button type="button" onClick={() => api.openPage(SWAPPER_PAGE)}>
          {t.openPage}
        </button>
      </section>
    );
  }
  if (pairs.length === 0) {
    return <p className="empty">{t.nothingHere}</p>;
  }
  return (
    <section className="patch-view">
      <div className="patch-bar">
        <strong>{t.patchCount(chosen.size)}</strong>
        <button
          type="button"
          onClick={() =>
            setChoices(Object.fromEntries(pairs.map((pair) => [pair.id, pair.suggested])))
          }
        >
          {t.patchSuggest}
        </button>
        <button type="button" disabled={objects.length === 0} onClick={() => setChoices({})}>
          {t.patchClear}
        </button>
        <span className="spacer" />
        <button type="button" disabled={objects.length === 0 || creating} onClick={create}>
          {t.patchCreate}
        </button>
        <button
          type="button"
          className="primary"
          disabled={!upToDate || action.busy}
          title={t.patchInstallHint}
          onClick={() => action.run({ type: "hide-objects", objects })}
        >
          {t.patchInstall}
        </button>
      </div>
      {installedObjects !== null && (
        <div className={more + fewer === 0 ? "patch-installed" : "patch-installed differs"}>
          <p className="patch-line">
            <strong>
              {t.patchInstalled(
                installedObjects.length,
                report.installedPatch?.provider?.name ?? null,
              )}
            </strong>{" "}
            {more + fewer === 0 ? t.patchMatches : t.patchDiffers(more, fewer)}
          </p>
          {obsolete > 0 && <p className="patch-line">{t.patchObsolete(obsolete)}</p>}
          {!sameAsInstalled && (
            <button type="button" onClick={() => setChoices(fromInstalled)}>
              {t.patchUseInstalled}
            </button>
          )}
        </div>
      )}
      {created && (
        <div className={upToDate ? "patch-file" : "patch-file stale"}>
          <span>
            {upToDate ? t.patchCreated : t.patchStale} {upToDate && <code>{created.path}</code>}
          </span>
          <button type="button" onClick={() => api.openFolder("patches")}>
            {t.openFolder}
          </button>
        </div>
      )}
      {createError && <p className="failure">{createError}</p>}
      <ActionResult state={action} />
      <ul className="patch-list">
        {pairs.map((pair) => (
          <li className="patch-pair" key={pair.id}>
            <span className="patch-headline">
              {pair.headline}
              {installedObjects !== null && <InstalledMark pair={pair} installed={installed} />}
            </span>
            <fieldset className="segmented" aria-label={pair.headline}>
              {pair.sides.map((side) => (
                <button
                  type="button"
                  key={side.plugin}
                  aria-pressed={choice(pair) === side.plugin}
                  disabled={side.objects.length === 0}
                  onClick={() => setChoices({ ...choices, [pair.id]: side.plugin })}
                >
                  {t.patchHide(side.plugin, side.objects.length)}
                  {pair.suggested === side.plugin && (
                    <span className="suggested-mark">{t.patchSuggested}</span>
                  )}
                </button>
              ))}
              <button
                type="button"
                aria-pressed={choice(pair) === null}
                onClick={() => setChoices({ ...choices, [pair.id]: null })}
              >
                {t.patchKeepBoth}
              </button>
            </fieldset>
          </li>
        ))}
      </ul>
    </section>
  );
}

/** How much of a collision the installed patch already hides. */
function InstalledMark({ pair, installed }: { pair: Pair; installed: Set<string> }) {
  const { t } = useSession();
  const counts = pair.sides.map((side) => ({
    hidden: side.objects.filter((object) => installed.has(keyOf(object))).length,
    of: side.objects.length,
  }));
  const best = counts.reduce((a, b) => (b.hidden > a.hidden ? b : a), { hidden: 0, of: 0 });
  if (best.hidden === 0) {
    return <span className="patch-mark new">{t.patchNew}</span>;
  }
  if (best.hidden === best.of) {
    return <span className="patch-mark done">{t.patchInPatch}</span>;
  }
  return <span className="patch-mark partly">{t.patchPartly(best.hidden, best.of)}</span>;
}
