// Whether a place a scan reads is a file or a folder, for the dialog that
// picks it (ADR 0018). Pure, so that the tests cover every place.

import type { ScanLocation } from "./generated/rendered-places";

/** What the dialog for a place picks. */
export type PlaceKind = "file" | "folder";

/** Every place, by what it is on disk. A new role does not compile until it is named here. */
const KINDS: Record<ScanLocation["role"], PlaceKind> = {
  "game-dir": "folder",
  "plugin-list": "file",
  "extender-log": "file",
  "manager-state": "file",
  downloads: "folder",
  unpacker: "file",
  "deployment-manifest": "file",
  masterlist: "file",
  "crash-logs": "folder",
  "mod-staging": "folder",
  "manager-instance": "folder",
  "manager-mods": "folder",
};

/** Whether the place with this role is a file or a folder. */
export function placeKind(role: ScanLocation["role"]): PlaceKind {
  return KINDS[role];
}

/**
 * Where the dialog opens for a place whose field holds `path`: at the path
 * itself, so that a file is already picked and a folder already open, or
 * where the system likes if the field is empty or holds no full path, such
 * as "7-Zip" before the program is found.
 */
export function dialogStart(path: string): string | null {
  const trimmed = path.trim();
  const full =
    /^[A-Za-z]:[\\/]/.test(trimmed) || trimmed.startsWith("\\\\") || trimmed.startsWith("/");
  return full ? trimmed : null;
}
