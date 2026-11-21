// The dependencies of one mod's plugins, on the mod's page (ADR 0068),
// and the parts the whole diagram shares with it.

import { createContext, type ReactNode, use, useMemo, useState } from "react";
import { DependencyArcs } from "./DependencyArcs";
import { type DependencyGraph, type Group, neighbours, pluginsOfMod, stem } from "./dependencies";
import { useDependencyTexts } from "./dependency-texts";

/** What the dependency views need from the Mods page around them. */
export interface DependencyPage {
  /** The load order as a graph, or `null` if the report has none. */
  graph: DependencyGraph | null;
  /** Shows a mod's page in the list of mods. */
  openMod: (modId: string) => void;
}

/** The Mods page's load order and the way to open a mod from a dependency. */
export const DependencyContext = createContext<DependencyPage>({
  graph: null,
  openMod: () => {},
});

/** How many groups a list shows before "show all". */
const SHORT_LIST = 8;

/** How many plugins of one mod a list names before "show all". */
const SHORT_GROUP = 12;

/** How many names the summary of what a mod needs gives. */
const SUMMARY_NAMES = 6;

/** The tabs of the Mods page: the list of mods, and the whole diagram. */
export function ModsTabs({
  tab,
  onTab,
}: {
  tab: "list" | "dependencies";
  onTab: (tab: "list" | "dependencies") => void;
}) {
  const d = useDependencyTexts();
  return (
    <div className="tabs" role="tablist">
      {(["list", "dependencies"] as const).map((id) => (
        <button
          type="button"
          role="tab"
          className="tab"
          key={id}
          aria-selected={tab === id}
          onClick={() => onTab(id)}
        >
          {id === "list" ? d.tabList : d.tabDependencies}
        </button>
      ))}
    </div>
  );
}

/** The small picture of a kind of arc, so that the lists say which arcs are theirs. */
export function ArcKey({ kind }: { kind: "needs" | "needed-by" }) {
  return (
    <svg className="dep-key" data-kind={kind} viewBox="0 0 28 12" aria-hidden="true">
      {kind === "needs" ? (
        <>
          <path d="M4 11 Q15 -3 26 11" fill="none" />
          <polygon points="4,4 1,10 7,10" />
        </>
      ) : (
        <>
          <path d="M2 1 Q14 15 26 1" fill="none" strokeDasharray="4 2.5" />
          <circle cx="26" cy="3" r="2.5" />
        </>
      )}
    </svg>
  );
}

