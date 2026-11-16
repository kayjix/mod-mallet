import { type CSSProperties, useEffect, useId, useRef, useState } from "react";

/** How many clicks within {@link WINDOW_MS} break the frame. */
const CLICKS = 5;
const WINDOW_MS = 1000;
/** How long the hammer lies at the bottom before everything mends. */
const BROKEN_MS = 10_000;
/** How long mending takes; matches the CSS animations. */
const MEND_MS = 900;

/** The frame's corners and edge points, clockwise; with the centre each pair makes a shard. */
const RIM: [number, number][] = [
  [0, 0],
  [210, 0],
  [512, 0],
  [512, 230],
  [512, 512],
  [290, 512],
  [0, 512],
  [0, 270],
];
const CENTRE: [number, number] = [256, 256];

/** The shards of the frame: a triangle each, and where it flies. */
const SHARDS = RIM.map((point, index) => {
  const next = RIM[(index + 1) % RIM.length] ?? point;
  const cx = (CENTRE[0] + point[0] + next[0]) / 3;
  const cy = (CENTRE[1] + point[1] + next[1]) / 3;
  return {
    points: `${CENTRE.join(",")} ${point.join(",")} ${next.join(",")}`,
    dx: (cx - 256) * 3.2,
    dy: (cy - 256) * 3.2 + 260,
    rot: (index % 2 === 0 ? 1 : -1) * (40 + index * 17),
  };
});

type Phase = "whole" | "broken" | "mending";

/** The hammer of the logo, in the logo's coordinates. */
function Hammer({ clip }: { clip: string }) {
  return (
    <g transform="translate(307.2 204.8) scale(0.9341) rotate(45)">
      <path fill="#2EC4B6" d="M -17,80 L 17,80 L 24.99,321.17 A 25 25 0 1 1 -24.99,321.17 Z" />
      <path
        fill="#FF7A18"
        d="M -122,-56 H 122 A 170 170 0 0 1 122,56 H -122 A 170 170 0 0 1 -122,-56 Z"
      />
      <circle clipPath={`url(#${clip})`} cx="0" cy="-606" r="600" fill="#FFA62B" />
    </g>
  );
}

/**
 * The logo. Hit it five times within a second and the frame shatters and
 * the hammer drops to the bottom of the window; ten seconds later it all
 * mends. Nothing else depends on it.
 */
export function Mallet() {
  const id = useId().replace(/:/g, "");
  const clicks = useRef<number[]>([]);
  const button = useRef<HTMLButtonElement>(null);
  const [phase, setPhase] = useState<Phase>("whole");
  const [fall, setFall] = useState<CSSProperties>({});

  useEffect(() => {
    if (phase === "whole") {
      return;
    }
    const timer = setTimeout(
      () => setPhase(phase === "broken" ? "mending" : "whole"),
      phase === "broken" ? BROKEN_MS : MEND_MS,
    );
    return () => clearTimeout(timer);
  }, [phase]);

  const hit = () => {
    if (phase !== "whole") {
      return;
    }
    const now = Date.now();
    clicks.current = [...clicks.current.filter((at) => now - at < WINDOW_MS), now];
    if (clicks.current.length < CLICKS) {
      return;
    }
    clicks.current = [];
    const box = button.current?.getBoundingClientRect();
    if (box !== undefined) {
      setFall({
        left: box.left,
        top: box.top,
        width: box.width,
        height: box.height,
        "--fall": `${Math.max(0, window.innerHeight - box.bottom - 6)}px`,
        "--drift": `${Math.round(window.innerWidth * 0.08 + 40)}px`,
      } as CSSProperties);
    }
    setPhase("broken");
  };

  const clip = `mallet-head-${id}`;
  return (
    <>
      <button
        ref={button}
        type="button"
        className="mallet"
        data-phase={phase}
        aria-label="Mod Mallet"
        onClick={hit}
      >
        <svg viewBox="0 0 512 512" aria-hidden="true">
          <defs>
            <clipPath id={clip}>
              <path d="M -122,-56 H 122 A 170 170 0 0 1 122,56 H -122 A 170 170 0 0 1 -122,-56 Z" />
            </clipPath>
            <clipPath id={`mallet-frame-${id}`}>
              <rect width="512" height="512" rx="112" />
            </clipPath>
          </defs>
          {/* Each shard is clipped on its own, so it keeps its rounded corner while it flies. */}
          <g>
            {SHARDS.map((shard) => (
              <polygon
                key={shard.points}
                className="mallet-shard"
                clipPath={`url(#mallet-frame-${id})`}
                points={shard.points}
                fill="#0B7285"
                stroke="#0B7285"
                strokeWidth="2"
                style={
                  {
                    "--dx": `${shard.dx}px`,
                    "--dy": `${shard.dy}px`,
                    "--rot": `${shard.rot}deg`,
                  } as CSSProperties
                }
              />
            ))}
          </g>
          <g className="mallet-head">
            <Hammer clip={clip} />
          </g>
        </svg>
      </button>
      {phase !== "whole" && (
        <svg
          className="mallet-falling"
          data-phase={phase}
          style={fall}
          viewBox="0 0 512 512"
          aria-hidden="true"
        >
          <Hammer clip={clip} />
        </svg>
      )}
    </>
  );
}
