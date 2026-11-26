# 0077 One UI kit for every page

- Status: accepted, amended 2026-11-25 (switches, sorting and notes that open)
- Date: 2026-11-22

## Context

The desktop app grew page by page, many of them built side by side in
parallel sessions. Each page drew its own pieces, so the same thing looked
and worked differently from page to page:

- Pages started differently: with underline tabs (Mods, Settings), with a
  segmented switch used as tabs (Map collisions, Asset collisions), with
  pill-shaped chips (the catalogue), with a note (Patch) or with nothing at
  all (Toolbox).
- There were three switch systems: `.segmented`, `.segmented-small` and
  `.tabs`. Some used `aria-pressed` for what was really a choice of tab.
- There were five search fields. Two had a style; three were the browser's
  bare input.
- Notes came in more than a dozen variants: left border or full border,
  tinted or plain, 6, 8 or 10 px corners, and none had a sign. Some told a
  warning by colour alone.
- Cards had 10 or 12 px corners, with or without a shadow, sometimes both
  on one page. Shadowed cards sat inside the shadowed table of mods.
- "Undo" was a quiet button, a small button or a plain one, depending on
  the page.
- `App.css` had grown to about 3,200 lines that held every page.

The user installs mods but has never opened a modding tool. A calm,
consistent window matters more to them than any single page.

## Decision

There is one UI kit in `apps/desktop/src/ui/`. Its CSS is `kit.css`, built
on the tokens in `tokens.css` so that both themes follow. Its React
components are in `kit.tsx`. Pages use the kit instead of drawing their
own pieces.

1. **The head of a page** is `PageHeader`. It holds the page's tabs on the
   left, a short word on the page's state in the middle, and the page's
   main action on the right. It has no title: the sidebar names the page,
   and the user preferred the room. The first run's form is the one page
   with a title.
2. **`Tabs`** switch between the parts of a page, using `role="tab"` and
   `aria-selected`. **`Segmented`** changes how the same list is grouped,
   sorted or shown, using `aria-pressed`. There are no other switch
   styles.
3. **Filters** sit in a `Toolbar`. In a narrow column the toolbar is
   `stacked`. There is one `SearchField`, with a magnifier. A select with
   its name in front is a `label.control`.
4. **Buttons** keep their classes: default, `primary` (the one call to
   action), `quiet`, `small` and `link-button`. A new `danger` class marks
   what throws something away; `danger primary` is the confirmed delete.
5. **State is never told by colour alone.**
   - A `Chip` says the state in words; its tone only adds colour.
   - A `Note` has a sign whose shape belongs to its tone: i in a circle for
     info, a tick for ok, ! in a triangle for warning, × in an octagon for
     danger. A screen reader hears the tone's word before the text.
   - Errors anywhere in the window are danger notes.
6. **Cards** have a frame, one radius (`--radius`) and the light shadow
   (`--shadow`), as the user chose. A card inside a card has no shadow.
7. **Undo** is `UndoButton` everywhere: the same sign and the same word.
   **Pins** keep their own colour, as the token `--pin`.
8. **An empty list or page** shows `EmptyState`. **Busy states** use
   `LoadingLine` or `LoadingScreen`.
9. **The window's CSS** is split into files:
   - the tokens and the base in `ui/`;
   - the shell (`shell.css`);
   - the kit;
   - one file per page.

   These files are imported from `App.tsx` in the order the rules used to
   cascade. Each page imports its own further CSS. A page's texts live in
   its own `*-texts.ts` file, not in `i18n.ts`.

## Consequences

- A new page builds from the kit and looks like the others without more
  work. A change to a piece shows on every page at once.
- `App.css` is gone. The split was mechanical first and was checked to give
  every element of every page the same computed style in both themes; the
  look changed only in the later, deliberate commits.
- Some pages still carried their own variants of a note or a mark: the
  passage and reach notes of the 3D panels, the review's effect lines, and
  the work list's kind counts. They are on the kit since 2026-11-26 (see
  the last amendment). The Tasks page, redesigned at the same time,
  joins the kit with its rows and groups.
- The kit's `Note` has the same name as the badge in `spot-notes.tsx`.
  Files that use both import the kit's as `NoteBox`.

## Alternatives considered

- **A component library** such as Radix or MUI. It would add dependencies
  and a look of its own to override. The kit is a few hundred lines on
  tokens the app already has.
- **A visible title on every page.** It was offered and the user declined:
  the sidebar already says where one is, and the full-window pages need the
  height.
- **Flat cards without a shadow.** They are calmer still, but the user
  preferred the cards to stand off the background a little.

## Amendment 2026-11-25: switches, sorting and notes that open

Working with the kit, the user found that the checkboxes looked dated,
that search, grouping and sorting took three rows above the list, and
that the magic wand's slider was wide and plain. The kit gains these
pieces, and the rules grow by these points:

10. **A switch is a `ToggleChip`, not a checkbox.** Its colour, its name
    and its count sit in one chip. When on, the chip is filled and its
    ring holds a tick. When off, the ring is empty and the words are muted.
    This covers the kinds of collision, the worklist's switches and every
    switch in the bar of the 3D views.
11. **Sorting sits at the end of the search, as `SearchSort`.** It is one
    button that steps through the orders. Its bars and arrow show which way
    the list runs; its tooltip says which order it is. Grouping stays a
    small `Segmented`.
12. **A tool with levels is a `LevelToggle`.** A click on its name turns it
    on and off; turning it on again goes back to the level it had. A pip
    for each level sets that level. The pips are filled up to the level,
    and each one says in its tooltip what its level does. The magic wand is
    the first such tool.
13. **A note that would stand as a thick block is a `Note` with a
    `summary`.** It is one line with its tone's sign, and it opens to the
    rest. A passage's summary is the first such note.
14. **A button that is a switch shows when it is on.** If it has no look of
    its own, it gets a frame and a tint. That rule is the weakest in the
    window, so a page's own style always wins.

Map collisions lost its Mod against mod tab, which the passages had made
redundant:

- A pair of mods is now decided as a whole below the spot's panel. "Decide
  this pair as a whole" opens it there.
- A pair handed over from another page opens there too, with its worst
  spot.
- Taking every suggestion moved to the page's head.
- The collisions that name no spot fold below the worklist.

## Amendment 2026-11-26: the last variants join the kit

15. **A kind with a colour of its own is a `Chip` with a `colour`.** A dot
    of the kind's colour stands in front of the words; where a chip holds
    only a count, its kind is named for screen readers and in its tooltip.
    The work list's counts per kind, the kind of a spot or passage in its
    head, and the kinds of collision on a mod's page use it.
16. **What a 3D panel's badge opens is a kit `Note`.** The reach of the
    ways (all reached: ok; ways that change: warning) and the passage's
    notes (why one version has to go: info; parts held back: warning) open
    to the kit's note, and in full they stand as one.
17. **The review's line on what keeping a side does is a compact `Note`**,
    info, or warning where keeping it hides nothing, with "Show these" as
    its action. A passage's summary is the kit's note that opens, without
    a frame of its own.
