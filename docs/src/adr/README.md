# Decisions

An architecture decision record (ADR) captures one decision that shapes the
project: what was decided, why, and what follows from it. Someone who joins
later can read why the code is the way it is, and someone who wants to change
it knows which reasons they have to answer.

## When to write one

Write an ADR when a choice is expensive to reverse or will be questioned
again: a new dependency with a different license, a change to a crate
boundary, a new kind of output Mod Mallet writes, a contract other programs
rely on. A bug fix or a new finding does not need one.

## How

1. Copy [the template](template.md) to `docs/src/adr/NNNN-short-title.md`
   with the next free number.
2. Fill it in. Short is good: context, the decision, the consequences.
3. Add it to `docs/src/SUMMARY.md` and to the table below.
4. Open a pull request. The discussion happens there.

An accepted ADR is not edited to say something else. To change a decision,
write a new ADR and mark the old one `superseded by NNNN`. Two kinds of
edit are allowed to an accepted record, because they keep it true without
rewriting what it decided:

- **Its status line.** When a later ADR changes part of a decision, that ADR
  names the earlier one in a `- Changes:` header, and the earlier one's
  status line says so: `accepted; changed by [ADR 0045](…): what changed`.
  The index below shows the same in compact form.
- **A clearly marked amendment.** A small correction or addition that does
  not deserve a record of its own (a reason that turned out wrong, a detail
  found on a real setup) is added as a section or note headed *Amended*
  with its date, and the status line says `amended <date>`. The original
  text stays as it was.

A change that reverses a decision, or that a reader would have to look for
in several places, is a new ADR.

Numbers 0062, 0084 and 0087 were reserved and never
used; no record with these numbers was ever committed.

## Index

