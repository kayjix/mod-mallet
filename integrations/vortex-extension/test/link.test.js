// Tests of the extension without Vortex: the state is a made-up object in
// the shape of Vortex's, and the API records what it was asked to do.

const assert = require("node:assert/strict");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { test } = require("node:test");

const link = require("../index.js");

const NOW = 1_800_000_000_000;

function state() {
  return {
    settings: { profiles: { activeProfileId: "p1" }, downloads: { path: "D:\\Downloads" } },
    persistent: {
      profiles: { p1: { gameId: "skyrimse", modState: { "patch-hub": { enabled: true } } } },
      mods: {
        skyrimse: {
          "patch-hub": {
            archiveId: "a1",
            installationPath: "Patch Hub 1.0",
            attributes: {
              logicalFileName: "Patch Hub",
              version: "1.0",
              author: "Someone",
              modId: 1234,
              installerChoices: { type: "fomod", options: [{ name: "Patches", groups: [] }] },
            },
          },
          "No Archive": { installationPath: "No Archive", attributes: {} },
          "Archive Gone": { archiveId: "deleted", installationPath: "Archive Gone" },
          Plain: { archiveId: "a2", installationPath: "Plain", attributes: { name: "Plain" } },
        },
      },
      downloads: { files: { a1: { localPath: "hub.7z" }, a2: { localPath: "plain.zip" } } },
    },
  };
}

function request(overrides = {}) {
  return {
    protocol: 1,
    id: "abc-123",
    action: "reinstall-mod",
    game: "skyrimse",
    modId: "Patch Hub 1.0",
    createdAt: NOW - 1000,
    ...overrides,
  };
}

/** An API that records the events and notifications it receives. */
function fakeApi(current = state()) {
  const api = {
    emitted: [],
    notifications: [],
    getState: () => current,
    events: { emit: (...args) => api.emitted.push(args) },
    sendNotification: (notification) => api.notifications.push(notification),
  };
  return api;
}

test("a mod is found by its staging folder and reinstalled with its previous choices", () => {
  const planned = link.plan(state(), request(), NOW);
  assert.equal(planned.code, "started");
  assert.equal(planned.modName, "Patch Hub");
  assert.equal(planned.install.archiveId, "a1");
  assert.equal(planned.install.options.allowAutoEnable, false);
  assert.deepEqual(planned.install.options.choices.options[0].name, "Patches");
});

test("a mod is also found by Vortex's own id", () => {
  const planned = link.plan(state(), request({ modId: "patch-hub" }), NOW);
  assert.equal(planned.code, "started");
});

test("a mod without saved choices is reinstalled without any", () => {
  const planned = link.plan(state(), request({ modId: "Plain" }), NOW);
  assert.deepEqual(planned.install.options, { allowAutoEnable: false });
});

test("requests that cannot be carried out say why", () => {
  const cases = [
    [request({ protocol: 2 }), "unsupported"],
    [request({ action: "delete-everything" }), "unsupported"],
    [request({ createdAt: NOW - 120_000 }), "expired"],
    [request({ createdAt: undefined }), "expired"],
    [request({ game: "fallout4" }), "wrong-game"],
    [request({ modId: "Not Installed" }), "unknown-mod"],
    [request({ modId: "__proto__" }), "unknown-mod"],
    [request({ modId: "No Archive" }), "archive-missing"],
    [request({ modId: "Archive Gone" }), "archive-missing"],
    [undefined, "unsupported"],
  ];
  for (const [input, code] of cases) {
    const planned = link.plan(state(), input, NOW);
    assert.equal(planned.code, code, JSON.stringify(input));
    assert.equal(planned.install, undefined);
  }
});

test("handling a request emits Vortex's install event and tells the user", () => {
  const api = fakeApi();
  const result = link.handle(api, request(), NOW);

  assert.deepEqual(result, { id: "abc-123", ok: true, code: "started", detail: undefined });
  assert.equal(api.emitted.length, 1);
  const [event, archiveId, options, callback] = api.emitted[0];
  assert.equal(event, "start-install-download");
  assert.equal(archiveId, "a1");
  assert.equal(options.allowAutoEnable, false);
  assert.equal(typeof callback, "function");
  assert.match(api.notifications[0].message, /Patch Hub/);
});

