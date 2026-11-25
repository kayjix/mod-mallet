// The offer of preferences learned (ADR 0086). A small mallet waits in a
// corner, with how many favourites wait on a badge, and now and then does
// a little move; it says nothing until clicked. Clicked, a panel slides up
// from it: one favourite at a time, how often it was kept, the list of
// what it would decide on request, and Apply, with one undo. Closing the
// panel puts it back into the mallet; "Don't suggest this again" sits in
// a small menu.

import {
  type KeyboardEvent,
  type ReactNode,
  type RefObject,
  useEffect,
  useId,
  useRef,
  useState,
} from "react";
import { useSession } from "./Actions";
import { InfoTip } from "./info-tip";
import { Hammer } from "./Mallet";
import { type PreferenceTexts, usePreferenceTexts } from "./preference-texts";
import type { Offer, PlanItem } from "./preferences";
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

/** The moves the mallet makes now and then. */
const MOVES = ["toss", "bounce", "tap-left", "tap-right"] as const;
type Move = (typeof MOVES)[number];

/** About a minute between moves, never quite the same. */
const PAUSE_MS = 50_000;
const JITTER_MS = 25_000;
/** How long the window must have been left alone for a move. */
const QUIET_MS = 4_000;

function reducedMotion(): boolean {
  return window.matchMedia?.("(prefers-reduced-motion: reduce)").matches ?? false;
}

/**
 * Whether the user is busy: a button held down (dragging in a 3D view,
 * say) or a key, a click or the wheel used a moment ago. The mallet keeps
 * still then.
 */
function useBusy(): () => boolean {
  const state = useRef({ pressed: false, last: 0 });
  useEffect(() => {
    const touched = (event: Event) => {
      state.current.last = performance.now();
      if (event instanceof PointerEvent) {
        state.current.pressed = event.buttons !== 0;
      }
    };
    const kinds = ["pointerdown", "pointerup", "pointercancel", "keydown", "wheel"] as const;
    for (const kind of kinds) {
      window.addEventListener(kind, touched, { capture: true, passive: true });
    }
    const dragged = (event: PointerEvent) => {
      if (event.buttons !== 0) {
        touched(event);
      }
    };
    window.addEventListener("pointermove", dragged, { capture: true, passive: true });
    return () => {
      for (const kind of kinds) {
        window.removeEventListener(kind, touched, { capture: true });
      }
      window.removeEventListener("pointermove", dragged, { capture: true });
    };
  }, []);
  return () =>
    state.current.pressed ||
    performance.now() - state.current.last < QUIET_MS ||
    document.hidden ||
    reducedMotion();
}

/** The mallet in the corner: the logo's hammer, with how many favourites wait. */
function Mascot({
  count,
  open,
  onClick,
  p,
  buttonRef,
}: {
  count: number;
  open: boolean;
  onClick: () => void;
  p: PreferenceTexts;
  buttonRef: RefObject<HTMLButtonElement | null>;
}) {
  const clip = `mascot-head-${useId().replace(/:/g, "")}`;
  const [move, setMove] = useState<Move | null>(null);
  const busy = useBusy();
  const busyRef = useRef(busy);
  busyRef.current = busy;

  // Now and then a move, while the panel is closed and the user is not busy.
  useEffect(() => {
    if (open) {
      setMove(null);
      return;
    }
    let timer = 0;
    const wait = () => {
      timer = window.setTimeout(
        () => {
          if (!busyRef.current()) {
            setMove(MOVES[Math.floor(Math.random() * MOVES.length)] ?? "toss");
          }
          wait();
        },
        PAUSE_MS + Math.random() * JITTER_MS,
      );
    };
    wait();
    return () => window.clearTimeout(timer);
  }, [open]);

  // A new favourite: a little hop to say so.
  const before = useRef(count);
  useEffect(() => {
    if (count > before.current && !open && !busyRef.current()) {
      setMove("toss");
    }
    before.current = count;
  }, [count, open]);

  const label = p.mascot(count);
  return (
    <button
      ref={buttonRef}
      type="button"
      className="preference-mascot"
      data-move={open ? undefined : (move ?? undefined)}
      data-open={open || undefined}
      aria-label={label}
      title={label}
      aria-expanded={open}
      aria-controls="preference-offer"
      onClick={onClick}
    >
      <svg
        viewBox="0 0 512 512"
        aria-hidden="true"
        className="preference-mascot-hammer"
        onAnimationEnd={() => setMove(null)}
      >
        <defs>
          <clipPath id={clip}>
            <path d="M -122,-56 H 122 A 170 170 0 0 1 122,56 H -122 A 170 170 0 0 1 -122,-56 Z" />
          </clipPath>
        </defs>
        <Hammer clip={clip} />
      </svg>
      {count > 0 && <span className="preference-mascot-badge">{count}</span>}
    </button>
  );
}

