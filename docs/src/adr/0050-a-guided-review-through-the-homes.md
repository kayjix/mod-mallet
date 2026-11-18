# 0050 A guided review through the homes, collisions structure by structure

- Status: accepted
- Date: 2026-11-19
- Changes: ADR 0048 (every thing has one home: a second exception), ADR 0040
  (collisions decided spot by spot: also structure by structure)

## Context

ADR 0048 gave every kind of thing one home: findings on Tasks, collisions
on Collisions, duels about files on Assets. Each page is a list to work
through in any order. The user wants to be led instead: through the setup
step by step, in an order that makes sense, able to stop and to carry on
later. In their words, the steps are the installation (which mods), the
load order (does it fit what they want), the assets, the world map, and
the collisions, structure by structure.

The collisions are the heart of it. On the reference setup, two bridges,
one of Lux Via and one of JK's Whiterun Outskirts, stand inside each
other, each built of about forty parts. The worklist of ADR 0040 shows
that as dozens of spots, one per pair of touching parts; deciding them one
by one is slow and hides what the user actually decides: which bridge
stays. The user wants to see the two structures side by side, as the place
looks now, with only the one and with only the other, each mod's parts in
a colour of its own.

## Decision

1. **The review is a page of its own, *Go through it* (*Durchgehen*)**,
   in the sidebar right under the overview. The start page and the
   overview lead to it, with the step where the user stopped.
2. **Five steps, in this order, and a report:**
   1. *Installation*: the mods, how many are switched on, which have a
      newer version, and the open findings about what is installed
      (missing masters and requirements, installer options, script
      extender plugins, recommendations about whole mods).
   2. *Load order*: the findings that depend on the order of the plugins
      (LOOT's rules and messages, incompatible plugins, generated plugins
      out of date, plugins to clean, navigation meshes another plugin
      undoes).
   3. *Assets*: the duels about files that are still open, each leading
      to its comparison on the assets page.
   4. *World map*: the worldspaces with what mods add to them, each
      leading to the map.
   5. *Collisions*: place by place, decided in the review itself (point
      7).
   6. *Report*: the places decided and how, the mods involved, and what
      is left open in every step, each with the way there.
3. **The review owns nothing.** It shows the homes' own rows (the task
   page's finding rows, the assets page's duel lines) and writes into the
   same state (patch choices, duel decisions, findings set aside). What is
   decided in the review shows on Collisions, Assets and Patch, and the
   other way round. This is the second exception to ADR 0048's rule that
   a thing is shown in full only at home: the review is a path through
   the homes, not a home of its own.
4. **A structure is the parts of one mod that hang together**: starting
   from the parts of a collision spot, every part of the same plugin whose
   model box (turned and placed as the game places it) meets another
   within about 6 cm, transitively, no farther than about 30 m from the
   spot (`AreaLayer.connected`, ADR 0040). Not the whole mod, not the
   whole cell. What it grows through depends on what it starts from:
   - from something built, through what is built; never through trees,
     rocks or the landscape, and not through clutter (crates, barrels, a
     `clutter` folder, anything under about 35 cm);
   - from clutter, through clutter: a pile of crates stays a pile;
   - decor of the same mod that touches the built parts (clutter and small
     plants, not trees) is taken along as a leaf, so that hiding a
     building takes its planters and banners with it; it does not grow on
     from there.
   *The same mod* is the mod that placed a part, or, for an object of the
   game, the mod whose version of it wins (ADR 0032): at Candlehearth Hall
   the books on JK's shelf were objects of the game that JK moved there,
   and hiding the shelf left them in the air. Decor counts also where the
   mod's version of another mod's object wins. Such moved objects of the
   game are not *the game's own* for the keep buttons: their ties decide,
   as for the mod's own parts. The keep effect names what rests on a part
   and goes with it, and what rests on it but stays ("then in the air").
   The part started from always belongs to the structure. The object
   card's *parts that hang together* follows the same rules.
5. **A structure is shown three times, through one camera**: the place as
   it is now (with what the patch hides left out), with only the first
   mod's structure, and with only the second's. The first mod's parts are
   blue, the second's orange (Okabe–Ito, told apart with any colour
   vision); red keeps meaning *hidden*. The markings can be switched off
   to see the place as the game shows it. Dragging turns all three views.
6. **Deciding a structure** hides all its parts that the patch may hide
   (objects of the game are never hidden), as objects hidden on their own
   (`chooseObject`), which decides every spot they are part of (ADR 0040's
   `decisionAt`). *Keep both* marks the spot as decided both ways and shows
   both structures again (`revealObjects`). *Undo* takes the parts out of
   the patch's choices. A keep button is never silent about what it cannot
   do: before deciding, the review asks the game's ties of the other side's
   parts (ADR 0046) and says how many can go ("3 of 5 parts of Immersive
   Citizens can go"). Parts the game needs (*locked*, or marked by the scan
   as not to be hidden) stay, with the reasons listed, parts with the same
   reasons together. Parts that are only tied in (*careful*) stay unless the
   user says *hide anyway*. Where nothing can go, the button is off and
   says so, and *keep both* is suggested.
