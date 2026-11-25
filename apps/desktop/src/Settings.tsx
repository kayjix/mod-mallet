import { type ReactNode, useEffect, useState } from "react";
import { useSession } from "./Actions";
import type { NamedPlaces } from "./api";
import type { RenderedPlaces } from "./generated/rendered-places";
import { applyTheme, initialTheme, type Lang, languages, type Theme } from "./i18n";
import { InfoTip } from "./info-tip";
import { Places } from "./Places";
import { preferenceTexts } from "./preference-texts";
import { PreferencesSection } from "./preferences-settings";
import { graphicsCard } from "./render-stats";
import { settingsTexts } from "./settings-texts";
import { PageHeader, Segmented, Tabs } from "./ui/kit";

interface SettingsProps {
  places: RenderedPlaces | null;
  busy: boolean;
  onConfirmPlaces: (named: NamedPlaces, manager: string | null) => void;
  onLang: (lang: Lang) => void;
  live: boolean;
  onLive: (live: boolean) => void;
}

type Tab = "general" | "places" | "preferences" | "about";

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
  const s = settingsTexts[lang];
  const [tab, setTab] = useState<Tab>("general");
  const [theme, setTheme] = useState<Theme>(initialTheme);
  const [logFile, setLogFile] = useState<string | null>(null);
  // Asked once: a drawing context of its own tells which card draws.
  const [card] = useState(graphicsCard);

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
    { id: "general", label: s.settingsTabGeneral },
    { id: "places", label: s.settingsTabPlaces },
    { id: "preferences", label: preferenceTexts[lang].tab },
    { id: "about", label: s.settingsTabAbout },
  ];
  const themes: { id: Theme; label: string }[] = [
    { id: "system", label: s.themeSystem },
    { id: "light", label: s.themeLight },
    { id: "dark", label: s.themeDark },
  ];

  return (
    <div className="page-narrow">
      <PageHeader
        tabs={<Tabs label={t.viewSettings} items={tabs} value={tab} onChange={setTab} />}
      />

      {tab === "general" && (
        <div className="stack">
          <section className="panel">
            <div className="panel-body">
              <Row label={s.settingsLanguage}>
                <select
                  aria-label={s.settingsLanguage}
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
              <Row label={s.settingsTheme}>
                <Segmented
                  label={s.settingsTheme}
                  items={themes}
                  value={theme}
                  onChange={changeTheme}
                />
              </Row>
              <Row label={s.settingsLive} note={t.liveNote}>
                <label className="check">
                  <input
                    type="checkbox"
                    checked={live}
                    onChange={(event) => onLive(event.target.checked)}
                  />
                  {s.settingsLiveLabel}
                </label>
              </Row>
              <Row label={s.settingsLog} note={s.settingsLogNote}>
                {logFile !== null && <code className="path">{logFile}</code>}
                <button type="button" onClick={() => api.openFolder("logs")}>
                  {s.openLog}
                </button>
              </Row>
            </div>
          </section>
        </div>
      )}

      {tab === "places" && places !== null && (
        <section className="panel">
          <div className="panel-head">
            <h2>{s.settingsPlaces}</h2>
          </div>
          <div className="panel-body">
            <Places places={places} t={t} busy={busy} onConfirm={onConfirmPlaces} />
          </div>
        </section>
      )}

      {tab === "preferences" && <PreferencesSection />}

      {tab === "about" && (
        <section className="panel">
          <div className="panel-body">
            <div className="about">
              <strong>Mod Mallet</strong>
              <span className="brand-tagline">{s.tagline}</span>
              <p>{s.aboutText}</p>
              <p className="about-version">
                {s.aboutVersion(
                  __MALLET_VERSION__,
                  __MALLET_COMMIT__,
                  __MALLET_COMMIT_DATE__ === null
                    ? null
                    : new Date(__MALLET_COMMIT_DATE__).toLocaleDateString(lang),
                )}
              </p>
              <p className="about-version">{s.aboutGraphics(card.renderer, card.software)}</p>
            </div>
          </div>
        </section>
      )}
    </div>
  );
}
