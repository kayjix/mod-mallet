import { type ReactNode, useCallback, useEffect, useMemo, useRef, useState } from "react";
import "./App.css";
import "./journey.css";
import "./review.css";
import { type Session, SessionContext, useSession } from "./Actions";
import { AllGames, useGameArt } from "./AllGames";
import type { Api, NamedPlaces } from "./api";
import { type Counts, countFindings, modsByName } from "./arrange";
import { CatalogueView } from "./Catalogue";
import { Collisions } from "./Collisions";
import { CompareView } from "./Compare";
import { catalogueTexts } from "./catalogue-texts";
import { type Games, useGames } from "./games";
import type { RenderedLink } from "./generated/rendered-link";
import type { RenderedPlaces } from "./generated/rendered-places";
import type { MapSpot, RenderedReport } from "./generated/rendered-report";
import type { RenderedToolbox } from "./generated/rendered-toolbox";
import { type Health, healthOf } from "./health";
import { initialLive, type Lang, rememberLang, rememberLive, type Texts, texts } from "./i18n";
import { type JourneyTexts, journeyTexts } from "./journey-texts";
import { LoadingMallet, LoadingScreen } from "./LoadingMallet";
import { Mallet } from "./Mallet";
import { MapView } from "./MapView";
import { ModsView } from "./ModsView";
import { Overview, openFileDuels } from "./Overview";
import { reportFile } from "./PairReport";
import { PatchView } from "./PatchView";
import { Places } from "./Places";
import {
  choicesOf,
  installedKeys,
  mergeChoices,
  type PatchChoices,
  pairsOf,
  withoutDecidedPins,
  withoutMisspelt,
} from "./patch";
import { pinTexts } from "./pins";
import { Review } from "./Review";
import { useReviewTexts } from "./review-texts";
import { SettingsView } from "./Settings";
import { TasksPage } from "./Tasks";
import { ToolboxView } from "./Toolbox";

interface AppProps {
  api: Api;
  startLang: Lang;
}

/**
 * The pages of the window (ADR 0048): all games, and the pages of the game
 * in use, starting with its overview. Assets is the home of the duels
 * about files, with the comparison stage and the ranking.
 */
type ViewName =
  | "games"
  | "overview"
  | "review"
  | "tasks"
  | "collisions"
  | "patch"
  | "map"
  | "assets"
  | "catalogue"
  | "mods"
  | "toolbox"
  | "settings"
  | "places";

/** How often the window looks whether the setup changed, in milliseconds. */
const WATCH_EVERY = 5000;

/** How often the toolbox is looked at again, in milliseconds. */
const TOOLBOX_EVERY = 15000;

/** How long the patch's choices rest before they are saved, in milliseconds. */
const SAVE_CHOICES_AFTER = 400;