/** One side of what plugins need or are needed by: the plugins by mod, each mod a link to its page. */
export function GroupList({
  graph,
  kind,
  title,
  where,
  groups,
  missing = [],
  empty,
  onPickPlugin,
}: {
  graph: DependencyGraph;
  kind: "needs" | "needed-by";
  title: string;
  /** Where the diagram shows them. */
  where: string;
  groups: Group[];
  /** Plugins needed that are not loaded. */
  missing?: { plugin: string; master: string }[];
  /** What to say when the list is empty. */
  empty: string;
  /** Picks a plugin in the diagram when its name is clicked, if the diagram can. */
  onPickPlugin?: (place: number) => void;
}) {
  const d = useDependencyTexts();
  const { openMod } = use(DependencyContext);
  const [all, setAll] = useState(false);
  const shown = all ? groups : groups.slice(0, SHORT_LIST);
  const total = groups.reduce((sum, group) => sum + group.plugins.length, 0);
  const long =
    groups.length > SHORT_LIST || groups.some((group) => group.plugins.length > SHORT_GROUP);
  return (
    <div className="dep-list" data-kind={kind}>
      <h4>
        <ArcKey kind={kind} />
        {title}
        {total > 0 && <span className="dep-count">{total}</span>}
        <span className="aside">{where}</span>
      </h4>
      {groups.length === 0 && missing.length === 0 ? (
        <p className="aside">{empty}</p>
      ) : (
        <ul>
          {missing.map(({ plugin, master }) => (
            <li key={`${plugin}/${master}`} className="dep-missing">
              <span>{stem(master)}</span>
              <span className="tag">{d.notLoaded}</span>
              <span className="aside">{d.notLoadedNote}</span>
            </li>
          ))}
          {shown.map((group) => (
            <li key={group.key}>
              {group.solo ? null : group.modId === null ? (
                <span className="dep-name">{group.label}</span>
              ) : (
                <button
                  type="button"
                  className="dep-name link-button"
                  onClick={() => group.modId !== null && openMod(group.modId)}
                >
                  {group.label}
                </button>
              )}
              <span className="dep-plugins" data-solo={group.solo}>
                {(all ? group.plugins : group.plugins.slice(0, SHORT_GROUP)).map((place) => {
                  const name = graph.nodes[place]?.name ?? "";
                  return onPickPlugin === undefined ? (
                    <span key={place}>{name}</span>
                  ) : (
                    <button
                      type="button"
                      className="dep-plugin"
                      key={place}
                      onClick={() => onPickPlugin(place)}
                    >
                      {name}
                    </button>
                  );
                })}
                {!all && group.plugins.length > SHORT_GROUP && (
                  <span>+{group.plugins.length - SHORT_GROUP}</span>
                )}
              </span>
            </li>
          ))}
        </ul>
      )}
      {long && (
        <button type="button" className="small quiet" onClick={() => setAll(!all)}>
          {all ? d.showFewer : d.showAll(total)}
        </button>
      )}
    </div>
  );
}

/** The plain sentence of what a set of plugins needs and what needs them. */
export function summaryOf(
  d: ReturnType<typeof useDependencyTexts>,
  near: ReturnType<typeof neighbours>,
): string {
  const names = near.needs.map((group) => group.label);
  const needs =
    names.length === 0 && near.missing.length === 0
      ? d.needsNothing
      : d.needsSummary(
          [...near.missing.map(({ master }) => stem(master)), ...names].slice(0, SUMMARY_NAMES),
          Math.max(0, names.length + near.missing.length - SUMMARY_NAMES),
        );
  const neededBy =
    near.neededByPlugins === 0
      ? d.neededByNothing
      : d.neededBySummary(near.neededByPlugins, near.neededByMods);
  return `${needs} ${neededBy}`;
}

/** What a mod's plugins need and what needs them: a strip of the whole load order, a sentence and two lists. */
export function ModDependencies({ modId }: { modId: string }): ReactNode {
  const { graph, openMod } = use(DependencyContext);
  const d = useDependencyTexts();
  const focus = useMemo(() => (graph === null ? [] : pluginsOfMod(graph, modId)), [graph, modId]);
  const near = useMemo(() => (graph === null ? null : neighbours(graph, focus)), [graph, focus]);
  if (graph === null || near === null || focus.length === 0) {
    return null;
  }
  const own = focus.map((place) => graph.nodes[place]?.name ?? "").join(", ");
  return (
    <div className="mod-section dep-section">
      <h3>{d.section}</h3>
      <p className="dep-summary">{summaryOf(d, near)}</p>
      <DependencyArcs
        graph={graph}
        focus={focus}
        height={96}
        compact
        onPick={(place) => {
          const node = graph.nodes[place];
          if (node?.hasPage && node.modId !== null && node.modId !== modId) {
            openMod(node.modId);
          }
        }}
      />
      <p className="aside dep-own">{d.ownPlugins(own)}</p>
      <div className="dep-lists">
        <GroupList
          graph={graph}
          kind="needs"
          title={d.needs}
          where={d.needsWhere}
          groups={near.needs}
          missing={near.missing}
          empty={d.needsNothing}
        />
        <GroupList
          graph={graph}
          kind="needed-by"
          title={d.neededBy}
          where={d.neededByWhere}
          groups={near.neededBy}
          empty={d.neededByNothing}
        />
      </div>
    </div>
  );
}
