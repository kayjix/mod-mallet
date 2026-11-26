import { useState } from "react";
import { useSession } from "./Actions";
import { useShareTexts } from "./share-texts";
import { Note } from "./ui/kit";
import "./share-report.css";

/** What became of the last press of a button. */
type Outcome =
  | { kind: "copied"; removed: string }
  | { kind: "not-copied"; removed: string }
  | { kind: "saved"; removed: string; path: string }
  | { kind: "failed"; error: string };

/**
 * The last report for someone who helps (ADR 0091): copied to the
 * clipboard as text, or saved as a file, with the user name and the
 * folders of this computer replaced. Says what was removed.
 */
export function ShareReport() {
  const { api, lang } = useSession();
  const s = useShareTexts();
  const [busy, setBusy] = useState(false);
  const [outcome, setOutcome] = useState<Outcome | null>(null);

  const share = async () => {
    setBusy(true);
    try {
      const shared = await api.sharedReport(lang);
      if (shared === null) {
        return;
      }
      try {
        await navigator.clipboard.writeText(shared.text);
        setOutcome({ kind: "copied", removed: shared.removed });
      } catch {
        setOutcome({ kind: "not-copied", removed: shared.removed });
      }
    } catch (error) {
      setOutcome({ kind: "failed", error: String(error) });
    } finally {
      setBusy(false);
    }
  };

  const save = async (format: "md" | "json") => {
    const removed = outcome !== null && outcome.kind !== "failed" ? outcome.removed : "";
    setBusy(true);
    try {
      const path = await api.saveSharedReport(lang, format);
      setOutcome({ kind: "saved", removed, path });
    } catch (error) {
      setOutcome({ kind: "failed", error: String(error) });
    } finally {
      setBusy(false);
    }
  };

  const saveButtons = (
    <>
      <button type="button" className="small" disabled={busy} onClick={() => save("md")}>
        {s.saveMarkdown}
      </button>
      <button type="button" className="small" disabled={busy} onClick={() => save("json")}>
        {s.saveJson}
      </button>
    </>
  );

  return (
    <section className="panel share-report">
      <div className="share-report-head">
        <div>
          <h3>{s.title}</h3>
          <p className="meta">{s.note}</p>
        </div>
        <button type="button" disabled={busy} onClick={share}>
          {s.share}
        </button>
      </div>
      {outcome !== null && outcome.kind === "failed" && (
        <Note tone="danger" role="alert">
          {s.failed(outcome.error)}
        </Note>
      )}
      {outcome !== null && outcome.kind !== "failed" && (
        <Note
          tone={outcome.kind === "not-copied" ? "warning" : "ok"}
          role="status"
          actions={saveButtons}
        >
          <p>
            {outcome.kind === "copied"
              ? s.copied
              : outcome.kind === "not-copied"
                ? s.notCopied
                : s.saved(outcome.path)}{" "}
            {outcome.removed}
          </p>
        </Note>
      )}
    </section>
  );
}
