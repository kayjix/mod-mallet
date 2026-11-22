import { useState } from "react";
import { useSession } from "./Actions";
import type { RenderedFinding, RenderedReport } from "./generated/rendered-report";
import { versionChangesTexts } from "./version-changes-texts";
import "./version-changes.css";

/** The kinds that say one script extender plugin no longer fits (ADR 0079). */
const PLUGIN_KINDS = new Set([
  "versions.plugin-wrong-game",
  "versions.plugin-needs-newer-game",
  "versions.plugin-before-anniversary",
  "versions.plugin-needs-newer-extender",
  "extender.plugin-incompatible",
]);

/** The kinds that say the script extender or its library does not fit. */
const FOUNDATION_KINDS = new Set([
  "versions.extender-lacks-game",
  "versions.address-library-lacks-game",
  "versions.address-library-missing",
]);

/** Where the change the banner was put away for is kept, per viewer. */
const DISMISSED_KEY = "mod-mallet.version-changes.dismissed";

function dismissedKey(): string | null {
  try {
    return window.localStorage.getItem(DISMISSED_KEY);
  } catch {
    return null;
  }
}

function rememberDismissed(key: string) {
  try {
    window.localStorage.setItem(DISMISSED_KEY, key);
  } catch {
    // Only a convenience: the banner comes back on the next start.
  }
}

/**
 * What a game or mod update changed since the last scan, and what of it no
 * longer fits, at the top of the overview (ADR 0079). Nothing is decided
 * here: the tasks say what to do about each plugin.
 */
export function VersionChanges({
  report,
  onTasks,
}: {
  report: RenderedReport;
  onTasks: () => void;
}) {
  const { lang } = useSession();
  const v = versionChangesTexts[lang];
  const changes = report.versionChanges;
  const key =
    changes == null
      ? null
      : `${changes.noticed}:${changes.game?.to ?? ""}:${(changes.unfit ?? []).join(",")}`;
  const [dismissed, setDismissed] = useState(dismissedKey);

  if (changes == null || key === null || dismissed === key) {
    return null;
  }

  const unfit: RenderedFinding[] = report.findings.filter((rendered) =>
    PLUGIN_KINDS.has(rendered.finding.kind),
  );
  const foundation = report.findings.filter((rendered) =>
    FOUNDATION_KINDS.has(rendered.finding.kind),
  );
  const date =
    changes.since === 0
      ? null
      : new Date(changes.since * 1000).toLocaleDateString(lang, {
          day: "numeric",
          month: "long",
        });
  const what = [
    changes.game == null ? null : v.game(changes.game.from, changes.game.to),
    changes.extender == null ? null : v.extender(changes.extender.from, changes.extender.to),
    (changes.plugins ?? []).length > 0 ? v.plugins((changes.plugins ?? []).length) : null,
    (changes.mods ?? []).length > 0 ? v.mods((changes.mods ?? []).length) : null,
  ].filter((part): part is string => part !== null);
  const broken = unfit.length > 0 || foundation.length > 0;
  const updated = [
    ...(changes.plugins ?? []).map((step) => `${step.file}: ${step.from} → ${step.to}`),
    ...(changes.mods ?? []).map((step) => `${step.mod.name}: ${step.from} → ${step.to}`),
  ];

  return (
    <section className="panel version-changes" data-tone={broken ? "error" : "ok"}>
      <h2>
        {v.since(date)}: {what.join(", ")}.
      </h2>
      {foundation.map((rendered) => (
        <p key={rendered.finding.id} className="version-changes-foundation">
          {rendered.headline}
        </p>
      ))}
      {unfit.length > 0 && (
        <>
          <p>{v.unfit(unfit.length)}</p>
          <ul className="version-changes-list">
            {unfit.map((rendered) => (
              <li key={rendered.finding.id}>
                <strong>{rendered.headline}</strong>
                {rendered.finding.provider != null && (
                  <span className="version-changes-mod"> · {rendered.finding.provider.name}</span>
                )}
              </li>
            ))}
          </ul>
        </>
      )}
      {broken ? <p className="aside">{v.waitNote}</p> : <p>{v.allFit}</p>}
      {updated.length > 0 && (
        <details>
          <summary className="more">{v.details}</summary>
          <ul className="version-changes-updated">
            {updated.map((line) => (
              <li key={line}>{line}</li>
            ))}
          </ul>
        </details>
      )}
      <div className="version-changes-actions">
        {broken && (
          <button type="button" className="primary" onClick={onTasks}>
            {v.toTasks}
          </button>
        )}
        <button
          type="button"
          onClick={() => {
            rememberDismissed(key);
            setDismissed(key);
          }}
        >
          {v.dismiss}
        </button>
      </div>
    </section>
  );
}
