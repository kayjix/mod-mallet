// The one way the window explains something: a small ⓘ beside the control
// it is about, which shows a short text while hovered or focused, and
// keeps it open when clicked. Pages carry no lines of explanation and no
// headings in their place.

import { type CSSProperties, useEffect, useId, useRef, useState } from "react";
import { useSession } from "./Actions";
import type { Lang } from "./i18n";
import "./info-tip.css";

const LABEL: Record<Lang, string> = { en: "What is this?", de: "Was ist das?" };

/** How wide the text may get, and how far it keeps from the window's edge. */
const WIDTH = 320;
const MARGIN = 8;

/**
 * A ⓘ that shows `text` on hover, on focus, and pinned open on click. The
 * text is placed against the window, so that no scrolling panel cuts it
 * off: below the ⓘ, or above where the ⓘ sits at the foot of a view.
 */
export function InfoTip({ text, above = false }: { text: string; above?: boolean }) {
  const { lang } = useSession();
  const [pinned, setPinned] = useState(false);
  const [hovered, setHovered] = useState(false);
  const [place, setPlace] = useState<CSSProperties | null>(null);
  const tipId = useId();
  const root = useRef<HTMLSpanElement>(null);
  const shown = pinned || hovered;

  // Where the text goes, measured each time it opens.
  const measure = () => {
    const box = root.current?.getBoundingClientRect();
    if (box === undefined) {
      return;
    }
    const left = Math.max(MARGIN, Math.min(box.left - MARGIN, window.innerWidth - WIDTH - MARGIN));
    setPlace(
      above ? { left, bottom: window.innerHeight - box.top + 6 } : { left, top: box.bottom + 6 },
    );
  };

  const open = () => {
    measure();
    setHovered(true);
  };

  // A pinned text closes on Escape and on a click anywhere else.
  useEffect(() => {
    if (!pinned) {
      return;
    }
    const close = (event: Event) => {
      if (event instanceof KeyboardEvent && event.key !== "Escape") {
        return;
      }
      if (event instanceof MouseEvent && root.current?.contains(event.target as Node)) {
        return;
      }
      setPinned(false);
    };
    document.addEventListener("keydown", close);
    document.addEventListener("mousedown", close);
    return () => {
      document.removeEventListener("keydown", close);
      document.removeEventListener("mousedown", close);
    };
  }, [pinned]);

  return (
    <span className="info-tip" data-pinned={pinned || undefined} ref={root}>
      <button
        type="button"
        className="info-tip-button"
        aria-label={LABEL[lang]}
        aria-describedby={tipId}
        aria-expanded={pinned}
        onMouseEnter={open}
        onMouseLeave={() => setHovered(false)}
        onFocus={open}
        onBlur={() => setHovered(false)}
        onClick={(event) => {
          // Inside a summary, the click opens the ⓘ, not the details.
          event.preventDefault();
          measure();
          setPinned(!pinned);
        }}
      >
        i
      </button>
      <span
        className="info-tip-text"
        role="tooltip"
        id={tipId}
        hidden={!shown}
        style={{ ...place, maxWidth: WIDTH }}
      >
        {text}
      </span>
    </span>
  );
}
