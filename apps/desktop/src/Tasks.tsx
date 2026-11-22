import {
  createContext,
  Fragment,
  type ReactNode,
  useContext,
  useId,
  useMemo,
  useState,
} from "react";
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
  type PartId,
  type PlannedGroup as Planned,
  type PlannedPart,
  planTasks,
  rowModsOf,
} from "./task-plan";
import { useTaskRowTexts } from "./task-row-texts";
import { ModLine, SeverityMark, TaskGroup, TaskPart, TaskRow } from "./task-rows";
import { ToolHint } from "./tool-hint";
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
function GroupList({ groups }: { groups: Planned[] }) {
  return (
    <ul className="task-list">
      {groups.map((group) =>
        group.findings.length === 1 ? (
          group.findings.map((rendered) => (
            <FindingRow key={rendered.finding.id} rendered={rendered} />
          ))
        ) : (
          <FindingGroup key={group.key} group={group} />
        ),
      )}
    </ul>
  );
}

/** Several findings of one kind behind one row that opens into theirs. */
function FindingGroup({ group }: { group: Planned }) {
  const { showToolbox } = useSession();
  const o = useOverviewTexts();
  const r = useTaskRowTexts();
  const [open, setOpen] = useState(group.findings.length <= OPEN_GROUP_LIMIT);
  const { mods } = group;
  // The tool that repairs paths repairs them all: offered once, on the group.
  const shared =
    group.kind.startsWith("navmeshes.") && showToolbox !== undefined ? showToolbox : undefined;
  return (
    <TaskGroup
      primary={
        shared !== undefined && (
          <button type="button" className="task-action" onClick={shared}>
            {o.navmeshChoice}
          </button>
        )
      }
      level={group.impact}
      levelLabel={o.impact[group.impact]}
      title={group.title}
      count={group.findings.length}
      meta={
        mods.length <= 2 ? (
          <ModLine mods={mods} more={r.moreMods} />
        ) : (
          <>
            <ModLine mods={mods} shown={5} names={false} more={() => ""} />
            <span>{r.modCount(mods.length)}</span>
          </>
        )
      }
      open={open}
      onToggle={() => setOpen(!open)}
      toggleLabel={open ? r.fold : r.showAll(group.findings.length)}
    >
      {group.findings.map((rendered) => (
        <FindingRow
          key={rendered.finding.id}
          rendered={rendered}
          groupLevel={group.impact}
          groupShares={shared !== undefined}
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
}: {
  rendered: RenderedFinding;
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
  const impact = impactOf(rendered);
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

/** The parts that are only to know or set aside; they start folded. */
const FOLDED = new Set<PartId>(["notes", "ignored"]);

/** The parts that are not things to do, and get no number. */
const UNNUMBERED = new Set<PartId>(["advice", "notes", "ignored"]);

/**
 * The task page (ADR 0045, 0076): what can crash the game first, then
 * the steps by what the user does, the collisions summed up in one line,
 * then the recommendations, and folded what is only to know or set aside.
 */
export function TasksPage({ report }: { report: RenderedReport }) {
  const { ignored, setIgnored } = useSession();
  const o = useOverviewTexts();
  const r = useTaskRowTexts();
  const [aside, setAside] = useState<{ id: string; title: string } | null>(null);
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
  const parts = useMemo(
    () =>
      planTasks(
        report.findings.filter(
          (rendered) => ignored.has(rendered.finding.id) || !inDuels.has(rendered.finding.id),
        ),
        ignored,
        impactOf,
      ),
    [report.findings, ignored, inDuels],
  );
  const open = openTaskCount(report.findings, ignored);
  // The choice between mods leads to Assets even when no finding is in it.
  const withDuels =
    duels.length > 0 && !parts.some(({ id }) => id === "choose") ? insertChoose(parts) : parts;
  let number = 0;

  return (
    <AsideNotice.Provider value={(id, title) => setAside({ id, title })}>
      <section className="tasks-page" data-row-scope="" aria-live="polite">
        <header className="tasks-top">
          <h2 className="tasks-headline">
            {open === 0 ? o.tasksHeadlineNone : o.tasksHeadline(open)}
          </h2>
          <Summary parts={parts} />
        </header>
        {open === 0 && (
          <div className="panel tasks-clear">
            <SeverityMark level="done" label={r.allClear} />
            <span>{r.allClearNote}</span>
          </div>
        )}
        {!withDuels.some(({ id }) => id === "crash") && <CollisionsCard report={report} />}
        {withDuels.map(({ id, groups, count }) => {
          if (!UNNUMBERED.has(id)) {
            number += 1;
          }
          return (
            <Fragment key={id}>
              <Part
                id={id}
                number={UNNUMBERED.has(id) ? undefined : number}
                count={id === "choose" ? count + duels.length : count}
              >
                {id === "choose" && duels.length > 0 && <DuelsLine duels={duels} />}
                {groups.length > 0 && <GroupList groups={groups} />}
              </Part>
              {id === "crash" && <CollisionsCard report={report} />}
            </Fragment>
          );
        })}
        {aside !== null && (
          <div className="task-undo" role="status">
            <span className="task-undo-text">{r.setAside(aside.title)}</span>
            <button
              type="button"
              className="small"
              onClick={() => {
                setIgnored(aside.id, false);
                setAside(null);
              }}
            >
              {r.undo}
            </button>
          </div>
        )}
      </section>
    </AsideNotice.Provider>
  );
}

/** The parts with an empty choice between mods where the step order puts it. */
function insertChoose(parts: PlannedPart[]): PlannedPart[] {
  const after = new Set<PartId>(["crash", "one-click", "install", "update"]);
  const index = parts.findIndex(({ id }) => !after.has(id));
  const at = index === -1 ? parts.length : index;
  return [...parts.slice(0, at), { id: "choose", groups: [], count: 0 }, ...parts.slice(at)];
}

/** Under the headline: what the things to do do to the game, in words and marks. */
function Summary({ parts }: { parts: PlannedPart[] }) {
  const r = useTaskRowTexts();
  const counts: Record<"crash" | "broken" | "quality" | "scan", number> = {
    crash: 0,
    broken: 0,
    quality: 0,
    scan: 0,
  };
  for (const part of parts) {
    if (UNNUMBERED.has(part.id)) {
      continue;
    }
    for (const group of part.groups) {
      for (const rendered of group.findings) {
        const impact = impactOf(rendered);
        if (
          impact === "crash" ||
          impact === "broken" ||
          impact === "quality" ||
          impact === "scan"
        ) {
          counts[impact] += 1;
        }
      }
    }
  }
  const items = [
    { level: "crash" as const, text: r.summaryCrash(counts.crash) },
    { level: "broken" as const, text: r.summaryBroken(counts.broken) },
    { level: "quality" as const, text: r.summaryQuality(counts.quality) },
    { level: "scan" as const, text: r.summaryScan(counts.scan) },
  ].filter(({ level }) => counts[level] > 0);
  if (items.length === 0) {
    return null;
  }
  return (
    <ul className="tasks-summary">
      {items.map(({ level, text }) => (
        <li key={level} data-level={level}>
          <SeverityMark level={level} label={text} />
          <span aria-hidden="true">{text}</span>
        </li>
      ))}
      {counts.crash === 0 && <li className="tasks-summary-calm">{r.summaryNoCrash}</li>}
    </ul>
  );
}

/** One part of the task page, numbered in the order to work in. */
function Part({
  id,
  number,
  count,
  children,
}: {
  id: PartId;
  number: number | undefined;
  count: number;
  children: ReactNode;
}) {
  const o = useOverviewTexts();
  const r = useTaskRowTexts();
  const folded = FOLDED.has(id);
  const [open, setOpen] = useState(!folded);
  const title = id === "crash" ? r.crashTitle : o.step[id].title;
  return (
    <TaskPart
      id={id}
      number={number}
      title={title}
      count={count}
      tag={id === "advice" ? o.optional : undefined}
      folded={folded}
      open={open}
      onToggle={() => setOpen(!open)}
      toggleLabel={open ? r.fold : o.showFolded(count)}
    >
      {children}
    </TaskPart>
  );
}

/**
 * The collisions in one line: how many spots are left to decide, and the
 * way to the worklist; and what is decided but not in the patch yet.
 */
function CollisionsCard({ report }: { report: RenderedReport }) {
  const { ignored, patchChoices, decideSpot, showPatch } = useSession();
  const o = useOverviewTexts();
  const headingId = useId();
  const spots = useMemo(
    () => reportSpots(report, ignored, patchChoices),
    [report, ignored, patchChoices],
  );
  const states: Record<SpotState, number> = {
    open: 0,
    kept: 0,
    waiting: 0,
    patched: 0,
  };
  for (const spot of spots) {
    states[spot.state] += 1;
  }
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
  const pairs = new Set(
    spots
      .filter((spot) => spot.state === "open")
      .map((spot) => pairOf.get(spot.finding) ?? spot.finding),
  ).size;
  const first = spots.find((spot) => spot.state === "open") ?? spots[0];

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
