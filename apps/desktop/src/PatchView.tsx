import { type ChangeEvent, useEffect, useMemo, useRef, useState } from "react";
import { ActionResult, type PatchAt, useAction, useSession } from "./Actions";
import type { BookPatch, PatchBook } from "./generated/patch-book";
import type { MapSpot, ObjectRef, RenderedReport } from "./generated/rendered-report";
import { InfoTip } from "./info-tip";
import { MovesPanel } from "./move-controls";
import { PairReport } from "./PairReport";
import {
  choicesOf,
  matchKey,
  objectsOf,
  openSpots,
  type Pair,
  type PatchChoices,
  pairsOf,
  spotsByKey,
} from "./patch";
import {
  activePatch,
  conflictsOf,
  decisionsOfObjects,
  duelsOf,
  installedObjectsOf,
  isForeign,
  noRepairKey,
  type PatchConflict,
  patchMod,
  repairAreasOf,
  stateOf,
  tickedOf,
  withDuplicate,
  withLoaded,
  withMoved,
  withNewPatch,
  withoutPatch,
  withPatch,
  withTicked,
  writtenPatch,
} from "./patch-book";
import {
  type Entry,
  entriesOf,
  GROUPS,
  modNamer,
  modsOf,
  type Target,
  type TouchedMod,
} from "./patch-entries";
import { type PatchTexts, patchTexts } from "./patch-texts";
import { PathsPanel } from "./repair-paths";
import { NOTHING_WAITS } from "./switched-off";
import { switchedOffTexts } from "./switched-off-texts";
import "./patches.css";
import "./wide-advice.css";
import { EmptyState, Note, UndoButton } from "./ui/kit";

/** Base Object Swapper's page, for when it is missing. */
const SWAPPER_PAGE = "https://www.nexusmods.com/skyrimspecialedition/mods/60805";

/** The old mods of moves and paths, written before patches were mods of their own. */
const OLD_MODS = ["Mod Mallet - Moves", "Mod Mallet - Paths"];

/**
 * The user's patches (ADR 0070): on the left each patch with where it
 * stands, on the right the chosen one with what it does, the mods it is
 * about, where it disagrees with another, and every decision in words,
 * each leading to where it was decided. The page makes no decisions about
 * the game of its own; installing and switching on or off are actions on
 * the user's click.
 */
