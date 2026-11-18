# The journey through the app

The desktop app grew one view at a time, each added for a problem found in
a real setup. By November 2026 the same things were shown and decided in
several places: a collision could be decided on four pages, the duels
appeared on three, the scan button and the game picker twice. This page
takes stock and sets the structure the app grows into: one job per page,
at most two ways to do something on it, and links instead of copies.
The decisions are recorded in
[ADR 0048](../adr/0048-one-job-per-page.md). On the same day the user
added a page for the comparison of files, *Assets*; this page describes
the plan with it.

The page describes the window as it stands on `main` at `fa8c395` plus the
work merged with it on the same day: the health score, the task page by
steps and the new mods page (ADR 0045), interiors in 3D (ADR 0042), and
the scan button at the top of the sidebar.

## What there is today

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
All games ──▶ Overview of a game ──▶ Tasks
                      │             ├─▶ Collisions  (Spots | Mod against mod)
                      │             ├─▶ Patch
                      │             ├─▶ Map
                      │             ├─▶ Assets      (Duels | Ranking)
                      │             └─▶ Mods
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
│ ☑ Tasks              12  │  ← open tasks (ADR 0045 counts.tasks)
│ ⧉ Collisions         37  │  ← open spots
│ ✎ Patch               ●  │  ← dot: decided but not in the patch
│ ⌖ Map                    │
│ ▨ Assets              4  │  ← duels about files without a side picked
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
  Tasks the open tasks, Collisions the open spots, Patch a dot when
  choices are not yet in the installed patch, Assets the duels about files
  with no side picked, Toolbox the tools needing attention. The toolbox pill goes; the manager line stays.
- Before a scan, every game page shows the same empty state with the scan
  button; before the places are confirmed, only Settings and the first-run
  page are open (as today).

### One job per page

| Page | Its one job | The ways to do it (at most two) | It does not |
|---|---|---|---|
| **Start** (all games) | Choose what to do and which game | the game in use with its state and the way to what is open; a card per game to switch | list findings, decide anything |
| **Overview** | Say how the game stands and where to go | the health with its reasons; a tile per page | list findings, decide anything |
| **Tasks** | Work through what is to do, except collisions and duels | by step | list collisions or duels (one line each leads to them) |
| **Collisions** | Decide every collision spot | *Spots*: one spot at a time, in 3D; *Mod against mod*: a pair as a whole | build the patch |
| **Patch** | Build and install what was decided | the list of decisions (undo, or *change* in Collisions); create and install | make new decisions |
| **Map** | Look at the places of any worldspace | flat or relief | decide; a spot links to the worklist |
| **Assets** | Decide whose textures, meshes and faces the game shows where two mods bring their own | *Duels*: two mods side by side on the comparison stage; *Ranking*: which mods to prefer, which suggests every duel's side | decide placed objects (Collisions does) |
| **Mods** | Know the mods, one at a time | the list; a mod's details | decide collisions or duels in place |
| **Toolbox** | Have the tools the setup needs | the list; the one button a tool offers | – |
| **Settings** | Where Mod Mallet reads, language, theme | tabs | – |

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
| Collision spot | Collisions · Spots | counts (sidebar, overview, Tasks' collisions line, a mod's details); a pin on the map with *open in the worklist* |
| Pair of mods whose objects collide | Collisions · Mod against mod | a line in each mod's details with *open the pair* |
| Duel about placed objects | merged into its pair on *Mod against mod* | – |
| Duel about files (textures, meshes, faces) | Assets · Duels | a count on the overview and in Tasks' *choose* step, a line in each mod's details, each with *compare*, which opens Assets at the duel |
| The comparison stage | Assets · Duels | – |
| Ranking of mods | Assets · Ranking | the reason given under a duel's decision |
| Patch choices | made in Collisions; listed in Patch | the dot on Patch; *waiting* spots in the worklist |
| Places and worldspaces | Map | the place named on a spot |
| Tools | Toolbox | the manager line; a finding's *needs tool* link |
| Scan statistics, *what was read* | Overview, folded at the bottom | – |

