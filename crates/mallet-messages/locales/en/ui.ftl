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
origin-given = named by you
origin-registry = from the entry the game's launcher wrote into the Windows registry
origin-standard = at its standard place
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
tool-vc-runtime-purpose = A Windows component almost every SKSE plugin is built with. Without it, those plugins do not load.
tool-base-object-swapper-name = Base Object Swapper
tool-base-object-swapper-purpose = An SKSE plugin that swaps objects while the game runs. The patches Mod Mallet makes use it to hide colliding objects.
tool-creation-kit-name = Creation Kit
tool-creation-kit-purpose = Bethesda's editor for making mods and looking inside them. Not needed to play.
tool-loot-name = LOOT
tool-loot-purpose = Sorts plugins and shows notes on them. Vortex sorts with LOOT's rules by itself; the program on its own adds its notes on each plugin.
tool-seven-zip-name = 7-Zip
tool-seven-zip-purpose = Opens mod archives. Mod Mallet uses it to read the installers of your mods; Vortex brings one along.

tool-state-ready = Ready
tool-state-missing = Missing
tool-state-outdated = Needs an update
tool-state-stopped = Not running
tool-need-required = Required
tool-need-recommended = Recommended
tool-need-optional = Optional
