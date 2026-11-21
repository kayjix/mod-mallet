# Changelog

What changed in each version of Mod Mallet, newest first. The app shows
its version and build under Settings → About. How versions are counted is
in [Releases](docs/src/contributing/releases.md).

## [Unreleased]

### Added

- The toolbox lists the modding tools Vortex knows for your game (SSEEdit and
  its variants, FNIS, Wrye Bash, BodySlide) with what each is for and whether
  it is still where Vortex expects it. Findings about plugins that need
  cleaning say which of them to use.
- Vortex link 0.8.0: a button in the toolbox adds Mod Mallet to the tools on
  Vortex's dashboard (and takes it off again), so you can start it from
  there. Nothing is added unless you press it. Update the link in the
  toolbox to get it.
- Oblivion Remastered is found and read: its plugins, load order and
  archives, with the checks that need only those. Its Unreal mods are
  listed with the mod each came from. Collisions, assets and the
  catalogue say why they stay empty for this game: it draws with Unreal
  Engine, not with the meshes and textures in its Data folder.
- A Vortex deployment of a game that was moved to another drive is read
  where it lies instead of stopping the scan.
- Where hiding an object takes away the paths characters walk on, Mod
  Mallet now says what they could no longer reach: a door, a bed, a
  forge, a place of the map, or a spot a quest sends them to, and where
  they would only get by a long way round. The collisions page draws the
  way before in grey, the way round in green, and marks in red where no
  way is left.
- A mod you switched off in Vortex that the game still loads is named in
  the tasks, with the files that keep it there and what to do; its
  collisions say so in the worklist.
- What you decided about a mod is kept when you switch it off. While the
  game no longer loads it, those decisions wait: they do not count and do
  not go into the patch, and the patch page says how many wait. They are
  back when you switch the mod on again.
- A mod that collides with very many others is shown where you work: first
  among the recommendations, on the mods page and on the collisions page.
  It says what relies on the mod before it suggests removing it, points to
  the clear suggestions where most of its places are clear, and switches it
  off in Vortex on a click.
- Objects can be moved a little to one side instead of hidden, so both
  mods' buildings stay: arrows, up and down and turning in the object card
  of the 3D views, alone or with the parts the magic wand took, with the
  overlap measured again as you go. Mod Mallet writes the moves into a
  small plugin, Mod Mallet - Moves, which Vortex installs on your click;
  remove it there to put everything back.
- Mod Organizer 2 as a second mod manager. Choose it on the first-run
  page or in Settings → Places; Mod Mallet finds global instances by
  itself and takes the folder of a portable one. It reads the profile in
  use, the mods' folders and the overwrite folder the way the game sees
  them, and never writes into Mod Organizer's files: where it would fix
  something in Vortex, it tells you what to click in Mod Organizer
  instead. On the command line: `--manager mo2`.
- Which plugin needs which: each mod's page shows what its plugins need
  and which plugins of which mods need them, on a strip of the whole load
  order. The tab "Dependencies" on the Mods page draws every active plugin
  and its links, with a search; pick a plugin to see both sides.
- Doors that mods stack on top of each other are found, the game's doors
  included: on the collisions page as the kind "doors", each shown with
  where it leads, whether characters walk through it, and which one to
  keep. "Merge into this one" hides the others and says what else has to
  change so that whatever used them uses the kept door (ADR 0067).

## [0.2.2] - 2026-11-19

### Changed

- Fewer words on the pages: introductions and notes that only repeated
  what the page shows are gone. What still helps, such as how the score
  comes about or how to steer the 3D views, sits behind a small ⓘ beside
  the control it belongs to.

### Fixed

- The bar over a 3D view no longer jumps when the magic wand changes its
  level: it keeps room for the longest text.

## [0.2.1] - 2026-11-19

### Changed

- The hammer and gear also show wherever Mod Mallet waits for something:
  in the scan button in the sidebar, while the catalogue of things reads
  the plugins, while versions, pictures, interiors and objects up close
  load.

## [0.2.0] - 2026-11-19

### Added

- While Mod Mallet scans, the hammer of the logo drives the teeth of a gear
  in, one stroke at a time; the gear moves on by one tooth and its teeth
  grow out again. Large on the first scan, small beside "Scanning…" on
  every later one, and still for people who asked their system for less
  motion.

## [0.1.0] - 2026-11-19

The first counted version: everything built so far. Mod Mallet is in
development; this version is tested on one real setup of Skyrim Special
Edition with Vortex.

### Find

- One list of what is wrong, most severe first, and an overview with a
  health score and the things that lower it most.
- Missing masters, script extender plugins the game switched off, plugins
  LOOT warns about, generated plugins such as a Bashed Patch or DynDOLOD
  that are older than what they were made from.
- Installer options that would be ticked now but were never installed.
- Characters with dark faces, meshes made for another game or for the 2011
  Skyrim, missing textures, also when they sit in a switched-off mod.
- Objects of different mods that collide on the map, and things that
  several mods bring in different versions.
- Paths characters walk on that a later mod undoes, and doors it cuts off.
- Recent crash reports, with the mods they point at and how sure that is.
- Suggestions about whole mods: two that collide widely, one of which
  little is left, the same mod twice.
- The other Bethesda games are found and get the checks their shared file
  formats allow.

### Understand

- The map of a world on the game's own land and level of detail, in 3D,
  with the places of the game and of the mods.
- A place up close, with the objects as the game places them; interiors
  in 3D.
- Collisions one spot at a time: one column per mod and their shared
  choices in the middle, markings for what each mod brings and what a
  patch would hide, and a warning where characters walk.
- A magic wand that picks what belongs together, from the object alone up
  to the whole structure.
- Faces, meshes and textures of every mod side by side; a catalogue of the
  game's things with outfits as sets.
- Going through the setup step by step, and tasks ordered by what to do.

### Fix

- Vortex reinstalls a mod with the options it missed, switches a mod off,
  or puts one version of a thing into the game through its file
  overrides.
- A patch of Mod Mallet's own hides colliding objects. It comes in parts
  with an installer, so it can be shared.
- The toolbox says what the setup needs besides its mods: Vortex and the
  link to it, the script extender, LOOT and others, each with the one thing
  to do about it. Updating the link shows that it landed and asks for
  Vortex's restart.

[Unreleased]: https://github.com/kayjix/mod-mallet/compare/v0.2.2...HEAD
[0.2.2]: https://github.com/kayjix/mod-mallet/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/kayjix/mod-mallet/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/kayjix/mod-mallet/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/kayjix/mod-mallet/releases/tag/v0.1.0
