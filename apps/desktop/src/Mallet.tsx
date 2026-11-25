import { type CSSProperties, type PointerEvent, useEffect, useId, useRef, useState } from "react";

/** How many clicks within {@link WINDOW_MS} break the frame. */
const CLICKS = 5;
const WINDOW_MS = 1000;
/** How long the hammer may lie untouched before everything mends. */
const IDLE_MS = 10_000;
/** How long mending takes; matches the CSS animation of the shards. */
const MEND_MS = 900;

/** Pixels per second squared the hammer falls with. */
const GRAVITY = 2400;
/** How much speed is left after hitting the floor, and after a wall. */
const FLOOR_BOUNCE = 0.45;
const WALL_BOUNCE = 0.6;
/** Below this speed the hammer counts as lying still. */
const RESTING = 30;
/** The fastest throw, in pixels per second. */
const MAX_THROW = 3200;

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

/** Where the logo is on the screen, where the hammer starts and returns to. */
interface Origin {
  left: number;
  top: number;
  size: number;
}

function reducedMotion(): boolean {
  return window.matchMedia?.("(prefers-reduced-motion: reduce)").matches ?? false;
}

/** The hammer of the logo, in the logo's coordinates; `clip` names a clip path of its head. */
export function Hammer({ clip }: { clip: string }) {
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
 * The hammer once it is loose: it falls, bounces off the edges of the
 * window, can be grabbed and thrown, and asks to go home when it has been
 * left alone long enough. Its position is written straight into the style,
 * not through React, so that it moves at the screen's pace.
 */
function LooseHammer({
  origin,
  mending,
  clip,
  onIdle,
}: {
  origin: Origin;
  mending: boolean;
  clip: string;
  onIdle: () => void;
}) {
  const element = useRef<SVGSVGElement>(null);
  const state = useRef({
    x: 0,
    y: 0,
    vx: 260,
    vy: -380,
    angle: 0,
    spin: 540,
    dragging: false,
    grab: { x: 0, y: 0 },
    trail: [] as { t: number; x: number; y: number }[],
    lastActive: performance.now(),
    mendFrom: null as { x: number; y: number; angle: number; at: number } | null,
  });
  const idle = useRef(onIdle);
  idle.current = onIdle;

  useEffect(() => {
    let frame = 0;
    let previous = performance.now();
    const step = (now: number) => {
      const s = state.current;
      const dt = Math.min(0.05, (now - previous) / 1000);
      previous = now;
      if (mending) {
        s.mendFrom ??= { x: s.x, y: s.y, angle: s.angle % 360, at: now };
        const share = Math.min(1, (now - s.mendFrom.at) / MEND_MS);
        const eased = 1 - (1 - share) ** 3;
        s.x = s.mendFrom.x * (1 - eased);
        s.y = s.mendFrom.y * (1 - eased);
        s.angle = s.mendFrom.angle * (1 - eased);
      } else if (!s.dragging) {
        s.vy += GRAVITY * dt;
        s.x += s.vx * dt;
        s.y += s.vy * dt;
        s.angle += s.spin * dt;
        // The window's edges, as offsets from where the logo is.
        const minX = -origin.left;
        const maxX = window.innerWidth - origin.left - origin.size;
        const minY = -origin.top;
        const maxY = window.innerHeight - origin.top - origin.size;
        if (s.y > maxY) {
          s.y = maxY;
          s.vy = Math.abs(s.vy) < RESTING * 4 ? 0 : -s.vy * FLOOR_BOUNCE;
          s.vx *= 0.8;
          s.spin *= 0.6;
        }
        if (s.y < minY) {
          s.y = minY;
          s.vy = -s.vy * WALL_BOUNCE;
        }
        if (s.x < minX || s.x > maxX) {
          s.x = Math.min(maxX, Math.max(minX, s.x));
          s.vx = -s.vx * WALL_BOUNCE;
          s.spin = -s.spin * 0.7;
        }
        if (s.y === maxY && s.vy === 0) {
          // Lying on the floor: it slides to a stop.
          const friction = 0.0015 ** dt;
          s.vx *= friction;
          s.spin *= friction;
        }
        if (Math.hypot(s.vx, s.vy) > RESTING) {
          s.lastActive = now;
        } else if (now - s.lastActive > IDLE_MS) {
          idle.current();
        }
      }
      if (element.current !== null) {
        element.current.style.transform = `translate(${s.x}px, ${s.y}px) rotate(${s.angle}deg)`;
      }
      frame = requestAnimationFrame(step);
    };
    frame = requestAnimationFrame(step);
    return () => cancelAnimationFrame(frame);
  }, [mending, origin]);

  const grab = (event: PointerEvent<SVGSVGElement>) => {
    if (mending) {
      return;
    }
    event.currentTarget.setPointerCapture(event.pointerId);
    const s = state.current;
    s.dragging = true;
    s.grab = { x: event.clientX - s.x, y: event.clientY - s.y };
    s.trail = [{ t: performance.now(), x: s.x, y: s.y }];
    s.lastActive = performance.now();
  };
  const move = (event: PointerEvent<SVGSVGElement>) => {
    const s = state.current;
    if (!s.dragging) {
      return;
    }
    const now = performance.now();
    s.x = event.clientX - s.grab.x;
    s.y = event.clientY - s.grab.y;
    s.trail = [...s.trail.filter((point) => now - point.t < 100), { t: now, x: s.x, y: s.y }];
    s.lastActive = now;
  };
  const release = () => {
    const s = state.current;
    if (!s.dragging) {
      return;
    }
    s.dragging = false;
    const first = s.trail[0];
    const last = s.trail[s.trail.length - 1];
    if (first !== undefined && last !== undefined && last.t > first.t) {
      const seconds = (last.t - first.t) / 1000;
      const clamp = (value: number) => Math.max(-MAX_THROW, Math.min(MAX_THROW, value));
      s.vx = clamp((last.x - first.x) / seconds);
      s.vy = clamp((last.y - first.y) / seconds);
      s.spin = s.vx * 0.6;
    } else {
      s.vx = 0;
      s.vy = 0;
    }
    s.lastActive = performance.now();
  };

  return (
    <svg
      ref={element}
      className="mallet-loose"
      data-mending={mending}
      style={{ left: origin.left, top: origin.top, width: origin.size, height: origin.size }}
      viewBox="0 0 512 512"
      aria-hidden="true"
      onPointerDown={grab}
      onPointerMove={move}
      onPointerUp={release}
      onPointerCancel={release}
    >
      <Hammer clip={clip} />
    </svg>
  );
}

/**
 * The logo. Hit it five times within a second and the frame shatters and
 * the hammer drops to the bottom of the window, where it can be picked up
 * and thrown. Left alone for ten seconds, it all mends. Nothing else
 * depends on it.
 */
export function Mallet() {
  const id = useId().replace(/:/g, "");
  const clicks = useRef<number[]>([]);
  const button = useRef<HTMLButtonElement>(null);
  const [phase, setPhase] = useState<Phase>("whole");
  const [origin, setOrigin] = useState<Origin | null>(null);
  const still = useRef(false);

  useEffect(() => {
    if (phase === "mending") {
      const timer = setTimeout(() => setPhase("whole"), MEND_MS);
      return () => clearTimeout(timer);
    }
    // Without animations there is no loose hammer to leave alone.
    if (phase === "broken" && still.current) {
      const timer = setTimeout(() => setPhase("mending"), IDLE_MS);
      return () => clearTimeout(timer);
    }
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
      setOrigin({ left: box.left, top: box.top, size: box.width });
    }
    still.current = reducedMotion();
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
      {phase !== "whole" && origin !== null && !still.current && (
        <LooseHammer
          origin={origin}
          mending={phase === "mending"}
          clip={clip}
          onIdle={() => setPhase((current) => (current === "broken" ? "mending" : current))}
        />
      )}
    </>
  );
}
