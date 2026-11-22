# 0073 Switched-off mods are taken out through Vortex's plugin list

- Status: accepted
- Date: 2026-11-22
- Changes: ADR 0061 (switched-off mods), its alternative of offering nothing

## Context

ADR 0061 warns when the game loads a plugin of a mod that is switched off
in the mod manager (`plugins.switched-off-but-loaded`). On the reference
setup that is Immersive Citizens - AI Overhaul: an older copy of its plugin
and of its archive is in the game's Data folder, put back by Vortex when it
took its own files away, and `plugins.txt` still lists the plugin as
active. The warning said why, but left the user to find the way out alone,
and the mod's page in the Mods view did not say that its collisions are
still there because of it.

Three ways out were looked at, read-only, on that setup and in Vortex:

- **Deploying** changes nothing: the files are not Vortex's.
- **Vortex's list of plugins** holds every plugin in the Data folder, also
  those it did not install, and writes `plugins.txt` from it. Vortex's own
  switch beside a plugin dispatches `SET_PLUGIN_ENABLED`; its "show
  plugin" opens the page `gamebryo-plugins` and narrows the list to the
  plugin by name. A plugin switched off there is no longer loaded, and
  neither is the archive of its name.
- **The files themselves** can only be moved out by hand. Mod Mallet never
  writes into the Data folder.

A plugin of another mod that names the switched-off plugin as its master
(on the reference setup the Lux patch for Immersive Citizens) would keep
the game from starting once that plugin is out.

## Decision

1. **A new action, `DisablePlugins`, switches plugins off in the mod
   manager's list of plugins.** For Vortex the link (extension 0.9.0)
   carries a new request, `set-plugins-enabled`, with the plugins by file
   name; the extension dispatches `SET_PLUGIN_ENABLED` for each, as the
   switch in the list does, and shows the list narrowed to the first one.
   It runs only on the user's click (ADR 0020). Mod Organizer 2 gets the
   words to untick them there by hand (ADR 0064).
2. **The warning offers it where a copy was left behind**: the plugin of
   the switched-off mod and every plugin that would lose a master with it,
   directly or through another (`stranded_positions`, ADR 0068). Before
   the request is sent, the window names those plugins and asks. Where the
   mod manager has not deployed since, the warning offers nothing new:
   deploying is the way, and one click in Vortex.
3. **The files are named and shown, never moved.** The engine lists, for
   every plugin of a switched-off mod, the files of its name in the Data
   folder that no mod manager put there (`SwitchedOffPlugin::left_behind`:
   the plugin and the archives named after it). A button opens the Data
   folder in the file explorer with the plugin selected; the text says to
   move the files out, for example into a folder next to it. The desktop
   app shows only a plugin or an archive, by its bare name, that is there.
4. **Once the plugin is no longer loaded, the files left behind are a
   note**, `plugins.switched-off-left-behind` (info): they do no harm, but
   if the mod is switched on again, Vortex sets them aside once more, and
   ticking the plugin loads the old copy.
5. **The mod's page leads with it.** Opened in the Mods view, a mod switched
   off but still loaded says so first, with the files, the plugins that
   build on it, the two buttons and what a new scan then shows: its
   collisions gone, the decisions about them waiting (ADR 0061). Its
   collisions, named by the plugin's file since no mod in Vortex brings
   that copy, count as the mod's. The same help is in the tasks, and the
   note on its spots in the collisions has the same buttons.

## Consequences

- On the reference setup the warning names `Immersive Citizens - AI
  Overhaul.bsa` and `.esp` as left behind and offers to switch off the
  plugin together with `Lux - Immersive Citizens patch.esp`. The mod's
  page counts its 84 collision spots, which the collisions name by the
  plugin's file.
- The link's version goes to 0.9.0; an older link answers the request
  `unsupported`, and the window says to install the link again.
- Vortex keeps the plugins switched off until the user ticks them again;
  that is the undo, as for every other change through the link.
- Moving the files out is still the user's step. Mod Mallet names and shows
  them, but does not offer to move or delete them.

## Alternatives considered

- **Move the files into a folder next to the Data folder.** It would finish
  the job in one click, but it writes into the game's folder, which a scan
  and the app never do (ADR 0005, 0020). Vortex has no call for files it
  does not manage.
- **Only open Vortex's plugin list, narrowed to the plugin.** Kept as part
  of the request, so the user sees the change, but not alone: switching the
  plugin off is what Vortex's own switch does, and asking the user to find
  and click it adds a step without adding safety.
- **Switch off only the plugin, not what builds on it.** The game would
  then not start; Vortex warns about missing masters only in its list.
