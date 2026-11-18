// A stand-in for the Rust side when the frontend runs in a plain browser.
//
// The report is made up. It is typed with the generated types, so it stops
// compiling when the real report changes shape. This file is only loaded in
// development and is not part of a release build.

import type { Api } from "../api";
import type { GameChoice, KnownGame } from "../generated/game-choice";
import type { MeshView } from "../generated/mesh-view";
import type { AnchorOther, ObjectAnchors } from "../generated/object-anchors";
import type { RenderedPlaces } from "../generated/rendered-places";
import type {
  Finding,
  MapCell,
  MapMarker,
  MapSpot,
  MapTerrain,
  MapWorld,
  ModInfo,
  ObjectRef,
  RenderedFinding,
  RenderedReport,
} from "../generated/rendered-report";
import type { RenderedTool, RenderedToolbox, Tool } from "../generated/rendered-toolbox";
import type { Lang } from "../i18n";

type Two = Record<Lang, string>;
const two = (en: string, de: string): Two => ({ en, de });

/** A made-up finding in both languages. */
interface Sample {
  kind: string;
  severity: Finding["severity"];
  confidence: Finding["confidence"];
  /** What makes this finding different from others of its kind. */
  key: string;
  mod?: string;
  title: Two;
  headline: Two;
  summary: Two;
  direct?: boolean;
  args?: Finding["args"];
}

const KIND = {
  extender: two(
    "Script extender plugin does not support this game version",
    "Script-Extender-Plugin passt nicht zur Spielversion",
  ),
  master: two(
    "A plugin builds on a plugin that is not installed",
    "Ein Plugin baut auf einem Plugin auf, das nicht installiert ist",
  ),
  installer: two(
    "Recommended installer options are not installed",
    "Empfohlene Installer-Optionen sind nicht installiert",
  ),
  faces: two(
    "Characters with dark or mismatched faces",
    "Charaktere mit dunklen oder unpassenden Gesichtern",
  ),
  meshes: two(
    "A texture mod does not reach replaced meshes",
    "Textur-Mod erreicht ersetzte Modelle nicht",
  ),
  loose: two(
    "Loose files cover files in another mod's archive",
    "Lose Dateien überdecken Dateien im Archiv einer anderen Mod",
  ),
};

const installer = (mod: string, count: number, where: string): Sample => ({
  kind: "installer.options-now-recommended",
  severity: "warning",
  confidence: "likely",
  key: mod,
  mod,
  title: KIND.installer,
  headline: two(
    `${mod}: ${count} recommended ${count === 1 ? "option is" : "options are"} not installed`,
    `${mod}: ${count} empfohlene ${count === 1 ? "Option fehlt" : "Optionen fehlen"}`,
  ),
  summary: two(
    `The installer of ${mod} would now tick ${count} options that were not chosen when the mod was installed: ${where}. Each is given as step › group › option.`,
    `Der Installer von ${mod} würde jetzt ${count} Optionen ankreuzen, die bei der Installation nicht gewählt wurden: ${where}. Angegeben ist jeweils Schritt › Gruppe › Option.`,
  ),
  direct: true,
});

const faces = (plugin: string, count: number): Sample => ({
  kind: "npcs.face-mismatch",
  severity: "warning",
  confidence: "likely",
  key: plugin,
  mod: plugin.replace(".esp", ""),
  title: KIND.faces,
  headline: two(
    `${plugin} changes ${count} faces made for Face Overhaul.esp`,
    `${plugin} verändert ${count} Gesichter, die für Face Overhaul.esp gemacht sind`,
  ),
  summary: two(
    `${plugin} is loaded after Face Overhaul.esp and describes different faces for ${count} characters than the face files in use.`,
    `${plugin} wird nach Face Overhaul.esp geladen und beschreibt für ${count} Charaktere andere Gesichter als die verwendeten Gesichtsdateien.`,
  ),
});

const overlap = (first: string, second: string, count: number): Sample => ({
  kind: "objects.overlapping",
  severity: "warning",
  confidence: "hint",
  key: `${first}+${second}`,
  title: two(
    "Objects of two mods stand inside each other",
    "Objekte zweier Mods stehen ineinander",
  ),
  headline: two(
    `${first} and ${second}: ${count} objects stand inside another`,
    `${first} und ${second}: ${count} Objekte stehen in einem anderen`,
  ),
  summary: two(
    `${first} and ${second} each place objects of similar size in the same spot.`,
    `${first} und ${second} stellen jeweils Objekte ähnlicher Größe an dieselbe Stelle.`,
  ),
  args: { first, second, count },
});

const covers = (winner: string, loser: string, count: number): Sample => ({
  kind: "assets.loose-over-archive",
  severity: "info",
  confidence: "certain",
  key: `${winner}/${loser}`,
  mod: winner,
  title: KIND.loose,
  headline: two(
    `${winner} covers ${count} files of ${loser}`,
    `${winner} überdeckt ${count} Dateien von ${loser}`,
  ),
  summary: two(
    `Loose files of ${winner} replace ${count} files in the archive of ${loser}.`,
    `Lose Dateien von ${winner} ersetzen ${count} Dateien im Archiv von ${loser}.`,
  ),
  args: { count, winnerMod: winner, archiveMod: loser },
});

