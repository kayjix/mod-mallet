### Texts for every kind of finding declared in `mallet-core/src/kinds.rs`.
###
### The message id is the kind id with `.` replaced by `-`. Every message
### needs a value (the title) and the attributes `.summary`, `.explanation`
### and `.remedy`. Only `.summary` may use the kind's arguments; the other
### texts must read on their own because the reference documentation shows
### them without a concrete finding.
###
### Write for someone who installs mods but has never opened a modding tool:
### say what happened, why it matters in the game, and what to do.

extender-plugin-incompatible = Script extender plugin does not support this game version
    .headline = { $file } does not support game version { $gameVersion }
    .summary = { $file } was switched off because it was not built for game version { $gameVersion }.
    .explanation = The script extender loads native plugins (DLL files) that mods use for features the game cannot provide on its own. Each plugin states which game versions it supports. This one does not support yours, so the script extender refused to load it. The game still starts, but everything the plugin provides is missing, and other mods that rely on it may misbehave.
    .remedy = Look on the mod's page for a version built for your game version and install it. If there is none yet, disable the mod until it has been updated.

extender-plugin-not-loaded = Script extender plugin was not loaded
    .headline = { $file } was not loaded
    .summary = { $file } was not loaded. The script extender reported: { $reason }
    .explanation = The script extender tried to load this native plugin (a DLL file) and gave up. The game still starts, but everything the plugin provides is missing. The reason is quoted from the script extender's own log. Common causes are a plugin built for a different game version, a missing requirement such as Address Library, or a damaged download.
    .remedy = Check that the mod's requirements are installed and that the mod matches your game version, then reinstall it. If that does not help, disable the mod and report the quoted reason to its author.

extender-plugin-no-version-data = File in the plugin folder is not a script extender plugin
    .headline = { $file } is not a script extender plugin
    .summary = { $file } lies in the script extender's plugin folder but does not identify itself as a plugin, so it was skipped.
    .explanation = Some mods ship support libraries next to their plugin. The script extender examines every DLL in its plugin folder and skips those that carry no plugin information, which is harmless. It only points to a problem if you expected this file to be a plugin: then it is damaged or was built for a different game.
    .remedy = Nothing to do if the file is a support library of one of your mods. If it should be a plugin, reinstall the mod it belongs to.

versions-extender-lacks-game = The script extender does not support this game version yet
    .headline = { $extender } { $extenderVersion } does not support game version { $gameVersion } yet
    .summary = The game is now version { $gameVersion }, and the installed { $extender } { $extenderVersion } only goes up to { $newest }. Started through { $extender }, the game will not run, and none of its { $plugins } plugins load. Its authors usually publish a version for a game update within days; you can also roll the game back to { $newest }.
    .explanation = The script extender has to match the game's version exactly: it brings one file per game version it supports. When Steam updates the game, the script extender's authors usually publish a new version within a day or a few. Until then the game only starts without it, and then everything that mods add through it is missing: menus, tweaks, fixes and new features.
    .remedy = Wait for the new script extender and install it from its page; this usually takes days, not weeks. Until then you can play without the mods that need it. If you want to play with them now, roll the game back to the newest version the script extender supports: if Steam offers that version under the game's Properties, Betas, pick it there; otherwise a downgrade patcher from Nexus Mods does it. To keep Steam from updating the game by surprise, set its Updates to "Only update this game when I launch it" and start the game through the script extender's loader.

versions-address-library-lacks-game = The Address Library has no file for this game version
    .headline = The Address Library has no file for game version { $gameVersion }
    .summary =
        { $count ->
            [one] 1 script extender plugin finds its way into the game through the Address Library
           *[other] { $count } script extender plugins find their way into the game through the Address Library
        }, which has no file for game version { $gameVersion } yet: { $examples }. They will stop with an error message when the game starts, or not work. Its author usually adds the file within days of a game update.
    .explanation = Most script extender plugins do not contain the places in the game's code they change. They look them up in the Address Library, a mod with one file per game version. That is why these plugins keep working after a game update without being rebuilt: only the Address Library needs a new file, which its author usually publishes within days of an update.
    .remedy = Update the Address Library for SKSE Plugins from Nexus Mods once it has a file for your game version; this usually takes days, not weeks. Until then, roll the game back through Steam's Betas or a downgrade patcher, or play without these plugins.

versions-address-library-missing = The Address Library is not installed
    .headline =
        The Address Library is not installed, and { $count ->
            [one] 1 plugin needs it
           *[other] { $count } plugins need it
        }
    .summary =
        { $count ->
            [one] 1 script extender plugin finds its way into the game through the Address Library
           *[other] { $count } script extender plugins find their way into the game through the Address Library
        }, which is not installed: { $examples }. They will stop with an error message when the game starts, or not work.
    .explanation = Most script extender plugins do not contain the places in the game's code they change. They look them up in the Address Library, a mod with one file per game version. Without it, these plugins cannot find their way into the game.
    .remedy = Install the Address Library for SKSE Plugins from Nexus Mods (the "All in one" file) with your mod manager.

versions-plugin-wrong-game = Script extender plugin was built for another game version
    .headline = { $file } was built for game version { $builtFor }, not { $gameVersion }
    .summary =
        { $file } lists only game version { $builtFor }, and the game is now { $gameVersion }. { $confirmed ->
            [yes] The script extender switched it off at the last start of the game.
           *[other] The script extender will switch it off when the game starts.
        } { $updateKnown ->
            [yes] Your mod manager knows a newer version of its mod, { $update }: install it first, it may already fit.
           *[other] Usually its author publishes a rebuilt version within days.
        }
    .explanation = This plugin contains the places in the game's code it changes for exactly the versions it lists, so it does not work after a game update until its author builds it again. For popular mods that rebuild usually comes within days of the update. The game still starts, but everything the plugin provides is missing, and mods that build on it may misbehave.
    .remedy = Look on the mod's page for a version made for your game version and install it; usually it comes within days. Until then, switch the mod off so that nothing relies on half of it, or roll the game back to the version the plugin was built for: through Steam's Betas under the game's Properties, if Steam offers that version, or with a downgrade patcher from Nexus Mods.

