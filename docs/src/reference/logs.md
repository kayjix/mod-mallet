# Logs

When something did not work, the logs say what Mod Mallet and the Vortex
extension did. They are plain text, one event per line, with times in UTC,
and meant to be read by people and by coding assistants alike. They hold
paths on the user's machine and names of mods, nothing else about the user;
still, read a log before passing it on.

| File | Written by |
|---|---|
| `%LOCALAPPDATA%\mod-mallet\logs\mod-mallet.log` | the desktop app: start, scans, patches written, requests to Vortex and their answers |
| `%LOCALAPPDATA%\mod-mallet\logs\mod-mallet.previous.log` | the app's log before the last one grew past 2 MB |
| `%LOCALAPPDATA%\mod-mallet\vortex-bridge\extension.log` | the Vortex extension: its start, every request it answered, and how an install ended |
| `%LOCALAPPDATA%\mod-mallet\vortex-bridge\extension.previous.log` | the extension's log before it grew past 1 MB |

The app's settings page has a button that opens the log folder. The
command line writes the same events to standard error with `--verbose`.

A line of the app's log:

```text
2026-11-15T20:22:55.123Z  INFO Vortex answered code=not-allowed detail=missing: C:\…\Mod Mallet Patch.zip
```

A line of the extension's log:

```text
2026-11-15T20:22:55.456Z info request {"id":"1a1e…","action":"install-archive","code":"installing"}
```

To follow a request from one side to the other, look for the same time:
the app writes "asking Vortex to …" just before the extension writes
"request", and "Vortex answered" just after.

## A trap when Mod Mallet is started from another app

Windows gives packaged apps (from the Microsoft Store, or installed as
MSIX, like some desktop apps for coding assistants) a private copy of
`AppData`. A program started from inside such an app inherits it: folders
it creates under `AppData` land in
`%LOCALAPPDATA%\Packages\<app>\LocalCache\` and are invisible to Vortex
and to the file explorer. Start Mod Mallet from the start menu or the file
explorer instead; `cargo xtask app` starts it that way on Windows.