The rule for the collision decisions in particular:

- **A spot, its passage, or one object** is decided on the stage of
  *Spots*.
- **A pair as a whole** is decided on *Mod against mod*. The stage offers
  *decide all spots of this pair* as a link that opens the pair there; the
  worklist's group heads no longer decide.
- **A duel about placed objects is the pair.** Its side picks become the
  pair's verdict; the duel card on that tab goes.
- **Patch** shows what was decided, grouped by pair, with *undo* and
  *change*, which opens the pair or the spot in Collisions. *Suggest all*
  moves to *Mod against mod* as *take every suggestion*. *Clear* stays on
  Patch, since it empties the patch.
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

The overview is Home's health panel and tiles of ADR 0045, moved under the
game, with a tile per page. Each tile is one button to its page; nothing on
the overview is decided.

#### Tasks

As ADR 0045 sets it out, with three changes: the scan statistics and
*what was read* move to the overview; a collision finding never appears
here (it already does not); the step *choose* no longer holds the duel
cards but one line, like the collisions' line: how many duels about files
are still open, and the way to Assets.

#### Collisions

```text
┌─ Collisions ──────────────────────────────────────────────────────┐
│ [ Spots 212 | Mod against mod 31 ]   ▓▓▓▓░░ 140/212   [To patch]  │
├────────────────────┬──────────────────────────────────────────────┤
│ search · kinds     │  Riverwood, by the mill · overlap            │
│ by region | by mods│  ┌────────────────────────────────────────┐  │
│ ▾ Riverwood (6)    │  │         3D close-up of the spot        │  │
│   ● mill overlap   │  │                                        │  │
│   ○ bridge touch   │  └────────────────────────────────────────┘  │
│ ▸ Whiterun (14)    │  JK's Riverwood ⟷ Riverwood Trader          │
│ ▸ Falkreath (3)    │  [Hide JK's] [Hide Trader's] [Keep both]     │
│                    │  this passage (3 spots) · this pair: 12 ▸    │
│                    │  [◂ prev] [next open ▸]     [show on map]    │
└────────────────────┴──────────────────────────────────────────────┘

┌─ Collisions · Mod against mod ────────────────────────────────────┐
│ [take every suggestion]                              sort: open ▾ │
│ JK's Riverwood ⟷ Riverwood Trader   12 spots  5 open  2 dup 3 ovl │
│    suggested: hide Trader's (smaller, later)                      │
│    [Hide JK's] [Hide Trader's] [Keep both]            [Spots ▸]   │
│    also: 1 object of the game moved by both (finding, no spot)    │
│    their textures differ too: 1 duel                   [Assets ▸] │
│ Cutting Room Floor ⟷ Lanterns of Skyrim   …                       │
└───────────────────────────────────────────────────────────────────┘
```

*Spots* is the visual way, *Mod against mod* the technical one (*fachlich*).
*Spots ▸* opens *Spots* grouped by mods at that pair; *this pair: 12 ▸* on
the stage opens the pair on *Mod against mod*. *Show on map* opens the map
at the spot. The duels about placed objects are folded into their pair
(the spots show them in 3D), and the collision findings without a spot are
listed under their pair as *also*. Where the same two mods also have a duel
about files, the pair says so and leads to Assets.

#### Patch

```text
┌─ Patch ───────────────────────────────────────────────────────────┐
│ 214 objects hidden · installed: 190 · 24 not in the game yet      │
│ [ Create ] [ Install through Vortex ]   [Clear]   [open folder]   │
├───────────────────────────────────────────────────────────────────┤
│ Decided                                                           │
│ JK's Riverwood ⟷ Riverwood Trader  hide Trader's  12 objects      │
│     4 spots decided on their own              [change] [undo]     │
│ Single objects hidden (3)                       [show again]      │
│ Waiting for a decision: 72 spots in 18 pairs   [to Collisions]    │
└───────────────────────────────────────────────────────────────────┘
```

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

