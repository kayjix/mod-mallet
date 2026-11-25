// The report at the end of "Step by step" (ADR 0083): what it all means.
// The setup in four numbers, what the patch does pair by pair of mods with
// the way to each place, what is still open, what to try in the game, and
// the full report with every part's name below.

import { useState } from "react";
import { useDecisions, useSession } from "./Actions";
import { objectKey } from "./area-layer";
import type { RenderedReport } from "./generated/rendered-report";
import { PairReport } from "./PairReport";
import { passagePlace } from "./passages";
import { pinsOf } from "./patch";
import { PinIcon } from "./pins";
import { type Cluster, decidedOf, openIn } from "./review-clusters";
import { useReviewStepTexts } from "./review-step-texts";
import { managerOf } from "./review-steps";
import type { ReviewStep } from "./review-texts";
import { Chip, Note } from "./ui/kit";

/** How many places a list of the report shows before it says how many more. */
const SHOWN = 8;

/** A plugin's name without its extension. */
const short = (plugin: string) => plugin.replace(/\.(esp|esm|esl)$/i, "");

/** One place as the report tells it. */
interface PlaceLine {
  key: string;
  name: string;
  /** Whether the name is a place's (a passage's), not only where it is. */
  named: boolean;
  console: string;
  /** The side kept, both kept, or not decided (yet, or alike). */
  kept: 0 | 1 | "both" | null;
  open: boolean;
  /** The parts hidden there, per side. */
  hidden: [number, number];
}

/** The places of one pair of mods. */
interface PairLine {
  plugins: [string, string];
  places: PlaceLine[];
}

