// The suggestion about a mod that collides with very many others (ADR
// 0043), where the user works: on the collisions' page when it is about
// that mod, on the mods page, and among the recommendations of the tasks.
//
// Loud for a mod whose places are hard decisions: think about removing it,
// with what relies on it first and a button that switches it off in Vortex
// on a click (ADR 0061). Quieter for a mod most of whose places are clear,
// where one object is far larger than the other: take every clear
// suggestion instead. Quietest for a mod with rooms of its own: keep it.

import { useState } from "react";
import "./wide-advice.css";
import { ActionResult, useAction, useSession } from "./Actions";
import type { Action, RenderedFinding } from "./generated/rendered-report";
import { wideTexts } from "./switched-off-texts";

/** How the suggestion is put. */
export type WideTone = "remove" | "bulk" | "keep";

/** The kind of the suggestion. */
export const COLLIDES_WIDELY = "mods.collides-widely";

const numberOf = (rendered: RenderedFinding, name: string) => {
  const value = rendered.finding.args[name];
  return typeof value === "number" ? value : 0;
};

const textOf = (rendered: RenderedFinding, name: string) => {
  const value = rendered.finding.args[name];
  return typeof value === "string" ? value : "";
};

/**
 * How loud the suggestion is: a mod with rooms of its own is kept; one at
 * least half of whose places are clear is decided in bulk; the rest are
 * candidates to remove.
 */
export function toneOf(rendered: RenderedFinding): WideTone {
  if (numberOf(rendered, "interiors") > 0) {
    return "keep";
  }
  const places = numberOf(rendered, "count");
  return places > 0 && 2 * numberOf(rendered, "clearCount") >= places ? "bulk" : "remove";
}

/** The suggestions about mods that collide widely, the loud ones first. */
export function wideFindings(findings: RenderedFinding[], ignored: Set<string>): RenderedFinding[] {
  const rank: Record<WideTone, number> = { remove: 0, bulk: 1, keep: 2 };
  return findings
    .filter(
      (rendered) => rendered.finding.kind === COLLIDES_WIDELY && !ignored.has(rendered.finding.id),
    )
    .sort(
      (a, b) => rank[toneOf(a)] - rank[toneOf(b)] || numberOf(b, "count") - numberOf(a, "count"),
    );
}

/** The suggestion about the mod named `mod`, if there is one. */
export function wideFor(
  findings: RenderedFinding[],
  ignored: Set<string>,
  mod: string,
): RenderedFinding | undefined {
  const wanted = mod.toLowerCase();
  return wideFindings(findings, ignored).find(
    (rendered) => textOf(rendered, "mod").toLowerCase() === wanted,
  );
}

/** The switch-off action a suggestion offers, if any. */
function switchOffOf(rendered: RenderedFinding): Action | undefined {
  return rendered.actions.find((available) => available.action.type === "disable-mod")?.action;
}

/**
 * The button that switches a mod off in Vortex, on a click; first a
 * question listing the plugins it would leave without their master.
 */
export function SwitchOffButton({ rendered }: { rendered: RenderedFinding }) {
  const { lang } = useSession();
  const w = wideTexts[lang];
  const action = useAction();
  const [asking, setAsking] = useState(false);
  const switchOff = switchOffOf(rendered);
  if (switchOff === undefined) {
    return null;
  }
  const mod = textOf(rendered, "mod");
  const dependents = textOf(rendered, "dependents");
  const needed = numberOf(rendered, "dependentCount") > 0;
  return (
    <div className="wide-switch">
      {asking ? (
        <div className="wide-confirm" role="alertdialog">
          <p>{w.confirm(mod, dependents)}</p>
          <span className="wide-buttons">
            <button
              type="button"
              className="primary"
              disabled={action.busy}
              onClick={() => {
                setAsking(false);
                void action.run(switchOff);
              }}
            >
              {w.confirmYes}
            </button>
            <button type="button" onClick={() => setAsking(false)}>
              {w.cancel}
            </button>
          </span>
        </div>
      ) : (
        <button
          type="button"
          disabled={action.busy}
          onClick={() => (needed ? setAsking(true) : void action.run(switchOff))}
        >
          {w.switchOff}
        </button>
      )}
      <ActionResult state={action} />
    </div>
  );
}

/** What relies on the mod: the plugins of other mods, the models others show. */
function Reliance({ rendered }: { rendered: RenderedFinding }) {
  const { lang } = useSession();
  const w = wideTexts[lang];
  const mod = textOf(rendered, "mod");
  const mods = numberOf(rendered, "dependentModCount");
  return (
    <>
      <p className="wide-reliance">
        {mods > 0 ? w.needs(mod, mods, textOf(rendered, "dependents")) : w.needsNothing(mod)}
      </p>
      {numberOf(rendered, "assetCount") > 0 && (
        <p className="wide-reliance">{w.models(mod, textOf(rendered, "assetUsers"))}</p>
      )}
    </>
  );
}

/**
 * The suggestion about one mod as a banner. `onShowMod` leads to the mod's
 * collisions, where the banner stands elsewhere.
 */
export function WideAdvice({
  rendered,
  onShowMod,
}: {
  rendered: RenderedFinding;
  onShowMod?: () => void;
}) {
  const { lang, showReview } = useSession();
  const w = wideTexts[lang];
  const tone = toneOf(rendered);
  const mod = textOf(rendered, "mod");
  const places = numberOf(rendered, "count");
  return (
    <div className="wide-advice" data-tone={tone}>
      {tone === "remove" && (
        <>
          <p className="wide-lead">
            <strong>{w.remove(mod, places, numberOf(rendered, "modCount"))}</strong>
          </p>
          <Reliance rendered={rendered} />
          <p className="aside">{w.removeNote}</p>
        </>
      )}
      {tone === "bulk" && (
        <>
          <p className="wide-lead">{w.bulk(mod, places, numberOf(rendered, "clearCount"))}</p>
          <p className="aside">{w.bulkNote}</p>
        </>
      )}
      {tone === "keep" && (
        <>
          <p className="wide-lead">{w.keep(mod, places, numberOf(rendered, "interiors"))}</p>
          <Reliance rendered={rendered} />
        </>
      )}
      <div className="wide-buttons">
        {tone === "bulk" && showReview !== undefined && (
          <button type="button" className="primary" onClick={showReview}>
            {w.toReview}
          </button>
        )}
        {onShowMod !== undefined && (
          <button
            type="button"
            className={tone === "remove" ? undefined : "quiet"}
            onClick={onShowMod}
          >
            {w.showMod}
          </button>
        )}
      </div>
      {/* Removing stays possible for every tone; only the loud one leads with it. */}
      {tone === "remove" ? (
        <SwitchOffButton rendered={rendered} />
      ) : (
        <details className="wide-more">
          <summary>{w.switchOff}</summary>
          {tone === "bulk" && <Reliance rendered={rendered} />}
          <SwitchOffButton rendered={rendered} />
        </details>
      )}
    </div>
  );
}

/** The tag beside a mod in the list of mods. */
export function WideTag({ rendered }: { rendered: RenderedFinding }) {
  const { lang } = useSession();
  const w = wideTexts[lang];
  const tone = toneOf(rendered);
  const label = tone === "remove" ? w.tagRemove : tone === "bulk" ? w.tagBulk : w.tagKeep;
  return (
    <span className="wide-tag" data-tone={tone}>
      {label}
    </span>
  );
}
