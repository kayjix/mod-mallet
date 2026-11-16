import { useEffect, useMemo, useRef, useState } from "react";
import { useSession } from "./Actions";
import {
  duelNames,
  OTHERS,
  rankedSide,
  rankKey,
  resolveDuel,
  sideName,
  useFindings,
  usePick,
} from "./Duels";
import type { FilePreview, PreviewVersion } from "./generated/file-preview";
import type {
  DuelFile,
  RenderedDuel,
  RenderedFinding,
  RenderedReport,
} from "./generated/rendered-report";
import type { Texts } from "./i18n";
import { Stage, type StageItem } from "./Viewer3D";

type Topic = DuelFile["topic"];

/** Previews already asked for, by file; one scan's worth. */
type PreviewCache = Map<string, Promise<FilePreview | null>>;

/**
 * Two mods next to each other, three columns: the duels on the left, the
 * thing they disagree about in the middle, seen through one camera, and
 * every file of the duel on the right, by the name of the object.
 */
export function CompareView({
  report,
  start,
  onBack,
}: {
  report: RenderedReport;
  start: string | null;
  onBack: () => void;
}) {
  const { t, decisions } = useSession();
  const byId = useFindings(report.findings);
  const [duelId, setDuelId] = useState<string | null>(
    start ??
      report.duels.find((rendered) => decisions[rendered.duel.id] === undefined)?.duel.id ??
      report.duels[0]?.duel.id ??
      null,
  );
  useEffect(() => {
    if (start !== null) {
      setDuelId(start);
    }
  }, [start]);
  const [tab, setTab] = useState<"duels" | "ranking">("duels");
  const cache = useRef<{ at: number; previews: PreviewCache }>({
    at: report.scannedAt,
    previews: new Map(),
  });
  if (cache.current.at !== report.scannedAt) {
    cache.current = { at: report.scannedAt, previews: new Map() };
  }
  const rendered = report.duels.find((candidate) => candidate.duel.id === duelId) ?? null;

  return (
    <div className="compare">
      <aside className="compare-left">
        <div className="compare-left-head">
          <button type="button" className="quiet small" onClick={onBack}>
            ← {t.compareBack}
          </button>
          <fieldset className="segmented" aria-label={t.viewCompare}>
            <button type="button" aria-pressed={tab === "duels"} onClick={() => setTab("duels")}>
              {t.compareDuels}
            </button>
            <button
              type="button"
              aria-pressed={tab === "ranking"}
              onClick={() => setTab("ranking")}
            >
              {t.compareRanking}
            </button>
          </fieldset>
        </div>
        {tab === "duels" ? (
          <DuelPicker duels={report.duels} selected={duelId} onSelect={setDuelId} />
        ) : (
          <RankingPanel duels={report.duels} byId={byId} />
        )}
      </aside>
      {rendered === null ? (
        <p className="aside compare-empty">{t.compareNone}</p>
      ) : (
        <DuelWorkspace
          key={rendered.duel.id}
          rendered={rendered}
          byId={byId}
          previews={cache.current.previews}
        />
      )}
    </div>
  );
}

// ---- left: the duels ----------------------------------------------------------

const TOPICS: Topic[] = ["faces", "textures", "meshes", "objects", "contained"];

