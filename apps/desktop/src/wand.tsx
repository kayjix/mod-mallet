// The magic wand of the 3D views (ADR 0055): picking an object takes along
// what belongs to it, the more the higher its level. And the bar of tools
// along the top of a 3D view that holds it.

import "./view-toolbar.css";
import { type ReactNode, useState } from "react";
import { useSession } from "./Actions";
import { WAND_TOP } from "./area-layer";
import type { Lang } from "./i18n";

interface WandTexts {
  wand: string;
  hint: string;
  /** What each level takes along, from 0 to {@link WAND_TOP}. */
  levels: string[];
  took: (count: number, level: number) => string;
  alone: string;
  raise: string;
}

const wandTexts: Record<Lang, WandTexts> = {
  en: {
    wand: "Magic wand",
    hint: "Pick an object in 3D: the wand takes along what belongs to it, the higher the level the more. Only parts of the same mod, at most about 30 m away.",
    levels: [
      "Off: the object alone",
      "With what lies on it",
      "The same ones right beside it",
      "Its kind in a row",
      "The whole structure",
      "The structure, gaps too",
    ],
    took: (count, level) =>
      count === 1
        ? `Magic wand, level ${level}: 1 part`
        : `Magic wand, level ${level}: ${count} parts`,
    alone: "At this level nothing else belongs to it.",
    raise: "To take more at once, raise the magic wand above the view.",
  },
  de: {
    wand: "Zauberstab",
    hint: "Klick in 3D ein Objekt an: Der Zauberstab nimmt mit, was dazugehört, je höher die Stufe, desto mehr. Nur Teile derselben Mod, höchstens etwa 30 m entfernt.",
    levels: [
      "Aus: nur das Objekt",
      "Mit dem, was darauf liegt",
      "Gleiche direkt daneben",
      "Gleiche Art in Reihe",
      "Das ganze Bauwerk",
      "Das Bauwerk, auch mit Lücken",
    ],
    took: (count, level) =>
      count === 1
        ? `Zauberstab Stufe ${level}: 1 Teil`
        : `Zauberstab Stufe ${level}: ${count} Teile`,
    alone: "Auf dieser Stufe gehört nichts weiter dazu.",
    raise: "Mehr auf einmal: den Zauberstab über der Ansicht höher stellen.",
  },
};

/** The texts of the magic wand in the language shown. */
export function useWandTexts(): WandTexts {
  const { lang } = useSession();
  return wandTexts[lang];
}

/** The magic wand's level as last set in any view, while the app runs; off at first. */
let lastLevel = 0;

/** The magic wand's level of a view, starting where it was last set. */
export function useWandLevel(): [number, (level: number) => void] {
  const [level, setLevel] = useState(lastLevel);
  return [
    level,
    (next: number) => {
      lastLevel = next;
      setLevel(next);
    },
  ];
}

/** The magic wand's control: its level, and what that level takes along. */
export function WandControl({
  level,
  onLevel,
}: {
  level: number;
  onLevel: (level: number) => void;
}) {
  const w = useWandTexts();
  return (
    <label className="view-tool wand" title={w.hint} data-on={level > 0 || undefined}>
      <WandIcon />
      <span className="view-tool-name">{w.wand}</span>
      <input
        type="range"
        min={0}
        max={WAND_TOP}
        step={1}
        value={level}
        aria-valuetext={w.levels[level]}
        onChange={(event) => onLevel(Number(event.target.value))}
      />
      <span className="view-tool-value">{w.levels[level]}</span>
    </label>
  );
}

/** The bar of tools along the top of a 3D view: tools on the left, the camera's on the right. */
export function ViewToolbar({ tools, camera }: { tools: ReactNode; camera: ReactNode }) {
  return (
    <div className="view-toolbar">
      <div className="view-toolbar-tools">{tools}</div>
      <div className="view-toolbar-camera">{camera}</div>
    </div>
  );
}

function WandIcon() {
  return (
    <svg className="wand-icon" viewBox="0 0 24 24" aria-hidden="true">
      <path d="M4 20 15 9" />
      <path d="M15 9l2-2" />
      <path d="M18 2v3M16.5 3.5h3M21 7v2M20 8h2M12 3v2M11 4h2" />
    </svg>
  );
}
