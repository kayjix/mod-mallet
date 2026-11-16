# 0003 Scope and non-goals

- Status: accepted
- Date: 2026-11-15

## Context

"A tool that fixes modding" has no natural boundary. Without one, the
project would grow into a worse copy of several mature tools at once: a mod
manager, a plugin editor, a level viewer. Each of those is years of work by
people who do nothing else.

## Decision

Mod Mallet does three things: it **diagnoses** a setup, **explains** what it
found, and **recommends** reversible fixes. Everything else is delegated.

It is explicitly not:

- a mod manager: it never downloads, installs, enables or orders mods,
- a plugin editor: complex patches are generated as scripts for xEdit,
- a renderer: there is no 3D view of the game world,
- an automatic navmesh generator,
- a platform for every moddable game.

The full reasoning for users is in
[Vision and non-goals](../product/vision.md).

## Consequences

- Feature requests can be answered by pointing here.
- Mod Mallet depends on other tools being installed for some fixes. It has
  to detect them and say clearly when one is missing.
- The value lies in connecting information that today sits in separate
  tools. That is where effort goes.
