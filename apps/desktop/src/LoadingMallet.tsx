import { useEffect, useId, useRef } from "react";
import "./loading-mallet.css";

// What Mod Mallet shows while it works: the hammer of the logo drives one
// tooth of a gear into the gear with every stroke. The gear stands still
// for the stroke, then moves on by one tooth; while it turns, the teeth
// grow out again until they come round to be hit once more. The hammer is
// held as in the logo: handle down on the left, head up on the right.

/** The gear: its centre, rim, teeth and how far a full tooth stands out. */
const C = { x: 240, y: 290 };
const R = 108;
const H = 34;
const N = 10;
const PITCH = (2 * Math.PI) / N;
const TOP = -Math.PI / 2;
const BASE_W = 30;
const TIP_W = 18;

/** The hammer of the logo, its head centred on (0,0), its handle down to y = 346. */
const S = 0.44;
const ARM = 346 * S;
const FACE = 131 * S;
const PIVOT = { x: C.x + ARM, y: C.y - R - H - FACE };
/** Degrees: the arm level and the face on the tooth's tip. */
const STRIKE = -90;
/** Degrees further down, to drive a tooth all the way in. */
const DEPTH = (Math.asin(H / ARM) * 180) / Math.PI;
/** At rest the hammer is held as in the logo, at 45 degrees. */
const RAISED = -45;

/** Seconds per stroke. */
const CYCLE = 1.15;
/** Where in a stroke the face meets the tooth, and where the tooth is in. */
const CONTACT = 0.42;
const PRESSED = 0.47;
/** Where in a stroke the gear moves on by one tooth. */
const TURN_FROM = 0.6;
const TURN_TO = 0.84;

const HEAD = "M -122,-56 H 122 A 170 170 0 0 1 122,56 H -122 A 170 170 0 0 1 -122,-56 Z";
const HANDLE = "M -17,80 L 17,80 L 24.99,321.17 A 25 25 0 1 1 -24.99,321.17 Z";
const VIEW = "0 -110 460 585";
/** Drawn with the hammer on the right, shown mirrored: held as in the logo. */
const MIRROR = `translate(${2 * C.x} 0) scale(-1 1)`;

const clamp = (x: number) => Math.min(1, Math.max(0, x));
const span = (u: number, a: number, b: number) => clamp((u - a) / (b - a));
const lerp = (a: number, b: number, t: number) => a + (b - a) * t;
const inQuad = (x: number) => x * x;
const outCubic = (x: number) => 1 - (1 - x) ** 3;
const inOut = (x: number) => (x < 0.5 ? 4 * x * x * x : 1 - (-2 * x + 2) ** 3 / 2);
const outBack = (x: number) => 1 + 2.2 * (x - 1) ** 3 + 1.2 * (x - 1) ** 2;

/** The hammer's angle at point `u` of a stroke. */
function hammerAngle(u: number): number {
  if (u < 0.3) return lerp(RAISED, RAISED + 7, inOut(span(u, 0, 0.3)));
  if (u < CONTACT) return lerp(RAISED + 7, STRIKE, inQuad(span(u, 0.3, CONTACT)));
  if (u < PRESSED) return lerp(STRIKE, STRIKE - DEPTH, outCubic(span(u, CONTACT, PRESSED)));
  if (u < 0.56) return lerp(STRIKE - DEPTH, STRIKE + 9, outCubic(span(u, PRESSED, 0.56)));
  return lerp(STRIKE + 9, RAISED, inOut(span(u, 0.56, 1)));
}

/** How far a tooth stands out, from how far the gear turned since it was hit. */
function regrown(turned: number): number {
  return inOut(span(turned / (2 * Math.PI), 0.06, 0.9));
}

interface Tooth {
  angle: number;
  /** 0 when driven in, 1 when it stands out fully. */
  out: number;
  /** 1 right after the hit, fading. */
  heat: number;
}

/** Everything at time `t` in seconds. */
function pose(t: number) {
  const n = t / CYCLE;
  const step = Math.floor(n);
  const u = n - step;
  const turn = (step + outBack(span(u, TURN_FROM, TURN_TO))) * PITCH;
  // Stroke k drives in tooth k mod N at n = k + CONTACT.
  const latest = Math.floor(n - CONTACT);
  const teeth: Tooth[] = [];
  for (let i = 0; i < N; i++) {
    const k = latest - ((((latest - i) % N) + N) % N);
    let out = 1;
    let heat = 0;
    if (k >= 0) {
      const since = n - (k + CONTACT);
      if (since < PRESSED - CONTACT) {
        out = 1 - outCubic(since / (PRESSED - CONTACT));
        heat = 1;
      } else {
        out = regrown(turn - k * PITCH);
        heat = Math.exp(-(since * CYCLE) / 0.35);
      }
    }
    teeth.push({ angle: TOP - turn + i * PITCH, out, heat });
  }
  return { u, turn, teeth, angle: hammerAngle(u) };
}