const samples: Sample[] = [
  {
    kind: "extender.plugin-incompatible",
    severity: "error",
    confidence: "certain",
    key: "old.dll",
    mod: "Old Mod",
    title: KIND.extender,
    headline: two(
      "Old.dll does not support game version 1.7.104",
      "Old.dll passt nicht zur Spielversion 1.7.104",
    ),
    summary: two(
      "Old.dll was switched off because it was not built for game version 1.7.104.",
      "Old.dll wurde abgeschaltet, weil es nicht für die Spielversion 1.7.104 gebaut ist.",
    ),
  },
  {
    kind: "plugins.master-missing",
    severity: "error",
    confidence: "certain",
    key: "patch.esp",
    mod: "Patch Hub",
    title: KIND.master,
    headline: two(
      "City - Dungeons Patch.esp: 1 plugin it needs is missing",
      "City - Dungeons Patch.esp: 1 benötigtes Plugin fehlt",
    ),
    summary: two(
      "City - Dungeons Patch.esp builds on a plugin that is not installed: Dungeons.esp.",
      "City - Dungeons Patch.esp baut auf einem Plugin auf, das nicht installiert ist: Dungeons.esp.",
    ),
  },
  installer("Patch Hub", 2, "Patches › Cities › City Overhaul, Patches › Dungeons › Dungeons"),
  installer("Water Mod", 1, "Compatibility › Lands › New Lands"),
  faces("College.esp", 16),
  faces("Cut Content.esp", 34),
  faces("Temple.esp", 4),
  overlap("Old Town", "Texture Pack", 3),
  {
    kind: "meshes.retexture-bypassed",
    severity: "warning",
    confidence: "likely",
    key: "textures/meshes",
    mod: "Texture Pack",
    title: KIND.meshes,
    headline: two(
      "Texture Pack does not reach 12 meshes of Mesh Improvements",
      "Texture Pack erreicht 12 Modelle von Mesh Improvements nicht",
    ),
    summary: two(
      "Texture Pack replaces textures that 12 meshes of Mesh Improvements no longer use.",
      "Texture Pack ersetzt Texturen, die von 12 Modellen aus Mesh Improvements nicht mehr verwendet werden.",
    ),
  },
  covers("Texture Pack", "Old Town", 120),
  covers("Texture Pack", "Castle Mod", 14),
  covers("Mesh Improvements", "Old Town", 31),
  covers("Grass Mod", "Landscape Mod", 8),
  covers("Tree Mod", "Landscape Mod", 3),
  covers("Rock Mod", "Landscape Mod", 44),
  {
    kind: "objects.duplicated",
    severity: "warning",
    confidence: "likely",
    key: "Old Town+Water Mod",
    title: two(
      "Two mods place the same object twice",
      "Zwei Mods stellen dasselbe Objekt doppelt hin",
    ),
    headline: two(
      "Old Town and Water Mod: 2 objects stand twice",
      "Old Town und Water Mod: 2 Objekte stehen doppelt",
    ),
    summary: two(
      "Old Town and Water Mod place the same objects at almost the same spot.",
      "Old Town und Water Mod stellen dieselben Objekte fast an dieselbe Stelle.",
    ),
    args: { first: "Old Town", second: "Water Mod", count: 2 },
  },
  {
    kind: "navmeshes.door-cut-off",
    severity: "warning",
    confidence: "likely",
    key: "Old Town.esp/Patch Hub.esp",
    mod: "Old Town",
    title: two(
      "Doors that mods add are cut off from the paths of characters",
      "Türen, die Mods hinzufügen, sind von den Wegen der Figuren abgeschnitten",
    ),
    headline: two(
      "Old Town.esp cuts off 2 doors of Patch Hub.esp",
      "Old Town.esp schneidet 2 Türen von Patch Hub.esp ab",
    ),
    summary: two(
      "Old Town.esp replaces the paths of characters in 2 places where Patch Hub.esp connects new doors.",
      "Old Town.esp ersetzt die Wege der Figuren an 2 Stellen, an denen Patch Hub.esp neue Türen anbindet.",
    ),
    args: { winner: "Old Town", loser: "Patch Hub", count: 2, places: "Riverwood" },
  },
  {
    kind: "plugins.loot-warning",
    severity: "warning",
    confidence: "hint",
    key: "Water Mod.esp",
    mod: "Water Mod",
    title: two("LOOT warns about a plugin", "LOOT warnt vor einem Plugin"),
    headline: two("LOOT: Water Mod.esp has 1 warning", "LOOT: Water Mod.esp hat 1 Warnung"),
    summary: two(
      "LOOT says: a newer version of this plugin is available.",
      "LOOT sagt: Es gibt eine neuere Version dieses Plugins.",
    ),
    args: { plugin: "Water Mod", count: 1, messages: "" },
  },
  {
    kind: "mods.mostly-overridden",
    severity: "info",
    confidence: "likely",
    key: "Old Town",
    mod: "Old Town",
    title: two(
      "A mod is mostly covered by others",
      "Eine Mod wird größtenteils von anderen überdeckt",
    ),
    headline: two(
      "Old Town: 151 of its 170 files are covered by other mods",
      "Old Town: 151 ihrer 170 Dateien werden von anderen Mods überdeckt",
    ),
    summary: two(
      "Most of what Old Town brings is replaced by Texture Pack and Mesh Improvements. Perhaps it is not needed any more.",
      "Das meiste, was Old Town mitbringt, ersetzen Texture Pack und Mesh Improvements. Vielleicht braucht es sie nicht mehr.",
    ),
    args: { mod: "Old Town", count: 151, total: 170 },
  },
];

