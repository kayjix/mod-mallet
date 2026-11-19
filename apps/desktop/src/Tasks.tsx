import { type ReactNode, useId, useMemo, useState } from "react";
import { ActionResult, type ActionState, CopyButton, useAction, useSession } from "./Actions";
import {
  type Filter,
  type Group,
  isCollision,
  isDirect,
  modsOf,
  openTaskCount,
  type Section,
  type Step,
  sectionsFor,
  stepsFor,
} from "./arrange";
import type {
  MapSpot,
  ModInfo,
  RenderedAction,
  RenderedDuel,
  RenderedFinding,
  RenderedReport,
} from "./generated/rendered-report";
import { reportSpots, type SpotState, weightOf } from "./health";
import type { Texts } from "./i18n";
import { useOverviewTexts } from "./overview-texts";
import { Previews } from "./Preview";
import { useTaskRowTexts } from "./task-row-texts";
import "./overview.css";
import "./tasks.css";

/** A group with more findings than this starts folded. */
const OPEN_GROUP_LIMIT = 4;

/** How many spots of a finding are listed. The map shows all of them. */
const SPOT_LIMIT = 8;

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
 * The findings as a list of things to do: most urgent first, one line
 * each, with the button that does something about it where there is one.
 * Everything else about a finding is behind its line.
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
    <>
      {sections.map((section) => {
        const { title, hint } = sectionTexts(section.id, t);
        return (
          <section className="tasks" key={section.id} data-section={section.id}>
            <header title={hint}>
              <h2>
                {title} <span className="count">{section.count}</span>
              </h2>
            </header>
            <ul>
              {section.groups.map((group) =>
                group.findings.length === 1 ? (
                  group.findings.map((rendered) => (
                    <FindingRow key={rendered.finding.id} rendered={rendered} />
                  ))
                ) : (
                  <GroupRow key={group.kind} group={group} />
                ),
              )}
            </ul>
          </section>
        );
      })}
    </>
  );
}

/** Several findings of one kind behind one line. */
function GroupRow({ group }: { group: Group }) {
  const startsOpen = group.findings.some(isDirect) || group.findings.length <= OPEN_GROUP_LIMIT;
  const [open, setOpen] = useState(startsOpen);
  const listId = useId();
  const severity = group.findings[0]?.finding.severity;

  return (
    <li className="group" data-severity={severity}>
      <button
        type="button"
        className="line"
        aria-expanded={open}
        aria-controls={listId}
        onClick={() => setOpen(!open)}
      >
        <span className="chevron" aria-hidden="true" />
        <span className="line-title">{group.title}</span>
        <span className="count">{group.findings.length}</span>
      </button>
      {open && (
        <ul id={listId}>
          {group.findings.map((rendered) => (
            <FindingRow key={rendered.finding.id} rendered={rendered} nested />
          ))}
        </ul>
      )}
    </li>
  );
}

/**
 * One finding: a line, and behind it what it is, one row of buttons with
 * the ways out, and everything else folded under "more about it".
 */
