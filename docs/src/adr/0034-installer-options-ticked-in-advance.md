# 0034 Missing installer options are ticked in advance

- Status: accepted
- Date: 2026-11-16

## Context

ADR 0020 lets the user reinstall a mod from a finding whose installer
would now tick options that were not installed. On a real setup Vortex
opened the installer with the choices of the last installation, and the
missing option, hidden on one page among dozens, stayed unticked twice.
The guide in Mod Mallet names the page, group and option, but finding it
in a long installer is tedious.

Reading Vortex 2.10's code showed why passing choices with the install
event does not help: when an installed mod is replaced, Vortex drops
choices passed with the event and starts the installer dialog from the
`installerChoices` attribute saved with the mod. It passes them to its
FOMOD engine as a preset, which ticks them while still showing every page.
A saved choice names its page, group and option, and the option's position
in its group.

## Decision

1. A reinstall request may name the options to tick (`tick`: step, group,
   option, and the option's position in its group in the order the
   installer shows them). Mod Mallet sends the options its installer
   finding names. The protocol stays at version 1: the field is optional,
   and older extensions ignore it.
2. The extension (0.6.0) adds these options to the choices saved with the
   mod and saves the result as the mod's `installerChoices` attribute, with
   Vortex's own action (`SET_MOD_ATTRIBUTE`), just before it starts the
   reinstall. Vortex then opens the installer with them ticked.
3. If the installer is cancelled or fails, the previous choices are saved
   back, provided the mod still exists. After a finished install Vortex
   saves the choices the user made, as always.
4. Without saved FOMOD choices nothing is ticked in advance: a preset of a
   few options would replace the installer's own defaults.

## Consequences

- The user sees the missing options ticked and only has to click through
  the installer.
- The extension changes one attribute of a mod in Vortex's state, for the
  length of a reinstall the user asked for, with the action Vortex itself
  uses. Nothing else in Vortex's state is touched.
- How the FOMOD engine matches a preset (by title or by position) is not
  documented. Both are sent; if a mod's installer orders its options
  differently than Mod Mallet reads them, a wrong option could be ticked.
  The user still sees the page before anything is installed.
- Whether it works has to be tried in a real Vortex; the tests cover the
  extension's side only.