export default function App({ api, startLang }: AppProps) {
  const [lang, setLang] = useState<Lang>(startLang);
  const [view, setView] = useState<ViewName>("overview");
  const [places, setPlaces] = useState<RenderedPlaces | null>(null);
  const [report, setReport] = useState<RenderedReport | null>(null);
  const reportRef = useRef(report);
  reportRef.current = report;
  // What became of pins when their spots were decided, shown a moment.
  const [pinNote, setPinNote] = useState<number | null>(null);
  const pinNoteTimer = useRef<number | undefined>(undefined);
  // Whether the collisions are to open with only the pinned spots.
  const [pinnedAsked, setPinnedAsked] = useState(false);
  const [link, setLink] = useState<RenderedLink | null>(null);
  const [ignored, setIgnoredList] = useState<Set<string>>(new Set());
  const [scanning, setScanning] = useState(false);
  const [changed, setChanged] = useState(false);
  const [failure, setFailure] = useState<string | null>(null);
  const [focus, setFocus] = useState<MapSpot | null>(null);
  // The spot to open the collisions' worklist with.
  const [workSpot, setWorkSpot] = useState<MapSpot | null>(null);
  const [toolbox, setToolbox] = useState<RenderedToolbox | null>(null);
  const [liveWanted, setLiveWanted] = useState(initialLive);
  const games = useGames(api);
  // What the user chose for the patch is kept in the settings. Where nothing
  // is kept yet, it starts from the patch in the game, once; later scans
  // keep what the user chose since.
  const [patchChoices, setPatchChoicesNow] = useState<PatchChoices | null>(null);
  // What the settings keep: undefined until read, null if nothing.
  const [storedChoices, setStoredChoices] = useState<PatchChoices | null | undefined>(undefined);
  useEffect(() => {
    api
      .patchChoices()
      .then(setStoredChoices)
      .catch(() => setStoredChoices(null));
  }, [api]);
  // Saving waits until the choices rest a moment, and saves go out one
  // after the other, so that the last choice is the one kept.
  const saveTimer = useRef<number | undefined>(undefined);
  const unsaved = useRef<PatchChoices | null>(null);
  const saving = useRef<Promise<void>>(Promise.resolve());
  const saveChoices = useCallback(() => {
    window.clearTimeout(saveTimer.current);
    const choices = unsaved.current;
    if (choices === null) {
      return;
    }
    unsaved.current = null;
    saving.current = saving.current
      .then(() => api.setPatchChoices(choices))
      .catch((error: unknown) => setFailure(String(error)));
  }, [api]);
  const setPatchChoices = useCallback(
    (wanted: PatchChoices) => {
      // A pin marks a spot to solve later; once the spot is decided, the
      // pin has done its work and goes, with a short note (ADR 0050).
      const { choices: next, cleared } = withoutDecidedPins(
        wanted,
        spotsByFinding(reportRef.current),
      );
      if (cleared > 0) {
        setPinNote(cleared);
        window.clearTimeout(pinNoteTimer.current);
        pinNoteTimer.current = window.setTimeout(() => setPinNote(null), 6000);
      }
      setPatchChoicesNow(next);
      unsaved.current = next;
      window.clearTimeout(saveTimer.current);
      saveTimer.current = window.setTimeout(saveChoices, SAVE_CHOICES_AFTER);
    },
    [saveChoices],
  );
  // A choice made just before the window closes is still kept.
  useEffect(() => {
    window.addEventListener("pagehide", saveChoices);
    return () => window.removeEventListener("pagehide", saveChoices);
  }, [saveChoices]);
  const [decisions, setDecisions] = useState<Record<string, number>>({});
  useEffect(() => {
    api
      .decisions()
      .then(setDecisions)
      .catch(() => {});
  }, [api]);
  const decide = useCallback(
    (id: string, side: number | null) => {
      api
        .setDecision(id, side)
        .then(setDecisions)
        .catch((error: unknown) => setFailure(String(error)));
    },
    [api],
  );
  const [ranking, setRankingList] = useState<string[]>([]);
  useEffect(() => {
    api
      .ranking()
      .then(setRankingList)
      .catch(() => {});
  }, [api]);
  const setRanking = useCallback(
    (next: string[]) => {
      setRankingList(next);
      api
        .setRanking(next)
        .then(setRankingList)
        .catch((error: unknown) => setFailure(String(error)));
    },
    [api],
  );
  // The duel the assets page is to open at.
  const [compareAt, setCompareAt] = useState<string | null>(null);
  // Where the collisions and the map are to open, when another page led there.
  const [pairAt, setPairAt] = useState<[string, string] | null>(null);
  const [mapMod, setMapMod] = useState<string | null>(null);
  useEffect(() => {
    if (patchChoices !== null || storedChoices === undefined || report === null) {
      return;
    }
    // What the installed patch hides counts as decided; what the user chose
    // since, and kept, wins over it.
    const installed = choicesOf(
      pairsOf(report, ignored),
      installedKeys(report),
      report.installedPatch?.objects ?? [],
    );
    // Objects an earlier reading of the patch kept under another spelling go.
    setPatchChoicesNow(
      withoutMisspelt(mergeChoices(installed, storedChoices ?? {}), pairsOf(report, ignored)),
    );
  }, [patchChoices, storedChoices, report, ignored]);
  const t = texts[lang];
  const j = journeyTexts[lang];

  const refreshToolbox = useCallback(() => {
    api
      .toolbox(lang)
      .then(setToolbox)
      .catch(() => {});
  }, [api, lang]);

  // The toolbox changes outside Mod Mallet: Vortex is started or closed,
  // a library is installed. It is looked at again now and then.
  useEffect(() => {
    refreshToolbox();
    const timer = setInterval(() => {
      if (!document.hidden) {
        refreshToolbox();
      }
    }, TOOLBOX_EVERY);
    return () => clearInterval(timer);
  }, [refreshToolbox]);

  const changeLive = (next: boolean) => {
    setLiveWanted(next);
    rememberLive(next);
  };

  useEffect(() => {
    document.documentElement.lang = lang;
    rememberLang(lang);
  }, [lang]);

  // When the app opens, and again in the new language whenever the language
  // changes: the places, the last scan and the state of the link.
  useEffect(() => {
    let outdated = false;
    const unlessOutdated =
      <T,>(apply: (value: T) => void) =>
      (value: T) => {
        if (!outdated) {
          apply(value);
        }
      };
    const failed = unlessOutdated((error: unknown) => setFailure(String(error)));

    api.places(lang).then(unlessOutdated(setPlaces)).catch(failed);
    api
      .lastReport(lang)
      .then(unlessOutdated((last) => last !== null && setReport(last)))
      .catch(failed);
    api
      .managerLink(lang)
      .then(unlessOutdated(setLink))
      .catch(() => {});
    api
      .ignored()
      .then(unlessOutdated((ids: string[]) => setIgnoredList(new Set(ids))))
      .catch(() => {});
    return () => {
      outdated = true;
    };
  }, [api, lang]);

  const refreshLink = useCallback(() => {
    api
      .managerLink(lang)
      .then(setLink)
      .catch(() => {});
  }, [api, lang]);

  // What the setup looked like at the last scan, to notice a change.
  const fingerprint = useRef<string | null>(null);
  const scanningNow = useRef(false);

  const scan = useCallback(async () => {
    if (scanningNow.current) {
      return;
    }
    scanningNow.current = true;
    setScanning(true);
    setFailure(null);
    try {
      fingerprint.current = await api.setupFingerprint().catch(() => null);
      setReport(await api.scan(lang));
    } catch (error: unknown) {
      setFailure(String(error));
    } finally {
      scanningNow.current = false;
      setScanning(false);
      setChanged(false);
      refreshLink();
    }
  }, [api, lang, refreshLink]);

  // Live: when a file the scan starts from changes, for example because
  // Vortex deployed, scan again. A scan only reads, so it is safe to do
  // without asking; it is what the person would do next anyway.
  const confirmed = places?.confirmed === true;
  const live = confirmed && liveWanted;
  useEffect(() => {
    if (!live) {
      return;
    }
    const timer = setInterval(async () => {
      if (scanningNow.current || document.hidden) {
        return;
      }
      const now = await api.setupFingerprint().catch(() => null);
      if (now === null) {
        return;
      }
      if (fingerprint.current === null) {
        fingerprint.current = now;
      } else if (now !== fingerprint.current) {
        setChanged(true);
        await scan();
      }
    }, WATCH_EVERY);
    return () => clearInterval(timer);
  }, [api, live, scan]);

  const confirmPlaces = async (named: NamedPlaces) => {
    try {
      setPlaces(await api.confirmPlaces(named, lang));
      setView("overview");
      await scan();
    } catch (error: unknown) {
      setFailure(String(error));
    }
  };

  const setIgnored = useCallback(
    (id: string, aside: boolean) => {
      api
        .setIgnored(id, aside)
        .then((ids) => setIgnoredList(new Set(ids)))
        .catch((error: unknown) => setFailure(String(error)));
    },
    [api],
  );

  // Going to a page: whatever another page asked it to open with is
  // forgotten.
  const go = useCallback((next: ViewName) => {
    setFocus(null);
    setWorkSpot(null);
    setPairAt(null);
    setMapMod(null);
    setCompareAt(null);
    setPinnedAsked(false);
    setView(next);
  }, []);

  const showPinned = useCallback(() => {
    go("collisions");
    setPinnedAsked(true);
  }, [go]);

  const showPatch = useCallback(() => go("patch"), [go]);

  const openCompare = useCallback(
    (duel: string | null) => {
      go("assets");
      setCompareAt(duel);
    },
    [go],
  );

  const showOnMap = useCallback(
    (spot: MapSpot) => {
      go("map");
      setFocus(spot);
    },
    [go],
  );

  const showToolbox = useCallback(() => go("toolbox"), [go]);

  const showModOnMap = useCallback(
    (mod: string) => {
      go("map");
      setMapMod(mod);
    },
    [go],
  );

  const decideSpot = useCallback(
    (spot: MapSpot) => {
      go("collisions");
      setWorkSpot(spot);
    },
    [go],
  );

  const openPair = useCallback(
    (first: string, second: string) => {
      go("collisions");
      setPairAt([first, second]);
    },
    [go],
  );

  // Creating the patch also writes the report of what it does next to it
  // (ADR 0050), from the decisions as they are at that moment.
  const reportNow = useRef({ report, ignored, patchChoices, lang });
  reportNow.current = { report, ignored, patchChoices, lang };
  const game = games.choice?.active ?? "";
  const sessionApi = useMemo<Api>(
    () => ({
      ...api,
      createPatch: async (objects) => {
        const path = await api.createPatch(objects);
        const now = reportNow.current;
        if (now.report !== null) {
          await api
            .savePatchReport(
              reportFile(now.report, now.ignored, now.patchChoices ?? {}, game, now.lang),
            )
            .catch(() => {});
        }
        return path;
      },
    }),
    [api, game],
  );
  const session = useMemo<Session>(
    () => ({
      api: sessionApi,
      lang,
      game,
      t,
      rescan: scan,
      linkChanged: refreshLink,
      ignored,
      setIgnored,
      mods: modsByName(report?.mods ?? []),
      spots: spotsByFinding(report),
      showOnMap,
      decideSpot,
      patchChoices: patchChoices ?? {},
      setPatchChoices,
      showPatch,
      decisions,
      decide,
      ranking,
      setRanking,
      openCompare,
      openPair,
      pairAt,
      showModOnMap,
      mapMod,
      showToolbox,
      showPinned,
      pinnedAsked,
    }),
    [
      sessionApi,
      lang,
      game,
      t,
      scan,
      refreshLink,
      ignored,
      setIgnored,
      report,
      showOnMap,
      decideSpot,
      patchChoices,
      setPatchChoices,
      showPatch,
      decisions,
      decide,
      ranking,
      setRanking,
      openCompare,
      openPair,
      pairAt,
      showModOnMap,
      mapMod,
      showToolbox,
      showPinned,
      pinnedAsked,
    ],
  );
  const counts = useMemo(
    () => (report === null ? null : countFindings(report.findings, ignored)),
    [report, ignored],
  );
  const health = useMemo(
    () => (report === null ? null : healthOf(report, ignored, patchChoices ?? {})),
    [report, ignored, patchChoices],
  );

  // Nothing is read before the user has seen where Mod Mallet looks.
  const askFirst = places !== null && !places.confirmed;
  const current: ViewName = askFirst ? "places" : view;

  return (
    <SessionContext value={session}>
      <div className="shell">
        <Sidebar
          view={current}
          onView={go}
          disabled={askFirst}
          counts={counts}
          health={health}
          report={report}
          link={link}
          toolbox={toolbox}
          t={t}
          j={j}
          gameCard={
            <GameCard
              games={games}
              current={current === "overview"}
              disabled={askFirst}
              onOpen={() => go("overview")}
              t={t}
              j={j}
            >
              {!askFirst && (
                <ScanBox
                  scannedAt={report?.scannedAt ?? null}
                  live={live}
                  scanning={scanning}
                  onScan={scan}
                  lang={lang}
                  t={t}
                />
              )}
            </GameCard>
          }
        />
        <main className="content" data-view={current}>
          {/* The sidebar says where the user is; only the first run needs a title. */}
          {current === "places" && (
            <header className="pagehead">
              <h1>{askFirst ? t.placesTitleFirst : t.placesTitle}</h1>
            </header>
          )}

          {scanning && report !== null && (
            <p className="progress" role="status">
              <span className="bar" aria-hidden="true" />
              <LoadingMallet size={28} />
              {changed ? t.changedRescanning : t.scanningNote}
            </p>
          )}
          {pinNote !== null && (
            <p className="pin-note" role="status">
              {pinTexts[lang].cleared(pinNote)}
            </p>
          )}
          {failure !== null && (
            <p className="failure" role="alert">
              <strong>{t.scanFailed}:</strong> {failure}
            </p>
          )}

          {current === "places" && places !== null ? (
            <Places
              places={places}
              t={t}
              busy={scanning}
              onConfirm={confirmPlaces}
              onCancel={askFirst ? undefined : () => go("overview")}
            />
          ) : current === "games" ? (
            <AllGames
              games={games}
              report={report}
              health={health}
              scanning={scanning}
              onScan={scan}
              onOpen={go}
            />
          ) : current === "overview" ? (
            <Overview
              games={games}
              report={report}
              health={health}
              toolbox={toolbox}
              scanning={scanning}
              onScan={scan}
              onOpen={go}
            />
          ) : current === "toolbox" ? (
            <ToolboxView toolbox={toolbox} refresh={refreshToolbox} />
          ) : current === "settings" ? (
            <SettingsView
              places={places}
              busy={scanning}
              onConfirmPlaces={confirmPlaces}
              onLang={setLang}
              live={liveWanted}
              onLive={changeLive}
            />
          ) : report === null && scanning ? (
            // The first scan: nothing to show yet but that Mod Mallet works.
            <LoadingScreen title={t.scanning} note={t.scanningNote} />
          ) : report === null ? (
            places !== null && (
              <section className="welcome">
                <p>{t.noScanYet}</p>
                <button type="button" className="primary" onClick={scan}>
                  {t.scan}
                </button>
              </section>
            )
          ) : current === "review" ? (
            <Review report={report} game={games.choice?.active ?? ""} onOpen={go} />
          ) : current === "collisions" ? (
            <Collisions report={report} start={workSpot} />
          ) : current === "assets" ? (
            <CompareView report={report} start={compareAt} />
          ) : current === "catalogue" ? (
            <CatalogueView scannedAt={report.scannedAt} />
          ) : current === "patch" ? (
            <PatchView report={report} />
          ) : current === "map" ? (
            <MapView map={report.map} headlines={headlinesOf(report)} focus={focus} />
          ) : current === "mods" ? (
            <ModsView mods={report.mods} findings={report.findings} />
          ) : (
            <TasksPage report={report} />
          )}
        </main>
      </div>
    </SessionContext>
  );
}

