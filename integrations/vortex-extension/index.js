// Mod Mallet link: lets Mod Mallet ask Vortex to do things.
//
// Mod Mallet never changes a setup behind the mod manager's back
// (ADR 0005). When a finding can be fixed by Vortex, Mod Mallet writes a
// request into a folder, and this extension, running inside Vortex, carries
// it out with the same calls Vortex's own menus make.
//
// The folder is %LOCALAPPDATA%\mod-mallet\vortex-bridge:
//
//   status.json            written here every few seconds: "I am running"
//   state\<game>.json      written here when it changes: the installed mods
//   requests\<id>.json     written by Mod Mallet
//   ..\patches\*.zip       written by Mod Mallet: patches to install
//   results\<id>.json      written here: what happened to a request
//
// The format of these files is described in docs/src/reference/vortex-link.md.
// It is versioned by PROTOCOL; a request of another version is refused.
//
// The file is plain CommonJS and needs nothing but Node's own modules, so
// that it can be copied into Vortex's plugin folder as it is. Everything
// that decides something is a pure function (see `plan`) and is tested in
// test/link.test.js without Vortex.

const fs = require("node:fs");
const path = require("node:path");

const PROTOCOL = 1;
const VERSION = "0.7.0";

/** How often status.json is refreshed. */
const HEARTBEAT_MS = 5000;
/** How often the request folder is looked at. */
const POLL_MS = 1000;
/** A request older than this was given up on by Mod Mallet long ago. */
const MAX_REQUEST_AGE_MS = 60000;
/** Request files are named by Mod Mallet; anything else is left alone. */
const REQUEST_FILE = /^[0-9a-f-]{1,64}\.json$/;
/** A game's identifier becomes a file name only if it looks like this. */
const GAME_ID = /^[A-Za-z0-9_-]{1,64}$/;

/** The folder shared with Mod Mallet, or undefined if it cannot be known. */
function bridgeDir(env) {
  const base = env.LOCALAPPDATA;
  return base ? path.join(base, "mod-mallet", "vortex-bridge") : undefined;
}

/** The folder in which Mod Mallet puts the patches it writes. */
function patchesDir(env) {
  const base = env.LOCALAPPDATA;
  return base ? path.join(base, "mod-mallet", "patches") : undefined;
}

