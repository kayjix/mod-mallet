// The way back (ADR 0070): back and forward through the places the user
// went, on every page, and back to the entry of a patch a page was opened
// from.

import type { PatchAt } from "./Actions";
import type { Lang } from "./i18n";
import "./nav-bar.css";

interface NavTexts {
  back: string;
  forward: string;
  keys: string;
  backTo: (label: string) => string;
}

const navTexts: Record<Lang, NavTexts> = {
  en: {
    back: "Back",
    forward: "Forward",
    keys: "Also with the mouse's side buttons, or Alt+← and Alt+→.",
    backTo: (label) => `Back to ${label}`,
  },
  de: {
    back: "Zurück",
    forward: "Vorwärts",
    keys: "Auch mit den Seitentasten der Maus oder mit Alt+← und Alt+→.",
    backTo: (label) => `Zurück zu ${label}`,
  },
};

interface NavBarProps {
  lang: Lang;
  canBack: boolean;
  canForward: boolean;
  onBack: () => void;
  onForward: () => void;
  /** The entry of a patch this page was opened from. */
  from: PatchAt | null;
  onReturn: (at: PatchAt) => void;
}

/** Back and forward, and the way back to a patch's entry. */
export function NavBar({
  lang,
  canBack,
  canForward,
  onBack,
  onForward,
  from,
  onReturn,
}: NavBarProps) {
  const t = navTexts[lang];
  if (!canBack && !canForward && from === null) {
    return null;
  }
  return (
    <nav className="nav-bar" aria-label={t.back}>
      <button
        type="button"
        className="nav-step"
        disabled={!canBack}
        title={`${t.back}. ${t.keys}`}
        aria-label={t.back}
        onClick={onBack}
      >
        ←
      </button>
      <button
        type="button"
        className="nav-step"
        disabled={!canForward}
        title={`${t.forward}. ${t.keys}`}
        aria-label={t.forward}
        onClick={onForward}
      >
        →
      </button>
      {from !== null && (
        <button type="button" className="nav-return" onClick={() => onReturn(from)}>
          ← {t.backTo(from.label)}
        </button>
      )}
    </nav>
  );
}
