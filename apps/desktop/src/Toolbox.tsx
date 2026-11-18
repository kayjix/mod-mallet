import { useState } from "react";
import { useSession } from "./Actions";
import type { RenderedTool, RenderedToolbox } from "./generated/rendered-toolbox";
import { toolboxTexts } from "./toolbox-texts";
import "./toolbox.css";

interface ToolboxProps {
  toolbox: RenderedToolbox | null;
  /** Looks at the tools again, after something was done about them. */
  refresh: () => void;
}

/** The colour of a tool's dot: how urgent its state is. */
function toneOf({ tool }: RenderedTool): "ok" | "warning" | "error" | "muted" {
  if (tool.state === "ready") {
    return "ok";
  }
  if (tool.state === "stopped" || tool.need === "optional") {
    return "muted";
  }
  return tool.need === "required" ? "error" : "warning";
}

/**
 * What the setup needs besides its mods: Vortex and the link to it, the
 * script extender and its libraries, and the tools Mod Mallet uses. Each
 * with its state and the one thing to do about it.
 */
export function ToolboxView({ toolbox, refresh }: ToolboxProps) {
  const { t, api, lang, linkChanged } = useSession();
  const [busy, setBusy] = useState(false);
  const [note, setNote] = useState<string | null>(null);
  const [failure, setFailure] = useState<string | null>(null);

  if (toolbox === null) {
    return <p className="empty">{t.scanning}</p>;
  }

  const attempt = async (work: () => Promise<string | null>) => {
    setBusy(true);
    setFailure(null);
    setNote(null);
    try {
      setNote(await work());
    } catch (error: unknown) {
      setFailure(String(error));
    } finally {
      setBusy(false);
      refresh();
      linkChanged();
    }
  };

  const restart = () =>
    attempt(async () => {
      setNote(t.restartingVortex);
      await api.restartManager();
      // Vortex takes a few seconds before its link answers again.
      setTimeout(() => {
        refresh();
        linkChanged();
      }, 8000);
      return t.restartedVortex;
    });

  const installLink = () => attempt(() => api.installManagerLink(lang));

  const actionFor = (rendered: RenderedTool) => {
    const { tool } = rendered;
    if (tool.id === "vortex" && tool.state !== "missing") {
      return (
        <button type="button" disabled={busy} title={t.restartHint} onClick={restart}>
          {tool.state === "stopped" ? t.startVortex : t.restartVortex}
        </button>
      );
    }
    if (tool.id === "mallet-link" && tool.state !== "ready" && tool.state !== "stopped") {
      return (
        <button type="button" disabled={busy} onClick={installLink}>
          {tool.state === "missing" ? t.installLink : t.updateLink}
        </button>
      );
    }
    if (tool.state !== "ready" && tool.page != null) {
      const page = tool.page;
      return (
        <button type="button" onClick={() => api.openPage(page)}>
          {t.getIt}
        </button>
      );
    }
    return null;
  };

  const texts = toolboxTexts[lang];
  // Tools the game or Mod Mallet needs first; the optional ones folded
  // below, each saying what it is for.
  const needed = toolbox.tools.filter((rendered) => rendered.tool.need !== "optional");
  const optional = toolbox.tools.filter((rendered) => rendered.tool.need === "optional");

  const row = (rendered: RenderedTool) => (
    <li
      key={rendered.tool.id}
      className="tool"
      data-state={rendered.tool.state}
      data-need={rendered.tool.need}
    >
      <span className="tool-dot" data-tone={toneOf(rendered)} aria-hidden="true" />
      <div className="tool-main">
        <div className="tool-head">
          <strong>{rendered.name}</strong>
          <span className="tool-need">{rendered.needLabel}</span>
        </div>
        <p className="tool-purpose">{rendered.purpose}</p>
        {(rendered.tool.version != null || rendered.tool.path != null) && (
          <p className="tool-facts">
            {rendered.tool.version != null && <span>{rendered.tool.version}</span>}
            {rendered.tool.path != null && <code>{rendered.tool.path}</code>}
          </p>
        )}
      </div>
      <span className="tool-state">
        {rendered.stateLabel}
        {rendered.providedLabel != null && (
          <span className="tool-provided">{rendered.providedLabel}</span>
        )}
      </span>
      <div className="tool-action">{actionFor(rendered)}</div>
    </li>
  );

  return (
    <section className="toolbox">
      {note !== null && (
        <p className="note" role="status">
          {note}
        </p>
      )}
      {failure !== null && (
        <p className="failure" role="alert">
          {failure}
        </p>
      )}
      <ul className="tool-list">{needed.map(row)}</ul>
      {optional.length > 0 && (
        <details className="tool-optional">
          <summary>
            {texts.optional(optional.length)}
            <span className="tool-optional-hint">{texts.optionalHint}</span>
          </summary>
          <ul className="tool-list">{optional.map(row)}</ul>
        </details>
      )}
    </section>
  );
}