versions-plugin-needs-newer-game = Script extender plugin needs a newer game version
    .headline = { $file } was built for game version { $builtFor }, newer than { $gameVersion }
    .summary =
        { $file } lists only game version { $builtFor }, and the game is { $gameVersion }. { $confirmed ->
            [yes] The script extender switched it off at the last start of the game.
           *[other] The script extender will switch it off when the game starts.
        } Its mod was probably updated for a game update you do not have yet.
    .explanation = Mod authors update their plugins for each new game version, and sometimes drop the older ones. A plugin built for a newer game version than yours does not load. The game still starts, but everything the plugin provides is missing, and mods that build on it may misbehave.
    .remedy = Update the game through Steam, or install the previous version of the mod again, the one made for your game version. Mod managers keep older files of a mod on its page under Files, often as "old files".

versions-plugin-before-anniversary = Script extender plugin is made for the game before version 1.6
    .headline = { $file } is made for the game before version 1.6
    .summary =
        { $file } only works with the game before the Anniversary update of 2021 (version 1.5.97 and older). The script extender for game version { $gameVersion } does not load it. { $updateKnown ->
            [yes] Your mod manager knows a newer version of its mod, { $update }, which may be made for the newer game.
           *[other] A rebuild is not enough here: the mod needs a real change by its author.
        }
    .explanation = Since game version 1.6, script extender plugins describe themselves in a new way, and the script extender no longer loads plugins that do not. A rebuild is not enough here: the mod's author has to change the plugin for the new game, and many old mods were never changed. The game still starts, but everything the plugin provides is missing.
    .remedy = Look on the mod's page for a version for the Anniversary Edition (often called "AE" or "1.6.x"), or for a newer mod that does the same. If there is none, switch the mod off. Going back to game version 1.5.97 for it is possible with a downgrade patcher, but then many newer plugins stop working instead.

versions-plugin-needs-newer-extender = Script extender plugin needs a newer script extender
    .headline = { $file } needs script extender { $required } or newer
    .summary = { $file } asks for at least script extender version { $required }, and { $extenderVersion } is installed. The script extender does not load it.
    .explanation = Plugins can rely on things the script extender only offers from a certain version on, and say so. Updating the script extender is safe as long as the new version supports your game version.
    .remedy = Install the newest script extender for your game version from its page. Its files go into the game's folder, next to the game's program.

versions-master-updated = A plugin builds on a mod that was updated under it
    .headline =
        { $plugin }: { $count ->
            [one] a plugin it builds on was updated
           *[other] { $count } plugins it builds on were updated
        }
    .summary = Since your last scan, the mod of what { $plugin } builds on was updated, while { $mod } was not: { $masters }.
    .explanation = A patch or add-on is made for the plugins it builds on as they were then. After an update of those, it usually still loads, but it may undo the update's changes or point at things that are no longer there. Many authors say on their page which version of the other mod their patch is for. This is only a hint: often nothing is wrong.
    .remedy = Look on the patch's page which version of the other mod it was made for, and whether there is a newer patch. If the update brought its own patch, switch the old one off.

versions-loot-warning = LOOT warns about the versions in your setup
    .headline =
        LOOT has { $count ->
            [one] a warning
           *[other] { $count } warnings
        } about versions
    .summary = LOOT says, in its own words: { $messages }
    .explanation = LOOT, the tool that sorts plugins, also knows which versions of the game and its script extender are current, and which go together. Mod Mallet quotes its warnings as LOOT writes them, in English, and only those whose conditions hold for your setup.
    .remedy = Read the message and follow it: usually it asks to update the script extender or the game.


plugins-master-missing = A plugin builds on a plugin that is not installed
    .headline =
        { $plugin }: { $count ->
            [one] 1 plugin it needs is
           *[other] { $count } plugins it needs are
        } missing
    .summary =
        { $plugin } builds on { $count ->
            [one] a plugin that is
           *[other] { $count } plugins that are
        } not installed: { $masters }.
    .explanation = A plugin can build on other plugins, its masters, and names them in its file. The game loads the masters first. If one of them is not there, the game closes without a message while it starts, usually right after the logo. This typically happens when a patch for two mods is installed and one of the two is not, or when a mod was removed and its patches stayed behind.
    .remedy = Install the mod that contains the missing plugin, or disable the mod this plugin belongs to. If the plugin is a patch that an installer offered, run that installer again and untick the patch.

plugins-generated-outdated = A generated plugin is older than the load order
    .headline = { $plugin } was made for an older load order
    .summary =
        { $tool } made { $plugin } before the load order changed. { $count ->
            [one] One plugin it was made from now loads
           *[other] { $count } plugins it was made from now load
        } in a different place: { $examples }.
    .explanation = Some tools look at all your plugins and write a plugin of their own from what they find: DynDOLOD for distant land, Synthesis and Wrye Bash for patches that merge the changes of many mods. Such a plugin is a snapshot. When the load order changes afterwards, because a mod was added, removed or sorted again, the snapshot still holds what was true before. Distant objects may then not match what you see up close, and changes of mods that were sorted again may be lost.
    .remedy = Run the tool that made the plugin again, and replace its old output with the new one. If you keep the output as a mod in your mod manager, install the new output into the same mod.

