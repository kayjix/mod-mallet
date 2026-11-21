// The texts of the comparison of files that are neither textures nor
// meshes (ADR 0066): what a kind of file is for, what choosing a side
// means for it, and what the two versions have in common. Kept apart from
// `i18n.ts`, like the other pages' texts.

import { useSession } from "./Actions";
import type { Lang } from "./i18n";

/** The kinds of files told apart, by what they do in the game. */
export type FileKind =
  | "script"
  | "source"
  | "menu"
  | "menuText"
  | "questStart"
  | "animation"
  | "settings"
  | "extenderSettings"
  | "extender"
  | "plugin"
  | "sound"
  | "voice"
  | "lip"
  | "faceShapes"
  | "material"
  | "picture"
  | "text"
  | "other";

interface KindTexts {
  /** A name of one or two words. */
  name: string;
  /** Up to four letters for the small square in the list. */
  short: string;
  /** What files of the kind are for, for someone who never opened a modding tool. */
  purpose: string;
  /** What it likely means when the other mod's version is not used. */
  consequence: string;
}

export interface CompareFilesTexts {
  kinds: Record<FileKind, KindTexts>;
  /** What choosing a side means for any file. */
  choiceMeaning: string;
  loading: string;
  cannotRead: string;
  /** The file is the same in both mods. */
  same: string;
  /** The file differs between the two mods. */
  differs: string;
  /** Short marks for the list of files. */
  sameShort: string;
  differsShort: string;
  /** Where a side's version comes from. */
  inArchive: (archive: string) => string;
  loose: string;
  keptBack: string;
  original: string;
  /** A side that has no version of its own and the game has none either. */
  noVersion: string;
  size: (bytes: string) => string;
  dated: (date: string) => string;
  /** The text differences: lines left out between changes. */
  sameLines: (count: number) => string;
  textCut: string;
  noTextDifference: string;
  /** A compiled script's functions. */
  scriptName: (name: string) => string;
  functionsDiffer: string;
  functionsOnlyIn: (name: string) => string;
  functionsSame: (count: number) => string;
  functionsAllSame: string;
  noDebugInfo: string;
  getter: string;
  setter: string;
  inState: (state: string) => string;
  /** The whole duel. */
  allSame: (count: number) => string;
  nothingToDecide: string;
  markedNothing: string;
  someDiffer: (differ: number, total: number) => string;
  /** More files than listed: the report lists only some. */
  moreFiles: (count: number) => string;
  /** The report is older and names no files of this duel. */
  rescanForFiles: string;
}