test("after the installer finishes, Vortex is told to put an enabled mod back into the game", () => {
  const api = fakeApi();
  link.handle(api, request(), NOW);
  const finished = api.emitted[0][3];

  finished(null, "patch-hub");

  assert.equal(api.emitted.length, 2);
  assert.deepEqual(api.emitted[1], ["mods-enabled", ["patch-hub"], true, "skyrimse"]);
});

test("a mod that was disabled stays out of the game after its reinstall", () => {
  const api = fakeApi();
  link.handle(api, request({ modId: "Plain" }), NOW);
  const finished = api.emitted[0][3];

  finished(null, "Plain");

  assert.equal(api.emitted.length, 1);
});

test("a cancelled or failed installer changes nothing further", () => {
  const api = fakeApi();
  const errors = [];
  api.showErrorNotification = (title, error) => errors.push([title, error.message]);
  link.handle(api, request(), NOW);
  const finished = api.emitted[0][3];

  const cancelled = new Error("cancelled by the user");
  cancelled.name = "UserCanceled";
  finished(cancelled);
  assert.equal(api.emitted.length, 1);
  assert.deepEqual(errors, []);

  finished(new Error("archive is damaged"));
  assert.equal(api.emitted.length, 1);
  assert.equal(errors.length, 1);
  assert.match(errors[0][1], /damaged/);
});

test("an archive is installed only from Mod Mallet's patches folder", (t) => {
  const local = fs.mkdtempSync(path.join(os.tmpdir(), "mallet-local-"));
  t.after(() => fs.rmSync(local, { recursive: true, force: true }));
  const env = { LOCALAPPDATA: local };
  const patches = link.patchesDir(env);
  fs.mkdirSync(patches, { recursive: true });
  const patch = path.join(patches, "Mod Mallet Patch.zip");
  fs.writeFileSync(patch, "zip");
  const elsewhere = path.join(local, "Other.zip");
  fs.writeFileSync(elsewhere, "zip");
  const ask = (file) =>
    link.plan(state(), request({ action: "install-archive", path: file }), NOW, env);

  const planned = ask(patch);
  assert.equal(planned.code, "installing");
  assert.equal(planned.archive, path.resolve(patch));
  assert.equal(planned.modName, "Mod Mallet Patch");

  assert.equal(ask(elsewhere).code, "not-allowed");
  assert.equal(ask(path.join(patches, "..", "Other.zip")).code, "not-allowed");
  assert.equal(ask(path.join(patches, "Missing.zip")).code, "not-allowed");
  assert.equal(ask(42).code, "not-allowed");
  // However the folder is spelled, only the file name decides.
  assert.equal(ask(patch.split(path.sep).join("/")).code, "installing");
  assert.equal(ask(path.join("X:", "elsewhere", "Mod Mallet Patch.zip")).code, "installing");
  assert.equal(ask(path.join(patches, "notes.txt")).code, "not-allowed");
  assert.match(ask(path.join(patches, "Missing.zip")).detail, /missing/);
});

test("what became of a request is written to Vortex's log", () => {
  const api = fakeApi();
  const lines = [];
  api.log = (level, message, detail) => lines.push([level, message, detail]);
  link.handle(api, request({ modId: "Not Installed" }), NOW);
  assert.deepEqual(lines, [
    [
      "info",
      "Mod Mallet link: request",
      { id: "abc-123", action: "reinstall-mod", code: "unknown-mod", detail: undefined },
    ],
  ]);
});

test("the extension keeps its own log in the shared folder", (t) => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "mallet-log-"));
  t.after(() => fs.rmSync(dir, { recursive: true, force: true }));
  link.appendLog(dir, "info", "request", { code: "installing" }, NOW);
  link.appendLog(dir, "warn", "install failed", undefined, NOW);
  const lines = fs.readFileSync(path.join(dir, "extension.log"), "utf8").split("\n");
  assert.equal(lines[0], `${new Date(NOW).toISOString()} info request {"code":"installing"}`);
  assert.equal(lines[1], `${new Date(NOW).toISOString()} warn install failed`);
});

