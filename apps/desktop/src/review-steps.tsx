// The steps of "Step by step" before the collisions (ADR 0083): the
// installation with what the app is for, the assets' biggest duels in
// pictures, and a map of where the collisions are. Each shows what is at
// stake and leads to the page that has it all.

import { useEffect, useMemo, useRef, useState } from "react";
import { useSession } from "./Actions";
import { useCollisionStepTexts } from "./collision-step-texts";
import { isPictured } from "./compare-files";
import { duelNames, rankedSide } from "./Duels";
import type { FilePreview } from "./generated/file-preview";
import type {
  MapTerrain,
  MapWorld,
  RenderedDuel,
  RenderedFinding,
  RenderedReport,
} from "./generated/rendered-report";
import { useWorldPicture } from "./MapView";
import { CELL, readLand, type WorkKind } from "./map-places";
import { type Cluster, openIn } from "./review-clusters";
import { useReviewStepTexts } from "./review-step-texts";
import { useReviewTexts } from "./review-texts";
import { FindingRow } from "./Tasks";
import { Chip, Note } from "./ui/kit";
import "./review-steps.css";

/** How many rows of a group the installation step shows before it leads to the tasks. */
const ROWS = 8;

/** The mod manager the setup is read from, by the source of its mods. */
export function managerOf(report: RenderedReport): string {
  return report.mods.some((info) => info.mod.source === "mo2") ? "Mod Organizer 2" : "Vortex";
}

/**
 * The installation (ADR 0083): who does what (Mod Mallet decides, the mod
 * manager carries out, the game shows), what this step checks and why,
 * the setup in a few numbers, and what to fix, mods and plugins apart.
 */
export function InstallStep({
  report,
  mods,
  plugins,
  onTasks,
  onMods,
}: {
  report: RenderedReport;
  /** What to fix about mods and their installers. */
  mods: RenderedFinding[];
  /** What to fix about plugins and their order. */
  plugins: RenderedFinding[];
  onTasks: () => void;
  onMods: () => void;
}) {
  const s = useReviewStepTexts();
  const manager = managerOf(report);
  const enabled = report.mods.filter((info) => info.enabled).length;
  const newer = report.mods.filter((info) => info.newerVersion != null).length;
  const loaded = report.loadOrder?.length ?? report.pluginsChecked;
  return (
    <div className="review-install">
      <ol className="review-roles">
        <li>
          <RoleIcon who="decides" />
          <strong>{s.decides}</strong>
          <span>{s.decidesNote}</span>
        </li>
        <li>
          <RoleIcon who="carries" />
          <strong>{s.carries(manager)}</strong>
          <span>{s.carriesNote}</span>
        </li>
        <li>
          <RoleIcon who="shows" />
          <strong>{s.shows}</strong>
          <span>{s.showsNote}</span>
        </li>
      </ol>
      <p className="review-purpose">{s.installPurpose}</p>
      <div className="review-tiles">
        <Tile
          value={s.modsOnValue(enabled, report.mods.length)}
          label={s.modsOn}
          onClick={onMods}
        />
        <Tile value={String(newer)} label={s.newer} onClick={onMods} />
        <Tile value={String(loaded)} label={s.plugins} />
        <Tile
          value={String(mods.length + plugins.length)}
          label={s.toFix}
          tone={mods.length + plugins.length > 0 ? "warning" : "ok"}
        />
      </div>
      {mods.length + plugins.length === 0 ? (
        <Note tone="ok" compact>
          <p>{s.nothingToFix}</p>
        </Note>
      ) : (
        <>
          <FindingGroup title={s.groupMods} findings={mods} onMore={onTasks} />
          <FindingGroup
            title={s.groupPlugins}
            note={s.pluginsNote(manager)}
            findings={plugins}
            onMore={onTasks}
          />
        </>
      )}
    </div>
  );
}

