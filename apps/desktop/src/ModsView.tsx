import { useEffect, useId, useMemo, useState } from "react";
import { useSession } from "./Actions";
import { stepOf } from "./arrange";
import { DependencyView } from "./DependencyView";
import { duelNames } from "./Duels";
import { buildGraph, type DependencyTarget } from "./dependencies";
import type {
  LoadedPlugin,
  ModInfo,
  RenderedDuel,
  RenderedFinding,
} from "./generated/rendered-report";
import { type CollisionSpot, collisionSpots, weightOf } from "./health";
import { DependencyContext, ModDependencies, ModsTabs } from "./ModDependencies";
import type { WorkKind } from "./map-places";
import { type ModFilter, type ModSort, useOverviewTexts } from "./overview-texts";
import { FindingRow } from "./Tasks";
import { COLLIDES_WIDELY, toneOf, WideAdvice, WideTag } from "./wide-advice";
import "./overview.css";

/** The kinds of collision, the worst first. */
const WORK_KINDS: WorkKind[] = ["duplicate", "overlap", "deep", "touch"];

/** The steps whose findings are problems of a mod rather than notes or suggestions. */
const PROBLEM_STEPS = new Set(["one-click", "install", "update", "choose", "scan", "check"]);

/** Findings that say what of a mod is covered by another, or covers another. */
const isOverride = (rendered: RenderedFinding) =>
  rendered.finding.kind.startsWith("assets.") ||
  rendered.finding.kind === "navmeshes.changes-undone";

/** The collisions of a mod with one other mod. */
interface Collided {
  other: string;
  spots: CollisionSpot[];
  open: number;
  kinds: Record<WorkKind, number>;
}

/** What is open about one mod. */
interface ModState {
  info: ModInfo;
  problems: RenderedFinding[];
  advice: RenderedFinding[];
  overridden: RenderedFinding[];
  duels: RenderedDuel[];
  collisions: Collided[];
  openSpots: number;
  crash: boolean;
}

/** Arguments in which findings name mods (or plugins, where no mod is known). */
const NAME_ARGS = [
  "first",
  "second",
  "plugin",
  "container",
  "mod",
  "other",
  "winner",
  "loser",
  "winnerMod",
  "archiveMod",
  "faceMod",
  "meshMod",
  "textureMod",
];

/** The mods a finding is about, by their identity in the mod manager. */
function modIdsOf(
  rendered: RenderedFinding,
  byName: Map<string, ModInfo>,
  byId: Map<string, ModInfo>,
): Set<string> {
  const ids = new Set<string>();
  const { finding } = rendered;
  const add = (id: string | undefined) => {
    if (id !== undefined && byId.has(id)) {
      ids.add(id);
    }
  };
  add(finding.provider?.id);
  const { subject } = finding;
  const parties =
    subject.type === "mod"
      ? [subject]
      : subject.type === "override"
        ? [subject.winner, subject.loser]
        : subject.type === "pair"
          ? [subject.first, subject.second]
          : [];
  for (const party of parties) {
    if (party.type === "mod") {
      add(party.id);
    }
  }
  for (const key of NAME_ARGS) {
    const value = finding.args[key];
    if (typeof value === "string") {
      add(byName.get(value.toLowerCase())?.mod.id);
    }
  }
  return ids;
}

/**
 * The mods the mod manager knows, like a mod manager lists them, each
 * with what is open about it: its findings, the mods it collides with
 * and what of it others cover. A mod opens to show all of that.
 */
export function ModsView({
  mods,
  findings,
  duels = [],
  loadOrder = [],
  at = null,
}: {
  mods: ModInfo[];
  findings: RenderedFinding[];
  /** The duels between two mods, for what of a mod wins or loses. */
  duels?: RenderedDuel[];
  /** The active plugins in load order, for which plugin needs which (ADR 0068). */
  loadOrder?: LoadedPlugin[];
  /** Where another page asked the Mods page to open. */
  at?: DependencyTarget | null;
}) {
  const [tab, setTab] = useState<"list" | "dependencies">("list");
  const [show, setShow] = useState<{ id: string } | null>(null);
  const [pickAt, setPickAt] = useState<{ plugin: string } | null>(null);
  const dependencies = useMemo(
    () => ({
      graph: loadOrder.length === 0 ? null : buildGraph(loadOrder, mods),
      openMod: (id: string) => {
        setTab("list");
        setShow({ id });
      },
      pickAt,
    }),
    [loadOrder, mods, pickAt],
  );
  useEffect(() => {
    if (at?.plugin !== undefined) {
      setTab("dependencies");
      setPickAt({ plugin: at.plugin });
    } else if (at?.mod !== undefined) {
      setTab("list");
      setShow({ id: at.mod });
    }
  }, [at]);
  return (
    <DependencyContext value={dependencies}>
      {dependencies.graph !== null && (
        <ModsTabs
          tab={tab}
          onTab={(next) => {
            setTab(next);
            setShow(null);
          }}
        />
      )}
      {tab === "dependencies" ? (
        <DependencyView />
      ) : (
        <ModList mods={mods} findings={findings} duels={duels} show={show} />
      )}
    </DependencyContext>
  );
}

