import { type ReactNode, useCallback, useEffect, useMemo, useRef, useState } from "react";
import "./App.css";
import { type Session, SessionContext } from "./Actions";
import type { Api, NamedPlaces } from "./api";
import { type Counts, countFindings, type Filter, modsByName } from "./arrange";
import type { RenderedLink } from "./generated/rendered-link";
import type { RenderedPlaces } from "./generated/rendered-places";
import type { MapSpot, RenderedReport } from "./generated/rendered-report";
import type { RenderedToolbox } from "./generated/rendered-toolbox";
import { Home } from "./Home";
import { initialLive, type Lang, rememberLang, rememberLive, type Texts, texts } from "./i18n";
import { Mallet } from "./Mallet";
import { MapView } from "./MapView";
import { ModsView } from "./ModsView";
import { PatchView } from "./PatchView";
import { Places } from "./Places";
import { choicesOf, installedKeys, type PatchChoices, pairsOf } from "./patch";
import { SettingsView } from "./Settings";
import { TaskList } from "./Tasks";
import { ToolboxView } from "./Toolbox";

interface AppProps {
  api: Api;
  startLang: Lang;
}

/** The views of the sidebar. */
type ViewName =
  | "home"
  | "tasks"
  | "collisions"
  | "patch"
  | "map"
  | "mods"
  | "toolbox"
  | "settings"
  | "places";

/** How often the window looks whether the setup changed, in milliseconds. */
const WATCH_EVERY = 5000;

/** How often the toolbox is looked at again, in milliseconds. */
const TOOLBOX_EVERY = 15000;

