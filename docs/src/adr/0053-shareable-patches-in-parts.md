# 0053 Shareable patches in parts, with an installer

- Status: accepted; changed by [ADR 0060](0060-objects-moved-a-little-through-a-plugin.md): moving objects through a plugin of overrides; [ADR 0070](0070-several-patches-that-say-what-they-decide.md): several patches, each its own mod
- Date: 2026-11-19
- Changes: [ADR 0025](0025-patches-through-base-object-swapper.md) (one patch file)

## Context

Mod Mallet's patch has so far been one swap file for Base Object Swapper,
`ModMallet_SWAP.ini`, with every decision of the setup in it (ADR 0025).
It cannot be passed on: it is all or nothing, and it says nothing about
who made it or what each line is for.

A patch of Mod Mallet carries no assets of anybody's mod. It is a list of
instructions: which placed object to hide, later also where to move one.
So anybody may make one and share it, on Nexus like any patch: the author
of a mod for other mods, the curator of a collection, or a player who
tidied up a few places. Mods on Nexus usually offer their patches for
other mods as options of their installer, ticked when the other mod is
there and greyed out when it is not. The user wants Mod Mallet's patches
to look and work the same.

What the user asked for, in their words:

1. Anybody can be the "modder": patches are made from the mods in one's
   own setup, and later also from mods loaded as archives or folders
   without installing them.
2. A patch is still one patch with a name, but it is made of parts. Each
   part is about one mod whose objects it changes. All parts are ticked in
   advance; a part that does not apply to a setup cannot be ticked; any
   part can be left out.
3. A patch says where it comes from: one's own or by an author. A
   signature that proves it comes later.
4. Patches made by others are read in: first Mod Mallet's own format,
   then, where possible, other swap files and patch plugins.

## Decision

### A patch has a name and is made of parts

- A **patch** has a name, an author (none for one's own), a version and
  its parts. Patches with different names are different mods in the mod
  manager and can be installed side by side; a patch with the same name
  replaces the one before.
- A **part** is about the plugin whose objects it hides, and, if the
  decision was made against another mod, that mod's plugin (`against`).
  A collision decision gives a part with `against`; an object hidden on
  its own, for example a sign the user does not like, gives a part
  without. Parts are listed grouped by the plugin they change.
- A part with `against` applies only when both plugins are there; one
  without applies when its plugin is there. Plugin file names are what
  setups have in common, so parts name plugins, not the mod manager's
  names of mods.

### The archive carries a FOMOD installer

- Each part is its own swap file, named after the patch and the part
  (`<patch> - <plugin> - <against>_SWAP.ini`), because Base Object Swapper
  reads every `*_SWAP.ini` in the data directory. Its first lines say, as
  comments, which patch, author, version and part it belongs to, so every
  installed file describes itself.
- The archive holds `fomod/info.xml` (name, author, version) and
  `fomod/ModuleConfig.xml`: one page, one group in which any number of
  parts may be chosen. A part is *recommended* (ticked in advance) when its
  plugins are active and *not usable* (greyed out) otherwise.
- So a shared patch works in Vortex, Mod Organizer 2 or any FOMOD
  installer without Mod Mallet. Leaving a part out is done in the
  installer, as with any mod; Mod Mallet can reinstall the patch with
  parts ticked (ADR 0034).
- The archive is written into Mod Mallet's own folder, named after the
  patch, and installed through the link as before (ADR 0020, 0025).

### Mod Mallet reads every patch of its own that is installed

- A scan lists the swap files in the data directory and reads those that
  start with Mod Mallet's description. It reports each patch with its
  name, author, version, parts and the mod it came with
  (`InstalledPatch.patches`). The old `ModMallet_SWAP.ini` is read as a
  patch without parts.
- Until patches made by others are told apart from one's own (below),
  the objects of every Mod Mallet patch count as installed.

### What comes first, and what later

The first step builds the format above: the patch page sends its
decisions as parts, the archive is written with its installer, and a scan
reads the installed patches with their parts. The patch keeps the name
"Mod Mallet Patch" and no author until the page can set them. The later
steps, in this order:

1. **Parts in the patch page.** Name, author and version are set on the
   page; the page lists the parts of each installed patch and which of
   them were left out.
2. **Patches by others.** A patch whose author is not the user is shown
   as theirs: its parts count as decided, but the user's own decisions are
   not replaced by it, and a part can be switched off by reinstalling
   without it.
3. **Other swap files and patch plugins.** Lines of other `*_SWAP.ini`
   files that hide objects, and patch plugins that disable or move placed
   objects, count as decisions someone already made, so Mod Mallet does not
   propose them again.
4. **Moving objects.** Base Object Swapper can also change where a placed
   object stands (its `[Transforms]` section); a part may then move an
   object instead of hiding it. This needs a decision of its own about
   how a position is chosen.
5. **Patches from archives.** A patch is made for mods chosen as archives
   or folders, read like installed ones, without installing them.
6. **A signature.** An author signs a patch with a key, and Mod Mallet
   checks it, so that the author it names is proven. A patch changed by the
   user, for example with parts left out and written anew, becomes their
   own.

## Consequences

- A patch can be uploaded to Nexus as it is, and installs there like any
  other patch with options.
- The data directory holds one swap file per part instead of one in all.
  Base Object Swapper reads them all at start; the number of lines is the
  same.
- A part that names a plugin by a file name the other setup does not use
  is greyed out there, even if the same mod is installed under another
  plugin name.
- The author field is plain text until signatures exist. Anybody can
  write any name into it; Mod Mallet says so where it shows it.

## Alternatives considered

- **One patch per pair of mods.** Simpler to read, but dozens of small
  mods in the mod manager; the user wants one patch with options.
- **Opting out only inside Mod Mallet.** It would keep the archive
  simple, but a shared patch would then need Mod Mallet to be useful.
- **Conditions inside the swap file.** A line about a plugin that is not
  loaded does nothing, but a swap file has no way to skip a line because
  *another* plugin is missing, which a part with `against` needs.