/** The list of mods, with a mod's page opened in place. */
function ModList({
  mods,
  findings,
  duels,
  show,
}: {
  mods: ModInfo[];
  findings: RenderedFinding[];
  duels: RenderedDuel[];
  /** A mod whose page to show, asked for from a dependency. */
  show: { id: string } | null;
}) {
  const session = useSession();
  const { ignored, patchChoices, spots } = session;
  const o = useOverviewTexts();
  const [search, setSearch] = useState("");
  const [sort, setSort] = useState<ModSort>("problems");
  const [filter, setFilter] = useState<ModFilter>("all");
  const [opened, setOpened] = useState<string | null>(null);
  // A mod's page asked for from a dependency: shown even if filtered out.
  useEffect(() => {
    if (show === null) {
      return;
    }
    setFilter("all");
    setSearch("");
    setOpened(show.id);
    requestAnimationFrame(() =>
      document
        .querySelector(`[data-mod="${CSS.escape(show.id)}"]`)
        ?.scrollIntoView({ block: "start", behavior: "smooth" }),
    );
  }, [show]);

  const states = useMemo(() => {
    const byName = new Map(mods.map((info) => [info.mod.name.toLowerCase(), info]));
    const byId = new Map(mods.map((info) => [info.mod.id, info]));
    const found = new Map<string, ModState>(
      mods.map((info) => [
        info.mod.id,
        {
          info,
          problems: [],
          advice: [],
          overridden: [],
          duels: [],
          collisions: [],
          openSpots: 0,
          crash: false,
        },
      ]),
    );
    for (const rendered of findings) {
      const step = stepOf(rendered, ignored);
      for (const id of modIdsOf(rendered, byName, byId)) {
        const state = found.get(id);
        if (state === undefined) {
          continue;
        }
        if (step === "advice") {
          state.advice.push(rendered);
        } else if (step !== null && PROBLEM_STEPS.has(step)) {
          state.problems.push(rendered);
          state.crash ||= weightOf(rendered).impact === "crash";
        } else if (step === "notes" && isOverride(rendered)) {
          state.overridden.push(rendered);
        }
      }
    }
    for (const rendered of duels) {
      for (const side of rendered.duel.sides) {
        const state = side.mod == null ? undefined : found.get(side.mod.id);
        if (
          state !== undefined &&
          rendered.duel.topics.some(({ topic }) => topic !== "objects" && topic !== "contained")
        ) {
          state.duels.push(rendered);
        }
      }
    }
    // The collisions by the two mods their findings name.
    const pairOf = new Map(
      findings.map((rendered) => [
        rendered.finding.id,
        [rendered.finding.args.first, rendered.finding.args.second].filter(
          (name): name is string => typeof name === "string",
        ),
      ]),
    );
    // Whether a decided spot is in the patch yet does not matter here.
    const all = collisionSpots(
      findings,
      [...spots.values()].flat(),
      new Set(),
      ignored,
      patchChoices,
    );
    for (const spot of all) {
      const names = pairOf.get(spot.finding) ?? [];
      for (const [index, name] of names.entries()) {
        const state = found.get(byName.get(name.toLowerCase())?.mod.id ?? "");
        const otherName = names[1 - index];
        if (state === undefined || otherName === undefined) {
          continue;
        }
        const other = byName.get(otherName.toLowerCase())?.mod.name ?? otherName;
        let entry = state.collisions.find((collided) => collided.other === other);
        if (entry === undefined) {
          entry = {
            other,
            spots: [],
            open: 0,
            kinds: { duplicate: 0, door: 0, overlap: 0, deep: 0, touch: 0 },
          };
          state.collisions.push(entry);
        }
        entry.spots.push(spot);
        entry.kinds[spot.kind] += 1;
        if (spot.state === "open") {
          entry.open += 1;
          state.openSpots += 1;
        }
      }
    }
    for (const state of found.values()) {
      state.collisions.sort((a, b) => b.open - a.open || b.spots.length - a.spots.length);
    }
    return [...found.values()];
  }, [mods, findings, duels, ignored, patchChoices, spots]);

  const needle = search.trim().toLowerCase();
  const shown = states
    .filter(
      (state) =>
        filter === "all" ||
        (filter === "disabled" && !state.info.enabled) ||
        (filter === "problems" && (state.problems.length > 0 || state.openSpots > 0)),
    )
    .filter(
      ({ info }) =>
        needle === "" ||
        info.mod.name.toLowerCase().includes(needle) ||
        (info.author ?? "").toLowerCase().includes(needle),
    )
    .sort((a, b) => {
      const byName = a.info.mod.name.localeCompare(b.info.mod.name);
      if (sort === "name") {
        return byName;
      }
      if (sort === "collisions") {
        return b.openSpots - a.openSpots || b.problems.length - a.problems.length || byName;
      }
      return (
        Number(b.crash) - Number(a.crash) ||
        b.problems.length - a.problems.length ||
        b.openSpots - a.openSpots ||
        byName
      );
    });

  return (
    <section className="mods-page">
      <div className="mods-toolbar">
        <input
          className="text-input mods-search"
          type="search"
          placeholder={o.modsSearch}
          aria-label={o.modsSearch}
          value={search}
          onChange={(event) => setSearch(event.target.value)}
        />
        <fieldset className="segmented" aria-label={o.colState}>
          {(["all", "problems", "disabled"] as ModFilter[]).map((value) => (
            <button
              type="button"
              key={value}
              aria-pressed={filter === value}
              onClick={() => setFilter(value)}
            >
              {o.modsFilter[value]}
            </button>
          ))}
        </fieldset>
        <label className="control mods-sort">
          {o.modsSort}
          <select value={sort} onChange={(event) => setSort(event.target.value as ModSort)}>
            {(["problems", "collisions", "name"] as ModSort[]).map((value) => (
              <option key={value} value={value}>
                {o.modsSortBy[value]}
              </option>
            ))}
          </select>
        </label>
        <span className="aside">{o.modsShown(shown.length, mods.length)}</span>
      </div>

      {shown.length === 0 ? (
        <p className="empty">{o.modsNone}</p>
      ) : (
        <div className="mod-table">
          <div className="mod-row mod-head" aria-hidden="true">
            <span />
            <span>{o.colMod}</span>
            <span>{o.colVersion}</span>
            <span>{o.colState}</span>
            <span>{o.colProblems}</span>
            <span />
          </div>
          <ul className="mod-list">
            {shown.map((state) => (
              <ModRow
                key={state.info.mod.id}
                state={state}
                open={opened === state.info.mod.id}
                onToggle={() => setOpened(opened === state.info.mod.id ? null : state.info.mod.id)}
              />
            ))}
          </ul>
        </div>
      )}
    </section>
  );
}