function render(sample: Sample, lang: Lang): RenderedFinding {
  const provider =
    sample.mod === undefined ? undefined : { source: "vortex", id: sample.mod, name: sample.mod };
  const de = lang === "de";
  return {
    finding: {
      id: `${sample.kind}:${sample.key}`,
      kind: sample.kind,
      severity: sample.severity,
      confidence: sample.confidence,
      subject: { type: "setup" },
      args: sample.args ?? {},
      provider,
      previews:
        sample.kind === "npcs.face-mismatch"
          ? [
              "data/meshes/actors/character/facegendata/facegeom/skyrim.esm/00013ba1.nif",
              "data/textures/actors/character/facegendata/facetint/skyrim.esm/00013ba1.dds",
            ]
          : [],
    },
    title: sample.title[lang],
    headline: sample.headline[lang],
    summary: sample.summary[lang],
    explanation: de
      ? "Hier steht in ein paar Sätzen, was dahintersteckt und warum es etwas ausmacht. Der Text kommt für jede Befundart aus der Rust-Seite."
      : "A few sentences on what is behind this and why it matters. The text comes from the Rust side for each kind of finding.",
    remedy: de
      ? "Hier steht, was du dagegen tun kannst, Schritt für Schritt."
      : "What you can do about it, step by step.",
    severityLabel: {
      error: de ? "Fehler" : "Error",
      warning: de ? "Warnung" : "Warning",
      info: de ? "Hinweis" : "Note",
    }[sample.severity],
    confidenceLabel: {
      certain: de ? "sicher" : "certain",
      likely: de ? "wahrscheinlich" : "likely",
      hint: de ? "Vermutung" : "a guess",
    }[sample.confidence],
    actions:
      sample.kind === "npcs.face-mismatch"
        ? [
            {
              action: { type: "load-after", plugin: "Face Overhaul.esp", after: sample.key },
              label: de ? "Face Overhaul.esp gewinnen lassen" : "Let Face Overhaul.esp win",
              description: de
                ? `Vortex bekommt eine Regel, Face Overhaul.esp nach ${sample.key} zu laden, und sortiert deine Plugins neu.`
                : `Vortex gets a rule to load Face Overhaul.esp after ${sample.key} and sorts your plugins again.`,
              guide: [],
            },
          ]
        : sample.direct && provider !== undefined
          ? [
              {
                action: { type: "reinstall-mod", mod: provider },
                label: de ? "In Vortex neu installieren" : "Reinstall in Vortex",
                description: de
                  ? `Vortex öffnet den Installer von ${provider.name} erneut, mit deiner früheren Auswahl vorbelegt.`
                  : `Vortex opens the installer of ${provider.name} again, with your earlier choices filled in.`,
                guide: [
                  {
                    tick: {
                      page: 5,
                      pages: 7,
                      step: "Additional Patches A-M",
                      group: "Towns",
                      option: "City Overhaul",
                      description: de
                        ? "Patch für City Overhaul. Nur ankreuzen, wenn City Overhaul installiert ist."
                        : "Patch for City Overhaul. Only tick if City Overhaul is installed.",
                    },
                    pageLabel: de ? "Seite 5 von 7" : "Page 5 of 7",
                  },
                ],
              },
            ]
          : [],
  };
}

function places(lang: Lang, confirmed: boolean): RenderedPlaces {
  const de = lang === "de";
  const standard = de ? "am üblichen Ort" : "at its standard place";
  return {
    confirmed,
    places: [
      {
        location: {
          role: "game-dir",
          path: "C:\\Games\\Skyrim Special Edition",
          origin: "registry",
          found: true,
        },
        roleLabel: de ? "Spielordner" : "Game directory",
        originLabel: de
          ? "aus dem Eintrag, den der Launcher des Spiels in die Windows-Registry schreibt"
          : "from the entry the game's launcher writes into the Windows registry",
      },
      {
        location: {
          role: "plugin-list",
          path: "C:\\Users\\you\\AppData\\Local\\Skyrim Special Edition\\Plugins.txt",
          origin: "standard",
          found: true,
        },
        roleLabel: de ? "Liste der aktiven Plugins" : "List of active plugins",
        originLabel: standard,
      },
      {
        location: {
          role: "extender-log",
          path: "C:\\Users\\you\\Documents\\My Games\\Skyrim Special Edition\\SKSE\\skse64.log",
          origin: "standard",
          found: false,
        },
        roleLabel: de ? "Protokoll des Script Extenders" : "Script extender's log",
        originLabel: standard,
      },
      {
        location: { role: "unpacker", path: "", origin: "standard", found: false },
        roleLabel: de
          ? "Programm zum Hineinsehen in Mod-Archive"
          : "Program for looking into mod archives",
        originLabel: standard,
      },
    ],
  };
}

