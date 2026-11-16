// A stand-in for the Rust side when the frontend runs in a plain browser.
//
// The report is made up. It is typed with the generated types, so it stops
// compiling when the real report changes shape. This file is only loaded in
// development and is not part of a release build.

import type { Api } from "../api";
import type { RenderedPlaces } from "../generated/rendered-places";
import type {
  Finding,
  MapCell,
  MapSpot,
  ModInfo,
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
      sample.direct && provider !== undefined
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
function sampleTerrain(): RenderedReport["map"]["worlds"][number]["terrain"] {
  const [west, south, columns, rows, samples] = [-28, -23, 57, 47, 4];
  const width = columns * samples;
  const height = rows * samples;
  let text = "";
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const north = 1 - y / height;
      const hills = Math.sin(x / 9) * Math.cos(y / 11) + Math.sin((x + y) / 23);
      const lake = Math.hypot(x - width * 0.6, y - height * 0.55) < 14;
      const value =
        north > 0.86 || lake ? 1 : Math.max(2, Math.min(255, Math.round(120 + hills * 60)));
      text += String.fromCharCode(value);
    }
  }
  return { west, south, columns, rows, samples, heights: btoa(text), lowest: -200, highest: 9000 };
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
  return {
    owners: ["Old Town", "Texture Pack", "Water Mod"],
    worlds: [{ name: "Tamriel", cells, terrain: sampleTerrain() }],
    spots: [
      spot(4, 2, "TownHouse / Tower"),
      spot(5, 2, "Fence / Stall"),
      spot(3, 3, "Tree / Roof"),
    ],
  };
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
      "creation-kit",
      "Creation Kit",
      "optional",
      "missing",
      two("Bethesda's editor.", "Bethesdas Editor."),
      { page: "https://store.steampowered.com/app/1946180/" },
    ),
  ];
  return {
    tools,
    attention: tools.filter((t) => t.tool.state !== "ready" && t.tool.need !== "optional").length,
  };
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
  logFile: () => later("C:\\Users\\you\\AppData\\Local\\mod-mallet\\logs\\mod-mallet.log"),
};