/** One mod: a line like in a mod manager, and behind it what is open about it. */
function ModRow({
  state,
  open,
  onToggle,
}: {
  state: ModState;
  open: boolean;
  onToggle: () => void;
}) {
  const { api } = useSession();
  const o = useOverviewTexts();
  const bodyId = useId();
  const { info } = state;
  const page = info.page;
  const worst = state.problems.some(({ finding }) => finding.severity === "error")
    ? "error"
    : "warning";
  const wide = wideOf(state);

  return (
    <li className="mod-entry" data-mod={info.mod.id} data-open={open} data-enabled={info.enabled}>
      <div className="mod-row">
        <button
          type="button"
          className="mod-toggle"
          aria-expanded={open}
          aria-controls={bodyId}
          onClick={onToggle}
        >
          <span className="chevron" aria-hidden="true" />
        </button>
        <button type="button" className="mod-name" onClick={onToggle}>
          <strong>{info.mod.name}</strong>
          {wide !== undefined && <WideTag rendered={wide} />}
          {info.author != null && <span className="aside">{info.author}</span>}
        </button>
        <span className="mod-version">
          {info.version ?? "–"}
          {info.newerVersion != null && <span className="newer">{o.newer(info.newerVersion)}</span>}
        </span>
        <span className="mod-state-cell">
          <span className="mod-state" data-enabled={info.enabled}>
            {info.enabled ? o.enabled : o.disabled}
          </span>
        </span>
        <span className="mod-problems-cell">
          {state.problems.length === 0 && state.openSpots === 0 ? (
            <span className="aside">–</span>
          ) : (
            <>
              {state.problems.length > 0 && (
                <span className="problem-chip" data-tone={state.crash ? "crash" : worst}>
                  {o.findingsCount(state.problems.length)}
                </span>
              )}
              {state.openSpots > 0 && (
                <span className="problem-chip" data-tone="collisions">
                  {o.spotsCount(state.openSpots)}
                </span>
              )}
            </>
          )}
        </span>
        <span className="mod-page-cell">
          {page?.startsWith("https://www.nexusmods.com/") && (
            <button type="button" className="small quiet" onClick={() => api.openPage(page)}>
              {o.page}
            </button>
          )}
        </span>
      </div>
      {open && (
        <div className="mod-body" id={bodyId}>
          <ModDetails state={state} />
        </div>
      )}
    </li>
  );
}