const en: CompareFilesTexts = {
  kinds: {
    script: {
      name: "Script",
      short: "PEX",
      purpose: "Game logic: what happens in quests, on objects, in fights and in menus.",
      consequence:
        "If the losing mod needs its own version of this script, its features may not work.",
    },
    source: {
      name: "Script source",
      short: "PSC",
      purpose:
        "The readable form of a script. The game does not use it; only tools for making mods read it.",
      consequence: "Makes no difference in the game.",
    },
    menu: {
      name: "Menu",
      short: "SWF",
      purpose: "How a menu or a part of the screen looks and works.",
      consequence:
        "The losing mod's changes to this menu do not show. If it relies on them, parts of its menu may be missing or not work.",
    },
    menuText: {
      name: "Menu texts",
      short: "TXT",
      purpose: "The words a menu shows, in one language.",
      consequence: "Some menu texts may show as codes such as $Name instead of words.",
    },
    questStart: {
      name: "Quest start data",
      short: "SEQ",
      purpose: "Which quests start right away when the game loads.",
      consequence: "Quests that only the losing mod lists may not start, or start late.",
    },
    animation: {
      name: "Animation or behaviour",
      short: "HKX",
      purpose: "How characters and creatures move, and which move follows which.",
      consequence:
        "The losing mod's moves may not play. Behaviour files that do not fit together can make characters stand stiff with their arms out.",
    },
    settings: {
      name: "Settings",
      short: "CFG",
      purpose: "Settings of a mod: numbers and switches it reads when the game starts.",
      consequence: "The losing mod's settings are not used; its own defaults may be lost.",
    },
    extenderSettings: {
      name: "SKSE plugin settings",
      short: "CFG",
      purpose:
        "How an SKSE plugin behaves. Such a plugin is a small program that extends what the game can do.",
      consequence: "The plugin runs with the winning mod's settings, not the losing one's.",
    },
    extender: {
      name: "SKSE plugin",
      short: "DLL",
      purpose:
        "A small program that extends what the game can do, started by the script extender (SKSE).",
      consequence:
        "Only one of the two programs runs. Features of the losing mod that need its version do not work.",
    },
    plugin: {
      name: "Plugin in an archive",
      short: "ESP",
      purpose:
        "A plugin packed in an archive. The game only loads plugins that lie in the Data folder itself.",
      consequence: "Usually makes no difference in the game.",
    },
    sound: {
      name: "Sound",
      short: "SND",
      purpose: "A sound or music the game plays.",
      consequence: "You hear the winning mod's sound.",
    },
    voice: {
      name: "Voice line",
      short: "VOX",
      purpose: "A spoken line, together with how the lips move to it.",
      consequence: "You hear the winning mod's line; it may not match the words shown.",
    },
    lip: {
      name: "Lip movement",
      short: "LIP",
      purpose: "How a face moves its lips while speaking one line.",
      consequence: "Lips may not match the spoken words.",
    },
    faceShapes: {
      name: "Face shapes",
      short: "TRI",
      purpose: "How a head can change shape: expressions, and the sliders when making a character.",
      consequence: "Faces may look wrong, or expressions may not work.",
    },
    material: {
      name: "Material",
      short: "MAT",
      purpose: "How a surface looks: which textures it uses and how it shines.",
      consequence: "Surfaces look the way the winning mod made them.",
    },
    picture: {
      name: "Picture",
      short: "IMG",
      purpose: "A picture, for example in a book or a menu.",
      consequence: "You see the winning mod's picture.",
    },
    text: {
      name: "Text",
      short: "TXT",
      purpose: "A text file: notes, a list or a description.",
      consequence: "Usually makes no difference in the game.",
    },
    other: {
      name: "File",
      short: "FILE",
      purpose: "A file the game or a mod reads.",
      consequence: "The losing mod's version is not used; what that changes depends on the mod.",
    },
  },
  choiceMeaning: "Whichever mod wins provides this file; the other mod's version is not used.",
  loading: "Reading both versions …",
  cannotRead: "The versions of this file could not be read.",
  same: "Both versions are the same, byte for byte. It makes no difference which one wins.",
  differs: "The two versions differ.",
  sameShort: "same",
  differsShort: "differs",
  inArchive: (archive) => `in ${archive}`,
  loose: "loose file",
  keptBack: "kept back by the mod manager",
  original: "the game's original",
  noVersion: "no version of its own",
  size: (bytes) => `${bytes}`,
  dated: (date) => `dated ${date}`,
  sameLines: (count) => (count === 1 ? "1 line the same" : `${count} lines the same`),
  textCut: "The text is long; only its beginning is compared.",
  noTextDifference: "The text is the same; only invisible characters such as line ends differ.",
  scriptName: (name) => `Script ${name}`,
  functionsDiffer: "Functions that work differently",
  functionsOnlyIn: (name) => `Only in ${name}`,
  functionsSame: (count) =>
    count === 1
      ? "1 function works the same in both."
      : `${count} functions work the same in both.`,
  functionsAllSame:
    "All functions are built the same way; the versions differ only in details such as the time they were made.",
  noDebugInfo: "This script does not carry the names of its functions.",
  getter: "reads",
  setter: "sets",
  inState: (state) => `in state ${state}`,
  allSame: (count) =>
    count === 1
      ? "The file is the same in both mods, byte for byte. Which one wins makes no difference."
      : `All ${count} files are the same in both mods, byte for byte. Which one wins makes no difference.`,
  nothingToDecide: "Nothing to decide",
  markedNothing: "Marked as nothing to decide: the files are the same.",
  someDiffer: (differ, total) =>
    differ === 1
      ? `1 of ${total} files differs between the two mods.`
      : `${differ} of ${total} files differ between the two mods.`,
  moreFiles: (count) => (count === 1 ? "and 1 more file" : `and ${count} more files`),
  rescanForFiles:
    "This scan is older and names only a few of the files. Scan again to see all of them.",
};