test("a log that cannot be written does not throw", () => {
  assert.doesNotThrow(() =>
    link.appendLog(path.join(os.tmpdir(), "mallet-no-such-folder", "deeper"), "info", "x"),
  );
});

test("starting creates the patches folder from inside Vortex", (t) => {
  const local = fs.mkdtempSync(path.join(os.tmpdir(), "mallet-local-"));
  const before = process.env.LOCALAPPDATA;
  process.env.LOCALAPPDATA = local;
  t.after(() => {
    process.env.LOCALAPPDATA = before;
    fs.rmSync(local, { recursive: true, force: true });
  });
  const dir = link.bridgeDir(process.env);
  const stop = link.start(fakeApi(), dir, () => {});
  stop();
  assert.ok(fs.statSync(link.patchesDir(process.env)).isDirectory());
});

test("installing an archive is handed to Vortex as a dropped file", (t) => {
  const local = fs.mkdtempSync(path.join(os.tmpdir(), "mallet-local-"));
  const before = process.env.LOCALAPPDATA;
  process.env.LOCALAPPDATA = local;
  t.after(() => {
    process.env.LOCALAPPDATA = before;
    fs.rmSync(local, { recursive: true, force: true });
  });
  const patches = link.patchesDir(process.env);
  fs.mkdirSync(patches, { recursive: true });
  const patch = path.join(patches, "Mod Mallet Patch.zip");
  fs.writeFileSync(patch, "zip");

  const api = fakeApi();
  const result = link.handle(api, request({ action: "install-archive", path: patch }), NOW);

  assert.equal(result.ok, true);
  assert.equal(result.code, "installing");
  assert.equal(api.emitted[0][0], "start-install");
  assert.equal(api.emitted[0][1], path.resolve(patch));
});

test("a refused request changes nothing in Vortex", () => {
  const api = fakeApi();
  const result = link.handle(api, request({ modId: "Not Installed" }), NOW);
  assert.equal(result.ok, false);
  assert.equal(result.code, "unknown-mod");
  assert.deepEqual(api.emitted, []);
  assert.deepEqual(api.notifications, []);
});

test("request files are answered once and removed", (t) => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "mod-mallet-link-"));
  t.after(() => fs.rmSync(dir, { recursive: true, force: true }));
  fs.mkdirSync(path.join(dir, "requests"));
  fs.mkdirSync(path.join(dir, "results"));
  const write = (name, content) => fs.writeFileSync(path.join(dir, "requests", name), content);

  write("abc-123.json", JSON.stringify(request()));
  write("0f.json", JSON.stringify(request({ id: "something-else" })));
  write("dead-beef.json", "{ not json");
  write("notes.txt", "not a request");

  const api = fakeApi();
  link.processRequests(api, dir, NOW);

  const result = (name) => JSON.parse(fs.readFileSync(path.join(dir, "results", name), "utf8"));
  assert.equal(result("abc-123.json").code, "started");
  assert.equal(result("0f.json").code, "unsupported");
  assert.equal(result("dead-beef.json").code, "failed");
  assert.equal(api.emitted.length, 1);

  // Only the file that is not a request is left, and nothing runs twice.
  assert.deepEqual(fs.readdirSync(path.join(dir, "requests")), ["notes.txt"]);
  link.processRequests(api, dir, NOW);
  assert.equal(api.emitted.length, 1);
});

test("the status file says which protocol is spoken and when", (t) => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "mod-mallet-link-"));
  t.after(() => fs.rmSync(dir, { recursive: true, force: true }));

  link.writeStatus(dir, NOW - 5000, NOW);
  const status = JSON.parse(fs.readFileSync(path.join(dir, "status.json"), "utf8"));
  assert.deepEqual(status, {
    protocol: link.PROTOCOL,
    extensionVersion: link.VERSION,
    startedAt: NOW - 5000,
    heartbeatAt: NOW,
  });
});