export function PatchView({ report }: { report: RenderedReport }) {
  const session = useSession();
  const { t, api, ignored, book, setBook, patchAt, lang } = session;
  const p = patchTexts[lang];
  const pairs = useMemo(() => pairsOf(report, ignored), [report, ignored]);
  const [selected, setSelected] = useState<string | null>(patchAt?.patch ?? null);
  useEffect(() => {
    if (patchAt !== null) {
      setSelected(patchAt.patch);
    }
  }, [patchAt]);
  const [note, setNote] = useState<string | null>(null);
  const [undo, setUndo] = useState<{ text: string; book: PatchBook } | null>(null);
  const loader = useRef<HTMLInputElement>(null);
  const conflicts = useMemo(() => (book === null ? [] : conflictsOf(book, pairs)), [book, pairs]);

  if (book === null) {
    return <EmptyState>{t.nothingHere}</EmptyState>;
  }
  const chosen =
    book.patches.find((patch) => patch.id === selected) ?? activePatch(book) ?? book.patches[0];

  const replace = (next: PatchBook, undoText?: string) => {
    if (undoText !== undefined) {
      setUndo({ text: undoText, book });
    }
    setBook(next);
  };
  const startPatch = () => {
    const { book: next, id } = withNewPatch(book, p.newPatchName);
    replace(next);
    setSelected(id);
  };
  const loadPatch = async (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    event.target.value = "";
    if (file === undefined) {
      return;
    }
    try {
      const bytes = Array.from(new Uint8Array(await file.arrayBuffer()));
      const loaded = await api.loadPatchArchive(file.name, bytes);
      const manifest = loaded.manifest ?? null;
      const { book: next, id } = withLoaded(book, {
        archive: loaded.archive,
        name: manifest?.name || loaded.patch.name || file.name.replace(/\.[^.]+$/, ""),
        author: manifest?.author ?? loaded.patch.author ?? null,
        version: manifest?.version ?? loaded.patch.version ?? null,
        decisions:
          manifest?.decisions ??
          decisionsOfObjects(loaded.patch.parts.flatMap((part) => part.objects)),
      });
      replace(next);
      setSelected(id);
      setNote(p.loaded(manifest?.name || loaded.patch.name || file.name));
    } catch (error: unknown) {
      setNote(p.loadFailed(String(error)));
    }
  };

  return (
    <section className="patches">
      <TopNotes report={report} book={book} pairs={pairs} p={p} onTake={replace} />
      <div className="patches-columns">
        <aside className="patch-shelf" aria-label={p.shelf}>
          <h3>{p.shelf}</h3>
          <ul>
            {book.patches.map((patch) => (
              <ShelfRow
                key={patch.id}
                patch={patch}
                report={report}
                active={activePatch(book)?.id === patch.id}
                chosen={chosen?.id === patch.id}
                conflicts={
                  conflicts.filter(
                    (conflict) => conflict.first.id === patch.id || conflict.second.id === patch.id,
                  ).length
                }
                p={p}
                onChoose={() => setSelected(patch.id)}
              />
            ))}
          </ul>
          <div className="patch-shelf-buttons">
            <button type="button" title={p.newPatchHint} onClick={startPatch}>
              + {p.newPatch}
            </button>
            <button type="button" title={p.loadHint} onClick={() => loader.current?.click()}>
              {p.load}
            </button>
            <input
              ref={loader}
              type="file"
              accept=".zip,.7z,.rar"
              hidden
              onChange={(event) => void loadPatch(event)}
            />
          </div>
          {note !== null && (
            <p className="aside" role="status">
              {note}
            </p>
          )}
        </aside>
        <div className="patch-detail">
          {undo !== null && (
            <Note
              tone="ok"
              role="status"
              actions={
                <UndoButton
                  onClick={() => {
                    setBook(undo.book);
                    setUndo(null);
                  }}
                />
              }
            >
              <p>{undo.text}</p>
            </Note>
          )}
          {chosen === undefined ? (
            <EmptyState>{p.emptyBook}</EmptyState>
          ) : (
            <PatchDetail
              key={chosen.id}
              report={report}
              book={book}
              patch={chosen}
              pairs={pairs}
              conflicts={conflicts.filter(
                (conflict) => conflict.first.id === chosen.id || conflict.second.id === chosen.id,
              )}
              entry={patchAt?.patch === chosen.id ? patchAt.entry : null}
              p={p}
              onChange={replace}
              onRemoved={() => setSelected(null)}
              onDuplicated={setSelected}
            />
          )}
        </div>
      </div>
      <PairReport report={report} game={session.game} />
    </section>
  );
}

/** One patch on the shelf: its name, whose it is, where it stands. */
function ShelfRow({
  patch,
  report,
  active,
  chosen,
  conflicts,
  p,
  onChoose,
}: {
  patch: BookPatch;
  report: RenderedReport;
  active: boolean;
  chosen: boolean;
  conflicts: number;
  p: PatchTexts;
  onChoose: () => void;
}) {
  const state = patch.off === true ? "off-here" : stateOf(report, patch);
  const count = Object.keys(patch.decisions).length;
  return (
    <li>
      <button
        type="button"
        className={chosen ? "patch-shelf-row chosen" : "patch-shelf-row"}
        aria-current={chosen}
        onClick={onChoose}
      >
        <span className="patch-shelf-name">
          {patch.name}
          {active && (
            <span className="patch-chip active" title={p.activeHint}>
              {p.active}
            </span>
          )}
        </span>
        <span className="patch-shelf-line">
          {isForeign(patch) && patch.author ? `${p.by(patch.author)} · ` : ""}
          {p.decisions(count)} · <span className={`patch-state ${state}`}>{p.state[state]}</span>
          {conflicts > 0 && <span className="patch-chip warn"> ⚠ {conflicts}</span>}
        </span>
      </button>
    </li>
  );
}

