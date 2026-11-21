// "Repair the paths" beside a warning that characters walk on what a choice
// hides (ADR 0058). The button asks the engine how the paths would be
// repaired, shows it in the 3D view (taken out in red, laid new in green)
// and chooses the repair of each navigation mesh it changes, stored among
// the patch choices as `repair:<navmesh>`. The writer of the patch carries
// the repaired meshes; other writers ignore the prefix.

import { useEffect, useState } from "react";
import { useSession } from "./Actions";
import type { ObjectRef } from "./generated/rendered-report";
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

/** The navigation meshes whose repair is chosen. */
export function repairedNavmeshes(choices: PatchChoices): string[] {
  return Object.entries(choices)
    .filter(([key, value]) => key.startsWith(REPAIR) && value !== null)
    .map(([key]) => key.slice(REPAIR.length));
}

/** The choices with the repair of `navmeshes` chosen or not. */
export function chooseRepair(
  choices: PatchChoices,
  navmeshes: string[],
  repaired: boolean,
): PatchChoices {
  const next = { ...choices };
  for (const navmesh of navmeshes) {
    if (repaired) {
      next[repairKey(navmesh)] = "repair";
    } else {
      delete next[repairKey(navmesh)];
    }
  }
  return next;
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
          setPatchChoices(chooseRepair(patchChoices, writable, true));
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
          onClick={() => setPatchChoices(chooseRepair(patchChoices, navmeshes, !chosen))}
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