plugins-requirement-missing = A plugin needs a file that is not in the game
    .headline =
        { $plugin }: { $count ->
            [one] 1 file it needs is missing
           *[other] { $count } files it needs are missing
        }
    .summary =
        According to LOOT, { $plugin } needs { $count ->
            [one] a file that is
           *[other] { $count } files that are
        } not in your game or not switched on: { $files }.
    .explanation = LOOT, the tool that sorts plugins, keeps a list of what thousands of plugins need and what they do not work with. Mod managers such as Vortex download this list on their own. Many mods only work together with another mod, often a script extender plugin or a library such as PapyrusUtil. Without it, parts of the mod do nothing, its scripts fail, or the game closes as soon as it needs the missing piece.
    .remedy = Install the mod that brings the missing file; the mod's page lists what it needs. If the missing file is a plugin that is installed, switch it on in your mod manager. If you do not want the other mod, disable the mod this plugin belongs to.

plugins-incompatible = Two mods do not work together
    .headline = { $plugin } does not work together with { $other }
    .summary = According to LOOT, { $plugin } cannot be used together with { $other }, and both are in your game.
    .explanation = Some mods change the same thing in ways that exclude each other: two versions of the same fix, two overhauls of the same part of the game, or an old mod whose work a newer one already contains. LOOT's list names such pairs. With both installed, one undoes the other, features break, or the game becomes unstable.
    .remedy = Decide which of the two you want and disable the other in your mod manager. If one replaces the other, keep the newer one. The mods' pages often say which to choose.

plugins-needs-cleaning = A plugin deletes things the game and other mods rely on
    .headline = { $plugin } should be cleaned
    .summary = LOOT knows exactly this version of { $plugin }. Counted with { $utility }: deleted objects: { $deleted }, deleted navigation meshes: { $navmeshes }, unchanged copies of records: { $identical }.
    .explanation = Some plugins delete objects of the game outright instead of switching them off, a leftover of how they were made. When another mod or one of the game's scripts refers to a deleted object, the game can crash. Deleted navigation meshes, the paths characters walk on, are the most serious: they can crash the game and cannot be repaired automatically. LOOT's list records which released versions of a plugin contain such deletions. It recognises a version by the file's checksum, so this is about exactly your file.
    .remedy = Install a newer version of the mod if there is one. Otherwise the plugin can be cleaned with SSEEdit's "Quick Auto Clean", which switches deleted objects off instead; LOOT's and SSEEdit's guides explain the steps. Keep a copy of the plugin first. With Vortex, clean the file in the mod's folder in Vortex's staging folder, so that the cleaned file stays. Deleted navigation meshes cannot be cleaned automatically: if there are any, look on the mod's page for a fixed version or a patch.

plugins-identical-records = A plugin contains unchanged copies of records
    .headline =
        { $plugin } contains { $count ->
            [one] 1 unchanged copy
           *[other] { $count } unchanged copies
        } of the game's records
    .summary =
        LOOT knows exactly this version of { $plugin }: counted with { $utility }, it contains { $count ->
            [one] 1 record
           *[other] { $count } records
        } copied without change from a plugin it builds on.
    .explanation = A record that a plugin copies without changing it does nothing by itself. It matters only when another mod that loads earlier changes the same record: the unchanged copy then quietly puts the original back. That is usually harmless, but it can hide another mod's change. LOOT's list records which released versions of a plugin contain such copies; it recognises a version by the file's checksum.
    .remedy = Usually nothing to do. If a change of another mod does not show in the game, clean this plugin with SSEEdit's "Quick Auto Clean" or let it load before that mod.

plugins-loot-warning = LOOT warns about a plugin
    .headline = LOOT warns about { $plugin }
    .summary = LOOT says about { $plugin }, in its own words: { $messages }
    .explanation = LOOT, the tool that sorts plugins, keeps notes about thousands of plugins and shows them when they apply to a setup: a missing compatibility patch, a mod that should not be used together with another, a plugin that should be deleted because another mod already contains it. Mod managers such as Vortex show these notes in their plugin list as well. Mod Mallet quotes them as LOOT writes them, in English, and only those whose conditions hold for your setup.
    .remedy = Read the message and follow it. A missing patch is usually on the page the message names, or offered by the installer of one of the two mods. If the message says to delete or disable a plugin, disable the mod it belongs to in your mod manager.

plugins-loot-note = LOOT has a note about a plugin
    .headline =
        LOOT has { $count ->
            [one] a note
           *[other] { $count } notes
        } about { $plugin }
    .summary = LOOT says about { $plugin }, in its own words: { $messages }
    .explanation = LOOT, the tool that sorts plugins, keeps notes about thousands of plugins and shows them when they apply to a setup: an update patch that fixes the mod, a patch collection for it, compatibility notes worth reading, or a hint that the mod needs a new save. Mod Mallet quotes them as LOOT writes them, in English, and only those whose conditions hold for your setup.
    .remedy = Nothing has to be done. Look at the note when you change something about this mod, or when it does not behave as expected.


