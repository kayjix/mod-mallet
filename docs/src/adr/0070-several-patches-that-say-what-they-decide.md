# 0070 Several patches, each its own mod, that say what they decide

- Status: accepted, amended 2026-11-22 (buttons moved into the sidebar; a patch is "being edited")
- Date: 2026-11-22
- Changes: [ADR 0053](0053-shareable-patches-in-parts.md) (one patch with
  parts), [ADR 0060](0060-objects-moved-a-little-through-a-plugin.md) and
  [ADR 0058](0058-paths-repaired-on-the-ground-below.md) (one plugin of
  moves, one of paths)

## Context

The patch page showed "1074 objects hidden · installed: 1050 · 24 not in
the game yet" and a long list of pairs of mods such as "Cutting Room Floor ⟷
JK's Nightgate Inn hide cutting room floor.esp's objects · 5 objects · in
the patch". The user said it tells nothing: what was decided is not
visible, everything goes into one patch, and the result shows only in
Vortex. In their words, they want to:

1. make several patches instead of one: a small one for the conflicts of
   two big mods, a big one for their whole setup or a collection; each with
   a name, each installed and switched on or off as its own mod;
2. see what a patch does: its decisions, grouped, each said plainly, and
   go from one to where it was decided, and back;
3. see which mods a patch touches, and which of them they do not have or
   have switched off;
4. switch a patch off without losing its decisions, start an empty one,
   forget everything with a way back, and untick or tick single decisions;
5. load a patch archive, their own or someone else's, back into Mod Mallet
   as decisions they can change and install again.

On overlapping patches the user said: refusing is wrong. Two patches may
decide the same place differently, for example two patches by two
authors. Mod Mallet says which patches disagree and where, and the user
decides. A patch by someone else is never edited: its author's next
version would undo the edit. What settles a disagreement belongs into a
patch of its own, as a compatibility patch does on Nexus.

Base Object Swapper reads every `*_SWAP.ini` in the data folder. Read in
its source: lines for the same placed object from several files are all
kept and sorted so that the file read *last* wins; a line that swaps an
object for its own base object is accepted and shadows every earlier line
for it.

## Decision

### A patch is a set of decisions with a name

- The window keeps a **patch book** per game, in the settings
  (`patch_book`): the patches, in an order, and which one is being edited.
  The old `patch_choices` become the first patch, named "Mod Mallet Patch",
  the first time the book is read.