/** Notes about the whole setup: decisions that wait, patches the game has that are not listed, old mods. */
function TopNotes({
  report,
  book,
  pairs,
  p,
  onTake,
}: {
  report: RenderedReport;
  book: PatchBook;
  pairs: Pair[];
  p: PatchTexts;
  onTake: (book: PatchBook) => void;
}) {
  const session = useSession();
  const action = useAction();
  const paused = session.waiting ?? NOTHING_WAITS;
  const off = switchedOffTexts[session.lang];
  const known = new Set(book.patches.map((patch) => patch.name.toLowerCase()));
  const unknown = (report.installedPatch?.patches ?? []).filter(
    (installed) => !known.has(installed.patch.name.toLowerCase()),
  );
  const oldMods = report.mods.filter(
    (info) =>
      info.enabled &&
      OLD_MODS.some(
        (name) =>
          info.mod.name.toLowerCase().replace(/\.zip$/, "") === name.toLowerCase() ||
          info.mod.id.toLowerCase().replace(/\.zip$/, "") === name.toLowerCase(),
      ),
  );
  const open = openSpots(pairs, session.patchChoices);
  const spots = spotsByKey(report);
  const firstOpen = open.keys[0] === undefined ? undefined : spots.get(open.keys[0]);
  return (
    <>
      {!report.objectSwapper && (
        <Note
          tone="warning"
          actions={
            <button type="button" onClick={() => session.api.openPage(SWAPPER_PAGE)}>
              {session.t.openPage}
            </button>
          }
        >
          <p>{p.needsSwapper}</p>
        </Note>
      )}
      {(paused.decisions > 0 || paused.installed > 0 || paused.stillLoaded.length > 0) && (
        <Note tone="neutral">
          {paused.decisions > 0 && (
            <p>
              {off.waiting(paused.decisions, paused.mods)} <InfoTip text={off.waitingNote} />
            </p>
          )}
          {paused.installed > 0 && (
            <p>
              {off.installed(paused.installed, paused.installedPlugins)} {off.installedNote}
            </p>
          )}
          {paused.stillLoaded.length > 0 && <p>{off.stillLoaded(paused.stillLoaded)}</p>}
        </Note>
      )}
      {firstOpen !== undefined && (
        <Note
          actions={
            <button type="button" className="small" onClick={() => session.decideSpot(firstOpen)}>
              {p.toCollisions}
            </button>
          }
        >
          <p>{p.open(open.keys.length, open.pairs)}</p>
        </Note>
      )}
      {unknown.map((installed) => {
        const objects = installed.patch.parts.flatMap((part) => part.objects);
        return (
          <Note
            key={installed.patch.name}
            actions={
              <button
                type="button"
                className="small"
                onClick={() =>
                  onTake(
                    withLoaded(book, {
                      archive: installed.provider?.name ?? installed.patch.name,
                      name: installed.patch.name,
                      author: installed.patch.author ?? null,
                      version: installed.patch.version ?? null,
                      decisions: choicesOf(pairs, new Set(objects.map(matchKey)), objects),
                    }).book,
                  )
                }
              >
                {p.takeInstalled}
              </button>
            }
          >
            <p>{p.unknownInstalled(installed.patch.name, objects.length)}</p>
          </Note>
        );
      })}
      {oldMods.map((info) => (
        <Note
          tone="warning"
          key={info.mod.id}
          actions={
            <button
              type="button"
              className="small"
              disabled={action.busy}
              onClick={() => action.run({ type: "disable-mod", mod: info.mod })}
            >
              {p.switchOldOff}
            </button>
          }
        >
          <p>{p.oldMod(info.mod.name)}</p>
        </Note>
      ))}
      <ActionResult state={action} />
    </>
  );
}

