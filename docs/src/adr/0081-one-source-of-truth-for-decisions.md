# 0081 One source of truth for decisions

- Status: accepted
- Date: 2026-11-22

## Context

The user decides about the same objects on many pages: the Map (its object
card and 3D relief), Map collisions (a spot, a passage, stacked doors), Go
through it (the three-pane comparison, its sections and part lists), the
Patch page and the counts of the Overview. Their rule: if an object is
hidden on one page, every other page shows it hidden, and it can be shown
again from any of them; the same for every kind of decision.

Every page already read and wrote one map of choices (`PatchChoices`, kept
in the patch book of ADR 0070). But each page worked out on its own what
the choices hide and why, and these copies drifted apart. Each of these
went out of sync once and was fixed where it showed: merged doors against
the object card (ADR 0067), passage decisions against "show again"
(ADR 0072), "show again" lost after a restart because the installed patch
was merged back, and "All of them stay" leaving hides in place. Looking for
more of the kind turned up these:

1. The Map's 3D relief was given nothing to draw as hidden. An object hidden
   on the map's card, or by a decision in Map collisions or Go through it,
   still stood there as before.
2. Findings set aside were counted differently. The written patch, the 3D
   view of Map collisions and the Map's paths left their spot decisions
   out; the object cards, Go through it, the list of conflicts of an object,
   the places where characters act and the report of a pair counted them.
   The same object was hidden on one page and not on another.
3. The cards told "hidden on its own" from "hidden by a decision" by
   looking for the object's choice under the exact spelling of its plugin.
   The patch in the game writes plugins in lower case, so an object it hid
   on its own read as hidden by a decision.
4. The card of Map collisions said "hidden by the passage decision" for any
   hidden part of a spot inside a passage, also where a single spot or the
   user alone had hidden it. The Map's card never named a passage or doors.
5. A door of stacked doors hidden from the Map's card, the magic wand or a
   part list of Go through it was hidden on its own. Only the 3D view of Map
   collisions went through the door decision, which merges the doors into
   the one left.
6. A passage decided as a whole stayed "decided" without exception when a
   part of it that no spot names, hidden with the passage, was shown again
   on its own; and stayed "both kept" when a part was hidden by hand after
   keeping both.
7. The file choices of duels (Asset collisions, ADR 0054) were kept twice:
   in the settings beside the patches, which every page read, and as
   `duel:` decisions of the patch being edited, which only the patch page
   read. Picks made before ADR 0070 were in no patch at all. On the user's
   setup all 114 picks were outside the patches; they started a new patch
   to begin afresh, yet every one of the 105 file duels still counted as
   decided, so the Asset page, which hides decided duels, looked empty.

## Decision

What the decisions mean is answered in one module, `decisions.ts`, for a
snapshot of the choices, and every page asks it rather than working it out
itself. The snapshot (`Decisions`) offers:

- `isHidden(object)` and `hiddenKeys`, `hiddenObjects`: what is hidden;
- `hiding(object)`: why, as one of on its own, by a spot (or its whole
  finding), by doors merged into another, or by a passage decided as a
  whole; and the patches that make the deciding choice, so a page can say
  when it is not the patch being edited;
- `waits(object)`: whether a decision that waits for a switched-off mod
  (ADR 0061) would hide it;
- `decisionOf(spot)` and `decisionOfPassage(passage)`, the latter with
  exceptions: a part shown again on purpose on the side that goes, or
  hidden by hand on the side that stays;
- `show(objects)` and `hide(objects)`: the choices with objects shown again
  whatever hid them, or hidden, a door of stacked doors always through the
  door decision;
- `duels`: the side picked in each duel of files, from the `duel:`
  decisions of the patches that count, and `patchesOf(key)` to say which
  patch makes a decision.

A duel's file choice is a decision of the patches like any other: a pick
goes into the patch being edited, taking one back leaves it out of the
patches that make it, and it counts while its patch does. The settings no
longer keep picks beside the patches. Picks kept there before move once
into the user's first patch, the one there was before there were several,
and leave the settings, so that they cannot come back from there. What a
pick already did in the mod manager (the file overrides and plugin rules it
asked for when it was made, ADR 0054) stays as it is: switching patches
changes what Mod Mallet counts as decided, not the mod manager. Where every
duel is decided, the Asset page lists them and names the patches they are
in instead of showing an empty list.

Decisions about findings set aside hide nothing, on any page and in the
patch.

The window makes the snapshot once per change of the patch book or the
report (`snapshotOf`, from the book, the report's spots and passages, the
switched-off plugins and the findings set aside) and hands it to every page
in the session (`useDecisions()`). A page keys what it derives on the
snapshot, so every page open at the same time redraws when a decision
changes on any of them. A page's change goes back into the book as before:
into the patch being edited, with the waiting decisions put back
(`bookWith`). The choices at the first start, from the patch in the game
and what was kept before, are made by `choicesAtStart`.

Pages may still compute with the pure functions of `patch.ts` where they
build a change from several steps (deciding a passage, keeping one side of
a section). They read what is hidden only from the snapshot.

The tests in `apps/desktop/test/decisions.test.ts` replay sequences across
pages through the same functions the window uses, and check what each page
shows: hidden on the map, then in the collisions, the review and the patch;
a passage decided and a part shown again from the map; doors merged, a
door hidden from the card, then "All of them stay"; the patch being edited
switched; a restart that merges the patch in the game again; a finding set
aside; a mod switched off; a new patch with every file duel open again
while the old patch keeps its picks.

## The decisions and where they live

| Decision | Key in the choices | Written by | Read by |
| --- | --- | --- | --- |
| A collision as a whole | finding id → plugin or null | Map collisions (pairs), Patch page | every page, through the snapshot |
| A spot | `spot:` → plugin, null, or `keep:` | Map collisions, Go through it, passage decisions | every page, through the snapshot |
| Stacked doors merged | `spot:` → `keep:` (ADR 0067) | door panel (`mergeDoors`), `hide` | every page; the patch's door moves |
| An object on its own | `object:` → plugin; null for shown again | object cards, wand, part lists, `show`/`hide` | every page, through the snapshot |
| A passage as a whole | its spots' `spot:` keys plus `object:` keys of parts no spot names | Map collisions, Go through it | `decisionOfPassage` |
| A move | `move:` (ADR 0060) | move controls | 3D views, the patch |
| Paths repaired | `repair:`, `norepair:` (ADR 0058, 0070) | repair panels, Patch page | the patch |
| A pin | `pin:` (ADR 0050), in the book, in no patch | pin buttons | worklist, 3D views |
| A file choice of a duel | `duel:` (ADR 0054); before, also in the settings | Asset collisions, ranking | every page, through the snapshot (`duels`) |
| Which patch | the book: patches, ticked, switched off, edited (ADR 0070) | Patch page, editing chip | the snapshot (`hiding().patches`) |
| Waiting | any key naming a switched-off plugin (ADR 0061) | — (split off when read) | the snapshot (`waits`), switched-off notes |

## Consequences

- A new page that shows or changes what is hidden uses `useDecisions()`;
  there is nothing to keep in step by hand.
- A new kind of decision that hides objects is taught to `decisions.ts`
  once, with a test that replays it across pages.
- The snapshot is made again on every change of the book, which costs a
  pass over the spots per change, as each page's own pass did before.
- A new patch with the old one switched off starts with every duel open
  again; the user decides them anew for it, and what that asks of the mod
  manager runs when they pick.
