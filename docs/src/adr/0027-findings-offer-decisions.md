# 0027 Findings offer decisions; the window has a home and themes

- Status: accepted
- Date: 2026-11-16

## Context

The task list explained every finding well and still left the user alone:
a summary, a remedy in prose, a long text behind it. Someone who has never
opened a modding tool reads "Cut Content.esp changes 34 faces made for
BruvRamm" and does not know what to do next. What they need is the few
things that can be done, each with what it does, and a button.

Several of those things are changes only the mod manager may make: which
plugin wins where two change the same records, whether a mod is on.
Vortex has both as ordinary, visible, reversible settings: rules in its
plugin list, and the switch of a mod in a profile.

The window also lacked a starting point, and its pages had grown apart:
each drew its own boxes with its own distances.

## Decision

1. **Every finding shows its options side by side**, each with what it
   does and one button: the actions of the finding, hiding one of the two
   sides of a collision (which goes into the patch's choice, shared with
   the patch view), what to do by hand when nothing can be handed over, and
   leaving it as it is (setting the finding aside).
2. **Two new actions** go to the mod manager through the link (ADR 0020):
   `LoadAfter` adds a rule to the plugin list that loads one plugin after
   another and lets Vortex sort again; `DisableMod` switches a mod off in
   the profile in use. The extension (0.5.0) dispatches the same Vortex
   actions that Vortex's own plugin list and mod list dispatch. Analyses
   offer them where they fix the finding: a face plugin that can be loaded
   after the plugin that overrides its characters; a script extender plugin
   or a plugin with missing masters whose mod is known.
3. **A home page** shows the game in use with its state and the games Mod
   Mallet knows. Pictures of games are read at run time from the mod
   manager's installation (Vortex ships one per game); Mod Mallet ships no
   pictures of games, and without Vortex there are none.
4. **One set of building blocks** for every page: panels with a head,
   rows of label and control, tabs, a spacing scale. The settings page is
   built from them, in tabs, and sits at the bottom of the sidebar.
5. **A light and a dark theme**, or whatever Windows uses, chosen in the
   settings.
6. **The map is drawn on the game's own terrain textures** (ADR 0011
   already said so): the LOD tiles of a worldspace are put together into
   one picture, reading only the small mipmap level that is needed. Every
   cell can be picked, not only those with objects of mods; cells are
   coloured by how many mods place objects there, like a heat map.

## Consequences

- An action that changes the load order or switches a mod off still needs
  the user's click in Mod Mallet; Vortex may then ask to deploy.
- Face textures are shown in 2D since ADR 0028. A real preview of a face
  needs the head mesh drawn in 3D, which ADR 0011 excludes; that is a
  decision of its own.
- LOD tiles inside archives are not read yet; a worldspace without loose
  LOD tiles keeps the height relief as its ground.
