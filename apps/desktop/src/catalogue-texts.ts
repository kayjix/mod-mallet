// The texts of the catalogue of things (ADR 0054): characters, weapons,
// armour and other things of the game and the mods, and the sets they form.
// Kept apart from `i18n.ts`, like the other pages' texts.

import { useSession } from "./Actions";
import type { CatalogueItem } from "./generated/catalogue";
import type { Lang } from "./i18n";

export type Category = CatalogueItem["category"];

/** The broad groups of things, as the page's tabs. */
export type Group = "characters" | "weapons" | "apparel" | "items" | "other";

export interface CatalogueTexts {
  nav: string;
  /** The sidebar entries of the two kinds of collisions. */
  navMapCollisions: string;
  navAssetCollisions: string;
  loading: string;
  unreadable: string;
  groupsLabel: string;
  groups: Record<Group, string>;
  categories: Record<Category, string>;
  search: string;
  arrangeLabel: string;
  byCategory: string;
  byMaterial: string;
  byRace: string;
  byOutfit: string;
  originLabel: string;
  originAll: string;
  originGame: string;
  originCreationClub: string;
  originMods: string;
  originChanged: string;
  /** One mod, in the list of where things come from. */
  originMod: (name: string) => string;
  noMaterial: string;
  noRace: string;
  nothingFound: string;
  showMore: (count: number) => string;
  pickSomething: string;
  /** How many characters wear an outfit. */
  worn: (count: number) => string;
  /** What a set consists of. */
  setPieces: (worn: number, weapons: number) => string;
  /** A pane on the stage: the thing as it lies on the ground, or as worn. */
  onGround: string;
  asWorn: string;
  face: string;
  detailCategory: string;
  detailMaterial: string;
  detailRace: string;
  detailFrom: string;
  detailChangedBy: string;
  detailVariants: (count: number) => string;
  detailModels: string;
  detailOutfit: string;
  detailInOutfits: string;
  detailWorn: string;
  detailWeapons: string;
  detailWeaponsHint: string;
  detailInternal: string;
  modelLabel: string;
  versionsTitle: string;
  versionsLoading: string;
  versionsOnlyOne: string;
  versionsHint: (count: number) => string;
  /** Which of a thing's files a version brings its own copy of. */
  versionBrings: (meshes: number, textures: number) => string;
  versionInGame: string;
  versionPartlyInGame: (shown: number, of: number) => string;
  looseFiles: string;
  versionChosen: string;
  take: string;
  takeHint: string;
  reset: string;
  resetHint: string;
  originChosen: string;
  game: string;
  creationClub: string;
  /** A material by its name in the game's keywords, in the user's language where known. */
  material: (name: string) => string;
}

/** Splits `SteelPlate` into `Steel Plate`. */
function words(name: string): string {
  return name.replace(/([a-z])([A-Z])/g, "$1 $2").replace(/([A-Z])([A-Z][a-z])/g, "$1 $2");
}

const MATERIALS_DE: Record<string, string> = {
  Amber: "Bernstein",
  Blades: "Klingen",
  Bonemold: "Knochenform",
  Chainmail: "Kettenhemd",
  Chitin: "Chitin",
  Daedric: "Daedrisch",
  Dark: "Dunkel",
  Dawnguard: "Dämmerwacht",
  Dragonbone: "Drachenknochen",
  Dragonplate: "Drachenplatten",
  Dragonscale: "Drachenschuppen",
  Draugr: "Draugr",
  DraugrHoned: "Draugr (geschärft)",
  Dwarven: "Dwemer",
  Ebony: "Ebenerz",
  Elven: "Elfisch",
  ElvenGilded: "Elfisch (vergoldet)",
  Falmer: "Falmer",
  FalmerHardened: "Falmer (gehärtet)",
  FalmerHoned: "Falmer (geschärft)",
  Forsworn: "Abgeschworene",
  Glass: "Glas",
  Golden: "Golden",
  Hide: "Fell",
  Imperial: "Kaiserlich",
  ImperialStudded: "Kaiserlich (beschlagen)",
  Iron: "Eisen",
  IronBanded: "Eisen (beschlagen)",
  Leather: "Leder",
  Madness: "Wahnsinn",
  Nordic: "Nordisch",
  Orcish: "Orkisch",
  Scaled: "Schuppen",
  Silver: "Silber",
  Stalhrim: "Stalhrim",
  Steel: "Stahl",
  SteelPlate: "Stahlplatten",
  Stormcloak: "Sturmmantel",
  Studded: "Beschlagen",
  ThievesGuild: "Diebesgilde",
  Wood: "Holz",
};

