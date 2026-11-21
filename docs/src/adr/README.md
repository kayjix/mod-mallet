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
write a new ADR and mark the old one `superseded by NNNN`.

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
| [0010](0010-games.md) | Games: the Bethesda family first | accepted |
| [0011](0011-no-own-renderer.md) | No renderer of our own | accepted |
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
| [0022](0022-the-report-is-a-task-list.md) | The desktop app shows a report as a task list | accepted |
| [0023](0023-placed-objects-by-their-bounds.md) | Placed objects are compared by the boxes around their models | accepted |
| [0024](0024-views-map-and-live-scans.md) | The desktop app has views, a map, and scans again by itself | accepted |
| [0025](0025-patches-through-base-object-swapper.md) | Patches hide objects through Base Object Swapper | accepted |
| [0026](0026-toolbox-log-and-restarting-vortex.md) | The app has a toolbox, a log, and can restart Vortex | accepted |
| [0027](0027-findings-offer-decisions.md) | Findings offer decisions; the window has a home and themes | accepted |
| [0028](0028-pictures-of-textures.md) | Findings show the textures they are about | accepted |
| [0029](0029-duels-and-a-3d-view-of-single-things.md) | Mods are decided pair by pair; single things are shown in 3D | accepted |
| [0030](0030-rules-from-the-loot-masterlist.md) | Rules from the LOOT masterlist, evaluated by libloot | accepted |
| [0031](0031-places-on-the-map-and-a-3d-relief.md) | The map shows the places of the game and of mods, also as a 3D relief | accepted |
| [0032](0032-objects-of-the-game-that-mods-move.md) | Objects of the game that mods move count as the mod's | accepted |
| [0033](0033-comparison-stage-and-ranking.md) | Mods are compared on a stage and decided by a ranking | accepted |
| [0034](0034-installer-options-ticked-in-advance.md) | Missing installer options are ticked in advance | accepted |
| [0035](0035-loots-messages-are-quoted.md) | LOOT's messages are quoted | accepted |
| [0036](0036-one-adapter-for-the-bethesda-family.md) | One adapter for the Bethesda family, settings per game | accepted |
| [0037](0037-the-map-draws-the-games-own-lod.md) | The map draws the game's own level of detail | accepted |
| [0038](0038-a-place-up-close.md) | A place up close: the objects themselves around a point | accepted |
| [0039](0039-collisions-checked-with-the-models.md) | Collisions are checked with the models, and touches count | accepted |
| [0040](0040-collisions-as-a-worklist.md) | Collisions as a worklist, decided spot by spot | accepted |
| [0041](0041-the-games-own-ground-up-close.md) | The game's own ground up close | accepted |
| [0042](0042-interiors-in-3d.md) | Interiors in 3D | accepted |
| [0043](0043-suggestions-about-whole-mods.md) | Suggestions about whole mods | accepted |
| [0044](0044-crash-reports-point-at-suspects.md) | Crash reports point at suspects, with honest confidence | accepted |
| [0045](0045-a-health-score-and-tasks-by-what-to-do.md) | A health score, and tasks by what to do | accepted |
| [0046](0046-why-the-game-relies-on-an-object.md) | Why the game relies on an object, and when it may be hidden all the same | accepted |
| [0047](0047-the-ground-painted-as-the-game-paints-it.md) | The ground painted as the game paints it | accepted |
| [0048](0048-one-job-per-page.md) | One job per page, and a home for each thing | accepted |
| [0049](0049-the-files-every-mod-brings.md) | The files every mod brings, from the mod manager's staging folder | accepted |
| [0050](0050-a-guided-review-through-the-homes.md) | A guided review through the homes, collisions structure by structure | accepted |
| [0051](0051-paths-over-hidden-objects.md) | Paths over hidden objects are found before and after hiding | accepted |
| [0052](0052-paths-over-hidden-objects-are-cut-out.md) | Paths over hidden objects are cut out of the navigation meshes | accepted |
| [0053](0053-shareable-patches-in-parts.md) | Shareable patches in parts, with an installer | accepted |
| [0054](0054-a-catalogue-of-the-games-things.md) | A catalogue of the game's things, and asset collisions beside map collisions | accepted |
| [0055](0055-a-magic-wand-for-what-belongs-together.md) | A magic wand for what belongs together, and one column per mod | accepted |
| [0056](0056-missing-textures-found-in-switched-off-mods.md) | Missing textures are looked for in switched-off mods | accepted |
| [0057](0057-one-version-for-the-app.md) | One version for the app, shown with its build | accepted |
| [0063](0063-the-tools-vortex-knows.md) | The tools Vortex knows, and Mod Mallet as one of them | accepted |
| [0065](0065-oblivion-remastered.md) | Oblivion Remastered: the old game's data below the installation, Unreal mods beside it | accepted |
