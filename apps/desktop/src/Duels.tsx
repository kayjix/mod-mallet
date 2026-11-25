import { useMemo, useState } from "react";
import { type Session, useSession } from "./Actions";
import { useAssetsTexts } from "./assets-texts";
import type {
  DuelSide,
  MapSpot,
  ModDuel,
  RenderedDuel,
  RenderedFinding,
} from "./generated/rendered-report";
import "./assets.css";
import { chooseFinding, decisionAt, type PatchChoices, pairFor, spotKey } from "./patch";
import { PreferenceMark } from "./preferences-ui";

/**
 * The side the collisions' worklist lets win at every spot of a duel (the
 * other side's objects hidden everywhere), if it does (ADR 0040).
 */
function collisionWinner(
  duel: ModDuel,
  spots: Map<string, MapSpot[]>,
  choices: PatchChoices,
): 0 | 1 | null {
  const losers = new Set<string>();
  for (const id of duel.findings) {
    for (const spot of spots.get(id) ?? []) {
      const key = spotKey(spot);
      if (key === null) {
        continue;
      }
      const decided = decisionAt(choices, id, key, spot.parts ?? []);
      if (decided === undefined || decided === null) {
        return null;
      }
      losers.add(decided.toLowerCase());
    }
  }
  const [loser] = [...losers];
  if (losers.size !== 1 || loser === undefined) {
    return null;
  }
  const index = duel.sides.findIndex((side) =>
    (side.plugins ?? []).some((plugin) => plugin.toLowerCase() === loser),
  );
  return index === 0 ? 1 : index === 1 ? 0 : null;
}

/** How many open duels are shown at first; the largest come first. */
const OPEN_LIMIT = 8;

/** Where the mods the user did not place stand in the ranking. */
export const OTHERS = "*";

/** The key a side has in the ranking. */
export function rankKey(side: DuelSide): string {
  return side.mod?.id ?? side.name;
}

/** The name the mod manager shows for a side. */
export function sideName(side: DuelSide, mods: Session["mods"]): string {
  const id = side.mod?.id;
  return (
    (id === undefined
      ? undefined
      : [...mods.values()].find((info) => info.mod.id === id)?.mod.name) ?? side.name
  );
}

/** Both names of a duel. */
export function duelNames(duel: ModDuel, mods: Session["mods"]): readonly [string, string] {
  return [sideName(duel.sides[0], mods), sideName(duel.sides[1], mods)];
}

/**
 * The side the ranking makes win, if it decides: the one higher in the
 * list. Mods not in it stand where the marker for the others is, or
 * below all that are when there is no marker.
 */
export function rankedSide(duel: ModDuel, ranking: string[]): 0 | 1 | null {
  const others = ranking.includes(OTHERS) ? ranking.indexOf(OTHERS) : ranking.length;
  const place = (side: DuelSide) => {
    const index = ranking.indexOf(rankKey(side));
    return index < 0 ? others : index;
  };
  const [a, b] = [place(duel.sides[0]), place(duel.sides[1])];
  if (a === b) {
    return null;
  }
  return a < b ? 0 : 1;
}

/** What picking a side did. */
export interface Applied {
  hidden: number;
  messages: string[];
  choices: PatchChoices;
  /** How many actions the mod manager did not carry out. */
  failed: number;
  /** Whether the mod manager cannot be reached, so that trying more is no use. */
  unreachable: boolean;
}

/** Outcomes after which no further action can succeed either. */
const UNREACHABLE = new Set(["link-missing", "manager-not-running", "no-answer", "wrong-game"]);
/** Outcomes that mean the action was carried out or is under way. */
const DONE = new Set(["started", "installing", "applied"]);

/**
 * Makes one side of a duel win: the other side's objects go into the
 * patch, everything else is asked of the mod manager. Changes nothing
 * itself; the caller keeps the choices and the decision.
 */
