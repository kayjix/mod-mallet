// The whole load order as an arc diagram, the "Dependencies" tab of the
// Mods page (ADR 0068): find a plugin, pick it, and see what it needs and
// what needs it.

import { use, useMemo, useState } from "react";
import { DependencyArcs } from "./DependencyArcs";
import { mostNeeded, neighbours, search } from "./dependencies";
import { useDependencyTexts } from "./dependency-texts";
import { InfoTip } from "./info-tip";
import { ArcKey, DependencyContext, GroupList, summaryOf } from "./ModDependencies";

/** How many plugins a search lists to pick from. */
const RESULTS = 12;

/** The plugins most needed, offered when nothing is picked. */
const MOST_NEEDED = 8;

/** The diagram of every active plugin and the plugins it needs. */
export function DependencyView() {
  const { graph, openMod } = use(DependencyContext);
  const d = useDependencyTexts();
  const [picked, setPicked] = useState<number | null>(null);
  const [needle, setNeedle] = useState("");
  const found = useMemo(() => (graph === null ? [] : search(graph, needle)), [graph, needle]);
  const marked = useMemo(() => new Set(found.map((node) => node.index)), [found]);
  const focus = useMemo(() => (picked === null ? [] : [picked]), [picked]);
  const near = useMemo(
    () => (graph === null || picked === null ? null : neighbours(graph, [picked])),
    [graph, picked],
  );
  const top = useMemo(() => (graph === null ? [] : mostNeeded(graph, MOST_NEEDED)), [graph]);

  if (graph === null || graph.nodes.length === 0) {
    return <p className="empty">{d.noLoadOrder}</p>;
  }
  const count = graph.nodes.length;
  const node = picked === null ? undefined : graph.nodes[picked];

  return (
    <section className="dep-page">
      <div className="dep-toolbar">
        <input
          className="text-input dep-search"
          type="search"
          placeholder={d.search}
          aria-label={d.search}
          value={needle}
          onChange={(event) => setNeedle(event.target.value)}
          onKeyDown={(event) => {
            const first = found[0];
            if (event.key === "Enter" && first !== undefined) {
              setPicked(first.index);
            }
          }}
        />
        <span className="aside">
          {needle.trim() === "" ? d.stats(count, graph.links) : d.found(found.length)}
        </span>
        <span className="dep-legend">
          <span>
            <ArcKey kind="needs" /> {d.needs}
          </span>
          <span>
            <ArcKey kind="needed-by" /> {d.neededBy}
          </span>
          <InfoTip text={d.explain} />
        </span>
      </div>
      {found.length > 0 && (
        <ul className="dep-results">
          {found.slice(0, RESULTS).map((result) => (
            <li key={result.index}>
              <button
                type="button"
                className="small"
                aria-pressed={picked === result.index}
                onClick={() => setPicked(result.index)}
              >
                {result.name}
              </button>
            </li>
          ))}
          {found.length > RESULTS && <li className="aside">+{found.length - RESULTS}</li>}
        </ul>
      )}

      <div className="dep-canvas-panel">
        <DependencyArcs
          graph={graph}
          focus={focus}
          marked={marked}
          height={340}
          onPick={(place) => setPicked(place === picked ? null : place)}
          onStep={(direction) =>
            setPicked(
              direction === 0
                ? null
                : Math.min(
                    count - 1,
                    Math.max(0, (picked ?? (direction > 0 ? -1 : count)) + direction),
                  ),
            )
          }
        />
      </div>

      {node === undefined || near === null ? (
        <div className="dep-detail">
          <p className="aside">{d.pickHint}</p>
          {top.length > 0 && (
            <>
              <h4>{d.mostNeeded}</h4>
              <ul className="dep-top">
                {top.map((entry) => (
                  <li key={entry.index}>
                    <button
                      type="button"
                      className="dep-plugin"
                      onClick={() => setPicked(entry.index)}
                    >
                      {entry.name}
                    </button>
                    <span className="aside">{d.dependentsCount(entry.dependents.length)}</span>
                  </li>
                ))}
              </ul>
            </>
          )}
        </div>
      ) : (
        <div className="dep-detail">
          <div className="dep-detail-head">
            <h3>{node.name}</h3>
            {node.modName !== null &&
              (node.hasPage && node.modId !== null ? (
                <button
                  type="button"
                  className="link-button"
                  onClick={() => node.modId !== null && openMod(node.modId)}
                >
                  {d.fromMod(node.modName)}
                </button>
              ) : (
                <span className="aside">{d.fromMod(node.modName)}</span>
              ))}
            <span className="aside">{d.place(node.index + 1, count)}</span>
            <button type="button" className="small quiet" onClick={() => setPicked(null)}>
              {d.clear}
            </button>
          </div>
          <p className="dep-summary">{summaryOf(d, near)}</p>
          <div className="dep-lists">
            <GroupList
              key={`needs-${node.index}`}
              graph={graph}
              kind="needs"
              title={d.needs}
              where={d.needsWhere}
              groups={near.needs}
              missing={near.missing}
              empty={d.needsNothing}
              onPickPlugin={setPicked}
            />
            <GroupList
              key={`needed-by-${node.index}`}
              graph={graph}
              kind="needed-by"
              title={d.neededBy}
              where={d.neededByWhere}
              groups={near.neededBy}
              empty={d.neededByNothing}
              onPickPlugin={setPicked}
            />
          </div>
        </div>
      )}
    </section>
  );
}