- A **patch** has an identity that does not change, a name, an author
  (none for one's own), a version, where it came from (made here, or loaded
  from an archive), its **decisions** and which of them are **unticked**.
  Its decisions use the keys the pages already use (ADR 0040, 0050, 0060,
  0058, 0067): a collision finding, a spot, an object hidden on its own, a
  move (which carries a merged door's new target), a repaired navigation
  mesh; and a new one for the file choices of Asset collisions,
  `duel:<duel>` for the side picked in a duel. The versions of things of the
  catalogue (ADR 0054) join later, as `version:<thing>`.
- Pins are no decisions (ADR 0050) and stay outside every patch.
- A patch is **switched on** unless the user switched it off: in the game
  that is its mod, installed and on in Vortex; here, that its decisions
  count. Several patches can be on at once, each its own mod.
- Exactly one of the user's own patches is the one **being edited**: every
  decision made anywhere goes into it. The book keeps it as `active`. The
  patch page marks it ("You are editing this patch"), and on every page
  where something is decided (Map collisions, Go through it, Asset
  collisions, the map) the sidebar shows "Decisions go to: Patch X ▾", with
  a quick switch to another patch or a new one. Making a new patch, or
  duplicating one, makes it the one being edited.
- What the views show is what counts: the ticked decisions of every patch
  that is switched on, first the patches by someone else, then the user's
  own, each in the order of the book, and the patch being edited last,
  whether it is switched on or not. On the same key the later wins, so the
  views always show what the user is editing.
  Every page reads and writes these, as before; nothing on Map collisions,
  Asset collisions or the 3D views needs to know about patches.
- A page's change goes to the patch being edited. A decision taken back on
  a page is taken out of the patch being edited, and unticked in any other
  patch that still makes it, which leaves that patch's decisions as they
  are. The patch page moves or copies decisions between patches.
- **Duplicating** a patch copies its decisions into a new patch of the
  user's own, which becomes the one being edited; the original is switched
  off here, so what the copy changes shows at once. In Vortex the original
  stays until the user switches it off there or installs the copy.
- Unticking keeps the decision in its patch but takes it out of what counts;
  the views show the object again at once.

### Patches that disagree are shown, and settled outside them

- In the game, two patches that are both on and decide the same place
  both apply: Base Object Swapper hides whatever either hides, and of two
  plugins that move or repair the same thing the one loaded later wins.
  Mod Mallet's views show the patch being edited winning instead, so the
  page warns about every such place.
- Two patches that count **disagree** where both decide the same key
  differently, where one hides an object that the other keeps at a spot it
  decided, where both move the same object differently, or where both
  repair the same navigation mesh.
- The patch page lists each pair of patches that disagree, with the
  places, each linked to where it is decided. In the game both patches
  apply; the page says what that does (both objects gone, or the object
  stays hidden).
- Where the losing patch is one of the user's own, a disagreement is
  settled by keeping one side: the losing patch takes the winning decision
  at those places.
- Where a patch is someone else's, it is not changed. The way to settle is
  a **compatibility patch**: an own patch, named after the two, that holds
  the winning decisions and is written so that it wins in the game. Its
  swap files are named to be read after every other (`~` first), and for an
  object the loser hides and the winner keeps it carries a line that swaps
  the object for its own base object, which Base Object Swapper accepts and
  which shadows the loser's line. The base object is read from the setup
  when the patch is written. Until that is built, disagreements with a
  patch by someone else are shown, and settled by switching one of the two
  off.

### A patch repairs the paths over what it hides by itself

- Where a patch hides something characters walk on, the paths there are
  repaired when the patch is written, as in ADR 0058, into its own plugin
  of paths. The user does not have to ask for it.
- The places are the cells of the objects it hides (or the interior
  cells), as the spots of the scan place them. In each, only a mesh whose
  version the game uses runs over a hidden object is changed (the checks of
  ADR 0051 and 0052); where the mod that wins already has a path around the
  object, nothing is written there.
- The patch page lists these places under "Paths repaired by the patch
  itself", each with a tick: unticking keeps the paths of that place as
  they are, kept in the patch as `norepair:<place>`. Repairs the user chose
  by hand in a 3D view (`repair:`) stay as they were.
- A mesh from another mod that is not the one the game uses is not taken
  over in place of a repair yet: that would copy a whole mesh of another
  mod, whose links to its neighbours this setup may not match. It is a
  later step.

### One patch is one archive is one mod

- Each patch is written as one archive, `<name>.zip`, into Mod Mallet's
  folder, and installed through the link as its own mod (ADR 0020, 0053).
  Vortex names the mod after the archive; Mod Mallet finds the mod of a
  patch again by that name among the mods of the report.
- The archive holds:
  - the swap files of its parts with the installer, as in ADR 0053;
  - when it moves objects or merges doors, the plugin `<name> - Mod Mallet Moves.esp`,
    and when it repairs paths, `<name> - Mod Mallet Paths.esp`, each light where it
    fits, as files the installer always installs. A patch is switched on or
    off with everything it does;
    Mod Mallet knows its own plugins of moves and paths by these endings, and
    a new one never copies from them;
  - its **manifest**, `mod-mallet-patch.json`, at the top of the archive,
    which the installer does not copy into the game.
- The old separate mods "Mod Mallet - Moves" and "Mod Mallet - Paths"
  are not written any more. Their moves and repairs became part of the first
  patch; once that patch is installed, the page offers to switch the old
  mods off, so that nothing is moved twice.
- **Installing**, **switching off** and **switching on** are actions on the
  user's click only: `hide-objects` with the patch, and `disable-mod` /
  `enable-mod` with the patch's mod. Switching off also stops the patch
  from counting, and switching on makes it count again; its decisions stay.
- A file choice cannot travel in an archive: it is a rule in the mod
  manager, and a patch carries no files of others (ADR 0053). It is kept in
  the manifest and listed; Mod Mallet applies it as it does today. Installed
  without Mod Mallet, it does nothing, and the page says so.

### The manifest carries everything to rebuild the decisions

- The manifest is JSON with a format number, `format: 1`: the patch's
  name, author and version, its ticked decisions by key, and the plugins and
  mods it touches. A reader refuses a format it does not know and says so.
- **Loading a patch**: the user picks an archive; Mod Mallet reads its
  manifest and adds the patch to the book as it is, with its author. One of
  the user's own comes back as their own. An archive without a manifest
  (written before this decision, or by hand) is read from its swap files:
  each hidden object becomes an object hidden on its own. Nothing is
  installed by loading.
- A loaded patch by someone else counts like any other; it is changed only
  by copying it into an own one ("make it mine").

### What the patch page shows

- On the left, the patches: name, author, whether it is in the game,
  switched off or not installed yet, how many decisions, the one being
  edited marked; "New patch" and "Load a patch".
- On the right, the chosen patch:
  - what it does in one sentence, its state in the game, and its buttons:
    install or update, switch off or on, edit this patch, duplicate,
    rename, delete;
  - the mods it touches, each with its number of decisions, marking those
    not installed and those switched off (ADR 0061): "You don't have
    Immersive Citizens; its 84 decisions in this patch do nothing and need no
    decision.";
  - its decisions, grouped: places where mods collide (by pair of mods),
    objects hidden on their own, moved objects and merged doors, repaired
    paths, file choices. Each says what was decided in plain words ("JK's
    Nightgate Inn kept, Cutting Room Floor's lantern post hidden"), has a
    tick, a menu to move or copy it to another patch, and leads to where it
    was decided: the spot in Map collisions, the duel in Asset collisions,
    the object on the map. The page draws no 3D;
  - where it disagrees with other patches;
  - the report per pair (ADR 0050), waiting decisions of switched-off mods
    (ADR 0061), and "forget all decisions" with its question and its undo.

### Back to where one came from

- The window keeps a **history** of where the user went: the page and
  what it opened at (a spot, a pair, a duel, a mod, a patch and its entry).
  Back and forward buttons sit in the sidebar, just above the toolbox; the
  mouse's back and forward buttons and Alt+← / Alt+→ do the same.
- A page opened from an entry of a patch shows "← Back to patch «name»:
  entry" below them, which returns to the patch with the entry in view and
  marked.
- *Amended 2026-11-22:* the buttons first sat in a bar of their own at the
  top of every page. That bar pushed every page down and took height from
  the 3D view in Map collisions, so they moved into the sidebar, where they
  take no height from any page.

### Amended later on 2026-11-22

- *Amended 2026-11-22, later:* "active" left the user unsure which patch
  they were editing after making a new one. It is now "being edited", shown
  on the patch page and in the sidebar of every page where something is
  decided; changes always go to it; patches can be duplicated; and a patch
  repairs the paths over what it hides by itself.

## Consequences

- A patch can be made for two mods and shared, or for a whole collection,
  and the user's own setup can carry several side by side.
- Each patch with moves or paths brings its own plugins; light plugins cost
  no slot of the load order where they fit.
- Two patches that repair the same navigation mesh both bring a copy of it;
  the later plugin wins. The page lists it as a disagreement.
- Paths are repaired per patch, around what that patch hides; a path over
  objects hidden by two patches is repaired by neither alone. The page says
  where a repair depends on another patch.
- The settings grow by the patch book; `patch_choices` stays readable for
  the move to the book and is no longer written.

## Alternatives considered

- **A decision in one patch only.** Simple, but two authors' patches for
  overlapping mods could then not both be loaded, and the user said not to
  refuse.
- **Editing the other author's patch to settle a disagreement.** The
  author's next version would undo it.
- **One plugin of moves and one of paths for all patches.** Fewer plugins,
  but a patch could not be switched off with what it moves, and a shared
  patch would not carry its moves.
- **A history only for the patch page.** The user goes from the patch to
  Map collisions to the map and wants to get back the whole way; a history
  of the window serves every page.
