# 0091 A report to share, without who and where the user is

- Status: accepted
- Date: 2026-11-26

## Context

ADR 0014 and ADR 0018 promise an explicit export of a report for sharing,
in a forum post or a bug report, that shortens the home directory in its
paths. Nothing did so: whoever wanted help copied paths such as
`C:\Users\<their name>\Documents\My Games\...` along with the findings, or
left out what the helper needed.

A scan report holds paths in many places: the places read, the arguments
of findings (a crash log's name, a file in a log), the mod manager's
folders. A list of fields to clean would miss the next field someone adds.
And a path to the same folder is spelled many ways: either slash, any case,
backslashes doubled where a log quotes JSON, `ALICE~1` instead of the
long name.

## Decision

1. **A shared report is the same report with its texts rewritten.** The
   engine turns the report into JSON, rewrites every text at any depth (keys
   included) and reads it back, so every field, present and future, is
   covered. Fields that are data rather than words, the heights and water of
   the terrain, are left alone (`DATA_FIELDS` in
   `crates/mallet-engine/src/share.rs`); a rewrite could only break them.
2. **What counts as private**, and is replaced:
   - the folders of this machine, wherever a text names them in either
     slash and any case: the game directory (`<game>`), the mod manager's
     folder of installed mods (`<staging>`), a Mod Organizer 2 instance
     (`<instance>`), the downloads (`<downloads>`), the documents folder
     (`<documents>`), `%LOCALAPPDATA%`, `%APPDATA%` and `%USERPROFILE%`. The
     longest folder wins, so a game below the home directory is `<game>`;
   - the user name and the computer's name where they are a folder of a
     path (`E:\Mods\<user>\...`, `\\<machine>\share`), not where they are a
     word of a sentence or a mod's name;
   - the folder below `C:\Users` or `/home`, whoever it belongs to and
     however it is spelled (`<user>`), except the shared ones (`Public`,
     `Default`, `All Users`);
   - the profile folder of a mod manager, Vortex's random profile id below
     `Vortex\<game>\profiles` and a Mod Organizer 2 profile's name below
     `<instance>\profiles` (`<profile>`): an id that links one report to
     another, and a name the user chose.
3. **What stays**: mod names, plugin names, versions, the load order, the
   findings and the paths inside the game directory. They are what a
   helper needs, and they are the same on every machine. A mod named after
   its user keeps its name.
4. **Shortening is a pure function** in `mallet-core`
   (`Shortener`), given the private folders and names. The engine collects
   them from the report's places and the system's known folders and
   environment; tests give their own, so the result is the same on every
   machine. It never panics, which a property test checks.
5. **One text for both shells.** `mallet-messages` renders the shared
   report as Markdown, which also reads well as plain text: the setup, a
   legend of the placeholders used, the places read, the errors and
   warnings in full, the notes in one line each, and the load order. The
   command line prints it with `mallet report --share` (and `mallet scan
   --share`); `--format json` prints the shortened report itself, valid
   against the scan report's schema. The desktop app has a "Share report"
   panel on the home page that copies the text to the clipboard, offers to
   save it as `.md` or `.json`, and says in one sentence what was removed.
6. **Saving writes only into Mod Mallet's own folder**,
   `%LOCALAPPDATA%\mod-mallet\shared\mod-mallet-report-<game>.md` (or
   `.json`), and shows the file in the file explorer; there is no save
   dialog, which would need a further dependency.

## Consequences

- Sharing is one button, and the user is told what was taken out before
  pasting it anywhere.
- A user name that is also an ordinary folder's name (`Data`, `Skyrim`) is
  replaced there too. That hides a little of a path from the helper; it
  never leaks.
- A known folder followed by a space and more words is taken to end at the
  space unless the path goes on with a separator: `<game> is missing` in a
  sentence, but `...\Skyrim Special Edition\Data` when the game is in
  `...\Skyrim`.
- The shared JSON keeps the terrain and is as large as the report. A helper
  rarely needs it; if forum limits bite, the share could leave the map out.

## Alternatives considered

- **Cleaning named fields only** (`locations[].path`, some finding
  arguments). Smaller, but every new field with a path would leak until
  someone remembered it.
- **Replacing the user name everywhere.** It would also change mod names
  and words of sentences (a user called `Bob` and a mod called `Bobs
  Armour`), which a helper needs to read as they are.
