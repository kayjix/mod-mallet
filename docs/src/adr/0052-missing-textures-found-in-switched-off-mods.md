# 0052 Missing textures found in switched-off mods

- Status: accepted
- Date: 2026-11-19
- Changes: ADR 0049 (the files every mod brings)

## Context

The finding that meshes of a mod name textures the game does not have
(`meshes.texture-missing`) named five of them and told the user to look on
the mod's page. On the user's setup it said that 56 textures were missing
for 42 meshes, and the user could do nothing with it: the five examples
were not enough to search for, and the only button opened the mod's page.

A common cause can be checked from what Mod Mallet already reads: the
textures sit in a mod that is installed in Vortex but switched off, a
texture download installed as a mod of its own and never enabled, or an
older version kept switched off. Vortex keeps such a mod's folder in its
staging folder like any other. ADR 0049 lists only the folders of mods
that have deployed files, so these were never seen.

## Decision

1. **The folders of switched-off mods are listed too.** After the mod
   manager's state is read, the engine lists the staging folders of the
   mods that are installed but not enabled in the profile in use, in the
   same way and with the same cache as ADR 0049. Only folder listings are
   read.
2. **A new question to the mod manager** (ADR 0009):
   `ModSource::switched_off_with(path)`, the switched-off mods that bring a
   file at that path. Managers that cannot say answer with none.
3. **A new action, `enable-mod`**, asks Vortex through the link to switch
   a mod on in the profile in use. The link already does this for
   `disable-mod`; switching the mod off again undoes it.
4. **The finding grows.** It keeps every missing texture with the meshes
   that use it (`list`, up to 500), counts those a switched-off mod brings
   (`offCount`), names those mods (`offMods`), and offers to switch on up
   to three of them, the one that brings the most first. Its summary says
   so in a sentence. The kind stays a hint; the window lists a finding
   that offers to switch a mod on with the one-click tasks, and shows the
   full list under *more about it*, with a button to copy it.

## Consequences

- Where a texture download was installed but never switched on, the
  finding names it and switching it on is one click.
- The scan lists a few more folders. Switched-off mods are usually few;
  their listings are cached like the others.
- Files a switched-off mod brings are not counted anywhere else: they do
  not reach the game, so the suggestions about whole mods (ADR 0043) do
  not see them.
- What the finding still cannot tell: whether any plugin uses the meshes
  at all, and whether the textures are an installer option that was not
  ticked. Both need data the scan does not read yet.

## Alternatives considered

- **Count switched-off mods' files among the staged files of ADR 0049.**
  Simpler, but the suggestion that a mod is mostly overridden would then
  judge mods that are not in the game.
- **Raise the finding's confidence when a switched-off mod brings the
  textures.** Confidence belongs to the kind (ADR 0007); the window's
  ordering of tasks is where the difference shows.
