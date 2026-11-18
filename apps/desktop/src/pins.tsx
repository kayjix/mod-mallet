// Pins (ADR 0050): spots the user marks to solve later by hand. A pin is
// no decision: the spot stays open, and the patch reads no pin. Pins are
// kept with the patch's choices, per game, and cleared once the spot is
// decided.

import { useSession } from "./Actions";
import type { Lang } from "./i18n";
import { pinsOf, setPins } from "./patch";
import "./pins.css";

export interface PinTexts {
  pin: string;
  unpin: string;
  pinHint: string;
  pinned: string;
  filter: (count: number) => string;
  cleared: (count: number) => string;
  reportPinned: (count: number) => string;
  showPinned: string;
}

const en: PinTexts = {
  pin: "Pin to solve later",
  unpin: "Unpin",
  pinHint:
    "Pinned spots stay open; you find them again with the filter Pinned on the collisions page and as flags on the map.",
  pinned: "pinned",
  filter: (count) => `Pinned only (${count})`,
  cleared: (count) =>
    count === 1
      ? "A pinned spot is decided now; its pin is gone."
      : `${count} pinned spots are decided now; their pins are gone.`,
  reportPinned: (count) =>
    count === 1 ? "1 spot pinned to solve yourself" : `${count} spots pinned to solve yourself`,
  showPinned: "Show them in the collisions",
};

const de: PinTexts = {
  pin: "Merken, später selbst lösen",
  unpin: "Nicht mehr merken",
  pinHint:
    "Gemerkte Stellen bleiben offen; du findest sie über den Filter „Gemerkte“ in den Kollisionen und als Fähnchen auf der Karte.",
  pinned: "gemerkt",
  filter: (count) => `Nur gemerkte (${count})`,
  cleared: (count) =>
    count === 1
      ? "Eine gemerkte Stelle ist jetzt entschieden; ihre Markierung ist weg."
      : `${count} gemerkte Stellen sind jetzt entschieden; ihre Markierungen sind weg.`,
  reportPinned: (count) =>
    count === 1 ? "1 Stelle zum Selbstlösen gemerkt" : `${count} Stellen zum Selbstlösen gemerkt`,
  showPinned: "In den Kollisionen zeigen",
};

export const pinTexts: Record<Lang, PinTexts> = { en, de };

/** The pins' texts in the window's language. */
export function usePinTexts(): PinTexts {
  return pinTexts[useSession().lang];
}

/** A pushpin: a round head on a needle, told apart by its shape. */
export function PinIcon({ filled = true }: { filled?: boolean }) {
  return (
    <svg className="pin-icon" viewBox="0 0 24 24" aria-hidden="true">
      <circle cx="12" cy="8" r="5" fill={filled ? "currentColor" : "none"} />
      <path d="M12 13v8" />
    </svg>
  );
}

/**
 * A button that pins the spots `keys` or clears their pins: pinned when
 * all of them are. Spots already decided are not pinned.
 */
export function PinButton({ keys, small = true }: { keys: string[]; small?: boolean }) {
  const { patchChoices, setPatchChoices } = useSession();
  const p = usePinTexts();
  const pins = pinsOf(patchChoices);
  const pinned = keys.length > 0 && keys.every((key) => pins.has(key));
  return (
    <button
      type="button"
      className={small ? "small pin-button" : "pin-button"}
      aria-pressed={pinned}
      title={p.pinHint}
      disabled={keys.length === 0}
      onClick={() => setPatchChoices(setPins(patchChoices, keys, !pinned))}
    >
      <PinIcon filled={pinned} /> {pinned ? p.unpin : p.pin}
    </button>
  );
}

/** The colour of a pin's head: reddish purple (Okabe–Ito), with a dark rim. */
export const PIN_COLOUR = "#cc79a7";

/**
 * Draws a pin on a flat map at `x`, `y` (the spot's point): a needle up
 * and a round head with a dark rim, in the accent colour, so that it reads
 * by its shape as well as its colour.
 */
export function drawPin(context: CanvasRenderingContext2D, x: number, y: number, colour: string) {
  context.save();
  context.lineWidth = 2;
  context.strokeStyle = "#111";
  context.beginPath();
  context.moveTo(x, y);
  context.lineTo(x, y - 14);
  context.stroke();
  context.beginPath();
  context.arc(x, y - 18, 5, 0, Math.PI * 2);
  context.fillStyle = colour;
  context.fill();
  context.lineWidth = 1.5;
  context.stroke();
  context.restore();
}