export function FindingRow({
  rendered,
  nested = false,
}: {
  rendered: RenderedFinding;
  nested?: boolean;
}) {
  const { t, mods, spots } = useSession();
  const o = useOverviewTexts();
  const r = useTaskRowTexts();
  const [open, setOpen] = useState(false);
  const action = useAction();
  const bodyId = useId();
  const { finding } = rendered;
  // Whatever an action answers is shown, also when the line is folded.
  const showBody = open || action.touched;
  const known = modsOf(rendered).flatMap((name) => {
    const info = mods.get(name);
    return info === undefined ? [] : [info];
  });
  const where = spots.get(finding.id) ?? [];
  // The mod a finding is about, on its line, unless the line names it.
  const provider = finding.provider?.name;
  const showProvider = provider != null && !rendered.headline.includes(provider);
  const crash = weightOf(rendered).impact === "crash";
  const ways = useWays(rendered, known);

  return (
    <li className="finding" data-severity={finding.severity}>
      <div className="line-with-action">
        <button
          type="button"
          className={nested ? "line nested" : "line"}
          aria-expanded={showBody}
          aria-controls={bodyId}
          onClick={() => setOpen(!open)}
        >
          <span className="chevron" aria-hidden="true" />
          <span className="line-title">{rendered.headline}</span>
          {crash && (
            <span className="impact-chip" data-impact="crash">
              {o.crashTag}
            </span>
          )}
          {showProvider && <span className="line-mod">{provider}</span>}
        </button>
        {/* Opened, the buttons move under the text, next to what they are about. */}
        {!open &&
          rendered.actions.map((available) => (
            <button
              type="button"
              className="act"
              key={available.label}
              title={available.description}
              disabled={action.busy}
              onClick={() => action.run(available.action)}
            >
              {available.label}
            </button>
          ))}
      </div>
      {showBody && (
        <div className={nested ? "body task-body nested" : "body task-body"} id={bodyId}>
          <ActionResult state={action} />
          {open && (
            <>
              <p className="summary">{rendered.summary}</p>
              {ways.length === 0 && rendered.actions.length === 0 && (
                <p className="remedy">{rendered.remedy}</p>
              )}
              <Ways rendered={rendered} action={action} ways={ways} />
              {(finding.previews?.length ?? 0) > 0 && <Previews paths={finding.previews ?? []} />}
              {rendered.actions.map((available) => (
                <Guide key={available.label} action={available} />
              ))}
              <details>
                <summary className="more">{t.moreAbout}</summary>
                {(ways.length > 0 || rendered.actions.length > 0) && (
                  <>
                    <h3>{t.whatToDo}</h3>
                    <p>{rendered.remedy}</p>
                  </>
                )}
                <h3>{t.whatItMeans}</h3>
                <p>{rendered.explanation}</p>
                {where.length > 0 && <Spots spots={where} collision={isCollision(rendered)} />}
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
                <p className="meta">
                  {rendered.severityLabel} · {rendered.confidenceLabel}
                  {finding.subject.type === "file" && (
                    <>
                      {` · ${t.file}: `}
                      <code>{finding.subject.path}</code>
                    </>
                  )}
                </p>
              </details>
            </>
          )}
        </div>
      )}
    </li>
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
    ways.push({ key: "pair", label: o.pairOpen, run: () => openPair(mod, other), primary: true });
  }
  if (finding.kind === "mods.collides-widely" && typeof mod === "string") {
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
    ways.push({ key: "toolbox", label: o.navmeshChoice, run: showToolbox, primary: false });
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
    ways.push({ key: "map", label: t.showOnMap, run: () => showOnMap(onMap), primary: false });
  }
  // Where Mod Mallet hands nothing to the mod manager, the first way is the main one.
  const first = ways[0];
  if (rendered.actions.length === 0 && first !== undefined && !ways.some((way) => way.primary)) {
    first.primary = true;
  }
  return ways;
}

/**
 * The buttons under an opened finding, in one row: what Mod Mallet can hand
 * to the mod manager, then the other ways out, then leaving it as it is.
 */
