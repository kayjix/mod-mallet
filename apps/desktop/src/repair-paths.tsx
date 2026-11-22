// "Repair the paths" beside a warning that characters walk on what a choice
// hides (ADR 0058). The button asks the engine how the paths would be
// repaired, shows it in the 3D view (taken out in red, laid new in green)
// and chooses the repair of each navigation mesh it changes, stored among
// the patch choices as `repair:<navmesh>`. The writer of the patch carries
// the repaired meshes; other writers ignore the prefix.

import { useEffect, useState } from "react";
import { ActionResult, useAction, useSession } from "./Actions";
import type { CreatedPaths } from "./generated/created-paths";
import type { ObjectRef, PathRepairs, RepairPlace } from "./generated/rendered-report";
import type { ReachChange, WalkwayReach } from "./generated/walkway-reach";
import type { WalkwayRepair } from "./generated/walkway-repair";
import type { PatchChoices } from "./patch";
import type { PathsOverlay } from "./paths-layer";
import { ReachNote, useReach } from "./reach";
import { reachTexts } from "./reach-texts";
import { repairTexts } from "./repair-paths-texts";
import "./repair-paths.css";

/** The prefix of a repair among the patch choices. */
export const REPAIR = "repair:";

/** A navigation mesh's repair among the patch choices. */
export const repairKey = (navmesh: string) => `${REPAIR}${navmesh}`;

/**
 * Where a repair was asked for, kept as the choice's value: the place and,
 * outside, a point near the objects, so the plugin of paths can repair it
 * again with every object the patch hides there.
 */
interface RepairAt {
  place: string;
  x: number | null;
  y: number | null;
}

/** Where a chosen repair was asked for; null for a value from elsewhere. */
function repairAt(value: string | null): RepairAt | null {
  if (value === null) {
    return null;
  }
  try {
    const parsed = JSON.parse(value) as Partial<RepairAt>;
    return typeof parsed.place === "string"
      ? {
          place: parsed.place,
          x: typeof parsed.x === "number" ? parsed.x : null,
          y: typeof parsed.y === "number" ? parsed.y : null,
        }
      : null;
  } catch {
    return null;
  }
}

/** The navigation meshes whose repair is chosen. */
export function repairedNavmeshes(choices: PatchChoices): string[] {
  return Object.entries(choices)
    .filter(([key, value]) => key.startsWith(REPAIR) && value !== null)
    .map(([key]) => key.slice(REPAIR.length));
}

/** The choices with the repair of `navmeshes`, asked for at `at`, chosen or not. */
export function chooseRepair(
  choices: PatchChoices,
  navmeshes: string[],
  at: RepairAt | null,
): PatchChoices {
  const next = { ...choices };
  for (const navmesh of navmeshes) {
    if (at !== null) {
      next[repairKey(navmesh)] = JSON.stringify(at);
    } else {
      delete next[repairKey(navmesh)];
    }
  }
  return next;
}

/** What the plugin of paths is to repair: the chosen meshes by place, and every object hidden. */
export function repairsOf(choices: PatchChoices, hidden: ObjectRef[]): PathRepairs {
  const places = new Map<string, RepairPlace>();
  for (const [key, value] of Object.entries(choices)) {
    const at = key.startsWith(REPAIR) ? repairAt(value) : null;
    if (at === null) {
      continue;
    }
    const centre: [number, number] | null =
      at.x === null || at.y === null ? null : [Math.round(at.x), Math.round(at.y)];
    const name = `${at.place}|${centre?.join(",") ?? "inside"}`;
    const place = places.get(name) ?? { place: at.place, centre, navmeshes: [] };
    place.navmeshes.push(key.slice(REPAIR.length));
    places.set(name, place);
  }
  return { places: [...places.values()], hidden };
}

/** What the 3D view draws for a repair. */
export function overlayOf(repair: WalkwayRepair): PathsOverlay {
  return {
    removed: repair.navmeshes.flatMap((mesh) => mesh.removed),
    added: repair.navmeshes.flatMap((mesh) => mesh.added),
  };
}

interface RepairPathsProps {
  /** The worldspace or interior cell the objects stand in. */
  place: string;
  /** Outside, a point near them; inside, null. */
  centre: [number, number] | null;
  /** The objects the choice would hide. */
  objects: ObjectRef[];
  /** Told what to draw in the 3D view, or null to draw nothing. */
  onShow: (overlay: PathsOverlay | null) => void;
}