function polar(r: number, a: number): string {
  return `${C.x + r * Math.cos(a)},${C.y + r * Math.sin(a)}`;
}

function gearPath(teeth: Tooth[]): string {
  const hb = BASE_W / 2 / R;
  let d = "";
  teeth.forEach((tooth, index) => {
    const h = tooth.out * H;
    const ht = TIP_W / 2 / (R + h);
    d += `${index === 0 ? "M" : "L"}${polar(R, tooth.angle - hb)}`;
    d += `L${polar(R + h, tooth.angle - ht)}L${polar(R + h, tooth.angle + ht)}`;
    d += `L${polar(R, tooth.angle + hb)}`;
    const next = (teeth[(index + 1) % teeth.length]?.angle ?? tooth.angle) - hb;
    d += `A${R},${R} 0 0 1 ${polar(R, next)}`;
  });
  return `${d}Z`;
}

/** The glowing dent where a tooth was driven in. */
function glowPath(tooth: Tooth): string {
  const hb = BASE_W / 2 / R + 0.05;
  const r = R - 2;
  return `M${polar(r, tooth.angle - hb)}A${r},${r} 0 0 1 ${polar(r, tooth.angle + hb)}`;
}

/** Fixed elements, moved by the animation: one glow per tooth, a few sparks. */
const GLOW_KEYS = Array.from({ length: N }, (_, i) => `glow-${i}`);
const SPARK_KEYS = Array.from({ length: 18 }, (_, i) => `spark-${i}`);

interface Spark {
  x: number;
  y: number;
  vx: number;
  vy: number;
  age: number;
  life: number;
}

interface LoadingMalletProps {
  /** Width and height in pixels. */
  size: number;
  /** Sparks, glow and the gear's jolt; worth it only when it is large. */
  effects?: boolean;
  className?: string;
}

/**
 * The hammer driving the teeth of a gear in, one stroke at a time: what
 * Mod Mallet shows while it reads a setup. It moves by itself, outside
 * React's rendering, and stands still for people who asked for less
 * motion.
 */
