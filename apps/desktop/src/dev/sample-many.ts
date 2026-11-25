// Many findings for the sample, shown with `?many` in the address: made-up
// mods in the numbers a large real setup has, so that the task page can be
// looked at full: dozens of doors cut off, walkways the patch hides,
// notes about paths undone, textures that do not reach
// replaced meshes, missing textures that are only a guess. Every name is
// made up.

import type { RenderedFinding } from "../generated/rendered-report";
import type { Lang } from "../i18n";

/** Whether the sample adds the many findings. */
export const SAMPLE_MANY =
  typeof window !== "undefined" && new URLSearchParams(window.location.search).has("many");

const MODS = [
  "Northwatch Keep",
  "Ashen Hollow",
  "Greywater Docks",
  "Stonehollow Mine",
  "Frostmere Village",
  "Old Road Inns",
  "Riverbend Farms",
  "Hearth and Hall",
  "Duskwood Caves",
  "Highmoor Castle",
  "Wayfarer Camps",
  "Saltmarsh Huts",
];

const pick = (index: number) => MODS[index % MODS.length] ?? "Northwatch Keep";

function finding(
  lang: Lang,
  kind: string,
  key: string,
  mod: string,
  severity: RenderedFinding["finding"]["severity"],
  confidence: RenderedFinding["finding"]["confidence"],
  title: string,
  headline: string,
  args: RenderedFinding["finding"]["args"],
): RenderedFinding {
  const de = lang === "de";
  return {
    finding: {
      id: `${kind}:many:${key}`,
      kind,
      severity,
      confidence,
      subject: { type: "setup" },
      args,
      provider: { source: "vortex", id: mod, name: mod },
      previews: [],
    },
    title,
    headline,
    summary: headline,
    explanation: de
      ? "Ein erfundener Befund für die Beispieldaten."
      : "A made-up finding for the sample.",
    remedy: de ? "Hier stünde, was zu tun ist." : "What to do would be here.",
    severityLabel: severity,
    confidenceLabel: confidence,
    actions: [],
  };
}

/** The many findings, in the window's language. */
export function manyFindings(lang: Lang): RenderedFinding[] {
  const de = lang === "de";
  const found: RenderedFinding[] = [];
  for (let index = 0; index < 33; index++) {
    const winner = pick(index * 5 + 1);
    const loser = pick(index * 3);
    const count = 1 + ((index * 7) % 4);
    found.push(
      finding(
        lang,
        "navmeshes.door-cut-off",
        `${index}`,
        loser,
        "warning",
        "likely",
        de
          ? "Figuren können Türen einer Mod nicht mehr benutzen"
          : "Characters can no longer use doors of a mod",
        de
          ? `${winner}.esp schneidet ${count} ${count === 1 ? "Tür" : "Türen"} von ${loser}.esp ab`
          : `${winner}.esp cuts off ${count} ${count === 1 ? "door" : "doors"} of ${loser}.esp`,
        { winner: `${winner}.esp`, loser: `${loser}.esp`, count, places: "Tamriel 3,-4" },
      ),
    );
  }
  for (let index = 0; index < 9; index++) {
    const meshMod = pick(index + 4);
    const textureMod = pick(index + 9);
    const count = 2 + ((index * 11) % 50);
    found.push(
      finding(
        lang,
        "meshes.retexture-bypassed",
        `${index}`,
        meshMod,
        "warning",
        "likely",
        de
          ? "Textur-Mod erreicht ersetzte Modelle nicht"
          : "A texture mod does not reach replaced meshes",
        de
          ? `${textureMod} erreicht ${count} Modelle von ${meshMod} nicht`
          : `${textureMod} does not reach ${count} meshes of ${meshMod}`,
        { meshMod, textureMod, count },
      ),
    );
  }
  for (let index = 0; index < 20; index++) {
    const mod = pick(index * 7 + 2);
    const count = 1 + ((index * 13) % 30);
    found.push(
      finding(
        lang,
        "meshes.texture-missing",
        `${index}`,
        mod,
        "warning",
        "hint",
        de
          ? "Modelle einer Mod verwenden Texturen, die im Spiel fehlen"
          : "Meshes of a mod use textures that are not in the game",
        de ? `${count} Texturen fehlen in ${mod}` : `${count} textures missing in ${mod}`,
        { count, meshes: count },
      ),
    );
  }
  for (let index = 0; index < 7; index++) {
    const mod = pick(index * 2 + 3);
    found.push(
      finding(
        lang,
        "patch.hides-walkway",
        `${index}`,
        mod,
        "warning",
        "likely",
        de
          ? "Der Patch blendet etwas aus, über das Figuren gehen"
          : "The patch hides something characters walk on",
        de
          ? `Der Patch blendet einen Steg von ${mod} aus, über den Figuren gehen`
          : `The patch hides a walkway of ${mod} that characters walk on`,
        { count: 1 + (index % 3) },
      ),
    );
  }
  for (let index = 0; index < 40; index++) {
    const mod = pick(index + 5);
    found.push(
      finding(
        lang,
        "navmeshes.changes-undone",
        `${index}`,
        mod,
        "info",
        "likely",
        de ? "Ein Plugin macht Wegänderungen rückgängig" : "A plugin undoes changes to paths",
        de
          ? `${pick(index + 1)}.esp macht Wege von ${mod}.esp rückgängig`
          : `${pick(index + 1)}.esp undoes paths of ${mod}.esp`,
        { count: 1 + (index % 4) },
      ),
    );
  }
  return found;
}
