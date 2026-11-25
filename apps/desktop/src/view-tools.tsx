// The tools every 3D view shares (Map collisions, the map's relief, an
// interior, the guided review's three views): markings, trees, places
// where characters do something, and the camera's buttons. Their state is
// the same in every view, so that switching one off in one place shows
// in the others, with the same way back.

import { type ReactNode, useSyncExternalStore } from "react";
import { useSession } from "./Actions";
import { ActionIconsToggle } from "./action-places";
import type { Lang } from "./i18n";
import { ToggleChip } from "./ui/kit";
import { viewToolTexts } from "./view-tool-texts";

/** A switch remembered across the window's views and its next start. */
interface SharedSwitch {
  key: string;
  now: boolean;
  listeners: Set<() => void>;
}

function sharedSwitch(key: string, fallback: boolean): SharedSwitch {
  let now = fallback;
  try {
    const stored = window.localStorage.getItem(key);
    now = stored === null ? fallback : stored === "on";
  } catch {
    // Only a convenience.
  }
  return { key, now, listeners: new Set() };
}

function setSwitch(shared: SharedSwitch, value: boolean) {
  shared.now = value;
  try {
    window.localStorage.setItem(shared.key, value ? "on" : "off");
  } catch {
    // Kept for this run only.
  }
  for (const listener of shared.listeners) {
    listener();
  }
}

function useShared(shared: SharedSwitch): [boolean, (value: boolean) => void] {
  const value = useSyncExternalStore(
    (listener) => {
      shared.listeners.add(listener);
      return () => shared.listeners.delete(listener);
    },
    () => shared.now,
  );
  return [value, (next) => setSwitch(shared, next)];
}

const markings = sharedSwitch("mallet.markings", true);
// Trees around a building hide it: left out unless wanted.
const trees = sharedSwitch("mallet.trees", false);

/** Whether the views tint the parts of collisions and show what is hidden as ghosts. */
export function useMarkings(): [boolean, (on: boolean) => void] {
  return useShared(markings);
}

/** Whether the views draw the trees, plants and grass placed near. */
export function useTreesShown(): [boolean, (on: boolean) => void] {
  return useShared(trees);
}

/** An eye, open or struck through. */
export function Eye({ open }: { open: boolean }) {
  return (
    <svg className="eye" viewBox="0 0 24 24" aria-hidden="true">
      <path d="M2 12s3.6-7 10-7 10 7 10 7-3.6 7-10 7S2 12 2 12z" />
      <circle cx="12" cy="12" r="3" />
      {!open && <path d="M4 4l16 16" />}
    </svg>
  );
}

/**
 * The switch of the markings, with what they mean while on: `legend`
 * where the page says it better, else the general one.
 */
export function MarkingsTool({ legend }: { legend?: ReactNode }) {
  const { lang } = useSession();
  const v = viewToolTexts[lang];
  const [on, setOn] = useMarkings();
  return (
    <span className="view-tool">
      <ToggleChip
        on={on}
        onChange={setOn}
        icon={<Eye open={on} />}
        label={v.markings}
        hint={v.markingsHint}
      />
      {on && legend !== null && (
        <span className="view-legend" aria-hidden="true">
          {legend === undefined ? <DefaultLegend lang={lang} /> : legend}
        </span>
      )}
    </span>
  );
}

function DefaultLegend({ lang }: { lang: Lang }) {
  const v = viewToolTexts[lang];
  return (
    <>
      <span className="work-legend-focus" /> {v.legendCollisions}
      <span className="work-legend-ghost" /> {v.legendHidden}
    </>
  );
}

/** The switch of the trees, plants and grass. */
export function TreesTool() {
  const { lang } = useSession();
  const v = viewToolTexts[lang];
  const [on, setOn] = useTreesShown();
  return (
    <span className="view-tool">
      <ToggleChip on={on} onChange={setOn} label={v.trees} hint={v.treesHint} />
    </span>
  );
}

/** The switch of the icons of places where characters do something, in the bar. */
export function ActionsTool() {
  return <ActionIconsToggle className="view-tool" />;
}

/** The camera's buttons: closer, farther, from above, and everything in view. */
export function CameraTools({
  onZoom,
  onTop,
  onFit,
  fitLabel,
}: {
  /** Closer (below 1) or farther (above 1), by this factor; none where the view cannot. */
  onZoom?: (factor: number) => void;
  onTop: () => void;
  onFit: () => void;
  /** What "everything in view" is called here, if not "Show all". */
  fitLabel?: string;
}) {
  const { lang } = useSession();
  const v = viewToolTexts[lang];
  return (
    <>
      {onZoom !== undefined && (
        <>
          <button
            type="button"
            className="map-zoom-button"
            onClick={() => onZoom(1 / 1.5)}
            aria-label={v.zoomIn}
            title={v.zoomIn}
          >
            +
          </button>
          <button
            type="button"
            className="map-zoom-button"
            onClick={() => onZoom(1.5)}
            aria-label={v.zoomOut}
            title={v.zoomOut}
          >
            −
          </button>
        </>
      )}
      <button type="button" className="map-zoom-button" onClick={onTop}>
        {v.fromAbove}
      </button>
      <button type="button" className="map-zoom-button" onClick={onFit}>
        {fitLabel ?? v.showAll}
      </button>
    </>
  );
}