function spotsByFinding(report: RenderedReport | null): Map<string, MapSpot[]> {
  const found = new Map<string, MapSpot[]>();
  for (const spot of report?.map.spots ?? []) {
    found.set(spot.finding, [...(found.get(spot.finding) ?? []), spot]);
  }
  return found;
}

function headlinesOf(report: RenderedReport): Map<string, string> {
  return new Map(report.findings.map((rendered) => [rendered.finding.id, rendered.headline]));
}

interface NavEntry {
  name: ViewName;
  label: string;
  icon: ReactNode;
  /** A number, or a dot for something waiting without a count. */
  badge?: number | "dot" | null;
  /** What the badge means, for a dot. */
  badgeLabel?: string;
}

interface SidebarProps {
  view: ViewName;
  onView: (view: ViewName) => void;
  disabled: boolean;
  counts: Counts | null;
  health: Health | null;
  report: RenderedReport | null;
  link: RenderedLink | null;
  toolbox: RenderedToolbox | null;
  t: Texts;
  j: JourneyTexts;
  /** The game in use, with scanning. */
  gameCard: ReactNode;
}

/**
 * The way to every page (ADR 0048): back to all games, the game in use,
 * its pages, and the toolbox and settings beside them. Each badge counts
 * what its page lists first, so that the numbers agree.
 */
