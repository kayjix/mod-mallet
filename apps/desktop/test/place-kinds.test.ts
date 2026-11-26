// Tests of which places the dialog picks as files and which as folders.

import assert from "node:assert/strict";
import { test } from "node:test";
import { dialogStart, placeKind } from "../src/place-kinds.ts";

test("lists and logs are files, the game's and the manager's folders are folders", () => {
  for (const role of [
    "plugin-list",
    "extender-log",
    "manager-state",
    "unpacker",
    "deployment-manifest",
    "masterlist",
  ] as const) {
    assert.equal(placeKind(role), "file", role);
  }
  for (const role of [
    "game-dir",
    "downloads",
    "crash-logs",
    "mod-staging",
    "manager-instance",
    "manager-mods",
  ] as const) {
    assert.equal(placeKind(role), "folder", role);
  }
});

test("the dialog opens at a full path and nowhere in particular otherwise", () => {
  assert.equal(dialogStart("  C:\\Games\\Skyrim  "), "C:\\Games\\Skyrim");
  assert.equal(dialogStart("D:/Mods/plugins.txt"), "D:/Mods/plugins.txt");
  assert.equal(dialogStart("\\\\nas\\games"), "\\\\nas\\games");
  // The unpacker before it is found is named, not placed.
  assert.equal(dialogStart("7-Zip"), null);
  assert.equal(dialogStart(""), null);
});