/** The suggestion that a mod collides widely, if there is one (ADR 0043). */
const wideOf = (state: ModState) =>
  state.advice.find((rendered) => rendered.finding.kind === COLLIDES_WIDELY);

/** What is open about a mod: findings, collisions, what others cover, suggestions. */
function ModDetails({ state }: { state: ModState }) {
  const { t, decideSpot, mods, showModOnMap } = useSession();
  const wide = wideOf(state);
  const advice = state.advice.filter((rendered) => rendered !== wide);
  const o = useOverviewTexts();
  const nothing =
    state.problems.length === 0 &&
    state.collisions.length === 0 &&
    state.overridden.length === 0 &&
    state.duels.length === 0 &&
    state.advice.length === 0;
  if (nothing) {
    return (
      <>
        <p className="aside">{o.nothingAbout}</p>
        <ModDependencies modId={state.info.mod.id} />
      </>
    );
  }
  return (
    <>
      {wide !== undefined && (
        <WideAdvice
          rendered={wide}
          onShowMod={toneOf(wide) === "keep" ? undefined : () => showModOnMap(state.info.mod.name)}
        />
      )}
      {state.problems.length > 0 && (
        <div className="mod-section tasks">
          <h3>{o.sectionFindings}</h3>
          <ul>
            {state.problems.map((rendered) => (
              <FindingRow key={rendered.finding.id} rendered={rendered} />
            ))}
          </ul>
        </div>
      )}
      {state.collisions.length > 0 && (
        <div className="mod-section">
          <h3>{o.sectionCollisions}</h3>
          <ul className="mod-collisions">
            {state.collisions.map((collided) => {
              const first =
                collided.spots.find((spot) => spot.state === "open") ?? collided.spots[0];
              return (
                <li key={collided.other} className="mod-collision">
                  <strong>{collided.other}</strong>
                  <span className="mod-collision-kinds">
                    {WORK_KINDS.filter((kind) => collided.kinds[kind] > 0).map((kind) => (
                      <span key={kind} className="kind-count" data-kind={kind}>
                        <span className="kind-dot" aria-hidden="true" />
                        {t.spotKinds[kind]} {collided.kinds[kind]}
                      </span>
                    ))}
                  </span>
                  <span className="aside">
                    {o.collisionSpots(collided.open, collided.spots.length)}
                  </span>
                  {first !== undefined && (
                    <button
                      type="button"
                      className={collided.open > 0 ? "small primary" : "small"}
                      onClick={() => decideSpot(first.spot)}
                    >
                      {o.decideInWorklist}
                    </button>
                  )}
                </li>
              );
            })}
          </ul>
        </div>
      )}
      {(state.overridden.length > 0 || state.duels.length > 0) && (
        <div className="mod-section">
          <h3>{o.sectionOverridden}</h3>
          <ul className="mod-overridden">
            {state.duels.flatMap((rendered) => {
              const names = duelNames(rendered.duel, mods);
              return rendered.duel.topics.flatMap((topic, index) => {
                if (topic.topic === "objects" || topic.topic === "contained") {
                  return [];
                }
                const label = rendered.topicLabels[index] ?? topic.topic;
                const winner = topic.winning == null ? undefined : names[topic.winning];
                return (
                  <li key={`${rendered.duel.id}/${topic.topic}`}>
                    <span className="aside">{names.join(" · ")}</span>{" "}
                    {winner === undefined ? o.undecidedTopic(label) : o.wins(label, winner)}
                  </li>
                );
              });
            })}
            {state.overridden.map((rendered) => (
              <li key={rendered.finding.id}>{rendered.headline}</li>
            ))}
          </ul>
        </div>
      )}
      {advice.length > 0 && (
        <div className="mod-section tasks">
          <h3>
            {o.sectionAdvice} <span className="tag optional-tag">{o.optional}</span>
          </h3>
          <ul>
            {advice.map((rendered) => (
              <FindingRow key={rendered.finding.id} rendered={rendered} />
            ))}
          </ul>
        </div>
      )}
      <ModDependencies modId={state.info.mod.id} />
    </>
  );
}
