// Tests of the tools: the ones Vortex lists, which Mod Mallet reads, and
// Mod Mallet's own entry among them. The state is a made-up object in the
// shape of Vortex's.

const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { test } = require("node:test");

const link = require("../index.js");

const EXE = "D:\\Made Up\\Mod Mallet\\mod-mallet.exe";

function toolsState(tools = {}, game = "skyrimse") {
  return {
    settings: {
      profiles: { activeProfileId: "p1" },
      downloads: { path: "D:\\Downloads" },
      gameMode: {
        discovered: {
          [game]: {
            path: "D:\\Made Up\\Game",
            tools: {
              sseedit: {
                id: "sseedit",
                name: "SSEEdit (Create or Patch)",
                path: "D:\\Made Up\\Tools\\SSEEdit.exe",
                custom: true,
                hidden: false,
                parameters: ["-IKnowWhatImDoing"],
                environment: { SECRET: "x" },
              },
              lost: { id: "lost", name: "Never Found" },
              ...tools,
            },
          },
        },
      },
    },
    persistent: {
      profiles: { p1: { gameId: game, modState: {} } },
      mods: { [game]: {} },
      downloads: { files: {} },
    },
  };
}

function fakeApi(current) {
  const dispatched = [];
  return {
    dispatched,
    getState: () => current,
    store: { dispatch: (action) => dispatched.push(action) },
  };
}

function appFolder(t, exe = EXE) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "mallet-app-"));
  t.after(() => fs.rmSync(dir, { recursive: true, force: true }));
  fs.writeFileSync(path.join(dir, "app.json"), JSON.stringify({ exe, version: "1.0.0" }));
  fs.writeFileSync(path.join(dir, "app-icon.png"), Buffer.from([1, 2, 3]));
  return dir;
}

test("the snapshot carries the tools of the game without their parameters", () => {
  const tools = link.snapshot(toolsState()).settings.gameMode.discovered.skyrimse.tools;
  assert.deepEqual(Object.keys(tools), ["sseedit"]);
  assert.deepEqual(tools.sseedit, {
    id: "sseedit",
    name: "SSEEdit (Create or Patch)",
    path: "D:\\Made Up\\Tools\\SSEEdit.exe",
    custom: true,
    hidden: false,
  });
});

test("Mod Mallet's note is trusted only for an absolute path to a program that is there", (t) => {
  const dir = appFolder(t);
  assert.equal(link.readAppNote(dir, () => true).exe, EXE);
  assert.equal(
    link.readAppNote(dir, () => false),
    undefined,
  );
  for (const exe of ["mod-mallet.exe", "D:\\Made Up\\script.bat", 7]) {
    assert.equal(
      link.readAppNote(appFolder(t, exe), () => true),
      undefined,
    );
  }
  assert.equal(
    link.readAppNote(path.join(dir, "nowhere"), () => true),
    undefined,
  );
});

test("the tool is planned for a game Vortex found, once, and follows Mod Mallet when it moves", () => {
  const note = { exe: EXE, version: "1.0.0" };
  const planned = link.planDashboardTool(toolsState(), note);
  assert.equal(planned.game, "skyrimse");
  assert.equal(planned.toolId, "mod-mallet");
  assert.equal(planned.result.path, EXE);
  assert.equal(planned.result.workingDirectory, "D:\\Made Up\\Mod Mallet");
  assert.equal(planned.result.name, "Mod Mallet");

  const there = toolsState({ "mod-mallet": { id: "mod-mallet", path: EXE } });
  assert.equal(link.planDashboardTool(there, note), undefined);
  const moved = { ...note, exe: "D:\\Elsewhere\\mod-mallet.exe" };
  assert.equal(link.planDashboardTool(there, moved).result.path, moved.exe);

  // Removed by the user since it was added: it stays removed.
  assert.equal(link.planDashboardTool(toolsState(), note, { skyrimse: EXE }), undefined);
  // Not a game Mod Mallet knows, and a game Vortex has not found.
  assert.equal(link.planDashboardTool(toolsState({}, "baldursgate3"), note), undefined);
  const undiscovered = toolsState();
  undiscovered.settings.gameMode.discovered = {};
  assert.equal(link.planDashboardTool(undiscovered, note), undefined);
});

test("the tool is added through the state with its icon, and remembered", (t) => {
  const dir = appFolder(t);
  const userData = fs.mkdtempSync(path.join(os.tmpdir(), "mallet-userdata-"));
  t.after(() => fs.rmSync(userData, { recursive: true, force: true }));
  const api = fakeApi(toolsState());
  // The program is not on this machine; only the note's path is let through.
  const real = fs.existsSync;
  fs.existsSync = (file) => file === EXE || real(file);
  t.after(() => {
    fs.existsSync = real;
  });

  link.registerDashboardTool(api, dir, () => {}, userData);
  assert.equal(api.dispatched.length, 1);
  assert.equal(api.dispatched[0].type, "ADD_DISCOVERED_TOOL");
  assert.equal(api.dispatched[0].payload.gameId, "skyrimse");
  assert.equal(api.dispatched[0].payload.manual, true);
  assert.deepEqual(
    fs.readFileSync(link.toolIconPath(userData, "skyrimse")),
    Buffer.from([1, 2, 3]),
  );

  // The state still has no such tool: the user removed it in Vortex. It
  // is not added again.
  link.registerDashboardTool(api, dir, () => {}, userData);
  assert.equal(api.dispatched.length, 1);
});
