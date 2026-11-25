import { createContext, type ReactNode, useContext, useId, useMemo, useState } from "react";
import { ActionResult, type ActionState, CopyButton, useAction, useSession } from "./Actions";
import {
  type Filter,
  type Group,
  isCollision,
  modsOf,
  openTaskCount,
  type Section,
  sectionsFor,
} from "./arrange";
import type {
  MapSpot,
  ModInfo,
  RenderedAction,
  RenderedDuel,
  RenderedFinding,
  RenderedReport,
} from "./generated/rendered-report";
import { type Impact, reportSpots, type SpotState, weightOf } from "./health";
import type { Texts } from "./i18n";
import { useOverviewTexts } from "./overview-texts";
import { Previews } from "./Preview";
import { LEFT_BEHIND, STILL_LOADED, StillLoadedHelp } from "./still-loaded";
import {
  countCategories,
  type ListView,
  type PartId,
  type PlannedGroup as Planned,
  type PlannedPart,
  planView,
  rowModsOf,
  TO_DO,
} from "./task-plan";
import { useTaskRowTexts } from "./task-row-texts";
import { ToolHint } from "./tool-hint";
import { EmptyState, Note, PageHeader, UndoButton } from "./ui/kit";
import { ModLine, SeverityMark, TaskGroup, TaskPart, TaskRow } from "./ui/rows";
import { type Tile, TileTabs } from "./ui/tiles";
import { COLLIDES_WIDELY, toneOf, WideAdvice } from "./wide-advice";
import "./overview.css";
import "./tasks.css";

/** A group with more findings than this starts folded. */
const OPEN_GROUP_LIMIT = 3;

/** How many spots of a finding are listed. The map shows all of them. */
const SPOT_LIMIT = 8;

/** Action outcomes after which the task is in the mod manager's hands. */
const HANDED_ON = new Set(["started", "installing", "applied"]);

/** What a finding does to the game, for its mark. */
const impactOf = (rendered: RenderedFinding): Impact => weightOf(rendered).impact;

/**
 * Sets a finding aside on the task page and offers the way back; outside
 * the task page there is none, and setting aside only sets aside.
 */
const AsideNotice = createContext<((id: string, title: string) => void) | undefined>(undefined);

function sectionTexts(id: Section["id"], t: Texts): { title: string; hint: string } {
  switch (id) {
    case "error":
      return { title: t.filterFix, hint: t.fixHint };
    case "warning":
      return { title: t.filterCheck, hint: t.checkHint };
    case "direct":
      return { title: t.filterDirect, hint: t.directSectionHint };
    case "info":
      return { title: t.filterNotes, hint: t.notesHint };
    case "ignored":
      return { title: t.filterIgnored, hint: t.ignoredHint };
    case "collisions":
      return { title: t.viewCollisions, hint: t.collisionsHint };
  }
}

/**
 * Findings as a list of things to do, by a filter: one row each, or one
 * row for several of a kind, with the button that does something about
 * it where there is one.
 */
export function TaskList({
  findings,
  filter,
  exclude,
}: {
  findings: RenderedFinding[];
  filter: Filter;
  /** Findings shown elsewhere, such as in a duel. */
  exclude?: Set<string>;
}) {
  const { t, ignored } = useSession();
  const shown =
    exclude === undefined
      ? findings
      : findings.filter((rendered) => !exclude.has(rendered.finding.id));
  const sections = sectionsFor(shown, filter, ignored);

  if (sections.length === 0) {
    return <p className="empty">{t.nothingHere}</p>;
  }
  return (
    <div className="task-sections" data-row-scope="">
      {sections.map((section) => {
        const { title, hint } = sectionTexts(section.id, t);
        return (
          <section className="tasks" key={section.id} data-section={section.id}>
            <header title={hint}>
              <h2>
                {title} <span className="count">{section.count}</span>
              </h2>
            </header>
            <GroupList groups={section.groups.map((group) => planned(section.id, group))} />
          </section>
        );
      })}
    </div>
  );
}