function report(lang: Lang): RenderedReport {
  return {
    locale: lang,
    scannedAt: 1794695581,
    game: {
      id: "skyrimse",
      name: "Skyrim Special Edition",
      version: "1.7.104",
      extenderVersion: "2.3.1",
    },
    extenderPluginsChecked: 3,
    pluginsChecked: 12,
    archivesChecked: 7,
    installersChecked: 4,
    meshesChecked: 120,
    locations: places(lang, true).places.filter(({ location }) => location.path !== ""),
    findings: samples.map((sample) => render(sample, lang)),
    mods: sampleMods,
    map: sampleMap(),
    objectSwapper: true,
    // Two of the three suggested objects, and one whose collision is gone.
    duels: sampleDuels(lang),
    installedPatch: {
      objects: [
        { plugin: "Gone Mod.esp", localId: 0x10 },
        { plugin: "Texture Pack.esp", localId: 0x942 },
        { plugin: "Texture Pack.esp", localId: 0x952 },
      ],
      provider: { source: "vortex", id: "Mod Mallet Patch", name: "Mod Mallet Patch" },
    },
  };
}

const modInfo = (name: string, version: string, extra: Partial<ModInfo> = {}): ModInfo => ({
  mod: { source: "vortex", id: name, name },
  enabled: true,
  version,
  author: "Someone",
  page: "https://www.nexusmods.com/skyrimspecialedition/mods/1",
  ...extra,
});

const sampleMods: ModInfo[] = [
  modInfo("Old Mod", "1.0", { newerVersion: "1.2" }),
  modInfo("Patch Hub", "3.1"),
  modInfo("Water Mod", "2.21"),
  modInfo("Old Town", "4.0"),
  modInfo("Texture Pack", "1.5"),
  modInfo("Mesh Improvements", "2.0", { enabled: false }),
];

/** Made-up land: hills around a lake, a sea in the north. */
function sampleTerrain(): MapTerrain {
  const [west, south, columns, rows, samples] = [-28, -23, 57, 47, 8];
  const [lowest, highest] = [-200, 9000];
  const width = columns * samples;
  const height = rows * samples;
  let heights = "";
  let water = "";
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const north = 1 - y / height;
      const hills = Math.sin(x / 18) * Math.cos(y / 22) + Math.sin((x + y) / 46);
      const lake = Math.hypot(x - width * 0.6, y - height * 0.55) < 28;
      // The sea lies at the lowest height, the lake a little higher.
      const ground = Math.max(0, Math.min(1, 0.45 + hills * 0.25));
      const share = north > 0.86 ? 0 : lake ? 0.3 : ground;
      const value = Math.round(1 + share * 65534);
      heights += String.fromCharCode(value & 255, value >> 8);
      water += String.fromCharCode(north > 0.86 || lake ? 1 : 0);
    }
  }
  return {
    west,
    south,
    columns,
    rows,
    samples,
    heights: btoa(heights),
    water: btoa(water),
    lowest,
    highest,
  };
}

/** A made-up world without objects of mods, for the picker's groups. */
function sampleWorld(
  name: string,
  title: string,
  origin: MapWorld["origin"],
  originName: string | null,
  small: boolean,
  terrain: MapTerrain | null,
): MapWorld {
  return {
    name,
    title,
    origin,
    originName,
    small,
    main: false,
    children: [],
    landCells: terrain === null ? 12 : terrain.columns * terrain.rows,
    bounds:
      terrain === null
        ? null
        : {
            west: terrain.west,
            south: terrain.south,
            east: terrain.west + terrain.columns - 1,
            north: terrain.south + terrain.rows - 1,
          },
    cells: [],
    terrain,
    markers: [],
  };
}

/** A made-up worldspace: a town where two mods meet, and land around it. */
function sampleMap(): RenderedReport["map"] {
  const cells: MapCell[] = [];
  for (let x = -20; x <= 20; x++) {
    for (let y = -15; y <= 15; y++) {
      const distance = Math.hypot(x - 4, y - 2);
      if (distance > 14 && (x * 7 + y * 3) % 5 !== 0) {
        continue;
      }
      const owners = distance < 3 ? [0, 1, 2] : distance < 6 ? [0, 1] : [(x + y + 40) % 3];
      cells.push({ x, y, objects: Math.round(200 / (1 + distance)), owners });
    }
  }
  const spot = (x: number, y: number, objects: string): MapSpot => ({
    finding: "objects.overlapping:Old Town+Texture Pack",
    location: { type: "exterior", world: "Tamriel", x, y },
    objects,
    console: `cow Tamriel ${x} ${y}`,
    parts: [
      {
        object: { plugin: "Old Town.esp", localId: 0x800 + x * 16 + y },
        name: objects.split(" / ")[0] ?? objects,
        hideable: true,
        suggested: false,
      },
      {
        object: { plugin: "Texture Pack.esp", localId: 0x900 + x * 16 + y },
        name: objects.split(" / ")[1] ?? objects,
        hideable: true,
        suggested: true,
      },
    ],
  });
  const terrain = sampleTerrain();
  const at = (x: number, y: number, dx = 0.5, dy = 0.5): [number, number, number] => [
    Math.round((x + dx) * 4096),
    Math.round((y + dy) * 4096),
    sampleHeight(terrain, x + dx, y + dy),
  ];
  const placed = (
    [x, y, dx, dy]: [number, number, number?, number?],
    spotted: MapSpot,
  ): MapSpot => ({ ...spotted, position: at(x, y, dx, dy) });
  const duplicate = (x: number, y: number, objects: string, id: number): MapSpot => ({
    finding: "objects.duplicated:Old Town+Water Mod",
    kind: "duplicate",
    location: { type: "exterior", world: "Tamriel", x, y },
    objects,
    console: `cow Tamriel ${x} ${y}`,
    parts: [
      {
        object: { plugin: "Old Town.esp", localId: id },
        name: objects,
        hideable: true,
        suggested: true,
      },
      {
        object: { plugin: "Water Mod.esp", localId: id + 1 },
        name: objects,
        hideable: true,
        suggested: false,
      },
    ],
  });
  return {
    owners: ["Old Town", "Texture Pack", "Water Mod"],
    worlds: [
      {
        name: "Tamriel",
        title: "Skyrim",
        origin: "game",
        main: true,
        // A city drawn on the main world's map.
        children: ["WhiterunWorld"],
        bounds: {
          west: terrain.west,
          south: terrain.south,
          east: terrain.west + terrain.columns - 1,
          north: terrain.south + terrain.rows - 1,
        },
        landCells: terrain.columns * terrain.rows,
        small: false,
        cells,
        terrain,
        markers: sampleMarkers(at),
      },
      // A world of the game's DLC, one of a mod, and a small one.
      sampleWorld("DLC2SolstheimWorld", "Solstheim", "game", null, false, sampleTerrain()),
      sampleWorld("Falskaar", "Falskaar", "mod", "Falskaar", false, sampleTerrain()),
      sampleWorld("BloatedMansGrottoWorld", "Bloated Man's Grotto", "game", null, true, null),
    ],
    spots: [
      placed([4, 2, 0.3, 0.6], spot(4, 2, "TownHouse / Tower")),
      placed([4, 2, 0.7, 0.4], spot(4, 2, "Well / Cart")),
      placed([5, 2, 0.2, 0.2], spot(5, 2, "Fence / Stall")),
      placed([3, 3, 0.8, 0.5], spot(3, 3, "Tree / Roof")),
      placed([6, 1, 0.4, 0.4], duplicate(6, 1, "Dock / Dock", 0xa10)),
      placed([6, 1, 0.6, 0.7], duplicate(6, 1, "Boat / Boat", 0xa20)),
    ],
  };
}