function DuelPicker({
  duels,
  selected,
  onSelect,
}: {
  duels: RenderedDuel[];
  selected: string | null;
  onSelect: (id: string) => void;
}) {
  const { t, decisions, mods, ranking } = useSession();
  const [topic, setTopic] = useState<Topic | "all">("all");
  const [search, setSearch] = useState("");
  const [showDecided, setShowDecided] = useState(false);
  const decidedCount = duels.filter((rendered) => decisions[rendered.duel.id] !== undefined).length;
  const wanted = search.trim().toLowerCase();
  const shown = duels.filter((rendered) => {
    const { duel } = rendered;
    if (!showDecided && decisions[duel.id] !== undefined && duel.id !== selected) {
      return false;
    }
    if (topic !== "all" && !duel.topics.some((candidate) => candidate.topic === topic)) {
      return false;
    }
    if (wanted === "") {
      return true;
    }
    const [a, b] = duelNames(duel, mods);
    return (
      a.toLowerCase().includes(wanted) ||
      b.toLowerCase().includes(wanted) ||
      (duel.files ?? []).some((file) => file.object?.toLowerCase().includes(wanted))
    );
  });

  return (
    <>
      <p className="compare-summary">
        <strong>{duels.length - decidedCount}</strong> {t.compareOpenLabel} ·{" "}
        <strong>{decidedCount}</strong> {t.compareDecidedLabel}
      </p>
      <div className="compare-filters">
        <input
          type="search"
          placeholder={t.compareSearch}
          value={search}
          onChange={(event) => setSearch(event.target.value)}
        />
        <select value={topic} onChange={(event) => setTopic(event.target.value as Topic | "all")}>
          <option value="all">{t.compareAllTopics}</option>
          {TOPICS.map((candidate) => (
            <option key={candidate} value={candidate}>
              {t.topicName[candidate]}
            </option>
          ))}
        </select>
        <label className="compare-check">
          <input
            type="checkbox"
            checked={showDecided}
            onChange={(event) => setShowDecided(event.target.checked)}
          />
          {t.compareShowDecided}
        </label>
      </div>
      <ul className="compare-duels">
        {shown.map((rendered) => {
          const { duel } = rendered;
          const names = duelNames(duel, mods);
          const decided = decisions[duel.id];
          const ranked = rankedSide(duel, ranking);
          return (
            <li key={duel.id}>
              <button
                type="button"
                className="compare-duel"
                aria-current={duel.id === selected ? "true" : undefined}
                data-decided={decided !== undefined}
                onClick={() => onSelect(duel.id)}
              >
                <span className="compare-duel-names">
                  {([0, 1] as const).map((index) => (
                    <span
                      key={index}
                      data-winner={decided === index}
                      data-ranked={decided === undefined && ranked === index}
                    >
                      {names[index]}
                    </span>
                  ))}
                </span>
                <span className="compare-duel-topics">
                  {rendered.topicLabels.join(" · ")}
                  {(duel.files?.length ?? 0) > 0 && ` · ${t.compareFiles(duel.files?.length ?? 0)}`}
                </span>
              </button>
            </li>
          );
        })}
        {shown.length === 0 && <li className="aside">{t.nothingHere}</li>}
      </ul>
    </>
  );
}

// ---- middle and right: one duel -----------------------------------------------

