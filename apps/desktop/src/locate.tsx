// Locating objects in the 3D view: a small button next to wherever objects
// are named or chosen. Pressed, those exact objects glow in the view for a
// few seconds and the camera turns to them, so that one can tell which
// ones a choice or a pick means.
//
// The cards and lists stand beside the view, not inside it. The view that
// is open registers itself here (`registerLocator`), and a button asks
// whichever view is open last.

import "./locate.css";
import { useSyncExternalStore } from "react";
import { useSession } from "./Actions";
import type { Lang } from "./i18n";

/** What a view does to locate objects: glow them, turn the camera to them. */
export type Locator = (
  objects: readonly { plugin: string; localId: number }[],
  /** Where they stand if the view does not hold them yet (east, north, height, game units). */
  at?: readonly number[] | null,
) => void;

const locators: Locator[] = [];
const listeners = new Set<() => void>();
/** The registry's state as a number that changes with every registration. */
let version = 0;

function changed() {
  version += 1;
  for (const listener of listeners) {
    listener();
  }
}

/** Lets a 3D view be asked to locate objects; returns what takes it back. */
export function registerLocator(locator: Locator): () => void {
  locators.push(locator);
  changed();
  return () => {
    const at = locators.indexOf(locator);
    if (at >= 0) {
      locators.splice(at, 1);
      changed();
    }
  };
}

/** Locates `objects` in the 3D view that is open; false where there is none. */
export function locate(
  objects: readonly { plugin: string; localId: number }[],
  at?: readonly number[] | null,
): boolean {
  const locator = locators[locators.length - 1];
  if (locator === undefined) {
    return false;
  }
  locator(objects, at);
  return true;
}

/** Whether a 3D view is open to locate objects in. */
function useCanLocate(): boolean {
  useSyncExternalStore(
    (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    () => version,
  );
  return locators.length > 0;
}

const texts: Record<Lang, { locate: string; noView: string }> = {
  en: {
    locate: "Show where it is: it glows in the 3D view for a moment",
    noView: "Open the 3D view to see where it is",
  },
  de: {
    locate: "Zeigen, wo es ist: es leuchtet kurz in der 3D-Ansicht",
    noView: "Die 3D-Ansicht öffnen, um zu sehen, wo es ist",
  },
};

/** A small button that locates `objects` in the 3D view (see {@link locate}). */
export function LocateButton({
  objects,
  at,
}: {
  objects: readonly { plugin: string; localId: number }[];
  at?: readonly number[] | null;
}) {
  const { lang } = useSession();
  const can = useCanLocate();
  const text = texts[lang];
  const label = can ? text.locate : text.noView;
  return (
    <button
      type="button"
      className="small quiet locate-button"
      disabled={!can || objects.length === 0}
      title={label}
      aria-label={label}
      onClick={() => locate(objects, at)}
    >
      <svg className="locate-icon" viewBox="0 0 24 24" aria-hidden="true">
        <circle cx="12" cy="12" r="6" />
        <circle cx="12" cy="12" r="1.6" />
        <path d="M12 2v4M12 18v4M2 12h4M18 12h4" />
      </svg>
    </button>
  );
}
