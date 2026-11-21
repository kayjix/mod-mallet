### Texts that are not tied to one kind of finding: labels and report lines
### shared by the command line and the desktop app.

severity-error = Error
severity-warning = Warning
severity-info = Note

confidence-certain = certain
confidence-likely = likely
confidence-hint = hint

label-file = File
label-mod = Mod
label-remedy = What to do
label-kind = Kind
label-severity = Severity
label-confidence = Confidence

location-game-dir = Game directory
location-plugin-list = List of active plugins
location-extender-log = Script extender log
location-deployment-manifest = Vortex's list of deployed files
location-manager-state = Vortex's saved state
location-downloads = Vortex's downloaded mod archives
location-unpacker = Program for looking into mod archives
location-masterlist = LOOT's list of known plugin problems (masterlist)
location-crash-logs = Folder of crash reports
location-mod-staging = Vortex's folder of installed mods (the names of their files, and the files of the things you look at under Assets)
origin-given = named by you
origin-registry = from the entry the game's launcher wrote into the Windows registry
origin-standard = at its standard place
origin-steam = found in one of your Steam libraries
origin-gog = from the entry GOG Galaxy wrote into the Windows registry
origin-manager = named in Vortex's list of deployed files
location-not-found = not found

report-game = { $name } { $version }
report-game-unknown-version = { $name } (version unknown)
report-extender = script extender { $version }
report-plugins-checked =
    { $count ->
        [one] 1 script extender plugin checked
       *[other] { $count } script extender plugins checked
    }
report-files-read =
    { $plugins ->
        [one] 1 plugin
       *[other] { $plugins } plugins
    } and { $archives ->
        [one] 1 archive
       *[other] { $archives } archives
    } read
report-installers-read =
    { $count ->
        [one] 1 mod installer read
       *[other] { $count } mod installers read
    }
report-meshes-compared =
    { $count ->
        [one] 1 replacement mesh compared with its original
       *[other] { $count } replacement meshes compared with their originals
    }
report-no-extender-log = The script extender has not written a log, so its plugins were not checked.
report-no-findings = No problems found.
report-totals =
    { $errors ->
        [one] 1 error
       *[other] { $errors } errors
    }, { $warnings ->
        [one] 1 warning
       *[other] { $warnings } warnings
    }, { $notes ->
        [one] 1 note
       *[other] { $notes } notes
    }
report-notes-hidden = Notes are not listed. Add --notes to see them.
report-explain-hint = More about a finding: mallet explain <kind>
report-no-scan = No scan has been stored yet. Run "mallet scan" first.

explain-unknown-kind = There is no kind of finding called "{ $kind }". Run "mallet kinds" to list them.

## Actions: what can be done about a finding, and what became of it.

label-action = Fix from here

action-reinstall-mod = Reinstall in Vortex
    .description = Vortex opens the installer of { $mod } again, with your earlier choices filled in. Go through it page by page and tick the options listed here; they are not ticked for you. Nothing changes until you finish the installer there.

guide-page = Page { $page } of { $pages }

action-hide-objects = Install the patch in Vortex
    .description = Mod Mallet writes a small mod that shows an invisible marker in place of { $count ->
        [one] 1 object
       *[other] { $count } objects
    } while the game runs, and Vortex installs it. It needs Base Object Swapper. Remove the mod in Vortex to bring the objects back.

action-move-objects = Install the moves in Vortex
    .description = Mod Mallet writes a small plugin that puts { $count ->
        [one] 1 object
       *[other] { $count } objects
    } a little to one side, and Vortex installs it. Remove the mod in Vortex to put them back where they were.

action-load-after = Let { $plugin } win
    .description = Vortex gets a rule to load { $plugin } after { $after } and sorts your plugins again. What both change then comes from { $plugin }; whatever only { $after } changes stays. You find the rule, and can remove it, in Vortex's plugin list.

action-disable-mod = Disable { $mod }
    .description = Vortex switches { $mod } off in the profile you use, and asks to deploy. Nothing is deleted; switch it on again in Vortex to undo it.

action-enable-mod = Switch on { $mod }
    .description = Vortex switches { $mod } on in the profile you use, and asks to deploy. Switch it off again in Vortex to undo it.

action-file-overrides = Use { $mod }'s version
    .description = Vortex leaves { $count } { $count ->
        [one] file
       *[other] files
    } of this thing out of the other mods that bring { $count ->
        [one] it
       *[other] them
    }, so that those of { $mod } reach the game, and deploys again. Only these files change. You find this, and can take it back, among the mods' file conflicts in Vortex.

action-file-overrides-archives = Use the version in the archives
    .description = Vortex leaves { $count } { $count ->
        [one] file
       *[other] files
    } of this thing out of the mods that bring { $count ->
        [one] it
       *[other] them
    } loose, so that the version in the archives shows, and deploys again. You find this, and can take it back, among the mods' file conflicts in Vortex.

