// The kit's signs (ADR 0077): simple strokes in the text's colour, like the
// sidebar's icons. A tone always has its own shape, so that a state reads
// without its colour.

import type { ReactNode } from "react";
import type { Tone } from "./ui-texts";

function Sign({ className, children }: { className: string; children: ReactNode }) {
  return (
    <svg
      className={className}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      {children}
    </svg>
  );
}

/** The sign of a tone: i in a circle, a tick, ! in a triangle, × in an octagon. */
export function ToneIcon({ tone, className = "tone-icon" }: { tone: Tone; className?: string }) {
  switch (tone) {
    case "ok":
      return (
        <Sign className={className}>
          <circle cx="12" cy="12" r="9" />
          <path d="M8 12.5l2.7 2.7L16 9.8" />
        </Sign>
      );
    case "warning":
      return (
        <Sign className={className}>
          <path d="M12 3.5L21.5 20h-19z" />
          <path d="M12 10v4.5M12 17.4v.1" />
        </Sign>
      );
    case "danger":
      return (
        <Sign className={className}>
          <path d="M8.3 3h7.4L21 8.3v7.4L15.7 21H8.3L3 15.7V8.3z" />
          <path d="M9.2 9.2l5.6 5.6M14.8 9.2l-5.6 5.6" />
        </Sign>
      );
    case "neutral":
      return (
        <Sign className={className}>
          <circle cx="12" cy="12" r="3" />
        </Sign>
      );
    default:
      return (
        <Sign className={className}>
          <circle cx="12" cy="12" r="9" />
          <path d="M12 11v5.5M12 7.6v.1" />
        </Sign>
      );
  }
}

/** A magnifier, in front of a search field. */
export function SearchIcon() {
  return (
    <Sign className="search-icon">
      <circle cx="10.5" cy="10.5" r="6" />
      <path d="M15 15l5 5" />
    </Sign>
  );
}

/** A bent arrow back: taking a change back. */
export function UndoIcon() {
  return (
    <Sign className="undo-icon">
      <path d="M9 14L4 9l5-5" />
      <path d="M4 9h10.5a5.5 5.5 0 0 1 0 11H11" />
    </Sign>
  );
}
