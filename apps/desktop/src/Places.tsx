import { type FormEvent, useState } from "react";
import type { NamedPlaces } from "./api";
import type { RenderedLocation, RenderedPlaces } from "./generated/rendered-places";
import type { Texts } from "./i18n";

interface PlacesProps {
  places: RenderedPlaces;
  t: Texts;
  busy: boolean;
  /** Called with the places the user named. */
  onConfirm: (named: NamedPlaces) => void;
  /** Absent on the first start, where there is nothing to go back to. */
  onCancel?: () => void;
}

/**
 * The places a scan reads, shown before anything is read (ADR 0018).
 *
 * Each place comes with the rule that found it. The user agrees or enters
 * another path. A field that is left as suggested stays "found by its
 * rule": only what was typed is remembered as the user's choice.
 */
export function Places({ places, t, busy, onConfirm, onCancel }: PlacesProps) {
  const first = !places.confirmed;
  const [paths, setPaths] = useState<Record<string, string>>(() =>
    Object.fromEntries(places.places.map(({ location }) => [location.role, location.path])),
  );

  const submit = (event: FormEvent) => {
    event.preventDefault();
    const named: NamedPlaces = {};
    for (const { location } of places.places) {
      const path = (paths[location.role] ?? "").trim();
      const chosen = location.origin === "given" || path !== location.path;
      if (chosen && path !== "") {
        named[location.role] = path;
      }
    }
    onConfirm(named);
  };

  return (
    <section className="places">
      {first && <p className="places-text">{t.placesIntro}</p>}
      <form onSubmit={submit}>
        {places.places.map((place) => (
          <PlaceField
            key={place.location.role}
            place={place}
            value={paths[place.location.role] ?? ""}
            onChange={(path) => setPaths({ ...paths, [place.location.role]: path })}
            t={t}
          />
        ))}
        <div className="buttons">
          <button type="submit" className="primary" disabled={busy}>
            {first ? t.placesConfirmFirst : t.placesSave}
          </button>
          {onCancel !== undefined && (
            <button type="button" onClick={onCancel} disabled={busy}>
              {t.cancel}
            </button>
          )}
        </div>
      </form>
    </section>
  );
}

interface PlaceFieldProps {
  place: RenderedLocation;
  value: string;
  onChange: (path: string) => void;
  t: Texts;
}

function PlaceField({ place, value, onChange, t }: PlaceFieldProps) {
  const { location } = place;
  const unchanged = value.trim() === location.path;
  // What is said about a path only holds for the path it was said about.
  const state = !unchanged
    ? null
    : location.found
      ? place.originLabel
      : location.path === ""
        ? t.placeUnknown
        : `${t.notFound} · ${place.originLabel}`;

  return (
    <label className="place" data-found={unchanged ? location.found : undefined}>
      <span className="place-role">{place.roleLabel}</span>
      <input
        type="text"
        value={value}
        spellCheck={false}
        placeholder={t.placeAutomatic}
        autoComplete="off"
        onChange={(event) => onChange(event.target.value)}
      />
      <span className="place-origin">{state ?? " "}</span>
    </label>
  );
}