export function LoadingMallet({ size, effects = size >= 96, className }: LoadingMalletProps) {
  const id = useId();
  const gear = useRef<SVGPathElement>(null);
  const glow = useRef<SVGGElement>(null);
  const holes = useRef<SVGGElement>(null);
  const body = useRef<SVGGElement>(null);
  const hammer = useRef<SVGGElement>(null);
  const sparks = useRef<SVGGElement>(null);

  useEffect(() => {
    const still = window.matchMedia?.("(prefers-reduced-motion: reduce)").matches ?? false;
    let particles: Spark[] = [];
    let frame = 0;
    let previous: number | null = null;
    let started: number | null = null;
    let lastU = 0;

    const draw = (now: number) => {
      started ??= now;
      const dt = previous === null ? 0 : Math.min(0.05, (now - previous) / 1000);
      previous = now;
      // Without motion: the moment the tooth has just gone in.
      const t = still ? CYCLE * (N + PRESSED) : (now - started) / 1000;
      const p = pose(t);

      gear.current?.setAttribute("d", gearPath(p.teeth));
      holes.current?.querySelectorAll("circle").forEach((hole, i) => {
        const a = -p.turn + (i * Math.PI) / 2 + Math.PI / 4;
        hole.setAttribute("cx", String(C.x + 32 * Math.cos(a)));
        hole.setAttribute("cy", String(C.y + 32 * Math.sin(a)));
      });
      hammer.current?.setAttribute(
        "transform",
        `translate(${PIVOT.x} ${PIVOT.y}) rotate(${p.angle})`,
      );

      if (effects) {
        const glows = glow.current?.querySelectorAll("path");
        glows?.forEach((path, i) => {
          const tooth = p.teeth[i];
          if (tooth === undefined) return;
          path.setAttribute("d", glowPath(tooth));
          path.setAttribute("opacity", String(tooth.heat * 0.9));
        });
        const since = (p.u - CONTACT + 1) % 1;
        const jolt = since < 0.12 ? Math.sin((since / 0.12) * Math.PI) * 4 : 0;
        body.current?.setAttribute("transform", `translate(0 ${jolt})`);

        if (!still && lastU < CONTACT && p.u >= CONTACT) {
          for (let i = 0; i < 9; i++) {
            const a = -Math.PI / 2 + (Math.random() - 0.5) * 2.4;
            const v = 160 + Math.random() * 260;
            particles.push({
              x: C.x + (Math.random() - 0.5) * 20,
              y: C.y - R - H,
              vx: Math.cos(a) * v,
              vy: Math.sin(a) * v,
              age: 0,
              life: 0.28 + Math.random() * 0.2,
            });
          }
        }
        particles = particles.filter((spark) => spark.age < spark.life);
        const lines = sparks.current?.querySelectorAll("line") ?? [];
        lines.forEach((line, i) => {
          const spark = particles[i];
          if (spark === undefined) {
            line.setAttribute("opacity", "0");
            return;
          }
          spark.age += dt;
          spark.vy += 900 * dt;
          spark.x += spark.vx * dt;
          spark.y += spark.vy * dt;
          const left = 1 - spark.age / spark.life;
          line.setAttribute("x1", String(spark.x));
          line.setAttribute("y1", String(spark.y));
          line.setAttribute("x2", String(spark.x - spark.vx * 0.025));
          line.setAttribute("y2", String(spark.y - spark.vy * 0.025));
          line.setAttribute("stroke", left > 0.5 ? "#ffe6a8" : "#FFA62B");
          line.setAttribute("opacity", String(left));
        });
      }
      lastU = p.u;
      if (!still) {
        frame = requestAnimationFrame(draw);
      }
    };
    frame = requestAnimationFrame(draw);
    return () => cancelAnimationFrame(frame);
  }, [effects]);

  const steel = `loading-steel-${id}`;
  const head = `loading-head-${id}`;
  return (
    <svg
      className={className === undefined ? "loading-mallet" : `loading-mallet ${className}`}
      viewBox={VIEW}
      width={size}
      height={size}
      aria-hidden="true"
    >
      <defs>
        <radialGradient id={steel} cx="40%" cy="35%" r="75%">
          <stop offset="0" stopColor="#2a8fa1" />
          <stop offset="1" stopColor="#0B7285" />
        </radialGradient>
        <clipPath id={head}>
          <path d={HEAD} />
        </clipPath>
      </defs>
      <g transform={MIRROR}>
        <ellipse className="loading-shadow" cx={C.x} cy={C.y + R + H + 24} rx="120" ry="10" />
        <g ref={body}>
          <path
            ref={gear}
            fill={`url(#${steel})`}
            stroke="#08505e"
            strokeWidth="3"
            strokeLinejoin="round"
          />
          {effects && (
            <g ref={glow} fill="none" stroke="#FFA62B" strokeWidth="7" strokeLinecap="round">
              {GLOW_KEYS.map((key) => (
                <path key={key} opacity="0" />
              ))}
            </g>
          )}
          <circle cx={C.x} cy={C.y} r="48" fill="#0a5f6f" stroke="#08505e" strokeWidth="3" />
          <circle className="loading-hole" cx={C.x} cy={C.y} r="18" />
          <g ref={holes}>
            {[0, 1, 2, 3].map((i) => (
              <circle key={i} className="loading-hole" r="9" />
            ))}
          </g>
        </g>
        {effects && (
          <g ref={sparks} strokeWidth="3" strokeLinecap="round">
            {SPARK_KEYS.map((key) => (
              <line key={key} opacity="0" />
            ))}
          </g>
        )}
        <g ref={hammer}>
          <g transform={`scale(${S}) translate(0 -346)`}>
            <path fill="#2EC4B6" d={HANDLE} />
            <path fill="#FF7A18" d={HEAD} />
            <circle clipPath={`url(#${head})`} cx="0" cy="-606" r="600" fill="#FFA62B" />
          </g>
        </g>
      </g>
    </svg>
  );
}

/** The first scan, while there is nothing to show yet but that Mod Mallet works. */
export function LoadingScreen({ title, note }: { title: string; note?: string }) {
  return (
    <section className="loading-screen" role="status">
      <LoadingMallet size={220} />
      <strong>{title}</strong>
      {note !== undefined && <span>{note}</span>}
    </section>
  );
}
