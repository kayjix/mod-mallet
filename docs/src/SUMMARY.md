# Summary

[Introduction](introduction.md)

# Product

- [Vision and non-goals](product/vision.md)
- [Roadmap](product/roadmap.md)
- [Glossary](product/glossary.md)

# Architecture

- [Overview](architecture/overview.md)
- [Crates and their rules](architecture/crates.md)
- [Adding a finding](architecture/findings.md)
- [Decisions](adr/README.md)
  - [0001 Record architecture decisions](adr/0001-record-architecture-decisions.md)
  - [0002 License: GPL-3.0-or-later](adr/0002-license.md)
  - [0003 Scope and non-goals](adr/0003-scope-and-non-goals.md)
  - [0004 A pipeline with I/O at the edges](adr/0004-pipeline.md)
  - [0005 Scans read, actions are reversible](adr/0005-read-only-and-reversible.md)
  - [0006 The store is a cache](adr/0006-store-is-a-cache.md)
  - [0007 Findings: kinds, confidence, identity](adr/0007-findings.md)
  - [0008 Where knowledge comes from](adr/0008-knowledge-sources.md)
  - [0009 Mod managers behind one trait](adr/0009-mod-managers.md)
  - [0010 Games: the Bethesda family first](adr/0010-games.md)
  - [0011 No renderer of our own](adr/0011-no-own-renderer.md)
  - [0012 Contracts are generated from Rust](adr/0012-typed-contracts.md)
  - [0013 Mod files are untrusted input](adr/0013-untrusted-input.md)
  - [0014 Offline and private](adr/0014-offline-and-private.md)
  - [0015 Technology stack](adr/0015-technology-stack.md)
  - [0016 Automation and dependencies](adr/0016-automation-and-dependencies.md)
  - [0017 Test data is synthetic](adr/0017-synthetic-test-data.md)
  - [0018 The app says where it looks, and asks first](adr/0018-ask-before-reading.md)
  - [0019 Mod archives are read with a 7-Zip that is already there](adr/0019-archives-through-7zip.md)
  - [0020 Actions are handed to the mod manager through a link](adr/0020-actions-through-the-mod-manager.md)
  - [0021 The link hands over the mod manager's current state](adr/0021-state-from-the-link.md)
  - [0022 The desktop app shows a report as a task list](adr/0022-the-report-is-a-task-list.md)
  - [0023 Placed objects are compared by the boxes around their models](adr/0023-placed-objects-by-their-bounds.md)
  - [0024 The desktop app has views, a map, and scans again by itself](adr/0024-views-map-and-live-scans.md)
  - [0025 Patches hide objects through Base Object Swapper](adr/0025-patches-through-base-object-swapper.md)
  - [0026 The app has a toolbox, a log, and can restart Vortex](adr/0026-toolbox-log-and-restarting-vortex.md)
  - [0027 Findings offer decisions; the window has a home and themes](adr/0027-findings-offer-decisions.md)
  - [0028 Findings show the textures they are about](adr/0028-pictures-of-textures.md)
  - [0029 Mods are decided pair by pair; single things are shown in 3D](adr/0029-duels-and-a-3d-view-of-single-things.md)
  - [0030 Rules from the LOOT masterlist, evaluated by libloot](adr/0030-rules-from-the-loot-masterlist.md)
  - [0031 The map shows the places of the game and of mods, also as a 3D relief](adr/0031-places-on-the-map-and-a-3d-relief.md)
  - [0032 Objects of the game that mods move count as the mod's](adr/0032-objects-of-the-game-that-mods-move.md)
  - [0033 Mods are compared on a stage and decided by a ranking](adr/0033-comparison-stage-and-ranking.md)
  - [0034 Missing installer options are ticked in advance](adr/0034-installer-options-ticked-in-advance.md)
  - [0035 LOOT's messages are quoted](adr/0035-loots-messages-are-quoted.md)
  - [0036 One adapter for the Bethesda family, settings per game](adr/0036-one-adapter-for-the-bethesda-family.md)
  - [0037 The map draws the game's own level of detail](adr/0037-the-map-draws-the-games-own-lod.md)
  - [0038 A place up close: the objects themselves around a point](adr/0038-a-place-up-close.md)
  - [Template](adr/template.md)

# Reference

- [Finding catalog](reference/findings.md)
- [Command line](reference/cli.md)
- [Data contracts](reference/contracts.md)
- [The link to Vortex](reference/vortex-link.md)
- [Logs](reference/logs.md)

# Contributing

- [Development setup](contributing/setup.md)
- [Conventions](contributing/conventions.md)
- [Dependencies](contributing/dependencies.md)
