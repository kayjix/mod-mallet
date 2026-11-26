# The journey through the app

The desktop app grew one view at a time, each added for a problem found in
a real setup. By November 2026 the same things were shown and decided in
several places: a collision could be decided on four pages, the duels
appeared on three, the scan button and the game picker twice. This page
took stock and set the structure the app grew into: one job per page, at
most two ways to do something on it, and links instead of copies. The
decisions are recorded in [ADR 0048](../adr/0048-one-job-per-page.md), and
the structure has been built.

The page describes the window as it stands on 2026-11-26. Two parts are
history and say so: "Where it started" (the window before ADR 0048, kept
as the reason for the structure) and the work packages at the end (done).
Since the structure was set, ADR 0050 added the guided review, now *Step by
step* (ADR 0083); ADR 0054 split the file duels (*Asset collisions*) from
the catalogue of the game's things (*Assets*); ADR 0072 made *passages* the
unit of a collision and removed the *Mod against mod* tab of the collisions
page; ADR 0070 made the patch several patches, each its own mod; ADR 0077
gave every page one UI kit; and ADR 0086 added a small helper, Mal the
Rubber Mallet.

The sidebar and the pages are, from the top: *Overview* (the home of the
game in use), *Step by step*, *Tasks*, *Map collisions*, *Asset
collisions*, *Patch*, *Map*, *Assets* (the catalogue of things), *Mods*;
beside them *Toolbox* and *Settings*; and the start page behind the name
*Mod Mallet*, which lists all games.

## Where it started

The tables below describe the window as it was when ADR 0048 was written,
with the pages it had then (*Home*, *Collisions · Mod against mod*,
*Compare*). They are kept as the reason for the structure and are no longer
true of the window.

### The frame

| Part | What it holds | Opens |
|---|---|---|
| Sidebar, top | Brand; **game picker** (a select of all games); **scan box** (scan, last scan, *live*) | – |
| Sidebar, navigation | Home, Tasks (badge: errors + warnings), Collisions (badge: collision *findings*), Patch, Map, Mods, Toolbox; Settings at the bottom | the view |
| Sidebar, foot | **Mod manager status** (Vortex running, link ready / outdated / restart); game and script extender version; **toolbox pill** (ready / *n* need attention) | both open Toolbox |
| Hidden views | **Compare** (duels and ranking), reached only from a duel's button, with a back button; **Places** (first run) | – |

### The pages

| Page | Panels | Actions |
|---|---|---|
| **Home** | Hero with the active game, its version, **scan**, *to the tasks*; **health** (score, verdict, four reasons with a button each); four **tiles** (open tasks, open spots, mods enabled, objects in the patch); **all games** as cards | scan; open tasks, collisions, mods, patch; switch game |
| **Tasks** | Headline; **collisions card** (open spots, pairs, *to the patch*, *open worklist*); steps (one click, install, update, choose, could not read, check, recommendations, good to know, set aside); in *choose*: **duels** not about placed objects; scan statistics; *what was read* | per finding: the mod manager's action, set aside, *hide in the patch* (for a collision finding), *show on map* per spot; per duel: pick a side, *compare* |
| **Collisions · Spots** | Filters (search, kinds, group by region or mods, sort, hide decided); **worklist**; **stage**: the spot in 3D (exterior or interior), what collides, decide | decide the spot, the passage, **the pair**; hide or show a single object; preview without what is hidden; previous / next / next open; *to the patch* |
| **Collisions · Mod against mod** | **Pairs** of mods with counts and a verdict; below them the **duels about placed objects**; below those the **collision findings** not in a duel, as a task list | decide a pair; show its spots; pick a duel's side; *compare*; per finding: hide in the patch, show on map |
| **Compare** (hidden) | Tabs *duels* and *ranking*; duel picker; stage with the two mods' files; decision under it; ranking of mods | pick a side; order the ranking; open a spot in the worklist; *to the patch* |
| **Patch** | Needs Base Object Swapper; counts; installed patch against the choices; **pairs** with *hide A* / *hide B* / *keep both*; objects hidden on their own | *suggest all*, *clear*, *create*, *install through Vortex*, *use what is installed*, open folder; decide a pair; show an object again |
| **Map** | World select (by internal name), mod select, flat or relief; flat: cell grid with mod density, places panel, legend; relief: the game's LOD, objects up close; detail panel (world, place, cell with console command, collisions there) | pick world, mod, cell, place; *decide* a collision spot (opens the worklist) |
| **Mods** | Search, filter, sort; table like a mod manager; a mod opens: problems, collisions per other mod, what others cover, duels, recommendations | per finding as on Tasks; open the worklist at a pair's spot; the mod's page |
| **Toolbox** | Tools by group (mod manager, link, game, helpers) with state and version | restart Vortex, install or update the link, open a tool's page |
| **Settings** | Tabs *general* (language, theme, live), *places* (where Mod Mallet reads), *about* | confirm the places |

### Shown or decided more than once

1. **A collision is decided in up to six places**: the spot on the stage,
   the passage and the pair on the stage, the group head of the worklist
   when grouped by mods, the pair on *Mod against mod*, the duel about
   placed objects on the same tab, the pair on *Patch* (with *suggest
   all*), *hide in the patch* on a collision finding, and the duel's
   decision in *Compare*. They all write the same patch choices, so they
   agree, but the user cannot tell which one is meant.