function Ways({
  rendered,
  action,
  ways,
}: {
  rendered: RenderedFinding;
  action: ActionState;
  ways: Way[];
}) {
  const { t, ignored, setIgnored } = useSession();
  const { finding } = rendered;
  const aside = ignored.has(finding.id);

  return (
    <div className="ways">
      {rendered.actions.map((available) => (
        <button
          type="button"
          className="primary"
          key={available.label}
          title={available.description}
          disabled={action.busy}
          onClick={() => action.run(available.action)}
        >
          {available.label}
        </button>
      ))}
      {ways.map((way) => (
        <button
          type="button"
          key={way.key}
          className={way.primary ? "primary" : undefined}
          onClick={way.run}
        >
          {way.label}
        </button>
      ))}
      <button type="button" className="quiet" onClick={() => setIgnored(finding.id, !aside)}>
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

/** The steps that are only to know or optional; their findings start folded. */
const FOLDED: Step[] = ["notes", "ignored"];

/**
 * The task page: what there is to do, grouped by what the user does about
 * it. The collisions have their own page and are summed up in one line;
 * recommendations and notes come after everything that is required.
 */
export function TasksPage({ report }: { report: RenderedReport }) {
  const { ignored } = useSession();
  const o = useOverviewTexts();
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
  const steps = useMemo(
    () =>
      stepsFor(
        report.findings.filter(
          (rendered) => ignored.has(rendered.finding.id) || !inDuels.has(rendered.finding.id),
        ),
        ignored,
      ),
    [report.findings, ignored, inDuels],
  );
  const open = openTaskCount(report.findings, ignored);
  const hasAdvice = steps.some(({ step }) => step === "advice");
  const hasChoose = steps.some(({ step }) => step === "choose");

  return (
    <section className="tasks-page" aria-live="polite">
      <h2 className="tasks-headline">{open === 0 ? o.tasksHeadlineNone : o.tasksHeadline(open)}</h2>
      <CollisionsCard report={report} />

      {steps.map(({ step, groups, count }) => (
        <StepSection
          key={step}
          step={step}
          count={step === "choose" ? count + duels.length : count}
          folded={FOLDED.includes(step)}
        >
          {step === "choose" && duels.length > 0 && <DuelsLine duels={duels} />}
          {groups.length > 0 && <GroupList groups={groups} />}
        </StepSection>
      ))}
      {!hasChoose && duels.length > 0 && (
        <StepSection step="choose" count={duels.length} folded={false}>
          <DuelsLine duels={duels} />
        </StepSection>
      )}
      {!hasAdvice && (
        <StepSection step="advice" count={0} folded={false}>
          <p className="aside step-empty">{o.adviceEmpty}</p>
        </StepSection>
      )}
    </section>
  );
}

/** One step of the task page: what to do, why, and the findings it is about. */
function StepSection({
  step,
  count,
  folded,
  children,
}: {
  step: Step;
  count: number;
  folded: boolean;
  children: ReactNode;
}) {
  const o = useOverviewTexts();
  const [open, setOpen] = useState(!folded);
  // The title says what to do; no line under it explains it again.
  const { title } = o.step[step];
  return (
    <section className="tasks step" data-step={step}>
      <header>
        <h2>
          {title} {count > 0 && <span className="count">{count}</span>}
        </h2>
        {step === "advice" && <span className="tag optional-tag">{o.optional}</span>}
        <span className="spacer" />
        {folded && (
          <button
            type="button"
            className="small quiet"
            aria-expanded={open}
            onClick={() => setOpen(!open)}
          >
            {open ? "–" : o.showFolded(count)}
          </button>
        )}
      </header>
      {open && children}
    </section>
  );
}

/** The groups of a step, one line per finding or per kind. */
function GroupList({ groups }: { groups: Group[] }) {
  return (
    <ul>
      {groups.map((group) =>
        group.findings.length === 1 ? (
          group.findings.map((rendered) => (
            <FindingRow key={rendered.finding.id} rendered={rendered} />
          ))
        ) : (
          <GroupRow key={group.kind} group={group} />
        ),
      )}
    </ul>
  );
}

/**
 * The collisions in one line: how many spots are left to decide, and the
 * way to the worklist; and what is decided but not in the patch yet.
 */
function CollisionsCard({ report }: { report: RenderedReport }) {
  const { ignored, patchChoices, decideSpot, showPatch } = useSession();
  const o = useOverviewTexts();
  const spots = useMemo(
    () => reportSpots(report, ignored, patchChoices),
    [report, ignored, patchChoices],
  );
  const states: Record<SpotState, number> = { open: 0, kept: 0, waiting: 0, patched: 0 };
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
    <section className="panel collisions-card" data-open={states.open > 0}>
      <div className="collisions-card-text">
        <strong>{o.collisionsTitle}</strong>
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
          className={states.open > 0 ? "primary" : undefined}
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
    <section className="panel collisions-card duels-line" data-open={open > 0}>
      <div className="collisions-card-text">
        <span>
          {open === 0 ? o.duelsAllDecided(duels.length) : o.duelsLine(open, duels.length)}
        </span>
      </div>
      <button
        type="button"
        className={open > 0 ? "primary" : undefined}
        onClick={() => openCompare(null)}
      >
        {o.toAssets}
      </button>
    </section>
  );
}