/** A number and what it counts, as a tile; leads somewhere where it can. */
function Tile({
  value,
  label,
  tone,
  onClick,
}: {
  value: string;
  label: string;
  tone?: "ok" | "warning";
  onClick?: () => void;
}) {
  const body = (
    <>
      <strong className="review-tile-value">{value}</strong>
      <span className="review-tile-label">{label}</span>
    </>
  );
  return onClick === undefined ? (
    <div className="review-tile" data-tone={tone}>
      {body}
    </div>
  ) : (
    <button type="button" className="review-tile" data-tone={tone} onClick={onClick}>
      {body}
    </button>
  );
}

/** A group of findings with its count, the first few, and the way to all of them. */
function FindingGroup({
  title,
  note,
  findings,
  onMore,
}: {
  title: string;
  note?: string;
  findings: RenderedFinding[];
  onMore: () => void;
}) {
  const r = useReviewTexts();
  if (findings.length === 0) {
    return null;
  }
  return (
    <section className="review-group">
      <header className="review-group-head">
        <h4>{title}</h4>
        <Chip tone="warning">{findings.length}</Chip>
        {note !== undefined && <span className="aside">{note}</span>}
        <span className="spacer" />
        {findings.length > ROWS && (
          <button type="button" className="small quiet" onClick={onMore}>
            {r.toTasks} →
          </button>
        )}
      </header>
      <ul className="review-findings">
        {findings.slice(0, ROWS).map((rendered) => (
          <FindingRow key={rendered.finding.id} rendered={rendered} />
        ))}
      </ul>
    </section>
  );
}

/** The sign of who does what: a mallet, a box, a screen. */
function RoleIcon({ who }: { who: "decides" | "carries" | "shows" }) {
  return (
    <svg className="review-role-icon" viewBox="0 0 24 24" aria-hidden="true">
      {who === "decides" && (
        <>
          <path d="M4 20l9-9" />
          <rect x="11" y="3" width="9" height="6" rx="1.5" transform="rotate(45 15.5 6)" />
        </>
      )}
      {who === "carries" && (
        <>
          <path d="M3 8l9-5 9 5v8l-9 5-9-5z" />
          <path d="M3 8l9 5 9-5M12 13v8" />
        </>
      )}
      {who === "shows" && (
        <>
          <rect x="3" y="4" width="18" height="13" rx="2" />
          <path d="M10 8.5l5 2.5-5 2.5zM8 21h8" />
        </>
      )}
    </svg>
  );
}

/** How many of the biggest open duels the assets step shows in pictures. */
const BIGGEST = 6;

/**
 * The assets (ADR 0083): what an asset collision is, how many are
 * decided, the biggest open ones with a picture of each side, and the
 * way to the page where they are decided.
 */
export function AssetsStep({
  duels,
  onOpen,
}: {
  /** Every duel of files, decided or not. */
  duels: RenderedDuel[];
  onOpen: () => void;
}) {
  const s = useReviewStepTexts();
  const { decisions } = useSession();
  const open = duels.filter((rendered) => decisions[rendered.duel.id] === undefined);
  const decided = duels.length - open.length;
  const size = (rendered: RenderedDuel) =>
    rendered.duel.topics.reduce((sum, topic) => sum + topic.count, 0);
  const biggest = [...open].sort((a, b) => size(b) - size(a)).slice(0, BIGGEST);
  return (
    <div className="review-assets">
      <p className="review-purpose">{s.assetsPurpose}</p>
      <div className="review-progress">
        <div
          className="review-progress-bar"
          role="progressbar"
          aria-valuemin={0}
          aria-valuemax={Math.max(1, duels.length)}
          aria-valuenow={decided}
        >
          <span style={{ width: `${(100 * decided) / Math.max(1, duels.length)}%` }} />
        </div>
        <span>{s.assetsDecided(decided, duels.length)}</span>
        <span className="spacer" />
        <button type="button" className="primary" onClick={onOpen}>
          {s.openAssets} →
        </button>
      </div>
      {open.length === 0 ? (
        <Note tone="ok" compact>
          <p>{s.allAssetsDecided}</p>
        </Note>
      ) : (
        <>
          <h4 className="review-subhead">{s.biggest}</h4>
          <div className="review-duels">
            {biggest.map((rendered) => (
              <DuelCard key={rendered.duel.id} rendered={rendered} />
            ))}
          </div>
        </>
      )}
    </div>
  );
}