/** A group of `arrange.ts` as the task page plans one. */
function planned(part: string, group: Group): Planned {
  const mods = [...new Set(group.findings.flatMap(rowModsOf))];
  const impacts = group.findings.map(impactOf);
  const order: Impact[] = ["crash", "broken", "quality", "scan", "note", "advice"];
  const impact = order.find((level) => impacts.includes(level)) ?? "note";
  return {
    key: `${part}/${group.kind}`,
    kind: group.kind,
    title: group.title,
    findings: group.findings,
    mods,
    impact,
  };
}

/** The rows of a part: one per finding, one per kind where there are several. */
function GroupList({ groups, calm = false }: { groups: Planned[]; calm?: boolean }) {
  return (
    <ul className="task-list">
      {groups.map((group) =>
        group.findings.length === 1 ? (
          group.findings.map((rendered) => (
            <FindingRow key={rendered.finding.id} rendered={rendered} calm={calm} />
          ))
        ) : (
          <FindingGroup key={group.key} group={group} calm={calm} />
        ),
      )}
    </ul>
  );
}

/** Several findings of one kind behind one row that opens into theirs. */
function FindingGroup({ group, calm }: { group: Planned; calm: boolean }) {
  const { showToolbox } = useSession();
  const o = useOverviewTexts();
  const r = useTaskRowTexts();
  const [open, setOpen] = useState(group.findings.length <= OPEN_GROUP_LIMIT);
  const { mods } = group;
  const impact = calm ? "note" : group.impact;
  // The tool that repairs paths repairs them all: offered once, on the group;
  // not where there is nothing to do.
  const shared =
    !calm && group.kind.startsWith("navmeshes.") && showToolbox !== undefined
      ? showToolbox
      : undefined;
  return (
    <TaskGroup
      primary={
        shared !== undefined && (
          <button type="button" className="task-action" onClick={shared}>
            {o.navmeshChoice}
          </button>
        )
      }
      level={impact}
      levelLabel={o.impact[impact]}
      title={group.title}
      count={group.findings.length}
      meta={<ModLine mods={mods} more={r.moreMods} />}
      open={open}
      onToggle={() => setOpen(!open)}
      toggleLabel={open ? r.fold : r.showAll(group.findings.length)}
    >
      {group.findings.map((rendered) => (
        <FindingRow
          key={rendered.finding.id}
          rendered={rendered}
          groupLevel={impact}
          groupShares={shared !== undefined}
          calm={calm}
        />
      ))}
    </TaskGroup>
  );
}

/** The one thing a row offers to do, beside opening it. */
interface Primary {
  /** Which of the row's ways it is, so that the opened row does not offer it twice. */
  key: string;
  label: string;
  title?: string;
  run: () => void;
  /** Whether it fixes something; a mod's page or the map is offered more quietly. */
  strong: boolean;
}

/**
 * One finding as a row: what it does to the game, a short title, the mods
 * it is about and the one thing to do. Opened: what it is, every way out,
 * and what there is to know about it.
 */
export function FindingRow({
  rendered,
  groupLevel,
  groupShares = false,
  calm = false,
}: {
  rendered: RenderedFinding;
  /** In a list of what is only to know or set aside: the mark of a note, whatever it would do. */
  calm?: boolean;
  /** The level of the group the row is in; its mark is left out where it is the same. */
  groupLevel?: Impact;
  /** Whether the group offers the ways every finding of it shares, so that the row does not. */
  groupShares?: boolean;
}) {
  const { t, mods, rescan } = useSession();
  const o = useOverviewTexts();
  const r = useTaskRowTexts();
  const [open, setOpen] = useState(false);
  const action = useAction();
  const known = modsOf(rendered).flatMap((name) => {
    const info = mods.get(name);
    return info === undefined ? [] : [info];
  });
  const ways = useWays(rendered, known);
  const impact = calm ? "note" : impactOf(rendered);
  const status = action.outcome?.outcome.status;
  const handed = status !== undefined && HANDED_ON.has(status);
  const primary = primaryOf(
    rendered,
    groupShares ? ways.filter((way) => !way.shared) : ways,
    action,
    () => setOpen(true),
  );
  // What an action answers is shown at once; once handed on, only on the line.
  const showBody = open || (action.touched && !handed);

  return (
    <TaskRow
      level={handed ? "done" : impact}
      levelLabel={handed ? r.handedOn : o.impact[impact]}
      title={rendered.headline}
      meta={
        <>
          <ModLine mods={rowModsOf(rendered)} more={r.moreMods} />
          {handed && (
            <>
              <span className="task-handed">{r.handedOn}</span>
              {/* Once the mod manager is done, a new scan shows whether it worked. */}
              <button type="button" className="small quiet" onClick={rescan}>
                {t.rescan}
              </button>
            </>
          )}
        </>
      }
      primary={
        primary !== null &&
        !handed && (
          <button
            type="button"
            className={primary.strong ? "task-action" : "task-action-plain"}
            title={primary.title}
            disabled={action.busy}
            onClick={primary.run}
          >
            {primary.label}
          </button>
        )
      }
      open={showBody}
      onToggle={() => setOpen(!showBody)}
      toggleLabel={r.details}
      nested={groupLevel !== undefined && groupLevel === impact && !handed}
    >
      <FindingBody
        rendered={rendered}
        action={action}
        ways={ways}
        primary={primary}
        known={known}
      />
    </TaskRow>
  );
}

