# 0054 A catalogue of the game's things, and asset collisions beside map collisions

- Status: accepted
- Date: 2026-11-19
- Changes: ADR 0048 (one job per page; the sidebar), ADR 0049 (what is
  read in the staging folder)

## Context

Mod Mallet showed things only where two mods disagree about them: a duel
about files on *Assets*, a collision on the map. Once a duel was decided,
the thing it was about was not seen again, and with a hundred duels the
user lost track of which sword, helmet or face had been chosen. There was
no way to look up a thing that is not in a duel, such as the steel sword
or the armour of Whiterun's guards, and see what the game and each mod
make of it.

The user asked for a list of every asset, sorted into characters, things
to carry and other assets, with the versions the installed mods bring of
each shown side by side, and for sets such as a guard's helmet, armour,
boots and sword shown together. They want to filter it like the map: only
the game's things, or those of one mod. The game already sorts its things
by record type, and its records say much of the rest: the kind of weapon,
the body slots armour covers, a material keyword, and what a group of
characters wears (an outfit).

The user also found the navigation misleading: *Collisions* is about
placed objects only, while the duels about files on *Assets* are
collisions too.

## Decision

1. **A catalogue, read on demand.** The engine reads the top-level groups
   of weapons, armour and armour addons, ammunition, characters, outfits,
   leveled items, keywords, races and the other things with a model of
   their own (books, potions, ingredients, keys, soul gems, other things to
   carry, furniture, containers, doors, activators, plants, lights) from
   every active plugin, in load order. The version the game uses wins, and
   the plugins that change a thing are listed with it. Names of localized
   plugins come from their string tables, in the window's language where
   the setup has it, else in English. The catalogue is not part of a scan:
   it is read when the *Assets* page opens and kept while the setup does
   not change. It is a fact of the setup only; it never finds anything.
2. **What a thing is, from its own records.** Weapons are sorted by the
   game's weapon-type keywords, else by how they are held; worn things by
   the body slots they cover (head, body, hands, feet, shield, jewellery);
   the material comes from the keyword (`WeapMaterialSteel`,
   `DLC2ArmorMaterialChitinLight`) without its prefix and weight. Things a
   player cannot use (a creature's claws or skin) are left out. Variants
   made from another thing (the game's enchanted weapons and armour, which
   name the plain one as their template and look the same) are counted
   there, not listed. Characters are listed where there is only one of
   them and they have a face of their own; the many others appear through
   what they wear.
3. **Sets are the game's outfits.** An outfit lists what is worn, following
   leveled lists, where the game picks one of several, to all of them. The
   weapons are those the characters who wear the outfit carry by default,
   the most common first. Sets are not put together by hand. The list can
   be arranged by kind, by material or as sets; things of mods that fit no
   outfit still appear by kind and material.
4. **Versions, from every place a file can come from.** A thing's version
   from a mod is what its files look like in that mod: the files the game
   uses, those in the archives, and the copies other mods replaced, which
   stay in the mod manager's staging folder. For these the engine opens
   files in the staging folder, which ADR 0049 limited to listing names.
   It opens only the meshes and textures of the thing looked at, only to
   show them, and writes nothing. At most four versions stand side by side
   on one stage; the others are picked from a list.
5. **Picking a version is a rule for this thing's files only.** Choosing
   which version the game shows is handed to the mod manager as an action
   that gives the chosen mod's copy of exactly these files precedence (in
   Vortex, a file override), not as a rule between the two mods, which
   would move every file they share. The choices are listed with the thing
   and can be taken back there.
6. **Map collisions and asset collisions.** The sidebar names the two kinds
   of collisions side by side: *Map collisions* (the former *Collisions*,
   placed objects) and *Asset collisions* (the former *Assets*: the duels
   about files and the ranking). *Assets* is the catalogue. Each keeps one
   job: the collisions decide what two mods disagree about, the catalogue
   shows every thing and decides its version.

## Consequences

- On the reference setup (397 plugins, with a large worldspace mod) the
  catalogue holds about 22,800 things (about 4,000 weapons and worn things,
  2,750 characters) and 1,260 outfits, after about 15,000 variants were
  folded into the things they are made from. Reading it takes eight to ten
  seconds the first time the page opens; it is then kept in memory until
  the setup changes. It is not kept on disk; if the wait matters, the
  plugins' contributions can be cached like the map's markers.
- Names follow the plugin whose version the game uses. A mod that changes
  a thing of the game without being localized brings its English name with
  it, as it does in the game.
- Only the Skyrim family has a catalogue; the other games' records are laid
  out differently and get one when they need it.
- Outfits without a body piece are named by their internal name in words,
  which is English.
- A character is shown by its face only. The body, and the outfit worn on
  it, are not put together.
- Reading files in the staging folder means the place's description on the
  first-run page changes from "only the names of their files" to "the files
  of the things you look at".
- A thing decided by a file override and the duel of the same two mods can
  disagree: the override wins for its files. The asset collisions show the
  pair's rule, the catalogue the thing's.

## Alternatives considered

- **Reading the catalogue with every scan.** The scan's report would grow
  by several megabytes and every scan by seconds, for a page that is not
  always opened.
- **Sets put together by hand.** The user rejected it: with thousands of
  things it is work nobody finishes, and the game already says what goes
  together.
- **Sets by material alone.** Material says what belongs to one kind of
  armour, not what a guard wears; it is offered as an arrangement of the
  list instead.
- **A rule between the two mods for a picked version.** It is how duels are
  decided, and it is simple, but picking a sword would move every other
  file the two mods share.
- **A third tab on the old *Assets* page.** It would give that page a
  third way and two jobs, against ADR 0048.