function Sidebar({
  view,
  onView,
  disabled,
  counts,
  health,
  report,
  link,
  toolbox,
  t,
  j,
  gameCard,
}: SidebarProps) {
  const reviewTexts = useReviewTexts();
  const { decisions, lang } = useSession();
  const c = catalogueTexts[lang];
  const assets = report === null ? null : openFileDuels(report, decisions);
  // Two kinds of collisions side by side (ADR 0054): placed objects on the
  // map, and files of mods for the same thing.
  const game: NavEntry[] = [
    { name: "overview", label: j.navOverview, icon: <IconOverview /> },
    { name: "review", label: reviewTexts.nav, icon: <IconReview /> },
    { name: "tasks", label: t.viewTasks, icon: <IconTasks />, badge: counts?.tasks },
    {
      name: "collisions",
      label: c.navMapCollisions,
      icon: <IconCollisions />,
      badge: health?.spots.open,
    },
    { name: "assets", label: c.navAssetCollisions, icon: <IconAssets />, badge: assets },
    {
      name: "patch",
      label: t.viewPatch,
      icon: <IconPatch />,
      badge: (health?.spots.waiting ?? 0) > 0 ? "dot" : null,
      badgeLabel: health === null ? undefined : j.spotsWaiting(health.spots.waiting),
    },
    { name: "map", label: t.viewMap, icon: <IconMap /> },
    { name: "catalogue", label: c.nav, icon: <IconCatalogue /> },
    { name: "mods", label: t.viewMods, icon: <IconMods /> },
  ];
  const beside: NavEntry[] = [
    { name: "toolbox", label: t.viewToolbox, icon: <IconToolbox />, badge: toolbox?.attention },
    { name: "settings", label: t.viewSettings, icon: <IconSettings /> },
  ];
  const navButton = (item: NavEntry) => (
    <button
      type="button"
      key={item.name}
      className="nav-item"
      aria-current={view === item.name ? "page" : undefined}
      disabled={disabled && item.name !== "settings"}
      onClick={() => onView(item.name)}
    >
      {item.icon}
      <span className="nav-label">{item.label}</span>
      {item.badge === "dot" ? (
        <span
          className="badge-dot"
          role="img"
          aria-label={item.badgeLabel}
          title={item.badgeLabel}
        />
      ) : (
        item.badge != null && item.badge > 0 && <span className="badge">{item.badge}</span>
      )}
    </button>
  );

  return (
    <aside className="sidebar">
      <div className="brand">
        <Mallet />
        {/* The name leads to the start page with all games; the logo is only for fun. */}
        <button
          type="button"
          className="brand-name brand-home"
          aria-current={view === "games" ? "page" : undefined}
          title={j.brandHint}
          disabled={disabled}
          onClick={() => onView("games")}
        >
          Mod Mallet
        </button>
      </div>
      {gameCard}
      <nav>{game.map(navButton)}</nav>
      <nav className="nav-bottom">{beside.map(navButton)}</nav>
      <div className="sidebar-foot">
        <ManagerStatus
          toolbox={toolbox}
          link={link}
          disabled={disabled}
          onOpen={() => onView("toolbox")}
          t={t}
        />
      </div>
    </aside>
  );
}

