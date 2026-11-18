# 0044 Crash reports point at suspects, with honest confidence

- Status: accepted
- Date: 2026-11-18

## Context

When the game crashes, people want to know which mod did it. Many setups
run a crash logger, a script extender plugin that writes a report on every
crash. Three are common, and each writes its own shape:

| Logger | Where | File |
|---|---|---|
| Crash Logger SSE AE VR | `<My Games>/<game>/SKSE/` | `crash-YYYY-MM-DD-HH-MM-SS.log` |
| Trainwreck | `<My Games>/<game>/SKSE/Crashlogs/` | `crash-YYYY-MM-DD-HH-MM-SS.log` |
| .NET Script Framework | `<game>/Data/NetScriptFramework/Crash/` | `Crash_YYYY_M_D_H-M-S.txt` |

All three say in which program file and at which offset the game failed,
list the probable call stack, and name game objects found near the crash
(characters, packages, meshes) with the plugin each comes from. Newer
Crash Logger versions and the .NET Script Framework list those objects in
a section of their own; Trainwreck and older versions show them only among
the registers and the stack.

Experienced people read these reports by pattern, and even they are often
wrong. A report shows what was nearby, not what was guilty. Mod Mallet must
not turn a report into a verdict.

## Decision

- **What is read.** The reports of the last 30 days, at most the 20
  newest, from the three folders above. Older crashes say little about the
  setup as it is now, and 20 reports are more than anyone reads by hand.
  Crash Logger and Trainwreck write next to the script extender's log, so
  their folders are taken from wherever that log was found, given or
  standard; a scan only reads them. The parser accepts any text and never
  panics (ADR 0013).
- **What is taken from a report.** The exception, the crash address
  (`module+offset`), the program files of the probable call stack (frames
  that Crash Logger marks as found by scanning the stack, `[S]`, are left
  out), and the relevant objects with name, type, form ID as written,
  plugin and mesh. Where a report has no list of relevant objects, they are
  taken from the registers and the stack, without the `Modified by` lines,
  which list every plugin that ever touched a record.
- **What is reported**, mapped to mods through the mod manager by plugin
  file, DLL path (`Data/SKSE/Plugins/…`) and mesh path:
  - `crashes.recent`, a note: how many crashes, the last one, where.
  - `crashes.in-dll`, a likely warning per DLL a crash happened in. The
    code of that file was running when the game failed.
  - `crashes.mod-involved`, a warning with the lowest confidence, a hint,
    per mod whose records or meshes were relevant objects, or whose DLL is
    among the first frames of the probable call stack.
  Files of Windows, of graphics drivers, of the game and of the script
  extender are never suspects. A crash in the game itself with nothing of
  a mod nearby is counted, not blamed on anyone.
- **Identities** do not depend on the reports: a DLL finding is the DLL's
  path, a mod finding is the mod, the summary is the setup. Setting one
  aside survives new crashes.

## Consequences

- People see at a glance whether crashes are recent and which mods they
  point at, and the texts say how sure that is.
- A mod near busy places is named more often than it deserves. The number
  of reports pointing at it is shown, so a single mention reads as what it
  is.
- Form IDs in reports depend on the load order; they are kept for showing,
  never compared.
- No new dependency: the reports are plain text.

## Alternatives considered

- **One finding per crash address.** Two crashes at the same address are
  usually the same bug, but the address alone names no mod, and the
  finding's identity would have to be built from an address in the game's
  code. Grouping by suspect answers the question people ask: which mod.
- **Reading every report ever written.** Old reports describe setups that
  no longer exist and would keep removed mods under suspicion.
- **Ranking a single culprit per crash.** Tempting, but the reports do not
  carry enough to do it honestly.