action-file-overrides-reset = Take the choice back
    .description = Vortex deploys { $count } { $count ->
        [one] file
       *[other] files
    } of this thing from every mod that brings { $count ->
        [one] it
       *[other] them
    } again; the order of the mods decides once more.

## Duels: everything two mods disagree about.

duel-topic-faces = { $count ->
    [one] 1 face
   *[other] { $count } faces
}
duel-topic-textures = { $count ->
    [one] 1 texture
   *[other] { $count } textures
}
duel-topic-meshes = { $count ->
    [one] 1 mesh
   *[other] { $count } meshes
}
duel-topic-objects = { $count ->
    [one] 1 object
   *[other] { $count } objects
}
duel-topic-contained = { $count ->
    [one] 1 contained plugin
   *[other] { $count } contained plugins
}
duel-topic-files = { $count ->
    [one] 1 other file
   *[other] { $count } other files
}
duel-subject-faces = faces
duel-subject-textures = textures
duel-subject-meshes = meshes
duel-subject-objects = placed objects
duel-subject-contained = plugins
duel-subject-files = other files
duel-suggest-built-on = { $winner } is built on { $other }: it was made to change it, so it should win.
duel-suggest-specialist = { $winner } is about { $subject } above all, for { $other } they are a small part. The specialist usually does its own thing better.

outcome-applied = Vortex has made the change. Deploy if Vortex asks for it; Mod Mallet scans again afterwards.
outcome-installing = Vortex is installing the patch. Deploy if Vortex does not do it by itself; Mod Mallet scans again afterwards.
outcome-started = Vortex has opened the installer. Continue there, and scan again when you are done.
outcome-link-missing = This needs a small extension in Vortex, which is not installed yet.
outcome-manager-not-running = Vortex is not answering. Start Vortex, or restart it if the extension was only just installed, and try again.
outcome-no-answer = Vortex did not answer in time. Check that Vortex is open and not waiting in a dialog, then try again.
outcome-wrong-game = Vortex is managing another game at the moment. Switch Vortex to this game and try again.
outcome-unknown-mod = Vortex no longer knows this mod. Scan again.
outcome-archive-missing = Vortex no longer has the archive this mod was installed from. Download the mod again in Vortex first.
outcome-unsupported = The extension in Vortex does not know this request. Install the extension again and restart Vortex.
outcome-failed = That did not work: { $detail }

link-ready = The Vortex extension is installed and Vortex is answering.
link-not-running = The Vortex extension is installed, but Vortex is not answering. Start Vortex, or restart it if the extension was only just installed.
link-restart-needed = Vortex is still running an older version of the extension. Restart Vortex to load the new one.
link-missing = The Vortex extension is not installed.
link-explanation = Mod Mallet does not change your mods itself. So that Vortex can do it, Mod Mallet puts a small extension into Vortex's plugin folder. The extension only acts when you press a button in Mod Mallet, and it does what Vortex's own menu entry does.
link-installed = The extension was installed into { $path }. Restart Vortex to load it.

## The toolbox: what the setup needs besides its mods.