const en: CatalogueTexts = {
  nav: "Assets",
  navMapCollisions: "Map collisions",
  navAssetCollisions: "Asset collisions",
  loading: "Reading every plugin. The first time this takes a few seconds.",
  unreadable: "The game's plugins could not be read. Check the places under Settings.",
  groupsLabel: "What to list",
  groups: {
    characters: "Characters",
    weapons: "Weapons",
    apparel: "Armour & clothes",
    items: "Items",
    other: "Other",
  },
  categories: {
    sword: "Swords",
    dagger: "Daggers",
    "war-axe": "War axes",
    mace: "Maces",
    greatsword: "Greatswords",
    battleaxe: "Battleaxes",
    warhammer: "Warhammers",
    bow: "Bows",
    crossbow: "Crossbows",
    staff: "Staves",
    "other-weapon": "Other weapons",
    ammunition: "Arrows & bolts",
    head: "Head",
    body: "Body",
    hands: "Hands",
    feet: "Feet",
    shield: "Shields",
    jewellery: "Rings & necklaces",
    "other-worn": "Other things worn",
    character: "Characters",
    book: "Books & scrolls",
    potion: "Potions & food",
    ingredient: "Ingredients",
    key: "Keys",
    "soul-gem": "Soul gems",
    misc: "Other things to carry",
    furniture: "Furniture",
    container: "Containers",
    door: "Doors",
    activator: "Things to use",
    flora: "Plants",
    light: "Lights",
  },
  search: "Search by name…",
  arrangeLabel: "Arrange",
  byCategory: "By kind",
  byMaterial: "By material",
  byRace: "By race",
  byOutfit: "As sets",
  originLabel: "From",
  originAll: "Everything",
  originGame: "Only the game, unchanged",
  originCreationClub: "Creation Club",
  originMods: "Added by mods",
  originChanged: "Changed by mods",
  originMod: (name) => name,
  noMaterial: "No material",
  noRace: "No race",
  nothingFound: "Nothing matches.",
  showMore: (count) => `Show ${count} more`,
  pickSomething: "Pick something on the left to look at it.",
  worn: (count) => (count === 1 ? "worn by 1 character" : `worn by ${count} characters`),
  setPieces: (worn, weapons) =>
    weapons === 0
      ? `${worn} pieces`
      : `${worn} pieces · ${weapons} ${weapons === 1 ? "weapon" : "weapons"}`,
  onGround: "On the ground",
  asWorn: "Worn",
  face: "Face",
  detailCategory: "Kind",
  detailMaterial: "Material",
  detailRace: "Race",
  detailFrom: "Comes from",
  detailChangedBy: "Changed by",
  detailVariants: (count) =>
    count === 1
      ? "1 variant looks the same, such as an enchanted one"
      : `${count} variants look the same, such as enchanted ones`,
  detailModels: "Files",
  detailOutfit: "Wears",
  detailInOutfits: "Part of these sets",
  detailWorn: "Worn",
  detailWeapons: "Carried",
  detailWeaponsHint: "What the characters wearing it carry, the most common first.",
  detailInternal: "Internal name",
  modelLabel: "Which model",
  versionsTitle: "Versions",
  versionsLoading: "Looking for every version in the game and the mods …",
  versionsOnlyOne: "The setup has only this one version.",
  versionsHint: (count) => `Up to ${count} stand side by side; tick which.`,
  versionBrings: (meshes, textures) =>
    [
      meshes === 0 ? "" : meshes === 1 ? "model" : `${meshes} models`,
      textures === 0 ? "" : textures === 1 ? "1 texture" : `${textures} textures`,
    ]
      .filter((part) => part !== "")
      .join(" and "),
  versionInGame: "in the game",
  versionPartlyInGame: (shown, of) => `${shown} of ${of} in the game`,
  looseFiles: "Loose files without a mod",
  versionChosen: "your choice",
  take: "Put into the game",
  takeHint:
    "Vortex leaves these files out of the other mods that bring them and deploys again. Only this thing's files change; you can take it back here or among the file conflicts in Vortex.",
  reset: "Take the choice back",
  resetHint:
    "Vortex deploys these files from every mod again; the order of the mods decides once more.",
  originChosen: "Chosen by you",
  game: "The game",
  creationClub: "Creation Club",
  material: (name) => words(name),
};