/** The height of the made-up land at a point, in game units. */
function sampleHeight(terrain: MapTerrain, x: number, y: number) {
  const binary = atob(terrain.heights);
  const width = terrain.columns * terrain.samples;
  const column = Math.floor((x - terrain.west) * terrain.samples);
  const row = Math.floor((terrain.south + terrain.rows - y) * terrain.samples);
  const at = (row * width + column) * 2;
  const value = binary.charCodeAt(at) | (binary.charCodeAt(at + 1) << 8) || 1;
  return Math.round(terrain.lowest + ((value - 1) / 65534) * (terrain.highest - terrain.lowest));
}

/** Made-up places: the game's own, and some that mods add, change or remove. */
function sampleMarkers(
  at: (x: number, y: number, dx?: number, dy?: number) => [number, number, number],
): MapMarker[] {
  const place = (
    name: string,
    kind: MapMarker["kind"],
    [x, y]: [number, number],
    extra: Partial<MapMarker> = {},
  ): MapMarker => ({
    name,
    kind,
    position: at(x, y),
    origin: "game",
    console: `cow Tamriel ${x} ${y}`,
    ...extra,
  });
  return [
    place("Aldmoor", "city", [-14, 8]),
    place("Brenwick", "city", [16, -10]),
    place("Highmere", "city", [-6, -14]),
    place("Kettleford", "town", [3, 1], {
      origin: "changed",
      owner: 0,
      movedFrom: at(1, 0),
    }),
    place("Saltreach", "town", [20, 9]),
    place("Dunwold", "settlement", [-20, -4]),
    place("Greyhollow Cave", "cave", [8, 6]),
    place("Frostfang Cave", "cave", [-10, 14]),
    place("Ironvein Mine", "mine", [12, 3]),
    place("Fort Tallow", "fort", [-2, 9]),
    place("Fort Ember", "fort", [22, -3], {
      origin: "removed",
      owner: 2,
    }),
    place("Old Barrow", "nordic-ruin", [-17, -11]),
    place("Brasswork Halls", "dwemer-ruin", [18, 14]),
    place("Ashen Lair", "dragon-lair", [-24, 12]),
    place("Moss Grove", "grove", [6, -8]),
    place("The Watcher Stone", "doomstone", [-8, 2]),
    place("Pinecrest Farm", "farm", [5, 4], { origin: "added", owner: 0 }),
    place("Old Town Mill", "wood-mill", [6, 1], { origin: "added", owner: 0 }),
    place("Kettleford Docks", "docks", [2, 4], {
      origin: "changed",
      owner: 0,
      renamedFrom: "Kettle Pier",
    }),
    place("Lakeside Shrine", "shrine", [9, -1], { origin: "added", owner: 2 }),
    place("Bandit Hideout", "camp", [-12, -2]),
    place("Giants' Rest", "giant-camp", [14, -16]),
    place("North Light", "lighthouse", [0, 18]),
  ];
}