/** One duel as a card: a picture of each side's file, what they disagree about, and the way to compare. */
function DuelCard({ rendered }: { rendered: RenderedDuel }) {
  const s = useReviewStepTexts();
  const { api, mods, ranking, openCompare } = useSession();
  const { duel } = rendered;
  const names = duelNames(duel, mods);
  const suggested = rankedSide(duel, ranking) ?? duel.suggestion?.side ?? null;
  const path = (duel.files ?? []).map((file) => file.path).find(isPictured) ?? null;
  const [preview, setPreview] = useState<FilePreview | null>(null);
  useEffect(() => {
    setPreview(null);
    if (path === null) {
      return;
    }
    let outdated = false;
    api
      .filePreview(path)
      .then((found) => {
        if (!outdated) {
          setPreview(found);
        }
      })
      .catch(() => {});
    return () => {
      outdated = true;
    };
  }, [api, path]);
  // The last two versions: the one the game shows now last.
  const versions = (preview?.versions ?? []).slice(-2);
  return (
    <article className="review-duel">
      <div className="review-duel-pictures">
        {versions.length === 0 ? (
          // Files without a picture: their names, the first few.
          <ul className="review-duel-files">
            {(duel.files ?? []).slice(0, 3).map((file) => (
              <li key={file.path}>
                <code>{file.path.split("/").pop()}</code>
              </li>
            ))}
            {(duel.files ?? []).length === 0 && <li className="aside">{s.noPicture}</li>}
          </ul>
        ) : (
          versions.map((version, index) => (
            <figure key={version.archive ?? version.provider?.id ?? index}>
              {version.image == null ? (
                <div className="review-duel-empty">{s.noPicture}</div>
              ) : (
                <img src={version.image} alt={version.provider?.name ?? version.archive ?? ""} />
              )}
              {index === versions.length - 1 && (
                <span className="review-duel-badge">{s.inGame}</span>
              )}
              <figcaption>{version.provider?.name ?? version.archive ?? s.loose}</figcaption>
            </figure>
          ))
        )}
      </div>
      <strong className="review-duel-names">
        {names[0]} <span aria-hidden="true">⚔</span> {names[1]}
      </strong>
      <span className="aside">{rendered.topicLabels.join(" · ")}</span>
      <div className="review-duel-foot">
        {suggested === 0 || suggested === 1 ? (
          <Chip tone="info">{s.suggested(names[suggested])}</Chip>
        ) : (
          <span />
        )}
        <button type="button" className="small" onClick={() => openCompare(duel.id)}>
          {s.compare} ▸
        </button>
      </div>
    </article>
  );
}

/** One place on the map of the open places. */
interface MapPlace {
  key: string;
  /** Where it is, in cells (east, north). */
  at: [number, number];
  kind: WorkKind;
  decided: boolean;
  title: string;
}

/** The kinds of collisions, the worst first, for the legend. */
const KINDS: WorkKind[] = ["duplicate", "door", "overlap", "deep", "touch"];

/** A plugin's name without its extension. */
const short = (plugin: string) => plugin.replace(/\.(esp|esm|esl)$/i, "");

/**
 * The map of the places (ADR 0083): for each world with collisions its
 * land and a point for each place, coloured by its worst kind, open ones
 * large; places inside as a list. A point leads to its place.
 */
