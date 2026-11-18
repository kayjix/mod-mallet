# 0048 One job per page, and a home for each thing

- Status: accepted
- Date: 2026-11-18
- Changes: ADR 0027 (the home page), ADR 0026 (the toolbox in the
  sidebar), ADR 0029 and 0033 (where duels and the ranking are shown),
  ADR 0031 (which worldspaces the map lists), ADR 0040 (deciding a pair of
  mods)

## Context

The desktop app grew one view at a time. On the reference setup a
collision could be decided on the stage of a spot, at the head of its group
in the worklist, as a pair on *Mod against mod*, as a duel about placed
objects on the same tab, as a pair on the patch page, from its finding, and
in the comparison of a duel. All of them write the same choices, but the
user could not tell which place was meant. Duels were shown in full on
three pages; the collisions were counted in four ways that did not agree;
scanning and switching the game had two places each; the ranking of mods
could only be reached through a duel. Home was the list of all games and
the state of the game in use at once.

The map listed worldspaces by their editor IDs, with small and child
worldspaces of the game next to Skyrim, and showed the land only around
the cells where mods place things. The toolbox reported LOOT as missing
on a setup where Vortex sorts plugins with LOOT's library, and listed the
Creation Kit without saying what it is still needed for.

The user's wish: each area has one clear job, with at most two ways to do
something there, and nothing is shown twice.

`docs/src/product/journey.md` takes stock of the window and draws the
pages; this record states the rules.

## Decision

1. **Two levels.** *All games* lists the games and is the only place to
   switch games. Every other page except Settings belongs to the game in
   use, starting with its **overview**: the health of ADR 0045, a tile per
   page, the scan statistics and what was read. The window opens on the
   overview of the game in use.
2. **One job per page, at most two ways.** Tasks: what is to do except
   collisions and duels, by step. Collisions: decide every spot, by *Spots* (one at
   a time, in 3D) or by *Mod against mod* (a pair as a whole). Patch: build
   and install what was decided. Map: look at the places of any
   worldspace, flat or as a relief. Assets: decide whose textures, meshes
   and faces the game shows where two mods bring their own, by *Duels* (the
   two side by side on the comparison stage) or by *Ranking* (which mods to
   prefer). Mods: the mods, as a list with each mod's details. Toolbox: the tools the setup needs. Settings: where Mod Mallet
   reads, language, theme.
3. **Every thing has one home.** It is shown in full and decided only
   there; elsewhere it appears as a count or a link to its home. The one
   exception is a mod's details, which show the mod's findings as the
   same rows as the task page. The homes are listed in the journey page.
4. **Collisions are decided in Collisions only.** A spot, its passage or
   one object on the stage of *Spots*; a pair as a whole on *Mod against
   mod*. A duel about placed objects is its pair there, not a card of its
   own. The patch page lists the decisions with *undo* and *change* (which
   leads back to Collisions) and makes none of its own. A collision
   finding offers *open in the worklist*, not *hide*. The map decides
   nothing.
5. **Assets is the home of the duels about files** (textures, meshes,
   faces): the comparison stage of ADR 0029 and 0033 becomes a page of its
   own, next to the map, with the ranking as its second tab. Duels about
   placed objects stay with their pair in Collisions. Tasks, the overview
   and a mod's details show the duels about files only as a count or a
   line with *compare*, which opens Assets at the duel. The ranking lives
   here because its one use is to suggest the side of these duels, and it
   is edited where they are decided.
6. **Badges count what their page lists first**: open tasks, open spots, a
   dot for choices not yet in the installed patch, duels about files with
   no side picked, tools needing attention. The sidebar keeps the mod manager's line and loses the
   toolbox pill and the game select.
7. **Worldspaces are listed by what they are**, not by whether mods touch
   them: every worldspace with land or places, read on every scan, named
   by its FULL name in the user's language with the editor ID beside it,
   grouped as the game, Creation Club, mods, and small worlds (folded);
   child worlds that use their parent's land are drawn on the parent. The
   land is shown within the world's bounds. The relief is one renderer
   for every world, coloured from the world's own LOD textures, with the
   CSS palette only as a fallback.
8. **The toolbox lists a tool only when the game or a feature of Mod
   Mallet needs it, and says what for.** LOOT counts as ready when Vortex
   is the mod manager and keeps LOOT's masterlist for the game. The
   Creation Kit stays optional, for repairing navigation meshes, looking
   at a cell as the game builds it, and making mods; hiding and moving
   objects need it not.
9. **New texts live in a module per area**, not in `i18n.ts`, so that the
   packages that build this can run side by side.

## Consequences

- Fewer buttons per page, and each decision has an obvious place. Someone
  used to deciding a pair on the patch page has to go to Collisions.
- The overview and *All games* are two pages where Home was one. A score
  per game on *All games* needs the last scan of each game to be kept;
  until it is, only the game in use shows one.
- The comparison, once reached only through a duel's button, has a place
  in the navigation, and the ranking with it.
- Reading every WRLD on every scan costs little (a few hundred records)
  but changes the cache of the game's plugins.
- The CSS map palette stays for worlds without LOD textures; worlds with
  them look like themselves, which is less uniform.
- The journey page has to be kept with the window: a page that gains a
  third way to do something, or shows another page's thing in full,
  contradicts this record.

## Alternatives considered

- **Keeping the duplicates as shortcuts.** Each was added because it was
  convenient where it stood. Together they made it unclear what a button
  does and whether two places agree; links reach the same place with one
  click more.
- **The comparison as a layer over the page it was opened from, and the
  ranking under Mods.** This record's first version said so. The user
  asked for a clear place for the comparison of files, as the map has one;
  a layer has no place to return to, and the ranking under Mods would be a
  second job on a page meant for knowing the mods.
- **Dropping *Mod against mod*.** The worklist can group by mods. But a
  pair as a whole, with its counts and its suggestion, is how someone who
  knows the mods thinks; the user asked for it as the technical view next
  to the visual one.
- **Deciding on the patch page.** The patch page is where the choices end
  up, which made deciding there natural. But it shows no spot and no
  object; a decision without seeing what it hides is a guess.
- **Dropping the Creation Kit from the toolbox.** Repairing navigation
  meshes stays manual work in it by the vision's non-goals, and findings
  about them need somewhere to point.
- **Listing only worldspaces with collisions.** It is what the map did, and
  it is why Blackreach appeared as a fragment and the cities' worldspaces
  as odd entries. The map's job is to look at places, not only at
  collisions.
