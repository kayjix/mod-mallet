# 0043 Suggestions about whole mods

- Status: accepted, amended by [0049](0049-the-files-every-mod-brings.md)
- Date: 2026-11-18

## Context

Every finding so far is about a pair of files, plugins or objects: these
two houses stand in each other, this loose file replaces that archive's.
On a large setup that is hundreds of findings, and the user loses sight of
the bigger picture. Two patterns stand out only at the level of a whole
mod.

- **A mod collides with many places of other mods.** On the reference
  setup, Immersive Citizens - AI Overhaul places objects into JK's Skyrim
  in 29 places, JK's Sky Haven Temple into JK's Skyrim in 698, and JK's
  Markarth Outskirts into Lux Via in 258. The collision worklist (ADR 0040)
  lists each place, but the decision the user actually faces is about the
  mod: keep it and decide place by place, let it lose everywhere, or
  remove it.
- **A mod hardly anything of which reaches the game**, because mods loaded
  later replace its records or files. It costs load time and attention and
  gives nothing back.
- **A mod places largely the same objects as another**, at the same
  places: a collection and a mod it contains, or two mods that rebuild a
  place the same way. One of the two is enough; a patch made for only one
  of them is a reason to keep that one.

None is a fault. All are suggestions the user may ignore.

## Decision

1. **Three kinds of finding, `mods.collides-widely`,
   `mods.mostly-overridden` and `mods.duplicates-another`.** Their subject
   is the mod. They are of the
   lowest severity (`info`) and confidence (`hint`): a suggestion, which
   their texts say in their first words. A severity of its own for
   suggestions was not added: severity says how bad something is, and
   `info` already means "worth knowing, nothing to fix". The kinds' area
   `mods.` tells suggestions apart, and the desktop app is to show them apart,
   under "Empfehlungen".
2. **Only mods a mod manager knows are suggested.** The game's own plugins
   and files no manager claims are never candidates, though they are named
   as the other side.
3. **Collides widely.** Every collision spot (ADR 0039, 0040) between
   objects of two different mods counts: duplicates, overlaps and touches,
   as the worklist lists them. Of the two mods of a spot, the one that
   places fewer objects is the candidate, as for which copy of a duplicate
   to hide (ADR 0023): its own content is what the collisions are about,
   while the larger mod merely shares a few places with it. A mod is
   suggested once it is the candidate in at least **20 spots**, about one
   town's worth and more than a patch would usually leave. The finding
   names the other mods with their number of spots, and how many of its
   own placed objects stand in a collision out of all it places.
4. **Mostly overridden.** Mod Mallet counts the parts of a mod it can
   compare, and whether each reaches the game:
   - the objects its plugins place, by whether their plugin's version is
     the one the game uses (objects of the game a mod moves are left out);
   - the characters its plugins define or change, by whether their
     plugin's version is the last;
   - the files in its archives, by whether the game takes them from that
     archive, or from another file of the same mod;
   - its loose files, which reach the game by being there.
   A version from a plugin that names the mod's plugin as a master still
   counts as the mod's: such a plugin was made for the mod and carries its
   changes on. A mod is suggested if at least **20 parts** were compared
   and at most **10 %** of them reach the game. The finding names the mods
   that replace the rest, with how many parts each.
5. **A mod other mods build on is never offered to be switched off.**
   Switching it off would stop the game from starting. Such a mod is not
   suggested as mostly overridden at all; as colliding widely it is
   suggested without the button.
   A mod that duplicates another is offered no button: which of the two
   to keep is the user's choice.
6. **Duplicates another.** Two objects of two mods are twins if they show
   the same model (or, without a model, the same base object) at the same
   place, within 32 units, the distance ADR 0023 uses for duplicates. Unlike
   the duplicates of the collision analysis, a twin may come from another
   base record: a collection often brings its own copies of the base
   objects of the mods it contains. A mod duplicates another if at least
   **20** of its objects and at least **half** of all it places have a twin
   there. If two mods duplicate each other, the one that places fewer
   objects is named. The finding names, for each of the two, the plugins
   of other mods made for it alone (that build on it and not on the
   other): patches only work with the mod they are made for. LOOT's notes
   about either mod are reported by their own kinds. Spots between two
   such mods do not count towards colliding widely; they are suggested
   once, as duplicates.

## Consequences

- On the reference setup of about 400 plugins, nine mods collide widely,
  among them the three cases above, and Cutting Room Floor, Lux Via and No
  Snow Under The Roof against JK's interiors and outskirts.
- On the same setup no mod duplicates another. JK's Sky Haven Temple,
  which the user saw as part of JK's Skyrim, places 1,585 objects: 711 in
  the temple's interior, where JK's Skyrim places nothing, and 874
  outside, of which only 156 have a twin in JK's Skyrim. The 698 spots
  between them are mostly different objects touching: JK's Skyrim brings
  its own version of the area around the temple. Switching the standalone
  off would lose the interior; the suggestion stays "collides widely".
- `plugins.contained` stays: a warning about a plugin most of whose solid
  objects stand twice as the same base object. A mod can get both; the
  warning says what goes wrong, the suggestion which mod to keep.
- On the same setup no mod is mostly overridden: the lowest share that
  reaches the game is 84 % (SkyUI, Unofficial Skyrim Special Edition
  Patch). The check is in place for setups that have such a mod.
- Loose files that the mod manager does not deploy because another mod's
  loose file wins are not seen: only the winning file is in the game
  folder, and the mod manager is only asked who deployed a file. A texture
  pack replaced almost entirely by a later one therefore goes unnoticed.
  Seeing it needs the list of every mod's files from the mod manager's
  staging folder, a further question to the mod manager (ADR 0009); ADR
  0049 adds it.
- Records other than placed objects and characters (items, quests,
  scripts) are not compared. The texts say so, and a mod made mostly of
  those has few compared parts or is judged by the ones that are.

## Alternatives considered

- **A share of colliding objects as the threshold** (say, a mod is
  suggested if 5 % of its objects collide). On the reference setup even the
  clearest cases stay below it: Immersive Citizens has 38 of its 1,384
  objects in collisions, JK's Markarth Outskirts 89 of 3,110. Large mods
  that collide in a few places would have to be weighed the same way, with
  a threshold no one could explain.
- **Suggesting both mods of every pair.** That would suggest removing JK's
  Skyrim because the many smaller mods placed into it collide with it.
