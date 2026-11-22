// The way back (ADR 0070): back and forward through the places the user
// went, and back to the entry of a patch a page was opened from. They sit
// in the sidebar, above the toolbox, so that they take no height from the
// pages.

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

interface NavStepsProps {
  lang: Lang;
  canBack: boolean;
  canForward: boolean;
  onBack: () => void;
  onForward: () => void;
  /** The entry of a patch this page was opened from. */
  from: PatchAt | null;
  onReturn: (at: PatchAt) => void;
}

/** Back and forward, and the way back to a patch's entry, as a small row of the sidebar. */
export function NavSteps({
  lang,
  canBack,
  canForward,
  onBack,
  onForward,
  from,
  onReturn,
}: NavStepsProps) {
  const t = navTexts[lang];
  return (
    <div className="nav-steps">
      <div className="nav-steps-row">
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
      </div>
      {from !== null && (
        <button
          type="button"
          className="nav-return"
          title={t.backTo(from.label)}
          onClick={() => onReturn(from)}
        >
          ← {t.backTo(from.label)}
        </button>
      )}
    </div>
  );
}