#### Assets

```text
┌─ Assets ──────────────────────────────────────────────────────────┐
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

Assets is today's comparison view made a page of its own: the duels about
files in the list on the left, the two mods' files side by side on the
stage, the decision under it, and *Ranking* as the second tab. The
duel list is grouped by what the mods compete for (faces, textures,
meshes). A duel opened from elsewhere (Tasks, a mod's details) opens here,
at that duel; the page has no *back* button, since the sidebar is the way
back.

The ranking lives here and not under Mods because it has one use: it
suggests the side of every duel about files, and it is edited where those
duels are decided, so that a change shows at once in the suggestions next
to it. Under Mods it would be a second job on a page whose job is knowing
the mods.

#### Mods

```text
┌─ Mods ────────────────────────────────────────────────────────────┐
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

*Compare ▸* opens Assets at the duel; *open the pair* opens the pair on
*Mod against mod*; *show on map* opens the map filtered to the mod.

#### Toolbox

Unchanged in shape. What changes is what it knows (see
[The toolbox](#the-toolbox)): LOOT inside Vortex counts as LOOT, and the
Creation Kit is optional, with what it is for.

## Fixes along the way

### Worldspaces

**What happens today.** The map's worlds come with the scan
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

**What the map lists.** Every worldspace that has land (LAND records) or
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

The relief becomes one renderer for every world, without special cases
by name:

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

**LOOT.** Today LOOT counts as present when `%LOCALAPPDATA%\LOOT` or
`LOOT.exe` exists. Vortex sorts plugins with LOOT's library and keeps the
masterlist at `%APPDATA%\Vortex\<game>\masterlist\masterlist.yaml`, which
Mod Mallet already reads for its rules (ADR 0030). The toolbox shows LOOT as
**ready, built into Vortex** when Vortex is the mod manager and that
masterlist exists, with its date (the masterlist reader already finds it);
standalone LOOT is then not needed and not listed as missing. Without
Vortex, LOOT is optional as today.

**The Creation Kit stays, as optional, with what it is for.** What Mod
Mallet does to a setup it does through its own patch: hiding objects now
(ADR 0025), moving them when that is built. None of that needs the
Creation Kit. What remains for it:

- **Repairing navigation meshes**, which the vision leaves as manual work.
  Findings about navigation meshes link to the Creation Kit's row.
- **Looking at a cell as the game builds it**, with its lighting, for
  someone who wants more than the close-up.
- **Making mods of one's own.**

Its row says this, and *not needed to play or to use Mod Mallet*. The rule
for every tool: the toolbox lists a tool only when the game or a feature of
Mod Mallet needs it, says what for, and folds the optional ones under
*optional*. xEdit is not listed until Mod Mallet writes xEdit scripts.

## Work packages

The build is split so that packages running at the same time never touch
the same files. Two files are shared by nearly everything and are handled
by rule:

- **Texts.** `i18n.ts` is not extended; each package keeps its new texts in
  a module of its own, as `overview-texts.ts` and `interior-texts.ts` do.
- **Generated files.** A package that changes a contract runs
  `cargo xtask gen` after rebasing; generated files are never merged by
  hand.

| # | Package | Files | Size | After |
|---|---|---|---|---|
| A | **Frame and overview.** Two levels; sidebar without the game select, with the game card, badges that match their pages, Toolbox and Settings at the bottom, toolbox pill gone; *All games* page (Home's game grid, with a score per game where a scan is kept); *Overview* page (Home's health and tiles plus a tile per page, scan statistics and *what was read*); *Assets* in the sidebar and as a tile, its route showing today's comparison view; new session links (`openPair`, `showModOnMap`; `openCompare` now opens Assets); the old report's code and styles removed. | `App.tsx`, `App.css`, `Actions.tsx`, `Home.tsx` → `AllGames.tsx`, new `Overview.tsx`, new `journey-texts.ts`, new `journey.css`; in `Tasks.tsx` only the removal of the statistics | L | the overview pages merged (ADR 0045) |
| B | **Collisions.** *Mod against mod* with the duels about placed objects folded into their pair, the findings without a spot as *also*, a pair's duel about files as a link to Assets, *take every suggestion*, open at `pairAt`; group heads no longer decide; on the stage *this pair* and *show on map*; `App.tsx` stops passing `Duels` and `TaskList` as children. | `Collisions.tsx`, new `collisions-texts.ts`, the collisions route in `App.tsx` | M | A |
| C | **Patch.** Decisions listed by pair with *change* and *undo*; *suggest all* and the per-pair buttons gone; *waiting* line. | `PatchView.tsx`, `patch.ts` | S | A |
| D | **Assets.** The comparison view as a page: no *back* button, the duel list grouped by faces, textures and meshes, only duels about files (those about placed objects belong to Collisions), *Duels* and *Ranking* as its two tabs; a compact duel line for other pages to link with. | `Compare.tsx`, `Duels.tsx`, new `assets-texts.ts` | M | A |
| D2 | **Mods.** A mod's links: *open the pair*, *compare ▸* (Assets at the duel), *show on map*; no duel cards of its own. | `ModsView.tsx` | S | A (uses D's duel line if merged, else a plain link) |
| E | **Task rows.** A collision finding offers *open in the worklist* instead of *hide*; *show on map* only for findings that are not collisions; the step *choose* shows one line to Assets instead of the duel cards. | `Tasks.tsx` (`FindingRow`, `Choices`, `Spots`, `TasksPage`) | S | A |
| F | **Worldspaces.** WRLD read on every scan (EDID, FULL, WNAM, flags, MNAM, NAM0/NAM9), worlds listed by land or places, `title`, `origin`, `parent`, `bounds`, `small`; terrain over the bounds; picker grouped; density colouring of cells removed; sample worlds (a DLC world, a mod world, a small one, a child). | `crates/mallet-game-bethesda` (`objects.rs`, `ingest.rs`, `terrain.rs`), `crates/mallet-core/src/report.rs`, `crates/mallet-analysis/src/map.rs`, generated, `MapView.tsx`, the world part of `dev/sample-api.ts` | L | – |
| G | **Relief for every world.** LOD textures from archives for the flat picture and the relief's colours, water by flags, height by range, relief from the land where there is no LOD, child worlds; Blackreach checked on the reference setup first. | `MapView3D.tsx`, `lod-layer.ts`, `land-layer.ts`, `crates/mallet-game-bethesda/src/lod.rs`, `lod_library.rs` | L | F, and the area terrain session's work on `MapView3D.tsx` merged |
| H | **Toolbox.** LOOT inside Vortex; the Creation Kit's purpose; optional tools folded; tools in the sample. | `crates/mallet-game-bethesda/src/tools.rs`, `crates/mallet-engine/src/toolbox.rs`, `crates/mallet-core/src/toolbox.rs` (a *provided by* field), `crates/mallet-messages/locales/*/ui.ftl`, `Toolbox.tsx`, the tools part of `dev/sample-api.ts` | S | – |

Order:

```text
now:            A ─────────┐        F ──────────▶ G (after area terrain)
                           │        H
after A:        B   C   D   D2   E   (all five at once)
```

A, F and H can start at once: A is the window's frame, F and H are engine
work with their own corner of the window. B to E start when A is merged,
since each needs its links, and run side by side. G waits for F and for the
session on the ground up close, which also changes `MapView3D.tsx`. F and H
both regenerate contracts and both touch `dev/sample-api.ts` in different
parts; whichever merges second rebases and runs `cargo xtask gen`.

Out of these packages, and left as they are: the first-run page, the
settings, the stage of a spot (beyond its two links), the interiors, the
health formula.
