// Preferences between mods in the window (ADR 0086): what is learned from
// the decisions, what the user said, and applying a preference to the
// open conflicts with one undo. Every page that shows a conflict asks
// `usePreferences()` whether a preference covers it; the offer and the
// settings section use the same.

import { useCallback, useEffect, useMemo, useRef, useState, useSyncExternalStore } from "react";
import { useSession } from "./Actions";
import { activePatch } from "./book";
import { rankedSide, resolveDuel } from "./Duels";
import { chooseDuel } from "./decisions";
import type { PatchPreferences, PreferenceRule } from "./generated/preferences";
import type { MapSpot, RenderedReport } from "./generated/rendered-report";
import { spotKey } from "./patch";
import { PreferenceOffer } from "./preference-offer";
import { type PreferenceTexts, usePreferenceTexts } from "./preference-texts";
import {
  type Conflict,
  changedKeys,
  conflictsOf,
  decideSpots,
  explicitId,
  historyOf,
  type Learned,
  learn,
  NO_PREFERENCES,
  namesOf,
  type Offer,
  type OpenCheck,
  offersOf,
  type PlanItem,
  type PreferenceWorld,
  type Preferred,
  planOf,
  preferredSide,
  type Rule,
  restoreKeys,
  rulesOf,
  votesOf,
} from "./preferences";
import { Chip } from "./ui/kit";
import { walkedOn } from "./walkways";
import "./preferences.css";

/** What became of applying a preference, with the way back. */
export interface Batch {
  rule: string;
  /** The choices before, and the keys the batch changed. */
  before: Record<string, string | null>;
  keys: string[];
  decided: number;
  walked: number;
  failed: number;
  stopped: string | null;
  /** Whether it was taken back. */
  undone: boolean;
}

/** What every page asks about preferences. */
export interface Preferences {
  /** What the user said; nothing until read. */
  kept: PatchPreferences;
  learned: Learned[];
  rules: Rule[];
  /** The preferences worth offering now, strongest first. */
  offers: Offer[];
  /** What preferences say about a duel, by its identity. */
  forDuel: (id: string) => Preferred | null;
  /** What preferences say about a spot; the plugin kept, if any. */
  forSpot: (spot: MapSpot) => (Preferred & { plugin: string }) | null;
  /** The preference that made a decision, by the decision's key. */
  madeBy: (key: string) => string | null;
  /** The open conflicts a preference would decide. */
  planFor: (rule: Rule) => PlanItem[];
  /** A preference in words. */
  describe: (rule: Pick<Rule, "mod" | "over" | "topic">) => string;
  /** Keeps a new state of what the user said. */
  save: (next: PatchPreferences) => void;
  /** Decides the items of a preference, as one batch that one undo takes back. */
  apply: (rule: Rule, items: PlanItem[]) => Promise<void>;
  /** The last batch, while it is shown. */
  batch: Batch | null;
  /** How far applying is, while it runs. */
  progress: { done: number; total: number } | null;
  undo: () => void;
  forgetBatch: () => void;
  /** Answers a preference offered. */
  answer: (learned: Learned, answer: "later" | "never") => void;
  /** Uses a preference turned down again. */
  again: (id: string) => void;
  addRule: (rule: PreferenceRule) => void;
  removeRule: (rule: PreferenceRule) => void;
  setLearning: (on: boolean) => void;
}

// The preferences of the window, as the host publishes them: one window,
// one host, so that the host sits beside the pages instead of around them.
let published: Preferences | null = null;
const listeners = new Set<() => void>();

function publish(next: Preferences | null) {
  published = next;
  for (const listener of listeners) {
    listener();
  }
}