test("starting creates the folders and stopping removes the status", (t) => {
  const dir = path.join(fs.mkdtempSync(path.join(os.tmpdir(), "mod-mallet-link-")), "bridge");
  t.after(() => fs.rmSync(path.dirname(dir), { recursive: true, force: true }));

  const stop = link.start(fakeApi(), dir, () => {});
  assert.ok(fs.existsSync(path.join(dir, "requests")));
  assert.ok(fs.existsSync(path.join(dir, "results")));
  assert.ok(fs.existsSync(path.join(dir, "status.json")));
  stop();
  assert.ok(!fs.existsSync(path.join(dir, "status.json")));
});

test("a snapshot holds what Mod Mallet reads, in the shape of Vortex's state", () => {
  const value = link.snapshot(state());

  assert.equal(value.protocol, 1);
  assert.equal(value.game, "skyrimse");
  assert.equal(value.settings.downloads.path, "D:\\Downloads");
  assert.deepEqual(value.settings.profiles.lastActiveProfile, { skyrimse: "p1" });
  assert.deepEqual(value.persistent.profiles.p1.modState, { "patch-hub": { enabled: true } });
  const hub = value.persistent.mods.skyrimse["patch-hub"];
  assert.equal(hub.installationPath, "Patch Hub 1.0");
  assert.equal(hub.attributes.logicalFileName, "Patch Hub");
  assert.equal(hub.attributes.installerChoices.options[0].name, "Patches");
  assert.equal(hub.attributes.version, "1.0");
  assert.equal(hub.attributes.author, "Someone");
  assert.equal(hub.attributes.modId, 1234);
  // Only archives a mod came from are named, and only by their file.
  assert.deepEqual(value.persistent.downloads.files, {
    a1: { localPath: "hub.7z" },
    a2: { localPath: "plain.zip" },
  });
});

test("a snapshot leaves out everything else Vortex knows", () => {
  const current = state();
  current.confidential = { account: "someone" };
  current.persistent.mods.othergame = { Elsewhere: {} };
  current.persistent.mods.skyrimse["patch-hub"].attributes.notes = "private";
  current.persistent.profiles.p2 = { gameId: "othergame" };

  const text = JSON.stringify(link.snapshot(current));

  for (const absent of ["someone", "othergame", "Elsewhere", "private", "p2"]) {
    assert.equal(text.includes(absent), false, absent);
  }
});

test("without a game in use there is no snapshot", () => {
  assert.equal(link.snapshot({}), undefined);
  assert.equal(link.snapshot(undefined), undefined);
  const odd = state();
  odd.persistent.profiles.p1.gameId = "..\\elsewhere";
  assert.equal(link.snapshot(odd), undefined);
});

test("the state is written once and again only after it changed", (t) => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "mallet-link-"));
  t.after(() => fs.rmSync(dir, { recursive: true, force: true }));
  const file = path.join(dir, "state", "skyrimse.json");
  const current = state();

  const first = link.writeStateIfChanged(dir, current, undefined, NOW);
  assert.equal(JSON.parse(fs.readFileSync(file, "utf8")).writtenAt, NOW);

  // Nothing changed: the file is left alone.
  const second = link.writeStateIfChanged(dir, current, first, NOW + 1000);
  assert.equal(second, first);
  assert.equal(JSON.parse(fs.readFileSync(file, "utf8")).writtenAt, NOW);

  // Vortex replaces the object that holds the mods when one changes.
  const changed = { ...current, persistent: { ...current.persistent } };
  changed.persistent.mods = { skyrimse: { ...current.persistent.mods.skyrimse, New: {} } };
  const third = link.writeStateIfChanged(dir, changed, second, NOW + 2000);
  assert.notEqual(third, second);
  const written = JSON.parse(fs.readFileSync(file, "utf8"));
  assert.equal(written.writtenAt, NOW + 2000);
  assert.ok(written.persistent.mods.skyrimse.New);
});

