// The report of what was decided, per pair of mods (ADR 0050), at the end
// of the review and on the patch page, with a way to keep it as a file.

import { useMemo, useState } from "react";
import { useSession } from "./Actions";
import type { RenderedReport } from "./generated/rendered-report";
import { counted, type PatchReport, type PlaceReport, patchReport } from "./pair-report";
import { type ReportTexts, reportTexts, useReportTexts } from "./report-texts";
import "./review.css";

/** A plugin's name without its extension. */
const short = (plugin: string) => plugin.replace(/\.(esp|esm|esl)$/i, "");

/** What is decided at a place, in words. */
function stateOf(place: PlaceReport, t: ReportTexts): string {
  const { decided } = place;
  const state =
    decided.type === "open"
      ? t.placeOpen
      : decided.type === "kept"
        ? t.placeKept
        : decided.type === "hidden"
          ? t.placeHidden(short(decided.plugin))
          : t.placeMixed;
  return place.exceptions > 0 ? `${state}, ${t.exceptions(place.exceptions)}` : state;
}

/** The report as plain text, to keep as a file. */
export function reportText(data: PatchReport, t: ReportTexts, when: string): string {
  const lines: string[] = [t.fileTitle(when), "", t.intro, ""];
  if (data.pairs.length === 0) {
    lines.push(t.none);
  }
  for (const pair of data.pairs) {
    const [a, b] = pair.plugins.map(short);
    lines.push(`== ${a} <-> ${b}`, t.pairLine(pair.places.length, pair.decided, pair.open));
    const counts = [...pair.hiddenCount.entries()].map(([plugin, count]) =>
      t.hiddenCount(short(plugin), count),
    );
    lines.push(counts.length === 0 ? t.nothingHidden : counts.join(" · "));
    for (const place of pair.places) {
      lines.push(`  - ${place.console}: ${stateOf(place, t)}`);
      for (const [plugin, names] of place.hidden) {
        lines.push(`      ${short(plugin)}: ${counted(names)}`);
      }
      if (place.stays.length > 0) {
        lines.push(`      ${t.stays(place.stays.join(", "))}`);
      }
    }
    lines.push("");
  }
  if (data.elsewhere.size > 0) {
    lines.push(`== ${t.elsewhere}`);
    for (const [plugin, names] of data.elsewhere) {
      lines.push(`  ${short(plugin)}: ${counted(names)}`);
    }
    lines.push("");
  }
  if (data.leftovers.length > 0) {
    lines.push(`== ${t.leftoversTitle}`);
    for (const leftover of data.leftovers) {
      lines.push(`  ${t.leftover(leftover.mod, leftover.total, leftover.left)}`);
    }
    lines.push("");
  }
  return lines.join("\r\n");
}

/** The report's text in `lang`, for saving it next to the patch. */
export function reportFile(
  report: RenderedReport,
  ignored: Set<string>,
  choices: Parameters<typeof patchReport>[2],
  game: string,
  lang: keyof typeof reportTexts,
): string {
  return reportText(
    patchReport(report, ignored, choices, game),
    reportTexts[lang],
    new Date().toLocaleString(lang),
  );
}

/**
 * What the decisions do, pair by pair of mods: hidden with names and by
 * place, kept with the exceptions, open, and what the game needs; and the
 * mods of which little is left. Saved as a file on request and whenever
 * the patch is created.
 */
export function PairReport({ report, game }: { report: RenderedReport; game: string }) {
  const { api, ignored, patchChoices, lang } = useSession();
  const t = useReportTexts();
  const data = useMemo(
    () => patchReport(report, ignored, patchChoices, game),
    [report, ignored, patchChoices, game],
  );
  const [saved, setSaved] = useState<string | null>(null);
  const save = () => {
    api
      .savePatchReport(reportText(data, t, new Date().toLocaleString(lang)))
      .then((path) => setSaved(t.saved(path)))
      .catch((error: unknown) => setSaved(String(error)));
  };

  return (
    <section className="pair-report">
      <header className="pair-report-head">
        <h3>{t.title}</h3>
        <p className="aside">{t.intro}</p>
      </header>
      {data.pairs.length === 0 && <p className="aside">{t.none}</p>}
      {data.leftovers.length > 0 && (
        <div className="pair-report-leftovers">
          <h4>{t.leftoversTitle}</h4>
          {data.leftovers.map((leftover) => (
            <p key={leftover.mod}>{t.leftover(leftover.mod, leftover.total, leftover.left)}</p>
          ))}
        </div>
      )}
      {data.pairs.map((pair, index) => {
        const counts = [...pair.hiddenCount.entries()].map(([plugin, count]) =>
          t.hiddenCount(short(plugin), count),
        );
        return (
          <details key={pair.plugins.join("+")} className="pair-report-pair" open={index === 0}>
            <summary>
              <strong>
                {short(pair.plugins[0])} ⟷ {short(pair.plugins[1])}
              </strong>{" "}
              <span className="aside">
                {t.pairLine(pair.places.length, pair.decided, pair.open)} ·{" "}
                {counts.length === 0 ? t.nothingHidden : counts.join(" · ")}
              </span>
            </summary>
            <ul>
              {pair.places.map((place) => (
                <li key={place.console} className="pair-report-place">
                  <span>
                    <code>{place.console}</code> · {stateOf(place, t)}
                  </span>
                  {[...place.hidden.entries()].map(([plugin, names]) => (
                    <span key={plugin} className="aside">
                      {short(plugin)}: {counted(names)}
                    </span>
                  ))}
                  {place.stays.length > 0 && (
                    <span className="aside">{t.stays(place.stays.join(", "))}</span>
                  )}
                </li>
              ))}
            </ul>
          </details>
        );
      })}
      {data.elsewhere.size > 0 && (
        <details className="pair-report-pair">
          <summary>
            <strong>{t.elsewhere}</strong>
          </summary>
          <ul>
            {[...data.elsewhere.entries()].map(([plugin, names]) => (
              <li key={plugin} className="pair-report-place">
                <span>
                  {short(plugin)}: {counted(names)}
                </span>
              </li>
            ))}
          </ul>
        </details>
      )}
      <p className="pair-report-save">
        <button type="button" className="small" onClick={save}>
          {t.save}
        </button>
        <span className="aside">{saved ?? t.savedWithPatch}</span>
      </p>
    </section>
  );
}
