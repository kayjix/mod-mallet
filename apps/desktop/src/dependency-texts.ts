// The texts of the plugin dependencies (ADR 0068), in every language of
// the app.

import { useSession } from "./Actions";
import type { WorldOrigin } from "./dependencies";
import type { Lang } from "./i18n";

/** What the dependency diagram and the dependencies of a mod say. */
export interface DependencyTexts {
  /** The tab of the Mods page with the list of mods. */
  tabList: string;
  /** The tab of the Mods page with the whole diagram. */
  tabDependencies: string;
  /** The heading of the dependencies on a mod's page. */
  section: string;
  /** What the diagram shows, behind the ⓘ. */
  explain: string;
  /** The one-line summary of what a mod's plugins need. */
  needsSummary: (names: string[], more: number) => string;
  /** The summary when they need nothing but each other. */
  needsNothing: string;
  /** The one-line summary of what needs a mod's plugins. */
  neededBySummary: (plugins: number, mods: number) => string;
  /** The summary when nothing else needs them. */
  neededByNothing: string;
  /** The heading of the plugins needed, with the side they are on. */
  needs: string;
  /** The heading of the plugins that need them, with the side they are on. */
  neededBy: string;
  /** Where to look in the diagram for the plugins needed. */
  needsWhere: string;
  /** Where to look in the diagram for the plugins that need them. */
  neededByWhere: string;
  /** A plugin that is needed but not loaded. */
  notLoaded: string;
  /** What a plugin that is not loaded does to the game. */
  notLoadedNote: string;
  /** The plugins a mod brings. */
  ownPlugins: (names: string) => string;
  /** The button that shows a long list in full. */
  showAll: (count: number) => string;
  /** The button that shortens it again. */
  showFewer: string;
  /** The search field of the diagram. */
  search: string;
  /** How many plugins and links the diagram shows. */
  stats: (plugins: number, links: number) => string;
  /** What to do when nothing is picked. */
  pickHint: string;
  /** The heading of the plugins most needed. */
  mostNeeded: string;
  /** How many plugins need one plugin. */
  dependentsCount: (count: number) => string;
  /** A plugin's place in the load order. */
  place: (place: number, total: number) => string;
  /** The mod a plugin comes from. */
  fromMod: (name: string) => string;
  /** Where a stretch of the load order comes from. */
  origins: Record<WorldOrigin, string>;
  /** The button that undoes the pick. */
  clear: string;
  /** What the diagram shows, for screen readers. */
  diagramLabel: (plugins: number, links: number) => string;
  /** When the last scan did not record the load order. */
  noLoadOrder: string;
  /** How many plugins a search found. */
  found: (count: number) => string;
  /** The heading of the plugins that would lose a master if a mod went. */
  wouldLose: (count: number) => string;
  /** The button that shows a plugin in the dependency diagram. */
  inDiagram: (plugin: string) => string;
  /** The button that opens a mod's page. */
  modPage: (name: string) => string;
}

const listEn = (names: string[], more: number) =>
  more > 0 ? `${names.join(", ")} and ${more} more` : names.join(", ");

const listDe = (names: string[], more: number) =>
  more > 0 ? `${names.join(", ")} und ${more} weitere` : names.join(", ");

const en: DependencyTexts = {
  tabList: "Mods",
  tabDependencies: "Dependencies",
  section: "Dependencies",
  explain:
    "Every active plugin in the order the game loads it, from left to right: the game first, then the Creation Club content the game brings along, then your mods. Each arc joins a plugin to one it needs. A plugin that needs another must load after it, and the game will not start if a needed plugin is missing. Arcs above lead to what the plugin needs, dashed arcs below come from what needs it.",
  needsSummary: (names, more) => `Needs: ${listEn(names, more)}.`,
  needsNothing: "Needs no other plugin.",
  neededBySummary: (plugins, mods) =>
    `Needed by ${plugins === 1 ? "1 plugin" : `${plugins} plugins`} of ${mods === 1 ? "1 mod" : `${mods} mods`}.`,
  neededByNothing: "No other plugin needs it.",
  needs: "Needs",
  neededBy: "Needed by",
  needsWhere: "left, solid arcs above",
  neededByWhere: "right, dashed arcs below",
  notLoaded: "not loaded",
  notLoadedNote: "The game does not start while a plugin needs one that is not loaded.",
  ownPlugins: (names) => `Its plugins: ${names}`,
  showAll: (count) => `Show all ${count}`,
  showFewer: "Show fewer",
  search: "Find a plugin or mod",
  stats: (plugins, links) =>
    `${plugins.toLocaleString("en")} plugins, ${links.toLocaleString("en")} links`,
  pickHint: "Pick a plugin in the diagram or find one by name.",
  mostNeeded: "Needed the most",
  dependentsCount: (count) => (count === 1 ? "1 plugin needs it" : `${count} plugins need it`),
  place: (place, total) => `Place ${place} of ${total} in the load order`,
  fromMod: (name) => `from ${name}`,
  origins: { game: "Game", creationClub: "Creation Club", mod: "Mods" },
  clear: "Clear",
  diagramLabel: (plugins, links) =>
    `Diagram of ${plugins} plugins in load order and the ${links} plugins they need`,
  noLoadOrder: "The last scan did not record the load order. Scan again to see it.",
  found: (count) => (count === 1 ? "1 plugin found" : `${count} plugins found`),
  wouldLose: (count) =>
    count === 1
      ? "This plugin would lose a plugin it needs:"
      : `These ${count} plugins would lose a plugin they need:`,
  inDiagram: (plugin) => `Show ${plugin} in the dependency diagram`,
  modPage: (name) => `Open the page of ${name}`,
};