const de: CatalogueTexts = {
  nav: "Assets",
  navMapCollisions: "Karten-Kollisionen",
  navAssetCollisions: "Asset-Kollisionen",
  loading: "Alle Plugins werden gelesen. Beim ersten Mal dauert das einige Sekunden.",
  unreadable: "Die Plugins des Spiels ließen sich nicht lesen. Prüfe die Orte unter Einstellungen.",
  groupsLabel: "Was aufgelistet wird",
  groups: {
    characters: "Figuren",
    weapons: "Waffen",
    apparel: "Rüstung & Kleidung",
    items: "Gegenstände",
    other: "Sonstiges",
  },
  categories: {
    sword: "Schwerter",
    dagger: "Dolche",
    "war-axe": "Kriegsäxte",
    mace: "Streitkolben",
    greatsword: "Zweihänder",
    battleaxe: "Streitäxte",
    warhammer: "Kriegshämmer",
    bow: "Bögen",
    crossbow: "Armbrüste",
    staff: "Stäbe",
    "other-weapon": "Andere Waffen",
    ammunition: "Pfeile & Bolzen",
    head: "Kopf",
    body: "Körper",
    hands: "Hände",
    feet: "Füße",
    shield: "Schilde",
    jewellery: "Ringe & Ketten",
    "other-worn": "Anderes zum Anziehen",
    character: "Figuren",
    book: "Bücher & Schriftrollen",
    potion: "Tränke & Essen",
    ingredient: "Zutaten",
    key: "Schlüssel",
    "soul-gem": "Seelensteine",
    misc: "Anderes zum Mitnehmen",
    furniture: "Möbel",
    container: "Behälter",
    door: "Türen",
    activator: "Dinge zum Benutzen",
    flora: "Pflanzen",
    light: "Lichter",
  },
  search: "Nach Namen suchen …",
  arrangeLabel: "Ordnen",
  byCategory: "Nach Art",
  byMaterial: "Nach Material",
  byRace: "Nach Volk",
  byOutfit: "Als Sets",
  originLabel: "Herkunft",
  originAll: "Alles",
  originGame: "Nur das Spiel, unverändert",
  originCreationClub: "Creation Club",
  originMods: "Von Mods hinzugefügt",
  originChanged: "Von Mods verändert",
  originMod: (name) => name,
  noMaterial: "Ohne Material",
  noRace: "Ohne Volk",
  nothingFound: "Nichts gefunden.",
  showMore: (count) => `${count} weitere zeigen`,
  pickSomething: "Wähle links etwas aus, um es anzusehen.",
  worn: (count) => (count === 1 ? "von 1 Figur getragen" : `von ${count} Figuren getragen`),
  setPieces: (worn, weapons) =>
    weapons === 0
      ? `${worn} Teile`
      : `${worn} Teile · ${weapons} ${weapons === 1 ? "Waffe" : "Waffen"}`,
  onGround: "Am Boden",
  asWorn: "Getragen",
  face: "Gesicht",
  detailCategory: "Art",
  detailMaterial: "Material",
  detailRace: "Volk",
  detailFrom: "Kommt aus",
  detailChangedBy: "Verändert von",
  detailVariants: (count) =>
    count === 1
      ? "1 Variante sieht gleich aus, etwa eine verzauberte"
      : `${count} Varianten sehen gleich aus, etwa verzauberte`,
  detailModels: "Dateien",
  detailOutfit: "Trägt",
  detailInOutfits: "Teil dieser Sets",
  detailWorn: "Getragen",
  detailWeapons: "Dabei",
  detailWeaponsHint: "Was die Figuren mit diesem Set bei sich tragen, das Häufigste zuerst.",
  detailInternal: "Interner Name",
  modelLabel: "Welches Modell",
  versionsTitle: "Fassungen",
  versionsLoading: "Alle Fassungen in Spiel und Mods werden gesucht …",
  versionsOnlyOne: "Das Setup hat nur diese eine Fassung.",
  versionsHint: (count) => `Bis zu ${count} stehen nebeneinander; hake an, welche.`,
  versionBrings: (meshes, textures) =>
    [
      meshes === 0 ? "" : meshes === 1 ? "Modell" : `${meshes} Modelle`,
      textures === 0 ? "" : textures === 1 ? "1 Textur" : `${textures} Texturen`,
    ]
      .filter((part) => part !== "")
      .join(" und "),
  versionInGame: "im Spiel",
  versionPartlyInGame: (shown, of) => `${shown} von ${of} im Spiel`,
  looseFiles: "Lose Dateien ohne Mod",
  versionChosen: "deine Wahl",
  take: "Ins Spiel nehmen",
  takeHint:
    "Vortex lässt diese Dateien bei den anderen Mods weg, die sie mitbringen, und stellt neu bereit. Nur die Dateien dieses Gegenstands wechseln; zurücknehmen kannst du es hier oder bei den Dateikonflikten in Vortex.",
  reset: "Wahl zurücknehmen",
  resetHint:
    "Vortex stellt diese Dateien wieder von allen Mods bereit; dann entscheidet wieder die Reihenfolge der Mods.",
  originChosen: "Von dir gewählt",
  game: "Das Spiel",
  creationClub: "Creation Club",
  material: (name) => MATERIALS_DE[name] ?? words(name),
};