export default function App({ api, startLang }: AppProps) {
  const [lang, setLang] = useState<Lang>(startLang);
  const [view, setView] = useState<ViewName>("home");
  const [places, setPlaces] = useState<RenderedPlaces | null>(null);
  const [report, setReport] = useState<RenderedReport | null>(null);
  const [link, setLink] = useState<RenderedLink | null>(null);
  const [ignored, setIgnoredList] = useState<Set<string>>(new Set());
  const [scanning, setScanning] = useState(false);
  const [changed, setChanged] = useState(false);
  const [failure, setFailure] = useState<string | null>(null);
  const [focus, setFocus] = useState<MapSpot | null>(null);
  const [toolbox, setToolbox] = useState<RenderedToolbox | null>(null);
  const [liveWanted, setLiveWanted] = useState(initialLive);
  // The patch's choice starts from the patch in the game, once; later scans
  // keep what the user chose since.
  const [patchChoices, setPatchChoices] = useState<PatchChoices | null>(null);
  useEffect(() => {
    if (patchChoices === null && report !== null) {
      setPatchChoices(choicesOf(pairsOf(report, ignored), installedKeys(report)));
    }
  }, [patchChoices, report, ignored]);
  const t = texts[lang];

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
      setView("tasks");
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

  const showPatch = useCallback(() => {
    setFocus(null);
    setView("patch");
  }, []);

  const showOnMap = useCallback((spot: MapSpot) => {
    setFocus(spot);
    setView("map");
  }, []);

  const session = useMemo<Session>(
    () => ({
      api,
      lang,
      t,
      rescan: scan,
      linkChanged: refreshLink,
      ignored,
      setIgnored,
      mods: modsByName(report?.mods ?? []),
      spots: spotsByFinding(report),
      showOnMap,
      patchChoices: patchChoices ?? {},
      setPatchChoices,
      showPatch,
    }),
    [
      api,
      lang,
      t,
      scan,
      refreshLink,
      ignored,
      setIgnored,
      report,
      showOnMap,
      patchChoices,
      showPatch,
    ],
  );
  const counts = useMemo(
    () => (report === null ? null : countFindings(report.findings, ignored)),
    [report, ignored],
  );

  // Nothing is read before the user has seen where Mod Mallet looks.
  const askFirst = places !== null && !places.confirmed;
  const current: ViewName = askFirst ? "places" : view;
  const titles: Record<ViewName, string> = {
    home: t.viewHome,
    tasks: t.viewTasks,
    collisions: t.viewCollisions,
    patch: t.viewPatch,
    map: t.viewMap,
    mods: t.viewMods,
    toolbox: t.viewToolbox,
    settings: t.viewSettings,
    places: askFirst ? t.placesTitleFirst : t.placesTitle,
  };

  return (
    <SessionContext value={session}>
      <div className="shell">
        <Sidebar
          view={current}
          onView={(next) => {
            setFocus(null);
            setView(next);
          }}
          disabled={askFirst}
          counts={counts}
          report={report}
          link={link}
          toolbox={toolbox}
          t={t}
        />
        <main className="content">
          <header className="pagehead">
            <h1>{titles[current]}</h1>
            {report !== null && !askFirst && (
              <span className="aside">
                {t.lastScan}: {new Date(report.scannedAt * 1000).toLocaleString(lang)}
              </span>
            )}
            {live && (
              <span className="live-dot" title={t.liveNote}>
                {t.live}
              </span>
            )}
            <span className="spacer" />
            {!askFirst && (
              <button type="button" className="primary" onClick={scan} disabled={scanning}>
                {scanning ? t.scanning : t.scan}
              </button>
            )}
          </header>

          {scanning && (
            <p className="progress" role="status">
              <span className="bar" aria-hidden="true" />
              {changed ? t.changedRescanning : t.scanningNote}
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
              onCancel={askFirst ? undefined : () => setView("tasks")}
            />
          ) : current === "home" ? (
            <Home
              report={report}
              counts={counts}
              scanning={scanning}
              onScan={scan}
              onOpen={(next) => {
                setFocus(null);
                setView(next);
              }}
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
          ) : report === null ? (
            places !== null &&
            !scanning && (
              <section className="welcome">
                <p>{t.noScanYet}</p>
                <button type="button" className="primary" onClick={scan}>
                  {t.scan}
                </button>
              </section>
            )
          ) : current === "collisions" ? (
            <TaskList findings={report.findings} filter="collisions" />
          ) : current === "patch" ? (
            <PatchView report={report} />
          ) : current === "map" ? (
            <MapView map={report.map} headlines={headlinesOf(report)} focus={focus} />
          ) : current === "mods" ? (
            <ModsView mods={report.mods} findings={report.findings} />
          ) : (
            counts !== null && <Report report={report} counts={counts} lang={lang} t={t} />
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

interface SidebarProps {
  view: ViewName;
  onView: (view: ViewName) => void;
  disabled: boolean;
  counts: Counts | null;
  report: RenderedReport | null;
  link: RenderedLink | null;
  toolbox: RenderedToolbox | null;
  t: Texts;
}

function Sidebar({ view, onView, disabled, counts, report, link, toolbox, t }: SidebarProps) {
  const open = counts === null ? null : counts.errors + counts.warnings;
  const collisions = counts === null ? null : counts.collisions;
  const items: { name: ViewName; label: string; icon: ReactNode; badge?: number | null }[] = [
    { name: "home", label: t.viewHome, icon: <IconHome /> },
    { name: "tasks", label: t.viewTasks, icon: <IconTasks />, badge: open },
    {
      name: "collisions",
      label: t.viewCollisions,
      icon: <IconCollisions />,
      badge: collisions,
    },
    { name: "patch", label: t.viewPatch, icon: <IconPatch /> },
    { name: "map", label: t.viewMap, icon: <IconMap /> },
    { name: "mods", label: t.viewMods, icon: <IconMods /> },
    { name: "toolbox", label: t.viewToolbox, icon: <IconToolbox /> },
  ];
  const navButton = (item: {
    name: ViewName;
    label: string;
    icon: ReactNode;
    badge?: number | null;
  }) => (
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
      {item.badge != null && item.badge > 0 && <span className="badge">{item.badge}</span>}
    </button>
  );
  const game = report?.game;
  const attention = toolbox?.attention ?? 0;
  const status =
    toolbox === null ? null : attention > 0 ? t.toolboxAttention(attention) : t.toolboxReady;

  return (
    <aside className="sidebar">
      <div className="brand">
        <Mallet />
        <span className="brand-name">Mod Mallet</span>
      </div>
      <GamePicker t={t} />
      <nav>{items.map(navButton)}</nav>
      <nav className="nav-bottom">
        {navButton({ name: "settings", label: t.viewSettings, icon: <IconSettings /> })}
      </nav>
      <div className="sidebar-foot">
        <ManagerStatus
          toolbox={toolbox}
          link={link}
          disabled={disabled}
          onOpen={() => onView("toolbox")}
          t={t}
        />
        {game != null && (
          <p className="game-version">
            {game.version ?? t.versionUnknown}
            {game.extenderVersion != null && ` · ${t.scriptExtender} ${game.extenderVersion}`}
          </p>
        )}
        {status !== null && (
          <button
            type="button"
            className="pill status-pill"
            data-state={attention > 0 ? "restart" : "ready"}
            title={link?.message}
            disabled={disabled}
            onClick={() => onView("toolbox")}
          >
            {status}
          </button>
        )}
      </div>
    </aside>
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

/**
 * The game Mod Mallet works on. Only Skyrim Special Edition so far; the
 * others are on the list so that it is clear where this is going.
 */
function GamePicker({ t }: { t: Texts }) {
  return (
    <label className="game-picker">
      <span className="brand-tagline">{t.tagline}</span>
      <select value={0} onChange={() => {}} aria-label={t.gameLabel}>
        {t.games.map((game, index) => (
          <option key={game.name} value={index} disabled={index > 0}>
            {game.quip === undefined ? game.name : `${game.name} – ${game.quip}`}
          </option>
        ))}
      </select>
    </label>
  );
}

interface ReportProps {
  report: RenderedReport;
  counts: Counts;
  lang: Lang;
  t: Texts;
}

function Report({ report, counts, lang, t }: ReportProps) {
  const open = counts.errors + counts.warnings;
  const [filter, setFilter] = useState<Filter>(open > 0 ? "open" : "info");

  return (
    <section aria-live="polite">
      <Overview
        report={report}
        counts={counts}
        filter={filter}
        onFilter={setFilter}
        lang={lang}
        t={t}
      />
      <TaskList findings={report.findings} filter={filter} />

      <details className="sources">
        <summary className="more">{t.whatWasRead}</summary>
        <dl>
          {report.locations.map(({ location, roleLabel, originLabel }) => (
            <div key={location.role}>
              <dt>{roleLabel}</dt>
              <dd>
                <code>{location.path}</code>
                <span className="origin">{location.found ? originLabel : t.notFound}</span>
              </dd>
            </div>
          ))}
        </dl>
      </details>
    </section>
  );
}

interface OverviewProps {
  report: RenderedReport;
  counts: Counts;
  filter: Filter;
  onFilter: (filter: Filter) => void;
  lang: Lang;
  t: Texts;
}

/**
 * The state of the setup at a glance: one sentence, then how much there is
 * of each sort. The numbers are also the switches of the list below.
 */
function Overview({ report, counts, filter, onFilter, t }: OverviewProps) {
  const open = counts.errors + counts.warnings;
  const state = counts.errors > 0 ? "error" : counts.warnings > 0 ? "warning" : "ok";
  const tiles: { filter: Filter; label: string; count: number; tone: string }[] = [
    { filter: "open", label: t.filterOpen, count: open, tone: "plain" },
    { filter: "error", label: t.filterFix, count: counts.errors, tone: "error" },
    { filter: "warning", label: t.filterCheck, count: counts.warnings, tone: "warning" },
    { filter: "direct", label: t.filterDirect, count: counts.direct, tone: "direct" },
    { filter: "info", label: t.filterNotes, count: counts.notes, tone: "info" },
  ];
  if (counts.ignored > 0) {
    tiles.push({
      filter: "ignored",
      label: t.filterIgnored,
      count: counts.ignored,
      tone: "plain",
    });
  }

  return (
    <section className="overview" data-state={state}>
      <h2>{open === 0 ? t.allGood : t.headline(counts.errors, counts.warnings)}</h2>
      <div className="tiles">
        {tiles.map((tile) => (
          <button
            type="button"
            key={tile.filter}
            className="tile"
            data-tone={tile.tone}
            aria-pressed={filter === tile.filter}
            onClick={() => onFilter(tile.filter)}
          >
            <strong>{tile.count}</strong>
            <span>{tile.label}</span>
          </button>
        ))}
      </div>
      <p className="meta">
        {t.scanStats({
          plugins: report.pluginsChecked,
          archives: report.archivesChecked,
          installers: report.installersChecked,
          meshes: report.meshesChecked,
        })}
      </p>
    </section>
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

function IconMods() {
  return (
    <Icon>
      <path d="M12 3l8 4-8 4-8-4z" />
      <path d="M4 12l8 4 8-4M4 17l8 4 8-4" />
    </Icon>
  );
}

function IconHome() {
  return (
    <Icon>
      <path d="M4 11l8-7 8 7v9h-5v-6H9v6H4z" />
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