npcs-face-mismatch = Characters with dark or mismatched faces
    .headline =
        { $winnerPlugin } changes { $count ->
            [one] 1 face
           *[other] { $count } faces
        } made for { $facePlugin }
    .summary =
        { $winnerPlugin } is loaded after { $facePlugin } and describes { $count ->
            [one] a different face for 1 character
           *[other] different faces for { $count } characters
        } than the face files in use, which come from { $faceOrigin ->
            [game] the game itself
           *[mod] { $faceMod }
        }. For example: { $examples }.
    .explanation = The game does not build a character's face while playing. It loads a face mesh that was generated in advance from the character's record. When the record that wins in the load order describes a different face than that mesh was generated from, the game draws the head in the wrong colours, usually much darker than the body. This happens when one mod provides the face files and a plugin of another mod, loaded later, changes the same characters. The check compares the records as written, so it can report a plugin whose change does not show in the game.
    .remedy = If you want the looks of the face files, load their plugin after the other one, or look for a compatibility patch for the two mods. If you want the other plugin's version of the characters, its mod has to provide matching face files.

installer-options-now-recommended = Recommended installer options are not installed
    .headline =
        { $mod }: { $count ->
            [one] 1 recommended option is
           *[other] { $count } recommended options are
        } not installed
    .summary =
        The installer of { $mod } would now tick { $count ->
            [one] 1 option that was not chosen
           *[other] { $count } options that were not chosen
        } when the mod was installed: { $examples }. Each is given as step › group › option.
    .explanation = Many mods come with an installer that offers patches for other mods and ticks the ones that fit what is installed at that moment. The dialog is shown once. If a mod that needs such a patch is installed afterwards, nobody offers the patch again, and the two mods run together unpatched: objects overlap, changes of one mod are lost, or lighting and textures do not match. Mod Mallet replays the installer's own rules against the current setup and reports the options it would tick now. An option you left out on purpose is reported as well.
    .remedy = Reinstall the mod in your mod manager, go through its installer again and tick the options named above. In Vortex, select the mod and choose Reinstall. Deploy and sort the plugins afterwards.

plugins-contained = A mod is already part of another
    .headline = { $plugin } is already part of { $container }
    .summary =
        { $container } places { $count } of the { $total } objects of { $plugin } in the same spots, so { $plugin } is already part of it and the game shows these objects twice. { $dependentCount ->
            [0] No other plugin builds on { $pluginFile }.
           *[other] These plugins build on { $pluginFile }: { $dependents }.
        }
    .explanation = Collections ("all in one") often contain mods that are also offered on their own. With both installed, their objects stand in the same spot twice: surfaces flicker, and doors and containers can exist twice. Often a patch needs the single mod as a master. Then the single mod cannot simply be switched off: the game would close while starting.
    .remedy = If no other plugin builds on the contained mod, disable it in your mod manager. If a patch builds on it, look on the patch's page for a version made for the collection; many patches come in one version for the single mod and one for the collection. If there is none, keep both and live with the doubled objects for now.

objects-duplicated = Two mods place the same object twice
    .headline =
        { $first } and { $second } place { $count ->
            [one] 1 object
           *[other] { $count } objects
        } twice
    .summary =
        { $first } and { $second } place the same { $count ->
            [one] object
           *[other] { $count } objects
        } at almost the same spot, so the game shows { $count ->
            [one] it
           *[other] them
        } twice. For example: { $examples }. Each is given with its place: an interior by its internal name, outside by worldspace and cell.
    .explanation = Mods that change the same town or the same stretch of land often add the same things, such as a fence, a market stall or a house that the game itself lacks. Each mod places its own copy. The game shows both copies in the same spot, which flickers where the surfaces meet and can make doors or containers appear twice. Mod managers do not show this, because the two mods change different records.
    .remedy = Look on the pages of both mods for a compatibility patch; for well-known combinations there usually is one. Install it with both mods and scan again. If there is none, decide which of the two mods you want to keep for this place, or live with the doubled objects.

objects-overlapping = Objects of two mods stand inside each other
    .headline =
        { $first } and { $second }: { $count ->
            [one] 1 object stands
           *[other] { $count } objects stand
        } inside another
    .summary =
        { $first } and { $second } each place objects of similar size in the same spot, so that they stand largely inside each other in { $count ->
            [one] 1 place
           *[other] { $count } places
        }. For example: { $examples }. Each is given with its place: an interior by its internal name, outside by worldspace and cell.
    .explanation = Two mods that change the same place each put their own buildings, trees or other objects there, without knowing of each other. Where they meet, a tree grows through a roof, two houses merge into one, or a path runs into a wall. Where the models of both objects can be read, Mod Mallet compares their shapes; otherwise only the box around each model, so an object with a lot of empty space, such as an arch, can then be reported although nothing visibly collides.
    .remedy = Look on the pages of both mods for a compatibility patch. If there is none, look at the places named above in the game. If something is in the way, decide which of the two mods you want to keep for this place.

objects-touching = Objects of two mods cut into each other
    .headline =
        { $first } and { $second }: { $count ->
            [one] 1 object cuts
           *[other] { $count } objects cut
        } into another
    .summary =
        Objects of { $first } and of { $second } touch or reach a little into each other in { $count ->
            [one] 1 place
           *[other] { $count } places
        }: their models cross, but neither stands largely inside the other. For example: { $examples }. Each is given with its place: an interior by its internal name, outside by worldspace and cell.
    .explanation = Two mods that place things in the same area did not know of each other, so a fence may run into a wall, a cart may stand half in a market stall, or a lamp post may grow out of a roof. Mod Mallet compared the shapes of both models, not just the boxes around them. Within one mod such contacts are usually wanted, so they are only reported between mods that do not know each other.
    .remedy = Look at the places named above in the game or on the map. Often it does not show; where it does, decide which object you want to keep, or look for a compatibility patch of both mods.