/** A made-up toolbox: most things in place, one library outdated. */
function toolbox(lang: Lang): RenderedToolbox {
  const de = lang === "de";
  // `?vortex=stopped` in the address shows the sidebar with Vortex closed.
  const asked = new URLSearchParams(window.location.search).get("vortex");
  const vortexState: Tool["state"] = asked === "stopped" || asked === "missing" ? asked : "ready";
  const tool = (
    id: string,
    name: string,
    need: Tool["need"],
    state: Tool["state"],
    purpose: Two,
    extra: Partial<Tool> = {},
  ): RenderedTool => ({
    tool: { id, need, state, version: null, path: null, page: null, ...extra },
    name,
    purpose: purpose[lang],
    stateLabel: {
      ready: de ? "Bereit" : "Ready",
      missing: de ? "Fehlt" : "Missing",
      outdated: de ? "Braucht ein Update" : "Needs an update",
      stopped: de ? "Läuft nicht" : "Not running",
    }[state],
    needLabel: {
      required: de ? "Nötig" : "Required",
      recommended: de ? "Empfohlen" : "Recommended",
      optional: "Optional",
    }[need],
    providedLabel:
      extra.providedBy === "vortex" ? (de ? "In Vortex eingebaut" : "Built into Vortex") : null,
  });
  const tools = [
    tool(
      "vortex",
      "Vortex",
      "recommended",
      vortexState,
      two("The mod manager.", "Der Mod-Manager."),
      {
        group: "mod-manager",
        path: "C:\\Program Files\\Vortex\\Vortex.exe",
      },
    ),
    tool(
      "mallet-link",
      de ? "Mod-Mallet-Verbindung in Vortex" : "Mod Mallet link in Vortex",
      "recommended",
      linkInstalled ? "ready" : "missing",
      two("A small Vortex extension.", "Eine kleine Vortex-Erweiterung."),
      { version: "0.4.2", group: "manager-link" },
    ),
    tool(
      "skse",
      "Skyrim Script Extender (SKSE)",
      "required",
      "ready",
      two(
        "Lets mods add program code to the game.",
        "Erlaubt Mods, dem Spiel Programmcode hinzuzufügen.",
      ),
      { version: "→ 1.7.104", page: "https://skse.silverlock.org/" },
    ),
    tool(
      "address-library",
      "Address Library for SKSE Plugins",
      "recommended",
      "outdated",
      two("Tells SKSE plugins where things are.", "Sagt SKSE-Plugins, wo was liegt."),
      { page: "https://www.nexusmods.com/skyrimspecialedition/mods/32444" },
    ),
    tool(
      "vc-runtime",
      "Microsoft Visual C++ Redistributable",
      "required",
      "ready",
      two("A Windows component.", "Ein Windows-Baustein."),
      { version: "14.51.36247.00" },
    ),
    tool(
      "loot",
      "LOOT",
      "recommended",
      "ready",
      two(
        "Sorts plugins and keeps a list of known problems with them.",
        "Sortiert Plugins und führt eine Liste bekannter Probleme mit ihnen.",
      ),
      {
        group: "helper",
        version: "2026-11-17",
        path: "C:\\Users\\Example\\AppData\\Roaming\\Vortex\\skyrimse\\masterlist\\masterlist.yaml",
        providedBy: "vortex",
      },
    ),
    tool(
      "seven-zip",
      "7-Zip",
      "recommended",
      "ready",
      two("Opens mod archives.", "Öffnet Mod-Archive."),
      {
        group: "helper",
        path: "C:\\Program Files\\Vortex\\resources\\app.asar.unpacked\\node_modules\\7z-bin\\win32\\7z.exe",
      },
    ),
    tool(
      "base-object-swapper",
      "Base Object Swapper",
      "optional",
      "ready",
      two(
        "Swaps objects while the game runs; Mod Mallet's patches use it.",
        "Tauscht Objekte, während das Spiel läuft; die Patches von Mod Mallet nutzen es.",
      ),
    ),
    tool(
      "creation-kit",
      "Creation Kit",
      "optional",
      "missing",
      two(
        "Bethesda's editor: for repairing navigation meshes, looking at a place with its lighting, and making mods of your own. Not needed to play or to use Mod Mallet.",
        "Bethesdas Editor: um Navigationsnetze zu reparieren, einen Ort mit seinem Licht anzusehen und eigene Mods zu bauen. Zum Spielen und für Mod Mallet nicht nötig.",
      ),
      { group: "helper", page: "https://store.steampowered.com/app/1946180/" },
    ),
  ];
  return {
    tools,
    attention: tools.filter((t) => t.tool.state !== "ready" && t.tool.need !== "optional").length,
  };
}

