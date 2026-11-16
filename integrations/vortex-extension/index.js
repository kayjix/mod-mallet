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
const VERSION = "0.4.2";

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
const ACTIONS = ["reinstall-mod", "install-archive"];

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
  const modId = findModId(state, game, request.modId);
  if (modId === undefined) {
    return { code: "unknown-mod" };
  }
  const mod = state.persistent.mods[game][modId];
  const archiveId = mod.archiveId;
  if (!archiveId || state?.persistent?.downloads?.files?.[archiveId] === undefined) {
    return { code: "archive-missing" };
  }

  // Exactly what Vortex's own "Reinstall" does: install the mod's archive
  // again, with the choices made last time as the starting point of the
  // installer dialog, and without changing whether the mod is enabled.
  const choices = mod.attributes?.installerChoices;
  const options =
    choices === undefined ? { allowAutoEnable: false } : { choices, allowAutoEnable: false };
  const attributes = mod.attributes ?? {};
  const modName =
    attributes.customFileName || attributes.logicalFileName || attributes.name || request.modId;
  const wasEnabled = activeProfile(state)?.modState?.[modId]?.enabled === true;
  return { code: "started", install: { archiveId, options, modId, game, wasEnabled }, modName };
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
  if (planned.install !== undefined) {
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
    ok: planned.code === "started" || planned.code === "installing",
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
  patchesDir,
  snapshot,
  writeStateIfChanged,
  handle,
  processRequests,
  writeStatus,
  start,
};