interface PatchDetailProps {
  report: RenderedReport;
  book: PatchBook;
  patch: BookPatch;
  pairs: Pair[];
  conflicts: PatchConflict[];
  /** The entry to show and mark, when the user came back to it. */
  entry: string | null;
  p: PatchTexts;
  onChange: (book: PatchBook, undoText?: string) => void;
  onRemoved: () => void;
  onDuplicated: (id: string) => void;
}

/** One patch: what it does, where it stands, its mods, its disagreements and its decisions. */
function PatchDetail({
  report,
  book,
  patch,
  pairs,
  conflicts,
  entry,
  p,
  onChange,
  onRemoved,
  onDuplicated,
}: PatchDetailProps) {
  const session = useSession();
  const { t, api } = session;
  const action = useAction();
  const [asking, setAsking] = useState<"remove" | "forget" | null>(null);
  const [renaming, setRenaming] = useState<string | null>(null);
  const [written, setWritten] = useState<string | null>(null);
  const [writeError, setWriteError] = useState<string | null>(null);
  const modOf = useMemo(() => modNamer(report), [report]);
  const spots = useMemo(() => spotsByKey(report), [report]);
  const entries = useMemo(
    () => entriesOf(patch, { report, pairs, spots, p, modOf }),
    [patch, report, pairs, spots, p, modOf],
  );
  const mods = useMemo(() => modsOf(patch.decisions, report, modOf), [patch, report, modOf]);
  const toWrite = useMemo(
    () => writtenPatch(report, pairs, patch, (plugin) => modOf(plugin)),
    [report, pairs, patch, modOf],
  );
  const foreign = isForeign(patch);
  const active = activePatch(book)?.id === patch.id;
  const state = stateOf(report, patch);
  const mod = patchMod(report, patch.name);
  const ticked = useMemo(() => tickedOf(patch), [patch]);
  const hidden = objectsOf(pairs, ticked);
  const installed = new Set(installedObjectsOf(report, patch.name).map(matchKey));
  const missing =
    state === "in-game" ? hidden.filter((object) => !installed.has(matchKey(object))).length : 0;
  const moved = toWrite.moves?.moves.length ?? 0;
  const paths = toWrite.repairs?.places.filter((place) => place.every !== true).length ?? 0;
  const files = duelsOf(tickedOf(patch)).length;
  const count = Object.keys(patch.decisions).length;
  const doesSomething =
    toWrite.parts.some((part) => part.objects.length > 0) || moved > 0 || paths > 0;
  const does = p.does(hidden.length, moved, paths, files);

  const change = (next: PatchBook, undoText?: string) => onChange(next, undoText);
  const here = (key: string | null, text: string): PatchAt => ({
    patch: patch.id,
    entry: key,
    label: `${p.patchLabel(patch.name)}: ${text.length > 60 ? `${text.slice(0, 57)}…` : text}`,
  });
  const goTo = (target: Target, from: PatchAt) => {
    switch (target.kind) {
      case "spot":
        session.decideSpot(target.spot, from);
        break;
      case "map":
        session.showOnMap(target.spot, from);
        break;
      case "pair":
        session.openPair(target.mods[0], target.mods[1], from);
        break;
      case "duel":
        session.openCompare(target.id, from);
        break;
    }
  };
  const switchTo = (off: boolean) => {
    change(withPatch(book, patch.id, (each) => ({ ...each, off })));
    if (mod !== null && mod.enabled === off) {
      const ref = { id: mod.id, name: mod.name, source: mod.source };
      void action.run(off ? { type: "disable-mod", mod: ref } : { type: "enable-mod", mod: ref });
    }
  };
  const writeOnly = async () => {
    setWriteError(null);
    try {
      setWritten(await api.createPatch(toWrite));
    } catch (error: unknown) {
      setWriteError(String(error));
    }
  };
  const others = book.patches.filter((other) => other.id !== patch.id && !isForeign(other));

  return (
    <article className="patch-card">
      <header className="patch-card-head">
        {renaming === null ? (
          <h2>
            {patch.name}
            {active && <span className="patch-chip active">{p.active}</span>}
          </h2>
        ) : (
          <form
            className="patch-rename"
            onSubmit={(event) => {
              event.preventDefault();
              change(
                withPatch(book, patch.id, (each) => ({
                  ...each,
                  name: renaming.trim() || each.name,
                })),
              );
              setRenaming(null);
            }}
          >
            <input
              // biome-ignore lint/a11y/noAutofocus: the field is opened by the user's click
              autoFocus
              value={renaming}
              onChange={(event) => setRenaming(event.target.value)}
              aria-label={p.rename}
            />
            <button type="submit" className="small primary">
              {p.save}
            </button>
            <button type="button" className="small" onClick={() => setRenaming(null)}>
              {p.cancel}
            </button>
          </form>
        )}
        {active ? (
          <p className="patch-editing">✎ {p.editingBanner}</p>
        ) : (
          !foreign && (
            <p>
              <button
                type="button"
                className="patch-edit-button"
                title={p.editHint}
                onClick={() => change({ ...book, active: patch.id })}
              >
                ✎ {p.edit}
              </button>
            </p>
          )
        )}
        <p className="patch-does">{does === "" ? p.nothing : does}</p>
        {foreign && patch.author && (
          <p className="aside">
            {p.foreignNote(patch.author, patch.version ?? null)} <InfoTip text={p.authorUnproven} />
          </p>
        )}
        {patch.loadedFrom && <p className="aside">{p.loadedFrom(patch.loadedFrom)}</p>}
        <p className={`patch-state-line ${patch.off === true ? "off-here" : state}`}>
          {patch.off === true
            ? active
              ? p.stateOffEditing
              : p.stateOffHere
            : state === "in-game"
              ? p.stateInGame(mod?.name ?? patch.name)
              : state === "switched-off"
                ? p.stateSwitchedOff(mod?.name ?? patch.name)
                : p.stateNotInstalled}
        </p>
        {missing > 0 && patch.off !== true && (
          <Note tone="warning">
            <p>{p.differs(missing)}</p>
          </Note>
        )}
        <div className="patch-buttons">
          {patch.off !== true && (
            <button
              type="button"
              className="primary"
              disabled={!doesSomething || action.busy}
              title={p.installHint}
              onClick={() => void action.run({ type: "hide-objects", patch: toWrite })}
            >
              {state === "not-installed" ? p.install : p.update}
            </button>
          )}
          <button
            type="button"
            title={patch.off === true ? p.switchOnHint : p.switchOffHint}
            disabled={action.busy}
            onClick={() => switchTo(patch.off !== true)}
          >
            {patch.off === true ? p.switchOn : p.switchOff}
          </button>
          <button
            type="button"
            title={p.duplicateHint}
            onClick={() => {
              const copy = withDuplicate(book, patch.id, p.copyName(patch.name));
              change(copy.book);
              onDuplicated(copy.id);
            }}
          >
            {p.duplicate}
          </button>
          {foreign && (
            <button
              type="button"
              title={p.makeOwnHint}
              onClick={() =>
                change(
                  withPatch(book, patch.id, (each) => ({ ...each, foreign: false, author: null })),
                )
              }
            >
              {p.makeOwn}
            </button>
          )}
          <button
            type="button"
            title={p.writeHint}
            disabled={!doesSomething}
            onClick={() => void writeOnly()}
          >
            {p.write}
          </button>
          {!foreign && (
            <button type="button" onClick={() => setRenaming(patch.name)}>
              {p.rename}
            </button>
          )}
          {!foreign && (
            <button
              type="button"
              className="danger"
              title={p.forgetHint}
              disabled={count === 0}
              onClick={() => setAsking("forget")}
            >
              {p.forget}
            </button>
          )}
          <button type="button" className="danger" onClick={() => setAsking("remove")}>
            {p.remove}
          </button>
        </div>
        {asking !== null && (
          <Note tone="warning" role="alertdialog">
            <p>
              {asking === "forget"
                ? p.confirmForget(count)
                : p.confirmRemove(patch.name, count, state === "in-game")}
            </p>
            <span className="relief-object-buttons">
              <button
                type="button"
                className="danger primary"
                onClick={() => {
                  if (asking === "forget") {
                    change(
                      withPatch(book, patch.id, (each) => ({
                        ...each,
                        decisions: {},
                        unticked: [],
                      })),
                      p.forgotten,
                    );
                  } else {
                    change(withoutPatch(book, patch.id), p.removed(patch.name));
                    onRemoved();
                  }
                  setAsking(null);
                }}
              >
                {asking === "forget" ? p.forget : p.remove}
              </button>
              <button type="button" onClick={() => setAsking(null)}>
                {p.cancel}
              </button>
            </span>
          </Note>
        )}
        {written !== null && (
          <div className="patch-file">
            <span>
              {p.written} <code>{written}</code>
            </span>
            <button type="button" onClick={() => api.openFolder("patches")}>
              {t.openFolder}
            </button>
          </div>
        )}
        {writeError !== null && (
          <Note tone="danger" role="alert">
            <p>{writeError}</p>
          </Note>
        )}
        <ActionResult state={action} />
      </header>

      {mods.length > 0 && <ModList mods={mods} p={p} />}

      {conflicts.length > 0 && (
        <Conflicts
          book={book}
          patch={patch}
          conflicts={conflicts}
          spots={spots}
          p={p}
          onChange={change}
          onGo={(target, text) => goTo(target, here(null, text))}
        />
      )}

      <EntryList
        entries={entries}
        marked={entry}
        p={p}
        others={others}
        copyOnly={foreign}
        onTick={(keys, ticked) => change(withTicked(book, patch.id, keys, ticked))}
        onMove={(keys, to, copy) => change(withMoved(book, patch.id, to, keys, copy))}
        onGo={(target, item) => goTo(target, here(item.key, item.text))}
      />
      <AutoRepairs
        report={report}
        hidden={hidden}
        decisions={patch.decisions}
        editable={!foreign}
        p={p}
        onTurn={(key, off) =>
          change(
            withPatch(book, patch.id, (each) => {
              if (off) {
                each.decisions[key] = "off";
              } else {
                delete each.decisions[key];
              }
              return each;
            }),
          )
        }
      />
      <MovesPanel inPatch={{ choices: ticked, note: p.inPatchNote }} />
      <PathsPanel hidden={hidden} inPatch={{ choices: ticked, note: p.inPatchNote }} />
    </article>
  );
}

