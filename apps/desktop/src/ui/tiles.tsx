// Tiles that are tabs (ADR 0082): a small board of categories, each with
// its count and a line on what it means, that switches the list under it.
// Part of the UI kit (ADR 0077); it knows nothing about findings.

import { type KeyboardEvent, type ReactNode, useRef } from "react";
import "./tiles.css";

/** One tile of a board. */
export interface Tile<T extends string> {
  id: T;
  label: string;
  /** How many the category holds; the tile's big number. */
  count: number;
  /** What the category means, in one short line. */
  meaning: string;
  /** Something that stands out about it, such as how many can crash the game. */
  badge?: ReactNode;
  /** A category that is only to know or set aside steps back. */
  quiet?: boolean;
}

/**
 * Tiles in a grid that work as tabs: the chosen one is marked, the arrow
 * keys move between them, Home and End go to the first and last.
 */
export function TileTabs<T extends string>({
  label,
  tiles,
  value,
  onChange,
  panelId,
}: {
  /** What the tiles choose between, for a screen reader. */
  label: string;
  tiles: Tile<T>[];
  value: T;
  onChange: (next: T) => void;
  /** The element the tiles switch. */
  panelId: string;
}) {
  const board = useRef<HTMLDivElement>(null);
  const move = (event: KeyboardEvent<HTMLButtonElement>, index: number) => {
    const last = tiles.length - 1;
    const next =
      event.key === "ArrowRight" || event.key === "ArrowDown"
        ? Math.min(last, index + 1)
        : event.key === "ArrowLeft" || event.key === "ArrowUp"
          ? Math.max(0, index - 1)
          : event.key === "Home"
            ? 0
            : event.key === "End"
              ? last
              : null;
    const tile = next === null ? undefined : tiles[next];
    if (tile === undefined) {
      return;
    }
    event.preventDefault();
    onChange(tile.id);
    board.current?.querySelectorAll<HTMLButtonElement>("[role=tab]")[next ?? 0]?.focus();
  };
  return (
    <div className="tile-tabs" role="tablist" aria-label={label} ref={board}>
      {tiles.map((tile, index) => {
        const chosen = tile.id === value;
        return (
          <button
            type="button"
            role="tab"
            key={tile.id}
            className="tile-tab"
            data-quiet={tile.quiet === true}
            aria-selected={chosen}
            aria-controls={panelId}
            tabIndex={chosen ? 0 : -1}
            onClick={() => onChange(tile.id)}
            onKeyDown={(event) => move(event, index)}
          >
            <span className="tile-tab-top">
              <span className="tile-tab-label">{tile.label}</span>
              <span className="tile-tab-count">{tile.count}</span>
            </span>
            <span className="tile-tab-meaning">{tile.meaning}</span>
            {tile.badge !== undefined && <span className="tile-tab-badge">{tile.badge}</span>}
          </button>
        );
      })}
    </div>
  );
}