/** The small menu of the panel, with what is said once and seldom. */
function MoreMenu({ p, onNever }: { p: PreferenceTexts; onNever: () => void }) {
  const [open, setOpen] = useState(false);
  const root = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!open) {
      return;
    }
    const close = (event: MouseEvent) => {
      if (!root.current?.contains(event.target as Node)) {
        setOpen(false);
      }
    };
    window.addEventListener("mousedown", close);
    return () => window.removeEventListener("mousedown", close);
  }, [open]);
  // Escape closes the menu only, not the panel around it.
  const keys = (event: KeyboardEvent) => {
    if (open && event.key === "Escape") {
      event.stopPropagation();
      setOpen(false);
    }
  };
  return (
    // biome-ignore lint/a11y/noStaticElementInteractions: Escape from the menu bubbles here.
    <div className="preference-menu" ref={root} onKeyDown={keys}>
      <button
        type="button"
        className="small quiet preference-icon-button"
        aria-label={p.more}
        title={p.more}
        aria-haspopup="menu"
        aria-expanded={open}
        onClick={() => setOpen(!open)}
      >
        ⋯
      </button>
      {open && (
        <div className="preference-menu-list" role="menu">
          <button
            type="button"
            role="menuitem"
            className="small quiet"
            onClick={() => {
              setOpen(false);
              onNever();
            }}
          >
            {p.never}
          </button>
        </div>
      )}
    </div>
  );
}

/** The ⓘ of an offer: what Apply does, in full. */
function offerTip(p: PreferenceTexts, offer: Offer): string {
  const { learned, items } = offer;
  const what =
    learned.kind === "favourite" && learned.topic !== null
      ? p.offerWhat(learned.mod, learned.topic, items.length)
      : p.offerPairWhat(learned.mod, items.length);
  const files = items.some((item) => item.conflict.type === "duel");
  return [what, p.keepHint, ...(files ? [p.managerNote(learned.mod)] : [])].join(" ");
}

