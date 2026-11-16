import { useId, useState } from "react";
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
  type Section,
  sectionsFor,
} from "./arrange";
import type { MapSpot, RenderedAction, RenderedFinding } from "./generated/rendered-report";
import type { Texts } from "./i18n";
import { pairFor } from "./patch";

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
export function TaskList({ findings, filter }: { findings: RenderedFinding[]; filter: Filter }) {
  const { t, ignored } = useSession();
  const sections = sectionsFor(findings, filter, ignored);

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
function FindingRow({ rendered, nested = false }: { rendered: RenderedFinding; nested?: boolean }) {
  const { t, mods, spots } = useSession();
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
              {where.length > 0 && <Spots spots={where} />}
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
  const { t, ignored, setIgnored, spots, patchChoices, setPatchChoices, showPatch } = useSession();
  const { finding } = rendered;
  const aside = ignored.has(finding.id);
  const pair = isCollision(rendered)
    ? pairFor(finding.id, rendered.headline, spots.get(finding.id) ?? [])
    : null;
  const chosen = pair === null ? null : (patchChoices[pair.id] ?? null);
  const choose = (plugin: string | null) => {
    if (pair !== null) {
      setPatchChoices({ ...patchChoices, [pair.id]: plugin });
    }
  };

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
        {pair !== null && (
          <div className="choice">
            <div className="choice-text">
              <strong>{t.choiceHide}</strong>
              <p className="choice-note">{t.choiceHideNote(chosen)}</p>
              <fieldset className="segmented" aria-label={t.choiceHide}>
                {pair.sides.map((side) => (
                  <button
                    type="button"
                    key={side.plugin}
                    aria-pressed={chosen === side.plugin}
                    disabled={side.objects.length === 0}
                    onClick={() => choose(side.plugin)}
                  >
                    {t.patchHide(side.plugin, side.objects.length)}
                    {pair.suggested === side.plugin && (
                      <span className="suggested-mark">{t.patchSuggested}</span>
                    )}
                  </button>
                ))}
                <button type="button" aria-pressed={chosen === null} onClick={() => choose(null)}>
                  {t.patchKeepBoth}
                </button>
              </fieldset>
            </div>
            <button type="button" onClick={showPatch}>
              {t.choiceToPatch}
            </button>
          </div>
        )}
        {rendered.actions.length === 0 && pair === null && (
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

/** Where the collisions of a finding are, with the way there in the game. */
function Spots({ spots }: { spots: MapSpot[] }) {
  const { t, showOnMap } = useSession();
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
            {spot.location.type === "exterior" && (
              <button type="button" className="small quiet" onClick={() => showOnMap(spot)}>
                {t.showOnMap}
              </button>
            )}
          </li>
        ))}
      </ul>
    </div>
  );
}
