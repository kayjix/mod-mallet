import { useSession } from "./Actions";
import {
  compareFunctions,
  type DiffRow,
  diffLines,
  formatDate,
  formatSize,
  kindOf,
  type SideVersion,
  sameOnBothSides,
  sideVersions,
} from "./compare-files";
import { type CompareFilesTexts, useCompareFilesTexts } from "./compare-files-texts";
import "./compare-files.css";
import type { FileComparison, ScriptFunction } from "./generated/file-comparison";
import type { ModDuel } from "./generated/rendered-report";
import { LoadingLine } from "./LoadingMallet";

/**
 * A file that is neither a texture nor a mesh, in place of the picture
 * (ADR 0066): what kind of file it is and what it is for, what choosing a
 * side means for it, and what can be compared of the two versions: their
 * text line by line, a script's functions, or their size, date and
 * whether the bytes are the same.
 */
export function OtherFileStage({
  duel,
  names,
  path,
  comparison,
}: {
  duel: ModDuel;
  names: readonly [string, string];
  path: string;
  /** `undefined` while it is read, `null` if it could not be. */
  comparison: FileComparison | null | undefined;
}) {
  const c = useCompareFilesTexts();
  const kind = c.kinds[kindOf(path)];
  return (
    <div className="compare-other">
      <div className="compare-other-about">
        <p>
          <span className="tag">{kind.name}</span> {kind.purpose}
        </p>
        <p className="aside">
          {c.choiceMeaning} {kind.consequence}
        </p>
      </div>
      {comparison === undefined ? (
        <LoadingLine text={c.loading} />
      ) : comparison === null ? (
        <p className="aside">{c.cannotRead}</p>
      ) : (
        <Versions duel={duel} names={names} comparison={comparison} />
      )}
    </div>
  );
}

function Versions({
  duel,
  names,
  comparison,
}: {
  duel: ModDuel;
  names: readonly [string, string];
  comparison: FileComparison;
}) {
  const c = useCompareFilesTexts();
  const sides = sideVersions(duel, comparison);
  const same = sameOnBothSides(duel, comparison);
  const [a, b] = sides;
  return (
    <>
      {same !== null && (
        <p className="compare-other-verdict" data-same={same}>
          {same ? c.same : c.differs}
        </p>
      )}
      <div className="compare-other-sides">
        {([0, 1] as const).map((side) => (
          <SideFacts key={side} name={names[side]} found={sides[side]} />
        ))}
      </div>
      {same === false && a !== null && b !== null && <Difference a={a} b={b} names={names} />}
    </>
  );
}

/** Where one side's version comes from, its size and date. */
function SideFacts({ name, found }: { name: string; found: SideVersion | null }) {
  const { lang } = useSession();
  const c = useCompareFilesTexts();
  if (found === null) {
    return (
      <div className="compare-other-side">
        <strong>{name}</strong>
        <span className="aside">{c.noVersion}</span>
      </div>
    );
  }
  const { version, own } = found;
  const where = !own
    ? c.original
    : version.staged
      ? c.keptBack
      : version.archive != null
        ? c.inArchive(version.archive)
        : c.loose;
  return (
    <div className="compare-other-side">
      <strong>{name}</strong>
      <span className="aside">{where}</span>
      <span className="aside">
        {c.size(formatSize(version.size, lang))}
        {version.modified != null && ` · ${c.dated(formatDate(version.modified, lang))}`}
      </span>
    </div>
  );
}

/** What can be shown of how the two versions differ. */
function Difference({
  a,
  b,
  names,
}: {
  a: SideVersion;
  b: SideVersion;
  names: readonly [string, string];
}) {
  const c = useCompareFilesTexts();
  const [left, right] = [a.version, b.version];
  if (left.text != null && right.text != null) {
    return (
      <>
        {(left.textCut === true || right.textCut === true) && <p className="aside">{c.textCut}</p>}
        <TextDifference a={left.text} b={right.text} names={names} />
      </>
    );
  }
  if (left.image != null && right.image != null) {
    return (
      <div className="compare-flat">
        {[left.image, right.image].map((image, index) => (
          <figure key={names[index]}>
            <img className="compare-flat-image" src={image} alt={names[index]} />
            <figcaption>{names[index]}</figcaption>
          </figure>
        ))}
      </div>
    );
  }
  if (left.script != null && right.script != null) {
    if (!left.script.hasDebugInfo || !right.script.hasDebugInfo) {
      return <p className="aside">{c.noDebugInfo}</p>;
    }
    return <ScriptDifference a={left.script.functions} b={right.script.functions} names={names} />;
  }
  return null;
}

