import { useState } from "react";
import { useSession } from "./Actions";
import type { RenderedTool, RenderedToolbox } from "./generated/rendered-toolbox";
import { InfoTip } from "./info-tip";
import { toolboxTexts } from "./toolbox-texts";
import "./toolbox.css";
import { LoadingScreen, Note } from "./ui/kit";

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
  const texts = toolboxTexts[lang];
  const [busy, setBusy] = useState(false);
  const [note, setNote] = useState<string | null>(null);
  const [failure, setFailure] = useState<string | null>(null);
  // The extension was just put into Vortex's folder: Vortex loads it only
  // after a restart, so the restart button asks for attention until then.
  const [placed, setPlaced] = useState(false);

  if (toolbox === null) {
    return <LoadingScreen title={t.scanning} />;
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
      setPlaced(false);
      setNote(texts.restartingVortex);
      await api.restartManager();
      // Vortex takes a few seconds before its link answers again.
      setTimeout(() => {
        refresh();
        linkChanged();
      }, 8000);
      return texts.restartedVortex;
    });

  const installLink = () =>
    attempt(async () => {
      const done = await api.installManagerLink(lang);
      setPlaced(true);
      return done;
    });

  // Asks Vortex, through the link, to list Mod Mallet among its tools or to
  // take it off the list. Only on the click.
  const setOwnTool = (add: boolean) =>
    attempt(async () => (await api.setVortexTool(add, lang)).message);

  const actionFor = (rendered: RenderedTool) => {
    const { tool } = rendered;
    if (tool.id === "vortex" && tool.state !== "missing") {
      return (
        <button
          type="button"
          className={placed ? "primary tool-wanted" : undefined}
          disabled={busy}
          title={texts.restartHint}
          onClick={restart}
        >
          {tool.state === "stopped" ? texts.startVortex : texts.restartVortex}
        </button>
      );
    }
    if (tool.id === "mod-mallet-tool") {
      const add = tool.state !== "ready";
      return (
        <button type="button" disabled={busy} onClick={() => setOwnTool(add)}>
          {add ? texts.addToVortex : texts.removeFromVortex}
        </button>
      );
    }
    if (tool.id === "mallet-link" && placed) {
      return (
        <button type="button" className="tool-placed" disabled title={texts.placedHint}>
          {texts.placed}
        </button>
      );
    }
    if (tool.id === "mallet-link" && tool.state !== "ready" && tool.state !== "stopped") {
      return (
        <button type="button" disabled={busy} onClick={installLink}>
          {tool.state === "missing" ? t.installLink : texts.updateLink}
        </button>
      );
    }
    if (tool.state !== "ready" && tool.page != null) {
      const page = tool.page;
      return (
        <button type="button" onClick={() => api.openPage(page)}>
          {texts.getIt}
        </button>
      );
    }
    return null;
  };

  // Tools the game or Mod Mallet needs first; the optional ones folded
  // below, each saying what it is for.
  const listed = toolbox.tools.filter((rendered) => rendered.tool.group === "mod-tool");
  const others = toolbox.tools.filter((rendered) => rendered.tool.group !== "mod-tool");
  const needed = others.filter((rendered) => rendered.tool.need !== "optional");
  const optional = others.filter((rendered) => rendered.tool.need === "optional");

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
        <Note tone="ok" role="status">
          <p>{note}</p>
        </Note>
      )}
      {failure !== null && (
        <Note tone="danger" role="alert">
          <p>{failure}</p>
        </Note>
      )}
      <ul className="tool-list">{needed.map(row)}</ul>
      {listed.length > 0 && (
        <div className="tool-listed">
          <h3>
            {texts.listed} <InfoTip text={texts.listedHint} />
          </h3>
          <ul className="tool-list">{listed.map(row)}</ul>
        </div>
      )}
      {optional.length > 0 && (
        <details className="tool-optional">
          <summary>
            {texts.optional(optional.length)} <InfoTip text={texts.optionalHint} />
          </summary>
          <ul className="tool-list">{optional.map(row)}</ul>
        </details>
      )}
    </section>
  );
}