/** Two made-up duels: faces, and objects. */
function sampleDuels(lang: Lang): RenderedReport["duels"] {
  const de = lang === "de";
  const side = (name: string, plugins: string[]) => ({
    mod: { source: "vortex", id: name, name },
    name,
    plugins,
  });
  return [
    {
      duel: {
        id: "duel:mod:vortex:Cut Content|mod:vortex:Face Overhaul",
        sides: [
          side("Cut Content", ["Cut Content.esp"]),
          side("Face Overhaul", ["Face Overhaul.esp"]),
        ],
        topics: [{ topic: "faces", count: 34, winning: 0 }],
        findings: ["npcs.face-mismatch:Cut Content.esp"],
        files: [
          {
            path: "meshes/actors/character/facegendata/facegeom/cut content.esp/00012345.nif",
            topic: "faces",
            object: "Lydia",
            objectKind: null,
          },
          {
            path: "textures/actors/character/facegendata/facetint/cut content.esp/00012345.dds",
            topic: "faces",
            object: "Lydia",
            objectKind: null,
          },
        ],
        suggestion: { side: 1, reason: "specialist", topic: "faces" },
        resolutions: [
          {
            side: 1,
            finding: "npcs.face-mismatch:Cut Content.esp",
            action: { type: "load-after", plugin: "Face Overhaul.esp", after: "Cut Content.esp" },
          },
        ],
      },
      topicLabels: [de ? "34 Gesichter" : "34 faces"],
      suggestion: de
        ? "Bei Face Overhaul dreht sich fast alles um Gesichter, bei Cut Content sind sie nur ein kleiner Teil. Der Spezialist kann sein Thema meist besser."
        : "Face Overhaul is about faces above all, for Cut Content they are a small part. The specialist usually does its own thing better.",
    },
    {
      duel: {
        id: "duel:mod:vortex:Old Town|mod:vortex:Texture Pack",
        sides: [side("Old Town", ["Old Town.esp"]), side("Texture Pack", ["Texture Pack.esp"])],
        topics: [{ topic: "objects", count: 3, winning: null }],
        findings: ["objects.overlapping:Old Town+Texture Pack"],
        files: [
          {
            path: "meshes/clutter/barrel01.nif",
            topic: "objects",
            object: "Barrel",
            objectKind: "container",
          },
          {
            path: "meshes/architecture/whiterun/wrhouse01.nif",
            topic: "objects",
            object: "Breezehome",
            objectKind: "static",
          },
          {
            path: "textures/architecture/whiterun/wrwoodplank01.dds",
            topic: "objects",
            object: null,
            objectKind: null,
          },
        ],
        suggestion: null,
        resolutions: [],
      },
      topicLabels: [de ? "3 Objekte" : "3 objects"],
      suggestion: null,
    },
  ];
}

/** Bytes of numbers as Base64, as the backend sends meshes. */
function base64Of(numbers: Float32Array | Uint32Array): string {
  const bytes = new Uint8Array(numbers.buffer);
  let text = "";
  for (const byte of bytes) {
    text += String.fromCharCode(byte);
  }
  return btoa(text);
}

/** A cube, standing in for a mesh. */
function sampleCube(path: string): MeshView {
  const corners = [
    [-1, -1, -1],
    [1, -1, -1],
    [1, 1, -1],
    [-1, 1, -1],
    [-1, -1, 1],
    [1, -1, 1],
    [1, 1, 1],
    [-1, 1, 1],
  ];
  const faces = [
    [0, 1, 2, 3],
    [4, 7, 6, 5],
    [0, 4, 5, 1],
    [1, 5, 6, 2],
    [2, 6, 7, 3],
    [3, 7, 4, 0],
  ];
  const positions: number[] = [];
  const uvs: number[] = [];
  const indices: number[] = [];
  for (const face of faces) {
    const start = positions.length / 3;
    for (const [corner, uv] of face.map((corner, index) => [corner, index] as const)) {
      positions.push(...(corners[corner] ?? [0, 0, 0]).map((value) => value * 30));
      uvs.push(uv === 1 || uv === 2 ? 1 : 0, uv >= 2 ? 1 : 0);
    }
    indices.push(start, start + 1, start + 2, start, start + 2, start + 3);
  }
  return {
    path,
    archive: null,
    tint: null,
    shapes: [
      {
        positions: base64Of(new Float32Array(positions)),
        uvs: base64Of(new Float32Array(uvs)),
        indices: base64Of(new Uint32Array(indices)),
        texture: samplePicture("#b07a4a"),
        tinted: false,
      },
    ],
  };
}

