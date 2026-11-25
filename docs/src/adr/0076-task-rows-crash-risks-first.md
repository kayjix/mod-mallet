# 0076 Task rows: crash risks first, one thing to do per row

- Status: accepted; the mods' initials and the page's order changed by [0082](0082-task-page-tiles-by-category.md)
- Date: 2026-11-22
- Changes: ADR 0045 (tasks by what to do), ADR 0061 (the wide-collider advice on the task page)

## Context

The task page sorted findings into steps by what the user does (ADR 0045):
one click in the mod manager, install, update, decide which mod wins, and
so on. On a real setup with about 170 mods that put 26 doors cut off by
navigation meshes, the findings that can crash the game, into *decide
which mod wins*, behind a line about 105 duels of textures and nine rows
about replaced meshes. Each of the 26 carried the same *can crash* tag, and
opened, each offered the same button to the Creation Kit.

The rows themselves read like a raw list. Their only mark of severity was
the colour of a chevron. The mods a finding is about were somewhere in a
long headline. Every row with an action drew a solid orange button at the
right edge, so a page of them was a column of orange; opened, the button
jumped under the text. A suggestion about a mod that collides widely was
shown twice, as a large banner and as a row. Setting a finding aside made
it disappear without a way back. The user's verdict: the list items are
not nice yet.

## Decision

1. **Crash risks come first.** A thing to do (not a recommendation, note or
   set aside) whose impact in the health score is *crash* is listed in a
   first part, *First: what can crash the game*, whatever step it would
   otherwise be in. The steps of ADR 0045 follow with the rest. The parts
   that are things to do are numbered in the order to work in;
   recommendations, notes and what is set aside are not. The collisions'
   line follows the crash risks, or the top of the page when there are none.
2. **Under the headline, a summary** says in words and marks how many of
   the things to do can crash the game, are wrong in it, cost quality or
   keep checks from running, and that none crashes the game where none does.
3. **Every row has the same shape:** a mark, a title, a line with the mods,
   one thing to do, and *Details*.
   - The mark has a shape per impact (a stop sign for *crash*, a triangle
     for *wrong in the game*, a diamond for *quality*, a square for *not
     read*, a ring for notes and recommendations, a tick once handed to the
     mod manager), its words as tooltip and for screen readers. Colour is
     never the only sign.
   - The mods are badges with their initials on a colour taken from the
     name, and their names. A plugin or archive is named only where no mod
     is.
   - The one thing to do is what Mod Mallet can hand to the mod manager, or
     else the way it suggests (the worklist, the pair, the tool). It is a
     tinted button that turns orange on hover. A way that only leads
     somewhere, a mod's page or the map, is a plain button. Switching off
     a mod that others rely on, or the plugins of one still loaded, opens
     the row first, as before (ADR 0061, 0073).
   - The whole row opens on a click; *Details* is what the keyboard
     reaches. Opened, it shows what the finding is, every other way out
     and *leave it as it is*, then what to do and what it means, the mods
     and the spots, without a second fold.
4. **Findings of one kind share one row** where there are several, with
   the kind's title, their count, their mods' badges and *Show all*. Groups
   of up to three start open. A way that is the same for every finding of
   the kind, the tool that repairs paths, is offered once on the group and
   not on its rows. A row inside a group shows its mark only where it
   differs from the group's.
5. **Done steps back.** Once an action is handed to the mod manager, the
   row folds, its mark becomes a tick and its line says so, with *Scan
   again*. A finding set aside moves to the folded part *set aside*, and a
   bar at the bottom of the page offers *Undo*.
6. **A recommendation about a mod that collides widely is one row** among
   the recommendations; its advice (ADR 0061) is behind *Details*. It is no
   longer also a banner above them.
7. **Keyboard:** the arrow keys move between rows across the page, Home and
   End to the first and last; the right arrow opens a row, the left one
   closes it.
8. **The rows are part of the UI kit** ([ADR 0077](0077-one-ui-kit-for-every-page.md)): `ui/rows.tsx` holds
   `TaskRow`, `TaskGroup`, `TaskPart`, `SeverityMark`, `ModBadge` and
   `ModLine`, styled in `ui/rows.css`, with their pure helpers (initials,
   colour, arrow keys) in `ui/row-helpers.ts`. They know nothing about
   findings. The page's head, the note when nothing is to do and the undo
   button are the kit's. The ordering and grouping is pure (`task-plan.ts`)
   and tested.

## Consequences

- On the real setup the 26 doors that can crash the game are the first
  row of the page, one row with one button, instead of 26 rows in the
  fourth step.
- The finding rows of the mods page and the guided review use the same
  row and look the same.
- A finding whose impact is *crash* but that is only a guess stays a
  recommendation: it is not certain enough to be put first.
- Which kinds share a way on their group is decided in the window, for now
  only the paths repaired in the Creation Kit. A new shared way is a change
  in `Tasks.tsx`.

## Alternatives considered

- **Keep the steps and sort crash risks to the top of each.** The steps
  stay where they are, but on the real setup the crash risks were in the
  fourth step, below the fold; the user would not see them first.
- **Order the whole page by impact only.** It loses what ADR 0045 gained:
  the parts say what to do, *install*, *update*, *decide*, not only how bad
  it is.
- **A badge with the word *can crash* on every row.** It repeats the same
  words dozens of times; the part's title says it once, the mark says it
  on every row without words.
- **A solid orange button on every row.** Kept the call to action loud,
  but a page of them had no focus; the tinted button is still the one
  thing to do on its row.
