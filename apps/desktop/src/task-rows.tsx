// The rows of a list of things to do (ADR 0076): one row per task, one row
// for several tasks of one sort, the mark that says how bad it is and the
// badges of the mods it is about. They know nothing about findings; the
// task page and the other lists fill them in. Kept apart so that they can
// move into the window's shared components.

import { type CSSProperties, type KeyboardEvent, type ReactNode, useId } from "react";
import { hueOf, initialsOf, isRowKey, nextRow } from "./task-plan.ts";
import "./task-rows.css";

/** How bad a task is, as its mark shows it; `done` once it is handed on. */
export type MarkLevel = "crash" | "broken" | "quality" | "scan" | "note" | "advice" | "done";

/**
 * The mark in front of a row: a shape for each level, so that it does not
 * rest on colour alone, with the level in words for screen readers and as
 * a tooltip.
 */
export function SeverityMark({ level, label }: { level: MarkLevel; label: string }) {
  return (
    <span className="severity-mark" data-level={level} title={label}>
      <svg viewBox="0 0 16 16" width="18" height="18" aria-hidden="true" focusable="false">
        {markShape(level)}
      </svg>
      <span className="visually-hidden">{label}</span>
    </span>
  );
}

function markShape(level: MarkLevel): ReactNode {
  switch (level) {
    case "crash":
      // A stop sign with an exclamation mark.
      return (
        <>
          <polygon
            className="mark-fill"
            points="5,0.8 11,0.8 15.2,5 15.2,11 11,15.2 5,15.2 0.8,11 0.8,5"
          />
          <rect className="mark-ink" x="7.1" y="3.6" width="1.8" height="5.6" rx="0.9" />
          <circle className="mark-ink" cx="8" cy="11.6" r="1.1" />
        </>
      );
    case "broken":
      // A warning triangle.
      return (
        <>
          <path
            className="mark-fill"
            d="M8 1.2 L15.3 14.2 Q15.6 14.9 14.8 14.9 H1.2 Q0.4 14.9 0.7 14.2 Z"
          />
          <rect className="mark-ink" x="7.1" y="5.4" width="1.8" height="4.8" rx="0.9" />
          <circle className="mark-ink" cx="8" cy="12.4" r="1.05" />
        </>
      );
    case "quality":
      // A diamond.
      return <path className="mark-fill" d="M8 1.5 L14.5 8 L8 14.5 L1.5 8 Z" />;
    case "scan":
      // A square with a gap: something could not be read.
      return (
        <>
          <rect className="mark-fill" x="1.5" y="1.5" width="13" height="13" rx="3" />
          <rect className="mark-ink" x="4.5" y="7.1" width="7" height="1.8" rx="0.9" />
        </>
      );
    case "note":
      // An "i" in a ring.
      return (
        <>
          <circle className="mark-ring" cx="8" cy="8" r="6.6" />
          <rect className="mark-dot" x="7.15" y="6.8" width="1.7" height="5" rx="0.85" />
          <circle className="mark-dot" cx="8" cy="4.6" r="1.05" />
        </>
      );
    case "advice":
      // A plus in a dashed ring: something that may be added.
      return (
        <>
          <circle className="mark-ring" cx="8" cy="8" r="6.6" strokeDasharray="3 2" />
          <rect className="mark-dot" x="7.15" y="4.5" width="1.7" height="7" rx="0.85" />
          <rect className="mark-dot" x="4.5" y="7.15" width="7" height="1.7" rx="0.85" />
        </>
      );
    case "done":
      // A tick in a disc.
      return (
        <>
          <circle className="mark-fill" cx="8" cy="8" r="7.2" />
          <path className="mark-tick" d="M4.6 8.3 L7 10.6 L11.5 5.8" />
        </>
      );
  }
}

/** The badge of one mod: its initials on a colour of its own. */
export function ModBadge({ name }: { name: string }) {
  return (
    <span
      className="mod-badge"
      style={{ "--mod-hue": hueOf(name) } as CSSProperties}
      aria-hidden="true"
    >
      {initialsOf(name)}
    </span>
  );
}

/**
 * The mods a row is about: up to `shown` badges and names, and how many
 * more there are. With `names` false only the badges show, and the names
 * are in their tooltip.
 */
export function ModLine({
  mods,
  shown = 2,
  names = true,
  more,
}: {
  mods: string[];
  shown?: number;
  names?: boolean;
  /** "+3 more", in the window's language. */
  more: (count: number) => string;
}) {
  if (mods.length === 0) {
    return null;
  }
  const visible = mods.slice(0, shown);
  const rest = mods.length - visible.length;
  return (
    <span className="mod-line" title={mods.join("\n")}>
      <span className="mod-badges">
        {visible.map((name) => (
          <ModBadge key={name} name={name} />
        ))}
      </span>
      {names ? (
        <span className="mod-names">{visible.join(" · ")}</span>
      ) : (
        <span className="visually-hidden">{visible.join(", ")}</span>
      )}
      {rest > 0 && <span className="mod-more">{more(rest)}</span>}
    </span>
  );
}

/**
 * Moves the focus between the rows of the list around `from` on the arrow
 * keys, Home and End. The rows are the buttons that open them; the list is
 * the nearest element marked `data-row-scope`.
 */
function moveBetweenRows(event: KeyboardEvent<HTMLButtonElement>) {
  if (!isRowKey(event.key) || event.altKey || event.ctrlKey || event.metaKey) {
    return;
  }
  const scope = event.currentTarget.closest("[data-row-scope]");
  if (scope === null) {
    return;
  }
  const rows = [...scope.querySelectorAll<HTMLButtonElement>("[data-row-toggle]")];
  const target = rows[nextRow(rows.indexOf(event.currentTarget), rows.length, event.key)];
  if (target !== undefined) {
    event.preventDefault();
    target.focus();
  }
}

