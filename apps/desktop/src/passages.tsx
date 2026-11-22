// The passages (ADR 0072): where two mods build the same thing twice, their
// parts close to and inside each other, as the scan found them. Named by
// the place they are nearest and summed up, to be decided as a whole.

import type { MapPassage, RenderedReport } from "./generated/rendered-report";
import { InfoTip } from "./info-tip";
import { nearest } from "./map-places";
import { usePassageTexts } from "./passage-texts";
import "./passages.css";

/** A plugin's name without its extension, to name a side. */
const short = (plugin: string) => plugin.replace(/\.(esp|esm|esl)$/i, "");

/** The place a passage is nearest: its interior, or the nearest place of the map's world. */
export function passagePlace(passage: MapPassage, worlds: RenderedReport["map"]["worlds"]): string {
  const { location } = passage;
  if (location.type === "interior") {
    return location.cell;
  }
  const world =
    worlds.find((candidate) => candidate.name === location.world) ??
    worlds.find((candidate) => (candidate.children ?? []).includes(location.world));
  const near = nearest(world?.markers ?? [], passage.centre, 1)[0]?.marker;
  return near?.name || world?.title || location.world;
}

/** A passage's title: what it is, and where. */
export function usePassageTitle(): (
  passage: MapPassage,
  worlds: RenderedReport["map"]["worlds"],
) => string {
  const p = usePassageTexts();
  return (passage, worlds) => p.title(p.kind[passage.severity], passagePlace(passage, worlds));
}

/**
 * What a passage is about, in one line: what and where, its sides, its
 * twins and how far they stand in each other; why one has to go behind
 * its ⓘ, the pictures beside it show the rest (ADR 0080).
 */
export function PassageSummary({
  passage,
  worlds,
}: {
  passage: MapPassage;
  worlds: RenderedReport["map"]["worlds"];
}) {
  const p = usePassageTexts();
  const title = usePassageTitle();
  const names = [short(passage.plugins[0]), short(passage.plugins[1])] as const;
  const percent = (thousandths: number) => Math.round(thousandths / 10);
  const facts = [
    p.sides(names[0], passage.parts[0].length, names[1], passage.parts[1].length),
    p.spots(passage.spots),
    ...(passage.duplicates > 0 ? [p.twins(passage.duplicates)] : []),
    ...([0, 1] as const)
      .filter((side) => percent(passage.inside[side]) >= 5)
      .map((side) =>
        p.inside(names[side], percent(passage.inside[side]), names[side === 0 ? 1 : 0]),
      ),
  ];
  return (
    <div className="passage-summary" data-severity={passage.severity}>
      <strong className="passage-title">{title(passage, worlds)}</strong>
      <span className="aside">{facts.join(" · ")}</span>
      <InfoTip text={`${p.why[passage.severity]} ${p.bounded}`} />
    </div>
  );
}
