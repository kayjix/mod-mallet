# 0049 The files every mod brings, from the mod manager's staging folder

- Status: accepted
- Date: 2026-11-18
- Changes: ADR 0043 (suggestions about whole mods)

## Context

The suggestion that a mod hardly reaches the game (ADR 0043) could not see
the most common case. When two mods bring the same loose file, Vortex
deploys only the winner's copy into the game folder; the other copy stays
in Vortex's staging folder, where Vortex keeps every installed mod in a
folder of its own. Mod Mallet only asked the mod manager who deployed a
file, so a texture pack replaced almost entirely by a newer one looked like
a small mod whose few files all reach the game.

The staging folder is named in Vortex's deployment manifest
(`stagingPath`), which Mod Mallet already reads. The user agreed that Mod
Mallet may read the mods' file lists there, with the general consent given
on the first-run page (ADR 0018).

## Decision

1. **A new question to the mod manager** (ADR 0009):
   `ModSource::staged_files(mod)`, every file the manager keeps for a mod,
   deployed or not, by the path the game would see it under. Managers that
   cannot say answer `None`, which is the default.
2. **Vortex answers it from the staging folder.** For each mod that has
   files in the deployment, the engine lists the mod's folder. Only folder
   listings are read; no file in the staging folder is opened. The folder
   is the one the user named, or else the one the deployment manifest
   names.
3. **The staging folder is a place like the others** (ADR 0018). It is
   listed on the first-run page as "Vortex's folder of installed mods
   (only the names of their files)", found as "named in Vortex's list of
   deployed files" (a new origin, `manager`). The manifest is not opened
   before the user agrees, so the page shows the place without a path;
   reports show the path that was read. The user can name another folder
   (`--staging-dir` on the command line).
4. **Listings are cached per mod**, under the modification time of the
   mod's folder. Vortex creates a mod's folder anew when it installs the
   mod again, which changes that time.
5. **Mostly overridden counts every file a mod brings.** A file counts as
   reaching the game if the manager deployed this mod's copy, and as
   replaced by the mod whose copy it deployed otherwise. A file no mod
   deployed is not counted: the user may have told the manager to leave it
   out, or the deployment is older than the folder.

## Consequences

- On the reference setup (164 deployed mods, 41,043 files in their
  folders) the listing takes 0.56 s the first time and 0.02 s from the
  cache.
- On that setup still no mod is mostly overridden. The lowest shares are
  MfgFix, 2 of 6 files of which reach the game because MfgFix NG replaces
  the rest (fewer than the 20 parts a suggestion needs), and SMIM at 80 %.
- A file changed by hand deep inside a mod's folder, without the folder
  itself changing, is seen only once the cache is cleared or the mod is
  installed again.
- Other mod managers keep their mods in folders as well (Mod Organizer 2);
  the question is theirs to answer the same way.