/** The files of a duel; for older reports, the files its findings name. */
function filesOf(rendered: RenderedDuel, byId: Map<string, RenderedFinding>): DuelFile[] {
  const own = rendered.duel.files ?? [];
  if (own.length > 0) {
    return own;
  }
  const topic = rendered.duel.topics[0]?.topic ?? "textures";
  const paths = new Set(
    rendered.duel.findings.flatMap((id) =>
      (byId.get(id)?.finding.previews ?? []).map((path) => path.replace(/^data\//, "")),
    ),
  );
  return [...paths].map((path) => ({ path, topic, object: null, objectKind: null }));
}

function isMesh(path: string): boolean {
  return path.toLowerCase().endsWith(".nif");
}

/**
 * Whether a texture is one of the technical maps beside a colour map:
 * normals, gloss, glow, environment, skin, parallax. Comparing them by eye
 * says little.
 */
function isTechnical(path: string): boolean {
  return /_(n|msn|s|g|e|em|m|p|sk|b|bl|rm).dds$/i.test(path);
}

function isTexture(path: string): boolean {
  return path.toLowerCase().endsWith(".dds");
}

function preview(
  cache: PreviewCache,
  api: { filePreview: (path: string) => Promise<FilePreview | null> },
  path: string,
) {
  let found = cache.get(path);
  if (found === undefined) {
    found = api.filePreview(path).catch(() => null);
    cache.set(path, found);
  }
  return found;
}

function DuelWorkspace({
  rendered,
  byId,
  previews,
}: {
  rendered: RenderedDuel;
  byId: Map<string, RenderedFinding>;
  previews: PreviewCache;
}) {
  const { t, api, mods } = useSession();
  const { duel } = rendered;
  const names = duelNames(duel, mods);
  const every = useMemo(() => filesOf(rendered, byId), [rendered, byId]);
  const [technical, setTechnical] = useState(false);
  const technicalCount = every.filter((file) => isTechnical(file.path)).length;
  const files = technical ? every : every.filter((file) => !isTechnical(file.path));
  // Meshes first: they show the most.
  const [path, setPath] = useState<string | null>(
    files.find((file) => isMesh(file.path))?.path ?? files[0]?.path ?? every[0]?.path ?? null,
  );
  const [found, setFound] = useState<FilePreview | null | undefined>(undefined);
  useEffect(() => {
    if (path === null) {
      return;
    }
    let outdated = false;
    setFound(undefined);
    preview(previews, api, path).then((result) => {
      if (!outdated) {
        setFound(result);
      }
    });
    return () => {
      outdated = true;
    };
  }, [api, path, previews]);

  const [nameA, nameB] = names;
  const original = t.duelOriginal;
  const items = useMemo<StageItem[]>(() => {
    if (path === null || found == null) {
      return [];
    }
    const sideOf = (version: PreviewVersion): 0 | 1 | null => {
      const id = version.provider?.id;
      if (id === undefined) {
        return null;
      }
      if (duel.sides[0].mod?.id === id) {
        return 0;
      }
      return duel.sides[1].mod?.id === id ? 1 : null;
    };
    const label = (side: 0 | 1, own: boolean) => {
      const name = side === 0 ? nameA : nameB;
      return own ? name : `${name} – ${original}`;
    };
    // Each side's own version, or what it leaves in place: the original.
    const indexFor = (side: 0 | 1) => {
      const own = found.versions.findIndex((version) => sideOf(version) === side);
      return own >= 0 ? own : found.versions.findIndex((version) => sideOf(version) === null);
    };
    return ([0, 1] as const).flatMap((side): StageItem[] => {
      const index = indexFor(side);
      if (index < 0) {
        return [];
      }
      const own = sideOf(found.versions[index] ?? {}) === side;
      if (isMesh(path)) {
        return [{ kind: "mesh", label: label(side, own), path, version: index }];
      }
      if (isTexture(path)) {
        return [
          {
            kind: "texture",
            label: label(side, own),
            path,
            image: found.versions[index]?.image ?? null,
          },
        ];
      }
      return [];
    });
  }, [path, found, duel, nameA, nameB, original]);
  const selected = every.find((file) => file.path === path);

  return (
    <>
      <section className="compare-middle">
        <header className="compare-stage-head">
          <span className="compare-side-name" title={names[0]}>
            {names[0]}
          </span>
          <span className="compare-versus" aria-hidden="true">
            ⚔
          </span>
          <span className="compare-side-name" title={names[1]}>
            {names[1]}
          </span>
        </header>
        {selected !== undefined && (
          <p className="compare-object">
            <strong>{selected.object ?? fileName(selected.path)}</strong>
            {selected.objectKind != null && (
              <span className="tag">{kindName(t, selected.objectKind)}</span>
            )}
            <span className="tag">{t.topicName[selected.topic]}</span>
          </p>
        )}
        {path === null ? (
          <p className="aside compare-stage-empty">{t.duelNoPictures}</p>
        ) : found === undefined ? (
          <p className="aside compare-stage-empty">{t.previewLoading}</p>
        ) : items.length === 0 ? (
          <p className="aside compare-stage-empty">{t.compareCannotShow}</p>
        ) : (
          <Stage items={items} />
        )}
        {path !== null && isTexture(path) && items.length > 0 && (
          <div className="compare-flat">
            {items.map((item) =>
              item.kind === "texture" && item.image !== null ? (
                <figure key={item.label}>
                  <img className="compare-flat-image" src={item.image} alt={item.label} />
                  <figcaption>{item.label}</figcaption>
                </figure>
              ) : null,
            )}
          </div>
        )}
        <DuelDecision rendered={rendered} byId={byId} />
      </section>
      <aside className="compare-right">
        <h3>
          {t.compareObjects} <span className="count">{files.length}</span>
        </h3>
        {files.length === 0 && <p className="aside">{t.duelNoPictures}</p>}
        {technicalCount > 0 && (
          <label className="compare-check">
            <input
              type="checkbox"
              checked={technical}
              onChange={(event) => setTechnical(event.target.checked)}
            />
            {t.compareTechnical(technicalCount)}
          </label>
        )}
        <ul className="compare-files">
          {files.map((file) => (
            <FileRow
              key={file.path}
              file={file}
              selected={file.path === path}
              previews={previews}
              onSelect={() => setPath(file.path)}
            />
          ))}
        </ul>
      </aside>
    </>
  );
}

function fileName(path: string): string {
  return path.split("/").pop() ?? path;
}

function kindName(t: Texts, kind: string): string {
  return t.objectKinds[kind] ?? kind;
}

/** One file of the duel: a small picture once it scrolls into view, the object's name, its kind. */
function FileRow({
  file,
  selected,
  previews,
  onSelect,
}: {
  file: DuelFile;
  selected: boolean;
  previews: PreviewCache;
  onSelect: () => void;
}) {
  const { t, api } = useSession();
  const row = useRef<HTMLLIElement>(null);
  const [image, setImage] = useState<string | null>(null);
  useEffect(() => {
    const element = row.current;
    if (element === null || !isTexture(file.path)) {
      return;
    }
    let outdated = false;
    const watcher = new IntersectionObserver((entries) => {
      if (entries.some((entry) => entry.isIntersecting)) {
        watcher.disconnect();
        preview(previews, api, file.path).then((found) => {
          const last = found?.versions[found.versions.length - 1];
          if (!outdated) {
            setImage(last?.image ?? null);
          }
        });
      }
    });
    watcher.observe(element);
    return () => {
      outdated = true;
      watcher.disconnect();
    };
  }, [api, file.path, previews]);

  return (
    <li ref={row}>
      <button
        type="button"
        className="compare-file"
        aria-current={selected ? "true" : undefined}
        onClick={onSelect}
      >
        <span className="compare-thumb" data-kind={isMesh(file.path) ? "mesh" : "texture"}>
          {image !== null ? (
            <img className="compare-thumb-image" src={image} alt="" />
          ) : isMesh(file.path) ? (
            "3D"
          ) : (
            "2D"
          )}
        </span>
        <span className="compare-file-text">
          <strong className="compare-file-name">{file.object ?? fileName(file.path)}</strong>
          <span className="aside">
            {[
              file.objectKind == null ? null : kindName(t, file.objectKind),
              t.topicName[file.topic],
            ]
              .filter((part) => part !== null)
              .join(" · ")}
          </span>
          <code>{file.path}</code>
        </span>
      </button>
    </li>
  );
}

/** The decision under the stage: who wins, and why Mod Mallet or the ranking would pick a side. */
function DuelDecision({
  rendered,
  byId,
}: {
  rendered: RenderedDuel;
  byId: Map<string, RenderedFinding>;
}) {
  const { t, decisions, decide, showPatch, mods, ranking } = useSession();
  const { duel } = rendered;
  const { busy, applied, pick } = usePick(duel, byId);
  const names = duelNames(duel, mods);
  const decided = decisions[duel.id];
  const suggested = duel.suggestion?.side;
  const ranked = rankedSide(duel, ranking);

  return (
    <div className="compare-decision">
      <div className="compare-picks">
        {([0, 1] as const).map((index) => (
          <button
            type="button"
            key={index}
            className={
              decided === index || (decided === undefined && suggested === index)
                ? "primary"
                : undefined
            }
            disabled={busy}
            onClick={() => pick(index)}
          >
            {decided === index ? `✓ ${t.duelPick(names[index])}` : t.duelPick(names[index])}
          </button>
        ))}
      </div>
      <p className="aside compare-why">
        {ranked !== null && <span className="ranked-mark">{t.rankedFor(names[ranked])}</span>}{" "}
        {rendered.suggestion}
      </p>
      <p className="aside">{rendered.topicLabels.join(" · ")}</p>
      {decided !== undefined && (
        <button type="button" className="quiet small" onClick={() => decide(duel.id, null)}>
          {t.duelUndo}
        </button>
      )}
      {applied !== null && (
        <div className="duel-applied" role="status">
          {applied.messages.map((message) => (
            <p key={message}>{message}</p>
          ))}
          {applied.hidden > 0 && (
            <button type="button" className="small quiet" onClick={showPatch}>
              {t.choiceToPatch}
            </button>
          )}
        </div>
      )}
    </div>
  );
}

// ---- the ranking ----------------------------------------------------------------

/**
 * The mods in the order the user likes them: higher wins. A marker stands
 * for every mod not placed, so that one mod can be put below everything
 * else without placing all others. Decides the open duels in one go.
 */
function RankingPanel({
  duels,
  byId,
}: {
  duels: RenderedDuel[];
  byId: Map<string, RenderedFinding>;
}) {
  const session = useSession();
  const { t, mods, ranking, setRanking, decisions, decide, setPatchChoices } = session;
  const [search, setSearch] = useState("");
  const [confirming, setConfirming] = useState(false);
  const [progress, setProgress] = useState<{ done: number; total: number } | null>(null);
  const [result, setResult] = useState<string | null>(null);

  // Every mod in a duel, with how many duels it is in.
  const everyMod = useMemo(() => {
    const found = new Map<string, { name: string; duels: number }>();
    for (const { duel } of duels) {
      for (const side of duel.sides) {
        const key = rankKey(side);
        const known = found.get(key);
        found.set(key, { name: sideName(side, mods), duels: (known?.duels ?? 0) + 1 });
      }
    }
    return found;
  }, [duels, mods]);
  const list = ranking.includes(OTHERS) ? ranking : [...ranking, OTHERS];
  const placed = new Set(list);
  const wanted = search.trim().toLowerCase();
  const unplaced = [...everyMod.entries()]
    .filter(([key, mod]) => !placed.has(key) && mod.name.toLowerCase().includes(wanted))
    .sort((a, b) => b[1].duels - a[1].duels || a[1].name.localeCompare(b[1].name));

  const move = (index: number, by: -1 | 1) => {
    const next = [...list];
    const other = index + by;
    if (other < 0 || other >= next.length) {
      return;
    }
    const [a, b] = [next[index], next[other]];
    if (a === undefined || b === undefined) {
      return;
    }
    next[index] = b;
    next[other] = a;
    setRanking(next);
  };
  const remove = (key: string) => setRanking(list.filter((candidate) => candidate !== key));
  const prefer = (key: string) => {
    const at = list.indexOf(OTHERS);
    setRanking([...list.slice(0, at), key, ...list.slice(at)]);
  };
  const defer = (key: string) => setRanking([...list, key]);

  const deciding = duels.filter(
    ({ duel }) => decisions[duel.id] === undefined && rankedSide(duel, list) !== null,
  );
  const apply = async () => {
    setConfirming(false);
    setResult(null);
    let choices = session.patchChoices;
    let hidden = 0;
    let failed = 0;
    let decidedNow = 0;
    let stopped: string | null = null;
    setProgress({ done: 0, total: deciding.length });
    for (const [index, { duel }] of deciding.entries()) {
      const side = rankedSide(duel, list);
      if (side === null) {
        continue;
      }
      const applied = await resolveDuel(session, duel, byId, side, choices);
      failed += applied.failed;
      if (applied.unreachable) {
        // Without the mod manager nothing more can happen; what was
        // decided so far stays decided.
        stopped = applied.messages[applied.messages.length - 1] ?? null;
        break;
      }
      choices = applied.choices;
      hidden += applied.hidden;
      decide(duel.id, side);
      decidedNow += 1;
      setProgress({ done: index + 1, total: deciding.length });
    }
    if (hidden > 0) {
      setPatchChoices(choices);
    }
    setProgress(null);
    setResult(
      stopped === null
        ? t.rankingDone(decidedNow, hidden, failed)
        : `${t.rankingDone(decidedNow, hidden, failed)} ${stopped}`,
    );
  };

  return (
    <div className="ranking">
      <p className="aside">{t.rankingHint}</p>
      <ol className="ranking-list">
        {list.map((key, index) =>
          key === OTHERS ? (
            <li key={key} className="ranking-row ranking-others">
              <span>{t.rankingOthers(unplaced.length)}</span>
              <span className="ranking-buttons">
                <button
                  type="button"
                  className="small quiet"
                  aria-label={t.rankingUp}
                  onClick={() => move(index, -1)}
                >
                  ↑
                </button>
                <button
                  type="button"
                  className="small quiet"
                  aria-label={t.rankingDown}
                  onClick={() => move(index, 1)}
                >
                  ↓
                </button>
              </span>
            </li>
          ) : (
            <li key={key} className="ranking-row">
              <span className="ranking-name">{everyMod.get(key)?.name ?? key}</span>
              <span className="ranking-buttons">
                <button
                  type="button"
                  className="small quiet"
                  aria-label={t.rankingUp}
                  onClick={() => move(index, -1)}
                >
                  ↑
                </button>
                <button
                  type="button"
                  className="small quiet"
                  aria-label={t.rankingDown}
                  onClick={() => move(index, 1)}
                >
                  ↓
                </button>
                <button
                  type="button"
                  className="small quiet"
                  aria-label={t.rankingRemove}
                  onClick={() => remove(key)}
                >
                  ✕
                </button>
              </span>
            </li>
          ),
        )}
      </ol>
      <div className="ranking-apply">
        {progress !== null ? (
          <p className="ranking-apply-text" role="status">
            {t.rankingRunning(progress.done, progress.total)}
          </p>
        ) : confirming ? (
          <>
            <p className="ranking-apply-text">{t.rankingConfirm(deciding.length)}</p>
            <button type="button" className="primary" onClick={apply}>
              {t.rankingApply(deciding.length)}
            </button>
            <button type="button" className="quiet" onClick={() => setConfirming(false)}>
              {t.cancel}
            </button>
          </>
        ) : (
          <button
            type="button"
            className="primary"
            disabled={deciding.length === 0}
            onClick={() => setConfirming(true)}
          >
            {deciding.length === 0 ? t.rankingNothing : t.rankingApply(deciding.length)}
          </button>
        )}
        {result !== null && (
          <p className="ranking-apply-text" role="status">
            {result}
          </p>
        )}
      </div>
      <h4>{t.rankingUnplaced}</h4>
      <input
        type="search"
        placeholder={t.compareSearch}
        value={search}
        onChange={(event) => setSearch(event.target.value)}
      />
      <ul className="ranking-unplaced">
        {unplaced.map(([key, mod]) => (
          <li key={key} className="ranking-row">
            <span className="ranking-name">
              {mod.name} <span className="aside">· {t.rankingDuels(mod.duels)}</span>
            </span>
            <span className="ranking-buttons">
              <button type="button" className="small quiet" onClick={() => prefer(key)}>
                {t.rankingPrefer}
              </button>
              <button type="button" className="small quiet" onClick={() => defer(key)}>
                {t.rankingDefer}
              </button>
            </span>
          </li>
        ))}
      </ul>
    </div>
  );
}
