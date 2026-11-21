// The plugins that would lose a master if a mod went, as the engine names
// them on a suggestion to remove it (ADR 0061, 0068): each with a link to
// its spot in the dependency diagram and to its mod's page.

import { useSession } from "./Actions";
import { useDependencyTexts } from "./dependency-texts";
import type { RenderedFinding } from "./generated/rendered-report";
import "./dependencies.css";

/** How many plugins the list shows; the rest are counted. */
const SHOWN = 30;

/** A list argument of a finding, one value per line. */
function linesOf(rendered: RenderedFinding, name: string): string[] {
  const value = rendered.finding.args[name];
  return typeof value === "string" && value !== "" ? value.split("\n") : [];
}

/** Every plugin a suggestion says would lose a master, linked to the diagram and to its mod. */
export function DependentLinks({ rendered }: { rendered: RenderedFinding }) {
  const { mods, showDependency } = useSession();
  const d = useDependencyTexts();
  const plugins = linesOf(rendered, "dependentPlugins");
  const modIds = linesOf(rendered, "dependentModIds");
  if (plugins.length === 0) {
    return null;
  }
  const byId = new Map([...mods.values()].map((info) => [info.mod.id, info]));
  return (
    <div className="dep-lose">
      <p>{d.wouldLose(plugins.length)}</p>
      <ul>
        {plugins.slice(0, SHOWN).map((plugin, index) => {
          const info = byId.get(modIds[index] ?? "");
          return (
            <li key={plugin} className="dep-lose-item">
              {showDependency === undefined ? (
                <span className="dep-name">{plugin}</span>
              ) : (
                <button
                  type="button"
                  className="dep-plugin"
                  title={d.inDiagram(plugin)}
                  onClick={() => showDependency({ plugin })}
                >
                  {plugin}
                </button>
              )}
              {info !== undefined &&
                (showDependency === undefined ? (
                  <span className="aside">{info.mod.name}</span>
                ) : (
                  <button
                    type="button"
                    className="link-button"
                    title={d.modPage(info.mod.name)}
                    onClick={() => showDependency({ mod: info.mod.id })}
                  >
                    {info.mod.name}
                  </button>
                ))}
            </li>
          );
        })}
        {plugins.length > SHOWN && <li className="aside">+{plugins.length - SHOWN}</li>}
      </ul>
    </div>
  );
}
