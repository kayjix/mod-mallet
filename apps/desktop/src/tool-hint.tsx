// A line under a finding that names the tool which helps with it, from the
// tools Vortex lists for the game (ADR 0063). It only names the tool and
// says where to start it; Mod Mallet starts nothing.

import { useSession } from "./Actions";
import type { RenderedToolbox } from "./generated/rendered-toolbox";
import type { Lang } from "./i18n";

/** The kinds of finding that a cleaning tool helps with. */
const CLEANING_KINDS = new Set(["plugins.needs-cleaning", "plugins.identical-records"]);

interface ToolHintTexts {
  /** The cleaning tool is in Vortex's tools; `name` is how Vortex calls it. */
  ready: (name: string) => string;
  /** Only the editor is, not its cleaning variant. */
  editorOnly: (name: string) => string;
  /** The tool is listed but the program is gone. */
  gone: (name: string) => string;
  /** Vortex lists no such tool. */
  none: string;
}

const texts: Record<Lang, ToolHintTexts> = {
  en: {
    ready: (name) =>
      `"${name}" is among your tools in Vortex: start it from Vortex's dashboard, under Tools.`,
    editorOnly: (name) =>
      `"${name}" is among your tools in Vortex: start it from Vortex's dashboard, under Tools, and use its cleaning mode.`,
    gone: (name) =>
      `Vortex lists "${name}", but the program is no longer where Vortex expects it. Install it again or point Vortex at it under Tools.`,
    none: "Vortex lists no SSEEdit for this game yet. Install SSEEdit, add it in Vortex under Tools, then start it from there.",
  },
  de: {
    ready: (name) =>
      `„${name}“ steht unter deinen Werkzeugen in Vortex: Starte es über Vortex' Übersicht, unter Werkzeuge.`,
    editorOnly: (name) =>
      `„${name}“ steht unter deinen Werkzeugen in Vortex: Starte es über Vortex' Übersicht, unter Werkzeuge, und nutze seinen Säuberungsmodus.`,
    gone: (name) =>
      `Vortex führt „${name}“ auf, aber das Programm liegt nicht mehr dort, wo Vortex es erwartet. Installiere es neu oder zeige Vortex den Ort unter Werkzeuge.`,
    none: "Vortex führt für dieses Spiel noch kein SSEEdit auf. Installiere SSEEdit, trage es in Vortex unter Werkzeuge ein und starte es dann von dort.",
  },
};

/** What to say about the tool for a finding of this kind, or null. */
export function toolHint(kind: string, toolbox: RenderedToolbox | null, lang: Lang): string | null {
  if (!CLEANING_KINDS.has(kind) || toolbox === null) {
    return null;
  }
  const words = texts[lang];
  const find = (id: string) => toolbox.tools.find((rendered) => rendered.tool.id === id);
  const clean = find("xedit-clean");
  const editor = find("xedit");
  if (clean?.tool.state === "ready") {
    return words.ready(clean.name);
  }
  if (editor?.tool.state === "ready") {
    return words.editorOnly(editor.name);
  }
  const missing = clean ?? editor;
  return missing === undefined ? words.none : words.gone(missing.name);
}

/** The hint as a line of the finding's body, if there is one. */
export function ToolHint({ kind }: { kind: string }) {
  const { toolbox, lang } = useSession();
  const hint = toolHint(kind, toolbox, lang);
  return hint === null ? null : <p className="tool-hint">{hint}</p>;
}