interface GameCardProps {
  games: Games;
  /** Whether the overview is the page shown. */
  current: boolean;
  disabled: boolean;
  onOpen: () => void;
  t: Texts;
  j: JourneyTexts;
  /** Scanning, below the game's name. */
  children: ReactNode;
}

/** The game in use: its picture and name, which lead to its overview, and scanning. */
function GameCard({ games, current, disabled, onOpen, t, j, children }: GameCardProps) {
  const activeId = games.choice?.active;
  const art = useGameArt(activeId === undefined ? [] : [activeId]);
  const picture = activeId === undefined ? undefined : art[activeId];
  const name = t.games.find((game) => game.id === activeId)?.name ?? "";
  return (
    <div className="side-game">
      <button
        type="button"
        className="side-game-head"
        aria-current={current ? "page" : undefined}
        title={j.gameCardHint}
        disabled={disabled || activeId === undefined}
        onClick={onOpen}
      >
        <span
          className="side-game-art"
          style={picture === undefined ? undefined : { backgroundImage: `url(${picture})` }}
          aria-hidden="true"
        >
          {picture === undefined && name.slice(0, 1)}
        </span>
        <span className="side-game-name">{name}</span>
      </button>
      {children}
    </div>
  );
}

interface ScanBoxProps {
  scannedAt: number | null;
  live: boolean;
  scanning: boolean;
  onScan: () => void;
  lang: Lang;
  t: Texts;
}