/** Opens a row on the right arrow key, closes it on the left one. */
function openOrClose(event: KeyboardEvent<HTMLButtonElement>, open: boolean, onToggle: () => void) {
  if ((event.key === "ArrowRight" && !open) || (event.key === "ArrowLeft" && open)) {
    event.preventDefault();
    onToggle();
  }
}

/**
 * One task: its mark, a short title, the mods it is about, the one thing
 * to do, and a button that opens the rest underneath. The whole row opens
 * on a click; the button is what the keyboard reaches.
 */
export function TaskRow({
  level,
  levelLabel,
  title,
  meta,
  primary,
  open,
  onToggle,
  toggleLabel,
  nested = false,
  children,
}: {
  level: MarkLevel;
  levelLabel: string;
  title: ReactNode;
  /** The line under the title: the mods, and what became of the task. */
  meta?: ReactNode;
  /** The one thing to do, as a button. */
  primary?: ReactNode;
  open: boolean;
  onToggle: () => void;
  /** "Details", in the window's language. */
  toggleLabel: string;
  /** Inside a group: no mark of its own when it says the same as the group's. */
  nested?: boolean;
  children?: ReactNode;
}) {
  const bodyId = useId();
  return (
    <li className="task-row" data-level={level} data-open={open} data-nested={nested}>
      <div className="task-row-head">
        <SeverityMark level={level} label={levelLabel} />
        <div className="task-row-text">
          <span className="task-row-title">{title}</span>
          {meta !== undefined && <span className="task-row-meta">{meta}</span>}
        </div>
        <div className="task-row-actions">
          {primary}
          <button
            type="button"
            className="task-row-toggle"
            data-row-toggle=""
            aria-expanded={open}
            aria-controls={bodyId}
            onClick={onToggle}
            onKeyDown={(event) => {
              moveBetweenRows(event);
              openOrClose(event, open, onToggle);
            }}
          >
            <span className="task-row-toggle-label">{toggleLabel}</span>
            <span className="task-chevron" aria-hidden="true" />
          </button>
        </div>
      </div>
      {open && (
        <div className="task-row-body" id={bodyId}>
          {children}
        </div>
      )}
    </li>
  );
}

/**
 * Several tasks of one sort behind one row: their mark, the sort's title,
 * the mods they are about, how many there are, and the one thing to do
 * for all of them where there is one. It opens into the rows of the tasks.
 */
export function TaskGroup({
  level,
  levelLabel,
  title,
  meta,
  primary,
  count,
  open,
  onToggle,
  toggleLabel,
  children,
}: {
  level: MarkLevel;
  levelLabel: string;
  title: ReactNode;
  meta?: ReactNode;
  /** The one thing to do for all of them, where there is one. */
  primary?: ReactNode;
  count: number;
  open: boolean;
  onToggle: () => void;
  /** "Show all 12" or "Fold", in the window's language. */
  toggleLabel: string;
  children: ReactNode;
}) {
  const listId = useId();
  return (
    <li className="task-group" data-level={level} data-open={open}>
      <div className="task-row-head task-group-head">
        <SeverityMark level={level} label={levelLabel} />
        <div className="task-row-text">
          <span className="task-row-title">
            {title} <span className="count task-group-count">{count}</span>
          </span>
          {meta !== undefined && <span className="task-row-meta">{meta}</span>}
        </div>
        <div className="task-row-actions">
          {primary}
          <button
            type="button"
            className="task-row-toggle"
            data-row-toggle=""
            aria-expanded={open}
            aria-controls={listId}
            onClick={onToggle}
            onKeyDown={(event) => {
              moveBetweenRows(event);
              openOrClose(event, open, onToggle);
            }}
          >
            <span className="task-row-toggle-label">{toggleLabel}</span>
            <span className="task-chevron" aria-hidden="true" />
          </button>
        </div>
      </div>
      {open && (
        <ul className="task-group-rows" id={listId}>
          {children}
        </ul>
      )}
    </li>
  );
}

/**
 * A part of the task list: a number for the order to work in (none for
 * what is optional), a title, a count, and its rows. Folded parts open on
 * their button.
 */
export function TaskPart({
  number,
  title,
  count,
  tag,
  folded,
  open,
  onToggle,
  toggleLabel,
  id,
  children,
}: {
  number?: number;
  title: string;
  count: number;
  /** A word beside the title, such as "optional". */
  tag?: string;
  /** Whether the part can be folded; only then is there a button. */
  folded?: boolean;
  open: boolean;
  onToggle?: () => void;
  toggleLabel?: string;
  id: string;
  children: ReactNode;
}) {
  const headingId = useId();
  return (
    <section className="task-part" data-part={id} aria-labelledby={headingId}>
      <header className="task-part-head">
        {number !== undefined && (
          <span className="task-part-number" aria-hidden="true">
            {number}
          </span>
        )}
        <h2 id={headingId}>
          {title} {count > 0 && <span className="count">{count}</span>}
        </h2>
        {tag !== undefined && <span className="tag task-part-tag">{tag}</span>}
        {folded === true && onToggle !== undefined && (
          <button
            type="button"
            className="small quiet task-part-toggle"
            aria-expanded={open}
            onClick={onToggle}
          >
            {toggleLabel}
          </button>
        )}
      </header>
      {open && children}
    </section>
  );
}