/**
 * The one thing to do: what Mod Mallet can hand to the mod manager, else
 * the way it suggests. Switching off a mod that others rely on, or the
 * plugins of one that is switched off but still loaded, opens the row
 * first: what relies on it is said there, and asked about (ADR 0061, 0073).
 */
function primaryOf(
  rendered: RenderedFinding,
  ways: Way[],
  action: ActionState,
  openRow: () => void,
): Primary | null {
  const { kind } = rendered.finding;
  const first = rendered.actions[0];
  const wide = kind === COLLIDES_WIDELY;
  if (first !== undefined && (!wide || toneOf(rendered) === "remove")) {
    const asks = wide || first.action.type === "disable-plugins";
    return {
      key: asks ? "asks" : "action:0",
      label: first.label,
      title: first.description,
      run: asks ? openRow : () => void action.run(first.action),
      strong: true,
    };
  }
  const way = ways.find((candidate) => candidate.primary) ?? (wide ? ways[0] : undefined);
  return way === undefined
    ? null
    : {
        key: way.key,
        label: way.label,
        run: way.run,
        strong: way.key !== "map" && !way.key.startsWith("page:"),
      };
}

/** What an opened row shows. */
function FindingBody({
  rendered,
  action,
  ways,
  primary,
  known,
}: {
  rendered: RenderedFinding;
  action: ActionState;
  ways: Way[];
  primary: Primary | null;
  known: ModInfo[];
}) {
  const { t, spots } = useSession();
  const r = useTaskRowTexts();
  const { finding } = rendered;
  const where = spots.get(finding.id) ?? [];
  const wide = finding.kind === COLLIDES_WIDELY;
  return (
    <>
      <ActionResult state={action} />
      {wide ? (
        <WideAdvice rendered={rendered} />
      ) : (
        <p className="task-summary-text">{rendered.summary}</p>
      )}
      {(finding.kind === STILL_LOADED || finding.kind === LEFT_BEHIND) && (
        <StillLoadedHelp rendered={rendered} />
      )}
      <Ways rendered={rendered} action={action} ways={ways} skip={primary?.key} />
      <ToolHint kind={finding.kind} />
      {(finding.previews?.length ?? 0) > 0 && <Previews paths={finding.previews ?? []} />}
      {rendered.actions.map((available) => (
        <Guide key={available.label} action={available} />
      ))}
      <div className="task-facts">
        <div>
          <h3>{t.whatToDo}</h3>
          <p>{rendered.remedy}</p>
          <h3>{t.whatItMeans}</h3>
          <p>{rendered.explanation}</p>
        </div>
        {(known.length > 0 || where.length > 0) && (
          <div>
            {known.length > 0 && (
              <>
                <h3>{r.mods}</h3>
                <ul className="mod-facts">
                  {known.map((info) => (
                    <ModFacts key={info.mod.id} info={info} />
                  ))}
                </ul>
              </>
            )}
            {where.length > 0 && <Spots spots={where} collision={isCollision(rendered)} />}
          </div>
        )}
      </div>
      <MissingFiles rendered={rendered} />
      <p className="meta">
        {rendered.severityLabel} · {rendered.confidenceLabel}
        {finding.subject.type === "file" && (
          <>
            {` · ${t.file}: `}
            <code>{finding.subject.path}</code>
          </>
        )}
      </p>
    </>
  );
}