/** The scan button, when the last scan was, and whether Mod Mallet scans by itself. */
function ScanBox({ scannedAt, live, scanning, onScan, lang, t }: ScanBoxProps) {
  return (
    <div className="scan-box">
      <button type="button" className="primary scan-button" onClick={onScan} disabled={scanning}>
        {scanning ? t.scanning : t.scan}
      </button>
      {scannedAt !== null && (
        <span className="scan-when" title={new Date(scannedAt * 1000).toLocaleString(lang)}>
          {t.lastScan}:{" "}
          {new Date(scannedAt * 1000).toLocaleString(lang, {
            dateStyle: "short",
            timeStyle: "short",
          })}
        </span>
      )}
      {live && (
        <span className="live-dot" title={t.liveNote}>
          {t.liveShort}
        </span>
      )}
    </div>
  );
}

interface ManagerStatusProps {
  toolbox: RenderedToolbox | null;
  link: RenderedLink | null;
  disabled: boolean;
  onOpen: () => void;
  t: Texts;
}

/**
 * The mod manager at a glance: whether it runs, and how Mod Mallet's link
 * into it stands. Whichever tool of the toolbox is a mod manager is shown,
 * so another manager needs no change here.
 */
function ManagerStatus({ toolbox, link, disabled, onOpen, t }: ManagerStatusProps) {
  const manager = toolbox?.tools.find((tool) => tool.tool.group === "mod-manager");
  if (manager === undefined) {
    return null;
  }
  const bridge = toolbox?.tools.find((tool) => tool.tool.group === "manager-link");
  const running = manager.tool.state === "ready";
  const state =
    manager.tool.state === "missing"
      ? t.managerMissing
      : running
        ? t.managerRunning
        : t.managerOffline;
  let linkLine: string | null = null;
  let tone: "ok" | "warning" | "muted" = running ? "ok" : "warning";
  if (bridge !== undefined && manager.tool.state !== "missing") {
    if (bridge.tool.state === "missing") {
      linkLine = t.managerLinkMissing;
      tone = "warning";
    } else if (link?.restartNeeded) {
      linkLine = t.managerLinkRestart;
      tone = "warning";
    } else if (bridge.tool.state === "outdated") {
      linkLine = t.managerLinkOutdated;
      tone = "warning";
    } else if (running) {
      linkLine = t.managerLinkReady(bridge.tool.version ?? null);
    }
  }
  if (manager.tool.state === "missing") {
    tone = "muted";
  }
  return (
    <button
      type="button"
      className="manager-status"
      data-tone={tone}
      title={link?.message}
      disabled={disabled}
      onClick={onOpen}
    >
      <span className="manager-dot" aria-hidden="true" />
      <span className="manager-text">
        <span>
          <strong>{manager.name}</strong> · {state}
        </span>
        {linkLine !== null && <span className="manager-link">{linkLine}</span>}
      </span>
    </button>
  );
}