2. **A pair of mods is shown three times on one tab**: as a pair, as a duel
   about placed objects, and as the findings that neither covers.
3. **Duels** appear on Tasks, on *Mod against mod*, in Compare and in a mod's
   details, each time as the full card with its buttons.
4. **The collisions are counted four ways**: the sidebar badge counts
   findings (104 on the reference setup), the tiles and the collisions card
   count open spots, the worklist counts spots in its filter.
5. **Scanning** is in the sidebar and in Home's hero; **switching the game**
   in the sidebar select and on Home's cards; **the game's version** in the
   sidebar foot and in the hero.
6. **The toolbox's state** has two indicators in the sidebar foot (the
   manager status and the pill), both opening the same page.
7. **Where a spot is** is answered twice in different ways: *show on map*
   from a finding opens the flat map, *open worklist* the 3D close-up.

### Orphaned or misplaced

- **Compare** has no place in the navigation. Its *ranking* tab, the only
  place where the ranking of mods is set, is found only through a duel.
- **Home is two pages at once**: the list of all games and the state of
  the one in use. Switching to another game keeps its own page title.
- **Scan statistics and *what was read*** close the task page, where
  nobody is looking for them.
- **The map's mod select** filters the map to one mod, but no mod links
  there.
- **The density of mods per cell** colours the flat map; it was an early
  stand-in for collisions and now competes with the spots.
- **The old report** (`Report`, `Overview`, the *arrange* switch) is no
  longer reachable once the task page by steps is in; its code and styles
  remain.

## The structure

### Two levels

The window has two levels: **all games**, and **one game**. Everything a
scan finds belongs to one game, so every page but *all games* and
*settings* is a page of the game in use.

```text
All games ──▶ Overview of a game ──▶ Step by step  (the guided review, ADR 0083)
                      │             ├─▶ Tasks
                      │             ├─▶ Map collisions    (grouped by region, mod pair or passage)
                      │             ├─▶ Asset collisions  (Duels | Ranking)
                      │             ├─▶ Patch             (several patches, ADR 0070)
                      │             ├─▶ Map
                      │             ├─▶ Assets            (the catalogue: by kind, material or as sets)
                      │             └─▶ Mods              (List | Dependencies)
                      └─ the sidebar reaches every page directly
Toolbox, Settings: beside the game's pages, always reachable
```

### The sidebar

```text
┌──────────────────────────┐
│ ◆ Mod Mallet             │  ← the name opens the start page
│ ┌──────────────────────┐ │
│ │ [art] Skyrim SE      │ │  ← the game in use (click: Overview)
│ │ [ Scan ]  14:02 ·live│ │
│ └──────────────────────┘ │
│ ▣ Overview               │
│ ➜ Step by step           │  ← the guided review (ADR 0050, 0083)
│ ☑ Tasks              12  │  ← open tasks (ADR 0045 counts.tasks)
│ ⧉ Map collisions     37  │  ← open spots
│ ▨ Asset collisions    4  │  ← duels about files without a side picked
│ ✎ Patch               ●  │  ← dot: decided but not in the patch
│   [editing: Patch 1]     │  ← the patch being edited, on the pages that decide
│ ⌖ Map                    │
│ ▥ Assets                 │  ← every thing of the game and the mods
│ ≡ Mods                   │
│ ──────────────────────── │
│ ⚒ Toolbox             2  │  ← tools needing attention
│ ⚙ Settings               │
│ ──────────────────────── │
│ ● Vortex · link ready    │  ← opens Toolbox
└──────────────────────────┘
```

- The select of games goes. The start page, opened by the name *Mod
  Mallet* at the top, is the one place to switch games. The logo beside
  it stays a toy.
- The game's card shows its name and art; version and script extender
  move to the overview.
- Badges count what the page itself lists first, so the numbers agree:
  Tasks the open tasks, Map collisions the open spots, Patch a dot when
  choices are not yet in the installed patch, Asset collisions the duels
  about files with no side picked, Toolbox the tools needing attention. The toolbox pill goes; the manager line stays.
- Before a scan, every game page shows the same empty state with the scan
  button; before the places are confirmed, only Settings and the first-run
  page are open (as today).

### One job per page

| Page | Its one job | The ways to do it (at most two) | It does not |
|---|---|---|---|
| **Start** (all games) | Choose what to do and which game | the game in use with its state and the way to what is open; a card per game to switch | list findings, decide anything |
| **Overview** | Say how the game stands and where to go | the health with its reasons; a tile per page | list findings, decide anything |
| **Step by step** | Lead through the setup step by step, and remember how far | the steps in order; a step's own items, shown by the homes' own rows | own any item: it is a path through the homes (ADR 0050) |
| **Tasks** | Work through what is to do, except collisions and duels | by step | list collisions or duels (one line each leads to them) |
| **Map collisions** | Decide every collision spot | the worklist grouped by region, by pair of mods or by passage (ADR 0072), one spot at a time in 3D; a passage or a whole pair decided from the stage | build the patch |
| **Patch** | Keep, build and install the patches: one patch is the one being edited, the others keep their decisions (ADR 0070) | the list of a patch's decisions (undo, or *change* in the page it was made on); create, load, switch off and install | make new decisions |
| **Map** | Look at the places of any worldspace | flat or relief | decide; a spot links to the worklist |
| **Asset collisions** | Decide whose textures, meshes and faces the game shows where two mods bring their own | *Duels*: two mods side by side on the comparison stage; *Ranking*: which mods to prefer, which suggests every duel's side | decide placed objects (Map collisions does) |
| **Assets** | Look at every thing of the game and the mods, and choose the version of one thing (ADR 0054) | the list, by kind, material or as sets; a thing's versions side by side | decide whole pairs of mods (Asset collisions does) |
| **Mods** | Know the mods, one at a time | *List*: the mods and a mod's details; *Dependencies*: which plugin needs which, as arcs (ADR 0068) | decide collisions or duels in place |
| **Toolbox** | Have the tools the setup needs | the list; the one button a tool offers | – |
| **Settings** | Where Mod Mallet reads, language, theme, which preferences it may offer | tabs: *General*, *Places*, *Preferences* (ADR 0086), *About* | – |