/** The mods a patch is about, those the game lacks or has switched off first. */
function ModList({ mods, p }: { mods: TouchedMod[]; p: PatchTexts }) {
  const [all, setAll] = useState(false);
  const shown = all ? mods : mods.slice(0, 12);
  return (
    <section className="patch-mods">
      <h3>
        {p.modsTitle} <InfoTip text={p.modsNote} />
      </h3>
      <ul>
        {shown.map((mod) => (
          <li key={mod.mod} className={`patch-mod ${mod.state}`}>
            {mod.state === "missing" ? (
              p.modMissing(mod.mod, mod.decisions)
            ) : mod.state === "switched-off" ? (
              p.modOff(mod.mod, mod.decisions)
            ) : (
              <>
                <strong>{mod.mod}</strong> · {p.modCount(mod.decisions)}
              </>
            )}
          </li>
        ))}
      </ul>
      {mods.length > shown.length && (
        <button type="button" className="small" onClick={() => setAll(true)}>
          + {mods.length - shown.length}
        </button>
      )}
    </section>
  );
}

/** Where this patch disagrees with others, with a way to keep one side. */
function Conflicts({
  book,
  patch,
  conflicts,
  spots,
  p,
  onChange,
  onGo,
}: {
  book: PatchBook;
  patch: BookPatch;
  conflicts: PatchConflict[];
  spots: Map<string, MapSpot>;
  p: PatchTexts;
  onChange: (book: PatchBook) => void;
  onGo: (target: Target, text: string) => void;
}) {
  /** The book with `loser` agreeing with `winner` at every place of a conflict. */
  const keep = (conflict: PatchConflict, winner: BookPatch) => {
    const loser = winner.id === conflict.first.id ? conflict.second : conflict.first;
    return withPatch(book, loser.id, (each) => {
      for (const place of conflict.places) {
        each.decisions[place.key] = winner.id === conflict.first.id ? place.first : place.second;
      }
      return each;
    });
  };
  return (
    <section className="patch-conflicts">
      <h3>
        {p.conflictsTitle} <InfoTip text={p.conflictNote} />
      </h3>
      {conflicts.map((conflict) => {
        const other = conflict.first.id === patch.id ? conflict.second : conflict.first;
        return (
          <div className="patch-conflict" key={other.id}>
            <p>{p.conflict(conflict.first.name, conflict.second.name, conflict.places.length)}</p>
            <ul className="patch-conflict-places">
              {conflict.places.slice(0, 8).map((place) => {
                const spot = spots.get(place.key);
                const text = spot === undefined ? place.key : spot.objects;
                return (
                  <li key={place.key}>
                    {spot === undefined ? (
                      <span>{text}</span>
                    ) : (
                      <button
                        type="button"
                        className="link"
                        onClick={() => onGo({ kind: "spot", spot }, text)}
                      >
                        {text}
                      </button>
                    )}
                  </li>
                );
              })}
            </ul>
            <span className="relief-object-buttons">
              {[conflict.first, conflict.second].map((winner) => {
                const loser = winner.id === conflict.first.id ? conflict.second : conflict.first;
                return isForeign(loser) ? null : (
                  <button
                    type="button"
                    className="small"
                    key={winner.id}
                    title={p.keepHint(loser.name)}
                    onClick={() => onChange(keep(conflict, winner))}
                  >
                    {p.keep(winner.name)}
                  </button>
                );
              })}
            </span>
            {(isForeign(conflict.first) || isForeign(conflict.second)) && (
              <p className="aside">
                {p.foreignConflict(
                  isForeign(conflict.first) ? conflict.first.name : conflict.second.name,
                )}
              </p>
            )}
          </div>
        );
      })}
    </section>
  );
}

