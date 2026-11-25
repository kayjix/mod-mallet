// The slot of a list row that shows a mesh as a small picture (ADR 0085):
// a shimmer while it is drawn, then the picture, or a sign that says why
// there is none.

import { type ReactNode, useEffect, useRef, useState } from "react";
import { useSession } from "./Actions";
import type { MeshGap, MeshThumbnail } from "./generated/mesh-thumbnail";
import { useMeshThumbTexts } from "./mesh-thumb-texts";
import { thumbQueue } from "./mesh-thumbs";
import "./mesh-thumb.css";

/**
 * A mesh's picture in a list. Asked for once the slot comes near the view;
 * `version` names a version as the catalogue does, `null` the one the game
 * uses. `className` sizes the slot.
 */
export function MeshThumb({
  path,
  version = null,
  className,
}: {
  path: string;
  version?: string | null;
  className?: string;
}) {
  const { api } = useSession();
  const texts = useMeshThumbTexts();
  const slot = useRef<HTMLSpanElement>(null);
  // `undefined`: not there yet; `null`: asking failed.
  const [found, setFound] = useState<MeshThumbnail | null | undefined>(undefined);
  useEffect(() => {
    const element = slot.current;
    if (element === null) {
      return;
    }
    setFound(undefined);
    const queue = thumbQueue(api);
    let asked = false;
    let outdated = false;
    const watcher = new IntersectionObserver(
      (entries) => {
        if (asked || !entries.some((entry) => entry.isIntersecting)) {
          return;
        }
        asked = true;
        watcher.disconnect();
        queue.get(path, version).then((result) => {
          if (!outdated) {
            setFound(result);
          }
        });
      },
      // A little ahead of the view, so pictures are there when scrolled to.
      { rootMargin: "160px 0px" },
    );
    watcher.observe(element);
    return () => {
      outdated = true;
      watcher.disconnect();
      if (asked) {
        queue.forget(path, version);
      }
    };
  }, [api, path, version]);

  const classes = ["mesh-thumb", className].filter(Boolean).join(" ");
  if (found === undefined) {
    return (
      <span ref={slot} className={classes} data-state="loading" title={texts.loading}>
        <span className="mesh-thumb-shimmer" aria-hidden="true" />
      </span>
    );
  }
  if (found?.image != null) {
    return (
      <span ref={slot} className={classes} data-state="ready">
        <img className="mesh-thumb-image" src={found.image} alt="" />
      </span>
    );
  }
  const gap = found?.gap ?? null;
  const why = gap === null ? texts.failed : texts.gaps[gap];
  return (
    <span
      ref={slot}
      className={classes}
      data-state="gap"
      data-gap={gap ?? "failed"}
      title={why}
      role="img"
      aria-label={why}
    >
      <GapSign gap={gap} />
    </span>
  );
}

/** A sign for why there is no picture, in the text's colour. */
function GapSign({ gap }: { gap: MeshGap | null }) {
  let strokes: ReactNode;
  switch (gap) {
    case "skeleton":
      // A bone.
      strokes = (
        <>
          <path d="M8.5 15.5l7-7" />
          <circle cx="7" cy="15" r="2" />
          <circle cx="9" cy="17" r="2" />
          <circle cx="15" cy="7" r="2" />
          <circle cx="17" cy="9" r="2" />
        </>
      );
      break;
    case "effects":
      // Sparks.
      strokes = (
        <>
          <path d="M12 4v4M12 16v4M4 12h4M16 12h4" />
          <path d="M7 7l1.5 1.5M15.5 15.5L17 17M7 17l1.5-1.5M15.5 8.5L17 7" />
        </>
      );
      break;
    case "empty":
      // An empty frame.
      strokes = <path d="M5 5h14v14H5z" strokeDasharray="3 3" />;
      break;
    case "missing":
      // A file with a question.
      strokes = (
        <>
          <path d="M7 3h7l4 4v14H7z" />
          <path d="M10.5 11a2 2 0 1 1 2.5 1.9c-.6.2-1 .7-1 1.3M12 17.5v.1" />
        </>
      );
      break;
    default:
      // A cube with a crack: cannot be read, cannot be drawn yet, or failed.
      strokes = (
        <>
          <path d="M12 3l8 4.5v9L12 21l-8-4.5v-9z" />
          <path d="M12 3l-1.5 5 3 3-2 4 .5 6" />
        </>
      );
  }
  return (
    <svg
      className="mesh-thumb-sign"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.6"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      {strokes}
    </svg>
  );
}
