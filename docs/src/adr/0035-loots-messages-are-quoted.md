# 0035 LOOT's messages are quoted

- Status: accepted
- Date: 2026-11-16

## Context

ADR 0030 left LOOT's messages out: they are English prose, and findings
carry no prose (ADR 0007). On a real setup with about 400 plugins, the
messages whose conditions hold were one error and fourteen notes. The
error named a missing compatibility patch for the very pair of mods that
Mod Mallet reports with 55 doubled objects; the notes named update patches,
patch collections and a mod that needs a new save. That is knowledge no
other check of Mod Mallet has.

There is a precedent for quoting: a script extender plugin that was not
loaded is reported with the reason quoted from the extender's log.

## Decision

1. The knowledge adapter reads LOOT's messages for each active plugin,
   with their conditions evaluated by libloot, in English, as plain text:
   emphasis is dropped and a link becomes its text with the address in
   brackets.
2. They are reported as two kinds per plugin: `plugins.loot-warning`
   (warning) for LOOT's errors and warnings, `plugins.loot-note` (note)
   for the rest. Both are *hints*. All messages of one level go into one
   finding, so the identity stays one per plugin and kind.
3. The kinds' own texts are translated as always and say that what follows
   is LOOT's, in English.

## Consequences

- Missing compatibility patches and update patches that LOOT knows of
  reach the user, with the page to get them.
- The quoted text is English in every language of Mod Mallet, and it is
  only as current as the masterlist.
- General notes such as "read this mod's compatibility notes" appear as
  notes; they are of little use, but leaving them out would mean judging
  LOOT's prose.