/** A patch's decisions, grouped, each with a tick, a way to it and a way to another patch. */
function EntryList({
  entries,
  marked,
  p,
  others,
  copyOnly,
  onTick,
  onMove,
  onGo,
}: {
  entries: Entry[];
  marked: string | null;
  p: PatchTexts;
  others: BookPatch[];
  copyOnly: boolean;
  onTick: (keys: string[], ticked: boolean) => void;
  onMove: (keys: string[], to: string, copy: boolean) => void;
  onGo: (target: Target, entry: Entry) => void;
}) {
  const markedRef = useRef<HTMLLIElement>(null);
  useEffect(() => {
    markedRef.current?.scrollIntoView({ block: "center" });
  }, []);
  if (entries.length === 0) {
    return null;
  }
  return (
    <>
      {GROUPS.map((group) => {
        const inGroup = entries.filter((each) => each.group === group);
        if (inGroup.length === 0) {
          return null;
        }
        const sections = [...new Set(inGroup.map((each) => each.section))];
        return (
          <section className="patch-group" key={group}>
            <h3>
              {p.groups[group]} <span className="aside">({inGroup.length})</span>
            </h3>
            {sections.map((section) => {
              const items = inGroup.filter((each) => each.section === section);
              return (
                <div className="patch-section" key={section ?? ""}>
                  {section !== null && (
                    <h4>
                      <span>{section}</span>
                      <MoveMenu
                        label={p.moveAll}
                        p={p}
                        others={others}
                        copyOnly={copyOnly}
                        onMove={(to, copy) =>
                          onMove(
                            items.map((each) => each.key),
                            to,
                            copy,
                          )
                        }
                      />
                    </h4>
                  )}
                  <ul className="patch-entries">
                    {items.map((item) => (
                      <li
                        key={item.key}
                        ref={item.key === marked ? markedRef : undefined}
                        className={[
                          "patch-entry",
                          item.ticked ? "" : "unticked",
                          item.key === marked ? "marked" : "",
                        ].join(" ")}
                      >
                        <input
                          type="checkbox"
                          checked={item.ticked}
                          title={p.tickHint}
                          aria-label={item.text}
                          onChange={(event) => onTick([item.key], event.target.checked)}
                        />
                        {item.target === null ? (
                          <span className="patch-entry-text">{item.text}</span>
                        ) : (
                          <button
                            type="button"
                            className="patch-entry-text link"
                            title={p.goHint}
                            onClick={() => item.target !== null && onGo(item.target, item)}
                          >
                            {item.text}
                          </button>
                        )}
                        {item.where !== null && (
                          <span className="patch-entry-where">{item.where}</span>
                        )}
                        <MoveMenu
                          label={p.moveMenu}
                          p={p}
                          others={others}
                          copyOnly={copyOnly}
                          onMove={(to, copy) => onMove([item.key], to, copy)}
                        />
                      </li>
                    ))}
                  </ul>
                </div>
              );
            })}
          </section>
        );
      })}
    </>
  );
}

