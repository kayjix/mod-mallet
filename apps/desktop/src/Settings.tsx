import { useEffect, useState } from "react";
import { useSession } from "./Actions";
import type { NamedPlaces } from "./api";
import type { RenderedPlaces } from "./generated/rendered-places";
import { type Lang, languages } from "./i18n";
import { Places } from "./Places";

interface SettingsProps {
  places: RenderedPlaces | null;
  busy: boolean;
  onConfirmPlaces: (named: NamedPlaces) => void;
  onLang: (lang: Lang) => void;
  live: boolean;
  onLive: (live: boolean) => void;
}

/** Language, live scans, the places Mod Mallet reads, and its log. */
export function SettingsView({
  places,
  busy,
  onConfirmPlaces,
  onLang,
  live,
  onLive,
}: SettingsProps) {
  const { t, api, lang } = useSession();
  const [logFile, setLogFile] = useState<string | null>(null);

  useEffect(() => {
    api
      .logFile()
      .then(setLogFile)
      .catch(() => {});
  }, [api]);

  return (
    <div className="settings">
      <section className="settings-group">
        <h2>{t.settingsLanguage}</h2>
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
      </section>

      <section className="settings-group">
        <h2>{t.settingsLive}</h2>
        <label className="check">
          <input
            type="checkbox"
            checked={live}
            onChange={(event) => onLive(event.target.checked)}
          />
          {t.settingsLiveLabel}
        </label>
        <p className="aside">{t.liveNote}</p>
      </section>

      <section className="settings-group">
        <h2>{t.settingsLog}</h2>
        <p className="aside">{t.settingsLogNote}</p>
        {logFile !== null && <code className="path">{logFile}</code>}
        <div>
          <button type="button" onClick={() => api.openFolder("logs")}>
            {t.openLog}
          </button>
        </div>
      </section>

      <section className="settings-group wide">
        <h2>{t.settingsPlaces}</h2>
        {places !== null && (
          <Places places={places} t={t} busy={busy} onConfirm={onConfirmPlaces} />
        )}
      </section>
    </div>
  );
}
