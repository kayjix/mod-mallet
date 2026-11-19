# Vision and non-goals

## In one sentence

Mod Mallet diagnoses a modded setup, explains the problems in plain language,
and proposes reversible fixes that existing tools carry out.

## Who it is for

1. **People who install mods through Vortex**, often as large collections,
   and who have never opened xEdit. They notice that something looks wrong
   but have no way to find out what.
2. **Authors of collections**, who need to know that the mods they bundle
   work together before they publish.

People who already resolve conflicts by hand in Mod Organizer 2 and xEdit
have their tools. Mod Mallet should be useful to them, but it is not designed
around them.

## Principles

- **Explain, do not just resolve.** A mod manager decides which file wins. It
  rarely says why, or what was lost. Every finding states what happened, why
  it matters in the game, and what to do.
- **Recommend with stated confidence.** A finding is *certain* (a fact read
  from the setup), *likely* (a heuristic) or a *hint* (knowledge that may not
  apply). Only certain findings may ever be fixed without asking, and only
  reversibly.
- **Reuse before building.** Vortex installs, LOOT knows compatibility,
  xEdit writes patches, the Creation Kit and the game itself render. Mod
  Mallet connects them.
- **Use the data that is already there.** A mod's own files, the mod
  manager's metadata and community-maintained lists come first. Machine
  learning is a last resort for free text nobody has structured, and its
  output is always marked as a hint.
- **Never touch the original mods.** Everything Mod Mallet produces lives in
  a separate output that can be switched off.
- **Local and private.** No account, no telemetry, no network access the user
  did not ask for.

## Non-goals

Stating these is as important as the goals: each one is a feature request
that will be declined.

- **Not a mod manager.** Mod Mallet does not download, install, enable or
  order mods. It tells the mod manager what to change.
- **Not a plugin editor.** It does not try to be xEdit. Complex patches are
  produced as xEdit scripts.
- **Not a renderer of the world.** There is no 3D view of the game world.
  Single things a finding is about, a mesh or a head, are shown turning in
  3D (ADR 0029, 0033). The map can show the shape of the land as a relief,
  with the places of the game and of mods standing on it (ADR 0031), but
  nothing that stands on the land. For looking at a place there is the
  game, reached by a console command or, later, an in-game bridge.
- **Not an automatic navmesh generator.** Navmesh conflicts are detected and
  explained. Paths over objects Mod Mallet hides are cut out of the meshes
  (ADR 0052), never laid anew: laying new paths stays manual work in the
  Creation Kit.
- **Not a universal modding platform.** The design targets the Bethesda
  family of games. Other games are not excluded, but nothing is built for
  them on speculation.

## What success looks like

Someone installs a few hundred mods, runs one scan, and gets a list short
enough to read: these five things are broken, this is why, and this is what
to do about each. After working through it the game looks and behaves the way
the mods intended.
