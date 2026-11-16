# Mod Mallet

Mod Mallet diagnoses a modded game setup, explains in plain language what is
wrong, and proposes fixes that can be undone. It does not replace the tools
modders already use. It reads what they produce and, where something has to
change, hands the work back to them.

The first supported game is The Elder Scrolls V: Skyrim Special Edition, with
Vortex as the mod manager.

> **Status: pre-alpha.** Mod Mallet reports script extender plugins that the
> game refused to load, characters whose face files do not match the record
> that wins, installer options that fit the setup now but were never
> installed, replacement meshes that a texture mod no longer reaches, and
> files in a mod's archive that the game takes from somewhere else. Each finding names the mods involved. Everything else on the
> [roadmap](product/roadmap.md) is still to be built.

## Why it exists

A large mod list can start without a crash and still be broken: features
missing, buildings standing inside each other, grass floating above the
ground. The causes are spread over several layers that no single tool shows
together:

- which **file** wins when two mods ship the same one, including files
  packed into archives,
- which **plugin record** wins when two mods change the same thing,
- what a mesh actually **references**, for example textures another mod was
  supposed to replace,
- where two mods **place objects** in the same spot,
- which **installer options** would apply now but did not when the mod was
  installed,
- whether the **setup** itself fits: game version, script extender plugins,
  generated files that are out of date.

Mod Mallet looks at all of these and turns what it sees into a short list of
findings, each with a reason and a suggestion.

## How to read this book

- **Product** says what Mod Mallet is for and what it deliberately is not.
- **Architecture** explains how the code is organised and records every
  decision that shaped it, with its reasons.
- **Reference** is generated from the code and always matches it.
- **Contributing** gets you from a fresh checkout to a passing build.
