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

function request(action, game = "skyrimse") {
  return { protocol: 1, id: "abc-1", action, game, createdAt: NOW };
}

const NOW = 1_800_000_000_000;

/** The folder Mod Mallet shares with the extension, as `bridgeDir` finds it. */
function environment(t) {
  const local = fs.mkdtempSync(path.join(os.tmpdir(), "mallet-local-"));
  const userData = fs.mkdtempSync(path.join(os.tmpdir(), "mallet-userdata-"));
  t.after(() => {
    fs.rmSync(local, { recursive: true, force: true });
    fs.rmSync(userData, { recursive: true, force: true });
  });
  const env = { LOCALAPPDATA: local, APPDATA: userData };
  const dir = link.bridgeDir(env);
  fs.mkdirSync(dir, { recursive: true });
  fs.writeFileSync(path.join(dir, "app.json"), JSON.stringify({ exe: EXE, version: "1.0.0" }));
  fs.writeFileSync(path.join(dir, "app-icon.png"), Buffer.from([1, 2, 3]));
  // The program is not on this machine; only the note's path is let through.
  return { env, dir, userData, exists: (file) => file === EXE };
}

test("nothing is added to the tools without a request", (t) => {
  const { dir, userData } = environment(t);
  const api = fakeApi(toolsState());
  // Not added before: the extension starting changes nothing.
  assert.equal(link.keepToolCurrent(api, dir, userData), false);
  const stop = link.start(api, dir, () => {});
  stop();
  assert.deepEqual(api.dispatched, []);
  assert.equal(fs.existsSync(link.toolIconPath(userData, "skyrimse")), false);
});

test("a request to add the tool is planned for a game Vortex found, with Mod Mallet's program", (t) => {
  const { env, exists } = environment(t);
  const planned = link.planMalletTool(toolsState(), request("add-to-tools"), env, exists);
  assert.equal(planned.code, "applied");
  assert.equal(planned.tool.op, "add");
  assert.equal(planned.tool.game, "skyrimse");
  assert.equal(planned.tool.result.path, EXE);
  assert.equal(planned.tool.result.workingDirectory, "D:\\Made Up\\Mod Mallet");
  assert.equal(planned.tool.result.name, "Mod Mallet");

  // No note, a program that is gone, a game Mod Mallet does not know and a
  // game Vortex has not found are refused.
  assert.equal(
    link.planMalletTool(toolsState(), request("add-to-tools"), env, () => false).code,
    "failed",
  );
  assert.equal(
    link.planMalletTool(toolsState({}, "baldursgate3"), request("add-to-tools"), env, exists).code,
    "failed",
  );
  const undiscovered = toolsState();
  undiscovered.settings.gameMode.discovered = {};
  assert.equal(
    link.planMalletTool(undiscovered, request("add-to-tools"), env, exists).code,
    "failed",
  );
});

test("a request through the link adds the tool with its icon and remembers it, and a request removes it", (t) => {
  const { env, dir } = environment(t);
  const before = { local: process.env.LOCALAPPDATA, data: process.env.APPDATA };
  process.env.LOCALAPPDATA = env.LOCALAPPDATA;
  process.env.APPDATA = env.APPDATA;
  const real = fs.existsSync;
  fs.existsSync = (file) => file === EXE || real(file);
  t.after(() => {
    fs.existsSync = real;
    process.env.LOCALAPPDATA = before.local;
    process.env.APPDATA = before.data;
  });
  const api = fakeApi(toolsState());
  api.log = () => {};

  const added = link.handle(api, request("add-to-tools"), NOW, () => {});
  assert.equal(added.code, "applied");
  assert.equal(api.dispatched.length, 1);
  assert.equal(api.dispatched[0].type, "ADD_DISCOVERED_TOOL");
  assert.equal(api.dispatched[0].payload.gameId, "skyrimse");
  assert.equal(api.dispatched[0].payload.manual, true);
  const icon = link.toolIconPath(vortexUserDataFor(env), "skyrimse");
  assert.deepEqual(fs.readFileSync(icon), Buffer.from([1, 2, 3]));
  assert.deepEqual(JSON.parse(fs.readFileSync(path.join(dir, "dashboard-tool.json"))), {
    skyrimse: EXE,
  });

  // With the tool in the state, a request removes it and forgets it.
  const there = fakeApi(toolsState({ "mod-mallet": { id: "mod-mallet", path: EXE } }));
  const removed = link.handle(there, request("remove-from-tools"), NOW, () => {});
  assert.equal(removed.code, "applied");
  assert.equal(there.dispatched[0].type, "SET_TOOL_VISIBLE");
  assert.equal(there.dispatched[0].payload.visible, false);
  assert.deepEqual(JSON.parse(fs.readFileSync(path.join(dir, "dashboard-tool.json"))), {});
});

function vortexUserDataFor(env) {
  return path.join(env.APPDATA, "Vortex");
}

test("the program of a tool the user added follows Mod Mallet; any other tool is left alone", (t) => {
  const { dir, userData } = environment(t);
  const moved = toolsState({ "mod-mallet": { id: "mod-mallet", path: "D:\\Old\\mod-mallet.exe" } });

  // Present in Vortex, but the user did not add it through Mod Mallet.
  const stranger = fakeApi(moved);
  assert.equal(link.keepToolCurrent(stranger, dir, userData), false);
  assert.deepEqual(stranger.dispatched, []);

  // Added before: the path is brought up to date, and not twice.
  fs.writeFileSync(path.join(dir, "dashboard-tool.json"), JSON.stringify({ skyrimse: "x" }));
  const real = fs.existsSync;
  fs.existsSync = (file) => file === EXE || real(file);
  t.after(() => {
    fs.existsSync = real;
  });
  const api = fakeApi(moved);
  assert.equal(link.keepToolCurrent(api, dir, userData), true);
  assert.equal(api.dispatched[0].payload.result.path, EXE);
  const current = fakeApi(toolsState({ "mod-mallet": { id: "mod-mallet", path: EXE } }));
  assert.equal(link.keepToolCurrent(current, dir, userData), false);

  // The user removed it in Vortex since: nothing is added back.
  const gone = fakeApi(toolsState());
  assert.equal(link.keepToolCurrent(gone, dir, userData), false);
  assert.deepEqual(gone.dispatched, []);
});
