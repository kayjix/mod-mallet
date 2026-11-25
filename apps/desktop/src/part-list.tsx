// The parts at a place, as a list to pick from (ADR 0080, 0083): by mod,
// in its colour, and within a mod parts of the same name in one row with
// a count, what kind of thing they are and what is known of them (resting
// on another, tied into the game, walked on), opened one by one where
// wanted. Rows are picked like files (click, Ctrl, Shift), then hidden or
// shown again together. Every row locates its parts in the views, and
// pointing at a row lights them up there.

import { type MouseEvent, useMemo, useRef, useState } from "react";
import { useCollisionStepTexts } from "./collision-step-texts";
import { InfoTip } from "./info-tip";
import { LocateButton } from "./locate";
import { groupByName, type PartKind, type PickHow, pickRows } from "./part-groups";
import { Eye } from "./view-tools";
import { PathMark } from "./walkways";
import "./part-list.css";

/** What is known of a part that matters for hiding it. */
export type PartNote = "rests" | "tied" | "needed" | "walked" | "game";

/** A part as the list shows it. */
export interface ListedPart {
  key: string;
  id: { plugin: string; localId: number };
  name: string;
  /** Whose it is: the first mod or the second. */
  side: 0 | 1;
  /** What kind of thing it is, from its model. */
  kind: PartKind;
  /** What is known of it, such as resting on another part. */
  notes: PartNote[];
  hidden: boolean;
  /** Why it cannot be hidden, if it cannot: the game's own, or the game needs it. */
  stays: "game" | "locked" | null;
  /** The pieces of path characters walk on over it. */
  walked: number;
}

/** One row of the list: a mod's head, a name with all its parts, or one part of a name opened. */
type Row =
  | { type: "side"; id: string; side: 0 | 1; count: number }
  | {
      type: "parts";
      id: string;
      side: 0 | 1;
      label: string;
      parts: ListedPart[];
      /** Whether it is one part of a name opened one by one. */
      member: boolean;
      /** For a name of several parts: whether it is opened. */
      open: boolean | null;
    };

/** The notes in the order a row shows them. */
const NOTES: PartNote[] = ["needed", "game", "tied", "walked", "rests"];

export function PartList({
  parts,
  names,
  picked,
  onPicked,
  onHide,
  onShow,
  onHover,
}: {
  parts: ListedPart[];
  /** The two mods' names, to head their parts where the list holds both. */
  names: readonly [string, string];
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
  const bySide = useMemo(
    () =>
      ([0, 1] as const).map((side) => groupByName(parts.filter((part) => part.side === side))) as [
        ReturnType<typeof groupByName<ListedPart>>,
        ReturnType<typeof groupByName<ListedPart>>,
      ],
    [parts],
  );
  const both = bySide[0].length > 0 && bySide[1].length > 0;
  const rows: Row[] = ([0, 1] as const).flatMap((side): Row[] => {
    const groups = bySide[side];
    if (groups.length === 0) {
      return [];
    }
    const head: Row[] = both
      ? [
          {
            type: "side",
            id: `side:${side}`,
            side,
            count: groups.reduce((sum, group) => sum + group.members.length, 0),
          },
        ]
      : [];
    return [
      ...head,
      ...groups.flatMap((group): Row[] => {
        const id = `${side}:${group.name}`;
        const many = group.members.length > 1;
        const open = many && opened.has(id);
        const row: Row = {
          type: "parts",
          id: `name:${id}`,
          side,
          label: group.name,
          parts: group.members,
          member: false,
          open: many ? open : null,
        };
        return open
          ? [
              row,
              ...group.members.map(
                (part, index): Row => ({
                  type: "parts",
                  id: `part:${part.key}`,
                  side,
                  label: c.member(group.name, index + 1),
                  parts: [part],
                  member: true,
                  open: null,
                }),
              ),
            ]
          : [row];
      }),
    ];
  });
  const rowKeys = rows.map((row) =>
    row.type === "parts" ? row.parts.map((part) => part.key) : [],
  );
  const mine = parts.filter((part) => picked.has(part.key));
  const canHide = mine.filter((part) => !part.hidden && part.stays === null);
  const canShow = mine.filter((part) => part.hidden);
  const hiddenCount = parts.filter((part) => part.hidden).length;

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
  const toggleOpen = (id: string) => {
    const next = new Set(opened);
    if (next.has(id)) {
      next.delete(id);
    } else {
      next.add(id);
    }
    anchor.current = null;
    setOpened(next);
  };

  return (
    <div className="part-list">
      {/* One line of the same height, whether something is picked or not. */}
      <div className="part-list-bar">
        {mine.length === 0 ? (
          <>
            <span className="aside">
              {c.partsCount(parts.length)}
              {hiddenCount > 0 && ` · ${c.hiddenCount(hiddenCount)}`}
            </span>
            <InfoTip text={c.listHint} />
          </>
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
          if (row.type === "side") {
            return (
              <li key={row.id} className="part-list-side">
                <strong className="stage3-swatch" data-side={row.side}>
                  {names[row.side]}
                </strong>
                <span className="aside">{c.partsCount(row.count)}</span>
              </li>
            );
          }
          const hidden = row.parts.filter((part) => part.hidden).length;
          const stays = row.parts.every((part) => part.stays !== null)
            ? (row.parts[0]?.stays ?? null)
            : null;
          const walked = row.parts.reduce((sum, part) => sum + part.walked, 0);
          const isPicked = row.parts.every((part) => picked.has(part.key));
          const partly = !isPicked && row.parts.some((part) => picked.has(part.key));
          const hideable = row.parts.filter((part) => !part.hidden && part.stays === null);
          const allHidden = hidden === row.parts.length;
          const kind = row.parts[0]?.kind ?? "other";
          const notes = NOTES.filter(
            (note) =>
              note !== "walked" &&
              row.parts.some((part) => part.notes.includes(note)) &&
              !(note === "needed" && stays === "locked") &&
              !(note === "game" && stays === "game"),
          );
          return (
            <li
              key={row.id}
              className="part-row"
              data-side={row.side}
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
                  onClick={() => toggleOpen(row.id.replace(/^name:/, ""))}
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
                {row.parts.length > 1 && (
                  <span className="part-row-count">×{row.parts.length}</span>
                )}
                {!row.member && kind !== "other" && (
                  <span className="part-row-kind">{c.kind[kind]}</span>
                )}
              </button>
              <span className="part-row-state">
                {notes.map((note) => (
                  <span key={note} className="part-row-note" data-note={note}>
                    {c.note[note]}
                  </span>
                ))}
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