doors-stacked = Doors of two mods stand on top of each other
    .headline =
        { $first } and { $second }: { $count ->
            [one] 1 door stands
           *[other] { $count } doors stand
        } on top of another
    .summary =
        { $first } and { $second } each put a door at the same spot, a few centimetres apart at most, in { $count ->
            [one] 1 place
           *[other] { $count } places
        }. For example: { $examples }. Each is given with its place: an interior by its internal name, outside by worldspace and cell.
    .explanation = Many mods add doors, and a second mod often puts its own door where the first, or the game, already has one: a little in front of it or behind it, because its wall is thicker or its frame sits elsewhere. The game shows both. One flickers through the other, you open whichever is in front, and it may lead somewhere else than the one you see. Characters of the game walk through the door their paths name, which may be the other one. A second door right behind the first on purpose, such as a porch with an inner door, is rare.
    .remedy = Open the spot on the collisions page. It shows the doors side by side, where each leads, and which one Mod Mallet suggests to keep: usually the one whose other side leads back and that characters walk through. Choose "merge into this one": the others are hidden, and whatever used them is to use the kept door.

navmeshes-door-cut-off = Characters can no longer walk through doors of a mod
    .headline =
        { $winner } cuts off { $count ->
            [one] 1 door
           *[other] { $count } doors
        } of { $loser }
    .summary =
        { $loser } connects { $count ->
            [one] a door
           *[other] { $count } doors
        } that mods add to the paths characters walk on. { $winner } replaces those paths with a version made without { $loser }, in which the { $count ->
            [one] door is
           *[other] doors are
        } not connected. For example: { $places }. An interior is given by its internal name, outside by worldspace and cell.
    .explanation = The game keeps an invisible net on the ground of every place, which characters use to find their way: its navigation mesh. A door can only be used by characters if the net leads to it. A mod that adds a house connects the net to the house's door. When a later mod replaces the same net with its own version, the door is no longer connected: followers, guards and townspeople stop in front of it or never go in, and quests that send someone through it can get stuck. Only you can still open it.
    .remedy = Look on the pages of both mods for a compatibility patch, and install it if there is one. Without a patch, decide which of the two mods you want to keep for these places. Loading the mod with the door later moves the problem elsewhere: then the other mod's paths are lost.

navmeshes-changes-undone = A mod undoes where another mod lets characters walk
    .headline =
        { $winner } undoes { $count ->
            [one] 1 walking path
           *[other] { $count } walking paths
        } of { $loser }
    .summary =
        { $loser } and { $winner } both change where characters can walk { $count ->
            [one] in one place
           *[other] in { $count } places
        }. The game uses only the version of { $winner }, which was not made with { $loser }. For example: { $places }. An interior is given by its internal name, outside by worldspace and cell.
    .explanation = The game keeps an invisible net on the ground of every place, which characters use to find their way: its navigation mesh. A mod that builds a house or a wall changes that net so that people walk around it. A navigation mesh is replaced as a whole, so when two mods change the same one, only the later mod's version counts. People and creatures then walk through the earlier mod's buildings, get stuck at its walls, or no longer find the way into what it added. Followers are often the first to show it.
    .remedy = Look on the pages of both mods for a compatibility patch, and install it if there is one. Without a patch, keep only one of the two mods for these places, or look at them in the game to judge whether it matters there.

meshes-made-for-other-game = Meshes of a mod were made for another game
    .headline =
        { $count ->
            [one] 1 mesh
           *[other] { $count } meshes
        } made for { $madeFor }
    .summary =
        { $count ->
            [one] One mesh of this mod is
           *[other] { $count } meshes of this mod are
        } in the format of { $madeFor }, not of the game you play{ $skinned ->
            [0] .
            [one] . One of them bends with a skeleton, like clothes or armour. If something crashes or stays invisible, look at that one first.
           *[other] . { $skinned } of them bend with a skeleton, like clothes or armour. If something crashes or stays invisible, look at these first.
        } For example: { $examples }.
    .explanation = A mesh is the 3D shape of a thing in the game. Each game, and each edition of a game, reads meshes in its own variant of the format. Mods for the original Skyrim of 2011 often reached Skyrim Special Edition with some meshes left as they were, and many well-kept mods still ship such meshes. Most of them work. Some do not: then the thing they belong to stays invisible, looks wrong, or the game closes when it appears, most often with clothes, armour and creatures, which bend with a skeleton. This note is a lead for when something like that happens, not a sign that it will.
    .remedy = Nothing to do while the game runs well. If something of this mod crashes the game or stays invisible, look on the mod's page for a version made for your game and install that instead. If there is none, the meshes can be converted with Cathedral Assets Optimizer, a free tool made for exactly this; then install the converted files as a mod of their own that overwrites this one.

meshes-texture-missing = Meshes of a mod use textures that are not in the game
    .headline =
        { $count ->
            [one] 1 texture
           *[other] { $count } textures
        } missing for { $meshes ->
            [one] 1 mesh
           *[other] { $meshes } meshes
        }
    .summary =
        { $meshes ->
            [one] One mesh of this mod uses
           *[other] { $meshes } meshes of this mod use
        } { $count ->
            [one] a texture
           *[other] { $count } textures
        } for their colour or surface that no mod and no archive in your game provides: { $examples }.{ $offCount ->
        [0] {""}
        [one] {" "}1 of them is in { $offMods }, installed but switched off.
       *[other] {" "}{ $offCount } of them are in { $offMods }, installed but switched off.
    }
    .explanation = A mesh is the 3D shape of a thing; its colour and the fine detail of its surface come from textures, separate image files that the mesh names. If a named texture is not there, the game draws the surface in a glaring purple, or dark and flat where the surface detail is missing. This usually means that a part of the mod was not installed, for example its textures in a separate download, or that the mod was made to go with another mod you do not have. Meshes a mod installs for a mod you do not use are never shown, so their missing textures do no harm.
    .remedy = If Mod Mallet names a mod that brings them but is switched off, switch it on in Vortex. Otherwise look on the mod's page for files that belong with it, such as a texture download or a mod it needs, and install them. If the meshes are part of a patch for a mod you do not have, you can leave them.

