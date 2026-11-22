// Help for a mod switched off in Vortex that the game still loads, and for
// the files such a mod left in the Data folder (ADR 0061, 0073).
//
// Mod Mallet moves nothing itself. On a click it asks Vortex to switch the
// plugin off in its list of plugins, together with the plugins that build
// on it (asked first, naming them); Vortex's list holds the copy although
// Vortex did not install it. The files themselves are shown in the file
// explorer, for the user to move out of the Data folder.

import { useState } from "react";
import { ActionResult, useAction, useSession } from "./Actions";
import { DependentLinks } from "./dependent-links";
import type { RenderedFinding } from "./generated/rendered-report";
import { switchedOffTexts } from "./switched-off-texts";
import "./still-loaded.css";

/** A plugin of a switched-off mod the game still loads. */
export const STILL_LOADED = "plugins.switched-off-but-loaded";
/** Files a switched-off mod left in the Data folder, no longer loaded. */
export const LEFT_BEHIND = "plugins.switched-off-left-behind";

const textOf = (rendered: RenderedFinding, name: string) => {
  const value = rendered.finding.args[name];
  return typeof value === "string" ? value : "";
};

const linesOf = (rendered: RenderedFinding, name: string) => {
  const value = textOf(rendered, name);
  return value === "" ? [] : value.split("\n");
};

/** The files a finding of either kind names, the plugin first if it is among them. */
export const stillFiles = (rendered: RenderedFinding) => linesOf(rendered, "fileList");

/** The switch-off in Vortex a finding offers, if any. */
function disableOf(rendered: RenderedFinding) {
  return rendered.actions.find((available) => available.action.type === "disable-plugins");
}

/**
 * The button that asks Vortex to switch the plugin off, with what builds
 * on it; first a question naming those.
 */
export function DisablePluginsButton({
  rendered,
  primary = true,
}: {
  rendered: RenderedFinding;
  primary?: boolean;
}) {
  const { lang } = useSession();
  const s = switchedOffTexts[lang];
  const action = useAction();
  const [asking, setAsking] = useState(false);
  const available = disableOf(rendered);
  if (available === undefined) {
    return null;
  }
  const plugins = available.action.type === "disable-plugins" ? available.action.plugins : [];
  const [plugin = "", ...others] = plugins;
  const run = () => {
    setAsking(false);
    void action.run(available.action);
  };
  return (
    <div className="still-switch">
      {asking ? (
        <div className="still-confirm" role="alertdialog">
          <p>{s.confirmPlugins(plugin, others)}</p>
          <span className="still-buttons">
            <button type="button" className="primary" disabled={action.busy} onClick={run}>
              {s.confirmYes}
            </button>
            <button type="button" onClick={() => setAsking(false)}>
              {s.cancel}
            </button>
          </span>
        </div>
      ) : (
        <button
          type="button"
          className={primary ? "primary" : undefined}
          title={available.description}
          disabled={action.busy}
          onClick={() => (others.length > 0 ? setAsking(true) : run())}
        >
          {available.label}
        </button>
      )}
      <ActionResult state={action} />
    </div>
  );
}

/** The button that shows the plugin, or the first file, in the Data folder. */
export function ShowFilesButton({ files }: { files: string[] }) {
  const { api, lang } = useSession();
  const s = switchedOffTexts[lang];
  const [failed, setFailed] = useState<string | null>(null);
  // The plugin, if it is among them: its archive lies right beside it.
  const first = files.find((file) => /\.(esp|esm|esl)$/i.test(file)) ?? files[0];
  if (first === undefined) {
    return null;
  }
  return (
    <>
      <button
        type="button"
        onClick={() => {
          setFailed(null);
          api.showDataFile(first).catch((error: unknown) => setFailed(String(error)));
        }}
      >
        {s.showFiles}
      </button>
      {failed !== null && <p className="aside">{failed}</p>}
    </>
  );
}

/**
 * What keeps the mod in the game, or what it left behind, and what to do:
 * the files Vortex did not put there, the plugins that build on it, the
 * switch-off in Vortex and the files in the file explorer. `lead` puts the
 * mod's state first, as its page does; the tasks' row has its headline.
 */
export function StillLoadedHelp({
  rendered,
  lead = false,
  spots = 0,
}: {
  rendered: RenderedFinding;
  lead?: boolean;
  /** How many collisions of the mod are listed, for the lead. */
  spots?: number;
}) {
  const { lang } = useSession();
  const s = switchedOffTexts[lang];
  const mod = textOf(rendered, "mod");
  const plugin = textOf(rendered, "plugin");
  const files = linesOf(rendered, "fileList");
  const left = rendered.finding.kind === LEFT_BEHIND;
  const deployed = rendered.finding.args.deployed === 1;
  return (
    <div className="still-loaded" data-kind={left ? "left" : "loaded"}>
      {lead && (
        <p className="still-lead">
          <strong>{left ? s.leftLead(mod, files.length) : s.modLead(mod, plugin, spots)}</strong>
        </p>
      )}
      {deployed ? (
        <p>{s.notDeployed}</p>
      ) : (
        <>
          {!left && files.length > 0 && <p>{s.notVortex(files.length)}</p>}
          {files.length > 0 && (
            <ul className="still-files">
              {files.map((file) => (
                <li key={file}>
                  <code>{file}</code>
                </li>
              ))}
            </ul>
          )}
          {!left && <DependentLinks rendered={rendered} />}
          <div className="still-buttons">
            <DisablePluginsButton rendered={rendered} />
            <ShowFilesButton files={files} />
          </div>
          {!left && disableOf(rendered) !== undefined && <p className="aside">{s.afterwards}</p>}
          <p className="aside">{s.moveOut(files.length)}</p>
        </>
      )}
    </div>
  );
}