export async function resolveDuel(
  session: Session,
  duel: ModDuel,
  byId: Map<string, RenderedFinding>,
  side: 0 | 1,
  choices: PatchChoices,
): Promise<Applied> {
  const { t, api, lang, spots } = session;
  const messages: string[] = [];
  const loser = duel.sides[side === 0 ? 1 : 0];
  let next = { ...choices };
  let hidden = 0;
  let failed = 0;
  let unreachable = false;
  for (const id of duel.findings) {
    const finding = byId.get(id);
    if (finding === undefined) {
      continue;
    }
    const pair = pairFor(id, finding.headline, spots.get(id) ?? []);
    const target = pair?.sides.find((candidate) =>
      (loser.plugins ?? []).some(
        (plugin) => plugin.toLowerCase() === candidate.plugin.toLowerCase(),
      ),
    );
    if (pair !== null && target !== undefined) {
      next = chooseFinding(next, pair, target.plugin);
      hidden += target.objects.length;
    }
  }
  if (hidden > 0) {
    messages.push(t.duelPatchChosen(hidden));
  }
  for (const resolution of duel.resolutions.filter((candidate) => candidate.side === side)) {
    try {
      const outcome = await api.runAction(resolution.action, lang);
      messages.push(outcome.message);
      if (!DONE.has(outcome.outcome.status)) {
        failed += 1;
      }
      if (UNREACHABLE.has(outcome.outcome.status)) {
        unreachable = true;
        break;
      }
    } catch (error: unknown) {
      messages.push(String(error));
      failed += 1;
    }
  }
  return { hidden, messages: [...new Set(messages)], choices: next, failed, unreachable };
}

/** Picking a side of one duel, with what became of it. */
export function usePick(duel: ModDuel, byId: Map<string, RenderedFinding>, onDecided?: () => void) {
  const session = useSession();
  const [busy, setBusy] = useState(false);
  const [applied, setApplied] = useState<Applied | null>(null);
  const pick = async (side: 0 | 1) => {
    setBusy(true);
    const result = await resolveDuel(session, duel, byId, side, session.patchChoices);
    if (result.hidden > 0) {
      session.setPatchChoices(result.choices);
    }
    onDecided?.();
    session.decide(duel.id, side);
    setApplied(result);
    setBusy(false);
  };
  return { busy, applied, pick };
}

/** The findings of a report by identity. */
export function useFindings(findings: RenderedFinding[]): Map<string, RenderedFinding> {
  return useMemo(
    () => new Map(findings.map((rendered) => [rendered.finding.id, rendered])),
    [findings],
  );
}

interface DuelsProps {
  duels: RenderedDuel[];
  findings: RenderedFinding[];
  /** Only duels about these topics, if given. */
  topics?: string[];
}

/**
 * Everything two mods disagree about, one card per pair: what they fight
 * over, who wins now, which side Mod Mallet would pick and why, and one
 * button per side that makes it win everywhere at once.
 */
export function Duels({ duels, findings, topics }: DuelsProps) {
  const { t, decisions, openCompare } = useSession();
  // Duels decided while this list is shown stay where they are, with what
  // became of the decision.
  const [recent, setRecent] = useState<Set<string>>(new Set());
  const [limit, setLimit] = useState(OPEN_LIMIT);
  const shown = duels.filter(
    (rendered) =>
      topics === undefined || rendered.duel.topics.some((topic) => topics.includes(topic.topic)),
  );
  const settled = (id: string) => decisions[id] !== undefined && !recent.has(id);
  const open = shown.filter((rendered) => !settled(rendered.duel.id));
  const decided = shown.filter((rendered) => settled(rendered.duel.id));
  const byId = useFindings(findings);

  if (shown.length === 0) {
    return null;
  }
  return (
    <section className="duels">
      <header className="duels-head">
        <h2>
          {t.duelsTitle} <span className="count">{open.length}</span>
        </h2>
        <p className="aside">{t.duelsHint}</p>
        <span className="spacer" />
        <button
          type="button"
          className="primary"
          onClick={() => openCompare(open[0]?.duel.id ?? null)}
        >
          {t.compareOpen}
        </button>
      </header>
      <div className="duel-list">
        {open.slice(0, limit).map((rendered) => (
          <DuelCard
            key={rendered.duel.id}
            rendered={rendered}
            byId={byId}
            onDecided={(id) => setRecent((known) => new Set(known).add(id))}
          />
        ))}
      </div>
      {open.length > limit && (
        <button type="button" className="quiet" onClick={() => setLimit(limit + OPEN_LIMIT)}>
          {t.duelsMore(open.length - limit)}
        </button>
      )}
      {decided.length > 0 && (
        <details className="duels-decided">
          <summary className="more">{t.duelsDecided(decided.length)}</summary>
          <div className="duel-list">
            {decided.map((rendered) => (
              <DuelCard
                key={rendered.duel.id}
                rendered={rendered}
                byId={byId}
                onDecided={(id) => setRecent((known) => new Set(known).add(id))}
              />
            ))}
          </div>
        </details>
      )}
    </section>
  );
}

