# 0020 Actions are handed to the mod manager through a link

- Status: accepted
- Date: 2026-11-15

## Context

Some findings can be fixed by the mod manager with a few clicks, if only
the user knew which mod and which clicks: a mod whose installer would now
offer a missing patch needs to be reinstalled. Telling the user to go and
do it is the least Mod Mallet can do. Doing it from the finding is better.

Mod Mallet must not do it by editing the mod manager's data or the mod's
files (ADR 0005): the mod manager would not know, and the two would drift
apart. Vortex offers no command line or other outside entry point for
"reinstall this mod". It does have an extension interface, and its own
"Reinstall" menu entry is one call on that interface.

Fixing things by the watering can, such as reinstalling every mod that has
an installer, was considered and rejected: it costs the user a dialog per
mod and risks changing choices that were fine. Findings name the mods that
need it.

## Decision

- A finding lists the **actions** that would address it. An action names
  what to do and to which mod; it carries no instructions for a particular
  mod manager.
- An action runs only because **the user pressed its button** (or typed its
  command). A scan never runs one.
- Actions for Vortex are carried out by a small **Vortex extension** that
  Mod Mallet contains and installs on request
  (`integrations/vortex-extension`). The extension does exactly what
  Vortex's own menu entry does, through the same call, so Vortex stays the
  one that changes the setup.
- Mod Mallet and the extension talk through a **folder** on the user's
  machine: Mod Mallet writes a request file, the extension answers with a
  result file and reports that it is alive with a status file. The format
  is versioned and described in
  [The link to Vortex](../reference/vortex-link.md).
- An action is **started**, not completed, by Mod Mallet. The user finishes
  it in the mod manager, and may cancel it there. The next scan shows
  whether the finding is gone.
- Installing the extension is itself something only the user starts. It
  writes two files into a folder of its own inside Vortex's plugin folder
  and nothing else.

## Consequences

- The first action, reinstalling a mod, works from the finding with one
  click plus the installer dialog in Vortex.
- Vortex has to be running, and restarted once after the extension is
  installed. Mod Mallet says so when it is not.
- A folder was chosen over a network port or a pipe: it needs no
  permission, cannot be reached from another machine, and can be inspected
  with a file manager. The cost is a delay of up to a second.
- Any program running as the same user could write a request. It could
  also start Vortex and click. The extension accepts one kind of request,
  which opens a dialog the user has to complete.
- "The same call" has to be read as "everything the menu entry does".
  The first version emitted the install event only. Vortex takes a mod's
  files out of the game before reinstalling it, and its menu entry
  afterwards reports the mod as enabled, which is what puts the new files
  back. Tried on a real setup, two reinstalled mods stayed out of the game
  until the extension did the same.
- After a reinstall the next scan showed the old state: a scan read the
  copy of Vortex's state that Vortex saves once an hour. The extension now
  hands over the current state
  ([ADR 0021](0021-state-from-the-link.md)).
- The reinstall starts from the choices made last time, as Vortex's own
  entry does. Ticking the missing options in advance would be better; that
  needs knowledge of how Vortex applies saved choices and is left for
  later.
- The extension is plain JavaScript without a build step, so that it can
  be read, tested with Node alone, and copied into Vortex as it is. Its
  logic is a pure function with tests; what only runs inside Vortex is a
  few lines.

## Alternatives considered

- **Instructions only.** Always possible and still shown, but the user has
  to find the mod in a list of hundreds.
- **Editing Vortex's state or files directly.** Fast and wrong: see
  ADR 0005.
- **A local network port or a named pipe.** Immediate, but needs a server
  inside Vortex, may trigger a firewall prompt, and is harder to test.
