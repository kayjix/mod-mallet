# The link to Vortex

Mod Mallet asks Vortex to act through a Vortex extension, the *Mod Mallet
link* (`integrations/vortex-extension`). The same extension tells Mod
Mallet which mods are installed right now. The two exchange files in one
folder. This page describes those files. The reasons are in
[ADR 0020](../adr/0020-actions-through-the-mod-manager.md) and
[ADR 0021](../adr/0021-state-from-the-link.md).

## Installing the extension

```text
mallet vortex install-extension
```

or the button the desktop app offers when an action needs the extension.
The extension is copied to `%APPDATA%\Vortex\plugins\mod-mallet-link`.
Vortex loads it the next time it starts. To remove it, delete that folder
or remove the extension on Vortex's Extensions page.

`mallet vortex status` says whether the extension is installed and whether
Vortex is answering.

## The folder

`%LOCALAPPDATA%\mod-mallet\vortex-bridge`

| File | Written by | Content |
|---|---|---|
| `status.json` | the extension, every five seconds while Vortex runs | that the extension is alive |
| `state\<game>.json` | the extension, at start and within a second of a change | the installed mods of the game |
| `requests\<id>.json` | Mod Mallet | what Vortex is asked to do |
| `results\<id>.json` | the extension | what became of the request |
| `extension.log` | the extension | one line per start, request and install, for looking up what happened |

Every file is written under a temporary name and then renamed, so a reader
never sees half a file. `<id>` consists of hexadecimal digits and `-`.

## status.json

```json
{ "protocol": 1, "extensionVersion": "0.5.0", "startedAt": 1794700000000, "heartbeatAt": 1794700005000 }
```

Times are milliseconds since 1970-01-01 UTC. Mod Mallet takes the extension
to be running if `heartbeatAt` is less than fifteen seconds old.

## The state of a game

`state\<game>.json` holds the parts of Vortex's state that a scan reads,
for the game Vortex is managing, in the shape Vortex's own state has:

```json
{
  "protocol": 1,
  "game": "skyrimse",
  "writtenAt": 1794700004000,
  "settings": {
    "downloads": { "path": "{USERDATA}\\downloads" },
    "profiles": { "activeProfileId": "p1", "lastActiveProfile": { "skyrimse": "p1" } }
  },
  "persistent": {
    "profiles": { "p1": { "gameId": "skyrimse", "modState": { "Patch Hub 1.0": { "enabled": true } } } },
    "mods": {
      "skyrimse": {
        "Patch Hub 1.0": {
          "archiveId": "a1",
          "installationPath": "Patch Hub 1.0",
          "attributes": {
            "logicalFileName": "Patch Hub",
            "installerChoices": { "type": "fomod", "options": [] }
          }
        }
      }
    },
    "downloads": { "files": { "a1": { "localPath": "Patch Hub 1.0.7z" } } }
  }
}
```

Of a mod, the file holds its folder, its archive, its names, what was
chosen in its installer, its version and the newest version Vortex knows
of, its author, and where it came from (source, game, mod number, page); of a profile, which mods are enabled; of an
archive, its file name. Nothing else of Vortex's state is written: no
account, no other game, no other profile.

A scan reads this file instead of the backup Vortex writes once an hour
(`%APPDATA%\Vortex\temp\state_backups_full\hourly.json`) if it is the
newer of the two. The file stays when Vortex is closed.

## A request

```json
{
  "protocol": 1,
  "id": "18a1f3c2e4-2f4c",
  "action": "reinstall-mod",
  "game": "skyrimse",
  "modId": "Patch Hub 1.0",
  "createdAt": 1794700004000
}
```

| Field | Meaning |
|---|---|
| `protocol` | Version of this format. The extension refuses other versions. |
| `id` | The name of the file without `.json`. |
| `action` | `reinstall-mod`, `install-archive`, `load-after` or `set-mod-enabled`. |
| `game` | Vortex's identifier of the game. The request is refused if Vortex is managing another game. |
| `modId` | For `reinstall-mod` and `set-mod-enabled`: the mod's folder in Vortex's staging directory, or Vortex's own id of the mod. |
| `plugin`, `after` | For `load-after`: two plugin file names (`.esp`, `.esm`, `.esl`). |
| `enabled` | For `set-mod-enabled`: `true` or `false`. |
| `path` | For `install-archive`: a zip archive in `%LOCALAPPDATA%\mod-mallet\patches`. Archives anywhere else are refused (`not-allowed`). |
| `createdAt` | When the request was written. A request older than a minute is refused. |

The extension looks for requests once a second, removes a request before
answering it, and never carries one out twice. Mod Mallet waits ten seconds
for the result and withdraws the request if none arrives.

### reinstall-mod

The extension emits Vortex's `start-install-download` event for the mod's
archive, with the choices saved from the last installation and without
changing whether the mod is enabled. This is what the *Reinstall* entry of
Vortex's mod list does. Vortex then shows the mod's installer.

Vortex takes the mod's old files out of the game before it installs. When
the installer has finished and the mod was enabled, the extension therefore
emits `mods-enabled` for it, again as Vortex's own entry does. Vortex then
deploys, or tells the user that a deployment is due, depending on its
settings. If the user cancels the installer, nothing further happens.

### install-archive

The extension emits Vortex's `start-install` event for the archive, as if
it had been dropped onto Vortex, and answers `installing`. Vortex installs
it as a mod named after the archive.

The extension creates the `patches` folder itself when it starts. Windows
gives a packaged app (one from the Microsoft Store, for example) a private
copy of `AppData`: a folder that such an app creates there is invisible
to every other program. Mod Mallet started from inside such an app would
otherwise write its patch where Vortex cannot see it, and the extension
would answer `not-allowed` with `missing: …`.

### load-after

The extension adds a rule to Vortex's plugin list, as the user would there:
`plugin` loads after `after` (Vortex's `ADD_USERLIST_RULE`). A rule that
exists already is not added twice. Then it asks Vortex to sort the plugins
(`autosort-plugins`) and answers `applied`. The rule is visible in Vortex's
plugin list and can be removed there.

### set-mod-enabled

The extension switches the mod on or off in the profile in use, as the
switch in Vortex's mod list does (`SET_MOD_ENABLED`, then `mods-enabled`),
and answers `applied`. Vortex then deploys, or asks to.

## A result

```json
{ "id": "18a1f3c2e4-2f4c", "ok": true, "code": "started" }
```

| `code` | Meaning |
|---|---|
| `started` | Vortex has begun; the user continues in Vortex. |
| `unknown-mod` | Vortex has no mod of that id for the game. |
| `archive-missing` | The archive the mod was installed from is gone. |
| `wrong-game` | Vortex is managing another game; `detail` names it. |
| `unsupported` | Another protocol version or an unknown action. |
| `expired` | The request was older than a minute. |
| `installing` | Vortex was handed the archive (`install-archive`). |
| `applied` | Vortex made the change (`load-after`, `set-mod-enabled`). |
| `not-allowed` | The archive is not a zip in the patches folder, or not there; `detail` says which. |
| `failed` | Something else; `detail` says what. |

Mod Mallet puts these into words for the user in their language. The
extension's own texts are only for Vortex's notification area.
