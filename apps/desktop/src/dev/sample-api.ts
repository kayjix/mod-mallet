// A stand-in for the Rust side when the frontend runs in a plain browser.
//
// The report is made up. It is typed with the generated types, so it stops
// compiling when the real report changes shape. This file is only loaded in
// development and is not part of a release build.

import type { Api } from "../api";
import type { ActionPlace, AreaObject, AreaObjects } from "../generated/area-objects";
import type { AssetVersion, AssetVersions } from "../generated/asset-versions";
import type { Catalogue, CatalogueItem } from "../generated/catalogue";
import type { GameChoice, KnownGame } from "../generated/game-choice";
import type { LodMesh } from "../generated/lod-mesh";
import type { MeshView } from "../generated/mesh-view";
import type { AnchorOther, ObjectAnchors } from "../generated/object-anchors";
import type { RenderedPlaces } from "../generated/rendered-places";
import type {
  Finding,
  LoadedPlugin,
  MapCell,
  MapMarker,
  MapPassage,
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
import { onSampleScanProgress, simulateScan } from "./sample-scan";

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
  overlap("Busy Streets.esp", "Old Town", 4),
  {
    kind: "plugins.switched-off-but-loaded",
    severity: "warning",
    confidence: "certain",
    key: "Busy Streets.esp",
    title: two(
      "A mod you switched off is still in the game",
      "Eine Mod, die du ausgeschaltet hast, ist noch im Spiel",
    ),
    headline: two(
      "Busy Streets is switched off, but the game still loads Busy Streets.esp",
      "Busy Streets ist ausgeschaltet, aber das Spiel lädt Busy Streets.esp trotzdem",
    ),
    summary: two(
      "Busy Streets is switched off in your mod manager, but the game still loads Busy Streets.esp. Copies of its files are still in the game's Data folder: Busy Streets.bsa, Busy Streets.esp.",
      "Busy Streets ist in deinem Mod-Manager ausgeschaltet, aber das Spiel lädt Busy Streets.esp trotzdem. Kopien ihrer Dateien liegen noch im Data-Ordner des Spiels: Busy Streets.bsa, Busy Streets.esp.",
    ),
    args: {
      plugin: "Busy Streets.esp",
      mod: "Busy Streets",
      deployed: 0,
      fileCount: 2,
      files: "Busy Streets.bsa, Busy Streets.esp",
      fileList: "Busy Streets.bsa\nBusy Streets.esp",
      dependentCount: 1,
      dependents: "Water Mod - Busy Streets Patch.esp",
      dependentPlugins: "Water Mod - Busy Streets Patch.esp",
      dependentModIds: "Water Mod",
    },
  },
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
  {
    kind: "meshes.texture-missing",
    severity: "warning",
    confidence: "hint",
    key: "Old Town",
    mod: "Old Town",
    title: two(
      "Meshes of a mod use textures that are not in the game",
      "Modelle einer Mod verwenden Texturen, die im Spiel fehlen",
    ),
    headline: two("3 textures missing for 2 meshes", "3 Texturen fehlen für 2 Modelle"),
    summary: two(
      "2 meshes of this mod use 3 textures for their colour or surface that no mod and no archive in your game provides: data/textures/town/gate.dds, data/textures/town/wall.dds, data/textures/town/wall_n.dds. 2 of them are in Mesh Improvements, installed but switched off.",
      "2 Modelle dieser Mod verwenden für Farbe oder Oberfläche 3 Texturen, die keine Mod und kein Archiv in deinem Spiel mitbringt: data/textures/town/gate.dds, data/textures/town/wall.dds, data/textures/town/wall_n.dds. 2 davon liegen in Mesh Improvements, installiert, aber ausgeschaltet.",
    ),
    args: {
      meshes: 2,
      count: 3,
      list: [
        "data/textures/town/gate.dds\tdata/meshes/town/gate.nif",
        "data/textures/town/wall.dds\tdata/meshes/town/gate.nif, data/meshes/town/wall.nif",
        "data/textures/town/wall_n.dds\tdata/meshes/town/wall.nif",
      ].join("\n"),
      offCount: 2,
      offMods: "Mesh Improvements",
    },
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
    kind: "doors.stacked",
    severity: "warning",
    confidence: "likely",
    key: "Old Town+Water Mod",
    title: two(
      "Doors of two mods stand on top of each other",
      "Türen zweier Mods stehen übereinander",
    ),
    headline: two(
      "Old Town and Water Mod: 1 door stands on top of another",
      "Old Town und Water Mod: 1 Tür steht auf einer anderen",
    ),
    summary: two(
      "Old Town and Water Mod each put a door at the same spot.",
      "Old Town und Water Mod setzen jeweils eine Tür an dieselbe Stelle.",
    ),
    args: { first: "Old Town", second: "Water Mod", count: 1 },
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
  {
    kind: "mods.collides-widely",
    severity: "info",
    confidence: "likely",
    key: "Water Mod",
    mod: "Water Mod",
    title: two(
      "A mod collides with many places of other mods",
      "Eine Mod kollidiert an vielen Stellen mit anderen",
    ),
    headline: two(
      "Suggestion: Water Mod collides with other mods in 2 places",
      "Vorschlag: Water Mod kollidiert an 2 Stellen mit anderen Mods",
    ),
    summary: two(
      "14 of the 17 objects Water Mod puts into the world stand in or against objects of Old Town.",
      "14 der 17 Objekte, die Water Mod in die Welt stellt, stehen in oder an Objekten von Old Town.",
    ),
    args: {
      mod: "Water Mod",
      count: 24,
      clearCount: 4,
      objects: 14,
      total: 17,
      interiors: 0,
      modCount: 1,
      dependentCount: 2,
      dependentModCount: 2,
      dependents:
        "Cut Content Patches (Cut Content - Water Patch.esp), Patch Hub (Water Mod - Old Town Patch.esp)",
      dependentPlugins: "Water Mod - Old Town Patch.esp\nCut Content - Water Patch.esp",
      dependentModIds: "Patch Hub\nCut Content Patches",
      assetCount: 3,
      assetUsers: "Old Town (3)",
    },
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
        : sample.kind === "meshes.texture-missing"
          ? [
              {
                action: {
                  type: "enable-mod",
                  mod: { source: "vortex", id: "Mesh Improvements", name: "Mesh Improvements" },
                },
                label: de ? "Mesh Improvements einschalten" : "Switch on Mesh Improvements",
                description: de
                  ? "Vortex schaltet Mesh Improvements im verwendeten Profil ein und bittet ums Bereitstellen."
                  : "Vortex switches Mesh Improvements on in the profile you use, and asks to deploy.",
                guide: [],
              },
            ]
          : sample.kind === "plugins.switched-off-but-loaded"
            ? [
                {
                  action: {
                    type: "disable-plugins",
                    plugins: ["Busy Streets.esp", "Water Mod - Busy Streets Patch.esp"],
                  },
                  label: de ? "Busy Streets.esp ausschalten" : "Switch off Busy Streets.esp",
                  description: "",
                  guide: [],
                },
              ]
            : sample.kind === "mods.collides-widely" && provider !== undefined
              ? [
                  {
                    action: { type: "disable-mod", mod: provider },
                    label: de ? "In Vortex ausschalten" : "Switch it off in Vortex",
                    description: "",
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

function places(lang: Lang, confirmed: boolean, manager = "vortex"): RenderedPlaces {
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
      ...(manager === "mo2"
        ? [
            {
              location: {
                role: "manager-instance" as const,
                path: "C:\\Users\\you\\AppData\\Local\\ModOrganizer\\Skyrim SE",
                origin: "standard" as const,
                found: true,
              },
              roleLabel: de
                ? "Mod Organizers Instanz (seine Einstellungen, das verwendete Profil und was er über jede Mod weiß)"
                : "Mod Organizer's instance (its settings, the profile in use and what it knows about each mod)",
              originLabel: standard,
            },
          ]
        : []),
    ],
    managers: {
      active: manager,
      chosen: confirmed,
      managers: [
        { id: "vortex", name: "Vortex", found: true, link: true, instances: [] },
        {
          id: "mo2",
          name: "Mod Organizer 2",
          found: true,
          link: false,
          instances: [
            {
              name: "Skyrim SE",
              path: "C:\\Users\\you\\AppData\\Local\\ModOrganizer\\Skyrim SE",
              portable: false,
            },
          ],
        },
      ],
    },
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
    loadOrder: sampleLoadOrder(),
    installedPatch: {
      objects: [
        { plugin: "Gone Mod.esp", localId: 0x10 },
        { plugin: "Texture Pack.esp", localId: 0x942 },
        { plugin: "Texture Pack.esp", localId: 0x952 },
      ],
      provider: { source: "vortex", id: "Mod Mallet Patch", name: "Mod Mallet Patch" },
    },
    // Switched off in Vortex; the decisions about it wait (ADR 0061).
    switchedOffPlugins: [
      {
        plugin: "Gone Mod.esp",
        mod: { source: "vortex", id: "Gone Mod", name: "Gone Mod" },
        loaded: false,
      },
      {
        plugin: "Busy Streets.esp",
        mod: { source: "vortex", id: "Busy Streets", name: "Busy Streets" },
        loaded: true,
        leftBehind: ["Busy Streets.bsa", "Busy Streets.esp"],
      },
    ],
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
  modInfo("Gone Mod", "1.1", { enabled: false }),
  modInfo("Busy Streets", "3.2", { enabled: false }),
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
  // Two doors on top of each other (ADR 0067): the mod's door 6 cm in
  // front of the town's, leading into the same house but not back.
  const doors = (x: number, y: number): MapSpot => {
    const town = { plugin: "Old Town.esp", localId: 0xc10 };
    const water = { plugin: "Water Mod.esp", localId: 0xc20 };
    const inside = { plugin: "Old Town.esp", localId: 0xc11 };
    const house = { location: { type: "interior" as const, cell: "OldTownInn" }, door: inside };
    return {
      finding: "doors.stacked:Old Town+Water Mod",
      kind: "door",
      location: { type: "exterior", world: "Tamriel", x, y },
      objects: "InnDoor01 / WaterInnDoor",
      console: `cow Tamriel ${x} ${y}`,
      parts: [
        { object: town, name: "InnDoor01", hideable: true, suggested: false },
        { object: water, name: "WaterInnDoor", hideable: true, suggested: true },
      ],
      doors: {
        flicker: false,
        doors: [
          {
            object: town,
            plugin: "Old Town.esp",
            name: "InnDoor01",
            leadsTo: house,
            leadsBack: true,
            walkable: true,
            otherSideWalkable: true,
            references: 3,
            front: false,
            offset: 0,
            suggested: true,
          },
          {
            object: water,
            plugin: "Water Mod.esp",
            name: "WaterInnDoor",
            broken: true,
            leadsBack: false,
            walkable: false,
            otherSideWalkable: true,
            references: 0,
            front: true,
            offset: 4,
            suggested: false,
          },
        ],
        plans: [
          { keep: town, hide: [water], changes: [], lost: [] },
          {
            keep: water,
            hide: [town],
            changes: [
              {
                type: "teleport",
                door: water,
                at: [x * 4096 + 2048, y * 4096 + 2040, 0],
                plugin: "Water Mod.esp",
                to: inside,
                arrival: [0, 100, 0],
                facing: [0, 0, 3142],
              },
              {
                type: "teleport",
                door: inside,
                at: [0, 0, 0],
                cell: "OldTownInn",
                plugin: "Old Town.esp",
                from: town,
                to: water,
                arrival: [x * 4096 + 2048, y * 4096 + 1900, 0],
              },
              {
                type: "navmesh-door",
                navmesh: { plugin: "Skyrim.esm", localId: 0x1234 },
                plugin: "Old Town.esp",
                triangle: 12,
                from: town,
                to: water,
              },
            ],
            lost: [],
          },
        ],
      },
    };
  };
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
      placed([7, 2, 0.5, 0.5], duplicate(7, 2, "Bridge / Lantern post", 0xb12)),
      placed([8, 2, 0.5, 0.5], duplicate(8, 2, "UpperShelf02 / Divider", 0xb20)),
      placed([5, 3, 0.5, 0.5], doors(5, 3)),
    ],
    // The docks of Old Town and Water Mod, built twice (ADR 0072): the two
    // docks with their pillars, but not Water Mod's long pier beyond.
    passages: [samplePassage(at(6, 1))],
  };
}

/** The docks of the sample's duplicates as one passage, as the scan would find it. */
function samplePassage(centre: [number, number, number]): MapPassage {
  const part = (plugin: string, localId: number, name: string) => ({
    object: { plugin, localId },
    name,
    hideable: true,
    suggested: false,
  });
  const dock = (plugin: string, seed: number, also: number, first: number) => [
    ...[-4, -3, -2, -1, 0, 1, 2, 3, 4].map((index) =>
      part(
        plugin,
        index === 0 ? seed : index === 2 ? also : first + index + 4,
        index === 0 ? "Dock" : index === 2 ? "Boat" : "Plank",
      ),
    ),
    ...[-4, -2, 0, 2, 4].map((index) => part(plugin, first + 20 + index + 4, "Pillar")),
  ];
  const [x, y, z] = centre;
  return {
    plugins: ["Old Town.esp", "Water Mod.esp"],
    location: { type: "exterior", world: "Tamriel", x: 6, y: 1 },
    console: "cow Tamriel 6 1",
    centre,
    min: [x - 360, y - 80, z],
    max: [x + 400, y + 80, z + 120],
    parts: [
      dock("Old Town.esp", 0xa10, 0xa20, 0x5000),
      [...dock("Water Mod.esp", 0xa11, 0xa21, 0x6000), part("Water Mod.esp", 0x6101, "Pier")],
    ],
    spots: 2,
    duplicates: 12,
    inside: [870, 820],
    volumes: [4200, 4500],
    severity: "doubled",
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
    tool(
      "xedit",
      "SSEEdit (Create or Patch)",
      "optional",
      "ready",
      two(
        "Looks inside plugins and can repair them.",
        "Schaut in Plugins hinein und kann sie reparieren.",
      ),
      { group: "mod-tool", label: "SSEEdit (Create or Patch)", path: "D:\\Tools\\SSEEdit.exe" },
    ),
    tool(
      "xedit-clean",
      "SSEEditQuickAutoClean (Clean Dirty Mods)",
      "optional",
      "missing",
      two(
        "Cleans a plugin of leftovers that can crash the game.",
        "Säubert ein Plugin von Überresten, die das Spiel abstürzen lassen können.",
      ),
      { group: "mod-tool", path: "D:\\Tools\\SSEEditQuickAutoClean.exe" },
    ),
  ];
  return {
    tools,
    attention: tools.filter((t) => t.tool.state !== "ready" && t.tool.need !== "optional").length,
  };
}

/** Made-up duels: faces, objects, and other files. */
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
        topics: [
          { topic: "objects", count: 3, winning: null },
          { topic: "meshes", count: 1, winning: null },
        ],
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
          {
            path: "meshes/actors/character/character assets/skeleton.nif",
            topic: "meshes",
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
    // Other files: a script, a menu and settings that differ, and two
    // files two house mods share byte for byte.
    {
      duel: {
        id: "duel:mod:vortex:Better Menus|mod:vortex:Quest Fixes",
        sides: [side("Better Menus", []), side("Quest Fixes", [])],
        topics: [{ topic: "files", count: 3, winning: 1 }],
        findings: ["assets.archive-over-archive:Better Menus+Quest Fixes"],
        files: [
          { path: "scripts/samplequestscript.pex", topic: "files", object: null, objectKind: null },
          { path: "interface/quest_journal.swf", topic: "files", object: null, objectKind: null },
          {
            path: "skse/plugins/samplemenus.json",
            topic: "files",
            object: null,
            objectKind: null,
          },
        ],
        suggestion: null,
        resolutions: [],
      },
      topicLabels: [de ? "3 andere Dateien" : "3 other files"],
      suggestion: null,
    },
    {
      duel: {
        id: "duel:mod:vortex:Hall House|mod:vortex:Inn House",
        sides: [side("Hall House", []), side("Inn House", [])],
        topics: [{ topic: "files", count: 2, winning: 0 }],
        findings: ["assets.archive-over-archive:Hall House+Inn House"],
        files: [
          { path: "scripts/sharedtoggle.pex", topic: "files", object: null, objectKind: null },
          {
            path: "scripts/source/sharedmarker.txt",
            topic: "files",
            object: null,
            objectKind: null,
          },
        ],
        suggestion: null,
        resolutions: [],
      },
      topicLabels: [de ? "2 andere Dateien" : "2 other files"],
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

/** A few bones of a standing figure, standing in for a skeleton: nothing to draw but lines. */
function sampleSkeleton(path: string): MeshView {
  const joints: Record<string, [number, number, number]> = {
    root: [0, 0, 0],
    pelvis: [0, 0, 60],
    spine: [0, 0, 90],
    neck: [0, 0, 115],
    head: [0, 0, 128],
    leftHand: [-30, 0, 80],
    rightHand: [30, 0, 80],
    leftFoot: [-10, 0, 2],
    rightFoot: [10, 0, 2],
  };
  const bones: [string, string][] = [
    ["root", "pelvis"],
    ["pelvis", "spine"],
    ["spine", "neck"],
    ["neck", "head"],
    ["neck", "leftHand"],
    ["neck", "rightHand"],
    ["pelvis", "leftFoot"],
    ["pelvis", "rightFoot"],
  ];
  const points = bones.flatMap(([from, to]) => [
    ...(joints[from] ?? [0, 0, 0]),
    ...(joints[to] ?? [0, 0, 0]),
  ]);
  return {
    path,
    archive: null,
    tint: null,
    shapes: [],
    bones: base64Of(new Float32Array(points)),
  };
}

/** A cube, standing in for a mesh; a skeleton for a path that names one. */
function sampleCube(path: string, colour = "#b07a4a"): MeshView {
  if (path.endsWith("skeleton.nif")) {
    return sampleSkeleton(path);
  }
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
        texture: samplePicture(colour),
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
let sampleManager = "vortex";
let linkInstalled = false;
const ignoredIds = new Set<string>();
let decided: Record<string, number> = {};
let ranked: string[] = [];
// A decision made while Gone Mod was on; it waits while the mod is off.
let chosen: Record<string, string | null> | null = {
  "objects.overlapping:pair:file:data/gone mod.esp+file:data/old town.esp": "Gone Mod.esp",
};

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
      game("oblivionremastered", "Oblivion Remastered", "OBSE64", null),
    ],
  };
}

export const sampleApi: Api = {
  // A scan takes a while and reports its steps, as a real one does.
  scan: async (lang) => {
    await simulateScan();
    return report(lang);
  },
  onScanProgress: onSampleScanProgress,
  lastReport: () => later(null),
  runAction: (action, lang) =>
    later(
      linkInstalled && action.type === "file-overrides"
        ? {
            outcome: { status: "applied" as const },
            message:
              lang === "de"
                ? "Vortex hat die Dateien umgestellt und stellt neu bereit."
                : "Vortex has switched the files and is deploying.",
          }
        : linkInstalled
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
  setVortexTool: (_add, lang) =>
    later({
      outcome: { status: "applied" as const },
      message:
        lang === "de" ? "Vortex hat die Änderung vorgenommen." : "Vortex has made the change.",
    }),
  places: (lang) => later(places(lang, confirmed, sampleManager)),
  placesForManager: (manager, _instance, lang) => later(places(lang, confirmed, manager)),
  confirmPlaces: (_named, manager, lang) => {
    confirmed = true;
    sampleManager = manager ?? sampleManager;
    return later(places(lang, confirmed, sampleManager));
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
  // Every move is written, after the masters the sample's objects come from.
  createMoves: (moves) =>
    later({
      path: "C:\\Users\\you\\AppData\\Local\\mod-mallet\\patches\\Mod Mallet - Moves.zip",
      moved: moves.moves.length,
      masters: [...new Set(["Skyrim.esm", ...moves.moves.map((each) => each.object.plugin)])],
      light: true,
      refused: [],
    }),
  createPaths: (repairs) =>
    later({
      path: "C:\\Users\\you\\AppData\\Local\\mod-mallet\\patches\\Mod Mallet - Paths.zip",
      navmeshes: repairs.places.flatMap((place) => place.navmeshes),
      masters: ["Skyrim.esm", "Water Mod.esp"],
      light: true,
      refused: [],
    }),
  savePatchReport: () =>
    later("C:\\Users\\you\\AppData\\Local\\mod-mallet\\patches\\Mod Mallet Patch - report.txt"),
  openFolder: async () => {},
  showDataFile: async () => {},
  toolbox: (lang) => later(toolbox(lang)),
  restartManager: () => later(undefined),
  // The sample has no pictures; the page shows its placeholder.
  gameArt: () => later(null),
  meshView: (path) => later(sampleCube(path)),
  catalogue: () => later(sampleCatalogue()),
  assetVersions: (models) => later(withChoices(sampleVersions(models))),
  assetView: (path, version) => later(sampleCube(path, SAMPLE_COLOURS[version] ?? "#8a8a8a")),
  assetChoices: () => later({ ...assetChoices }),
  setAssetChoice: (id, version) => {
    if (version === null) {
      delete assetChoices[id];
    } else {
      assetChoices[id] = version;
    }
    return later({ ...assetChoices });
  },
  // Up close, made-up bridges at every collision spot; no level of detail.
  areaObjects: (world, x, y, radius) => later(sampleArea(world, x, y, radius)),
  areaLand: () => later(null),
  worldLand: () => later(null),
  interiorObjects: () => later(null),
  areaMesh: (name) => later(sampleBox(name)),
  areaTexture: () => later(null),
  objectAnchors: (_world, objects) => later(objects.map(sampleAnchors)),
  // In the sample, a path of the game runs over the first object to hide.
  // Characters walk on the planks of Water Mod's dock at the docks (made
  // up): hiding them would leave them on thin air.
  // Cutting those paths out would leave the dock's far end cut off.
  hiddenWalkways: (worldOrCell, _x, _y, objects) => {
    const walkways = objects
      .filter(
        (object) =>
          object.plugin === "Water Mod.esp" &&
          (object.localId === 0xa11 ||
            object.localId === 0xa21 ||
            (object.localId >= 0x6000 && object.localId <= 0x6008)),
      )
      .map((object) => ({
        object,
        navmesh: "skyrim.esm:00A1B2",
        plugin: "Skyrim.esm",
        used: true,
        cell: `${worldOrCell} 6,1`,
        triangles: 6,
      }));
    const cutOff =
      walkways.length > 0
        ? [
            {
              cell: `${worldOrCell} 6,1`,
              centre: [25_100, 5_300, -120] as [number, number, number],
              squareMetres: 18,
              doors: [],
            },
          ]
        : [];
    return later({
      place: worldOrCell,
      found: objects,
      walkways,
      repair: {
        triangles: walkways.length * 6,
        safe: cutOff.length === 0,
        cutOff,
        lostDoors: [],
      },
    });
  },
  // Hiding the same planks cuts off a bench on the dock's far end, and
  // the way between the two ends of the dock goes round (made up).
  walkwayReach: (worldOrCell, _x, _y, objects) => {
    const hides = objects.some(
      (object) =>
        object.plugin === "Water Mod.esp" &&
        (object.localId === 0xa11 ||
          object.localId === 0xa21 ||
          (object.localId >= 0x6000 && object.localId <= 0x6008)),
    );
    const cell = `${worldOrCell} 6,1`;
    const place = (localId: number, name: string, position: [number, number, number]) => ({
      object: { plugin: "Skyrim.esm", localId },
      kind: "furniture" as const,
      name,
      position,
      cell,
    });
    return later({
      place: worldOrCell,
      after: "cut" as const,
      checked: hides ? 6 : 0,
      changes: hides
        ? [
            {
              outcome: "gone" as const,
              destination: {
                ...place(0x6201, "RailLeanMarker", [24_730, 5_120, -60]),
                object: { plugin: "Water Mod.esp", localId: 0x6201 },
              },
            },
            {
              outcome: "gone" as const,
              destination: {
                ...place(0x6202, "WallLeanMarker", [24_420, 5_150, -60]),
                object: { plugin: "Water Mod.esp", localId: 0x6202 },
              },
            },
            {
              outcome: "cut-off" as const,
              destination: place(0xb01, "Bench", [25_100, 5_300, -120]),
              before: {
                metres: 14,
                points: [
                  [24_400, 5_200, -120],
                  [24_800, 5_250, -120],
                  [25_100, 5_300, -120],
                ] as [number, number, number][],
              },
            },
            {
              outcome: "longer" as const,
              destination: place(0xb02, "Chair", [24_300, 4_900, -100]),
              from: { ...place(0xb03, "Crate", [24_900, 4_700, -110]), kind: "used" as const },
              before: {
                metres: 9,
                points: [
                  [24_900, 4_700, -110],
                  [24_600, 4_800, -105],
                  [24_300, 4_900, -100],
                ] as [number, number, number][],
              },
              after: {
                metres: 31,
                points: [
                  [24_900, 4_700, -110],
                  [25_000, 4_200, -110],
                  [24_500, 4_000, -105],
                  [24_100, 4_400, -100],
                  [24_300, 4_900, -100],
                ] as [number, number, number][],
              },
            },
          ]
        : [],
    });
  },
  // The repair of the dock's paths (made up): two pieces on the planks
  // taken out, four laid on the bed of the water below.
  walkwayRepair: (worldOrCell, x, y, objects) => {
    const walked = objects.some((object) => object.plugin === "Water Mod.esp");
    const [cx, cy] = [x ?? 25_000, y ?? 5_200];
    const deck = -60;
    const bed = -220;
    return later({
      place: worldOrCell,
      navmeshes: walked
        ? [
            {
              navmesh: "skyrim.esm:00A1B2",
              plugin: "Skyrim.esm",
              cell: `${worldOrCell} 6,1`,
              removed: [
                [
                  [cx - 200, cy - 100, deck],
                  [cx + 200, cy - 100, deck],
                  [cx + 200, cy + 100, deck],
                ],
                [
                  [cx - 200, cy - 100, deck],
                  [cx + 200, cy + 100, deck],
                  [cx - 200, cy + 100, deck],
                ],
              ],
              added: [
                [
                  [cx - 260, cy - 160, bed],
                  [cx, cy - 160, bed],
                  [cx, cy, bed],
                ],
                [
                  [cx, cy - 160, bed],
                  [cx + 260, cy - 160, bed],
                  [cx, cy, bed],
                ],
                [
                  [cx - 260, cy - 160, bed],
                  [cx, cy, bed],
                  [cx - 260, cy + 160, bed],
                ],
                [
                  [cx + 260, cy - 160, bed],
                  [cx + 260, cy + 160, bed],
                  [cx, cy, bed],
                ],
              ],
              water: 4,
              writable: true,
            },
          ]
        : [],
      areasLaid: walked ? 1 : 0,
      bareObjects: 0,
      lostDoors: [],
    });
  },
  // After the repair (made up): the bench is reached again over the bed of
  // the water; the dock's door is lost with its planks.
  walkwayRepairReach: (worldOrCell, _x, _y, objects) => {
    const walked = objects.some((object) => object.plugin === "Water Mod.esp");
    return later({
      place: worldOrCell,
      after: "repaired" as const,
      checked: walked ? 6 : 0,
      changes: walked
        ? [
            {
              outcome: "gone" as const,
              destination: {
                object: { plugin: "Water Mod.esp", localId: 0xa30 },
                kind: "door" as const,
                name: "Boathouse door",
                position: [25_050, 5_250, -60] as [number, number, number],
                cell: `${worldOrCell} 6,1`,
              },
            },
          ]
        : [],
    });
  },
  // Places on the planks lose their ground once the planks are hidden; the
  // bank below is reached (made up).
  spotsReach: (worldOrCell, _x, _y, spots, hidden) =>
    later({
      place: worldOrCell,
      spots: spots.map(({ object, at }) => {
        const onPlanks = object.plugin === "Water Mod.esp" && hidden.length > 0;
        return {
          object,
          at,
          state: onPlanks ? ("off-paths" as const) : ("reached" as const),
          metresOff: onPlanks ? 4 : 0,
          ground: [at[0], at[1] - 180, at[2] - 60] as [number, number, number],
        };
      }),
    }),
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
  // Every other file in the game's archive and in both mods' archives: the
  // two mods' copies the same for files named `shared`, different else.
  compareFiles: (paths, mods) =>
    later(
      paths.map((path) => {
        const [first = "First Mod", second = "Second Mod"] = mods;
        const shared = path.includes("shared");
        const text = (who: string) =>
          path.endsWith(".json")
            ? `{\n  "enabled": true,\n  "radius": ${who === first ? 4096 : 8192},\n  "fadeIn": 1.5\n}\n`
            : null;
        const script = (who: string) =>
          path.endsWith(".pex")
            ? {
                name: "SampleScript",
                hasDebugInfo: true,
                functions: [
                  { state: "", name: "OnInit", kind: "plain" as const, fingerprint: "3:a" },
                  {
                    state: "",
                    name: "OnActivate",
                    kind: "plain" as const,
                    fingerprint: who === first || shared ? "5:b" : "7:c",
                  },
                  ...(who === second && !shared
                    ? [
                        {
                          state: "Busy",
                          name: "OnUpdate",
                          kind: "plain" as const,
                          fingerprint: "2:d",
                        },
                      ]
                    : []),
                ],
              }
            : null;
        const version = (who: string, archive: string, sameAs: number | null) => ({
          archive,
          staged: false,
          provider: { source: "vortex", id: who, name: who },
          size: who === first || shared ? 1246 : 1530,
          modified: 1_700_000_000,
          sameAs,
          text: text(shared ? first : who),
          image: null,
          script: script(who),
        });
        return {
          path,
          versions: [
            {
              archive: "Skyrim - Misc.bsa",
              staged: false,
              provider: null,
              size: 1100,
              modified: 1_225_494_000,
              sameAs: null,
              text: null,
              image: null,
              script: null,
            },
            version(first, `${first}.bsa`, null),
            version(second, `${second}.bsa`, shared ? 1 : null),
          ],
        };
      }),
    ),
  logFile: () => later("C:\\Users\\you\\AppData\\Local\\mod-mallet\\logs\\mod-mallet.log"),
};

// ---- objects up close ------------------------------------------------------------

/** The half sizes of the made-up models up close, in game units. */
const SAMPLE_MODELS: Record<string, [number, number, number]> = {
  "sample\\plank.nif": [40, 14, 4],
  "sample\\pillar.nif": [6, 6, 30],
  "sample\\barrel.nif": [16, 16, 22],
  "sample\\rock.nif": [45, 35, 25],
  "landscape\\trees\\sampletree.nif": [20, 20, 120],
  "sample\\cup.nif": [5, 5, 6],
  "sample\\shelf.nif": [40, 12, 30],
  "sample\\book.nif": [4, 6, 8],
  "sample\\post.nif": [5, 5, 85],
  "sample\\roof.nif": [115, 115, 8],
  "sample\\bench.nif": [40, 12, 12],
  "sample\\signpost.nif": [4, 4, 50],
  "sample\\pole.nif": [3, 3, 110],
};

/**
 * What stands around a point up close: at every collision spot of the
 * sample, two bridges of planks on pillars, one of Old Town and one of
 * Texture Pack, built into each other; a barrel of Old Town a little
 * away that belongs to neither bridge; and rocks of the game.
 */
function sampleArea(world: string, cx: number, cy: number, radius: number): AreaObjects {
  const cellX = Math.floor(cx / 4096);
  const cellY = Math.floor(cy / 4096);
  const ground = sampleHeight(sampleTerrain(), cx / 4096, cy / 4096);
  const objects: AreaObject[] = [];
  const place = (
    plugin: string,
    localId: number,
    model: string,
    position: [number, number, number],
    turn: number,
    name: string,
  ) =>
    objects.push({
      id: { plugin, localId },
      plugin,
      model,
      name,
      position,
      rotation: [0, 0, turn],
      scale: 1,
    });
  const bridge = (
    plugin: string,
    seed: number,
    first: number,
    turn: number,
    lift: number,
    also: number | null = null,
  ) => {
    const along: [number, number] = [Math.cos(turn), -Math.sin(turn)];
    for (let index = -4; index <= 4; index++) {
      const at: [number, number, number] = [
        cx + along[0] * index * 76,
        cy + along[1] * index * 76,
        ground + 60 + lift,
      ];
      const id = index === 0 ? seed : index === 2 && also !== null ? also : first + index + 4;
      place(plugin, id, "sample\\plank.nif", at, turn, "Plank");
      if (index % 2 === 0) {
        place(
          plugin,
          first + 20 + index + 4,
          "sample\\pillar.nif",
          [at[0], at[1], ground + 26 + lift],
          0,
          "Pillar",
        );
      }
    }
  };
  // A shelf of Old Town against a divider of Water Mod, with three books on
  // the shelf's top: one Old Town placed, one of the game that Old Town
  // moved there (its version wins), and one of Patch Hub. Hiding the shelf
  // takes the first two along; the third is another mod's.
  if (cellX === 8 && cellY === 2) {
    place("Old Town.esp", 0xb20, "sample\\shelf.nif", [cx, cy, ground + 30], 0, "UpperShelf02");
    place("Water Mod.esp", 0xb21, "sample\\pillar.nif", [cx + 46, cy, ground + 30], 0, "Divider");
    place("Old Town.esp", 0xb22, "sample\\book.nif", [cx - 20, cy, ground + 68], 0, "Book");
    objects.push({
      id: { plugin: "Skyrim.esm", localId: 0xb23 },
      plugin: "Old Town.esp",
      model: "sample\\book.nif",
      name: "Iron",
      position: [cx, cy, ground + 68],
      rotation: [0, 0, 0],
      scale: 1,
    });
    place("Patch Hub.esp", 0xb24, "sample\\book.nif", [cx + 20, cy, ground + 68], 0, "Book");
    return { world, centre: [cx, cy], radius, objects };
  }
  // The docks of the sample's duplicates: Old Town and Water Mod place the same.
  const docks = cellX === 6 && cellY === 1;
  const lantern = cellX === 7 && cellY === 2;
  bridge(
    "Old Town.esp",
    docks ? 0xa10 : lantern ? 0xb12 : 0x800 + cellX * 16 + cellY,
    0x5000,
    0,
    0,
    docks ? 0xa20 : null,
  );
  if (docks) {
    bridge("Water Mod.esp", 0xa11, 0x6000, 0.08, 1, 0xa21);
    // A third mod built on the docks too: two crates and a pillar of Patch Hub.
    place(
      "Patch Hub.esp",
      0x8001,
      "sample\\barrel.nif",
      [cx + 40, cy + 60, ground + 86],
      0,
      "Crate",
    );
    place(
      "Patch Hub.esp",
      0x8002,
      "sample\\barrel.nif",
      [cx - 90, cy + 70, ground + 86],
      0,
      "Crate",
    );
    place(
      "Patch Hub.esp",
      0x8003,
      "sample\\pillar.nif",
      [cx + 152, cy - 40, ground + 26],
      0,
      "Post",
    );
    // A long pier of Water Mod that touches its dock and runs on far beyond
    // the spot, as a town's walls run on from its gate.
    for (let index = 1; index <= 40; index++) {
      place(
        "Water Mod.esp",
        0x6100 + index,
        "sample\\plank.nif",
        [
          cx + (4 + index) * 76 * Math.cos(0.08),
          cy - (4 + index) * 76 * Math.sin(0.08),
          ground + 61,
        ],
        0.08,
        "Pier",
      );
    }
  } else if (lantern) {
    // A whole bridge of Old Town against a single lantern post of Water
    // Mod: a clear case for the suggestion by size.
    place(
      "Water Mod.esp",
      0xb13,
      "sample\\pillar.nif",
      [cx + 30, cy + 10, ground + 40],
      0,
      "Lantern post",
    );
    // A pavilion of Old Town nearby: a roof on four posts, a bench with a cup
    // on it a step away, around a signpost of the game, and a banner pole of
    // Water Mod through its roof. The magic wand at "the whole structure"
    // takes a post's roof and the other posts along, not the pole; at level
    // 1 the bench takes its cup (ADR 0055).
    const [px, py] = [cx, cy + 340];
    const corners: [number, number][] = [
      [-90, -90],
      [90, -90],
      [90, 90],
      [-90, 90],
    ];
    corners.forEach(([dx, dy], index) => {
      place(
        "Old Town.esp",
        0xb30 + index,
        "sample\\post.nif",
        [px + dx, py + dy, ground + 85],
        0,
        "Pavilion post",
      );
    });
    place("Old Town.esp", 0xb34, "sample\\roof.nif", [px, py, ground + 178], 0, "Pavilion roof");
    place("Old Town.esp", 0xb35, "sample\\bench.nif", [px + 200, py, ground + 12], 0, "Bench");
    place("Old Town.esp", 0xb38, "sample\\cup.nif", [px + 210, py, ground + 30], 0, "Cup");
    place("Skyrim.esm", 0xb36, "sample\\signpost.nif", [px, py, ground + 50], 0, "Signpost");
    place(
      "Water Mod.esp",
      0xb37,
      "sample\\pole.nif",
      [px + 60, py + 40, ground + 110],
      0,
      "Banner pole",
    );
  } else {
    bridge("Texture Pack.esp", 0x900 + cellX * 16 + cellY, 0x6000, 0.55, 3);
  }
  place(
    "Old Town.esp",
    0x5100,
    "sample\\barrel.nif",
    [cx - 260, cy + 240, ground + 22],
    0,
    "Barrel",
  );
  // A tree of Old Town whose crown touches its bridge, and a cup on a
  // plank: they touch the bridge but are no part of it.
  place(
    "Old Town.esp",
    0x5101,
    "landscape\\trees\\sampletree.nif",
    [cx - 152, cy + 30, ground + 100],
    0,
    "Tree",
  );
  place("Old Town.esp", 0x5102, "sample\\cup.nif", [cx + 76, cy, ground + 70], 0, "Cup");
  for (let index = 0; index < 6; index++) {
    const angle = (index / 6) * Math.PI * 2;
    place(
      "Skyrim.esm",
      0x7000 + index,
      "sample\\rock.nif",
      [cx + Math.cos(angle) * 520, cy + Math.sin(angle) * 520, ground + 20],
      angle,
      "Rock",
    );
  }
  // On the docks, two places to lean on Water Mod's planks, and a bench of
  // the game on the bank (ADR 0069).
  const action = (
    plugin: string,
    localId: number,
    kind: ActionPlace["kind"],
    base: string,
    position: [number, number, number],
    drawn: boolean,
  ): ActionPlace => ({
    object: { plugin, localId },
    plugin,
    kind,
    idleMarker: false,
    drawn,
    base,
    position,
    rotation: [0, 0, 1.6],
  });
  const actions: ActionPlace[] = docks
    ? [
        action(
          "Water Mod.esp",
          0x6201,
          "lean",
          "RailLeanMarker",
          [cx + 2 * 76, cy - 12, ground + 61],
          false,
        ),
        action(
          "Water Mod.esp",
          0x6202,
          "lean",
          "WallLeanMarker",
          [cx - 2 * 76, cy + 12, ground + 61],
          false,
        ),
        action("Skyrim.esm", 0xb01, "sit", "BenchMarker", [cx - 300, cy - 200, ground], false),
      ]
    : [];
  return { world, centre: [cx, cy], radius, objects, actions };
}

/** A box of the made-up models, as the backend sends a model up close. */
function sampleBox(name: string): LodMesh | null {
  const half = SAMPLE_MODELS[name];
  if (half === undefined) {
    return null;
  }
  const positions: number[] = [];
  const indices: number[] = [];
  const corners = (face: number[][]) => {
    const start = positions.length / 3;
    for (const corner of face) {
      positions.push(...corner.map((value, axis) => value * (half[axis] ?? 1)));
    }
    indices.push(start, start + 1, start + 2, start, start + 2, start + 3);
  };
  corners([
    [-1, -1, -1],
    [-1, 1, -1],
    [1, 1, -1],
    [1, -1, -1],
  ]);
  corners([
    [-1, -1, 1],
    [1, -1, 1],
    [1, 1, 1],
    [-1, 1, 1],
  ]);
  corners([
    [-1, -1, -1],
    [1, -1, -1],
    [1, -1, 1],
    [-1, -1, 1],
  ]);
  corners([
    [1, -1, -1],
    [1, 1, -1],
    [1, 1, 1],
    [1, -1, 1],
  ]);
  corners([
    [1, 1, -1],
    [-1, 1, -1],
    [-1, 1, 1],
    [1, 1, 1],
  ]);
  corners([
    [-1, 1, -1],
    [-1, -1, -1],
    [-1, -1, 1],
    [-1, 1, 1],
  ]);
  return {
    shapes: [
      {
        positions: base64Of(new Float32Array(positions)),
        indices: base64Of(new Uint32Array(indices)),
        uvs: "",
        texture: null,
        alphaTest: false,
      },
    ],
  };
}

/** A made-up catalogue: some things of the game, of the Creation Club and of mods. */
function sampleCatalogue(): Catalogue {
  const item = (
    id: string,
    category: CatalogueItem["category"],
    name: string,
    model: string,
    extra: Partial<CatalogueItem> = {},
  ): CatalogueItem => ({
    id,
    category,
    name,
    editorId: name.replace(/[^A-Za-z]/g, ""),
    plugin: id.split("/")[0] ?? "skyrim.esm",
    models: [`meshes/${model}.nif`],
    ...extra,
  });
  const items: CatalogueItem[] = [
    item("skyrim.esm/00013989", "sword", "Steel Sword", "weapons/steel/steelsword", {
      material: "Steel",
      changedBy: ["better blades.esp"],
      variants: 24,
    }),
    item("skyrim.esm/00013983", "greatsword", "Steel Greatsword", "weapons/steel/steelgreatsword", {
      material: "Steel",
    }),
    item("skyrim.esm/0001397e", "dagger", "Iron Dagger", "weapons/iron/irondagger", {
      material: "Iron",
    }),
    item("skyrim.esm/0001359d", "bow", "Hunting Bow", "weapons/longbow/longbow", {
      material: "Wood",
    }),
    item("skyrim.esm/00013952", "body", "Steel Armor", "armor/steel/cuirass_go", {
      material: "Steel",
      models: ["meshes/armor/steel/cuirass_go.nif", "meshes/armor/steel/m/cuirass_1.nif"],
      variants: 12,
    }),
    item("skyrim.esm/00013954", "head", "Steel Helmet", "armor/steel/helmet_go", {
      material: "Steel",
    }),
    item("skyrim.esm/00013951", "feet", "Steel Boots", "armor/steel/boots_go", {
      material: "Steel",
    }),
    item("skyrim.esm/00013953", "hands", "Steel Gauntlets", "armor/steel/gauntlets_go", {
      material: "Steel",
    }),
    item("skyrim.esm/00013955", "shield", "Steel Shield", "armor/steel/shield", {
      material: "Steel",
    }),
    item(
      "skyrim.esm/0001b3a1",
      "body",
      "Whiterun Guard's Armor",
      "armor/whiterunguard/cuirass_go",
      {
        material: "Imperial",
      },
    ),
    item("skyrim.esm/0001b3a3", "head", "Guard's Helmet", "armor/guard/helmet_go", {
      material: "Iron",
      changedBy: ["better blades.esp", "guard variety.esp"],
    }),
    item("skyrim.esm/0001b3a4", "feet", "Guard's Boots", "armor/guard/boots_go"),
    item("skyrim.esm/0001b3a5", "hands", "Guard's Gauntlets", "armor/guard/gauntlets_go"),
    item("skyrim.esm/0001b3a2", "shield", "Whiterun Guard's Shield", "armor/whiterunguard/shield", {
      material: "Iron",
    }),
    item("skyrim.esm/0001b3a6", "jewellery", "Gold Ring", "clothes/ring/goldring"),
    item("skyrim.esm/0002c3a1", "potion", "Potion of Healing", "clutter/potions/healthpotion"),
    item("skyrim.esm/0002c3a2", "book", "The Lusty Argonian Maid", "clutter/books/book01"),
    item("skyrim.esm/0002c3a3", "misc", "Tankard", "clutter/tankard01"),
    item("skyrim.esm/0002c3a4", "container", "Chest", "clutter/containers/chest01"),
    item("skyrim.esm/0002c3a5", "furniture", "Chair", "furniture/chair01"),
    item(
      "skyrim.esm/000a2c94",
      "character",
      "Lydia",
      "actors/character/facegendata/facegeom/skyrim.esm/000a2c94",
      {
        race: "Nord",
        outfit: "skyrim.esm/0001b3b0",
      },
    ),
    item(
      "skyrim.esm/0001414d",
      "character",
      "Ulfric Stormcloak",
      "actors/character/facegendata/facegeom/skyrim.esm/0001414d",
      {
        race: "Nord",
        changedBy: ["face overhaul.esp"],
      },
    ),
    item("ccbgssse001-fish.esm/00000d70", "misc", "Fishing Rod", "clutter/fishing/rod", {
      plugin: "ccbgssse001-fish.esm",
    }),
    item("laser blades.esp/00000801", "sword", "Laser Blade", "laserblades/blade", {
      material: "Plasma",
    }),
    item("laser blades.esp/00000802", "greatsword", "Laser Greatblade", "laserblades/greatblade", {
      material: "Plasma",
    }),
    item("guard variety.esp/00000901", "head", "Whiterun Guard's Hood", "guardvariety/hood", {
      material: "Leather",
    }),
  ];
  return {
    items: items.sort((a, b) => (a.name ?? "").localeCompare(b.name ?? "")),
    outfits: [
      {
        id: "skyrim.esm/0001b3b0",
        editorId: "GuardWhiterunOutfit",
        plugin: "skyrim.esm",
        worn: [
          "skyrim.esm/0001b3a1",
          "skyrim.esm/0001b3a3",
          "guard variety.esp/00000901",
          "skyrim.esm/0001b3a4",
          "skyrim.esm/0001b3a5",
          "skyrim.esm/0001b3a2",
        ],
        weapons: ["skyrim.esm/00013989", "skyrim.esm/0001359d"],
        characters: 48,
      },
      {
        id: "skyrim.esm/0001b3b1",
        editorId: "ArmorSteelOutfit",
        plugin: "skyrim.esm",
        worn: [
          "skyrim.esm/00013952",
          "skyrim.esm/00013954",
          "skyrim.esm/00013951",
          "skyrim.esm/00013953",
        ],
        weapons: ["skyrim.esm/00013983"],
        characters: 7,
      },
    ],
    sources: [
      { plugin: "skyrim.esm", origin: "game" },
      { plugin: "ccbgssse001-fish.esm", origin: "creationClub" },
      { plugin: "better blades.esp", origin: "mod", mod: sampleModRef("Better Blades") },
      { plugin: "guard variety.esp", origin: "mod", mod: sampleModRef("Guard Variety") },
      { plugin: "face overhaul.esp", origin: "mod", mod: sampleModRef("Face Overhaul") },
      { plugin: "laser blades.esp", origin: "mod", mod: sampleModRef("Laser Blades") },
    ],
  };
}

/** A mod of the sample, as the mod manager names it. */
function sampleModRef(name: string) {
  return { source: "vortex", id: `${name}-1234-1-0`, name };
}

/** The colour each version of the sample paints its cube in. */
const SAMPLE_COLOURS: Record<string, string> = {
  game: "#9a9a9a",
  "mod:Better Blades-1234-1-0": "#c9a227",
  "mod:Guard Variety-1234-1-0": "#5b8bd6",
  "mod:Face Overhaul-1234-1-0": "#d27c5a",
  "mod:Sharp Steel-1234-1-0": "#5ac08a",
  "mod:Rusty Steel-1234-1-0": "#a0522d",
  "mod:Shiny Steel-1234-1-0": "#e0e0ff",
  "mod:Blood Stains-1234-1-0": "#8b1a1a",
  "archive:ccbgssse001-fish.bsa": "#3fa7a7",
};

/** The versions of things put into the game in the sample, by thing. */
const assetChoices: Record<string, string> = {};

/** The versions with what would put each into the game, and the way back. */
function withChoices(versions: AssetVersions): AssetVersions {
  const loose = versions.versions.flatMap((version) =>
    version.source.type === "mod" ? [version.source.mod] : [],
  );
  return {
    ...versions,
    reset:
      loose.length === 0 ? null : { type: "file-overrides", files: versions.files, provide: loose },
    versions: versions.versions.map((version) => ({
      ...version,
      choice:
        version.shown.length === version.files.length
          ? null
          : {
              type: "file-overrides",
              files: version.files,
              leaveOut: loose.filter(
                (mod) => version.source.type !== "mod" || mod.id !== version.source.mod.id,
              ),
              provide: version.source.type === "mod" ? [version.source.mod] : [],
            },
    })),
  };
}

/** Made-up versions of a thing: the steel sword has many, others one or two. */
function sampleVersions(models: string[]): AssetVersions {
  const model = models[0] ?? "meshes/unknown.nif";
  const texture = model.replace(/^meshes\//, "textures/").replace(/\.nif$/, ".dds");
  const files = [...models, texture];
  const mod = (name: string, own: string[], shown: string[] = []): AssetVersion => ({
    id: `mod:${name}-1234-1-0`,
    source: { type: "mod", mod: sampleModRef(name) },
    files: own,
    shown,
  });
  const game = (shown: string[]): AssetVersion => ({
    id: "game",
    source: { type: "game" },
    files,
    shown,
  });
  if (model.includes("steelsword")) {
    return {
      files,
      versions: [
        mod("Better Blades", [texture], [texture]),
        game(models),
        mod("Sharp Steel", files),
        mod("Rusty Steel", [texture]),
        mod("Shiny Steel", [texture]),
        mod("Blood Stains", [texture]),
      ],
    };
  }
  if (model.includes("guard") || model.includes("facegeom")) {
    return {
      files,
      versions: [
        mod(model.includes("guard") ? "Guard Variety" : "Face Overhaul", files, files),
        game([]),
      ],
    };
  }
  if (model.includes("fishing")) {
    return {
      files,
      versions: [
        {
          id: "archive:ccbgssse001-fish.bsa",
          source: { type: "archive", name: "ccbgssse001-fish.bsa" },
          files,
          shown: files,
        },
      ],
    };
  }
  return { files, versions: [game(files)] };
}

/** A made-up load order: the game, its Creation Club content, then mods and their patches. */
function sampleLoadOrder(): LoadedPlugin[] {
  const game = ["Skyrim.esm", "Update.esm", "Dawnguard.esm", "HearthFires.esm", "Dragonborn.esm"];
  const order: LoadedPlugin[] = game.map((name, index) => ({
    name,
    origin: "game",
    masters: index === 0 ? [] : ["Skyrim.esm"],
  }));
  for (const name of [
    "ccBGSSSE001-Fish.esm",
    "ccQDRSSE001-SurvivalMode.esl",
    "ccBGSSSE037-Curios.esl",
    "ccBGSSSE025-AdvDSGS.esm",
    "_ResourcePack.esl",
  ]) {
    order.push({
      name,
      origin: "creationClub",
      masters: name.startsWith("_") ? [] : ["Skyrim.esm", "Update.esm"],
    });
  }
  const vanilla = [
    "Skyrim.esm",
    "Update.esm",
    "Dawnguard.esm",
    "HearthFires.esm",
    "Dragonborn.esm",
  ];
  const add = (name: string, mod: string, masters: string[]) =>
    order.push({
      name,
      origin: "mod",
      masters,
      provider: { source: "vortex", id: mod, name: mod },
    });
  add("Unofficial Patch.esp", "Unofficial Patch", [...vanilla, "ccBGSSSE001-Fish.esm"]);
  add("Lighting Base.esm", "Lighting", ["Skyrim.esm", "Update.esm"]);
  add("Lighting - Resources.esm", "Lighting Resources", ["Skyrim.esm", "Lighting Base.esm"]);
  add("Old Mod.esp", "Old Mod", ["Skyrim.esm", "Update.esm"]);
  add("Old Town.esp", "Old Town", ["Skyrim.esm", "Update.esm", "Dawnguard.esm"]);
  add("Water Mod.esp", "Water Mod", [
    "Skyrim.esm",
    "Update.esm",
    "Dawnguard.esm",
    "Dragonborn.esm",
  ]);
  add("Water Mod - Rivers.esp", "Water Mod", ["Skyrim.esm", "Water Mod.esp"]);
  add("Cut Content.esp", "Cut Content", ["Skyrim.esm", "Update.esm", "Unofficial Patch.esp"]);
  add("Texture Pack.esp", "Texture Pack", ["Skyrim.esm"]);
  for (const place of ["Riverwood", "Whiterun", "Solitude", "Markarth", "Riften", "Windhelm"]) {
    add(`Lighting - ${place}.esp`, "Lighting", [
      "Skyrim.esm",
      "Lighting Base.esm",
      "Lighting - Resources.esm",
    ]);
  }
  add("Cut Content - Old Town Patch.esp", "Patch Hub", [
    "Skyrim.esm",
    "Cut Content.esp",
    "Old Town.esp",
  ]);
  add("Cut Content - Lighting Patch.esp", "Patch Hub", ["Cut Content.esp", "Lighting Base.esm"]);
  add("Old Town - Lighting Patch.esp", "Patch Hub", ["Old Town.esp", "Lighting - Whiterun.esp"]);
  add("Water Mod - Old Town Patch.esp", "Patch Hub", ["Water Mod.esp", "Old Town.esp"]);
  add("City - Dungeons Patch.esp", "Patch Hub", ["Skyrim.esm", "Old Town.esp", "Dungeons.esp"]);
  add("Cut Content - Water Patch.esp", "Cut Content Patches", [
    "Cut Content.esp",
    "Water Mod - Rivers.esp",
  ]);
  add("Face Overhaul.esp", "Face Overhaul", ["Skyrim.esm", "Update.esm", "Cut Content.esp"]);
  return order;
}