const de: CompareFilesTexts = {
  kinds: {
    script: {
      name: "Skript",
      short: "PEX",
      purpose: "Spiellogik: was in Quests, an Gegenständen, in Kämpfen und in Menüs geschieht.",
      consequence:
        "Braucht die unterlegene Mod ihre eigene Fassung dieses Skripts, funktionieren manche ihrer Funktionen womöglich nicht.",
    },
    source: {
      name: "Skript-Quelltext",
      short: "PSC",
      purpose:
        "Die lesbare Form eines Skripts. Das Spiel nutzt sie nicht; nur Werkzeuge zum Bau von Mods lesen sie.",
      consequence: "Macht im Spiel keinen Unterschied.",
    },
    menu: {
      name: "Menü",
      short: "SWF",
      purpose: "Wie ein Menü oder ein Teil der Anzeige aussieht und funktioniert.",
      consequence:
        "Die Änderungen der unterlegenen Mod an diesem Menü erscheinen nicht. Verlässt sie sich darauf, fehlen Teile ihres Menüs oder funktionieren nicht.",
    },
    menuText: {
      name: "Menütexte",
      short: "TXT",
      purpose: "Die Wörter, die ein Menü anzeigt, in einer Sprache.",
      consequence: "Manche Menütexte erscheinen womöglich als Codes wie $Name statt als Wörter.",
    },
    questStart: {
      name: "Quest-Startdaten",
      short: "SEQ",
      purpose: "Welche Quests gleich beim Laden des Spiels beginnen.",
      consequence:
        "Quests, die nur die unterlegene Mod aufführt, starten womöglich nicht oder erst spät.",
    },
    animation: {
      name: "Animation oder Verhalten",
      short: "HKX",
      purpose: "Wie sich Figuren und Kreaturen bewegen und welche Bewegung auf welche folgt.",
      consequence:
        "Die Bewegungen der unterlegenen Mod werden womöglich nicht abgespielt. Verhaltensdateien, die nicht zusammenpassen, lassen Figuren steif mit ausgestreckten Armen dastehen.",
    },
    settings: {
      name: "Einstellungen",
      short: "CFG",
      purpose: "Einstellungen einer Mod: Zahlen und Schalter, die sie beim Spielstart liest.",
      consequence:
        "Die Einstellungen der unterlegenen Mod werden nicht genutzt; ihre eigenen Vorgaben gehen womöglich verloren.",
    },
    extenderSettings: {
      name: "Einstellungen eines SKSE-Plugins",
      short: "CFG",
      purpose:
        "Wie sich ein SKSE-Plugin verhält. So ein Plugin ist ein kleines Programm, das erweitert, was das Spiel kann.",
      consequence:
        "Das Plugin läuft mit den Einstellungen der siegreichen Mod, nicht mit denen der anderen.",
    },
    extender: {
      name: "SKSE-Plugin",
      short: "DLL",
      purpose:
        "Ein kleines Programm, das erweitert, was das Spiel kann; der Script Extender (SKSE) startet es.",
      consequence:
        "Nur eines der beiden Programme läuft. Funktionen der unterlegenen Mod, die ihre Fassung brauchen, gehen nicht.",
    },
    plugin: {
      name: "Plugin in einem Archiv",
      short: "ESP",
      purpose:
        "Ein Plugin, in ein Archiv gepackt. Das Spiel lädt nur Plugins, die direkt im Data-Ordner liegen.",
      consequence: "Macht im Spiel meist keinen Unterschied.",
    },
    sound: {
      name: "Klang",
      short: "SND",
      purpose: "Ein Geräusch oder Musik, die das Spiel abspielt.",
      consequence: "Zu hören ist der Klang der siegreichen Mod.",
    },
    voice: {
      name: "Sprachzeile",
      short: "VOX",
      purpose: "Ein gesprochener Satz, zusammen mit der Lippenbewegung dazu.",
      consequence:
        "Zu hören ist der Satz der siegreichen Mod; er passt womöglich nicht zum angezeigten Text.",
    },
    lip: {
      name: "Lippenbewegung",
      short: "LIP",
      purpose: "Wie ein Gesicht beim Sprechen eines Satzes die Lippen bewegt.",
      consequence: "Die Lippen passen womöglich nicht zu den gesprochenen Worten.",
    },
    faceShapes: {
      name: "Gesichtsformen",
      short: "TRI",
      purpose:
        "Wie ein Kopf seine Form ändern kann: Mimik und die Regler beim Erstellen einer Figur.",
      consequence: "Gesichter sehen womöglich falsch aus, oder die Mimik funktioniert nicht.",
    },
    material: {
      name: "Material",
      short: "MAT",
      purpose: "Wie eine Oberfläche aussieht: welche Texturen sie nutzt und wie sie glänzt.",
      consequence: "Oberflächen sehen so aus, wie die siegreiche Mod sie gemacht hat.",
    },
    picture: {
      name: "Bild",
      short: "IMG",
      purpose: "Ein Bild, zum Beispiel in einem Buch oder einem Menü.",
      consequence: "Zu sehen ist das Bild der siegreichen Mod.",
    },
    text: {
      name: "Text",
      short: "TXT",
      purpose: "Eine Textdatei: Notizen, eine Liste oder eine Beschreibung.",
      consequence: "Macht im Spiel meist keinen Unterschied.",
    },
    other: {
      name: "Datei",
      short: "DATEI",
      purpose: "Eine Datei, die das Spiel oder eine Mod liest.",
      consequence:
        "Die Fassung der unterlegenen Mod wird nicht genutzt; was das ändert, hängt von der Mod ab.",
    },
  },
  choiceMeaning:
    "Die siegreiche Mod liefert diese Datei; die Fassung der anderen Mod wird nicht genutzt.",
  loading: "Beide Fassungen werden gelesen …",
  cannotRead: "Die Fassungen dieser Datei ließen sich nicht lesen.",
  same: "Beide Fassungen sind Byte für Byte gleich. Welche gewinnt, macht keinen Unterschied.",
  differs: "Die beiden Fassungen unterscheiden sich.",
  sameShort: "gleich",
  differsShort: "anders",
  inArchive: (archive) => `in ${archive}`,
  loose: "lose Datei",
  keptBack: "vom Mod-Manager zurückgehalten",
  original: "Original des Spiels",
  noVersion: "keine eigene Fassung",
  size: (bytes) => `${bytes}`,
  dated: (date) => `vom ${date}`,
  sameLines: (count) => (count === 1 ? "1 Zeile gleich" : `${count} Zeilen gleich`),
  textCut: "Der Text ist lang; nur sein Anfang wird verglichen.",
  noTextDifference:
    "Der Text ist gleich; nur unsichtbare Zeichen wie Zeilenenden unterscheiden sich.",
  scriptName: (name) => `Skript ${name}`,
  functionsDiffer: "Funktionen, die anders arbeiten",
  functionsOnlyIn: (name) => `Nur in ${name}`,
  functionsSame: (count) =>
    count === 1
      ? "1 Funktion arbeitet in beiden gleich."
      : `${count} Funktionen arbeiten in beiden gleich.`,
  functionsAllSame:
    "Alle Funktionen sind gleich aufgebaut; die Fassungen unterscheiden sich nur in Einzelheiten wie dem Zeitpunkt, zu dem sie erstellt wurden.",
  noDebugInfo: "Dieses Skript enthält die Namen seiner Funktionen nicht.",
  getter: "liest",
  setter: "setzt",
  inState: (state) => `im Zustand ${state}`,
  allSame: (count) =>
    count === 1
      ? "Die Datei ist in beiden Mods Byte für Byte gleich. Welche gewinnt, macht keinen Unterschied."
      : `Alle ${count} Dateien sind in beiden Mods Byte für Byte gleich. Welche gewinnt, macht keinen Unterschied.`,
  nothingToDecide: "Nichts zu entscheiden",
  markedNothing: "Als „nichts zu entscheiden“ markiert: die Dateien sind gleich.",
  someDiffer: (differ, total) =>
    differ === 1
      ? `1 von ${total} Dateien unterscheidet sich zwischen den beiden Mods.`
      : `${differ} von ${total} Dateien unterscheiden sich zwischen den beiden Mods.`,
  moreFiles: (count) => (count === 1 ? "und 1 weitere Datei" : `und ${count} weitere Dateien`),
  rescanForFiles:
    "Dieser Scan ist älter und nennt nur einige der Dateien. Neu scannen, um alle zu sehen.",
};

export const compareFilesTexts: Record<Lang, CompareFilesTexts> = { en, de };

/** The comparison's texts in the window's language. */
export function useCompareFilesTexts(): CompareFilesTexts {
  return compareFilesTexts[useSession().lang];
}