meshes-retexture-bypassed = A texture mod does not reach replaced meshes
    .headline =
        { $textureMod } does not reach { $count ->
            [one] 1 mesh
           *[other] { $count } meshes
        } { $meshOrigin ->
            [unmanaged] installed without a mod manager
           *[mod] of { $meshMod }
        }
    .summary =
        { $textureMod } replaces textures that are no longer used by { $count ->
            [one] 1 mesh
           *[other] { $count } meshes
        } { $meshOrigin ->
            [unmanaged] that no mod manager installed
           *[mod] of { $meshMod }
        }. { $count ->
            [one] The new mesh points
           *[other] The new meshes point
        } at other textures instead. For example: { $examples }.
    .explanation = A mesh names the textures it uses by path. A texture mod works by providing new files under the paths the original meshes name. When another mod replaces a mesh and its version names other paths, usually textures in that mod's own folder, the texture mod no longer reaches the object. The object keeps one look while everything around it gets the new one: a bright wall next to dark ones, a sharp roof next to blurry ones. How much it shows depends on how different the two sets of textures are.
    .remedy = Check whether the texture mod offers an option or a patch for the mesh mod; many do. Otherwise decide which look you prefer: remove the listed meshes of the mesh mod, so that the original meshes and the texture mod apply, or keep them and accept the difference.

assets-loose-over-archive = Loose files cover files in another mod's archive
    .headline =
        { $winnerOrigin ->
            [unmanaged] Files installed without a mod manager cover
           *[mod] { $winnerMod } covers
        } { $count ->
            [one] 1 file
           *[other] { $count } files
        } of { $archiveMod }
    .summary =
        { $winnerOrigin ->
            [unmanaged] Files that no mod manager installed
           *[mod] Loose files of { $winnerMod }
        } replace { $count ->
            [one] 1 file
           *[other] { $count } files
        } in { $archive } of { $archiveMod }. For example: { $examples }.
    .explanation = A mod can ship its files packed into an archive or as loose files. The game always prefers a loose file over the same file in an archive, whatever the order of the mods. Mod managers show conflicts between loose files but not this kind, because one side is hidden inside the archive. Often it is intended, for example a texture pack over an older mod. It becomes a problem when only some of the files that belong together are replaced, such as a mesh without its texture: the result can be purple or misplaced objects.
    .remedy = Nothing to do if you want the loose files to win. If the mod in the archive looks broken in the game, disable the mod that provides the loose files, or check on its page whether the two are compatible.

assets-archive-over-archive = An archive covers files in another mod's archive
    .headline =
        { $winnerOrigin ->
            [unmanaged] { $winnerArchive }
           *[mod] { $winnerMod }
        } covers { $count ->
            [one] 1 file
           *[other] { $count } files
        } of { $archiveMod }
    .summary =
        { $winnerOrigin ->
            [unmanaged] { $winnerArchive }
           *[mod] { $winnerArchive } of { $winnerMod }
        } is loaded later and replaces { $count ->
            [one] 1 file
           *[other] { $count } files
        } in { $archive } of { $archiveMod }. For example: { $examples }.
    .explanation = Archives are loaded in the order of the plugins they belong to. When two archives contain the same file, the game uses the one loaded later. Mod managers do not show this, because both sides are hidden inside archives. It is harmless when the later mod is meant to build on the earlier one. It is a problem when the later archive carries older copies of files that it shares with the earlier mod.
    .remedy = Nothing to do if the later mod is meant to win. Otherwise change the load order of the two plugins so that the archive you want is loaded last.

setup-file-unreadable = A file of the setup could not be read
    .headline = { $file } could not be read
    .summary = { $file } could not be read: { $reason }
    .explanation = Mod Mallet skipped this file, so problems that involve it are not reported. If the file is damaged, the game may have trouble with it as well. A damaged download is the usual cause.
    .remedy = Reinstall the mod the file belongs to. If the game works with the file and Mod Mallet does not, please report it.

setup-load-order-unknown = The load order could not be read
    .headline = The load order could not be read
    .summary = Plugins and archives were not checked: { $reason }
    .explanation = Mod Mallet needs to know which plugins are active and in which order the game loads them. Without that it cannot tell which files and records win, so those checks did not run. The other checks are not affected.
    .remedy = Start the game or your mod manager once so that the list of plugins is written, then scan again. If the game is installed in an unusual place, tell Mod Mallet where it is.

crashes-recent = The game crashed recently
    .headline =
        { $count ->
            [one] 1 crash
           *[other] { $count } crashes
        } in the last { $days } days, last on { $last }
    .summary =
        A crash logger recorded { $count ->
            [one] one crash
           *[other] { $count } crashes
        } of the game in the last { $days } days, the last on { $last }. Where the game failed: { $examples }.
    .explanation = A crash logger is a small addition to the script extender that writes a report every time the game crashes. The report says in which program file the game failed and lists what was nearby: characters, objects, meshes and the mods they come from. Reading it is a matter of likelihood, not certainty. Mod Mallet looks at the reports of the last weeks and says which mods they point at; a crash inside the game itself with nothing of a mod nearby is counted but not blamed on anyone.
    .remedy = Look at the other findings about crashes, if there are any. If the game crashes at one place again and again, note what you were doing; that helps whoever looks into it.

