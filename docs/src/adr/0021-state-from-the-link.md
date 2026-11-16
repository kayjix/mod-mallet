# 0021 The link hands over the mod manager's current state

- Status: accepted
- Date: 2026-11-15

## Context

A scan learns from Vortex which mods are installed and what was chosen in
their installers. It reads that from the backup of its state that Vortex
writes about once an hour, because Vortex's database is not for others to
open (ADR 0009). Restarting Vortex does not renew the backup.

With actions (ADR 0020) the hour became a defect. A user reinstalled a mod
from its finding, ticked the missing patches, scanned again, and was shown
the same finding: the scan had read choices that were an hour old. A tool
that says "still broken" after the fix cannot be trusted with the next
finding either.

## Decision

- The Mod Mallet link, the extension that carries out actions inside
  Vortex, also **writes the parts of Vortex's state that a scan reads**
  into the folder it shares with Mod Mallet, at start and within a second
  of a change. There is one file per game.
- The file has **the shape of Vortex's own state**, reduced to those
  parts, and is read by the code that reads the backup.
- A scan reads **the newer of the two files**, by modification time, and
  the report names the one it read. If the link's file cannot be read, the
  backup is.
- A scan still **only reads**. It does not ask the link for anything; the
  file is there or it is not.

## Consequences

- With the link installed and Vortex running, a scan sees what Vortex
  shows. Without the link nothing changes: the backup is read as before.
- The link's file is older than the backup only if Vortex ran without the
  link in between, and then the backup is the one to believe. That is why
  the newer file wins and not always the link's.
- The file names the user's mods, their archives and installer choices. It
  stays in the user's local application data, next to the request files,
  and holds nothing that the backup does not also hold. Account data,
  other games and everything else in Vortex's state are left out.
- The extension writes the file again whenever the list of mods changes.
  During a large installation that is once a second, for a file well below
  a megabyte.
- A state file named explicitly on the command line is read as it is; the
  link's file is then not looked for.

## Alternatives considered

- **Asking the link during a scan.** Would always be current, but a scan
  would then write a request and wait for Vortex, and would behave
  differently depending on whether Vortex is running.
- **Reading Vortex's database.** Current as well, but it is locked while
  Vortex runs and its format is Vortex's private matter.
- **Telling the user to wait.** Honest, and no way to treat someone who
  just did what the tool asked.