/**
 * Every texture a mod's meshes miss, with the meshes that use it, to look
 * for on the mod's page (ADR 0056). The finding's `list` holds one texture
 * per line, the meshes after a tab.
 */
function MissingFiles({ rendered }: { rendered: RenderedFinding }) {
  const r = useTaskRowTexts();
  const list = rendered.finding.args.list;
  if (
    rendered.finding.kind !== "meshes.texture-missing" ||
    typeof list !== "string" ||
    list === ""
  ) {
    return null;
  }
  const rows = list.split("\n").map((line) => {
    const [texture = "", meshes = ""] = line.split("\t");
    return { texture, meshes };
  });
  return (
    <div className="missing-files">
      <h3>
        {r.missingTextures(rows.length)}{" "}
        <CopyButton text={rows.map(({ texture }) => texture).join("\n")} />
      </h3>
      <ul>
        {rows.map(({ texture, meshes }) => (
          <li key={texture}>
            <code>{texture}</code>
            {meshes !== "" && <span className="aside"> {r.usedBy(meshes)}</span>}
          </li>
        ))}
      </ul>
    </div>
  );
}

/**
 * A mod a finding is about, in one line: its version, author, whether it is
 * switched off and whether there is a newer one. Its page is a button above.
 */
function ModFacts({ info }: { info: ModInfo }) {
  const { t } = useSession();
  return (
    <li>
      <strong>{info.mod.name}</strong>
      {!info.enabled && <span className="tag">{t.disabled}</span>}
      {info.version != null && ` · ${t.version} ${info.version}`}
      {info.author != null && ` · ${t.author}: ${info.author}`}
      {info.newerVersion != null && (
        <span className="newer"> · {t.newerVersion(info.newerVersion)}</span>
      )}
    </li>
  );
}

/** A way out of a finding besides the mod manager's actions and leaving it. */
interface Way {
  key: string;
  label: string;
  run: () => void;
  /** The way Mod Mallet suggests, drawn as the main button. */
  primary: boolean;
  /** Whether it is the same for every finding of the kind, such as a tool. */
  shared?: boolean;
}

/**
 * The ways out of a finding, the one Mod Mallet suggests first: where a
 * collision is decided, the tool that repairs it, the mod's page, the
 * place on the map.
 */
function useWays(rendered: RenderedFinding, known: ModInfo[]): Way[] {
  const { t, api, spots, decideSpot, openPair, showToolbox, showOnMap, showModOnMap } =
    useSession();
  const o = useOverviewTexts();
  const r = useTaskRowTexts();
  const { finding } = rendered;
  const collision = isCollision(rendered);
  const where = spots.get(finding.id) ?? [];
  const ways: Way[] = [];

  if (collision) {
    // The first spot to open, or the pair of mods where no spot names its objects.
    const first = where[0];
    const pair = pairOf(rendered);
    if (first !== undefined) {
      ways.push({
        key: "worklist",
        label: o.worklistOpen,
        run: () => decideSpot(first),
        primary: true,
      });
    }
    if (pair !== null) {
      ways.push({
        key: "pair",
        label: o.pairOpen,
        run: () => openPair(pair[0], pair[1]),
        primary: first === undefined,
      });
    }
  }
  // A suggestion about a whole mod leads to where its collisions are decided.
  const { mod, other, others, modCount } = finding.args;
  if (
    finding.kind === "mods.duplicates-another" &&
    typeof mod === "string" &&
    typeof other === "string"
  ) {
    ways.push({
      key: "pair",
      label: o.pairOpen,
      run: () => openPair(mod, other),
      primary: true,
    });
  }
  if (finding.kind === COLLIDES_WIDELY && typeof mod === "string") {
    // With one other mod, `others` is its name and its count of spots: "Large Town (25)".
    const single =
      modCount === 1 && typeof others === "string" ? others.replace(/ \(\d+\)$/, "") : null;
    if (single !== null) {
      ways.push({
        key: "pair",
        label: o.pairOpen,
        run: () => openPair(mod, single),
        primary: true,
      });
    }
    ways.push({
      key: "mod-map",
      label: r.modOnMap(mod),
      run: () => showModOnMap(mod),
      primary: single === null,
    });
  }
  if (finding.kind.startsWith("navmeshes.") && showToolbox !== undefined) {
    // Named by what it is for; the button opens the tool's row in the toolbox.
    ways.push({
      key: "toolbox",
      label: o.navmeshChoice,
      run: showToolbox,
      primary: false,
      shared: true,
    });
  }
  for (const info of known) {
    const page = info.page;
    if (page?.startsWith("https://www.nexusmods.com/")) {
      ways.push({
        key: `page:${info.mod.id}`,
        label: known.length === 1 ? t.openPage : r.modPage(info.mod.name),
        run: () => api.openPage(page),
        primary: false,
      });
    }
  }
  const onMap = collision ? undefined : where.find((spot) => spot.location.type === "exterior");
  if (onMap !== undefined) {
    ways.push({
      key: "map",
      label: t.showOnMap,
      run: () => showOnMap(onMap),
      primary: false,
    });
  }
  // Where Mod Mallet hands nothing to the mod manager, the first way is the main one.
  const first = ways[0];
  if (rendered.actions.length === 0 && first !== undefined && !ways.some((way) => way.primary)) {
    first.primary = true;
  }
  return ways;
}

