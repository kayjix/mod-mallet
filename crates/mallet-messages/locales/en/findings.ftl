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
    .explanation = Two mods that change the same place each put their own buildings, trees or other objects there, without knowing of each other. Where they meet, a tree grows through a roof, two houses merge into one, or a path runs into a wall. Mod Mallet compares the box around each object's model, not the model itself, so an object with a lot of empty space, such as an arch, can be reported although nothing visibly collides.
    .remedy = Look on the pages of both mods for a compatibility patch. If there is none, look at the places named above in the game. If something is in the way, decide which of the two mods you want to keep for this place.

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