/** The button, and once pressed, what the repair does. */
export function RepairPaths({ place, centre, objects, onShow }: RepairPathsProps) {
  const { api, lang, patchChoices, setPatchChoices } = useSession();
  const t = repairTexts[lang];
  const [state, setState] = useState<"idle" | "working" | "failed" | WalkwayRepair>("idle");
  const signature = `${place}|${objects.map((object) => `${object.plugin}:${object.localId}`).join(",")}`;
  const [shownFor, setShownFor] = useState(signature);
  // Other objects: start again, and draw nothing.
  useEffect(() => {
    if (shownFor !== signature) {
      setShownFor(signature);
      setState("idle");
      onShow(null);
    }
  }, [shownFor, signature, onShow]);
  useEffect(() => () => onShow(null), [onShow]);

  const repair = typeof state === "object" ? state : null;
  const navmeshes =
    repair?.navmeshes.filter((mesh) => mesh.writable).map((mesh) => mesh.navmesh) ?? [];
  const chosenBefore = new Set(repairedNavmeshes(patchChoices));
  const chosen = navmeshes.length > 0 && navmeshes.every((navmesh) => chosenBefore.has(navmesh));
  const here: RepairAt = { place, x: centre?.[0] ?? null, y: centre?.[1] ?? null };

  const ask = () => {
    setState("working");
    api
      .walkwayRepair(place, centre?.[0] ?? null, centre?.[1] ?? null, objects)
      .then((found) => {
        if (found === null) {
          setState("failed");
          onShow(null);
          return;
        }
        setState(found);
        onShow(overlayOf(found));
        const writable = found.navmeshes
          .filter((mesh) => mesh.writable)
          .map((mesh) => mesh.navmesh);
        if (writable.length > 0) {
          setPatchChoices(chooseRepair(patchChoices, writable, here));
        }
      })
      .catch(() => {
        setState("failed");
        onShow(null);
      });
  };

  if (repair === null) {
    return (
      <div className="repair-paths">
        <div className="repair-paths-row">
          <button
            type="button"
            className="small"
            disabled={state === "working" || objects.length === 0}
            onClick={ask}
          >
            {state === "working" ? t.working : t.repair}
          </button>
        </div>
        {state === "failed" && <p className="aside">{t.failed}</p>}
      </div>
    );
  }
  const removed = repair.navmeshes.reduce((sum, mesh) => sum + mesh.removed.length, 0);
  const added = repair.navmeshes.reduce((sum, mesh) => sum + mesh.added.length, 0);
  const water = repair.navmeshes.reduce((sum, mesh) => sum + mesh.water, 0);
  return (
    <div className="repair-paths">
      <div className="repair-paths-row">
        <button
          type="button"
          className="small"
          aria-pressed={chosen}
          onClick={() =>
            setPatchChoices(chooseRepair(patchChoices, navmeshes, chosen ? null : here))
          }
        >
          {chosen ? t.repaired : t.repair}
        </button>
        <span className="repair-legend" data-kind="removed">
          {t.legendRemoved}
        </span>
        <span className="repair-legend" data-kind="added">
          {t.legendAdded}
        </span>
      </div>
      <p>{t.summary(removed, added)}</p>
      {repair.bareObjects > 0 && added > 0 && (
        <p className="aside">{t.around(repair.bareObjects)}</p>
      )}
      {water > 0 && <p className="aside">{t.water(water)}</p>}
      {repair.lostDoors.length > 0 && (
        <p className="aside">{t.lostDoors(repair.lostDoors.length)}</p>
      )}
      {repair.navmeshes.some((mesh) => !mesh.writable) && <p className="aside">{t.notWritable}</p>}
      <RepairedReach place={place} centre={centre} objects={objects} />
      {chosen && <p className="aside">{t.chosen}</p>}
    </div>
  );
}

/** A destination's identity across two answers. */
const destinationKey = (change: ReachChange) =>
  `${change.destination.object.plugin.toLowerCase()}:${change.destination.object.localId}`;

/** Whether no way leads to it any more. */
const lost = (change: ReachChange) => change.outcome !== "longer";

/**
 * Where characters go once the paths are repaired (ADR 0059): what the
 * repair makes reachable again of what cutting the paths would lose, and
 * what stays lost, such as a door that stood on a hidden deck.
 */
