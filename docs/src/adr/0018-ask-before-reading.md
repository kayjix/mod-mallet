# 0018 The app says where it looks, and asks first

- Status: accepted
- Date: 2026-11-15

## Context

The first version of the desktop app found the game by itself and showed
findings as soon as "Scan" was pressed. To the person in front of it that
raised fair questions: how does it know where my game is, did it search my
whole computer, and why was I never asked? A tool that reads someone's
files owes them the answer before it reads, not after.

Mod Mallet in fact never searches a disk. It learns the game directory from
the entry the game's launcher writes into the Windows registry, and derives
the other places from conventions: the script extender's log in the
documents folder, the list of active plugins in the local application data,
the mod manager's deployment list inside the game directory. But nothing
told the user so.

## Decision

1. **Mod Mallet never searches the disk.** It reads a small, fixed set of
   places, each of which is either named by the user or found by a rule
   that can be stated in one sentence. Below the game directory it reads the
   data directory.
2. **Every report lists the places that were read**, how each was found
   (named by the user, from the registry, at its standard place), and
   whether something was there. The command line prints the list with
   `--verbose`; the desktop app shows it with every report.
3. **On its first start, the desktop app asks before it reads.** It shows
   the places it would use and where each suggestion comes from. The user
   confirms them or picks others. Nothing is read before that.
4. **The confirmed places are settings**, shown and changeable in the app at
   any time. Settings are kept in their own human-readable file. They are
   not part of the cache (ADR 0006), which may be deleted at will.
5. **A place that cannot be found is asked for**, with a sentence saying
   what is needed and why. The app does not guess and does not scan drives
   for it.

All five points are implemented. The settings are a JSON file,
`settings.json`, in Mod Mallet's folder below the user's application
settings (`%APPDATA%mod-mallet` on Windows). It holds whether the user
has agreed, and the places the user named; a place that is not named there
is found by its rule every time. Paths are typed or pasted into the
screen; a button that opens a folder dialog is still missing.

## Consequences

- A first-time user sees what will happen before it happens.
- Unusual installations (several copies of the game, a portable mod
  manager) are handled by the same dialog that every user sees once,
  instead of by special cases.
- The app needs a settings file and a first-run screen before it can be
  given to other people.
- The scan report contains paths of the user's machine. An export meant for
  sharing has to shorten them (ADR 0014).