/** A small picture of one colour, standing in for a texture. */
function samplePicture(colour: string): string {
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="128" height="128"><rect width="128" height="128" fill="${colour}"/><circle cx="64" cy="56" r="30" fill="#00000022"/></svg>`;
  return `data:image/svg+xml,${encodeURIComponent(svg)}`;
}

/**
 * Made-up ties for an object, by its number: nothing, a link from a gate
 * that is gone, or a door to the other side.
 */
function sampleAnchors(object: ObjectRef): ObjectAnchors {
  const other = (name: string, localId: number, state: AnchorOther["state"]): AnchorOther => ({
    plugin: object.plugin,
    localId,
    recordType: "REFR",
    name,
    state,
  });
  switch (object.localId % 3) {
    case 1:
      return {
        object,
        verdict: "careful",
        reasons: [
          { kind: "linked-ref", other: other("WRGate01", 0x801, "gone"), keyword: null },
          { kind: "persistent", other: null, keyword: null },
        ],
      };
    case 2:
      return {
        object,
        verdict: "locked",
        reasons: [
          { kind: "teleport-door", other: other("WRDoor01", 0x802, "present"), keyword: null },
          { kind: "persistent", other: null, keyword: null },
        ],
      };
    default:
      return { object, verdict: "free", reasons: [] };
  }
}

/** Results arrive a little later, as they do from the real backend. */
function later<T>(value: T): Promise<T> {
  return new Promise((resolve) => setTimeout(() => resolve(value), 400));
}

// The made-up machine: nothing is agreed to and the link is missing until
// the buttons are pressed.
let confirmed = false;
let linkInstalled = false;
const ignoredIds = new Set<string>();
let decided: Record<string, number> = {};
let ranked: string[] = [];
let chosen: Record<string, string | null> | null = null;

/** The game the sample looks at. */
let activeGame = "skyrimse";

/** The games of the sample: two installed, the others not. */
function sampleGames(): GameChoice {
  const game = (
    id: string,
    name: string,
    scriptExtender: string,
    dir: string | null,
  ): KnownGame => ({
    id,
    name,
    support: id === "skyrimse" ? "full" : "basic",
    dir,
    origin: dir === null ? null : "steam",
    scriptExtender,
  });
  return {
    active: activeGame,
    games: [
      game("skyrimse", "Skyrim Special Edition", "SKSE", "D:SteamSkyrim Special Edition"),
      game("skyrimvr", "Skyrim VR", "SKSEVR", null),
      game("skyrim", "Skyrim (2011)", "SKSE", null),
      game("fallout4", "Fallout 4", "F4SE", "D:SteamFallout 4"),
      game("fallout4vr", "Fallout 4 VR", "F4SEVR", null),
      game("falloutnv", "Fallout: New Vegas", "xNVSE", null),
      game("fallout3", "Fallout 3", "FOSE", null),
      game("oblivion", "Oblivion", "OBSE", null),
    ],
  };
}

export const sampleApi: Api = {
  scan: (lang) => later(report(lang)),
  lastReport: () => later(null),
  runAction: (_action, lang) =>
    later(
      linkInstalled
        ? {
            outcome: { status: "started" as const },
            message:
              lang === "de"
                ? "Vortex hat den Installer geöffnet. Mach dort weiter und scanne danach erneut."
                : "Vortex has opened the installer. Continue there, and scan again when you are done.",
          }
        : {
            outcome: { status: "link-missing" as const },
            message:
              lang === "de"
                ? "Dafür braucht es eine kleine Erweiterung in Vortex, die noch nicht installiert ist."
                : "This needs a small extension in Vortex, which is not installed yet.",
          },
    ),
  managerLink: (lang) =>
    later({
      installed: linkInstalled,
      running: linkInstalled,
      restartNeeded: false,
      message: "",
      explanation:
        lang === "de"
          ? "Mod Mallet verändert deine Mods nicht selbst. Damit Vortex das übernehmen kann, legt Mod Mallet eine kleine Erweiterung in Vortex' Plugin-Ordner."
          : "Mod Mallet does not change your mods itself. So that Vortex can do it, Mod Mallet puts a small extension into Vortex's plugin folder.",
    }),
  installManagerLink: (lang) => {
    linkInstalled = true;
    return later(
      lang === "de"
        ? "Die Erweiterung wurde installiert. Starte Vortex neu, damit sie geladen wird."
        : "The extension was installed. Restart Vortex to load it.",
    );
  },
  places: (lang) => later(places(lang, confirmed)),
  confirmPlaces: (_named, lang) => {
    confirmed = true;
    return later(places(lang, confirmed));
  },
  ignored: () => later([...ignoredIds]),
  setIgnored: (id, aside) => {
    if (aside) {
      ignoredIds.add(id);
    } else {
      ignoredIds.delete(id);
    }
    return later([...ignoredIds]);
  },
  setupFingerprint: () => later("unchanged"),
  openPage: async () => {},
  createPatch: () =>
    later("C:\\Users\\you\\AppData\\Local\\mod-mallet\\patches\\Mod Mallet Patch.zip"),
  openFolder: async () => {},
  toolbox: (lang) => later(toolbox(lang)),
  restartManager: () => later(undefined),
  // The sample has no pictures; the page shows its placeholder.
  gameArt: () => later(null),
  meshView: (path) => later(sampleCube(path)),
  // The sample has no objects up close and no level of detail.
  areaObjects: () => later(null),
  areaLand: () => later(null),
  worldLand: () => later(null),
  interiorObjects: () => later(null),
  areaMesh: () => later(null),
  areaTexture: () => later(null),
  objectAnchors: (_world, objects) => later(objects.map(sampleAnchors)),
  lodTiles: (world) => later({ world, tiles: [] }),
  lodMesh: () => later(null),
  lodTexture: () => later(null),
  ranking: () => later([...ranked]),
  setRanking: (next) => {
    ranked = [...next];
    return later([...ranked]);
  },
  patchChoices: () => later(chosen === null ? null : { ...chosen }),
  setPatchChoices: (choices) => {
    chosen = { ...choices };
    return later(undefined);
  },
  decisions: () => later({ ...decided }),
  setDecision: (id, side) => {
    const next = { ...decided };
    if (side === null) {
      delete next[id];
    } else {
      next[id] = side;
    }
    decided = next;
    return later({ ...decided });
  },
  games: () => later(sampleGames()),
  selectGame: (game) => {
    activeGame = game;
    return later(sampleGames());
  },
  // The sample has no terrain textures; the map draws its height relief.
  mapImage: () => later(null),
  // Two versions of every file: the game's, and a mod's that wins.
  filePreview: (path) =>
    later({
      path,
      versions: [
        { archive: "Skyrim - Textures1.bsa", provider: null, image: samplePicture("#8a6f5a") },
        {
          archive: null,
          provider: {
            source: "vortex",
            id: path.includes("whiterun") ? "Texture Pack" : "Face Overhaul",
            name: path.includes("whiterun") ? "Texture Pack" : "Face Overhaul",
          },
          image: samplePicture("#c9a48c"),
        },
      ],
    }),
  logFile: () => later("C:\\Users\\you\\AppData\\Local\\mod-mallet\\logs\\mod-mallet.log"),
};