function RepairedReach({ place, centre, objects }: Omit<RepairPathsProps, "onShow">) {
  const { api, lang } = useSession();
  const t = repairTexts[lang];
  const r = reachTexts[lang];
  const cut = useReach(place, centre, objects);
  const [repaired, setRepaired] = useState<WalkwayReach | null | undefined>(undefined);
  const key = `${place}|${objects.map((object) => `${object.plugin}:${object.localId}`).join(",")}`;
  // biome-ignore lint/correctness/useExhaustiveDependencies: the objects are compared by `key`.
  useEffect(() => {
    let outdated = false;
    setRepaired(undefined);
    api
      .walkwayRepairReach(place, centre?.[0] ?? null, centre?.[1] ?? null, objects)
      .then((found) => {
        if (!outdated) {
          setRepaired(found);
        }
      })
      .catch(() => {
        if (!outdated) {
          setRepaired(null);
        }
      });
    return () => {
      outdated = true;
    };
  }, [api, key]);
  if (repaired === undefined) {
    return <p className="aside">{t.checkingReach}</p>;
  }
  if (repaired === null) {
    return null;
  }
  const stillLost = new Set(repaired.changes.filter(lost).map(destinationKey));
  const again = (cut?.changes ?? [])
    .filter((change) => lost(change) && !stillLost.has(destinationKey(change)))
    .map((change) => r.what(change.destination));
  return (
    <>
      {again.length > 0 && <p className="repair-again">{t.reachedAgain([...new Set(again)])}</p>}
      <ReachNote reach={repaired} />
    </>
  );
}

/**
 * On the patch page: the paths chosen for repair, and the plugin that
 * carries them into the game (ADR 0058). `hidden` is every object the
 * patch hides: each place is repaired with all of those that stand there.
 */
export function PathsPanel({ hidden }: { hidden: ObjectRef[] }) {
  const { t: app, api, lang, patchChoices: choices, setPatchChoices } = useSession();
  const t = repairTexts[lang];
  const action = useAction();
  const navmeshes = repairedNavmeshes(choices);
  const repairs = repairsOf(choices, hidden);
  const signature = JSON.stringify(repairs);
  const [written, setWritten] = useState<{ result: CreatedPaths; signature: string } | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  if (navmeshes.length === 0 && written === null) {
    return null;
  }
  const upToDate = written !== null && written.signature === signature;
  const create = async () => {
    setBusy(true);
    setError(null);
    try {
      setWritten({ result: await api.createPaths(repairs), signature });
    } catch (failure: unknown) {
      setError(String(failure));
    } finally {
      setBusy(false);
    }
  };
  const reason = (refused: CreatedPaths["refused"][number]) =>
    refused.reason.type === "unknown-field"
      ? t.reasons["unknown-field"](refused.reason.field)
      : t.reasons[refused.reason.type];
  return (
    <section className="moves-panel">
      <h3>{t.panelTitle}</h3>
      <div className="patch-bar">
        <strong>{t.panelSummary(navmeshes.length)}</strong>
        <span className="spacer" />
        <button type="button" disabled={repairs.places.length === 0 || busy} onClick={create}>
          {busy ? t.writing : t.write}
        </button>
        <button
          type="button"
          className="primary"
          disabled={!upToDate || action.busy || (written?.result.navmeshes.length ?? 0) === 0}
          title={t.installHint}
          onClick={() => action.run({ type: "repair-paths", repairs })}
        >
          {t.install}
        </button>
        <button
          type="button"
          disabled={navmeshes.length === 0}
          onClick={() => setPatchChoices(chooseRepair(choices, navmeshes, null))}
        >
          {t.dropAll}
        </button>
      </div>
      <ul className="moves-list">
        {navmeshes.map((navmesh) => (
          <li key={navmesh}>
            <code>{navmesh}</code>
          </li>
        ))}
      </ul>
      {written !== null && (
        <div className={upToDate ? "patch-file" : "patch-file stale"}>
          {written.result.navmeshes.length === 0 ? (
            <span>{t.nothingWritten}</span>
          ) : (
            <span>
              {upToDate
                ? t.written(written.result.navmeshes.length, written.result.masters.join(", "))
                : t.outdated}{" "}
              {upToDate && <code>{written.result.path}</code>}
            </span>
          )}
          <button type="button" onClick={() => api.openFolder("patches")}>
            {app.openFolder}
          </button>
        </div>
      )}
      {written !== null && written.result.refused.length > 0 && (
        <div className="moves-refused">
          <p>{t.refusedTitle(written.result.refused.length)}</p>
          <ul>
            {written.result.refused.map((refused) => (
              <li key={refused.navmesh}>
                <code>{refused.navmesh}</code>: {reason(refused)}
              </li>
            ))}
          </ul>
        </div>
      )}
      {error !== null && <p className="failure">{error}</p>}
      <ActionResult state={action} />
      <p className="aside">{t.limits}</p>
    </section>
  );
}