/** A menu that moves or copies decisions to another patch of the user's own. */
function MoveMenu({
  label,
  p,
  others,
  copyOnly = false,
  onMove,
}: {
  label: string;
  p: PatchTexts;
  others: BookPatch[];
  copyOnly?: boolean;
  onMove: (to: string, copy: boolean) => void;
}) {
  if (others.length === 0) {
    return null;
  }
  return (
    <select
      className="patch-move"
      value=""
      aria-label={label}
      onChange={(event) => {
        const [how, to] = event.target.value.split(":", 2);
        if (to !== undefined) {
          onMove(to, how === "copy");
        }
      }}
    >
      <option value="">{label}</option>
      {!copyOnly &&
        others.map((other) => (
          <option key={`move:${other.id}`} value={`move:${other.id}`}>
            {p.moveTo(other.name)}
          </option>
        ))}
      {others.map((other) => (
        <option key={`copy:${other.id}`} value={`copy:${other.id}`}>
          {p.copyTo(other.name)}
        </option>
      ))}
    </select>
  );
}

/**
 * The paths a patch repairs by itself (ADR 0070): where it hides something
 * characters walk on, the paths there are repaired when the patch is
 * written, place by place, unless turned off for a place.
 */
function AutoRepairs({
  report,
  hidden,
  decisions,
  editable,
  p,
  onTurn,
}: {
  report: RenderedReport;
  hidden: ObjectRef[];
  decisions: PatchChoices;
  editable: boolean;
  p: PatchTexts;
  onTurn: (key: string, off: boolean) => void;
}) {
  const [all, setAll] = useState(false);
  const areas = useMemo(() => repairAreasOf(report, hidden), [report, hidden]);
  if (areas.length === 0) {
    return null;
  }
  const off = areas.filter((area) => noRepairKey(area) in decisions).length;
  const shown = all ? areas : areas.slice(0, 8);
  return (
    <section className="patch-group">
      <h3>
        {p.autoRepairTitle} <InfoTip text={p.autoRepairNote} />
      </h3>
      <p className="aside">{p.autoRepairSummary(areas.length, off)}</p>
      <ul className="patch-entries">
        {shown.map((area) => {
          const key = noRepairKey(area);
          const repaired = !(key in decisions);
          return (
            <li key={area.key} className={repaired ? "patch-entry" : "patch-entry unticked"}>
              <input
                type="checkbox"
                checked={repaired}
                disabled={!editable}
                aria-label={area.label}
                title={p.autoRepairTick}
                onChange={(event) => onTurn(key, !event.target.checked)}
              />
              <span className="patch-entry-text">{area.label}</span>
              <span className="patch-entry-where">{p.objectsHere(area.objects)}</span>
              <span />
            </li>
          );
        })}
      </ul>
      {areas.length > shown.length && (
        <button type="button" className="small" onClick={() => setAll(true)}>
          + {areas.length - shown.length}
        </button>
      )}
    </section>
  );
}