export function MapStep({
  report,
  clusters,
  onPlace,
  onMap,
}: {
  report: RenderedReport;
  clusters: Cluster[];
  onPlace: (key: string) => void;
  onMap: () => void;
}) {
  const s = useReviewStepTexts();
  const c = useCollisionStepTexts();
  const worlds = report.map.worlds;
  const { byWorld, inside } = useMemo(() => {
    const byWorld = new Map<string, MapPlace[]>();
    const inside: (MapPlace & { cell: string })[] = [];
    for (const cluster of clusters) {
      const spot = cluster.spots[0]?.item.spot;
      if (spot === undefined) {
        continue;
      }
      const pair = `${short(cluster.plugins[0])} ⟷ ${short(cluster.plugins[1])}`;
      const decided = openIn(cluster) === 0;
      const place: MapPlace = {
        key: cluster.key,
        at: [cluster.centre[0] / CELL, cluster.centre[1] / CELL],
        kind: cluster.kind,
        decided,
        title: s.placeTitle(pair, c.spotKind[cluster.kind], decided),
      };
      const { location } = spot;
      if (location.type === "interior") {
        inside.push({ ...place, cell: location.cell });
        continue;
      }
      const world =
        worlds.find((candidate) => (candidate.children ?? []).includes(location.world))?.name ??
        location.world;
      byWorld.set(world, [...(byWorld.get(world) ?? []), place]);
    }
    return { byWorld, inside };
  }, [clusters, worlds, s, c]);
  const open = clusters.filter((cluster) => openIn(cluster) > 0).length;
  const shown = [...byWorld.entries()].sort(
    (a, b) =>
      b[1].filter((place) => !place.decided).length - a[1].filter((place) => !place.decided).length,
  );
  return (
    <div className="review-map">
      <p className="review-purpose">{s.mapPurpose}</p>
      <div className="review-progress">
        <span>{s.mapSummary(open, byWorld.size + (inside.length > 0 ? 1 : 0))}</span>
        {KINDS.filter((kind) => clusters.some((cluster) => cluster.kind === kind)).map((kind) => (
          <span key={kind} className="review-map-legend">
            <span className="review-map-key" data-kind={kind} /> {c.spotKind[kind]}
          </span>
        ))}
        <span className="review-map-legend">
          <span className="review-map-key" data-decided="" /> {s.legendDecided}
        </span>
        <span className="spacer" />
        <button type="button" className="small quiet" onClick={onMap}>
          {s.toMap} →
        </button>
      </div>
      {shown.length === 0 && inside.length === 0 && <p className="aside">{s.noPlaces}</p>}
      {shown.map(([name, places]) => {
        const world = worlds.find((candidate) => candidate.name === name);
        return (
          <section key={name} className="review-map-world">
            <h4>{world?.title ?? name}</h4>
            <PlacesMap world={world} places={places} onPlace={onPlace} />
          </section>
        );
      })}
      {inside.length > 0 && (
        <section className="review-map-world">
          <h4>{s.interiors}</h4>
          <ul className="review-map-inside">
            {inside.map((place) => (
              <li key={place.key}>
                <button
                  type="button"
                  className="small"
                  data-decided={place.decided ? "" : undefined}
                  onClick={() => onPlace(place.key)}
                >
                  <span className="review-map-key" data-kind={place.kind} />
                  {place.cell} · {place.title}
                </button>
              </li>
            ))}
          </ul>
        </section>
      )}
    </div>
  );
}

/** How many cells the map shows around its outermost places. */
const AROUND = 6;
/** The height of a world's map, in pixels. */
const MAP_HEIGHT = 320;