### Explanations

Pages carry no lines of introduction or explanation, and no headings in
their place. What a control does is said by its label. Where a short text
really helps, such as how the score is counted, how to order the ranking or
how to move a 3D view, it sits behind one small ⓘ beside the control; it
shows on hover and stays open on a click (`info-tip.tsx`). Not
explanations, and so shown as they are: what an empty panel waits for, and
notes that belong to a decision, such as a warning that characters walk on
what a choice would hide.

### Where each thing lives

Every kind of thing has one **home**, where it is shown in full and
decided. Elsewhere it appears as a **count** or a **link** to its home,
never as a second full copy with its own buttons. The one exception is a
mod's details on the mods page, which shows the mod's findings as the
same rows as Tasks: looking at one mod is the second way to work through
them.

| Thing | Home | Elsewhere, only as |
|---|---|---|
| Finding (not a collision) | Tasks, in its step | a reason on the overview; a row in a mod's details |
| Recommendation about a mod (ADR 0043) | Tasks, *recommendations* | a line in the mod's details |
| Collision spot | Map collisions | counts (sidebar, overview, Tasks' collisions tile, a mod's details); a pin on the map with *open in the worklist* |
| Passage (places where two mods build the same thing, ADR 0072) | Map collisions, when grouped by passage; Step by step, in its collisions step (ADR 0080) | the spots it holds |
| Pair of mods whose objects collide | the pair's panel under the stage in Map collisions | a line in each mod's details with *open the pair*; the report per pair in Step by step |
| Duel about placed objects | merged into its pair | – |
| Duel about files (textures, meshes, faces) | Asset collisions · Duels | a count on the overview and in Tasks' *choose* step, a line in each mod's details, each with *compare*, which opens Asset collisions at the duel |
| The comparison stage | Asset collisions · Duels | the same stage shows a thing's versions on Assets |
| Ranking of mods | Asset collisions · Ranking | the reason given under a duel's decision |
| Patch choices | made in Map collisions, Asset collisions, Step by step and the 3D views, into the patch being edited; listed in Patch (ADR 0070, 0081) | the dot on Patch; *waiting* spots in the worklist |
| A preference between mods, learned from the decisions | Settings, *Preferences*; offered by Mal in the corner (ADR 0086) | – |
| Version changes since the last scan (ADR 0079) | the overview | the toolbox counts the plugins that fit |
| Places and worldspaces | Map | the place named on a spot |
| A thing of the game (weapon, armour, character, item), its versions and the version chosen for it | Assets | – |
| Outfit (a set of things characters wear) | Assets, *as sets* | a line under each of its things |
| Tools | Toolbox | the manager line; a finding's *needs tool* link |
| Scan statistics, *what was read* | Overview, folded at the bottom | – |

The rule for the collision decisions in particular:

- **A spot, its passage, or one object** is decided on the stage of
  *Map collisions*, or in the collisions step of *Step by step*, which
  writes the same choices.
- **A pair as a whole** is decided from the pair's panel under the stage,
  which the stage opens with *this pair*. The *Mod against mod* tab of the
  first plan is gone: passages (ADR 0072) took its place, and the report
  per pair of Step by step says what was decided per pair.
- **A duel about placed objects is the pair.** Its side picks become the
  pair's verdict; there is no card of its own.
- **Patch** shows what was decided, grouped by what it is about (places
  where mods collide, objects hidden or shown on their own, moved objects
  and merged doors, repaired paths, file choices), with *undo* and
  *change*, which opens the place where it was made. Every decision goes
  into the patch being edited; the other patches keep theirs.