test("the shared folder lies below the local application data", () => {
  assert.equal(
    link.bridgeDir({ LOCALAPPDATA: path.join("C:", "Users", "x", "AppData", "Local") }),
    path.join("C:", "Users", "x", "AppData", "Local", "mod-mallet", "vortex-bridge"),
  );
  assert.equal(link.bridgeDir({}), undefined);
});

test("the version matches info.json and package.json", () => {
  const read = (name) => JSON.parse(fs.readFileSync(path.join(__dirname, "..", name), "utf8"));
  assert.equal(read("info.json").version, link.VERSION);
  assert.equal(read("package.json").version, link.VERSION);
});

test("a plugin rule is planned for plugin names only, and not twice", () => {
  const ask = (current, plugin, after) =>
    link.plan(current, { ...request(), action: "load-after", plugin, after }, NOW);
  const planned = ask(state(), "Faces.esp", "College.esp");
  assert.equal(planned.code, "applied");
  assert.deepEqual(planned.rule, { pluginId: "faces.esp", reference: "college.esp" });

  assert.equal(ask(state(), "Faces.esp", "..evil.dll").code, "failed");
  assert.equal(ask(state(), "Faces.esp", undefined).code, "failed");

  const withRule = {
    ...state(),
    userlist: { plugins: [{ name: "faces.esp", after: ["College.esp"] }] },
  };
  const again = ask(withRule, "Faces.esp", "College.esp");
  assert.equal(again.code, "applied");
  assert.equal(again.rule, undefined);
});

test("a plugin rule is dispatched as Vortex's own and the plugins are sorted", () => {
  const api = fakeApi();
  const dispatched = [];
  api.store = { dispatch: (action) => dispatched.push(action) };
  const result = link.handle(
    api,
    { ...request(), action: "load-after", plugin: "Faces.esp", after: "College.esp" },
    NOW,
  );
  assert.equal(result.ok, true);
  assert.deepEqual(dispatched, [
    {
      type: "ADD_USERLIST_RULE",
      payload: { pluginId: "faces.esp", reference: "college.esp", type: "after" },
    },
  ]);
  assert.deepEqual(api.emitted, [["autosort-plugins", true]]);
});

test("a mod is switched off in the active profile as Vortex's mod list does it", () => {
  const api = fakeApi();
  const dispatched = [];
  api.store = { dispatch: (action) => dispatched.push(action) };
  const result = link.handle(
    api,
    { ...request(), action: "set-mod-enabled", modId: "Patch Hub 1.0", enabled: false },
    NOW,
  );
  assert.equal(result.code, "applied");
  assert.deepEqual(dispatched, [
    { type: "SET_MOD_ENABLED", payload: { profileId: "p1", modId: "patch-hub", enable: false } },
  ]);
  assert.deepEqual(api.emitted, [["mods-enabled", ["patch-hub"], false, "skyrimse"]]);
  const unclear = link.plan(
    state(),
    { ...request(), action: "set-mod-enabled", modId: "Patch Hub 1.0" },
    NOW,
  );
  assert.equal(unclear.code, "failed");
});

test("plugins are switched off in the plugin list, which is then shown narrowed to them", () => {
  const api = fakeApi();
  const dispatched = [];
  api.store = { dispatch: (action) => dispatched.push(action) };
  const result = link.handle(
    api,
    {
      ...request(),
      action: "set-plugins-enabled",
      plugins: ["Citizens.esp", "Lights - Citizens patch.esp"],
      enabled: false,
    },
    NOW,
  );
  assert.equal(result.code, "applied");
  assert.equal(result.ok, true);
  assert.deepEqual(dispatched, [
    { type: "SET_PLUGIN_ENABLED", payload: { pluginName: "Citizens.esp", enabled: false } },
    {
      type: "SET_PLUGIN_ENABLED",
      payload: { pluginName: "Lights - Citizens patch.esp", enabled: false },
    },
    {
      type: "SET_ATTRIBUTE_FILTER",
      payload: { tableId: "gamebryo-plugins", attributeId: "name", filter: "Citizens.esp" },
    },
  ]);
  assert.deepEqual(api.emitted, [["show-main-page", "gamebryo-plugins"]]);

  const ask = (fields) =>
    link.plan(state(), { ...request(), action: "set-plugins-enabled", ...fields }, NOW).code;
  assert.equal(ask({ plugins: ["Citizens.esp"] }), "failed");
  assert.equal(ask({ plugins: [], enabled: false }), "failed");
  assert.equal(ask({ plugins: ["..\\evil.dll"], enabled: false }), "failed");
  assert.equal(ask({ plugins: "Citizens.esp", enabled: false }), "failed");
});