/**
 * The buttons of an opened row, in one row: what Mod Mallet can hand to
 * the mod manager, then the other ways out, then leaving it as it is. The
 * one the row's line offers is not repeated.
 */
function Ways({
  rendered,
  action,
  ways,
  skip,
}: {
  rendered: RenderedFinding;
  action: ActionState;
  ways: Way[];
  skip: string | undefined;
}) {
  const { t, ignored, setIgnored } = useSession();
  const notice = useContext(AsideNotice);
  const { finding } = rendered;
  const aside = ignored.has(finding.id);
  // A mod that collides widely is switched off from its advice above, which
  // asks first about what relies on it (ADR 0061); the plugins of a
  // switched-off mod from the help above, likewise (ADR 0073).
  const actions =
    finding.kind === COLLIDES_WIDELY
      ? []
      : rendered.actions.flatMap((available, index) =>
          available.action.type === "disable-plugins" || `action:${index}` === skip
            ? []
            : [{ available, index }],
        );

  return (
    <div className="task-ways">
      {actions.map(({ available, index }) => (
        <button
          type="button"
          className="task-action"
          key={`action:${index}`}
          title={available.description}
          disabled={action.busy}
          onClick={() => void action.run(available.action)}
        >
          {available.label}
        </button>
      ))}
      {ways
        .filter((way) => way.key !== skip)
        .map((way) => (
          <button type="button" key={way.key} onClick={way.run}>
            {way.label}
          </button>
        ))}
      <button
        type="button"
        className="quiet task-aside"
        onClick={() => {
          setIgnored(finding.id, !aside);
          if (!aside) {
            notice?.(finding.id, rendered.headline);
          }
        }}
      >
        {aside ? t.unignore : t.choiceLeave}
      </button>
    </div>
  );
}

/** The options to tick in a mod's installer, page by page. */
function Guide({ action }: { action: RenderedAction }) {
  const { t } = useSession();
  if (action.guide.length === 0) {
    return null;
  }
  return (
    <div className="guide">
      <h3>{t.guideTitle}</h3>
      <ol className="guide-list">
        {action.guide.map(({ tick, pageLabel }) => (
          <li className="guide-item" key={`${tick.step}/${tick.group}/${tick.option}`}>
            <span className="guide-page">{pageLabel}</span>
            <span className="guide-where">
              {tick.step}
              {tick.group !== "" && tick.group !== tick.step && ` › ${tick.group}`}
            </span>
            <span className="guide-tick">
              <span className="guide-box" aria-hidden="true">
                ✓
              </span>
              {tick.option}
            </span>
            {tick.description !== "" && <span className="guide-about">{tick.description}</span>}
          </li>
        ))}
      </ol>
    </div>
  );
}

/**
 * Where a finding is, with the way there in the game: a collision's spots
 * open in the worklist, where they are decided; any other place on the map.
 */
