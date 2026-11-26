// The texts of sharing the report (ADR 0091), kept apart from i18n.ts like
// the other pages' texts (ADR 0077).

import { useSession } from "./Actions";
import type { Lang } from "./i18n";

export interface ShareTexts {
  title: string;
  /** What sharing gives, before anything is copied. */
  note: string;
  share: string;
  /** Said after copying, before the sentence of what was removed. */
  copied: string;
  /** Said when there is no clipboard; the text can still be saved. */
  notCopied: string;
  saveMarkdown: string;
  saveJson: string;
  saved: (path: string) => string;
  failed: (error: string) => string;
}

export const shareTexts: Record<Lang, ShareTexts> = {
  en: {
    title: "Share the report",
    note: "For a forum post or for someone who helps you: the problems, the places read and the load order, without your user name and the folders of your computer.",
    share: "Share report",
    copied: "The report is on the clipboard; paste it where you ask for help.",
    notCopied: "The report could not be put on the clipboard; save it as a file instead.",
    saveMarkdown: "Save as text (.md)",
    saveJson: "Save as data (.json)",
    saved: (path) => `Saved as ${path}.`,
    failed: (error) => `The report could not be shared: ${error}`,
  },
  de: {
    title: "Bericht weitergeben",
    note: "Für einen Forenbeitrag oder für jemanden, der dir hilft: die Probleme, die gelesenen Orte und die Ladereihenfolge, ohne deinen Benutzernamen und die Ordner deines Computers.",
    share: "Bericht weitergeben",
    copied: "Der Bericht ist in der Zwischenablage; füge ihn dort ein, wo du um Hilfe bittest.",
    notCopied:
      "Der Bericht konnte nicht in die Zwischenablage gelegt werden; speichere ihn stattdessen als Datei.",
    saveMarkdown: "Als Text speichern (.md)",
    saveJson: "Als Daten speichern (.json)",
    saved: (path) => `Gespeichert als ${path}.`,
    failed: (error) => `Der Bericht konnte nicht weitergegeben werden: ${error}`,
  },
};

export function useShareTexts(): ShareTexts {
  return shareTexts[useSession().lang];
}
