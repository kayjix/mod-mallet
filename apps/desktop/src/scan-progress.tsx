import { useEffect, useState } from "react";
import type { Api } from "./api";
import type { ScanProgress } from "./generated/scan-progress";
import type { Lang } from "./i18n";
import "./charm.css";
import { scanTexts } from "./scan-texts";

/**
 * Where the running scan stands: the last report of the engine, or `null`
 * while no scan runs or before the first report.
 */
export function useScanProgress(api: Api, scanning: boolean): ScanProgress | null {
  const [progress, setProgress] = useState<ScanProgress | null>(null);
  useEffect(() => {
    if (!scanning) {
      setProgress(null);
      return;
    }
    return api.onScanProgress(setProgress);
  }, [api, scanning]);
  return progress;
}

/** What the step reads and how far: "Reading plugins 37 of 398". */
export function scanStepLine(progress: ScanProgress, lang: Lang): string {
  const texts = scanTexts[lang];
  const name = texts.step[progress.step];
  return progress.of > 0 ? `${name} ${texts.count(progress.done, progress.of)}` : name;
}

/** The step, a bar by the step's own counts, and which step of how many. */
export function ScanProgressView({ progress, lang }: { progress: ScanProgress; lang: Lang }) {
  const texts = scanTexts[lang];
  const counted = progress.of > 0;
  return (
    <div className="scan-progress" role="status">
      <span className="scan-progress-step">{scanStepLine(progress, lang)}</span>
      <span
        className="scan-progress-bar"
        data-counted={counted}
        role="progressbar"
        aria-valuemin={0}
        aria-valuemax={counted ? progress.of : undefined}
        aria-valuenow={counted ? progress.done : undefined}
      >
        <span
          className="scan-progress-fill"
          style={
            counted ? { width: `${Math.round((progress.done / progress.of) * 100)}%` } : undefined
          }
        />
      </span>
      <span className="scan-progress-of">{texts.stepOf(progress.stepNumber, progress.steps)}</span>
    </div>
  );
}