function Spots({ spots, collision }: { spots: MapSpot[]; collision: boolean }) {
  const { t, showOnMap, decideSpot } = useSession();
  const o = useOverviewTexts();
  const shown = spots.slice(0, SPOT_LIMIT);
  return (
    <div className="spots">
      <h3>{t.spotsTitle(shown.length, spots.length)}</h3>
      <ul className="spot-list">
        {shown.map((spot) => (
          <li className="spot" key={`${spot.console}/${spot.objects}`}>
            <span className="spot-objects">{spot.objects}</span>
            <code className="command" title={t.consoleHint}>
              {spot.console}
            </code>
            <CopyButton text={spot.console} />
            {collision ? (
              <button type="button" className="small quiet" onClick={() => decideSpot(spot)}>
                {o.spotToWorklist}
              </button>
            ) : (
              spot.location.type === "exterior" && (
                <button type="button" className="small quiet" onClick={() => showOnMap(spot)}>
                  {t.showOnMap}
                </button>
              )
            )}
          </li>
        ))}
      </ul>
    </div>
  );
}

/** What the tiles choose: everything open, the collisions, or a category. */
type View = ListView | "collisions";

/** The categories that are not things to do: their tiles step back. */
const QUIET = new Set<View>(["notes", "ignored"]);

/**
 * The task page (ADR 0045, 0076, 0082): a board of tiles, one per category
 * with its count and what it means, and under it the list of the tile
 * chosen. Everything open is shown first, its crash risks leading.
 */
export function TasksPage({ report }: { report: RenderedReport }) {
  const { ignored, setIgnored, decisions } = useSession();
  const o = useOverviewTexts();
  const r = useTaskRowTexts();
  const [view, setView] = useState<View>("open");
  const [aside, setAside] = useState<{ id: string; title: string } | null>(null);
  const panelId = useId();
  // Duels about the same files in two mods are decided on Assets; their
  // findings are not listed again here, only one line leads there. Duels
  // about placed objects belong to the collisions.
  const duels = useMemo(
    () =>
      report.duels.filter((rendered) =>
        rendered.duel.topics.some(({ topic }) => topic !== "objects" && topic !== "contained"),
      ),
    [report.duels],
  );
  const inDuels = useMemo(
    () => new Set(report.duels.flatMap((rendered) => rendered.duel.findings)),
    [report.duels],
  );
  const listed = useMemo(
    () =>
      report.findings.filter(
        (rendered) => ignored.has(rendered.finding.id) || !inDuels.has(rendered.finding.id),
      ),
    [report.findings, ignored, inDuels],
  );
  const counts = useMemo(() => countCategories(listed, ignored, impactOf), [listed, ignored]);
  const collisions = useCollisionState(report);
  const open = openTaskCount(report.findings, ignored);
  const crash = TO_DO.reduce((sum, category) => sum + counts[category].crash, 0);
  const openDuels = duels.filter((rendered) => decisions[rendered.duel.id] === undefined).length;

  const crashBadge = (count: number) =>
    count > 0 ? (
      <>
        <SeverityMark level="crash" label={o.impact.crash} />
        {r.crashCount(count)}
      </>
    ) : undefined;
  const tile = (id: View, count: number, badge?: ReactNode): Tile<View> => ({
    id,
    label: r.tile[id].label,
    meaning: r.tile[id].meaning,
    count,
    badge,
    quiet: QUIET.has(id),
  });
  const tiles: Tile<View>[] = [
    tile("open", open, crashBadge(crash)),
    tile("collisions", collisions.states.open),
    tile("one-click", counts["one-click"].count, crashBadge(counts["one-click"].crash)),
    tile("decide", counts.decide.count + duels.length, crashBadge(counts.decide.crash)),
    tile("yourself", counts.yourself.count, crashBadge(counts.yourself.crash)),
    tile("advice", counts.advice.count),
    tile("notes", counts.notes.count),
    tile("ignored", counts.ignored.count),
  ];

  return (
    <AsideNotice.Provider value={(id, title) => setAside({ id, title })}>
      <section className="tasks-page" aria-live="polite">
        <PageHeader
          status={
            <h2 className="tasks-headline">
              {open === 0 ? o.tasksHeadlineNone : o.tasksHeadline(open)}
            </h2>
          }
        />
        <TileTabs
          label={r.tilesLabel}
          tiles={tiles}
          value={view}
          onChange={setView}
          panelId={panelId}
        />
        <div className="task-view" id={panelId} role="tabpanel" data-view={view} data-row-scope="">
          {view === "collisions" ? (
            <CollisionsView report={report} />
          ) : (
            <ListOf
              view={view}
              findings={listed}
              duels={duels}
              openDuels={openDuels}
              nothingOpen={open === 0}
            />
          )}
        </div>
        {aside !== null && (
          <div className="task-undo" role="status">
            <span className="task-undo-text">{r.setAside(aside.title)}</span>
            <UndoButton
              onClick={() => {
                setIgnored(aside.id, false);
                setAside(null);
              }}
            />
          </div>
        )}
      </section>
    </AsideNotice.Provider>
  );
}