const VIGIL = { step: "Patches", group: "Guilds", option: "Vigil", index: 3 };

test("options to tick are added to the choices saved last time", () => {
  const saved = {
    type: "fomod",
    options: [
      { name: "Patches", groups: [{ name: "Guilds", choices: [{ name: "Thieves", idx: 1 }] }] },
    ],
  };
  const ticked = link.withTicked(saved, [
    VIGIL,
    { step: "Patches", group: "Towns", option: "Riverwood", index: 0 },
    { step: "Extras", group: "Maps", option: "Paper Map", index: 2 },
  ]);

  assert.deepEqual(ticked.options, [
    {
      name: "Patches",
      groups: [
        {
          name: "Guilds",
          choices: [
            { name: "Thieves", idx: 1 },
            { name: "Vigil", idx: 3 },
          ],
        },
        { name: "Towns", choices: [{ name: "Riverwood", idx: 0 }] },
      ],
    },
    { name: "Extras", groups: [{ name: "Maps", choices: [{ name: "Paper Map", idx: 2 }] }] },
  ]);
  // The saved choices themselves are not changed.
  assert.equal(saved.options[0].groups[0].choices.length, 1);
});

test("nothing is ticked without saved FOMOD choices, twice, or from a malformed list", () => {
  assert.equal(link.withTicked(undefined, [VIGIL]), undefined);
  const other = { type: "something-else", options: [] };
  assert.equal(link.withTicked(other, [VIGIL]), other);

  const saved = {
    type: "fomod",
    options: [
      { name: "Patches", groups: [{ name: "Guilds", choices: [{ name: "Vigil", idx: 3 }] }] },
    ],
  };
  assert.equal(link.withTicked(saved, [VIGIL]), saved);
  for (const tick of [
    undefined,
    "Vigil",
    [{ ...VIGIL, index: -1 }],
    [{ ...VIGIL, option: "" }],
    [null],
  ]) {
    assert.equal(link.withTicked(saved, tick), saved, JSON.stringify(tick));
  }
});

test("a reinstall with options to tick plans them as the installer's starting point", () => {
  const planned = link.plan(state(), request({ tick: [VIGIL] }), NOW);

  assert.equal(planned.code, "started");
  const groups = planned.install.preset.options[0].groups;
  assert.deepEqual(groups, [{ name: "Guilds", choices: [{ name: "Vigil", idx: 3 }] }]);
  assert.equal(planned.install.options.choices, planned.install.preset);
  assert.deepEqual(planned.install.previous.options[0].groups, []);
});

/** An API with a store that records what is dispatched to it. */
function apiWithStore(current = state()) {
  const api = fakeApi(current);
  api.dispatched = [];
  api.store = { dispatch: (action) => api.dispatched.push(action) };
  return api;
}

test("the choices are saved with the mod before its installer starts", () => {
  const api = apiWithStore();
  link.handle(api, request({ tick: [VIGIL] }), NOW);

  assert.equal(api.dispatched.length, 1);
  const [saved] = api.dispatched;
  assert.equal(saved.type, "SET_MOD_ATTRIBUTE");
  assert.equal(saved.payload.gameId, "skyrimse");
  assert.equal(saved.payload.modId, "patch-hub");
  assert.equal(saved.payload.attribute, "installerChoices");
  assert.equal(saved.payload.value.options[0].groups[0].choices[0].name, "Vigil");
  assert.equal(api.emitted[0][0], "start-install-download");
});