function TextDifference({
  a,
  b,
  names,
}: {
  a: string;
  b: string;
  names: readonly [string, string];
}) {
  const c = useCompareFilesTexts();
  const rows = diffLines(a, b);
  if (rows.every((row) => row.kind !== "changed")) {
    return <p className="aside">{c.noTextDifference}</p>;
  }
  return (
    <div className="text-diff">
      <table>
        <colgroup>
          <col className="text-diff-number" />
          <col />
          <col className="text-diff-number" />
          <col />
        </colgroup>
        <thead>
          <tr>
            <th colSpan={2}>{names[0]}</th>
            <th colSpan={2}>{names[1]}</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((row, index) => (
            // Rows have no identity of their own; their order never changes.
            // biome-ignore lint/suspicious/noArrayIndexKey: see above
            <DiffLine key={index} row={row} c={c} />
          ))}
        </tbody>
      </table>
    </div>
  );
}

function DiffLine({ row, c }: { row: DiffRow; c: CompareFilesTexts }) {
  if (row.kind === "skipped") {
    return (
      <tr className="text-diff-skipped">
        <td colSpan={4}>{c.sameLines(row.count)}</td>
      </tr>
    );
  }
  if (row.kind === "same") {
    return (
      <tr>
        <td className="text-diff-number">{row.left}</td>
        <td>{row.text}</td>
        <td className="text-diff-number">{row.right}</td>
        <td>{row.text}</td>
      </tr>
    );
  }
  return (
    <tr data-changed="true">
      <td className="text-diff-number">{row.left ?? ""}</td>
      <td data-side="a" data-empty={row.left === null}>
        {row.a}
      </td>
      <td className="text-diff-number">{row.right ?? ""}</td>
      <td data-side="b" data-empty={row.right === null}>
        {row.b}
      </td>
    </tr>
  );
}

function functionLabel(function_: ScriptFunction, c: CompareFilesTexts): string {
  const role =
    function_.kind === "getter"
      ? ` (${c.getter})`
      : function_.kind === "setter"
        ? ` (${c.setter})`
        : "";
  const state = function_.state === "" ? "" : ` – ${c.inState(function_.state)}`;
  return `${function_.name}${role}${state}`;
}

function ScriptDifference({
  a,
  b,
  names,
}: {
  a: ScriptFunction[];
  b: ScriptFunction[];
  names: readonly [string, string];
}) {
  const c = useCompareFilesTexts();
  const found = compareFunctions(a, b);
  const groups: [string, ScriptFunction[]][] = [
    [c.functionsDiffer, found.differ],
    [c.functionsOnlyIn(names[0]), found.onlyA],
    [c.functionsOnlyIn(names[1]), found.onlyB],
  ];
  const shown = groups.filter(([, functions]) => functions.length > 0);
  return (
    <div className="script-diff">
      {shown.length === 0 ? (
        <p className="aside">{c.functionsAllSame}</p>
      ) : (
        shown.map(([title, functions]) => (
          <div key={title}>
            <h4>{title}</h4>
            <ul>
              {functions.map((function_) => (
                <li key={`${function_.state}.${function_.name}.${function_.kind}`}>
                  <code>{functionLabel(function_, c)}</code>
                </li>
              ))}
            </ul>
          </div>
        ))
      )}
      {found.same > 0 && <p className="aside">{c.functionsSame(found.same)}</p>}
    </div>
  );
}

/**
 * Above the decision, for a duel only about such files: whether they are
 * all the same in both mods, and if so, the way to call it settled
 * without changing anything.
 */
export function DuelSameness({
  duel,
  comparisons,
}: {
  duel: ModDuel;
  comparisons: FileComparison[];
}) {
  const { decisions, decide } = useSession();
  const c = useCompareFilesTexts();
  const verdicts = comparisons.map((comparison) => sameOnBothSides(duel, comparison));
  const known = verdicts.filter((verdict) => verdict !== null);
  if (known.length === 0) {
    return null;
  }
  const differ = known.filter((verdict) => verdict === false).length;
  if (differ > 0) {
    return <p className="aside compare-sameness">{c.someDiffer(differ, known.length)}</p>;
  }
  if (known.length < verdicts.length) {
    return null;
  }
  const decided = decisions[duel.id] !== undefined;
  // Settled as it stands: the side the game takes the files from now.
  const now = duel.topics.find((topic) => topic.topic === "files")?.winning ?? 0;
  return (
    <div className="compare-sameness" data-same="true">
      <p>{decided ? c.markedNothing : c.allSame(known.length)}</p>
      {!decided && (
        <button type="button" className="primary small" onClick={() => decide(duel.id, now)}>
          {c.nothingToDecide}
        </button>
      )}
    </div>
  );
}
