// The shared UI kit (ADR 0077): the head of a page, tabs and segmented
// switches, toolbars with a search, chips, notes, empty lists and the undo
// button. Pages build from these instead of drawing their own, so that
// every page looks and works like the others. The styles are in kit.css.

import type { ReactNode } from "react";
import { useSession } from "../Actions";
import { SearchIcon, ToneIcon, UndoIcon } from "./icons";
import { type Tone, uiTexts } from "./ui-texts";

export { LoadingLine, LoadingScreen } from "../LoadingMallet";
export type { Tone } from "./ui-texts";

/**
 * The head of a page: its tabs on the left, a short word on its state in
 * the middle and its main action on the right. The sidebar names the page,
 * so a head has no title. Nothing is drawn when it would be empty.
 */
export function PageHeader({
  tabs,
  status,
  actions,
}: {
  /** The page's parts, as `Tabs`. */
  tabs?: ReactNode;
  /** How far the page is, or how many it shows. */
  status?: ReactNode;
  /** The page's one main action, and at most one beside it. */
  actions?: ReactNode;
}) {
  if (!tabs && !status && !actions) {
    return null;
  }
  return (
    <header className="page-head">
      {tabs}
      {status && <div className="page-head-status">{status}</div>}
      {actions && <div className="page-head-actions">{actions}</div>}
    </header>
  );
}

/** One choice of a tab set or a segmented switch. */
export interface Choice<T extends string> {
  id: T;
  label: ReactNode;
  /** How many the choice holds, shown beside its name. */
  count?: number;
  /** What the choice means, in a tooltip. */
  hint?: string;
}

/** Switches between the parts of a page. */
export function Tabs<T extends string>({
  label,
  items,
  value,
  onChange,
}: {
  /** What the tabs choose between, for a screen reader. */
  label: string;
  items: Choice<T>[];
  value: T;
  onChange: (next: T) => void;
}) {
  return (
    <div className="tabs" role="tablist" aria-label={label}>
      {items.map((item) => (
        <button
          type="button"
          role="tab"
          className="tab"
          key={item.id}
          aria-selected={value === item.id}
          title={item.hint}
          onClick={() => onChange(item.id)}
        >
          {item.label}
          {item.count !== undefined && <span className="count">{item.count}</span>}
        </button>
      ))}
    </div>
  );
}

/** Changes how the same list is grouped, sorted or shown. */
export function Segmented<T extends string>({
  label,
  items,
  value,
  onChange,
  small = false,
  className,
}: {
  /** What the switch chooses, for a screen reader. */
  label: string;
  items: Choice<T>[];
  value: T;
  onChange: (next: T) => void;
  small?: boolean;
  className?: string;
}) {
  return (
    <fieldset
      className={className === undefined ? "segmented" : `segmented ${className}`}
      aria-label={label}
      data-size={small ? "small" : undefined}
    >
      {items.map((item) => (
        <button
          type="button"
          key={item.id}
          aria-pressed={value === item.id}
          title={item.hint}
          onClick={() => onChange(item.id)}
        >
          {item.label}
          {item.count !== undefined && <span className="count">{item.count}</span>}
        </button>
      ))}
    </fieldset>
  );
}

/** A row of filters and switches above a list; `stacked` in a narrow column. */
export function Toolbar({
  children,
  stacked = false,
  className,
}: {
  children: ReactNode;
  stacked?: boolean;
  className?: string;
}) {
  return (
    <div
      className={className === undefined ? "toolbar" : `toolbar ${className}`}
      data-stacked={stacked ? "" : undefined}
    >
      {children}
    </div>
  );
}

/** The one search field of the app: a magnifier and the words to look for. */
export function SearchField({
  value,
  onChange,
  placeholder,
  wide = false,
  onEnter,
}: {
  value: string;
  onChange: (next: string) => void;
  /** What can be searched for; also its name for a screen reader. */
  placeholder: string;
  /** Whether it takes the whole width of its column. */
  wide?: boolean;
  /** What Enter does, if anything: usually picking the first hit. */
  onEnter?: () => void;
}) {
  return (
    <span className="search-field" data-wide={wide ? "" : undefined}>
      <SearchIcon />
      <input
        type="search"
        placeholder={placeholder}
        aria-label={placeholder}
        value={value}
        onChange={(event) => onChange(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Enter" && onEnter !== undefined) {
            onEnter();
          }
        }}
      />
    </span>
  );
}

/** A short state in words, tinted by its tone; `icon` adds the tone's sign. */
export function Chip({
  tone = "neutral",
  icon = false,
  title,
  children,
}: {
  tone?: Tone;
  icon?: boolean;
  title?: string;
  children: ReactNode;
}) {
  return (
    <span className="chip" data-tone={tone} title={title}>
      {icon && <ToneIcon tone={tone} />}
      {children}
    </span>
  );
}

/**
 * Something the page says about itself, with the sign of its tone: a hint
 * (info), something done (ok), something to be careful about (warning) or
 * a problem (danger). A `role` of status or alert makes a screen reader say
 * it when it appears; alertdialog asks something.
 */
export function Note({
  tone = "info",
  role = "note",
  compact = false,
  actions,
  className,
  children,
}: {
  tone?: Tone;
  role?: "note" | "status" | "alert" | "alertdialog";
  /** One line with its buttons beside it, where room is short. */
  compact?: boolean;
  /** Buttons that do something about it, below the text. */
  actions?: ReactNode;
  className?: string;
  children: ReactNode;
}) {
  const { lang } = useSession();
  const said = uiTexts[lang].tone[tone];
  return (
    <div
      className={className === undefined ? "note" : `note ${className}`}
      data-tone={tone}
      data-compact={compact ? "" : undefined}
      role={role}
    >
      <ToneIcon tone={tone} />
      <div className="note-body">
        {said !== "" && <span className="visually-hidden">{said}: </span>}
        {children}
        {actions && <div className="note-actions">{actions}</div>}
      </div>
    </div>
  );
}

/** What a page or a list shows when there is nothing in it, and the way on. */
export function EmptyState({ children, action }: { children: ReactNode; action?: ReactNode }) {
  return (
    <div className="empty">
      <p>{children}</p>
      {action}
    </div>
  );
}

/** Takes back what was just changed: the same sign and word everywhere. */
export function UndoButton({
  onClick,
  disabled = false,
  label,
  small = true,
}: {
  onClick: () => void;
  disabled?: boolean;
  /** Where "Undo" alone does not say enough. */
  label?: string;
  small?: boolean;
}) {
  const { lang } = useSession();
  return (
    <button
      type="button"
      className={small ? "small undo-button" : "undo-button"}
      disabled={disabled}
      onClick={onClick}
    >
      <UndoIcon />
      {label ?? uiTexts[lang].undo}
    </button>
  );
}
