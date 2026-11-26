# Vision and non-goals

## In one sentence

Mod Mallet diagnoses a modded setup, explains the problems in plain language,
lets the user decide each one where they can see it, and carries out those
decisions reversibly through the mod manager.

## Who it is for

1. **People who install mods through Vortex**, often as large collections,
   and who have never opened xEdit or the Creation Kit. They notice that
   something looks wrong but have no way to find out what.
2. **Authors of collections and people who help others**, who need to know
   that the mods they bundle work together, and who want to hand on a patch
   or a report that someone else can use.

People who already resolve conflicts by hand in Mod Organizer 2 and xEdit
have their tools. Mod Mallet reads Mod Organizer 2 as well and should be
useful to them, but it is designed around Vortex first.

## Principles

- **Explain, do not just resolve.** A mod manager decides which file wins. It
  rarely says why, or what was lost. Every finding states what happened, why
  it matters in the game, and what to do.
- **Show, then ask.** Where a problem has a place, the user sees it: the
  meshes side by side, the spot on the map, the objects of a place in 3D.
  The decision is made there, with the consequences said in one line before
  it is taken.
- **The user decides; Mod Mallet remembers.** A decision is made once and
  holds on every page that shows the same thing. It can be taken back at
  any time, and every decision Mod Mallet carries out can be switched off
  again in the mod manager.
- **Recommend with stated confidence.** A finding is *certain* (a fact read
  from the setup), *likely* (a heuristic) or a *hint* (knowledge that may not
  apply). Suggestions are offered, never taken silently: nothing is changed
  in a setup without the user's click.
- **Reuse before building.** Vortex installs, LOOT knows compatibility,
  Base Object Swapper hides objects, xEdit and the Creation Kit edit
  anything. Mod Mallet connects them, and builds something of its own only
  where no existing tool can be driven from outside for the job, as for the
  few plugins it writes.
- **Use the data that is already there.** A mod's own files, the mod
  manager's metadata and community-maintained lists come first. What Mod
  Mallet learns about the user's taste comes from the user's own decisions,
  stays on the machine and is offered as a suggestion.
- **Never touch the original mods.** A scan only reads. Everything Mod
  Mallet produces lives in an output of its own that the mod manager
  installs as a mod and that can be switched off.
- **Local and private.** No account, no telemetry, no network access the user
  did not ask for. What is shared with others is shared on purpose, with
  private paths taken out.
- **Plain words, a little charm.** Texts address someone who has never
  opened a modding tool. The app may be friendly (Mal the Rubber Mallet
  waits in a corner), but it never stands in the way of the work.

## Non-goals

Stating these is as important as the goals: each one is a feature request
that will be declined. The lines are drawn where Mod Mallet's own work ends,
not where it would merely be inconvenient.

- **Not a mod manager.** Mod Mallet does not download, install, enable or
  order mods itself. It asks the mod manager to: switch a mod or plugin on
  or off, load one after another, install Mod Mallet's own patch. Under a
  mod manager it cannot talk to, these become instructions for the user.
- **Not a plugin editor.** Mod Mallet writes only plugins of its own, each
  for one narrow purpose that follows from a decision made in its views:
  objects moved, paths repaired around what it hid or moved, doors merged.
  It never changes another mod's plugin and offers no free editing of
  records. Anything beyond that is xEdit's or the Creation Kit's work.
- **Not a game viewer.** Mod Mallet draws a place to decide about it: the
  objects mods put there, the land and the distant view around it, the
  paths characters walk. It does not try to look like the game: no
  characters, effects, weather or the game's lighting. How a place really
  looks is seen in the game, reached by a console command or, later, an
  in-game bridge.
- **Not a navmesh generator.** Paths are repaired only where Mod Mallet's
  own decisions broke them: cut around a hidden object, joined across the
  hole it leaves, re-attached to a kept door. It does not lay paths for new
  land or a new building, and it does not finalize navigation meshes the way
  the Creation Kit does.
- **Not a universal modding platform.** The design targets the Bethesda
  family of games. Other games are not excluded, but nothing is built for
  them on speculation.

## What success looks like

Someone installs a few hundred mods, runs one scan, and gets a list short
enough to read: these five things are broken, this is why, and this is what
to do about each. Where two mods build in the same place, they look at it,
pick what stays, and Mod Mallet's patch carries that into the game. After
working through it the game looks and behaves the way the mods intended,
and every step can be undone.