tool-vortex-name = Vortex
tool-vortex-purpose = The mod manager. Mod Mallet reads which mod brought which file, and hands every change to Vortex.
tool-mallet-link-name = Mod Mallet link in Vortex
tool-mallet-link-purpose = A small Vortex extension. Through it, the buttons in Mod Mallet can ask Vortex to reinstall a mod or install a patch.
tool-skse-name = Skyrim Script Extender (SKSE)
tool-skse-purpose = Lets mods add program code to the game. Needs a version made for exactly your game version; Steam updates break it until SKSE catches up.
tool-address-library-name = Address Library for SKSE Plugins
tool-address-library-purpose = Tells SKSE plugins where things are in your game version, so they keep working after small game updates. Most SKSE plugins need it.
tool-vc-runtime-name = Microsoft Visual C++ Redistributable
tool-vc-runtime-purpose = A Windows component almost every script extender plugin is built with. Without it, those plugins do not load.
tool-base-object-swapper-name = Base Object Swapper
tool-base-object-swapper-purpose = An SKSE plugin that swaps objects while the game runs. The patches Mod Mallet makes use it to hide colliding objects.
tool-creation-kit-name = Creation Kit
tool-creation-kit-purpose = Bethesda's editor. You need it to repair the paths characters walk on (navigation meshes), which Mod Mallet can only point at; to look at a place as the game builds it, with its lighting; and to make mods of your own. Not needed to play or to use Mod Mallet.
tool-skse-vr-name = Skyrim VR Script Extender (SKSEVR)
tool-skse-vr-purpose = Lets mods add program code to Skyrim VR. Needs a version made for exactly your game version.
tool-skse-classic-name = Skyrim Script Extender (SKSE)
tool-skse-classic-purpose = Lets mods add program code to the 2011 Skyrim. That game no longer changes, so one version fits.
tool-f4se-name = Fallout 4 Script Extender (F4SE)
tool-f4se-purpose = Lets mods add program code to the game. Needs a version made for exactly your game version; Steam updates break it until F4SE catches up.
tool-f4se-vr-name = Fallout 4 VR Script Extender (F4SEVR)
tool-f4se-vr-purpose = Lets mods add program code to Fallout 4 VR. Needs a version made for exactly your game version.
tool-nvse-name = New Vegas Script Extender (xNVSE)
tool-nvse-purpose = Lets mods add program code to the game. Most larger New Vegas mods need it.
tool-fose-name = Fallout Script Extender (FOSE)
tool-fose-purpose = Lets mods add program code to Fallout 3. Many larger mods need it.
tool-obse-name = Oblivion Script Extender (OBSE)
tool-obse-purpose = Lets mods add program code to Oblivion. Many larger mods need it.
tool-obse64-name = Oblivion Remastered Script Extender (OBSE64)
tool-obse64-purpose = Lets mods add program code to the old game inside Oblivion Remastered. It runs with the Steam version only, and needs a version made for exactly your game version.
tool-ue4ss-name = UE4SS
tool-ue4ss-purpose = Loads mods for the Unreal Engine side of Oblivion Remastered: the ones in the LogicMods folder, and mods with scripts. Mods that only replace pictures or models do not need it.
tool-address-library-vr-name = VR Address Library for SKSEVR
tool-address-library-vr-purpose = Tells plugins made for Skyrim Special Edition where things are in Skyrim VR, so that many of them work there too.
tool-address-library-f4se-name = Address Library for F4SE Plugins
tool-address-library-f4se-purpose = Tells F4SE plugins where things are in your game version, so they keep working after small game updates. Most F4SE plugins need it.
tool-geck-name = G.E.C.K.
tool-geck-purpose = Bethesda's editor for Fallout 3 and New Vegas. You need it to repair the paths characters walk on (navigation meshes), to look at a place with its lighting, and to make mods of your own. Not needed to play or to use Mod Mallet.
tool-construction-set-name = Construction Set
tool-construction-set-purpose = Bethesda's editor for Oblivion. You need it to look at a place with its lighting and to make mods of your own. Not needed to play or to use Mod Mallet.
tool-loot-name = LOOT
tool-loot-purpose = Sorts plugins and keeps a list of known problems with them. Vortex has LOOT built in and keeps that list up to date; Mod Mallet reads it to check your plugins. Without Vortex, LOOT on its own does the same.
tool-seven-zip-name = 7-Zip
tool-seven-zip-purpose = Opens mod archives. Mod Mallet uses it to read the installers of your mods; Vortex brings one along.
tool-xedit-name = xEdit (SSEEdit)
tool-xedit-purpose = Looks inside plugins and can repair them. Its guided variants clean a plugin of leftovers that crash the game, or build a patch that makes two mods work together. Start it from Vortex's dashboard so that it sees your mods.
tool-xedit-clean-name = xEdit Quick Auto Clean
tool-xedit-clean-purpose = Cleans a plugin of leftovers that can crash the game, in a few clicks. Mod Mallet points you to it when LOOT says a plugin needs cleaning. Start it from Vortex's dashboard so that it sees your mods.
tool-wrye-bash-name = Wrye Bash
tool-wrye-bash-purpose = Combines the small changes of many mods into one plugin (a Bashed Patch), so that they work together and you stay under the plugin limit.
tool-fnis-name = Fores New Idles in Skyrim (FNIS)
tool-fnis-purpose = Makes the game aware of the new animations that mods bring, such as walking or fighting styles. It has to run again after you add or remove such a mod, or the characters stand stiff.
tool-bodyslide-name = BodySlide
tool-bodyslide-purpose = Builds the bodies and outfits of mods that come in several shapes to your liking. The result is made new each time you change a setting.
tool-mod-mallet-tool-name = Mod Mallet
tool-mod-mallet-tool-purpose = Lets you start Mod Mallet from Vortex's dashboard, beside your other tools. Vortex only changes its list when you press the button here.
tool-vortex-tool-purpose = A program that Vortex lists among your tools. Mod Mallet does not know what it is for; start it from Vortex's dashboard.

tool-state-ready = Ready
tool-state-missing = Missing
tool-state-outdated = Needs an update
tool-state-stopped = Not running
tool-need-required = Required
tool-need-recommended = Recommended
tool-need-optional = Optional
tool-provided-by = Built into { $tool }
