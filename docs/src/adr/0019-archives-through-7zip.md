# 0019 Mod archives are read with a 7-Zip that is already there

- Status: accepted
- Date: 2026-11-15

## Context

To check whether a mod's installer would offer other options today, Mod
Mallet needs one small file out of the archive the mod was installed from.
Mods are distributed as zip, 7z and rar archives, in roughly that order of
frequency reversed: in a real setup most are 7z, a good part rar.

Reading all three with libraries means three decompression dependencies.
The rar library's license is not compatible with the GPL (ADR 0002), so rar
could not be covered that way at all, and the mods that matter most for
this check, large patch collections, are often rar.

Every user of the Vortex mod manager already has a 7-Zip on their machine,
because Vortex ships one. 7-Zip reads all three formats.

## Decision

- Mod Mallet reads archives by starting a 7-Zip program
  (`mallet-unpack`). It asks for the list of files and for single files
  written to standard output. Nothing is unpacked to disk.
- The program is looked for in a fixed list of places: the 7-Zip that
  Vortex ships, an installed 7-Zip, then `PATH`. The user can name another
  one. The place that was used is listed in the report (ADR 0018).
- Without a 7-Zip the checks that need archives do not run, and the report
  says that no unpacker was found. Everything else works.
- What is read from an archive is kept in the cache under the archive's
  size and modification time (ADR 0006). An archive is opened again only
  when it changed. Archives that have to be opened are opened several at a
  time.

## Consequences

- All three archive formats are covered without a decompression library,
  and without a license problem.
- The first scan of a setup takes noticeably longer than later ones: in a
  setup with about forty installers, around fifteen seconds against one.
- Mod Mallet runs a program it did not ship. It only ever runs a program at
  one of the listed places or one the user named, and passes the archive's
  path after `--` so that a file name cannot be taken for an option.
- Tests that need a real archive run only where a 7-Zip is present and are
  skipped elsewhere. The parts that do not need one (reading a listing,
  choosing the file) are tested everywhere.
- A user without Vortex and without 7-Zip gets no installer check until
  they install 7-Zip. The report tells them.

## Alternatives considered

- **Libraries for zip and 7z, nothing for rar.** Would miss a good part of
  the archives, among them the largest patch collections.
- **Asking the mod manager to do it** through an extension. Possible later,
  and the right way for *re-running* an installer, but it needs Vortex to
  be running and an extension to be installed just to read a file.
