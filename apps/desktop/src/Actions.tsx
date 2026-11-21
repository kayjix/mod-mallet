import { createContext, useContext, useState } from "react";
import type { Api } from "./api";
import type { RenderedOutcome } from "./generated/rendered-outcome";
import type { Action, MapSpot, ModInfo } from "./generated/rendered-report";
import type { Lang, Texts } from "./i18n";
import type { PatchChoices } from "./patch";

/** What the components of a report need from the window around them. */
export interface Session {
  /** Switches to the collisions, showing only the spots pinned to solve later. */
  showPinned: () => void;
  /** Whether the collisions are to open with only the pinned spots. */
  pinnedAsked: boolean;
  api: Api;
  lang: Lang;
  /** The game in use, by its identity; empty until known. */
  game: string;
  t: Texts;
  /** Scans again, for after an action was finished in the mod manager. */
  rescan: () => void;
  /** Tells the window that the state of the link may have changed. */
  linkChanged: () => void;
  /** The findings the user has set aside, by identity. */
  ignored: Set<string>;
  /** Sets a finding aside or brings it back. */
  setIgnored: (id: string, ignored: boolean) => void;
  /** The mods of the report, by the name findings use for them. */
  mods: Map<string, ModInfo>;
  /** The spots of each finding about placed objects, by finding. */
  spots: Map<string, MapSpot[]>;
  /** Switches to the map and shows a spot there. */
  showOnMap: (spot: MapSpot) => void;
  /** Switches to the collisions' worklist and opens a spot there. */
  decideSpot: (spot: MapSpot) => void;
  /** What the patch is to hide, per collision; shared by all views. */
  patchChoices: PatchChoices;
  /** Replaces the patch's choice. */
  setPatchChoices: (choices: PatchChoices) => void;
  /** Switches to the patch view. */
  showPatch: () => void;
  /** The side picked in each decided duel, by duel. */
  decisions: Record<string, number>;
  /** Remembers the side picked in a duel, or forgets it with null. */
  decide: (id: string, side: number | null) => void;
  /** The mods in the order the user prefers them, the favourite first. */
  ranking: string[];
  /** Keeps a new order of preference. */
  setRanking: (ranking: string[]) => void;
  /** Switches to the assets page, the comparison of two mods' files, at one duel if given. */
  openCompare: (duel: string | null) => void;
  /**
   * Switches to the collisions' pairs of mods and opens the pair of two
   * mods there, by the names the mod manager shows for them (ADR 0048).
   */
  openPair: (first: string, second: string) => void;
  /** The pair the collisions are to open with, until the user goes elsewhere. */
  pairAt: [string, string] | null;
  /** Switches to the map, filtered to what one mod places, by its name. */
  showModOnMap: (mod: string) => void;
  /** The mod the map is to be filtered to, until the user goes elsewhere. */
  mapMod: string | null;
  /**
   * Switches to the toolbox, for a tool that does what Mod Mallet cannot,
   * such as the Creation Kit repairing navigation meshes. Absent where the
   * window has no toolbox to go to.
   */
  showToolbox?: () => void;
}

export const SessionContext = createContext<Session | null>(null);

export function useSession(): Session {
  const session = useContext(SessionContext);
  if (session === null) {
    throw new Error("no session: the component is used outside the app");
  }
  return session;
}

/** An action as one finding sees it: how to start it and what became of it. */
export interface ActionState {
  busy: boolean;
  /** Whether anything happened that the user should read. */
  touched: boolean;
  outcome: RenderedOutcome | null;
  /** Why the link is needed, when the action failed for lack of it. */
  linkExplanation: string | null;
  /** What installing the link did. */
  linkNote: string | null;
  run: (action: Action) => Promise<void>;
  installLink: () => Promise<void>;
}

/**
 * Hands actions to the mod manager and keeps what became of them.
 *
 * Mod Mallet changes nothing itself. If the link to the mod manager is
 * missing, the user is told what it is and can install it from here.
 * Nothing is installed without that click.
 */
export function useAction(): ActionState {
  const { api, lang, linkChanged } = useSession();
  const [busy, setBusy] = useState(false);
  const [outcome, setOutcome] = useState<RenderedOutcome | null>(null);
  const [linkExplanation, setLinkExplanation] = useState<string | null>(null);
  const [linkNote, setLinkNote] = useState<string | null>(null);

  const run = async (action: Action) => {
    setBusy(true);
    setLinkNote(null);
    setLinkExplanation(null);
    try {
      const result = await api.runAction(action, lang);
      setOutcome(result);
      if (result.outcome.status === "link-missing") {
        setLinkExplanation((await api.managerLink(lang)).explanation);
      }
    } catch (error: unknown) {
      setOutcome({ outcome: { status: "failed" }, message: String(error) });
    } finally {
      setBusy(false);
      linkChanged();
    }
  };

  const installLink = async () => {
    setBusy(true);
    try {
      setLinkNote(await api.installManagerLink(lang));
      setOutcome(null);
      setLinkExplanation(null);
    } catch (error: unknown) {
      setLinkNote(String(error));
    } finally {
      setBusy(false);
      linkChanged();
    }
  };

  return {
    busy,
    touched: outcome !== null || linkExplanation !== null || linkNote !== null,
    outcome,
    linkExplanation,
    linkNote,
    run,
    installLink,
  };
}

/** What became of an action, below the finding it belongs to. */
export function ActionResult({ state }: { state: ActionState }) {
  const { t, rescan } = useSession();
  const { outcome, linkExplanation, linkNote, busy, installLink } = state;

  return (
    <>
      {outcome !== null && (
        <div className="outcome" data-status={outcome.outcome.status} role="status">
          <p className="outcome-text">{outcome.message}</p>
          {outcome.outcome.status === "started" && (
            <button type="button" onClick={rescan}>
              {t.rescan}
            </button>
          )}
        </div>
      )}
      {linkExplanation !== null && (
        <div className="outcome">
          <p className="outcome-text">{linkExplanation}</p>
          <button type="button" onClick={installLink} disabled={busy}>
            {t.installLink}
          </button>
        </div>
      )}
      {linkNote !== null && (
        <div className="outcome" role="status">
          <p className="outcome-text">{linkNote}</p>
        </div>
      )}
    </>
  );
}

/** A button that puts a text on the clipboard and says so. */
export function CopyButton({ text }: { text: string }) {
  const { t } = useSession();
  const [copied, setCopied] = useState(false);
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      // Without a clipboard the command can still be selected by hand.
    }
  };
  return (
    <button type="button" className="small" onClick={copy}>
      {copied ? t.copied : t.copy}
    </button>
  );
}