/**
 * The list under the tiles: everything open, numbered in the order to work
 * in with the crash risks first, or the rows of one category under a line
 * that says what it is and leads to the page where it is handled.
 */
function ListOf({
  view,
  findings,
  duels,
  openDuels,
  nothingOpen,
}: {
  view: ListView;
  findings: RenderedFinding[];
  duels: RenderedDuel[];
  openDuels: number;
  nothingOpen: boolean;
}) {
  const { ignored } = useSession();
  const r = useTaskRowTexts();
  const parts = useMemo(
    () => planView(findings, ignored, impactOf, view),
    [findings, ignored, view],
  );
  if (view === "open") {
    const withDuels =
      duels.length > 0 && !parts.some(({ id }) => id === "decide") ? insertDecide(parts) : parts;
    if (nothingOpen && withDuels.length === 0) {
      return (
        <Note tone="ok" className="tasks-clear">
          {r.allClearNote}
        </Note>
      );
    }
    return (
      <>
        {withDuels.map(({ id, groups, count }, index) => (
          <TaskPart
            key={id}
            id={id}
            number={index + 1}
            title={r.partTitle[id]}
            count={id === "decide" ? count + openDuels : count}
            open
          >
            {id === "decide" && duels.length > 0 && <DuelsLine duels={duels} />}
            {groups.length > 0 && <GroupList groups={groups} />}
          </TaskPart>
        ))}
      </>
    );
  }
  const [part] = parts;
  return (
    <>
      <ViewHead view={view} />
      {view === "decide" && duels.length > 0 && <DuelsLine duels={duels} />}
      {part === undefined ? (
        view === "decide" && duels.length > 0 ? null : (
          <EmptyState>{r.emptyCategory}</EmptyState>
        )
      ) : (
        <GroupList groups={part.groups} calm={view === "notes" || view === "ignored"} />
      )}
    </>
  );
}

/** The parts with an empty decision part where the order puts it, for the duels. */
function insertDecide(parts: PlannedPart[]): PlannedPart[] {
  const after = new Set<PartId>(["crash", "one-click"]);
  const index = parts.findIndex(({ id }) => !after.has(id));
  const at = index === -1 ? parts.length : index;
  return [...parts.slice(0, at), { id: "decide", groups: [], count: 0 }, ...parts.slice(at)];
}

/** What a category is, and the way to the page where it is handled. */
function ViewHead({ view }: { view: Exclude<ListView, "open"> }) {
  const { showToolbox, showMods } = useSession();
  const r = useTaskRowTexts();
  // Decisions lead to Assets from their duels' line, under this head.
  const jump: { label: string; run: () => void } | null =
    view === "yourself" && showToolbox !== undefined
      ? { label: r.toToolbox, run: showToolbox }
      : (view === "advice" || view === "one-click") && showMods !== undefined
        ? { label: r.toMods, run: showMods }
        : null;
  return (
    <header className="task-view-head">
      <div className="task-view-text">
        <h2>{r.partTitle[view]}</h2>
        <span className="aside">{r.tile[view].hint}</span>
      </div>
      {jump !== null && (
        <button type="button" onClick={jump.run}>
          {jump.label}
        </button>
      )}
    </header>
  );
}