// Icons of the sidebar: simple strokes in the text colour.

function Icon({ children }: { children: ReactNode }) {
  return (
    <svg
      className="icon"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.8"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      {children}
    </svg>
  );
}

function IconTasks() {
  return (
    <Icon>
      <path d="M9 6h11M9 12h11M9 18h11" />
      <path d="M4 6l1 1 2-2M4 12l1 1 2-2M4 18l1 1 2-2" />
    </Icon>
  );
}

function IconCollisions() {
  return (
    <Icon>
      <rect x="3" y="7" width="10" height="10" rx="1" />
      <rect x="11" y="4" width="10" height="10" rx="1" />
    </Icon>
  );
}

function IconPatch() {
  return (
    <Icon>
      <path d="M14 4l6 6-10 10H4v-6z" />
      <path d="M12 6l6 6" />
    </Icon>
  );
}

function IconMap() {
  return (
    <Icon>
      <path d="M3 6l6-2 6 2 6-2v14l-6 2-6-2-6 2z" />
      <path d="M9 4v14M15 6v14" />
    </Icon>
  );
}

function IconAssets() {
  return (
    <Icon>
      <rect x="3" y="4" width="18" height="16" rx="2" />
      <circle cx="9" cy="10" r="2" />
      <path d="M21 16l-5-5-9 9" />
    </Icon>
  );
}

function IconCatalogue() {
  return (
    <Icon>
      <path d="M5 4h4v16H5zM10 4h4v16h-4z" />
      <path d="M15.5 5l3.8-1 3 15.5-3.8 1z" />
    </Icon>
  );
}

function IconMods() {
  return (
    <Icon>
      <path d="M12 3l8 4-8 4-8-4z" />
      <path d="M4 12l8 4 8-4M4 17l8 4 8-4" />
    </Icon>
  );
}

function IconReview() {
  return (
    <Icon>
      <path d="M4 6h9M4 12h7M4 18h9" />
      <path d="M16 9l4 3-4 3" />
    </Icon>
  );
}

function IconOverview() {
  return (
    <Icon>
      <rect x="3" y="3" width="8" height="8" rx="1.5" />
      <rect x="13" y="3" width="8" height="5" rx="1.5" />
      <rect x="13" y="10" width="8" height="11" rx="1.5" />
      <rect x="3" y="13" width="8" height="8" rx="1.5" />
    </Icon>
  );
}

function IconToolbox() {
  return (
    <Icon>
      <rect x="3" y="8" width="18" height="12" rx="1.5" />
      <path d="M9 8V5h6v3M3 13h18M10 13v2h4v-2" />
    </Icon>
  );
}

function IconSettings() {
  return (
    <Icon>
      <circle cx="12" cy="12" r="3" />
      <path d="M12 3v2M12 19v2M3 12h2M19 12h2M5.6 5.6l1.4 1.4M17 17l1.4 1.4M5.6 18.4L7 17M17 7l1.4-1.4" />
    </Icon>
  );
}
