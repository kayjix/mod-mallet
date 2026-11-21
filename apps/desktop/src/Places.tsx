import { type FormEvent, useEffect, useState } from "react";
import "./managers.css";
import { useSession } from "./Actions";
import type { NamedPlaces } from "./api";
import type {
  KnownManager,
  ManagerChoice,
  RenderedLocation,
  RenderedPlaces,
} from "./generated/rendered-places";
import type { Texts } from "./i18n";
import { type ManagerTexts, managerTexts } from "./managers-texts";

interface PlacesProps {
  places: RenderedPlaces;
  t: Texts;
  busy: boolean;
  /**
   * Called with the places the user named and the mod manager the places
   * are for, or null where the page knows of none.
   */
  onConfirm: (named: NamedPlaces, manager: string | null) => void;
  /** Absent on the first start, where there is nothing to go back to. */
  onCancel?: () => void;
}

/**
 * The places a scan reads, shown before anything is read (ADR 0018), and
 * the mod manager they belong to (ADR 0064).
 *
 * Each place comes with the rule that found it. The user agrees or enters
 * another path. A field that is left as suggested stays "found by its
 * rule": only what was typed is remembered as the user's choice. Picking
 * another manager shows the places it would read; nothing is kept until the
 * user confirms.
 */
export function Places({ places, t, busy, onConfirm, onCancel }: PlacesProps) {
  const { api, lang } = useSession();
  const [shown, setShown] = useState(places);
  const [failure, setFailure] = useState<string | null>(null);
  // Whether the user picked a manager here; until then the suggestion shows.
  const [picked, setPicked] = useState(false);
  useEffect(() => setShown(places), [places]);

  const choose = (manager: string, instance: string | null) => {
    setFailure(null);
    setPicked(true);
    api
      .placesForManager(manager, instance, lang)
      .then(setShown)
      .catch((error: unknown) => setFailure(String(error)));
  };
  const managers = shown.managers;
  const instancePath =
    shown.places.find(({ location }) => location.role === "manager-instance")?.location.path ?? "";

  return (
    <section className="places">
      {!places.confirmed && <p className="places-text">{t.placesIntro}</p>}
      {managers !== undefined && managers.managers.length > 0 && (
        <ManagerPicker
          choice={managers}
          suggested={!picked && !managers.chosen}
          instancePath={instancePath}
          texts={managerTexts[lang]}
          busy={busy}
          onChoose={choose}
        />
      )}
      {failure !== null && (
        <p className="failure" role="alert">
          {failure}
        </p>
      )}
      <PlacesForm
        // The fields start again from the places of the chosen manager.
        key={`${managers?.active ?? ""}|${instancePath}`}
        places={shown}
        first={!places.confirmed}
        t={t}
        busy={busy}
        onConfirm={(named) => onConfirm(named, managers?.active ?? null)}
        onCancel={onCancel}
      />
    </section>
  );
}

interface ManagerPickerProps {
  choice: ManagerChoice;
  /** The instance the places are for, or "" if none. */
  instancePath: string;
  texts: ManagerTexts;
  busy: boolean;
  /** Whether the manager in use is only suggested so far. */
  suggested: boolean;
  onChoose: (manager: string, instance: string | null) => void;
}

/** One option per manager, and one per instance of a manager that has them. */
function ManagerPicker({
  choice,
  instancePath,
  texts,
  busy,
  suggested,
  onChoose,
}: ManagerPickerProps) {
  const options = choice.managers.flatMap((manager) => managerOptions(manager));
  const active = choice.managers.find((manager) => manager.id === choice.active);
  const samePath = (a: string, b: string) => a.toLowerCase() === b.toLowerCase();
  const selected = (option: ManagerOption) =>
    option.manager.id === choice.active &&
    (option.instance === null
      ? !options.some(
          (other) =>
            other.manager.id === choice.active &&
            other.instance !== null &&
            samePath(other.instance.path, instancePath),
        )
      : samePath(option.instance.path, instancePath));

  return (
    <fieldset className="managers" disabled={busy}>
      <legend>{texts.heading}</legend>
      <p className="managers-intro">{texts.intro}</p>
      <div className="managers-options">
        {options.map((option) => {
          const isSelected = selected(option);
          return (
            <label
              key={`${option.manager.id}|${option.instance?.path ?? ""}`}
              className="manager-option"
              data-selected={isSelected}
              data-found={option.manager.found}
            >
              <input
                type="radio"
                name="manager"
                checked={isSelected}
                onChange={() => onChoose(option.manager.id, option.instance?.path ?? null)}
              />
              <span className="manager-name">{option.manager.name}</span>
              {option.instance !== null && <span>{texts.instance(option.instance.name)}</span>}
              {option.instance?.portable === true && (
                <span className="manager-tag">{texts.portable}</span>
              )}
              <span className="manager-state">
                {option.manager.found ? texts.found : texts.notFound}
              </span>
              {isSelected && suggested && <span className="manager-tag">{texts.suggested}</span>}
            </label>
          );
        })}
      </div>
      {active !== undefined && !active.link && (
        <p className="managers-note">
          {active.instances.length === 0 ? `${texts.noInstance} ` : ""}
          {texts.byHand}
        </p>
      )}
    </fieldset>
  );
}

interface ManagerOption {
  manager: KnownManager;
  instance: KnownManager["instances"][number] | null;
}

/**
 * The options a manager offers: itself, or each of its instances. With
 * several instances the user picks one; with one, picking the manager is
 * enough.
 */
function managerOptions(manager: KnownManager): ManagerOption[] {
  if (manager.instances.length < 2) {
    return [{ manager, instance: null }];
  }
  return manager.instances.map((instance) => ({ manager, instance }));
}

interface PlacesFormProps {
  places: RenderedPlaces;
  first: boolean;
  t: Texts;
  busy: boolean;
  onConfirm: (named: NamedPlaces) => void;
  onCancel?: () => void;
}

function PlacesForm({ places, first, t, busy, onConfirm, onCancel }: PlacesFormProps) {
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
      <span className="place-origin">{state ?? " "}</span>
    </label>
  );
}