- **A collision finding** (on a mod's details, or set aside) offers
  *open in the worklist*, no *hide* buttons. *Show on map* stays for
  findings that are not collisions but have a place, such as a door cut
  off by a navigation mesh.

### The pages in turn

#### Start (all games)

```text
┌─ [art of the game in use] ────────────────────────────────────────┐
│ ╭──╮ Skyrim Special Edition                                       │
│ │72│ 1.6.1170 · SKSE 2.2.6 · scanned today 14:02                  │
│ ╰──╯ Playable, but the quality suffers                            │
│ [ To the overview ]                                               │
├────────────────┬────────────────┬────────────────┬────────────────┤
│ 12 things      │ 37 collision   │ 4 pairs of     │ 214 objects    │
│ to do          │ spots open     │ mods to compare│ hidden by patch│
├────────────────┴────────────────┴────────────────┴────────────────┤
│ Games                                                             │
│ ┌────────────┐ ┌────────────┐ ┌────────────┐ ┌────────────┐       │
│ │   [art]    │ │   [art]    │ │   [art]    │ │   [art]    │       │
│ │ Skyrim SE  │ │ Fallout 4  │ │ Oblivion   │ │ Starfield  │       │
│ │ ● in use   │ │ basic      │ │ basic      │ │ soon – …   │       │
│ │            │ │ [switch]   │ │ [switch]   │ │            │       │
│ └────────────┘ └────────────┘ └────────────┘ └────────────┘       │
└───────────────────────────────────────────────────────────────────┘
```

The user asked for the old home page back: the game in use at a glance,
with the way to what is open, and every game with its picture and state,
to switch to another. The start page is that page, reached by the name
*Mod Mallet* at the top of the sidebar. The game's overview stays the
place for the reasons behind the score. The window opens on the overview
of the game in use; the start page is one click away.

#### Step by step

```text
┌─ Step by step ───────────────────────────────────────────────────┐
│ ① Installation ✓  ② Assets 2  ③ World map  ④ Collisions  ⑤ Report │
├───────────────────────────────────────────────────────────────────┤
│ Collisions · place 4 of 31            Lux Via ⟷ JK's Outskirts    │
│ ┌─────────────────┬─────────────────┬─────────────────┐           │
│ │ now             │ only Lux Via    │ only JK's       │           │
│ │  blue + orange  │  blue           │  orange         │           │
│ └─────────────────┴─────────────────┴─────────────────┘           │
│ ☑ markings    Lux Via: 41 parts · JK's: 38 parts                  │
│ [Keep Lux Via's] [Keep JK's] [Keep both]  [skip]  [undo]          │
│ [◂ back]                                          [next step ▸]   │
└───────────────────────────────────────────────────────────────────┘
```

The guided review walks through five steps: the installation (with the
plugins and their order, which the mod manager sets), the assets, a map
of where the collisions are, the collisions and a report. The
installation opens with who does what: Mod Mallet decides, the mod
manager carries out, the game shows. The assets show the biggest duels in
pictures, the map leads to each place, and the report says what the
patch does, what is still open and what to try in the game (ADR 0083).
The collisions are decided in the review itself, one place at a time. A structure is
the parts of one mod that hang together; the stage shows the place as it
is now, with only the one structure and with only the other, through one
camera. Deciding writes the same patch choices the collisions page
writes. ADR 0050 records the rules.

#### Overview of a game

```text
┌─ [art banner]  Skyrim Special Edition · 1.6.1170 · SKSE 2.2.6 ────┐
├──────────────────────────────┬────────────────────────────────────┤
│  ╭───╮  Playable, but the    │ What lowers it                     │
│  │72 │  quality suffers      │ [broken] 2 plugins switched off −12│
│  ╰───╯                       │ [quality] 37 spots undecided   −9  │
│                              │ [crash]  door cut off          −8  │
│                              │ (each line: [ Go ] to its page)    │
├──────────────┬───────────────┼──────────────┬─────────────────────┤
│ Tasks     12 │ Collisions 37 │ Assets     4 │ Patch           214 │
│ 1 can crash  │ 8 decided,    │ pairs to     │ objects hidden      │
│              │ not in patch  │ compare      │ ● not current       │
├──────────────┼───────────────┼──────────────┼─────────────────────┤
│ Mods 168/172 │ Map         4 │ Toolbox    2 │                     │
│ 3 newer      │ Skyrim, Sol-  │ need         │                     │
│ versions     │ stheim, …     │ attention    │                     │
├──────────────┴───────────────┴──────────────┴─────────────────────┤
│ ▸ 412 plugins, 960 archives, 170 installers, 28 812 meshes read   │
│ ▸ What was read                                                   │
└───────────────────────────────────────────────────────────────────┘
```

The overview is the home of a game. Its banner names the game and its
version and carries the button that starts or continues *Step by step*.
Under it are the health panel and the tiles of ADR 0045, with a tile per
page, then what changed since the last scan (ADR 0079: a game or mod update
and what it did to the setup), and the scan statistics folded at the bottom.
Each tile is one button to its page; nothing on the overview is decided.
Before a scan it only offers the scan.

#### Tasks

The tasks of ADR 0045, laid out as ADR 0076 and 0082 set it out: a headline
with the number of open things, and over one list a row of **tiles by
category**: everything open (crash risks first), collisions, one click
(the mod manager does it), decisions (which of two mods wins, with the
duels about files), things to do yourself, advice, notes, and what was set
aside. Each tile says in a few words what it means and how many of its
rows can crash the game; a tile chooses what the list below shows. The
scan statistics and *what was read* are on the overview; a collision
finding never appears here, only the tile that leads to Map collisions; the
duels about files are one line that leads to Asset collisions.

An opened task row is compact: what the finding is, then one row of
buttons right under it (the mod manager's action, the way to where it is
decided, the mod's page, the map, *leave it as it is*). Everything else,
the places, the mods' versions, what to do and what it means, is under
*more about it*. The page explains nothing in hint lines: its tile names
and buttons say what they are for.

#### Map collisions

```text
┌─ Map collisions ──────────────────────────────────────────────────┐
│ ▓▓▓▓░░ 140/212 decided           [Take suggestions] [To patch]    │
├────────────────────┬──────────────────────────────────────────────┤
│ search · kinds     │  Riverwood, by the mill · overlap            │
│ by region | by mods│  ┌────────────────────────────────────────┐  │
│ | by passage       │  │         3D close-up of the spot        │  │
│ ▾ Riverwood (6)    │  │                                        │  │
│   ● mill overlap   │  └────────────────────────────────────────┘  │
│   ○ bridge touch   │  JK's Riverwood ⟷ Riverwood Trader           │
│ ▸ Whiterun (14)    │  [Keep JK's] [Keep Trader's] [Keep both]     │
│ ▸ Falkreath (3)    │  this passage (3 spots) · this pair: 12 ▸    │
│                    │  [◂ prev] [next open ▸]     [show on map]    │
└────────────────────┴──────────────────────────────────────────────┘
```

One worklist and one stage. The worklist groups the spots by *region*, by
*pair of mods* or by *passage* (ADR 0072: where two mods build the same
thing, such as a stable that both rebuilt, decided as one). The stage shows
the spot, interior or exterior, in 3D with what collides tinted, says in a
few words what differs, and offers the choices: keep one mod's version,
keep both, hide single objects or parts of a mod there. *This pair* opens the pair's
panel with all its spots below; a passage can be decided as a whole. The
magic wand picks what belongs together in the 3D view (ADR 0055).
Duels about placed objects are folded into their pair, and the collision
findings without a spot are listed with their pair. The *Mod against mod*
tab of the first plan was removed with ADR 0072.

Beside the choices, a spot carries notes that say why something cannot just
go: tied in, walked on, needed by the game, resting on another object
(ADR 0046). Moved objects and doors stacked on each other (ADR 0060, 0067)
are decided here as well. A learned preference between two mods (ADR 0086)
can take the open spots between them at once; it is offered by Mal.

#### Patch

```text
┌─ Patch ───────────────────────────────────────────────────────────┐
│ Your patches          [New patch] [Load a patch…]                 │
│ ● Patch 1  editing · on in the game · 214 decisions               │
│     This patch hides 214 objects, moves 3 and repairs the paths   │
│     in 2 places.      [Update in Vortex] [Rename] [Switch off]    │
│   JK's Whiterun, shared · by someone · not installed yet          │
├───────────────────────────────────────────────────────────────────┤
│ Decisions of the patch                                            │
│ Places where mods collide · Objects hidden or shown on their own  │
│ Moved objects and merged doors · Repaired paths · File choices    │
│ each decision with [change] [undo]                                │
└───────────────────────────────────────────────────────────────────┘
```

The patch is several patches since ADR 0070. Each is a mod of its own in
the mod manager with its own decisions and says what it decides: it hides
objects (through Base Object Swapper, needed only for that), moves them
through a plugin of overrides, repairs paths, and keeps file choices.
One patch is *being edited*: the decisions made anywhere go into it, and a
chip in the sidebar names it on the pages that decide. A patch somebody
shared can be loaded; Mod Mallet does not change its decisions, but it can
be made one's own. A patch is installed in Vortex, written only as an
archive to share or install by hand, or switched off, which keeps its
decisions. A shared archive carries a FOMOD installer (ADR 0053), so it
installs in any mod manager.

#### Map

```text
┌─ Map ─────────────────────────────────────────────────────────────┐
│ World [ Skyrim ▾ ]   mod [ all ▾ ]   [ flat | relief ]            │
│        ├ The game: Skyrim, Solstheim, Blackreach, Sovngarde, …    │
│        ├ Mods: Falskaar, Blackbone Isle (Creation), Wyrmstooth    │
│        └ Small: 14 more  ▸                                        │
├───────────────────────────────────────────────┬───────────────────┤
│                                               │ Place / cell      │
│        relief or flat map of the world        │ Mods placing here │
│        places as icons, spots as pins         │ Spots here: 2     │
│                                               │ [open in worklist]│
│                                               │ console: cow …    │
└───────────────────────────────────────────────┴───────────────────┘
```

The density colouring of cells goes; the cell grid stays as the way to pick
a cell. A spot on the map opens the worklist at it; the map decides
nothing. The worldspaces are named and grouped as set out in
[Worldspaces](#worldspaces).

#### Asset collisions

```text
┌─ Asset collisions ────────────────────────────────────────────────┐
│ [ Duels 6 | Ranking ]                                             │
├──────────────────┬─────────────────────────────┬──────────────────┤
│ search · open ▾  │  Cut Content ⟷ Face Overhaul│ What they share  │
│ ▾ Faces (2)      │  ┌──────────┐ ┌──────────┐  │  Lydia  face     │
│   ● Lydia ×34    │  │ 3D left  │ │ 3D right │  │  Uthgerd face    │
│ ▾ Textures (3)   │  └──────────┘ └──────────┘  │  …               │
│   ○ Old Town     │  light · turn · reset       │                  │
│ ▸ Meshes (1)     │  [Cut Content wins] [Face Overhaul wins]       │
│                  │  ranking suggests: Face Overhaul               │
└──────────────────┴─────────────────────────────┴──────────────────┘
```

Asset collisions is the comparison view made a page of its own: the duels about
files in the list on the left, the two mods' files side by side on the
stage, the decision under it, and *Ranking* as the second tab. The
duel list is grouped by what the mods compete for (faces, textures,
meshes, and other files). Meshes are shown as pictures drawn in the engine
(ADR 0085); a file that is neither texture nor mesh, such as a script or a
configuration file, is compared side by side as text, by its functions or
by size and date (ADR 0066). A duel opened from elsewhere (Tasks, a mod's details) opens here,
at that duel; the page has no *back* button, since the sidebar is the way
back.

The ranking lives here and not under Mods because it has one use: it
suggests the side of every duel about files, and it is edited where those
duels are decided, so that a change shows at once in the suggestions next
to it. Under Mods it would be a second job on a page whose job is knowing
the mods.

#### Assets

The catalogue of the game's things (ADR 0054), named *Assets* in the
sidebar: characters, weapons, armour and clothes, items and other things
with a model of their own, listed by kind, by material or as the game's
outfits. A thing opens to its versions, from the game and
from each mod, side by side on the same stage as the duels; choosing one
puts it into the game through Vortex's file overrides. The page decides
things, not whole pairs of mods.

#### Mods

```text
┌─ Mods ────────────────────────────────────────────────────────────┐
│ [ List | Dependencies ]                                           │
│ search…   [all|problems|off]   sort ▾                             │
│ ▾ JK's Riverwood   2.1  on   1 problem · 5 open spots   [page]    │
│     Problems: (the same rows as on Tasks)                         │
│     Collides with: Riverwood Trader 5 open  [open the pair]       │
│     Its files against: Ethereal Elven Overhaul   [compare ▸]      │
│     Covered by others: 12 files                                   │
│     Places it adds or changes          [show on map]              │
│ ▸ Riverwood Trader  …                                             │
└───────────────────────────────────────────────────────────────────┘
```

*Compare ▸* opens Asset collisions at the duel; *open the pair* opens the
pair in Map collisions; *show on map* opens the map filtered to the mod.
*Dependencies* draws which plugin needs which as arcs, for one mod or for
the whole setup (ADR 0068).

#### Toolbox

The tools the setup needs, by group: the mod manager, Mod Mallet's link
into it, what the game needs (its script extender), helpers such as 7-Zip,
and the modding tools the mod manager lists, with the state and version of
each and the one button a tool offers. LOOT inside Vortex counts as LOOT,
the Creation Kit is optional with what it is for (see
[The toolbox](#the-toolbox)), and Mod Mallet itself can be added to
Vortex's tools, but only when the user asks.

#### Settings

Four tabs: *General* (language, theme, scanning again by itself), *Places*
(where Mod Mallet reads, which the user confirms first), *Preferences*
(the preferences learned from the decisions, ADR 0086, and which of them
may be offered) and *About* (the version with its build, ADR 0057).

#### Mal the Rubber Mallet

A small helper in the corner of the window (ADR 0086): the logo's rubber
mallet, called Mal. It is not a page and decides nothing. When Mod Mallet
has learned a preference between two mods from the decisions made, Mal
waits with the number of favourites on a badge and makes a short move now
and then, never while a button is held or a key is typed, and not with
reduced motion. Clicked, a panel slides up with one favourite at a time:
the topic, how often it was kept, *Show list* and *Apply*; *Don't suggest
this again* sits in a small menu. Closing puts the panel back into the
mallet. The same mallet is the logo at the top of the sidebar, a toy that
breaks the frame when clicked often enough.

## Fixes along the way

Both fixes below are built (work packages F, G and H). Each section keeps
what was wrong, since that is why the fix is what it is, and says what
became of it.

### Worldspaces

**What happened before the fix.** The map's worlds come with the scan
(`world_map`, `crates/mallet-analysis/src/map.rs`). A worldspace is on the
list when a mod places an object in it or when any plugin has a place of
the world map in it; nothing else decides. It is named by its editor ID
(EDID) alone, and the name is also the key for its LOD, its picture and the
objects up close. The WRLD record's own name (FULL), its parent (WNAM), its
flags and its bounds (MNAM, NAM0/NAM9) are not read. Three things follow:

- **Odd entries.** Worldspaces of the game that only hold a few places (the
  cities' own worldspaces, small dungeon worlds) sit next to Skyrim, under
  names such as `WhiterunWorld` or `DLC2ApocryphaWorld`. Where no EDID was
  collected the name falls back to the record's identity
  (`skyrim.esm:00003C`), which matches no LOD folder. This likely happens
  on every scan after the first: once the game's land is cached, the game's
  plugins are read without their WRLD records (`ingest.rs`), so a
  worldspace gets its EDID only where a mod overrides its record. To be
  confirmed on the reference setup.
- **Partial land.** The relief is cut to the cells where mods place objects,
  plus eight cells around them (`MARGIN`). A world in which mods place a
  few objects, Blackreach for instance, shows a patch of land.
- **The picture of the flat map** is read from loose LOD textures only; the
  game's own are in archives. Worlds without loose LOD files get no
  picture.

**What the map lists** (built; ADR 0048, point 7). Every worldspace that has land (LAND records) or
places, from every plugin, read on every scan (WRLD records are few; the
cache must not drop them). Each gets:

| Field | From | Used for |
|---|---|---|
| `name` | EDID | the key: LOD folders, the console (`cow`), the objects up close |
| `title` | FULL, through the game's string files in the user's language | what the picker and the page show |
| `origin` | the plugin that defines the WRLD: the game and its DLC, Creation Club content, or a mod (named) | the picker's groups |
| `parent` | WNAM, with the flags for using the parent's land and map | a child worldspace (Whiterun inside its walls) is shown on its parent's map, its spots and places drawn there |
| `bounds` | NAM0/NAM9's corners, else MNAM's cells (the part the world map shows, much smaller for Tamriel), else the cells with land | the whole land, not only where mods place things |
| `cells` | count of cells with land | *small* worlds |
| `lod` | whether `meshes/terrain/<name>/` exists, loose or in an archive | relief from LOD, or from the land itself |

**How they are named and grouped.** The picker shows the FULL name and,
muted, the EDID: *Falskaar* `Falskaar`, *Blackbone Isle* (no FULL name:
the EDID made readable, `ccBGSSSE031BlackboneIsle` → *Blackbone Isle*,
by dropping a known prefix and splitting words). Groups, in this order:

1. **The game**: the game's main world first (the largest of the official
   masters, Tamriel for Skyrim), then its DLC worlds (Solstheim,
   Blackreach, the Forgotten Vale, Soul Cairn, Apocrypha, Sovngarde).
2. **Creation Club**: worlds from the Creation Club's plugins (`cc…`), which
   the game lists in its own file of Creation content.
3. **Mods**: worlds a mod adds, each with the mod's name.
4. **Small worlds**, folded: worlds with the *small world* flag or fewer
   than 64 cells of land, unless they hold a collision spot. Child worlds
   that use their parent's land are not listed at all; they are part of
   the parent.

The map opens on the world of the spot or place it was opened for, else on
the game's main world (not on the one with the most collisions).

### Blackreach and other worlds that are not open land

The relief draws every world alike: the land's heights normalised to its
own range, in the green and beige of the CSS map colours, lit from above,
on a transparent background. For Tamriel that reads as land. Blackreach is
a cave floor under a ceiling of objects; drawn alike, and cut to the few
cells where mods place things (see above), it is a small green blot.

The relief became one renderer for every world, without special cases
by name (built; ADR 0037, 0041, 0074, 0085):

1. **The whole land**, from the world's bounds (above).
2. **The game's own colours.** The land is coloured with the world's LOD
   textures, read from archives as well as loose files; the CSS palette is
   only the fallback where a world has none. A cave world then takes the
   colours of its own ground instead of a meadow's.
3. **Water** at the world's default height (DNAM), and none where the
   world's flags say it has no LOD water.
4. **Height** exaggerated by the world's own range: a flat cave floor is
   not stretched to look like mountains. The slider stays.
5. **Without LOD**, the land's own heights and vertex colours (as the
   ground up close of ADR 0041 already reads them) are drawn instead of
   saying that there is no relief.
6. **Child worlds** draw their parent's land.

What stands on the land stays out (the vision's non-goal), except the
game's object LOD that ADR 0037 already draws and the objects up close of
ADR 0038. A cave's ceiling is therefore not drawn; seen from above, that is
what makes the floor visible. The first step of this work is a screenshot of
Blackreach and Falskaar on the reference setup, to check each point above
against what is actually wrong there.

**Found on the reference setup** (checked by reading the files, not by
screenshot). Blackreach has level of detail of its own (85 blocks of
land, in the game's archives) and a picture of its land in the archives;
what was wrong was the land cut to the few cells where mods place things
and the flat picture read only from loose files, which only Tamriel had.
Falskaar, Solstheim, Sovngarde and the other DLC worlds have level of
detail too. Some worlds have none at all: Blackbone Isle (Creation Club),
and mods' worlds such as Arnimatown and Sancre Tor. MNAM, the world map's
cells, is much smaller than the land for Tamriel (-30 to 40 against -57
to 61), so the bounds come from NAM0/NAM9 first.

### The toolbox

**LOOT.** Before the fix LOOT counted as present when `%LOCALAPPDATA%\LOOT` or
`LOOT.exe` existed. Vortex sorts plugins with LOOT's library and keeps the
masterlist at `%APPDATA%\Vortex\<game>\masterlist\masterlist.yaml`, which
Mod Mallet already reads for its rules (ADR 0030). The toolbox shows LOOT as
**ready, built into Vortex** (built) when Vortex is the mod manager and that
masterlist exists, with its date (the masterlist reader already finds it);
standalone LOOT is then not needed and not listed as missing. Without
Vortex, LOOT is optional as today.

**The Creation Kit stays, as optional, with what it is for.** What Mod
Mallet does to a setup it does through its own patches: hiding objects
(ADR 0025), moving them through a plugin of overrides (ADR 0060), and
repairing the paths over hidden objects in the navigation meshes of Skyrim
(ADR 0052, 0058), including the door paths of merged doors (ADR 0088).
None of that needs the Creation Kit. What remains for it:

- **Repairing navigation meshes where Mod Mallet does not**: it only
  writes Skyrim's, and only the repairs of ADR 0052 and 0058. Findings
  about other navigation meshes link to the Creation Kit's row.
- **Looking at a cell as the game builds it**, with its lighting, for
  someone who wants more than the close-up.
- **Making mods of one's own.**

Its row says this, and *not needed to play or to use Mod Mallet*. (The
text of that row in `crates/mallet-messages/locales/*/ui.ftl` still says
that Mod Mallet can only point at the navigation meshes to repair.) The rule
for every tool: the toolbox lists a tool only when the game or a feature of
Mod Mallet needs it, says what for, and folds the optional ones under
*optional*. xEdit is not listed until Mod Mallet writes xEdit scripts.

## Work packages

**All packages, A to H, are done and merged.** They are kept as the record
of how the build was split. Where later decisions reshaped a package, the
last column says so.

The build was split so that packages running at the same time never
touched the same files. Two files are shared by nearly everything and were
handled by rule:

- **Texts.** `i18n.ts` is not extended; each package keeps its new texts in
  a module of its own, as `overview-texts.ts` and `interior-texts.ts` do.
- **Generated files.** A package that changes a contract runs
  `cargo xtask gen` after rebasing; generated files are never merged by
  hand.

| # | Package | Files | Size | After | State |
|---|---|---|---|---|---|
| A | **Frame and overview.** Two levels; sidebar without the game select, with the game card, badges that match their pages, Toolbox and Settings at the bottom, toolbox pill gone; *All games* page (Home's game grid, with a score per game where a scan is kept); *Overview* page (Home's health and tiles plus a tile per page, scan statistics and *what was read*); *Assets* in the sidebar and as a tile, its route showing today's comparison view; new session links (`openPair`, `showModOnMap`; `openCompare` now opens Assets); the old report's code and styles removed. | `App.tsx`, `App.css`, `Actions.tsx`, `Home.tsx` → `AllGames.tsx`, new `Overview.tsx`, new `journey-texts.ts`, new `journey.css`; in `Tasks.tsx` only the removal of the statistics | L | the overview pages merged (ADR 0045) | done; the start page and the overview are in `AllGames.tsx` and `Overview.tsx` (ADR 0048) |
| B | **Collisions.** *Mod against mod* with the duels about placed objects folded into their pair, the findings without a spot as *also*, a pair's duel about files as a link to Assets, *take every suggestion*, open at `pairAt`; group heads no longer decide; on the stage *this pair* and *show on map*; `App.tsx` stops passing `Duels` and `TaskList` as children. | `Collisions.tsx`, new `collisions-texts.ts`, the collisions route in `App.tsx` | M | A | done, then reshaped: the *Mod against mod* tab was removed for passages and the guided review (ADR 0050, 0072) |
| C | **Patch.** Decisions listed by pair with *change* and *undo*; *suggest all* and the per-pair buttons gone; *waiting* line. | `PatchView.tsx`, `patch.ts` | S | A | done, then reshaped: the patch became several patches (ADR 0070) |
| D | **Assets.** The comparison view as a page: no *back* button, the duel list grouped by faces, textures and meshes, only duels about files (those about placed objects belong to Collisions), *Duels* and *Ranking* as its two tabs; a compact duel line for other pages to link with. | `Compare.tsx`, `Duels.tsx`, new `assets-texts.ts` | M | A | done; the page is *Asset collisions*, and *Assets* became the catalogue (ADR 0054) |
| D2 | **Mods.** A mod's links: *open the pair*, *compare ▸* (Assets at the duel), *show on map*; no duel cards of its own. | `ModsView.tsx` | S | A (uses D's duel line if merged, else a plain link) | done; the Mods page also has *Dependencies* (ADR 0068) |
| E | **Task rows.** A collision finding offers *open in the worklist* instead of *hide*; *show on map* only for findings that are not collisions; the step *choose* shows one line to Assets instead of the duel cards. | `Tasks.tsx` (`FindingRow`, `Choices`, `Spots`, `TasksPage`) | S | A | done; the task page is now tiles by category (ADR 0076, 0082) |
| F | **Worldspaces.** WRLD read on every scan (EDID, FULL, WNAM, flags, MNAM, NAM0/NAM9), worlds listed by land or places, `title`, `origin`, `parent`, `bounds`, `small`; terrain over the bounds; picker grouped; density colouring of cells removed; sample worlds (a DLC world, a mod world, a small one, a child). | `crates/mallet-game-bethesda` (`objects.rs`, `ingest.rs`, `terrain.rs`), `crates/mallet-core/src/report.rs`, `crates/mallet-analysis/src/map.rs`, generated, `MapView.tsx`, the world part of `dev/sample-api.ts` | L | – | done (ADR 0048, point 7) |
| G | **Relief for every world.** LOD textures from archives for the flat picture and the relief's colours, water by flags, height by range, relief from the land where there is no LOD, child worlds; Blackreach checked on the reference setup first. | `MapView3D.tsx`, `lod-layer.ts`, `land-layer.ts`, `crates/mallet-game-bethesda/src/lod.rs`, `lod_library.rs` | L | F, and the area terrain session's work on `MapView3D.tsx` merged | done (ADR 0037, 0041, 0074, 0085) |
| H | **Toolbox.** LOOT inside Vortex; the Creation Kit's purpose; optional tools folded; tools in the sample. | `crates/mallet-game-bethesda/src/tools.rs`, `crates/mallet-engine/src/toolbox.rs`, `crates/mallet-core/src/toolbox.rs` (a *provided by* field), `crates/mallet-messages/locales/*/ui.ftl`, `Toolbox.tsx`, the tools part of `dev/sample-api.ts` | S | – | done; the row of the Creation Kit still says Mod Mallet cannot repair navigation meshes (see the toolbox section) |

Order in which they were built:

```text
now:            A ─────────┐        F ──────────▶ G (after area terrain)
                           │        H
after A:        B   C   D   D2   E   (all five at once)
```

A, F and H could start at once: A is the window's frame, F and H are engine
work with their own corner of the window. B to E started when A was merged,
since each needs its links, and ran side by side. G waited for F and for the
session on the ground up close, which also changes `MapView3D.tsx`. F and H
both regenerate contracts and both touch `dev/sample-api.ts` in different
parts; whichever merges second rebases and runs `cargo xtask gen`.

Out of these packages, and left as they are: the first-run page, the
settings, the stage of a spot (beyond its two links), the interiors, the
health formula.