crashes-in-dll = The game crashed inside a mod's program file
    .headline =
        { $file }: { $count ->
            [one] 1 recent crash
           *[other] { $count } recent crashes
        } happened inside it
    .summary =
        { $count ->
            [one] One recent crash, on { $last }, happened
           *[other] { $count } recent crashes, the last on { $last }, happened
        } inside { $file }, a program file that a mod adds to the script extender.
    .explanation = Some mods bring a program file of their own, a DLL that the script extender loads into the game. When the game crashes while running that file's code, the file is the most likely cause: it may not fit the game's version, clash with another such file, or have a bug. It is not certain: the file may also have been handed broken data by something else.
    .remedy = Look on the mod's page for an update and for known problems with your game version. If the crashes go on, disable the mod for a while and see whether they stop.

crashes-mod-involved = Recent crash reports point at a mod
    .headline =
        { $count ->
            [one] 1 crash report points
           *[other] { $count } crash reports point
        } at this mod
    .summary =
        { $count ->
            [one] One recent crash report, from { $last }, names
           *[other] { $count } recent crash reports, the last from { $last }, name
        } things of this mod near the crash: { $examples }.
    .explanation = Crash reports list what was in use when the game failed: characters, objects, meshes, and program files on the way to the crash. When things of one mod turn up there, the mod may be involved, for example through a broken mesh or a character with an odd setup. It may also just have been close by, especially when the crash happened in a busy place. The more reports point at the same mod, the more likely it is involved.
    .remedy = If several reports point at this mod, look on its page for known crashes and patches. To be sure, disable it for a while, or avoid the place or thing named above, and see whether the crashes stop.
mods-collides-widely = Suggestion: a mod collides with many places of other mods
    .headline =
        Suggestion: { $mod } collides with other mods in { $count ->
            [one] 1 place
           *[other] { $count } places
        }
    .summary =
        { $objects } of the { $total } objects { $mod } puts into the world stand in or against objects of { $modCount ->
            [one] another mod
           *[other] { $modCount } other mods
        }, in { $count } places: { $others }. The number after each mod is how many places it shares with { $mod }. { $interiors ->
            [0] {""}
            [one] { $mod } also furnishes 1 interior of its own that these mods hardly touch, with { $interiorObjects } objects: switching it off would lose that room, so keep it and decide only the places where they collide.
           *[other] { $mod } also furnishes { $interiors } interiors of its own that these mods hardly touch, with { $interiorObjects } objects: switching it off would lose these rooms, so keep it and decide only the places where they collide.
        } { $lootCount ->
            [0] {""}
           *[other] LOOT says: { $loot }
        } { $dependentModCount ->
            [0] No other mod's plugin builds on { $mod }.
            [one] 1 other mod needs { $mod }: { $dependents }. Removing { $mod } breaks it unless you remove it too.
           *[other] { $dependentModCount } other mods need { $mod }: { $dependents }. Removing { $mod } breaks them unless you remove them too.
        } { $assetCount ->
            [0] {""}
           *[other] Objects of other mods show models that only { $mod } brings ({ $assetUsers }); without it they would be missing.
        }
    .explanation = This is only a suggestion; nothing is broken. Two mods that change the same part of the world put their buildings, trees and fences where the other already has something, even mods by the same author when one was not made to fit the other there. A few such places are normal and are listed one by one under collisions. When one mod collides this often, the two do not fit together in these places as they are, or a patch that makes them fit is missing.
    .remedy = Decide which version of these places you prefer. If LOOT names a patch, install it first and scan again. If you prefer the other mods' version, keep the mod and, in the list of collisions, let it lose against the other mods in every place at once; switch it off only if it brings nothing else you want, such as rooms of its own. If you like the mod, keep it, and look on its page for a patch that makes it work with the other mods.

mods-mostly-overridden = Suggestion: hardly anything of a mod reaches the game
    .headline = Suggestion: only { $percent } % of { $mod } reaches the game
    .summary = Of the { $total } parts of { $mod } that Mod Mallet compared, only { $surviving } reach the game. Mods loaded later replace the rest: { $overriders }. The number after each is how many parts it replaces.
    .explanation = This is only a suggestion; nothing is broken. When two mods change the same thing, the game uses the version of the mod it loads later. Mod Mallet compared the objects the mod places, the characters it changes, and the files in its archives. A mod can bring other things as well, such as quests, items or scripts, which are not counted here.
    .remedy = If you do not miss anything from the mod in the game, you can switch it off in your mod manager; that makes the setup simpler and loads faster. Before you do, check its page: some mods are needed by others, and those would then stop working. If you prefer the version of the mod, place its plugin after the mods named above.

mods-duplicates-another = Suggestion: a mod places largely the same objects as another
    .headline = Suggestion: keep { $mod } or { $other }, not both
    .summary =
        { $count } of the { $total } objects { $mod } puts into the world are placed by { $other } as well, the same model at the same place. { $patchCount ->
            [0] No other mod is made for { $mod } alone.
            [one] One plugin is made for { $mod } alone: { $patches }.
           *[other] { $patchCount } plugins are made for { $mod } alone: { $patches }.
        } { $otherPatchCount ->
            [0] No other mod is made for { $other } alone.
            [one] One plugin is made for { $other } alone: { $otherPatches }.
           *[other] { $otherPatchCount } plugins are made for { $other } alone: { $otherPatches }.
        } { $lootCount ->
            [0] {""}
           *[other] LOOT says: { $loot }
        }
    .explanation = This is only a suggestion; nothing is broken. Collections ("all in one") often contain mods that are also offered on their own, and two mods can rebuild the same place in the same way. With both installed, the same buildings stand twice in the same spot and their surfaces flicker. A plugin made for one of the two, such as a compatibility patch, only works with that one, which is a good reason to keep it.
    .remedy = Keep the one you prefer and switch the other off in your mod manager; prefer the one the patches you use are made for. Before you switch one off, check whether it brings something the other lacks, such as an interior: then keep both, and in the list of collisions let one of them lose in every place at once.

