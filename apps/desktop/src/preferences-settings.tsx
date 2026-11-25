// The preferences between mods in the settings (ADR 0086): learning on or
// off, what was learned with how often, each to turn off or use again and
// to apply to the open conflicts, and the preferences the user sets.

import { useMemo, useState } from "react";
import { useSession } from "./Actions";
import { PreferencePreview } from "./preference-offer";
import { usePreferenceTexts } from "./preference-texts";
import { isTopic, MIN_DECISIONS, type Rule, TOPICS } from "./preferences";
import { type Preferences, usePreferences } from "./preferences-ui";
import { Chip, EmptyState } from "./ui/kit";

/** One preference with the open conflicts it would decide and the way to apply it. */
function RuleApply({ rule, preferences }: { rule: Rule; preferences: Preferences }) {
  const p = usePreferenceTexts();
  const [showing, setShowing] = useState(false);
  const items = useMemo(() => preferences.planFor(rule), [preferences, rule]);
  if (items.length === 0) {
    return null;
  }
  return (
    <>
      <button type="button" className="small quiet" onClick={() => setShowing(!showing)}>
        {p.openConflicts(items.length)} {showing ? "▾" : "▸"}
      </button>
      <button
        type="button"
        className="small"
        disabled={preferences.progress !== null}
        onClick={() => preferences.apply(rule, items)}
      >
        {p.applyHere(items.length)}
      </button>
      {showing && (
        <div className="preference-rule-preview">
          <PreferencePreview items={items} />
        </div>
      )}
    </>
  );
}

/** The section of the settings about preferences between mods. */
export function PreferencesSection() {
  const preferences = usePreferences();
  const { mods } = useSession();
  const p = usePreferenceTexts();
  const [mod, setMod] = useState("");
  const [topic, setTopic] = useState("");
  const [over, setOver] = useState("");
  const names = useMemo(
    () =>
      [...new Set([...mods.values()].map((info) => info.mod.name))].sort((a, b) =>
        a.localeCompare(b),
      ),
    [mods],
  );
  if (preferences === null) {
    return null;
  }
  const { kept, learned, rules } = preferences;
  const learning = kept.learningOff !== true;
  const own = kept.rules ?? [];
  const add = () => {
    if (mod === "") {
      return;
    }
    preferences.addRule({
      mod,
      over: over === "" ? null : over,
      topic: isTopic(topic) ? topic : null,
    });
    setMod("");
    setOver("");
    setTopic("");
  };

  return (
    <div className="stack preferences-section">
      <section className="panel">
        <div className="panel-head">
          <h2>{p.title}</h2>
        </div>
        <div className="panel-body">
          <p>{p.intro}</p>
          <p>{p.applyNote}</p>
          <label className="check">
            <input
              type="checkbox"
              checked={learning}
              onChange={(event) => preferences.setLearning(event.target.checked)}
            />
            {p.learningLabel}
          </label>
          <p className="aside">{p.learningNote}</p>
        </div>
      </section>

      {learning && (
        <section className="panel">
          <div className="panel-head">
            <h2>{p.learned}</h2>
          </div>
          <div className="panel-body">
            {learned.length === 0 ? (
              <EmptyState>{p.learnedNone(MIN_DECISIONS)}</EmptyState>
            ) : (
              <ul className="preference-list">
                {learned.map((entry) => {
                  const off = kept.answers?.[entry.id]?.answer === "never";
                  const rule = rules.find((candidate) => candidate.id === entry.id);
                  return (
                    <li key={entry.id} className="preference-row" data-off={off || undefined}>
                      <span className="preference-row-text">
                        <strong>{preferences.describe(entry)}</strong>
                        <span className="aside">
                          {" "}
                          · {p.kept(entry.chosen, entry.of)}
                          {entry.followed > 0 && ` ${p.followed(entry.followed)}`}
                        </span>
                      </span>
                      <Chip tone={off ? "neutral" : "ok"}>{off ? p.turnedOff : p.inUse}</Chip>
                      <span className="preference-row-actions">
                        {!off && rule !== undefined && (
                          <RuleApply rule={rule} preferences={preferences} />
                        )}
                        {off ? (
                          <button
                            type="button"
                            className="small quiet"
                            onClick={() => preferences.again(entry.id)}
                          >
                            {p.useAgain}
                          </button>
                        ) : (
                          <button
                            type="button"
                            className="small quiet"
                            onClick={() => preferences.answer(entry, "never")}
                          >
                            {p.turnOff}
                          </button>
                        )}
                      </span>
                    </li>
                  );
                })}
              </ul>
            )}
          </div>
        </section>
      )}

      <section className="panel">
        <div className="panel-head">
          <h2>{p.own}</h2>
        </div>
        <div className="panel-body">
          {own.length === 0 ? (
            <p className="aside">{p.ownNone}</p>
          ) : (
            <ul className="preference-list">
              {own.map((entry) => {
                const shape = {
                  mod: entry.mod,
                  over: entry.over ?? null,
                  topic: isTopic(entry.topic) ? entry.topic : null,
                };
                const rule = rules.find(
                  (candidate) =>
                    candidate.explicit &&
                    candidate.mod === shape.mod &&
                    candidate.over === shape.over &&
                    candidate.topic === shape.topic,
                );
                return (
                  <li
                    key={`${entry.mod}|${entry.over ?? ""}|${entry.topic ?? ""}`}
                    className="preference-row"
                  >
                    <span className="preference-row-text">
                      <strong>{preferences.describe(shape)}</strong>
                    </span>
                    <span className="preference-row-actions">
                      {rule !== undefined && <RuleApply rule={rule} preferences={preferences} />}
                      <button
                        type="button"
                        className="small quiet"
                        onClick={() => preferences.removeRule(entry)}
                      >
                        {p.remove}
                      </button>
                    </span>
                  </li>
                );
              })}
            </ul>
          )}
          <div className="preference-add">
            <strong>{p.addTitle}</strong>
            <select
              aria-label={p.addMod}
              value={mod}
              onChange={(event) => setMod(event.target.value)}
            >
              <option value="">{p.pickMod}</option>
              {names.map((name) => (
                <option key={name} value={name}>
                  {name}
                </option>
              ))}
            </select>
            <label>
              {p.addTopic}{" "}
              <select
                aria-label={p.addTopic}
                value={topic}
                onChange={(event) => setTopic(event.target.value)}
              >
                <option value="">{p.everything}</option>
                {TOPICS.map((entry) => (
                  <option key={entry} value={entry}>
                    {p.topic[entry]}
                  </option>
                ))}
              </select>
            </label>
            <label>
              {p.addOver}{" "}
              <select
                aria-label={p.addOver}
                value={over}
                onChange={(event) => setOver(event.target.value)}
              >
                <option value="">{p.everyOther}</option>
                {names
                  .filter((name) => name !== mod)
                  .map((name) => (
                    <option key={name} value={name}>
                      {name}
                    </option>
                  ))}
              </select>
            </label>
            <button type="button" className="primary small" disabled={mod === ""} onClick={add}>
              {p.add}
            </button>
          </div>
        </div>
      </section>
    </div>
  );
}
