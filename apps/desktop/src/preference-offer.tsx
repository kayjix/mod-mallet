// The offer of a preference learned (ADR 0086): a small card in a corner,
// once a pattern is clear. Preview lists what would be decided, each with
// the way to it; Apply decides them in the patch being edited, with one
// undo; Not now asks again after more decisions; Never stops it.

import { useState } from "react";
import { useSession } from "./Actions";
import { usePreferenceTexts } from "./preference-texts";
import type { PlanItem } from "./preferences";
import { usePreferences } from "./preferences-ui";
import { Chip, Note, UndoButton } from "./ui/kit";

/** Where a spot is, in a few words. */
function placeName(item: PlanItem): string {
  if (item.conflict.type !== "spot") {
    return "";
  }
  const { location } = item.conflict.spot;
  return location.type === "interior"
    ? location.cell
    : `${location.world} ${location.x}, ${location.y}`;
}

/** The list of what a preference would decide, each with the way to it. */
export function PreferencePreview({ items }: { items: PlanItem[] }) {
  const { openCompare, decideSpot } = useSession();
  const p = usePreferenceTexts();
  return (
    <ul className="preference-preview">
      {items.map((item) => (
        <li key={item.conflict.id} className="preference-preview-row">
          <span className="preference-preview-text">
            {item.conflict.type === "duel" ? (
              <>
                {p.duelRow(item.winner, item.loser)}
                <span className="aside">
                  {" "}
                  · {item.conflict.topics.map((topic) => p.topic[topic]).join(", ")}
                </span>
              </>
            ) : (
              <>
                {p.spotRow(item.loser, item.conflict.spot.objects)}
                <span className="aside"> · {placeName(item)}</span>
              </>
            )}
          </span>
          <button
            type="button"
            className="small quiet"
            onClick={() =>
              item.conflict.type === "duel"
                ? openCompare(item.conflict.id)
                : decideSpot(item.conflict.spot)
            }
          >
            {p.open} ▸
          </button>
        </li>
      ))}
    </ul>
  );
}

/** What became of applying a preference, with one undo for the whole batch. */
export function PreferenceBatch() {
  const preferences = usePreferences();
  const p = usePreferenceTexts();
  const batch = preferences?.batch ?? null;
  if (preferences === null || batch === null) {
    return null;
  }
  const lines = [
    p.applied(batch.decided),
    ...(batch.walked > 0 ? [p.walked(batch.walked)] : []),
    ...(batch.failed > 0 ? [p.failed(batch.failed)] : []),
    ...(batch.stopped !== null ? [p.stopped(batch.stopped)] : []),
  ];
  return (
    <Note
      tone={batch.undone ? "info" : batch.stopped !== null ? "warning" : "ok"}
      role="status"
      actions={
        <>
          {!batch.undone && batch.keys.length > 0 && (
            <UndoButton label={p.undoAll} onClick={preferences.undo} />
          )}
          <button type="button" className="small quiet" onClick={preferences.forgetBatch}>
            {p.close}
          </button>
        </>
      }
    >
      {batch.undone ? (
        <p>{p.undone}</p>
      ) : (
        <>
          {lines.map((line) => (
            <p key={line}>{line}</p>
          ))}
          {batch.keys.some((key) => key.startsWith("duel:")) && (
            <p className="aside">{p.undoNote}</p>
          )}
        </>
      )}
    </Note>
  );
}

/** The card in the corner: a preference offered, or what applying it did. */
export function PreferenceOffer() {
  const preferences = usePreferences();
  const p = usePreferenceTexts();
  const [previewing, setPreviewing] = useState(false);
  const [busy, setBusy] = useState(false);
  if (preferences === null) {
    return null;
  }
  const { offer, batch, progress } = preferences;
  if (progress !== null) {
    return (
      <aside className="preference-offer" aria-live="polite">
        <p>{p.checking(progress.done, progress.total)}</p>
      </aside>
    );
  }
  if (batch !== null) {
    return (
      <aside className="preference-offer">
        <PreferenceBatch />
      </aside>
    );
  }
  if (offer === null) {
    return null;
  }
  const { learned, items } = offer;
  const rule = preferences.rules.find((candidate) => candidate.id === learned.id);
  const files = items.some((item) => item.conflict.type === "duel");
  const apply = async () => {
    if (rule === undefined) {
      return;
    }
    setBusy(true);
    setPreviewing(false);
    try {
      await preferences.apply(rule, items);
    } finally {
      setBusy(false);
    }
  };
  return (
    <aside className="preference-offer" aria-labelledby="preference-offer-title">
      <header className="preference-offer-head">
        <strong id="preference-offer-title">{p.offerTitle}</strong>
        <Chip tone="info">{learned.topic === null ? learned.over : p.topic[learned.topic]}</Chip>
      </header>
      <p>
        {learned.kind === "favourite" && learned.topic !== null
          ? p.offerFavourite(learned.mod, p.topic[learned.topic], learned.chosen, learned.of)
          : p.offerPair(learned.mod, learned.over ?? "", learned.chosen, learned.of)}
      </p>
      <p>
        <strong>
          {learned.kind === "favourite" && learned.topic !== null
            ? p.offerQuestion(learned.mod, learned.topic, items.length)
            : p.offerPairQuestion(learned.mod, items.length)}
        </strong>
      </p>
      {files && <p className="aside">{p.managerNote(learned.mod)}</p>}
      {previewing && (
        <>
          <p className="aside">{p.previewHint}</p>
          <PreferencePreview items={items} />
        </>
      )}
      <div className="preference-offer-actions">
        <button type="button" className="small" onClick={() => setPreviewing(!previewing)}>
          {previewing ? p.hidePreview : p.preview}
        </button>
        <button
          type="button"
          className="small primary"
          disabled={busy || rule === undefined}
          onClick={apply}
        >
          {p.apply(items.length)}
        </button>
        <button
          type="button"
          className="small quiet"
          onClick={() => preferences.answer(learned, "later")}
        >
          {p.notNow}
        </button>
        <button
          type="button"
          className="small quiet"
          onClick={() => preferences.answer(learned, "never")}
        >
          {p.never}
        </button>
      </div>
    </aside>
  );
}
