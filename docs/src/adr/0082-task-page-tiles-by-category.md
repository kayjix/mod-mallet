# 0082 The task page: tiles by category over one list

- Status: accepted
- Date: 2026-11-25
- Changes: ADR 0076 (task rows; the mods' initials and the numbered steps)

## Context

After ADR 0076 the user found the task page *a bit better, but not there
yet*. Two things stood out. The rows showed each mod as two-letter
initials on a colour, like "JS" for *JK's Skyrim*; initials stand for a
person's first and last name and say nothing about a mod. And the page was
one long column: on the real setup the 33 crash risks, the one click, the
decisions, the things to do by hand, the recommendations and 261 notes all
followed each other, so there was no overview of what kinds of work there
are and how much of each.

Vortex knows a picture for many mods, but the Mod Mallet extension does not
pass it on, and showing it would load images from Nexus.

## Decision

1. **A board of tiles at the top of the task page**, one per category, each
   with its count and a line on what it means:
   - *Everything open* (the things to do, crash risks first),
   - *Collisions* (spots still to decide),
   - *One click* (the mod manager does it),
   - *Decisions* (which of two mods wins, with the duels of files),
   - *Do it yourself* (install, update, look into it),
   - *Recommendations*, *Good to know*, *Set aside*.

   A tile with crash risks says how many, with the crash mark. The tiles
   that are only to know or set aside step back.
2. **The tiles are tabs.** Clicking one shows its list under the board;
   the arrow keys, Home and End move between them. The page opens on
   *Everything open*: the crash risks as the first part, then one click,
   decisions and the things to do yourself, numbered. Recommendations,
   notes and what is set aside are only under their tiles.
3. **A category's list** starts with a line on what it is and, where it is
   handled elsewhere, the way there: the toolbox for things to do
   yourself, the mods page for one click and recommendations, Assets from
   the duels' line for decisions, the worklist and the patch for
   collisions. Collisions are listed by kind and pair of mods with the
   button to their spot in the worklist.
4. **Lists of what is only to know or set aside are calm**: their rows
   carry the mark of a note, whatever they would do to the game, and offer
   no shared repair.
5. **No initials.** A row names its mods in words, the first two and how
   many more; the tooltip names all. No picture from Vortex for now: the
   user chose names only over a neutral icon or pictures from Nexus.
6. The categories are pure and tested (`task-plan.ts`: `categoryOf`,
   `countCategories`, `planView`). The tiles are a component of the UI kit
   (`ui/tiles.tsx`: `TileTabs`), knowing nothing about findings.

## Consequences

- On the real setup the page opens with four rows: 33 doors that can crash
  the game in one group, one switched-off plugin still loaded, the nine
  textures that miss replaced meshes, and seven walkways the patch hides.
  The 261 notes are behind their tile.
- A category is a change of `task-plan.ts` and its texts; the steps of
  ADR 0045 remain beneath it as the sort inside a category.
- Mods without a picture look the same as before; a picture would need the
  extension and the report to carry it.

## Alternatives considered

- **Pictures from Vortex.** Recognisable, but the extension and the report
  would have to carry them, the window would load them from Nexus, and
  many mods have none. Left for later.
- **A neutral icon before every mod.** Says nothing a name does not.
- **Only the crash risks before a tile is chosen.** Quieter, but hides the
  one-click fixes that take a second; the user chose everything open with
  the crash risks first.
