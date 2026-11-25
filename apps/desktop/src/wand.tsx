// The magic wand of the 3D views (ADR 0055): picking an object takes along
// what belongs to it, the more the higher its level. And the bar of tools
// along the top of a 3D view that holds it.

import "./view-toolbar.css";
import { type ReactNode, useState } from "react";
import { useDecisions, useSession } from "./Actions";
import { WAND_TOP } from "./area-layer";
import type { AreaObject } from "./generated/area-objects";
import type { Lang } from "./i18n";
import { LevelToggle } from "./ui/kit";

interface WandTexts {
  wand: string;
  hint: string;
  /** What each level takes along, from 0 to {@link WAND_TOP}. */
  levels: string[];
  took: (count: number, level: number) => string;
  alone: string;
  raise: string;
  /** A selection of hidden and visible parts: how many of each. */
  mixed: (hidden: number, visible: number) => string;
  /** What a level pointed at would take: its name, how many parts, how many of them hidden. */
  preview: (name: string, count: number, hidden: number) => string;
  /** A level pointed at while no object is picked. */
  previewNothing: (name: string) => string;
}

const wandTexts: Record<Lang, WandTexts> = {
  en: {
    wand: "Magic wand",
    hint: "Pick an object in 3D: the wand takes along what belongs to it, the higher the level the more. Point at a level to see what it would take. Only parts of the same mod, at most about 30 m away.",
    levels: [
      "Off: the object alone",
      "With what lies on it",
      "The same ones right beside it",
      "Its kind in a row",
      "This structure: built of the same kind of pieces",
      "The whole building, up to where it meets the next",
      "Everything joined to it",
    ],
    took: (count, level) =>
      count === 1
        ? `Magic wand, level ${level}: 1 part`
        : `Magic wand, level ${level}: ${count} parts`,
    alone: "At this level nothing else belongs to it.",
    raise: "To take more at once, raise the magic wand above the view.",
    mixed: (hidden, visible) => `${hidden} of them hidden, ${visible} there.`,
    preview: (name, count, hidden) => {
      const parts = count === 1 ? "1 part" : `${count} parts`;
      if (hidden === 0) {
        return `${parts} · ${name}`;
      }
      if (hidden === count) {
        return count === 1 ? `1 part, hidden · ${name}` : `${parts}, all hidden · ${name}`;
      }
      return `${parts}, ${hidden} hidden, ${count - hidden} there · ${name}`;
    },
    previewNothing: (name) => `${name}. Pick an object to see what it takes.`,
  },
  de: {
    wand: "Zauberstab",
    hint: "Klick in 3D ein Objekt an: Der Zauberstab nimmt mit, was dazugehört, je höher die Stufe, desto mehr. Zeig auf eine Stufe, um zu sehen, was sie nehmen würde. Nur Teile derselben Mod, höchstens etwa 30 m entfernt.",
    levels: [
      "Aus: nur das Objekt",
      "Mit dem, was darauf liegt",
      "Gleiche direkt daneben",
      "Gleiche Art in Reihe",
      "Dieses Bauwerk: aus gleichartigen Bauteilen",
      "Das ganze Gebäude, bis dort, wo das nächste anschließt",
      "Alles, was daran hängt",
    ],
    took: (count, level) =>
      count === 1
        ? `Zauberstab Stufe ${level}: 1 Teil`
        : `Zauberstab Stufe ${level}: ${count} Teile`,
    alone: "Auf dieser Stufe gehört nichts weiter dazu.",
    raise: "Mehr auf einmal: den Zauberstab über der Ansicht höher stellen.",
    mixed: (hidden, visible) => `${hidden} davon ausgeblendet, ${visible} sichtbar.`,
    preview: (name, count, hidden) => {
      const parts = count === 1 ? "1 Teil" : `${count} Teile`;
      if (hidden === 0) {
        return `${parts} · ${name}`;
      }
      if (hidden === count) {
        return count === 1
          ? `1 Teil, ausgeblendet · ${name}`
          : `${parts}, alle ausgeblendet · ${name}`;
      }
      return `${parts}, ${hidden} ausgeblendet, ${count - hidden} sichtbar · ${name}`;
    },
    previewNothing: (name) => `${name}. Klick ein Objekt an, um zu sehen, was sie nimmt.`,
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
  const [level, setLevel] = useState(Math.min(lastLevel, WAND_TOP));
  return [
    level,
    (next: number) => {
      lastLevel = next;
      setLevel(next);
    },
  ];
}

/** A level of the magic wand pointed at, and what it would take; none while nothing is picked. */
export interface WandPreview {
  level: number;
  parts: AreaObject[] | null;
}

/**
 * The magic wand's control (ADR 0077): one button that turns it on and off,
 * and a pip for each of its levels, filled up to the level set. Pointing
 * at a pip shows below it what that level would take (ADR 0055); the view
 * lights those parts meanwhile.
 */
export function WandTool({
  level,
  onLevel,
  preview = null,
  onPreview,
}: {
  level: number;
  onLevel: (level: number) => void;
  /** The level pointed at, and what it would take. */
  preview?: WandPreview | null;
  onPreview?: (level: number | null) => void;
}) {
  const w = useWandTexts();
  const { decisions } = useDecisions();
  const name = preview === null ? "" : (w.levels[preview.level] ?? "");
  const note =
    preview === null
      ? null
      : preview.parts === null
        ? w.previewNothing(name)
        : w.preview(
            name,
            preview.parts.length,
            preview.parts.filter((part) => decisions.isHidden(part.id)).length,
          );
  return (
    <span className="view-tool wand">
      <LevelToggle
        icon={<WandIcon />}
        label={w.wand}
        level={level}
        levels={w.levels}
        onLevel={onLevel}
        hint={w.hint}
        onPreview={onPreview}
      />
      {note !== null && (
        <span className="wand-preview" role="status">
          {note}
        </span>
      )}
    </span>
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
