// The notes of a spot panel (walkways, reach, repairs, what a door's choice
// changes) as small badges in the card they are about, so that the 3D view
// keeps its height. A switch beside "Fast forward" shows them all in full;
// pages that do not set it show them in full as before.

import { createContext, type ReactNode, useContext, useState } from "react";
import { useSession } from "./Actions";
import { spotNotesTexts } from "./spot-notes-texts";
import "./spot-notes.css";

/** Whether the notes are shown in full; badges where false. */
export const NotesInFull = createContext(true);

/** Whether the notes around are shown in full. */
export function useNotesInFull(): boolean {
  return useContext(NotesInFull);
}

/** How a note reads at a glance. */
export type NoteTone = "warn" | "ok" | "bad" | "info";

/**
 * One note: in full where notes are, otherwise a badge with a one-line
 * `summary` whose tooltip is `hint`, and which opens the note when clicked.
 */
export function Note({
  tone,
  icon,
  summary,
  hint,
  children,
}: {
  tone: NoteTone;
  icon?: ReactNode;
  summary: string;
  /** The note in plain words, for the badge's tooltip. */
  hint: string;
  children: ReactNode;
}) {
  const full = useNotesInFull();
  const { lang } = useSession();
  const [open, setOpen] = useState(false);
  if (full) {
    return <>{children}</>;
  }
  return (
    <>
      <button
        type="button"
        className="note-badge"
        data-tone={tone}
        aria-expanded={open}
        title={`${hint}\n${spotNotesTexts[lang].openHint}`}
        onClick={() => setOpen(!open)}
      >
        {icon ?? <span className="note-dot" aria-hidden="true" />}
        <span className="note-summary">{summary}</span>
      </button>
      {open && <div className="note-open">{children}</div>}
    </>
  );
}

/** The notes of one card: a row of badges, or the notes in full one below the other. */
export function NoteRow({ children }: { children: ReactNode }) {
  const full = useNotesInFull();
  return (
    <div className="note-row" data-full={full || undefined}>
      {children}
    </div>
  );
}