const de: DependencyTexts = {
  tabList: "Mods",
  tabDependencies: "Abhängigkeiten",
  section: "Abhängigkeiten",
  explain:
    "Jedes aktive Plugin in der Reihenfolge, in der das Spiel es lädt, von links nach rechts: zuerst das Spiel, dann die Creation-Club-Inhalte, die das Spiel mitbringt, dann deine Mods. Jeder Bogen verbindet ein Plugin mit einem, das es braucht. Ein Plugin, das ein anderes braucht, muss nach ihm laden, und das Spiel startet nicht, wenn ein gebrauchtes Plugin fehlt. Bögen oben führen zu dem, was das Plugin braucht, gestrichelte Bögen unten kommen von den Plugins, die es brauchen.",
  needsSummary: (names, more) => `Braucht: ${listDe(names, more)}.`,
  needsNothing: "Braucht kein anderes Plugin.",
  neededBySummary: (plugins, mods) =>
    `Gebraucht von ${plugins === 1 ? "1 Plugin" : `${plugins} Plugins`} aus ${mods === 1 ? "1 Mod" : `${mods} Mods`}.`,
  neededByNothing: "Kein anderes Plugin braucht es.",
  needs: "Braucht",
  neededBy: "Gebraucht von",
  needsWhere: "links, durchgezogene Bögen oben",
  neededByWhere: "rechts, gestrichelte Bögen unten",
  notLoaded: "nicht geladen",
  notLoadedNote:
    "Das Spiel startet nicht, solange ein Plugin eines braucht, das nicht geladen ist.",
  ownPlugins: (names) => `Seine Plugins: ${names}`,
  showAll: (count) => `Alle ${count} zeigen`,
  showFewer: "Weniger zeigen",
  search: "Plugin oder Mod finden",
  stats: (plugins, links) =>
    `${plugins.toLocaleString("de")} Plugins, ${links.toLocaleString("de")} Verbindungen`,
  pickHint: "Wähle ein Plugin im Diagramm, oder finde eines über seinen Namen.",
  mostNeeded: "Am meisten gebraucht",
  dependentsCount: (count) =>
    count === 1 ? "1 Plugin braucht es" : `${count} Plugins brauchen es`,
  place: (place, total) => `Platz ${place} von ${total} in der Ladereihenfolge`,
  fromMod: (name) => `aus ${name}`,
  origins: { game: "Spiel", creationClub: "Creation Club", mod: "Mods" },
  clear: "Aufheben",
  diagramLabel: (plugins, links) =>
    `Diagramm von ${plugins} Plugins in Ladereihenfolge und den ${links} Plugins, die sie brauchen`,
  noLoadOrder:
    "Der letzte Scan hat die Ladereihenfolge nicht festgehalten. Scanne erneut, um sie zu sehen.",
  found: (count) => (count === 1 ? "1 Plugin gefunden" : `${count} Plugins gefunden`),
  wouldLose: (count) =>
    count === 1
      ? "Diesem Plugin würde ein Plugin fehlen, das es braucht:"
      : `Diesen ${count} Plugins würde ein Plugin fehlen, das sie brauchen:`,
  inDiagram: (plugin) => `${plugin} im Abhängigkeitsdiagramm zeigen`,
  modPage: (name) => `Seite von ${name} öffnen`,
};

const dependencyTexts: Record<Lang, DependencyTexts> = { en, de };

/** The texts of the plugin dependencies in the window's language. */
export function useDependencyTexts(): DependencyTexts {
  return dependencyTexts[useSession().lang];
}
