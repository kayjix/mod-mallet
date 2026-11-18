import { type ReactNode, useId, useMemo, useState } from "react";
import {
  ActionResult,
  type ActionState,
  CopyButton,
  ModCard,
  useAction,
  useSession,
} from "./Actions";
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
  RenderedAction,
  RenderedDuel,
  RenderedFinding,
  RenderedReport,
} from "./generated/rendered-report";
import { reportSpots, type SpotState, weightOf } from "./health";
import type { Texts } from "./i18n";
import { useOverviewTexts } from "./overview-texts";
import { Previews } from "./Preview";
import "./overview.css";

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

/** One finding: a line, and behind it what it means and what to do. */
export function FindingRow({
  rendered,
  nested = false,
}: {
  rendered: RenderedFinding;
  nested?: boolean;
}) {
  const { t, mods, spots } = useSession();
  const o = useOverviewTexts();
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
        {rendered.actions.map((available) => (
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
        <div className={nested ? "body nested" : "body"} id={bodyId}>
          <ActionResult state={action} />
          {open && (
            <>
              <p className="summary">{rendered.summary}</p>
              <Choices rendered={rendered} action={action} />
              {(finding.previews?.length ?? 0) > 0 && <Previews paths={finding.previews ?? []} />}
              {rendered.actions.map((available) => (
                <Guide key={available.label} action={available} />
              ))}
              {known.length > 0 && (
                <div className="mod-cards">
                  {known.map((info) => (
                    <ModCard key={info.mod.id} info={info} />
                  ))}
                </div>
              )}
              {where.length > 0 && <Spots spots={where} collision={isCollision(rendered)} />}
              <details>
                <summary className="more">{t.moreAbout}</summary>
                <h3>{t.whatToDo}</h3>
                <p>{rendered.remedy}</p>
                <h3>{t.whatItMeans}</h3>
                <p>{rendered.explanation}</p>
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
 * What the user can do about a finding, side by side: what Mod Mallet can
 * hand to the mod manager, what goes into the patch, what is left to do by
 * hand, and leaving it as it is. Each with what it does, and one button.
 */
function Choices({ rendered, action }: { rendered: RenderedFinding; action: ActionState }) {
  const { t, ignored, setIgnored, spots, decideSpot, openPair, showToolbox } = useSession();
  const o = useOverviewTexts();
  const { finding } = rendered;
  const aside = ignored.has(finding.id);
  const collision = isCollision(rendered);
  // The first spot to open, or the pair of mods where no spot names its objects.
  const first = collision ? (spots.get(finding.id) ?? [])[0] : undefined;
  const pair = collision ? pairOf(rendered) : null;
  const navmesh = finding.kind.startsWith("navmeshes.") && showToolbox !== undefined;

  return (
    <div className="choices">
      <h3>{t.choicesTitle}</h3>
      <div className="choice-list">
        {rendered.actions.map((available) => (
          <div className="choice" key={available.label}>
            <div className="choice-text">
              <strong>{available.label}</strong>
              <p className="choice-note">{available.description}</p>
            </div>
            <button
              type="button"
              className="primary"
              disabled={action.busy}
              onClick={() => action.run(available.action)}
            >
              {t.choiceRun}
            </button>
          </div>
        ))}
        {collision && (first !== undefined || pair !== null) && (
          <div className="choice">
            <div className="choice-text">
              <strong>{o.worklistChoice}</strong>
              <p className="choice-note">{o.worklistChoiceNote}</p>
            </div>
            <div className="choice-buttons">
              {first !== undefined && (
                <button type="button" className="primary" onClick={() => decideSpot(first)}>
                  {o.worklistOpen}
                </button>
              )}
              {pair !== null && (
                <button type="button" onClick={() => openPair(pair[0], pair[1])}>
                  {o.pairOpen}
                </button>
              )}
            </div>
          </div>
        )}
        {navmesh && (
          <div className="choice">
            <div className="choice-text">
              <strong>{o.navmeshChoice}</strong>
              <p className="choice-note">{o.navmeshChoiceNote}</p>
            </div>
            <button type="button" onClick={showToolbox}>
              {o.toToolbox}
            </button>
          </div>
        )}
        {rendered.actions.length === 0 && !collision && !navmesh && (
          <div className="choice">
            <div className="choice-text">
              <strong>{t.choiceByHand}</strong>
              <p className="choice-note">{rendered.remedy}</p>
            </div>
          </div>
        )}
        <div className="choice quiet">
          <div className="choice-text">
            <strong>{aside ? t.unignore : t.choiceLeave}</strong>
            <p className="choice-note">{aside ? t.choiceBringBackNote : t.choiceLeaveNote}</p>
          </div>
          <button type="button" onClick={() => setIgnored(finding.id, !aside)}>
            {aside ? t.unignore : t.ignore}
          </button>
        </div>
      </div>
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
  const { title, hint } = o.step[step];
  return (
    <section className="tasks step" data-step={step}>
      <header>
        <h2>
          {title} {count > 0 && <span className="count">{count}</span>}
        </h2>
        {step === "advice" && <span className="tag optional-tag">{o.optional}</span>}
        <span className="tasks-hint">{hint}</span>
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