| # | Decision | Status |
|---|---|---|
| [0001](0001-record-architecture-decisions.md) | Record architecture decisions | accepted |
| [0002](0002-license.md) | License: GPL-3.0-or-later | accepted |
| [0003](0003-scope-and-non-goals.md) | Scope and non-goals | accepted |
| [0004](0004-pipeline.md) | A pipeline with I/O at the edges | accepted |
| [0005](0005-read-only-and-reversible.md) | Scans read, actions are reversible | accepted |
| [0006](0006-store-is-a-cache.md) | The store is a cache | accepted |
| [0007](0007-findings.md) | Findings: kinds, confidence, identity | accepted |
| [0008](0008-knowledge-sources.md) | Where knowledge comes from | accepted |
| [0009](0009-mod-managers.md) | Mod managers behind one trait | accepted |
| [0010](0010-games.md) | Games: the Bethesda family first | accepted; changed by 0036 |
| [0011](0011-no-own-renderer.md) | No renderer of our own | accepted; changed by 0029, 0031, 0037, 0038 |
| [0012](0012-typed-contracts.md) | Contracts are generated from Rust | accepted |
| [0013](0013-untrusted-input.md) | Mod files are untrusted input | accepted |
| [0014](0014-offline-and-private.md) | Offline and private | accepted |
| [0015](0015-technology-stack.md) | Technology stack | accepted |
| [0016](0016-automation-and-dependencies.md) | Automation and dependencies | accepted |
| [0017](0017-synthetic-test-data.md) | Test data is synthetic | accepted |
| [0018](0018-ask-before-reading.md) | The app says where it looks, and asks first | accepted |
| [0019](0019-archives-through-7zip.md) | Mod archives are read with a 7-Zip that is already there | accepted |
| [0020](0020-actions-through-the-mod-manager.md) | Actions are handed to the mod manager through a link | accepted |
| [0021](0021-state-from-the-link.md) | The link hands over the mod manager's current state | accepted |
| [0022](0022-the-report-is-a-task-list.md) | The desktop app shows a report as a task list | accepted; changed by 0045 |
| [0023](0023-placed-objects-by-their-bounds.md) | Placed objects are compared by the boxes around their models | accepted; changed by 0039, 0040 |
| [0024](0024-views-map-and-live-scans.md) | The desktop app has views, a map, and scans again by itself | accepted |
| [0025](0025-patches-through-base-object-swapper.md) | Patches hide objects through Base Object Swapper | accepted; changed by 0040, 0046, 0051, 0052, 0053, 0060, 0067 |
| [0026](0026-toolbox-log-and-restarting-vortex.md) | The app has a toolbox, a log, and can restart Vortex | accepted; changed by 0048 |
| [0027](0027-findings-offer-decisions.md) | Findings offer decisions; the window has a home and themes | accepted; changed by 0045, 0048; also amended 2026-11-26 |
| [0028](0028-pictures-of-textures.md) | Findings show the textures they are about | accepted |
| [0029](0029-duels-and-a-3d-view-of-single-things.md) | Mods are decided pair by pair; single things are shown in 3D | accepted; changed by 0031, 0033, 0048 |
| [0030](0030-rules-from-the-loot-masterlist.md) | Rules from the LOOT masterlist, evaluated by libloot | accepted |
| [0031](0031-places-on-the-map-and-a-3d-relief.md) | The map shows the places of the game and of mods, also as a 3D relief | accepted; changed by 0037, 0048 |
| [0032](0032-objects-of-the-game-that-mods-move.md) | Objects of the game that mods move count as the mod's | accepted |
| [0033](0033-comparison-stage-and-ranking.md) | Mods are compared on a stage and decided by a ranking | accepted; changed by 0048 |
| [0034](0034-installer-options-ticked-in-advance.md) | Missing installer options are ticked in advance | accepted |
| [0035](0035-loots-messages-are-quoted.md) | LOOT's messages are quoted | accepted |
| [0036](0036-one-adapter-for-the-bethesda-family.md) | One adapter for the Bethesda family, settings per game | accepted; changed by 0065, 0078 |
| [0037](0037-the-map-draws-the-games-own-lod.md) | The map draws the game's own level of detail | accepted; changed by 0074 |
| [0038](0038-a-place-up-close.md) | A place up close: the objects themselves around a point | accepted; changed by 0040, 0041, 0042, 0074, 0075 |
| [0039](0039-collisions-checked-with-the-models.md) | Collisions are checked with the models, and touches count | accepted; changed by 0040, 0067 |
| [0040](0040-collisions-as-a-worklist.md) | Collisions as a worklist, decided spot by spot | accepted; changed by 0046, 0048, 0050, 0055, 0067, 0072 |
| [0041](0041-the-games-own-ground-up-close.md) | The game's own ground up close | accepted; changed by 0047 |
| [0042](0042-interiors-in-3d.md) | Interiors in 3D | accepted |
| [0043](0043-suggestions-about-whole-mods.md) | Suggestions about whole mods | accepted; changed by 0049, 0061; also amended 2026-11-18 |
| [0044](0044-crash-reports-point-at-suspects.md) | Crash reports point at suspects, with honest confidence | accepted |
| [0045](0045-a-health-score-and-tasks-by-what-to-do.md) | A health score, and tasks by what to do | accepted; changed by 0076 |
| [0046](0046-why-the-game-relies-on-an-object.md) | Why the game relies on an object, and when it may be hidden all the same | accepted; amended 2026-11-25 |
| [0047](0047-the-ground-painted-as-the-game-paints-it.md) | The ground painted as the game paints it | accepted |
| [0048](0048-one-job-per-page.md) | One job per page, and a home for each thing | accepted; changed by 0050, 0054 |
| [0049](0049-the-files-every-mod-brings.md) | The files every mod brings, from the mod manager's staging folder | accepted; changed by 0054, 0056 |
| [0050](0050-a-guided-review-through-the-homes.md) | A guided review through the homes, collisions structure by structure | accepted; changed by 0072, 0080, 0083 |
| [0051](0051-paths-over-hidden-objects.md) | Paths over hidden objects are found before and after hiding | accepted; changed by 0052, 0059 |
| [0052](0052-paths-over-hidden-objects-are-cut-out.md) | Paths over hidden objects are cut out of the navigation meshes | accepted; changed by 0058, 0059, 0078 |
| [0053](0053-shareable-patches-in-parts.md) | Shareable patches in parts, with an installer | accepted; changed by 0060, 0070 |
| [0054](0054-a-catalogue-of-the-games-things.md) | A catalogue of the game's things, and asset collisions beside map collisions | accepted |
| [0055](0055-a-magic-wand-for-what-belongs-together.md) | A magic wand for what belongs together, and one column per mod | accepted; amended 2026-11-25 |
| [0056](0056-missing-textures-found-in-switched-off-mods.md) | Missing textures are looked for in switched-off mods | accepted |
| [0057](0057-one-version-for-the-app.md) | One version for the app, shown with its build | accepted |
| [0058](0058-paths-repaired-on-the-ground-below.md) | Paths over hidden objects are repaired on the ground below | accepted; changed by 0070, 0071, 0088 |
| [0059](0059-where-characters-still-go.md) | Where characters still go once objects are hidden | accepted; changed by 0069, 0071 |
| [0060](0060-objects-moved-a-little-through-a-plugin.md) | Objects are moved a little, through a plugin Mod Mallet writes | accepted; changed by 0067, 0069, 0070 |
| [0061](0061-switched-off-mods-and-mods-to-remove.md) | Switched-off mods, and mods worth removing | accepted; changed by 0073, 0076 |
| [0063](0063-the-tools-vortex-knows.md) | The tools Vortex knows, and Mod Mallet as one of them | accepted |
| [0064](0064-mod-organizer-2-as-the-second-manager.md) | Mod Organizer 2 as the second mod manager | accepted |
| [0065](0065-oblivion-remastered.md) | Oblivion Remastered: the old game's data below the installation, Unreal mods beside it | accepted; changed by 0078 |
| [0066](0066-other-files-compared-side-by-side.md) | Other files are compared side by side | accepted |
| [0067](0067-stacked-doors-are-merged-into-one.md) | Stacked doors are found and merged into one | accepted; changed by 0088 |
| [0068](0068-which-plugin-needs-which.md) | The report carries which plugin needs which, drawn as arcs | accepted |
| [0069](0069-places-where-characters-do-something.md) | Places where characters do something can be seen, removed and moved | accepted |
| [0070](0070-several-patches-that-say-what-they-decide.md) | Several patches, each its own mod, that say what they decide | accepted; amended 2026-11-22 |
| [0071](0071-the-paths-mode-of-the-3d-views.md) | The paths mode of the 3D views shows where characters walk | accepted |
| [0072](0072-passages-where-two-mods-build-the-same.md) | Passages: where two mods build the same thing, decided as one | accepted; amended 2026-11-22 |
| [0073](0073-switched-off-mods-taken-out-through-vortex.md) | Switched-off mods still loaded are taken out through Vortex's plugin list | accepted |
| [0074](0074-level-of-detail-makes-room-for-real-objects.md) | The level of detail makes room for the real objects up close | accepted |
| [0075](0075-one-call-per-material-for-a-place-up-close.md) | A place up close is drawn with one call per material | accepted |
| [0076](0076-task-rows-crash-risks-first.md) | Task rows: crash risks first, one thing to do per row | accepted; changed by 0082 |
| [0077](0077-one-ui-kit-for-every-page.md) | One UI kit for every page | accepted; amended 2026-11-25 |
| [0078](0078-other-games-closer-to-skyrim.md) | Other games closer to Skyrim: Enderal, Fallout 4's paths, and saying what does not apply | accepted |
| [0079](0079-version-changes.md) | Version changes are read from the files, and compared with the last scan | accepted |
| [0080](0080-collisions-decided-by-side-with-the-pictures.md) | The collisions step is decided by side, with the pictures | accepted; changed by 0083 |
| [0081](0081-one-source-of-truth-for-decisions.md) | One source of truth for decisions | accepted |
| [0082](0082-task-page-tiles-by-category.md) | The task page: tiles by category over one list | accepted |
| [0083](0083-step-by-step-says-what-is-at-stake.md) | "Step by step" says what is at stake in every step | accepted |
| [0085](0085-pictures-of-meshes-and-a-faster-relief.md) | Pictures of meshes are drawn in the engine; the relief reads its textures from an index | accepted |
| [0086](0086-preferences-learned-from-decisions.md) | Preferences between mods, learned from the decisions | accepted |
| [0088](0088-door-paths-re-attached-to-the-kept-door.md) | The paths of merged doors are re-attached to the kept door | accepted |
| [0089](0089-when-the-game-rewrites-the-load-order.md) | When the game rewrites the load order, it is told why | accepted |
| [0090](0090-the-vision-catches-up.md) | The vision draws its lines where Mod Mallet's own work ends | accepted |
| [0091](0091-a-report-to-share.md) | A report to share, without who and where the user is | accepted |