/** The name of an archive Mod Mallet may ask to install. */
const PATCH_FILE = /^[^\\/:*?"<>|]{1,120}\.zip$/i;

/** The actions this version of the extension carries out. */
const ACTIONS = [
  "reinstall-mod",
  "install-archive",
  "load-after",
  "set-mod-enabled",
  "file-overrides",
];

/** How many files one request for file overrides may name. */
const MAX_OVERRIDE_FILES = 200;

/** A plugin's file name, as a rule in the plugin list may name it. */
const PLUGIN_FILE = /^[^\\/:*?"<>|]{1,200}\.(esp|esm|esl)$/i;

/** The profile Vortex is currently using. */
function activeProfile(state) {
  const profileId = state?.settings?.profiles?.activeProfileId;
  return state?.persistent?.profiles?.[profileId];
}

/** The game Vortex is currently managing. */
function activeGame(state) {
  return activeProfile(state)?.gameId;
}

/**
 * Finds Vortex's id of a mod, given that id or the mod's folder in the
 * staging directory. Mod Mallet knows mods by their folder, because that is
 * what Vortex's deployment manifest names.
 */
function findModId(state, game, modId) {
  const mods = state?.persistent?.mods?.[game] ?? {};
  if (Object.hasOwn(mods, modId)) {
    return modId;
  }
  return Object.keys(mods).find((id) => mods[id]?.installationPath === modId);
}

/**
 * Decides what to do with a request, without doing it.
 *
 * Returns `{ code, detail }` and, if something is to be installed,
 * `modName` and either `install: { archiveId, options, modId, game,
 * wasEnabled }` to reinstall a mod or `archive` to install a file.
 */
function plan(state, request, now, env = process.env) {
  if (request?.protocol !== PROTOCOL || !ACTIONS.includes(request?.action)) {
    return { code: "unsupported", detail: `protocol ${PROTOCOL}, actions ${ACTIONS.join(", ")}` };
  }
  if (typeof request.createdAt !== "number" || now - request.createdAt > MAX_REQUEST_AGE_MS) {
    return { code: "expired" };
  }
  const game = activeGame(state);
  if (game !== request.game) {
    return { code: "wrong-game", detail: game ?? "no game" };
  }
  if (request.action === "install-archive") {
    return planArchive(request.path, env);
  }
  if (request.action === "load-after") {
    return planLoadAfter(state, request);
  }
  if (request.action === "file-overrides") {
    return planFileOverrides(state, request, game);
  }
  const modId = findModId(state, game, request.modId);
  if (modId === undefined) {
    return { code: "unknown-mod" };
  }
  if (request.action === "set-mod-enabled") {
    const profileId = state?.settings?.profiles?.activeProfileId;
    if (typeof request.enabled !== "boolean" || profileId === undefined) {
      return { code: "failed", detail: "no profile, or enabled is not true or false" };
    }
    return {
      code: "applied",
      enable: { profileId, modId, game, enabled: request.enabled },
    };
  }
  const mod = state.persistent.mods[game][modId];
  const archiveId = mod.archiveId;
  if (!archiveId || state?.persistent?.downloads?.files?.[archiveId] === undefined) {
    return { code: "archive-missing" };
  }

  // Exactly what Vortex's own "Reinstall" does: install the mod's archive
  // again, with the choices made last time as the starting point of the
  // installer dialog, and without changing whether the mod is enabled.
  // The options Mod Mallet names are added to those choices. Vortex starts
  // the dialog of a reinstall from the choices saved with the mod, so they
  // are saved there just before (see `handle`).
  const previous = mod.attributes?.installerChoices;
  const choices = withTicked(previous, request.tick);
  const preset = choices === previous ? undefined : choices;
  const options =
    choices === undefined ? { allowAutoEnable: false } : { choices, allowAutoEnable: false };
  const attributes = mod.attributes ?? {};
  const modName =
    attributes.customFileName || attributes.logicalFileName || attributes.name || request.modId;
  const wasEnabled = activeProfile(state)?.modState?.[modId]?.enabled === true;
  return {
    code: "started",
    install: { archiveId, options, modId, game, wasEnabled, preset, previous },
    modName,
  };
}

/** A field of an option to tick, as Mod Mallet sends it. */
function isTitle(value) {
  return typeof value === "string" && value.length > 0 && value.length <= 500;
}

/**
 * The choices a FOMOD installer saved last time, with the options in
 * `tick` added, each with its title and its position in its group. Without
 * saved FOMOD choices nothing is added: the installer then starts from its
 * own defaults, which a list of a few options must not replace. Returns
 * `choices` itself if nothing was added.
 */
function withTicked(choices, tick) {
  const wanted = (Array.isArray(tick) ? tick : []).filter(
    (item) =>
      isTitle(item?.step) &&
      isTitle(item?.group) &&
      isTitle(item?.option) &&
      Number.isInteger(item?.index) &&
      item.index >= 0 &&
      item.index < 10000,
  );
  if (wanted.length === 0 || choices?.type !== "fomod" || !Array.isArray(choices.options)) {
    return choices;
  }
  const options = structuredClone(choices.options);
  let added = 0;
  for (const { step, group, option, index } of wanted) {
    let stepChoice = options.find((entry) => entry?.name === step);
    if (stepChoice === undefined) {
      stepChoice = { name: step, groups: [] };
      options.push(stepChoice);
    }
    if (!Array.isArray(stepChoice.groups)) {
      stepChoice.groups = [];
    }
    let groupChoice = stepChoice.groups.find((entry) => entry?.name === group);
    if (groupChoice === undefined) {
      groupChoice = { name: group, choices: [] };
      stepChoice.groups.push(groupChoice);
    }
    if (!Array.isArray(groupChoice.choices)) {
      groupChoice.choices = [];
    }
    if (!groupChoice.choices.some((entry) => entry?.name === option)) {
      groupChoice.choices.push({ name: option, idx: index });
      added += 1;
    }
  }
  return added === 0 ? choices : { ...choices, options };
}

/**
 * A rule for the plugin list: `plugin` loads after `after`. Only plugin
 * file names are accepted; a rule that exists already is not added again.
 */
function planLoadAfter(state, request) {
  const { plugin, after } = request;
  if (typeof plugin !== "string" || typeof after !== "string") {
    return { code: "failed", detail: "plugin or after missing" };
  }
  if (!PLUGIN_FILE.test(plugin) || !PLUGIN_FILE.test(after)) {
    return { code: "failed", detail: `not plugin names: ${plugin}, ${after}` };
  }
  const pluginId = plugin.toLowerCase();
  const reference = after.toLowerCase();
  const existing = (state?.userlist?.plugins ?? []).find((entry) => entry?.name === pluginId);
  const known = (existing?.after ?? []).some(
    (entry) => (typeof entry === "string" ? entry : entry?.name)?.toLowerCase() === reference,
  );
  return { code: "applied", rule: known ? undefined : { pluginId, reference } };
}

/** A file below the data directory, as Mod Mallet names it: relative, without `..`. */
function isDataFile(value) {
  return (
    typeof value === "string" &&
    value.length > 0 &&
    value.length <= 260 &&
    !path.win32.isAbsolute(value) &&
    !/[:*?"<>|]/.test(value) &&
    !value.split(/[\\/]/).includes("..")
  );
}

/**
 * Which files the mods are to leave out (ADR 0053): `files` (below the data
 * directory) are added to the file overrides of the mods in `leaveOut`, so
 * that Vortex does not deploy their copies, and taken out of those of the
 * mods in `provide`, so that it deploys theirs again. Vortex keeps the
 * overrides as full paths into the folder it deploys to, as its own editor
 * of file conflicts does.
 */
function planFileOverrides(state, request, game) {
  const { files } = request;
  const leaveOut = request.leaveOut ?? [];
  const provide = request.provide ?? [];
  if (
    !Array.isArray(files) ||
    files.length === 0 ||
    files.length > MAX_OVERRIDE_FILES ||
    !files.every(isDataFile) ||
    !Array.isArray(leaveOut) ||
    !Array.isArray(provide)
  ) {
    return { code: "failed", detail: "files, leaveOut or provide are not lists of names" };
  }
  const discovered = state?.settings?.gameMode?.discovered?.[game]?.path;
  if (typeof discovered !== "string" || discovered.length === 0) {
    return { code: "failed", detail: "Vortex does not know where the game is" };
  }
  const full = files.map((file) => path.win32.join(discovered, "Data", file));
  const named = (entry) =>
    typeof entry === "string" && full.some((file) => file.toLowerCase() === entry.toLowerCase());
  const overrides = [];
  for (const [ids, leave] of [
    [leaveOut, true],
    [provide, false],
  ]) {
    for (const id of ids) {
      const modId = typeof id === "string" ? findModId(state, game, id) : undefined;
      if (modId === undefined) {
        return { code: "unknown-mod", detail: String(id) };
      }
      const existing = state.persistent.mods[game][modId].fileOverrides ?? [];
      const kept = (Array.isArray(existing) ? existing : []).filter(
        (entry) => typeof entry === "string" && !named(entry),
      );
      overrides.push({ modId, files: leave ? [...kept, ...full] : kept });
    }
  }
  return { code: "applied", overrides };
}

/**
 * Whether an archive may be installed: only a zip file that Mod Mallet put
 * into its own patches folder, so that a request cannot make Vortex
 * install anything else.
 */
function planArchive(file, env) {
  const allowed = patchesDir(env);
  if (allowed === undefined || typeof file !== "string") {
    return { code: "not-allowed", detail: "no patches folder, or no file named" };
  }
  // Only the file name counts. The folder is always Mod Mallet's own, so
  // no request can point anywhere else, however its path is spelled.
  const name = file.split(/[\\/]/).pop() ?? "";
  if (!PATCH_FILE.test(name)) {
    return { code: "not-allowed", detail: `not a patch archive: ${name}` };
  }
  const archive = path.join(allowed, name);
  let isFile = false;
  try {
    isFile = fs.statSync(archive).isFile();
  } catch {
    // Missing; said below.
  }
  if (!isFile) {
    return { code: "not-allowed", detail: `missing: ${archive}` };
  }
  return { code: "installing", archive, modName: name.slice(0, -".zip".length) };
}

/** The name of the extension's own log file in the shared folder. */
const LOG_FILE = "extension.log";

/** From this size on, the log file is set aside before writing more. */
const LOG_MAX_BYTES = 1024 * 1024;

/**
 * Appends one line to the extension's log in the shared folder, where Mod
 * Mallet, its user or a supporter can read it. Vortex's own log is not
 * enough: it is rotated on every start and not every build writes an
 * extension's messages into it. Never throws.
 */
function appendLog(dir, level, message, detail, now = Date.now()) {
  try {
    const file = path.join(dir, LOG_FILE);
    let size = 0;
    try {
      size = fs.statSync(file).size;
    } catch {
      // No log yet.
    }
    if (size > LOG_MAX_BYTES) {
      fs.renameSync(file, path.join(dir, "extension.previous.log"));
    }
    const rest = detail === undefined ? "" : ` ${JSON.stringify(detail)}`;
    fs.appendFileSync(file, `${new Date(now).toISOString()} ${level} ${message}${rest}\n`);
  } catch {
    // A log that cannot be written must not stop the link.
  }
}

/** Carries out a request and returns the result to write. */
function handle(
  api,
  request,
  now,
  log = (level, message, detail) => api.log?.(level, `Mod Mallet link: ${message}`, detail),
) {
  const planned = plan(api.getState(), request, now);
  log("info", "request", {
    id: request?.id,
    action: request?.action,
    code: planned.code,
    detail: planned.detail,
  });
  if (planned.archive !== undefined) {
    // What Vortex does when a file is dropped on it: install it as a mod.
    api.events.emit("start-install", planned.archive, (error) => {
      if (error && error.name !== "UserCanceled") {
        log("warn", "install failed", {
          archive: planned.archive,
          error: String(error?.message ?? error),
        });
        api.showErrorNotification?.("Mod Mallet: the patch could not be installed", error, {
          allowReport: false,
        });
      } else {
        log("info", error ? "install cancelled" : "installed", { archive: planned.archive });
      }
    });
    api.sendNotification?.({
      type: "info",
      message: `Mod Mallet asked to install "${planned.modName}".`,
      displayMS: 5000,
    });
  }
  if (planned.rule !== undefined) {
    // What Vortex's plugin list does when the user adds a rule there.
    api.store?.dispatch({
      type: "ADD_USERLIST_RULE",
      payload: {
        pluginId: planned.rule.pluginId,
        reference: planned.rule.reference,
        type: "after",
      },
    });
  }
  if (request?.action === "load-after" && planned.code === "applied") {
    api.events.emit("autosort-plugins", true);
  }
  if (planned.overrides !== undefined) {
    // What Vortex's editor of file conflicts does when it saves.
    for (const { modId, files } of planned.overrides) {
      api.store?.dispatch({
        type: "SET_FILE_OVERRIDE",
        payload: { gameId: request.game, modId, files },
      });
    }
    // What Vortex's own "Deploy" button does.
    api.events.emit("deploy-mods", (error) => {
      if (error) {
        log("warn", "deployment after file overrides failed", {
          error: String(error?.message ?? error),
        });
      }
    });
  }
  if (planned.enable !== undefined) {
    const { profileId, modId, game, enabled } = planned.enable;
    // What Vortex's own switch in the mod list does.
    api.store?.dispatch({
      type: "SET_MOD_ENABLED",
      payload: { profileId, modId, enable: enabled },
    });
    api.events.emit("mods-enabled", [modId], enabled, game);
  }
  if (planned.install !== undefined) {
    const { game, modId, preset, previous } = planned.install;
    // What Vortex does when it saves a mod's installer choices.
    const saveChoices = (value) =>
      api.store?.dispatch({
        type: "SET_MOD_ATTRIBUTE",
        payload: { gameId: game, modId, attribute: "installerChoices", value },
      });
    if (preset !== undefined) {
      saveChoices(preset);
      log("info", "options ticked in advance", { modId, ticked: request.tick?.length });
    }
    api.events.emit(
      "start-install-download",
      planned.install.archiveId,
      planned.install.options,
      (error) => {
        if (error) {
          log("info", "reinstall ended", {
            modId: planned.install.modId,
            error: String(error?.message ?? error),
          });
          // The installer did not run to its end, so the choices saved
          // with the mod are put back, if the mod is still there.
          if (preset !== undefined && api.getState()?.persistent?.mods?.[game]?.[modId]) {
            saveChoices(previous);
          }
          // The user closing the installer is not an error.
          if (error.name !== "UserCanceled") {
            api.showErrorNotification?.("Mod Mallet: the mod could not be reinstalled", error, {
              allowReport: false,
            });
          }
          return;
        }
        // Vortex's own "Reinstall" ends with this as well. Without it the
        // mod's new files stay out of the game: Vortex took the old ones
        // out before installing, and only this event makes it put the new
        // ones in (or tell the user that a deployment is due).
        if (planned.install.wasEnabled) {
          api.events.emit("mods-enabled", [planned.install.modId], true, planned.install.game);
        }
      },
    );
    api.sendNotification?.({
      type: "info",
      message: `Mod Mallet asked to reinstall "${planned.modName}".`,
      displayMS: 5000,
    });
  }
  return {
    id: request?.id,
    ok: planned.code === "started" || planned.code === "installing" || planned.code === "applied",
    code: planned.code,
    detail: planned.detail,
  };
}

/**
 * The parts of Vortex's state that Mod Mallet reads, for the game in use:
 * the installed mods, which of them are enabled, the archives they came
 * from and what was chosen in their installers.
 *
 * The result has the shape of Vortex's own state, so that Mod Mallet reads
 * it the same way as the backup Vortex writes once an hour. Returns
 * undefined while Vortex manages no game.
 */
function snapshot(state) {
  const profileId = state?.settings?.profiles?.activeProfileId;
  const profile = activeProfile(state);
  const game = profile?.gameId;
  if (typeof game !== "string" || !GAME_ID.test(game)) {
    return undefined;
  }

  const mods = {};
  const downloads = {};
  for (const [id, mod] of Object.entries(state?.persistent?.mods?.[game] ?? {})) {
    const attributes = mod?.attributes ?? {};
    mods[id] = {
      archiveId: mod?.archiveId,
      installationPath: mod?.installationPath,
      attributes: {
        customFileName: attributes.customFileName,
        logicalFileName: attributes.logicalFileName,
        modName: attributes.modName,
        name: attributes.name,
        installerChoices: attributes.installerChoices,
        version: attributes.version,
        newestVersion: attributes.newestVersion,
        author: attributes.author,
        homepage: attributes.homepage,
        source: attributes.source,
        downloadGame: attributes.downloadGame,
        modId: attributes.modId,
      },
    };
    const localPath = state?.persistent?.downloads?.files?.[mod?.archiveId]?.localPath;
    if (localPath !== undefined) {
      downloads[mod.archiveId] = { localPath };
    }
  }

  const modState = {};
  for (const [id, entry] of Object.entries(profile.modState ?? {})) {
    modState[id] = { enabled: entry?.enabled === true };
  }

  return {
    protocol: PROTOCOL,
    game,
    settings: {
      downloads: { path: state?.settings?.downloads?.path },
      profiles: { activeProfileId: profileId, lastActiveProfile: { [game]: profileId } },
    },
    persistent: {
      profiles: { [profileId]: { gameId: game, modState } },
      mods: { [game]: mods },
      downloads: { files: downloads },
    },
  };
}

/**
 * What a snapshot is made from. Vortex replaces these objects when
 * something in them changes and keeps them otherwise, so comparing them is
 * a cheap way to see that there is something new to write.
 */
function snapshotSources(state) {
  const profile = activeProfile(state);
  return [
    state?.settings?.profiles?.activeProfileId,
    profile?.gameId,
    profile?.modState,
    state?.persistent?.mods?.[profile?.gameId],
    state?.settings?.downloads?.path,
  ];
}

/**
 * Writes the snapshot if its sources differ from `previous`, the sources
 * of the last one written. Returns the sources to compare with next time.
 */
function writeStateIfChanged(dir, state, previous, now) {
  const sources = snapshotSources(state);
  if (previous !== undefined && sources.every((source, index) => source === previous[index])) {
    return previous;
  }
  const value = snapshot(state);
  if (value !== undefined) {
    const folder = path.join(dir, "state");
    fs.mkdirSync(folder, { recursive: true });
    writeJson(path.join(folder, `${value.game}.json`), { ...value, writtenAt: now });
  }
  return sources;
}

/** Writes JSON so that a reader never sees half a file. */
function writeJson(file, value) {
  const temporary = `${file}.tmp`;
  fs.writeFileSync(temporary, JSON.stringify(value));
  fs.renameSync(temporary, file);
}

/** Looks at the request folder once and answers what is there. */
function processRequests(api, dir, now, log) {
  const requests = path.join(dir, "requests");
  const results = path.join(dir, "results");
  for (const name of fs.readdirSync(requests)) {
    if (!REQUEST_FILE.test(name)) {
      continue;
    }
    const file = path.join(requests, name);
    const id = name.slice(0, -".json".length);
    let result;
    try {
      const request = JSON.parse(fs.readFileSync(file, "utf8"));
      result =
        request?.id === id
          ? handle(api, request, now, log)
          : { id, ok: false, code: "unsupported", detail: "id does not match file name" };
    } catch (error) {
      result = { id, ok: false, code: "failed", detail: String(error?.message ?? error) };
    }
    // The request is removed first: whatever happens next, it is not run
    // a second time.
    fs.rmSync(file, { force: true });
    writeJson(path.join(results, name), result);
  }
}

/** Says "I am running" for Mod Mallet to read. */
function writeStatus(dir, startedAt, now) {
  writeJson(path.join(dir, "status.json"), {
    protocol: PROTOCOL,
    extensionVersion: VERSION,
    startedAt,
    heartbeatAt: now,
  });
}

/**
 * Starts answering requests. Returns a function that stops it.
 *
 * Nothing here may throw into Vortex: a problem with the shared folder is
 * logged and tried again on the next tick.
 */
function start(api, dir, log) {
  const startedAt = Date.now();
  let written;
  const guarded = (what, work) => {
    try {
      work();
    } catch (error) {
      log("warn", `Mod Mallet link: ${what} failed`, String(error?.message ?? error));
    }
  };

  guarded("creating the shared folder", () => {
    fs.mkdirSync(path.join(dir, "requests"), { recursive: true });
    fs.mkdirSync(path.join(dir, "results"), { recursive: true });
    // Created from here, by Vortex, so that it is certainly the folder
    // Vortex looks into. A folder created by Mod Mallet can end up
    // somewhere else when Mod Mallet runs inside a packaged app, which
    // Windows gives a private copy of AppData.
    const patches = patchesDir(process.env);
    if (patches !== undefined) {
      fs.mkdirSync(patches, { recursive: true });
    }
    writeStatus(dir, startedAt, startedAt);
  });
  log("info", "started", { version: VERSION });
  const writeState = () =>
    guarded("writing the state", () => {
      written = writeStateIfChanged(dir, api.getState(), written, Date.now());
    });
  writeState();

  const heartbeat = setInterval(
    () => guarded("writing the status", () => writeStatus(dir, startedAt, Date.now())),
    HEARTBEAT_MS,
  );
  const poll = setInterval(() => {
    guarded("reading requests", () => processRequests(api, dir, Date.now(), log));
    writeState();
  }, POLL_MS);
  return () => {
    clearInterval(heartbeat);
    clearInterval(poll);
    // Whatever changed in the last second is still written.
    writeState();
    guarded("removing the status", () => fs.rmSync(path.join(dir, "status.json"), { force: true }));
  };
}

/** Entry point called by Vortex. */
function main(context) {
  context.once(() => {
    const dir = bridgeDir(process.env);
    if (dir === undefined) {
      return;
    }
    const log = (level, message, detail) => {
      context.api.log?.(level, `Mod Mallet link: ${message}`, detail);
      appendLog(dir, level, message, detail);
    };
    const stop = start(context.api, dir, log);
    // Without this, status.json would claim for a few seconds that Vortex
    // is still running after it was closed.
    globalThis.addEventListener?.("beforeunload", stop);
  });
  return true;
}

module.exports = {
  default: main,
  // For the tests.
  PROTOCOL,
  VERSION,
  appendLog,
  bridgeDir,
  plan,
  withTicked,
  patchesDir,
  snapshot,
  writeStateIfChanged,
  handle,
  processRequests,
  writeStatus,
  start,
};