export function ReportStep({
  report,
  game,
  clusters,
  open,
  onCluster,
  onStep,
}: {
  report: RenderedReport;
  game: string;
  clusters: Cluster[];
  /** What is open in each step, if it counts. */
  open: Record<ReviewStep, number | null>;
  onCluster: (key: string) => void;
  onStep: (step: ReviewStep) => void;
}) {
  const s = useReviewStepTexts();
  const { patchChoices, showPinned } = useSession();
  const { decisions } = useDecisions();
  const manager = managerOf(report);
  const hidden = decisions.hiddenKeys;
  const worlds = report.map.worlds;

  // Place by place, pair by pair, in the order of the collisions step.
  const pairs = new Map<string, PairLine>();
  for (const cluster of clusters) {
    const first = cluster.spots[0]?.item.spot;
    if (first === undefined) {
      continue;
    }
    const decided = decidedOf(cluster.spots, patchChoices);
    const sides = cluster.plugins.map((plugin) => plugin.toLowerCase());
    const keys = new Set([
      ...cluster.spots.flatMap(({ item }) =>
        (item.spot.parts ?? []).map((part) => objectKey(part.object.plugin, part.object.localId)),
      ),
      ...(cluster.passage?.keys ?? []),
    ]);
    const count: [number, number] = [0, 0];
    for (const key of keys) {
      if (hidden.has(key)) {
        const plugin = key.slice(0, key.lastIndexOf(":"));
        const side = sides.indexOf(plugin);
        if (side === 0 || side === 1) {
          count[side] += 1;
        }
      }
    }
    const kept: PlaceLine["kept"] =
      decided.type === "kept"
        ? "both"
        : decided.type === "hidden"
          ? decided.plugin.toLowerCase() === sides[1]
            ? 0
            : 1
          : null;
    const pairKey = [...sides].sort().join("+");
    const pair = pairs.get(pairKey) ?? { plugins: cluster.plugins, places: [] };
    // The pair's sides keep their first order; a place's counts follow it.
    const flip = pair.plugins[0].toLowerCase() !== sides[0];
    pair.places.push({
      key: cluster.key,
      name:
        cluster.passage === undefined
          ? first.console
          : passagePlace(cluster.passage.passage, worlds),
      named: cluster.passage !== undefined,
      console: first.console,
      kept: flip && (kept === 0 || kept === 1) ? (kept === 0 ? 1 : 0) : kept,
      open: openIn(cluster) > 0,
      hidden: flip ? [count[1], count[0]] : count,
    });
    pairs.set(pairKey, pair);
  }
  const decidedPlaces = clusters.filter((cluster) => openIn(cluster) === 0).length;
  const duels = report.duels.length;
  const openDuels = open.assets ?? 0;
  const hiddenTotal = hidden.size;
  const openPlaces = [...pairs.values()].flatMap((pair) =>
    pair.places.filter((place) => place.open).map((place) => ({ pair, place })),
  );
  const tried = [...pairs.values()]
    .flatMap((pair) =>
      pair.places
        .filter((place) => !place.open && place.kept !== null)
        .map((place) => ({ pair, place })),
    )
    .slice(0, SHOWN);
  const pinned = pinsOf(patchChoices).size;
  const install = open.install ?? 0;
  const pairName = (pair: PairLine) => `${short(pair.plugins[0])} ⟷ ${short(pair.plugins[1])}`;

  return (
    <div className="review-report">
      <div className="review-tiles">
        <ReportTile
          value={s.decidedOf(decidedPlaces, clusters.length)}
          label={s.tileCollisions}
          done={decidedPlaces === clusters.length}
          onClick={() => onStep("collisions")}
        />
        <ReportTile
          value={s.decidedOf(duels - openDuels, duels)}
          label={s.tileAssets}
          done={openDuels === 0}
          onClick={() => onStep("assets")}
        />
        <ReportTile
          value={String(install)}
          label={s.tileInstall}
          done={install === 0}
          onClick={() => onStep("install")}
        />
        <ReportTile value={String(hiddenTotal)} label={s.tileHidden} />
      </div>

      <section className="review-report-part">
        <h4>{s.patchTitle}</h4>
        <p className="review-purpose">{s.patchPurpose(manager)}</p>
        {[...pairs.values()].every((pair) => pair.places.every((place) => place.kept === null)) ? (
          <p className="aside">{s.nothingDecided}</p>
        ) : (
          <ul className="review-report-pairs">
            {[...pairs.values()]
              .filter((pair) => pair.places.some((place) => place.kept !== null))
              .map((pair) => {
                const keptBy = (side: 0 | 1) =>
                  pair.places.filter((place) => place.kept === side).length;
                const both = pair.places.filter((place) => place.kept === "both").length;
                const hiddenBy = (side: 0 | 1) =>
                  pair.places.reduce((sum, place) => sum + place.hidden[side], 0);
                return (
                  <li key={pair.plugins.join("+")}>
                    <details className="review-report-pair">
                      <summary>
                        <strong>{pairName(pair)}</strong>
                        <span className="aside">
                          {s.pairLine(
                            pair.places.length,
                            pair.places.filter((place) => !place.open).length,
                          )}
                        </span>
                        {([0, 1] as const)
                          .filter((side) => keptBy(side) > 0)
                          .map((side) => (
                            <Chip key={side} tone="ok">
                              {s.keptAt(short(pair.plugins[side]), keptBy(side))}
                            </Chip>
                          ))}
                        {both > 0 && <Chip>{s.bothAt(both)}</Chip>}
                        {([0, 1] as const)
                          .filter((side) => hiddenBy(side) > 0)
                          .map((side) => (
                            <span key={`h${side}`} className="aside">
                              {s.hiddenParts(short(pair.plugins[side]), hiddenBy(side))}
                            </span>
                          ))}
                      </summary>
                      <ul className="review-report-places">
                        {pair.places.map((place) => (
                          <li key={place.key}>
                            <span>{place.name}</span>
                            <span className="review-state" data-decided={!place.open}>
                              {place.kept === "both"
                                ? s.bothKept
                                : place.kept === null
                                  ? place.open
                                    ? s.openTitle
                                    : "✓"
                                  : `✓ ${short(pair.plugins[place.kept])}`}
                            </span>
                            <span className="spacer" />
                            <button
                              type="button"
                              className="small quiet"
                              onClick={() => onCluster(place.key)}
                            >
                              {s.goThere} →
                            </button>
                          </li>
                        ))}
                      </ul>
                    </details>
                  </li>
                );
              })}
          </ul>
        )}
      </section>

      <section className="review-report-part">
        <h4>{s.openTitle}</h4>
        {openPlaces.length + openDuels + install + pinned === 0 ? (
          <Note tone="ok" compact>
            <p>{s.nothingOpen}</p>
          </Note>
        ) : (
          <ul className="review-report-open">
            {openPlaces.length > 0 && (
              <li>
                <strong>{s.openPlaces(openPlaces.length)}</strong>
                <span className="review-report-chips">
                  {openPlaces.slice(0, SHOWN).map(({ pair, place }) => (
                    <button
                      type="button"
                      key={place.key}
                      className="small"
                      title={pairName(pair)}
                      onClick={() => onCluster(place.key)}
                    >
                      {place.name}
                    </button>
                  ))}
                  {openPlaces.length > SHOWN && (
                    <span className="aside">+{openPlaces.length - SHOWN}</span>
                  )}
                </span>
              </li>
            )}
            {openDuels > 0 && (
              <li>
                <strong>{s.openAssetDuels(openDuels)}</strong>
                <span className="spacer" />
                <button type="button" className="small quiet" onClick={() => onStep("assets")}>
                  {s.goThere} →
                </button>
              </li>
            )}
            {install > 0 && (
              <li>
                <strong>{s.openInstall(install)}</strong>
                <span className="spacer" />
                <button type="button" className="small quiet" onClick={() => onStep("install")}>
                  {s.goThere} →
                </button>
              </li>
            )}
            {pinned > 0 && (
              <li>
                <strong>
                  <PinIcon /> {s.pinned(pinned)}
                </strong>
                <span className="spacer" />
                <button type="button" className="small quiet" onClick={showPinned}>
                  {s.goThere} →
                </button>
              </li>
            )}
          </ul>
        )}
      </section>

      <section className="review-report-part">
        <h4>{s.tryTitle}</h4>
        <ol className="review-report-try">
          <li>{s.tryPatch(manager)}</li>
          <li>{s.tryStart}</li>
          {tried.map(({ pair, place }) => {
            const where = place.named ? place.name : pairName(pair);
            return (
              <li key={place.key}>
                <span>
                  {place.kept === "both"
                    ? s.tryBoth(where)
                    : place.kept === null
                      ? where
                      : s.tryPlace(where, short(pair.plugins[place.kept]))}
                </span>
                <ConsoleCommand command={place.console} />
              </li>
            );
          })}
        </ol>
        {tried.length > 0 && <p className="aside">{s.tryConsole}</p>}
      </section>

      <details className="review-report-full">
        <summary className="more">{s.fullReport}</summary>
        <PairReport report={report} game={game} />
      </details>
    </div>
  );
}

/** A number of the report and what it counts, done or not; leads to its step. */
function ReportTile({
  value,
  label,
  done,
  onClick,
}: {
  value: string;
  label: string;
  done?: boolean;
  onClick?: () => void;
}) {
  const tone = done === undefined ? undefined : done ? "ok" : "warning";
  const body = (
    <>
      <strong className="review-tile-value">
        {done === true && "✓ "}
        {value}
      </strong>
      <span className="review-tile-label">{label}</span>
    </>
  );
  return onClick === undefined ? (
    <div className="review-tile" data-tone={tone}>
      {body}
    </div>
  ) : (
    <button type="button" className="review-tile" data-tone={tone} onClick={onClick}>
      {body}
    </button>
  );
}

/** A command for the game's console, with a button that copies it. */
function ConsoleCommand({ command }: { command: string }) {
  const s = useReviewStepTexts();
  const [copied, setCopied] = useState(false);
  return (
    <span className="review-console">
      <code>{command}</code>
      <button
        type="button"
        className="small quiet"
        onClick={() => {
          navigator.clipboard
            ?.writeText(command)
            .then(() => setCopied(true))
            .catch(() => {});
        }}
      >
        {copied ? s.copied : s.copy}
      </button>
    </span>
  );
}
