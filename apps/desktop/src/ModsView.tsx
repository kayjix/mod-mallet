import { useMemo, useState } from "react";
import { ModCard, useSession } from "./Actions";
import { modsOf } from "./arrange";
import type { ModInfo, RenderedFinding } from "./generated/rendered-report";

/**
 * The mods the mod manager knows, each with what Mod Mallet found about it:
 * the view for someone who thinks in mods rather than in problems.
 */
export function ModsView({ mods, findings }: { mods: ModInfo[]; findings: RenderedFinding[] }) {
  const { t, ignored } = useSession();
  const [onlyProblems, setOnlyProblems] = useState(true);
  const [search, setSearch] = useState("");

  // The open findings of each mod, by the name findings use for it.
  const open = useMemo(() => {
    const byMod = new Map<string, RenderedFinding[]>();
    for (const rendered of findings) {
      if (rendered.finding.severity === "info" || ignored.has(rendered.finding.id)) {
        continue;
      }
      for (const name of modsOf(rendered)) {
        byMod.set(name, [...(byMod.get(name) ?? []), rendered]);
      }
    }
    return byMod;
  }, [findings, ignored]);

  const needle = search.trim().toLowerCase();
  const shown = mods
    .filter((info) => !onlyProblems || (open.get(info.mod.name)?.length ?? 0) > 0)
    .filter((info) => needle === "" || info.mod.name.toLowerCase().includes(needle))
    .sort(
      (a, b) =>
        (open.get(b.mod.name)?.length ?? 0) - (open.get(a.mod.name)?.length ?? 0) ||
        a.mod.name.localeCompare(b.mod.name),
    );

  return (
    <section className="mods-view">
      <div className="controls">
        <label className="control">
          {t.modsSearch}
          <input
            className="text-input"
            type="search"
            value={search}
            onChange={(event) => setSearch(event.target.value)}
          />
        </label>
        <label className="control-check">
          <input
            type="checkbox"
            checked={onlyProblems}
            onChange={(event) => setOnlyProblems(event.target.checked)}
          />
          {t.modsOnlyProblems}
        </label>
      </div>
      {shown.length === 0 ? (
        <p className="empty">{t.modsNone}</p>
      ) : (
        <ul className="mod-list">
          {shown.map((info) => {
            const problems = open.get(info.mod.name) ?? [];
            return (
              <li className="mod-entry" key={info.mod.id}>
                <ModCard info={info} />
                {problems.length > 0 && (
                  <ul className="mod-problems">
                    {problems.map((rendered) => (
                      <li
                        className="mod-problem"
                        key={rendered.finding.id}
                        data-severity={rendered.finding.severity}
                      >
                        <span className="dot" aria-hidden="true" />
                        {rendered.headline}
                      </li>
                    ))}
                  </ul>
                )}
              </li>
            );
          })}
        </ul>
      )}
    </section>
  );
}