7. **Places and segments.** On the reference setup the bridges at
   Markarth came up as one small structure after another, 3, then 5, then
   2 parts, each to be decided anew. So the collisions step works on two
   levels:
   - A *place* (cluster) is the spots of the same two mods in one
     worldspace or cell whose points lie within about 30 m of one another,
     chained: the bridges at Markarth. The three views show the whole
     place, and one choice decides it (*keep JK's here*).
   - A *segment* is what one structure decision covers: the structures of
     the two mods around one or more of the place's spots, joined where
     they share a part. Below the views each segment is listed with what
     is decided there, as it is now; each can be shown on its own and
     decided differently.
   Deciding a place or a segment hides the losing side's structures and
   decides every spot in it, also where the losing side's parts there are
   the game's own and stay. Deciding a place moves on to the next place
   with an open spot. *Previous* and *next* go place by place, decided
   ones included, and say what was decided. Other mods with objects among
   the structures (at Riften's gate a third and fourth) are listed and can
   be picked out.
8. **Progress is what is decided**, wherever it was decided: open duels,
   open spots, open findings. Steps that are only looked at (the
   installation, the load order, the map) are marked as seen, and the
   step the user was on is remembered, both in the window's storage per
   game; losing them costs a click, never a decision.
9. **Three levels, all visible.** At JK's Bee and Barb against JKs Skyrim
   a whole structure stood against a single item; hiding the item is
   harmless, hiding a segment is not. So the step says what each side
   brings ("JKs Skyrim: a single item, a woodpile"; "JK's Bee and Barb: 40
   parts in 2 sections"), and every segment opens, on request, into its
   objects one by one, each to hide or to show again; a click on an object
   in any of the three views does the same. These are objects hidden on
   their own (`chooseObject`, `revealObjects`); the game's own and the
   parts it needs are not offered. A segment's decision counts them as
   exceptions ("decided, with 2 exceptions").
10. **A report of what was decided**, at the end of the review and on the
    patch page, pair by pair of mods: what of each mod is hidden, place by
    place with the objects' names; what was kept and with how many
    exceptions; what is still open; what stays because the game needs it.
    Objects hidden on their own are named and placed from what the window
    remembers of the objects it showed (in its storage per game; without
    it they are counted). The report is written as a text file into Mod
    Mallet's own folder next to the patch whenever the patch is created,
    and on request (`save_patch_report`); nothing goes into the game's or
    a mod's folders.
11. **"You might not need it any more."** Where the suggestions about whole
    mods (ADR 0043) know how many objects a mod places and that it has no
    rooms of its own, the report names a mod of which, after the
    decisions, no more than 15 % and no more than 30 objects are left. It
    says so gently and leaves uninstalling to the user. This is counted in
    the window from the patch's choices; findings stay what the scan read.
12. **A suggestion by size.** At JK's Nightgate Inn against Cutting Room
    Floor a building of 270 parts stood against one lantern post. Each
    place and segment gets a suggested side by geometry alone, apart from
    LOOT's word and the suggestions about whole mods: the room each side
    takes (the sum of its parts' model boxes as placed, each side at least
    one unit) and its parts. A side is suggested when it takes at least 20
    times the room of the other, however many parts either has (one huge
    bridge against a fence of thirty posts keeps the bridge), or has at
    least 20 times the parts with at least 5 times the room. Otherwise
    there is no suggestion and the place is the user's to look at. The
    suggested keep button says so, with one line of reason (parts and
    cubic metres of each side). *Measure the open places* reads every open
    place in the background, one after the other, as the stage would; then
    *take every clear suggestion* decides them as honestly as the keep
    buttons (only parts that may go are hidden; tied-in and needed parts
    stay, and a place where only those could go stays open), and one
    *undo* takes the whole batch back. The ratios are a first guess, to be
    tuned on the user's places.

## Consequences

- The collisions can be decided where the user thinks in structures; the
  worklist stays for spots one by one and the pairs for a mod against a
  mod as a whole. Three ways now lead to the same choices; the review is
  the guided one, the other two stay the pages to look things up.
- The three views draw the area three times per frame, through one
  drawing context with the models read once; the looks of a model are
  rewritten in place between the views, and the stage draws at half
  resolution while the camera moves.
- A place is found by distance alone. Two unrelated buildings of the same
  two mods within 30 m of each other are one place; its segments keep
  them apart where they do not touch.
- Finding a structure needs the models' boxes, so it is known only once
  the area's models are read; until then the view shows the spot's own
  parts.
- The assets step leads to the assets page for now. Its own three-way
  view (the first mod, the game's own file, the second mod, and which one
  wins in the load order now) uses the same stage and comes next.

## Alternatives considered

- **A mode of the overview.** The overview says how the game stands; a
  mode that walks through decisions would give it a second job.
- **Walking the worklist's spots in order.** One bridge of forty parts
  is dozens of spots. The user decides which bridge stays, once.
- **The structure as all of a mod's objects in the cell.** A mod places
  unrelated things next to each other; hiding them all with the bridge
  takes away more than the user decided.