/**
 * One duel about files in one line, for the pages that only lead to the
 * assets page (ADR 0048): the two mods, where it stands, and the button that
 * opens it there.
 */
export function DuelLine({ rendered }: { rendered: RenderedDuel }) {
  const { t, decisions, mods, ranking, openCompare } = useSession();
  const a = useAssetsTexts();
  const { duel } = rendered;
  const names = duelNames(duel, mods);
  const decided = decisions[duel.id];
  const suggested = rankedSide(duel, ranking) ?? duel.suggestion?.side ?? null;
  const files = duel.files?.length ?? 0;
  const state =
    decided === 0 || decided === 1
      ? a.lineDecided(names[decided])
      : suggested === 0 || suggested === 1
        ? a.lineSuggested(names[suggested])
        : a.lineOpen;
  return (
    <div className="duel-line" data-decided={decided !== undefined}>
      <span className="duel-line-names">
        {names[0]} <span aria-hidden="true">⚔</span> {names[1]}
      </span>
      <span className="aside duel-line-topics">{rendered.topicLabels.join(" · ")}</span>
      <span className="duel-line-state">{state}</span>
      <button type="button" className="small quiet" onClick={() => openCompare(duel.id)}>
        {files > 0 ? t.duelCompareFiles(files) : t.duelCompare} ▸
      </button>
    </div>
  );
}

function DuelCard({
  rendered,
  byId,
  onDecided,
}: {
  rendered: RenderedDuel;
  byId: Map<string, RenderedFinding>;
  onDecided: (id: string) => void;
}) {
  const { t, decisions, decide, showPatch, mods, ranking, openCompare, spots, patchChoices } =
    useSession();
  const { duel } = rendered;
  const fromCollisions = collisionWinner(duel, spots, patchChoices);
  const { busy, applied, pick } = usePick(duel, byId, () => onDecided(duel.id));
  const decided = decisions[duel.id];
  const names = duelNames(duel, mods);
  const suggested = duel.suggestion?.side;
  const ranked = rankedSide(duel, ranking);
  const files = duel.files?.length ?? 0;

  return (
    <article className="duel" data-decided={decided !== undefined}>
      <div className="duel-main">
        <div className="duel-sides">
          {([0, 1] as const).map((index) => (
            <div className="duel-side" key={index} data-suggested={suggested === index}>
              <strong>{names[index]}</strong>
              {suggested === index && <span className="suggested-mark">{t.patchSuggested}</span>}
              {ranked === index && <span className="ranked-mark">{t.rankedMark}</span>}
              <PreferenceMark duel={duel.id} side={index} />
              {decided === index && <span className="duel-winner">{t.duelWinner}</span>}
            </div>
          ))}
          <span className="duel-versus" aria-hidden="true">
            ⚔
          </span>
        </div>
        <ul className="duel-topics">
          {duel.topics.map((topic, index) => (
            <li className="duel-topic" key={topic.topic}>
              {rendered.topicLabels[index]}
              {topic.winning != null && (
                <span className="aside"> · {t.duelNow(names[topic.winning === 0 ? 0 : 1])}</span>
              )}
            </li>
          ))}
        </ul>
        {rendered.suggestion != null && <p className="duel-why">{rendered.suggestion}</p>}
        {decided === undefined && fromCollisions !== null && (
          <p className="duel-why duel-from-collisions">
            {t.duelFromCollisions(names[fromCollisions])}{" "}
            <button
              type="button"
              className="small primary"
              disabled={busy}
              onClick={() => pick(fromCollisions)}
            >
              {t.duelTakeOver}
            </button>
          </p>
        )}
        <div className="duel-actions">
          {([0, 1] as const).map((index) => (
            <button
              type="button"
              key={index}
              className={suggested === index ? "primary" : undefined}
              disabled={busy}
              onClick={() => pick(index)}
            >
              {t.duelPick(names[index])}
            </button>
          ))}
          <button type="button" className="quiet" onClick={() => openCompare(duel.id)}>
            {files > 0 ? t.duelCompareFiles(files) : t.duelCompare}
          </button>
          {decided !== undefined && (
            <button type="button" className="quiet" onClick={() => decide(duel.id, null)}>
              {t.duelUndo}
            </button>
          )}
        </div>
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
    </article>
  );
}