/** The mallet in the corner, and the panel it opens. */
export function PreferenceOffer() {
  const preferences = usePreferences();
  const p = usePreferenceTexts();
  const [open, setOpen] = useState(false);
  const [index, setIndex] = useState(0);
  const [listed, setListed] = useState(false);
  const [busy, setBusy] = useState(false);
  const panel = useRef<HTMLElement>(null);
  const mascot = useRef<HTMLButtonElement>(null);

  const offers = preferences?.offers ?? [];
  const working =
    preferences !== null && (preferences.progress !== null || preferences.batch !== null);
  const shown = open || working;

  // Opened by a click on the mallet: the panel takes the focus.
  useEffect(() => {
    if (open) {
      panel.current?.focus();
    }
  }, [open]);

  if (preferences === null || (offers.length === 0 && !working)) {
    return null;
  }

  const at = Math.min(index, offers.length - 1);
  const offer = offers[at] ?? null;
  const minimise = () => {
    // The focus goes back to the mallet the panel came from.
    if (panel.current?.contains(document.activeElement)) {
      mascot.current?.focus();
    }
    setOpen(false);
    setListed(false);
  };
  const keys = (event: KeyboardEvent) => {
    if (event.key === "Escape" && !working) {
      minimise();
    }
  };

  let body: ReactNode = null;
  if (preferences.progress !== null) {
    body = <p>{p.checking(preferences.progress.done, preferences.progress.total)}</p>;
  } else if (preferences.batch !== null) {
    body = <PreferenceBatch />;
  } else if (offer !== null) {
    const { learned, items } = offer;
    const rule = preferences.rules.find((candidate) => candidate.id === learned.id);
    const apply = async () => {
      if (rule === undefined) {
        return;
      }
      setBusy(true);
      // What applying did shows in the panel; closing it goes back to the mallet.
      setOpen(false);
      setListed(false);
      try {
        await preferences.apply(rule, items);
      } finally {
        setBusy(false);
      }
    };
    const step = (by: number) => {
      setIndex((at + by + offers.length) % offers.length);
      setListed(false);
    };
    body = (
      <>
        <header className="preference-offer-head">
          <strong id="preference-offer-title">{p.offerTitle}</strong>
          <InfoTip text={offerTip(p, offer)} />
          <span className="preference-offer-spacer" />
          <MoreMenu p={p} onNever={() => preferences.answer(learned, "never")} />
          <button
            type="button"
            className="small quiet preference-icon-button"
            aria-label={p.minimise}
            title={p.minimise}
            onClick={minimise}
          >
            <svg viewBox="0 0 16 16" aria-hidden="true">
              <path d="M4 6l4 4 4-4" fill="none" stroke="currentColor" strokeWidth="1.8" />
            </svg>
          </button>
        </header>
        <p className="preference-offer-mod">
          <strong>{learned.mod}</strong>{" "}
          <Chip tone="info">
            {learned.kind === "favourite" && learned.topic !== null
              ? p.topic[learned.topic]
              : p.overChip(learned.over ?? "")}
          </Chip>
        </p>
        <p className="aside">{p.keptSentence(learned.chosen, learned.of)}</p>
        {listed && <PreferencePreview items={items} />}
        <div className="preference-offer-actions">
          {offers.length > 1 && (
            <span className="preference-pager">
              <button
                type="button"
                className="small quiet preference-icon-button"
                aria-label={p.previous}
                title={p.previous}
                onClick={() => step(-1)}
              >
                ‹
              </button>
              <span aria-live="polite">{p.pager(at + 1, offers.length)}</span>
              <button
                type="button"
                className="small quiet preference-icon-button"
                aria-label={p.next}
                title={p.next}
                onClick={() => step(1)}
              >
                ›
              </button>
            </span>
          )}
          <span className="preference-offer-spacer" />
          <button
            type="button"
            className="small"
            aria-expanded={listed}
            onClick={() => setListed(!listed)}
          >
            {listed ? p.hideList : p.showList}
          </button>
          <button
            type="button"
            className="small primary"
            disabled={busy || rule === undefined}
            onClick={apply}
          >
            {p.apply(items.length)}
          </button>
        </div>
      </>
    );
  }

  return (
    <div className="preference-corner">
      {shown && body !== null && (
        <aside
          id="preference-offer"
          ref={panel}
          tabIndex={-1}
          className="preference-offer"
          aria-labelledby={offer !== null && !working ? "preference-offer-title" : undefined}
          aria-live={working ? "polite" : undefined}
          onKeyDown={keys}
        >
          {body}
        </aside>
      )}
      <Mascot
        buttonRef={mascot}
        count={offers.length}
        open={shown}
        onClick={() => {
          if (!shown) {
            setOpen(true);
          } else if (preferences.batch !== null) {
            preferences.forgetBatch();
          } else {
            minimise();
          }
        }}
        p={p}
      />
    </div>
  );
}
