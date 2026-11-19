import { type ReactNode, useEffect, useState } from "react";
import { useSession } from "./Actions";
import type { NamedPlaces } from "./api";
import type { RenderedPlaces } from "./generated/rendered-places";
import { applyTheme, initialTheme, type Lang, languages, type Theme } from "./i18n";
import { InfoTip } from "./info-tip";
import { Places } from "./Places";

interface SettingsProps {
  places: RenderedPlaces | null;
  busy: boolean;
  onConfirmPlaces: (named: NamedPlaces) => void;
  onLang: (lang: Lang) => void;
  live: boolean;
  onLive: (live: boolean) => void;
}

type Tab = "general" | "places" | "about";

/** One setting: what it is on the left, the control on the right. */
function Row({ label, note, children }: { label: string; note?: string; children: ReactNode }) {
  return (
    <div className="row">
      <div className="row-label">
        <span>
          {label} {note !== undefined && <InfoTip text={note} />}
        </span>
      </div>
      <div className="row-control">{children}</div>
    </div>
  );
}

/**
 * The settings, in tabs: how the window looks and behaves, where Mod
 * Mallet reads, and what Mod Mallet is.
 */
export function SettingsView({
  places,
  busy,
  onConfirmPlaces,
  onLang,
  live,
  onLive,
}: SettingsProps) {
  const { t, api, lang } = useSession();
  const [tab, setTab] = useState<Tab>("general");
  const [theme, setTheme] = useState<Theme>(initialTheme);
  const [logFile, setLogFile] = useState<string | null>(null);

  useEffect(() => {
    api
      .logFile()
      .then(setLogFile)
      .catch(() => {});
  }, [api]);

  const changeTheme = (next: Theme) => {
    setTheme(next);
    applyTheme(next);
  };

  const tabs: { id: Tab; label: string }[] = [
    { id: "general", label: t.settingsTabGeneral },
    { id: "places", label: t.settingsTabPlaces },
    { id: "about", label: t.settingsTabAbout },
  ];
  const themes: { id: Theme; label: string }[] = [
    { id: "system", label: t.themeSystem },
    { id: "light", label: t.themeLight },
    { id: "dark", label: t.themeDark },
  ];

  return (
    <div className="page-narrow">
      <div className="tabs" role="tablist">
        {tabs.map((entry) => (
          <button
            type="button"
            role="tab"
            className="tab"
            key={entry.id}
            aria-selected={tab === entry.id}
            onClick={() => setTab(entry.id)}
          >
            {entry.label}
          </button>
        ))}
      </div>

      {tab === "general" && (
        <div className="stack">
          <section className="panel">
            <div className="panel-body">
              <Row label={t.settingsLanguage}>
                <select
                  aria-label={t.settingsLanguage}
                  value={lang}
                  onChange={(event) => onLang(event.target.value as Lang)}
                >
                  {Object.entries(languages).map(([code, name]) => (
                    <option key={code} value={code}>
                      {name}
                    </option>
                  ))}
                </select>
              </Row>
              <Row label={t.settingsTheme}>
                <div className="segmented-small">
                  {themes.map((entry) => (
                    <button
                      type="button"
                      key={entry.id}
                      className="segment"
                      aria-pressed={theme === entry.id}
                      onClick={() => changeTheme(entry.id)}
                    >
                      {entry.label}
                    </button>
                  ))}
                </div>
              </Row>
              <Row label={t.settingsLive} note={t.liveNote}>
                <label className="check">
                  <input
                    type="checkbox"
                    checked={live}
                    onChange={(event) => onLive(event.target.checked)}
                  />
                  {t.settingsLiveLabel}
                </label>
              </Row>
              <Row label={t.settingsLog} note={t.settingsLogNote}>
                {logFile !== null && <code className="path">{logFile}</code>}
                <button type="button" onClick={() => api.openFolder("logs")}>
                  {t.openLog}
                </button>
              </Row>
            </div>
          </section>
        </div>
      )}

      {tab === "places" && places !== null && (
        <section className="panel">
          <div className="panel-head">
            <h2>{t.settingsPlaces}</h2>
          </div>
          <div className="panel-body">
            <Places places={places} t={t} busy={busy} onConfirm={onConfirmPlaces} />
          </div>
        </section>
      )}

      {tab === "about" && (
        <section className="panel">
          <div className="panel-body">
            <div className="about">
              <strong>Mod Mallet</strong>
              <span className="brand-tagline">{t.tagline}</span>
              <p>{t.aboutText}</p>
              <p className="about-version">
                {t.aboutVersion(
                  __MALLET_VERSION__,
                  __MALLET_COMMIT__,
                  __MALLET_COMMIT_DATE__ === null
                    ? null
                    : new Date(__MALLET_COMMIT_DATE__).toLocaleDateString(lang),
                )}
              </p>
            </div>
          </div>
        </section>
      )}
    </div>
  );
}
