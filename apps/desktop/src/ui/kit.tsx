// The shared UI kit (ADR 0077): the head of a page, tabs and segmented
// switches, toolbars with a search, chips, notes, empty lists and the undo
// button. Pages build from these instead of drawing their own, so that
// every page looks and works like the others. The styles are in kit.css.

import { type CSSProperties, type ReactNode, useState } from "react";
import { useSession } from "../Actions";
import { ChevronIcon, SearchIcon, SortIcon, TickIcon, ToneIcon, UndoIcon } from "./icons";
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

/**
 * A short state in words, tinted by its tone; `icon` adds the tone's sign.
 * With a `colour` it names something that has a colour of its own, such
 * as a kind of collision: a dot of that colour goes in front, and the words
 * say what it is, so that the colour is never alone.
 */
export function Chip({
  tone = "neutral",
  icon = false,
  colour,
  title,
  children,
}: {
  tone?: Tone;
  icon?: boolean;
  /** The colour of what it names, as a CSS colour. */
  colour?: string;
  title?: string;
  children: ReactNode;
}) {
  const style = colour === undefined ? undefined : ({ "--chip-colour": colour } as CSSProperties);
  return (
    <span className="chip" data-tone={tone} title={title} style={style}>
      {icon && <ToneIcon tone={tone} />}
      {colour !== undefined && <span className="chip-dot" aria-hidden="true" />}
      {children}
    </span>
  );
}

/**
 * Something the page says about itself, with the sign of its tone: a hint
 * (info), something done (ok), something to be careful about (warning) or
 * a problem (danger). A `role` of status or alert makes a screen reader say
 * it when it appears; alertdialog asks something. With a `summary` it is
 * one line that opens to the rest.
 */
export function Note({
  tone = "info",
  role = "note",
  compact = false,
  summary,
  actions,
  className,
  children,
}: {
  tone?: Tone;
  role?: "note" | "status" | "alert" | "alertdialog";
  /** One line with its buttons beside it, where room is short. */
  compact?: boolean;
  /** The one line shown while closed; the children are what opens below it. */
  summary?: ReactNode;
  /** Buttons that do something about it, below the text. */
  actions?: ReactNode;
  className?: string;
  children: ReactNode;
}) {
  const { lang } = useSession();
  const said = uiTexts[lang].tone[tone];
  if (summary !== undefined) {
    return (
      <details
        className={className === undefined ? "note" : `note ${className}`}
        data-tone={tone}
        data-opens=""
        role={role}
      >
        <summary className="note-summary-line">
          <ToneIcon tone={tone} />
          {said !== "" && <span className="visually-hidden">{said}: </span>}
          <span className="note-summary-text">{summary}</span>
          <span className="note-more" title={uiTexts[lang].more}>
            <ChevronIcon />
          </span>
        </summary>
        <div className="note-body">
          {children}
          {actions && <div className="note-actions">{actions}</div>}
        </div>
      </details>
    );
  }
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

/**
 * A switch that is one chip: its colour, its name and its count together.
 * On it is filled and ticked, off it is an empty ring and muted, so that
 * on and off read without the colour.
 */
export function ToggleChip({
  on,
  onChange,
  label,
  count,
  colour,
  icon,
  hint,
}: {
  on: boolean;
  onChange: (on: boolean) => void;
  label: ReactNode;
  count?: number;
  /** The colour of what it shows, such as a kind of collision; the brand's if none. */
  colour?: string;
  /** A sign after the ring, for what is not a colour. */
  icon?: ReactNode;
  hint?: string;
}) {
  const style = colour === undefined ? undefined : ({ "--chip-colour": colour } as CSSProperties);
  return (
    <button
      type="button"
      className="toggle-chip"
      aria-pressed={on}
      title={hint}
      style={style}
      onClick={() => onChange(!on)}
    >
      <span className="toggle-chip-mark" aria-hidden="true">
        {on && <TickIcon />}
      </span>
      {icon}
      <span className="toggle-chip-label">{label}</span>
      {count !== undefined && <span className="toggle-chip-count">{count}</span>}
    </button>
  );
}

/** One order of a list that the sorting button steps through. */
export interface SortOrder<S extends string> {
  id: S;
  /** What the order is, as the tooltip and a screen reader say it. */
  label: string;
  /** Whether the smallest come first. */
  up: boolean;
}

/**
 * The search above a list, with its sorting at its end: one button that
 * steps through the orders, its arrow showing which way, its tooltip
 * saying which.
 */
export function SearchSort<S extends string>({
  value,
  onChange,
  placeholder,
  orders,
  order,
  onOrder,
}: {
  value: string;
  onChange: (next: string) => void;
  placeholder: string;
  orders: SortOrder<S>[];
  order: S;
  onOrder: (next: S) => void;
}) {
  const { lang } = useSession();
  const at = Math.max(
    0,
    orders.findIndex((candidate) => candidate.id === order),
  );
  const now = orders[at];
  const next = orders[(at + 1) % orders.length];
  const said = now === undefined ? "" : uiTexts[lang].sortedBy(now.label);
  return (
    <span className="search-field search-sort" data-wide="">
      <SearchIcon />
      <input
        type="search"
        placeholder={placeholder}
        aria-label={placeholder}
        value={value}
        onChange={(event) => onChange(event.target.value)}
      />
      {now !== undefined && next !== undefined && (
        <button
          type="button"
          className="search-sort-button"
          title={said}
          aria-label={said}
          onClick={() => onOrder(next.id)}
        >
          <SortIcon up={now.up} />
        </button>
      )}
    </span>
  );
}

/**
 * A tool with levels in one button: a click turns it on or off, the pips
 * inside set its level, filled up to the level it has. Off is level 0;
 * turning it on again goes back to the level it had.
 */
export function LevelToggle({
  icon,
  label,
  level,
  levels,
  onLevel,
  hint,
  onPreview,
}: {
  icon?: ReactNode;
  label: string;
  level: number;
  /** What each level does, from off (0) to the highest. */
  levels: string[];
  onLevel: (level: number) => void;
  hint?: string;
  /**
   * Called with the level of the pip pointed at or focused, to show what
   * it would do before it is set; null when none is.
   */
  onPreview?: (level: number | null) => void;
}) {
  const [lastOn, setLastOn] = useState(level > 0 ? level : 1);
  const set = (next: number) => {
    if (next > 0) {
      setLastOn(next);
    }
    onLevel(next);
  };
  const top = levels.length - 1;
  return (
    <span className="level-toggle" data-on={level > 0 || undefined} title={levels[level]}>
      <button
        type="button"
        className="level-toggle-main"
        aria-pressed={level > 0}
        title={hint}
        onClick={() => set(level > 0 ? 0 : lastOn)}
      >
        {icon}
        {label}
      </button>
      <span className="level-toggle-pips" onPointerLeave={() => onPreview?.(null)}>
        {Array.from({ length: top }, (_, index) => index + 1).map((step) => (
          <button
            type="button"
            key={step}
            className="level-pip"
            aria-label={levels[step]}
            aria-pressed={step <= level}
            title={levels[step]}
            onClick={() => set(step === level ? 0 : step)}
            onPointerEnter={() => onPreview?.(step)}
            onFocus={() => onPreview?.(step)}
            onBlur={() => onPreview?.(null)}
          />
        ))}
      </span>
    </span>
  );
}