function subscribe(listener: () => void) {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

/** The preferences, where the window offers them; null elsewhere. */
export function usePreferences(): Preferences | null {
  return useSyncExternalStore(subscribe, () => published);
}

/** A preference in words. */
export function describeRule(
  p: PreferenceTexts,
  rule: Pick<Rule, "mod" | "over" | "topic">,
): string {
  if (rule.over !== null && rule.topic !== null) {
    return p.pairFor(rule.mod, rule.over, p.topic[rule.topic]);
  }
  if (rule.over !== null) {
    return p.pair(rule.mod, rule.over);
  }
  if (rule.topic !== null) {
    return p.favourite(rule.mod, p.topic[rule.topic]);
  }
  return p.always(rule.mod);
}

/**
 * Learns the preferences from the user's decisions and offers them to
 * every page through {@link usePreferences}. Sits beside the pages; shows
 * the offer itself, in a corner.
 */
export function PreferencesHost({ report }: { report: RenderedReport | null }) {
  const session = useSession();
  const { api, game, book, decided, ranking, setPatchChoices } = session;
  const p = usePreferenceTexts();
  const [kept, setKept] = useState<PatchPreferences>(NO_PREFERENCES);
  // biome-ignore lint/correctness/useExhaustiveDependencies: read again for every game.
  useEffect(() => {
    let outdated = false;
    setKept(NO_PREFERENCES);
    api
      .preferences()
      .then((found) => {
        if (!outdated) {
          setKept(found ?? NO_PREFERENCES);
        }
      })
      .catch(() => {});
    return () => {
      outdated = true;
    };
  }, [api, game]);
  const keptRef = useRef(kept);
  keptRef.current = kept;
  const save = useCallback(
    (next: PatchPreferences) => {
      keptRef.current = next;
      setKept(next);
      api.setPreferences(next).catch(() => {});
    },
    [api],
  );

  const world = useMemo<PreferenceWorld | null>(
    () =>
      report === null
        ? null
        : {
            duels: report.duels.map((rendered) => rendered.duel),
            spots: decided.spots,
            passages: report.map.passages ?? [],
            ...namesOf(report),
            ranked: (duel) => rankedSide(duel, ranking),
          },
    [report, decided.spots, ranking],
  );
  const learned = useMemo(() => {
    if (world === null || book === null) {
      return [];
    }
    const history = historyOf(book.patches, activePatch(book)?.id ?? null);
    return learn(votesOf(history, world, new Set(Object.keys(kept.applied ?? {}))));
  }, [world, book, kept.applied]);
  const rules = useMemo(() => rulesOf(learned, kept), [learned, kept]);
  const conflicts = useMemo(() => (world === null ? [] : conflictsOf(world)), [world]);
  const byId = useMemo(
    () => new Map(conflicts.map((conflict) => [conflict.id, conflict])),
    [conflicts],
  );
  const open = useMemo<OpenCheck>(
    () => ({
      duelOpen: (id) => decided.duels[id] === undefined,
      spotOpen: (spot) => decided.decisionOf(spot) === undefined,
    }),
    [decided],
  );
  const offers = useMemo(
    () => offersOf(learned, kept, rules, conflicts, open),
    [learned, kept, rules, conflicts, open],
  );

  const preferred = useCallback(
    (conflict: Conflict | undefined) =>
      conflict === undefined ? null : preferredSide(rules, conflict),
    [rules],
  );

  const choicesRef = useRef(decided.choices);
  choicesRef.current = decided.choices;
  const [batch, setBatch] = useState<Batch | null>(null);
  const [progress, setProgress] = useState<{ done: number; total: number } | null>(null);
  const findings = useMemo(
    () => new Map((report?.findings ?? []).map((rendered) => [rendered.finding.id, rendered])),
    [report],
  );

  const apply = useCallback(
    async (rule: Rule, items: PlanItem[]) => {
      const before = choicesRef.current;
      let next = before;
      // Spots: where characters walk on what would go, the place is the
      // user's to decide (ADR 0051), as everywhere.
      const spots = items.filter((item) => item.conflict.type === "spot");
      const taken: PlanItem[] = [];
      let walked = 0;
      setProgress({ done: 0, total: items.length });
      for (const [index, item] of spots.entries()) {
        if (item.conflict.type !== "spot" || item.plugin === null) {
          continue;
        }
        const { spot } = item.conflict;
        const going = (item.plugin ?? "").toLowerCase();
        const losing = (spot.parts ?? []).filter(
          (part) => part.hideable && part.object.plugin.toLowerCase() === going,
        );
        const inside = spot.location.type === "interior";
        const place = spot.location.type === "interior" ? spot.location.cell : spot.location.world;
        const centre: [number, number] | null =
          inside || spot.position == null ? null : [spot.position[0], spot.position[1]];
        const paths =
          losing.length === 0
            ? new Map<string, number>()
            : await walkedOn(
                api,
                place,
                centre,
                losing.map((part) => ({ id: part.object, position: spot.position })),
              ).catch(() => new Map<string, number>());
        if (paths.size > 0) {
          walked += 1;
        } else {
          taken.push(item);
        }
        setProgress({ done: index + 1, total: items.length });
      }
      next = decideSpots(next, taken);
      // Duels: picked as by hand, the mod manager asked for each.
      let decidedDuels = 0;
      let failed = 0;
      let stopped: string | null = null;
      for (const [index, item] of items.entries()) {
        if (item.conflict.type !== "duel") {
          continue;
        }
        const applied = await resolveDuel(session, item.conflict.duel, findings, item.side, next);
        failed += applied.failed;
        if (applied.unreachable) {
          stopped = applied.messages[applied.messages.length - 1] ?? "";
          break;
        }
        next = chooseDuel(applied.choices, item.conflict.id, item.side);
        decidedDuels += 1;
        setProgress({ done: spots.length + index + 1, total: items.length });
      }
      const keys = changedKeys(before, next);
      // What the user changed meanwhile stays; only the batch's keys change.
      if (keys.length > 0) {
        setPatchChoices(restoreKeys(choicesRef.current, next, keys));
      }
      const now = keptRef.current;
      const learnedOne = learned.find((entry) => entry.id === rule.id);
      save({
        ...now,
        applied: {
          ...(now.applied ?? {}),
          ...Object.fromEntries(keys.map((key) => [key, rule.id])),
        },
        answers:
          learnedOne === undefined
            ? now.answers
            : { ...(now.answers ?? {}), [rule.id]: { answer: "applied", at: learnedOne.of } },
      });
      setProgress(null);
      setBatch({
        rule: rule.id,
        before,
        keys,
        decided: taken.length + decidedDuels,
        walked,
        failed,
        stopped,
        undone: false,
      });
    },
    [api, session, findings, learned, save, setPatchChoices],
  );

  const undo = useCallback(() => {
    if (batch === null || batch.undone) {
      return;
    }
    setPatchChoices(restoreKeys(choicesRef.current, batch.before, batch.keys));
    const now = keptRef.current;
    const applied = { ...(now.applied ?? {}) };
    for (const key of batch.keys) {
      delete applied[key];
    }
    save({ ...now, applied });
    setBatch({ ...batch, undone: true });
  }, [batch, save, setPatchChoices]);

  const value = useMemo<Preferences>(
    () => ({
      kept,
      learned,
      rules,
      offers,
      forDuel: (id) => preferred(byId.get(id)),
      forSpot: (spot) => {
        const key = spotKey(spot);
        const conflict = key === null ? undefined : byId.get(key);
        const found = preferred(conflict);
        return found === null || conflict?.type !== "spot"
          ? null
          : { ...found, plugin: conflict.plugins[found.side] };
      },
      madeBy: (key) => kept.applied?.[key] ?? null,
      planFor: (rule) => planOf(rule, rules, conflicts, open),
      describe: (rule) => describeRule(p, rule),
      save,
      apply,
      batch,
      progress,
      undo,
      forgetBatch: () => setBatch(null),
      answer: (entry, answer) =>
        save({
          ...keptRef.current,
          answers: { ...(keptRef.current.answers ?? {}), [entry.id]: { answer, at: entry.of } },
        }),
      again: (id) => {
        const answers = { ...(keptRef.current.answers ?? {}) };
        delete answers[id];
        save({ ...keptRef.current, answers });
      },
      addRule: (rule) => {
        const rulesNow = keptRef.current.rules ?? [];
        if (!rulesNow.some((known) => explicitId(known) === explicitId(rule))) {
          save({ ...keptRef.current, rules: [...rulesNow, rule] });
        }
      },
      removeRule: (rule) =>
        save({
          ...keptRef.current,
          rules: (keptRef.current.rules ?? []).filter(
            (known) => explicitId(known) !== explicitId(rule),
          ),
        }),
      setLearning: (on) => save({ ...keptRef.current, learningOff: !on }),
    }),
    [
      kept,
      learned,
      rules,
      offers,
      preferred,
      byId,
      conflicts,
      open,
      p,
      save,
      apply,
      batch,
      progress,
      undo,
    ],
  );

  useEffect(() => publish(value), [value]);
  useEffect(() => () => publish(null), []);

  return report === null ? null : <PreferenceOffer />;
}

/**
 * The mark on a conflict a preference covers: "suggested: your
 * preference" on the side it keeps, or "from your preference" where
 * applying it decided the conflict. Shows nothing elsewhere, and outside
 * the window's preferences.
 */
export function PreferenceMark({
  duel,
  spots,
  side,
  plugin,
  named = false,
}: {
  /** Whether the mark names the mod it keeps, where it stands at neither side. */
  named?: boolean;
  /** A duel, by its identity, with the side the mark stands at. */
  duel?: string;
  side?: 0 | 1;
  /** The spots of a place, with the plugin of the side the mark stands at. */
  spots?: readonly MapSpot[];
  plugin?: string;
}) {
  const preferences = usePreferences();
  const p = usePreferenceTexts();
  if (preferences === null) {
    return null;
  }
  let found: Preferred | null = null;
  let madeBy: string | null = null;
  if (duel !== undefined) {
    const at = preferences.forDuel(duel);
    found = at !== null && (side === undefined || at.side === side) ? at : null;
    madeBy = preferences.madeBy(`duel:${duel}`);
  } else if (spots !== undefined && plugin !== undefined) {
    for (const spot of spots) {
      const at = preferences.forSpot(spot);
      if (at !== null && at.plugin.toLowerCase() === plugin.toLowerCase()) {
        found = at;
        const key = spotKey(spot);
        madeBy = key === null ? null : preferences.madeBy(key);
        break;
      }
    }
  }
  if (found === null) {
    return null;
  }
  return (
    <span className="preference-mark">
      <Chip tone="info" title={p.markTip(preferences.describe(found.rule))}>
        {madeBy === null ? (named ? p.markFor(found.rule.mod) : p.mark) : p.markFrom}
      </Chip>
    </span>
  );
}
