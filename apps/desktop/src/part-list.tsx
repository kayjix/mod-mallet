// One mod's parts at a place, as a list to pick from (ADR 0080): parts of
// the same name in one row with a count, opened one by one where wanted;
// rows picked like files (click, Ctrl, Shift), then hidden or shown again
// together. Every row locates its parts in the views, and pointing at a
// row lights them up there.

import { type MouseEvent, useMemo, useRef, useState } from "react";
import { useCollisionStepTexts } from "./collision-step-texts";
import { LocateButton } from "./locate";
import { groupByName, type PickHow, pickRows } from "./part-groups";
import { Eye } from "./view-tools";
import { PathMark } from "./walkways";
import "./part-list.css";

/** A part as the list shows it. */
export interface ListedPart {
  key: string;
  id: { plugin: string; localId: number };
  name: string;
  hidden: boolean;
  /** Why it cannot be hidden, if it cannot: the game's own, or the game needs it. */
  stays: "game" | "locked" | null;
  /** The pieces of path characters walk on over it. */
  walked: number;
}

/** One row of the list: a name with all its parts, or one part of a name opened. */
interface Row {
  id: string;
  label: string;
  count: number;
  parts: ListedPart[];
  /** Whether it is one part of a name opened one by one. */
  member: boolean;
  /** For a name of several parts: whether it is opened. */
  open: boolean | null;
}

export function PartList({
  parts,
  side,
  picked,
  onPicked,
  onHide,
  onShow,
  onHover,
}: {
  parts: ListedPart[];
  /** Whose parts they are: the first mod or the second, for its colour. */
  side: 0 | 1;
  /** What is picked, by key; may hold parts of other lists too. */
  picked: ReadonlySet<string>;
  onPicked: (next: Set<string>) => void;
  /** Hides these parts; only parts that can be hidden are passed. */
  onHide: (parts: ListedPart[]) => void;
  /** Shows these parts again. */
  onShow: (parts: ListedPart[]) => void;
  /** Told which parts the pointer is on, to light them up in the views; null when it left. */
  onHover: (keys: string[] | null) => void;
}) {
  const c = useCollisionStepTexts();
  const [opened, setOpened] = useState<Set<string>>(new Set());
  const anchor = useRef<number | null>(null);
  const groups = useMemo(() => groupByName(parts), [parts]);
  const rows: Row[] = groups.flatMap((group): Row[] => {
    const many = group.members.length > 1;
    const open = many && opened.has(group.name);
    const head: Row = {
      id: `name:${group.name}`,
      label: group.name,
      count: group.members.length,
      parts: group.members,
      member: false,
      open: many ? open : null,
    };
    return open
      ? [
          head,
          ...group.members.map((part, index) => ({
            id: `part:${part.key}`,
            label: c.member(group.name, index + 1),
            count: 1,
            parts: [part],
            member: true,
            open: null,
          })),
        ]
      : [head];
  });
  const rowKeys = rows.map((row) => row.parts.map((part) => part.key));
  const mine = parts.filter((part) => picked.has(part.key));
  const canHide = mine.filter((part) => !part.hidden && part.stays === null);
  const canShow = mine.filter((part) => part.hidden);

  const click = (event: MouseEvent, index: number) => {
    const how: PickHow = event.shiftKey
      ? "range"
      : event.ctrlKey || event.metaKey
        ? "toggle"
        : "only";
    // A plain click picks within this list; picks of other lists go.
    const own = new Set(rowKeys.flat());
    const current = how === "only" ? new Set([...picked].filter((key) => own.has(key))) : picked;
    const next = pickRows(current, rowKeys, anchor.current, index, how);
    if (how !== "range") {
      anchor.current = index;
    }
    onPicked(next);
  };
  const toggleOpen = (name: string) => {
    const next = new Set(opened);
    if (next.has(name)) {
      next.delete(name);
    } else {
      next.add(name);
    }
    anchor.current = null;
    setOpened(next);
  };

  return (
    <div className="part-list" data-side={side}>
      <div className="part-list-bar">
        {mine.length === 0 ? (
          <span className="aside">{c.listHint}</span>
        ) : (
          <>
            <strong>{c.picked(mine.length)}</strong>
            <button
              type="button"
              className="small"
              disabled={canHide.length === 0}
              onClick={() => onHide(canHide)}
            >
              {c.hidePicked}
            </button>
            <button
              type="button"
              className="small"
              disabled={canShow.length === 0}
              onClick={() => onShow(canShow)}
            >
              {c.showPicked}
            </button>
            <button
              type="button"
              className="small quiet"
              title={c.clearPicked}
              aria-label={c.clearPicked}
              onClick={() =>
                onPicked(
                  new Set([...picked].filter((key) => !mine.some((part) => part.key === key))),
                )
              }
            >
              ×
            </button>
          </>
        )}
      </div>
      <ul className="part-list-rows" onMouseLeave={() => onHover(null)}>
        {rows.map((row, index) => {
          const hidden = row.parts.filter((part) => part.hidden).length;
          const stays = row.parts.every((part) => part.stays !== null)
            ? (row.parts[0]?.stays ?? null)
            : null;
          const walked = row.parts.reduce((sum, part) => sum + part.walked, 0);
          const isPicked = row.parts.every((part) => picked.has(part.key));
          const partly = !isPicked && row.parts.some((part) => picked.has(part.key));
          const hideable = row.parts.filter((part) => !part.hidden && part.stays === null);
          const allHidden = hidden === row.parts.length;
          return (
            <li
              key={row.id}
              className="part-row"
              data-member={row.member ? "" : undefined}
              data-picked={isPicked ? "all" : partly ? "some" : undefined}
              data-hidden={allHidden ? "" : undefined}
              onMouseEnter={() => onHover(row.parts.map((part) => part.key))}
            >
              {row.open !== null ? (
                <button
                  type="button"
                  className="part-row-open quiet"
                  aria-expanded={row.open}
                  title={c.oneByOne}
                  aria-label={`${c.oneByOne}: ${row.label}`}
                  onClick={() => toggleOpen(row.label)}
                >
                  <span aria-hidden="true">{row.open ? "▾" : "▸"}</span>
                </button>
              ) : (
                <span className="part-row-open" />
              )}
              <button
                type="button"
                className="part-row-pick"
                aria-pressed={isPicked}
                title={row.label}
                onClick={(event) => click(event, index)}
              >
                <span className="part-row-name">{row.label}</span>
                {row.count > 1 && <span className="part-row-count">×{row.count}</span>}
              </button>
              <span className="part-row-state">
                {walked > 0 && <PathMark triangles={walked} />}
                {stays === "game" ? (
                  <span className="aside">{c.gameOwn}</span>
                ) : stays === "locked" ? (
                  <span className="aside">{c.stays}</span>
                ) : hidden > 0 ? (
                  <span className="part-row-hidden">{c.hiddenOf(hidden, row.parts.length)}</span>
                ) : null}
              </span>
              <LocateButton objects={row.parts.map((part) => part.id)} />
              <button
                type="button"
                className="small quiet part-row-eye"
                disabled={allHidden ? false : hideable.length === 0}
                aria-pressed={allHidden}
                title={allHidden ? c.showOne(row.label) : c.hideOne(row.label)}
                aria-label={allHidden ? c.showOne(row.label) : c.hideOne(row.label)}
                onClick={() =>
                  allHidden ? onShow(row.parts.filter((part) => part.hidden)) : onHide(hideable)
                }
              >
                <Eye open={!allHidden} />
              </button>
            </li>
          );
        })}
      </ul>
    </div>
  );
}