/** One world's land, as the game pictures it or from its heights, with a point for each place. */
function PlacesMap({
  world,
  places,
  onPlace,
}: {
  world: MapWorld | undefined;
  places: MapPlace[];
  onPlace: (key: string) => void;
}) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const frame = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(800);
  const picture = useWorldPicture(world?.name);
  const land = useMemo(
    () => (world?.terrain == null ? null : reliefOf(world.terrain)),
    [world?.terrain],
  );
  useEffect(() => {
    const element = frame.current;
    if (element === null) {
      return;
    }
    const observer = new ResizeObserver(() => setWidth(Math.max(200, element.clientWidth)));
    observer.observe(element);
    return () => observer.disconnect();
  }, []);
  // The cells shown: the places with some land around them, as wide as the frame.
  const xs = places.map((place) => place.at[0]);
  const ys = places.map((place) => place.at[1]);
  const middle = [(Math.min(...xs) + Math.max(...xs)) / 2, (Math.min(...ys) + Math.max(...ys)) / 2];
  const spanX = Math.max(...xs) - Math.min(...xs) + AROUND * 2;
  const spanY = Math.max(...ys) - Math.min(...ys) + AROUND * 2;
  const perCell = Math.min(width / spanX, MAP_HEIGHT / spanY, 48);
  const west = (middle[0] ?? 0) - width / perCell / 2;
  const north = (middle[1] ?? 0) + MAP_HEIGHT / perCell / 2;
  const toScreen = (x: number, y: number) => [(x - west) * perCell, (north - y) * perCell] as const;
  useEffect(() => {
    const element = canvas.current;
    const context = element?.getContext("2d") ?? null;
    if (element === null || context === null) {
      return;
    }
    const ratio = window.devicePixelRatio || 1;
    element.width = Math.round(width * ratio);
    element.height = Math.round(MAP_HEIGHT * ratio);
    context.setTransform(ratio, 0, 0, ratio, 0, 0);
    context.clearRect(0, 0, width, MAP_HEIGHT);
    context.imageSmoothingEnabled = true;
    if (picture !== null) {
      const { meta, image } = picture;
      const [left, top] = toScreen(meta.west, meta.south + meta.rows);
      context.drawImage(image, left, top, meta.columns * perCell, meta.rows * perCell);
    } else if (land !== null && world?.terrain != null) {
      const terrain = world.terrain;
      const [left, top] = toScreen(terrain.west, terrain.south + terrain.rows);
      context.drawImage(land, left, top, terrain.columns * perCell, terrain.rows * perCell);
    }
  });
  return (
    <div className="review-map-frame" ref={frame} style={{ height: MAP_HEIGHT }}>
      <canvas ref={canvas} style={{ width, height: MAP_HEIGHT }} />
      {[...places]
        .sort((a, b) => Number(b.decided) - Number(a.decided))
        .map((place) => {
          const [x, y] = toScreen(place.at[0], place.at[1]);
          return (
            <button
              type="button"
              key={place.key}
              className="review-map-place"
              data-kind={place.kind}
              data-decided={place.decided ? "" : undefined}
              style={{ left: x, top: y }}
              title={place.title}
              aria-label={place.title}
              onClick={() => onPlace(place.key)}
            />
          );
        })}
    </div>
  );
}

/** The land from its heights, one pixel per sample: water dark blue, land grey-green by height. */
function reliefOf(terrain: MapTerrain): HTMLCanvasElement | null {
  const land = readLand(terrain);
  const canvas = document.createElement("canvas");
  const context = canvas.getContext("2d");
  if (land === null || context === null) {
    return null;
  }
  const { width, height, heights, kinds } = land;
  canvas.width = width;
  canvas.height = height;
  const image = context.createImageData(width, height);
  const span = Math.max(1, terrain.highest - terrain.lowest);
  for (let index = 0; index < width * height; index++) {
    const kind = kinds[index] ?? 0;
    if (kind === 0) {
      continue;
    }
    const share = ((heights[index] ?? terrain.lowest) - terrain.lowest) / span;
    const colour: [number, number, number] =
      kind === 1 ? [28, 52, 70] : [62 + share * 120, 72 + share * 110, 60 + share * 110];
    image.data[index * 4] = colour[0];
    image.data[index * 4 + 1] = colour[1];
    image.data[index * 4 + 2] = colour[2];
    image.data[index * 4 + 3] = 255;
  }
  context.putImageData(image, 0, 0);
  return canvas;
}