/**
 * The collisions under their tile: how many spots are left, the ways to
 * the worklist and the patch, and the collisions by kind and pair of mods.
 */
function CollisionsView({ report }: { report: RenderedReport }) {
  const { ignored } = useSession();
  const [section] = sectionsFor(report.findings, "collisions", ignored);
  return (
    <>
      <CollisionsCard report={report} />
      {section !== undefined && (
        <GroupList groups={section.groups.map((group) => planned(section.id, group))} />
      )}
    </>
  );
}

/** Every collision spot with its state, and the pairs of mods still to decide. */
function useCollisionState(report: RenderedReport) {
  const { ignored, patchChoices } = useSession();
  const spots = useMemo(
    () => reportSpots(report, ignored, patchChoices),
    [report, ignored, patchChoices],
  );
  // Pairs of mods, whatever kind of collision they have.
  const pairOf = useMemo(() => {
    const found = new Map<string, string>();
    for (const { finding } of report.findings) {
      const names = [
        finding.args.first,
        finding.args.second,
        finding.args.plugin,
        finding.args.container,
      ]
        .filter((name) => typeof name === "string")
        .map(String)
        .sort();
      found.set(finding.id, names.join("|"));
    }
    return found;
  }, [report.findings]);
  const states: Record<SpotState, number> = { open: 0, kept: 0, waiting: 0, patched: 0 };
  for (const spot of spots) {
    states[spot.state] += 1;
  }
  const pairs = new Set(
    spots
      .filter((spot) => spot.state === "open")
      .map((spot) => pairOf.get(spot.finding) ?? spot.finding),
  ).size;
  const first = spots.find((spot) => spot.state === "open") ?? spots[0];
  return { spots, states, pairs, first };
}

/**
 * The collisions in one line: how many spots are left to decide, and the
 * way to the worklist; and what is decided but not in the patch yet.
 */
function CollisionsCard({ report }: { report: RenderedReport }) {
  const { decideSpot, showPatch } = useSession();
  const o = useOverviewTexts();
  const headingId = useId();
  const { spots, states, pairs, first } = useCollisionState(report);

  return (
    <section
      className="panel collisions-card"
      data-open={states.open > 0}
      aria-labelledby={headingId}
    >
      <div className="collisions-card-text">
        <strong id={headingId}>{o.collisionsTitle}</strong>
        <span>
          {spots.length === 0
            ? o.collisionsNone
            : states.open === 0
              ? o.collisionsAllDecided(spots.length)
              : o.collisionsSummary(states.open, spots.length, pairs)}
          {states.waiting > 0 && ` ${o.collisionsWaiting(states.waiting)}`}
        </span>
      </div>
      {states.waiting > 0 && (
        <button type="button" onClick={showPatch}>
          {o.collisionsToPatch}
        </button>
      )}
      {first !== undefined && (
        <button
          type="button"
          className={states.open > 0 ? "task-action" : undefined}
          onClick={() => decideSpot(first.spot)}
        >
          {o.collisionsOpen}
        </button>
      )}
    </section>
  );
}

/** The names of the two mods a collision finding is about, if it names them. */
function pairOf(rendered: RenderedFinding): [string, string] | null {
  const { args } = rendered.finding;
  const first = args.first ?? args.plugin;
  const second = args.second ?? args.container;
  return typeof first === "string" && typeof second === "string" ? [first, second] : null;
}

/**
 * The duels about files in one line, like the collisions' line: how many
 * are still to decide, and the way to Assets, where they are decided.
 */
function DuelsLine({ duels }: { duels: RenderedDuel[] }) {
  const { decisions, openCompare } = useSession();
  const o = useOverviewTexts();
  const open = duels.filter((rendered) => decisions[rendered.duel.id] === undefined).length;
  return (
    <div className="panel collisions-card duels-line" data-open={open > 0}>
      <div className="collisions-card-text">
        <span>
          {open === 0 ? o.duelsAllDecided(duels.length) : o.duelsLine(open, duels.length)}
        </span>
      </div>
      <button
        type="button"
        className={open > 0 ? "task-action" : undefined}
        onClick={() => openCompare(null)}
      >
        {o.toAssets}
      </button>
    </div>
  );
}