export const catalogueTexts: Record<Lang, CatalogueTexts> = { en, de };

/** The catalogue's texts in the window's language. */
export function useCatalogueTexts(): CatalogueTexts {
  return catalogueTexts[useSession().lang];
}

/** The broad group of each category. */
export const GROUP_OF: Record<Category, Group> = {
  sword: "weapons",
  dagger: "weapons",
  "war-axe": "weapons",
  mace: "weapons",
  greatsword: "weapons",
  battleaxe: "weapons",
  warhammer: "weapons",
  bow: "weapons",
  crossbow: "weapons",
  staff: "weapons",
  "other-weapon": "weapons",
  ammunition: "weapons",
  head: "apparel",
  body: "apparel",
  hands: "apparel",
  feet: "apparel",
  shield: "apparel",
  jewellery: "apparel",
  "other-worn": "apparel",
  character: "characters",
  book: "items",
  potion: "items",
  ingredient: "items",
  key: "items",
  "soul-gem": "items",
  misc: "items",
  furniture: "other",
  container: "other",
  door: "other",
  activator: "other",
  flora: "other",
  light: "other",
};

/** Words for a set's internal name: `GuardWhiterunOutfit` → `Guard Whiterun`. */
export function outfitWords(editorId: string): string {
  return words(editorId.replace(/Outfit$/i, "").replace(/^(DLC\d|cc[A-Za-z0-9]+_)/, ""));
}