plugins-switched-off-but-loaded = A mod you switched off is still in the game
    .headline = { $mod } is switched off, but the game still loads { $plugin }
    .summary =
        { $mod } is switched off in your mod manager, but the game still loads { $plugin }. { $deployed ->
            [1] Your mod manager has not deployed since you switched it off, so its files are still in the game's Data folder: { $files }. Deploy in your mod manager to take them out.
           *[other] { $fileCount ->
                [one] A copy of its plugin is
               *[other] Copies of its files are
            } still in the game's Data folder: { $files }. Your mod manager did not put { $fileCount ->
                [one] it
               *[other] them
            } there: { $fileCount ->
                [one] it was
               *[other] they were
            } in the folder before the mod was installed, so switching the mod off, and deploying, left { $fileCount ->
                [one] it
               *[other] them
            } behind.
        } Until { $plugin } is out of the game, its collisions are real and stay in the list; what you decide about them is kept for when the mod is on again. { $dependentCount ->
            [0] {""}
            [one] One plugin builds on it: { $dependents }. Take that one out as well, or the game will not start once { $plugin } is gone.
           *[other] { $dependentCount } plugins build on it: { $dependents }. Take these out as well, or the game will not start once { $plugin } is gone.
        }
    .explanation = When a mod manager installs a mod, it puts the mod's files into the game's Data folder; when you switch the mod off, it takes them out again the next time it deploys. Until it deploys, the game still loads them. And if a file of the same name was already in the folder before, from an installation by hand or by another tool, the mod manager sets it aside and puts it back when the mod goes. The game then keeps finding the plugin, and the mod manager keeps it in the list of active plugins. Either way, the mod is off in the mod manager but still in the game.
    .remedy = If your mod manager has not deployed yet, deploy. If the files stay, they were not the mod manager's: switch the plugin off in its list of plugins, together with the plugins of other mods that build on it (Vortex can be asked to do this from here). The game then no longer loads it, nor the archive of the same name. To tidy up, move the files named here out of the game's Data folder afterwards, for example into a folder next to it; do not delete them until you are sure nothing is missing. Then scan again. If you want the mod after all, switch it on again in your mod manager instead.

plugins-switched-off-left-behind = Files of a switched-off mod are still in the Data folder
    .headline = { $mod } is out of the game, but { $fileCount ->
        [one] a file of it is
       *[other] { $fileCount } files of it are
    } still in the Data folder
    .summary = The game no longer loads { $plugin }, so { $mod } is out of the game. But { $fileCount ->
        [one] this file is
       *[other] these files are
    } still in the game's Data folder: { $files }. Your mod manager did not put { $fileCount ->
        [one] it
       *[other] them
    } there, so it will not take { $fileCount ->
        [one] it
       *[other] them
    } away. { $fileCount ->
        [one] It does
       *[other] They do
    } no harm while { $plugin } is switched off.
    .explanation = A copy that was in the Data folder before a mod manager installed the mod is set aside while the mod is on and put back when it goes. Such leftovers are easy to forget: if you switch the mod on again, the mod manager sets them aside once more; if you tick its plugin again in the list of plugins, the game loads the old copy again.
    .remedy = Move the files named here out of the game's Data folder, for example into a folder next to it, and scan again. Keep them until you are sure nothing is missing.

patch-hides-walkway = Mod Mallet's patch hides something characters walk on
    .headline =
        The patch hides { $count ->
            [one] an object
           *[other] { $count } objects
        } of { $plugin } that characters walk on
    .summary =
        The paths the game's characters follow run over { $count ->
            [one] an object
           *[other] { $count } objects
        } of { $plugin } that Mod Mallet's patch hides: { $objects }. { $triangles ->
            [one] One piece of path lies
           *[other] { $triangles } pieces of path lie
        } on { $count ->
            [one] it
           *[other] them
        }, in { $placeCount ->
            [one] one place
           *[other] { $placeCount } places
        }: { $places }. These paths come from { $navmeshes }. An interior is given by its internal name, outside by worldspace and cell.{ $unreachable ->
            [0] {""}
            [one] {" "}One place characters walk to, such as a door, a bed or a seat, can only be reached over what is hidden, or stands on it: { $unreachableNames }.
           *[other] {" "}{ $unreachable } places characters walk to, such as doors, beds or seats, can only be reached over what is hidden, or stand on it: { $unreachableNames }.
        }{ $longer ->
            [0] {""}
            [one] {" "}Without the paths over it, characters would reach one other place only by a much longer way.
           *[other] {" "}Without the paths over it, characters would reach { $longer } other places only by a much longer way.
        }
    .explanation = Characters in the game find their way on an invisible net laid over everything they can walk on: the ground, but also bridges, piers, decks and stairs. Mod Mallet's patch hides an object by making it invisible and letting everyone pass through it, but the net stays as it was made. Where the net runs over a hidden deck, pier or bridge, townspeople, guards, followers and animals walk on thin air or get stuck at its edge, and a quest that sends someone across can stop. You yourself fall through where the object was.
    .remedy = Decide what matters more to you here. Keep the object: on the collisions page, let the other mod's object give way instead, or keep both. Or accept it knowingly if nobody needs to walk there, such as on a ship that nobody boards. Or repair the paths: where the warning shows on the collisions page or while going through them, "Repair the paths" takes out the paths over the object and lays new ones on the ground below, and shows both before you choose.