test("a cancelled installer gets the mod's previous choices back", () => {
  const api = apiWithStore();
  link.handle(api, request({ tick: [VIGIL] }), NOW);
  const finished = api.emitted[0][3];

  const cancelled = new Error("cancelled by the user");
  cancelled.name = "UserCanceled";
  finished(cancelled);

  assert.equal(api.dispatched.length, 2);
  assert.deepEqual(api.dispatched[1].payload.value.options, [{ name: "Patches", groups: [] }]);
});

test("choices are not put back on a mod that is gone, nor after a finished install", () => {
  const current = state();
  const api = apiWithStore(current);
  link.handle(api, request({ tick: [VIGIL] }), NOW);
  delete current.persistent.mods.skyrimse["patch-hub"];
  api.emitted[0][3](new Error("archive is damaged"));
  assert.equal(api.dispatched.length, 1);

  const other = apiWithStore();
  link.handle(other, request({ tick: [VIGIL] }), NOW);
  other.emitted[0][3](null, "patch-hub");
  assert.equal(other.dispatched.length, 1);
});

test("a reinstall without options to tick saves nothing with the mod", () => {
  const api = apiWithStore();
  link.handle(api, request(), NOW);
  assert.deepEqual(api.dispatched, []);
});

/** A file in the game's data folder, as Vortex keeps it in an override. */
const inData = (file) => path.win32.join("C:/Games/Skyrim/Data", file);

/** A state in which Vortex knows the game and two mods bring the same sword. */
function swordState() {
  const current = state();
  current.settings.gameMode = { discovered: { skyrimse: { path: "C:/Games/Skyrim" } } };
  current.persistent.mods.skyrimse.Plain.fileOverrides = [
    inData("meshes/weapons/sword.nif"),
    inData("meshes/other.nif"),
  ];
  return current;
}

function overrides(extra = {}) {
  return {
    ...request(),
    action: "file-overrides",
    files: ["meshes/weapons/sword.nif", "textures/weapons/sword.dds"],
    leaveOut: ["Patch Hub 1.0"],
    provide: ["Plain"],
    ...extra,
  };
}

test("file overrides leave the files out of some mods and give them back to others", () => {
  const planned = link.plan(swordState(), overrides(), NOW);
  assert.equal(planned.code, "applied");
  assert.deepEqual(planned.overrides, [
    {
      modId: "patch-hub",
      files: [inData("meshes/weapons/sword.nif"), inData("textures/weapons/sword.dds")],
    },
    // Only the files asked about are taken out; other overrides stay.
    { modId: "Plain", files: [inData("meshes/other.nif")] },
  ]);
});

test("file overrides are dispatched as Vortex's editor does and the mods deployed", () => {
  const api = fakeApi(swordState());
  const dispatched = [];
  api.store = { dispatch: (action) => dispatched.push(action) };
  const result = link.handle(api, overrides(), NOW);
  assert.equal(result.ok, true);
  assert.deepEqual(
    dispatched.map((action) => [action.type, action.payload.modId]),
    [
      ["SET_FILE_OVERRIDE", "patch-hub"],
      ["SET_FILE_OVERRIDE", "Plain"],
    ],
  );
  assert.equal(api.emitted.length, 1);
  assert.equal(api.emitted[0][0], "deploy-mods");
  assert.equal(typeof api.emitted[0][1], "function");
});

test("file overrides are refused for paths out of the data folder, unknown mods and unknown games", () => {
  for (const files of [[], ["../Skyrim.ini"], ["C:/Windows/x.dll"], "meshes/x.nif"]) {
    assert.equal(link.plan(swordState(), overrides({ files }), NOW).code, "failed");
  }
  assert.equal(
    link.plan(swordState(), overrides({ leaveOut: ["Nobody"] }), NOW).code,
    "unknown-mod",
  );
  assert.equal(link.plan(state(), overrides(), NOW).code, "failed");
});
