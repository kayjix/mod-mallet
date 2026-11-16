# 0005 Scans read, actions are reversible

- Status: accepted
- Date: 2026-11-15

## Context

People will point Mod Mallet at setups that took them weeks to build. A tool
that might damage such a setup will not be trusted, and should not be. At
the same time, a tool that only complains and never helps fix anything is of
limited use.

## Decision

1. **A scan only reads.** It changes nothing in the game directory, in any
   mod, or in the mod manager's data. A test compares the game directory
   before and after a scan.
2. **Original mod files are never modified**, by any feature.
3. **Every change is an action.** An action is a described, previewable step
   that the user starts. Its result lands in an output of Mod Mallet's own,
   for example a patch plugin in a separate mod that the mod manager
   handles, and can be removed again.
4. **Changes go through the tool that owns the thing changed.** File
   priorities are changed through the mod manager, not by moving files
   behind its back.
5. Only findings with confidence *certain* may ever be fixed without asking,
   and only by an action that is trivially undone.

## Consequences

- Trying Mod Mallet is safe, which matters for a new tool.
- Fixes need integrations: a Vortex extension for file decisions, xEdit for
  complex patches. Those are more work than writing files directly.
- An action can be saved as a plan and shared, for example with a collection.
