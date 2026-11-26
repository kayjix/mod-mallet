# 0025 Patches hide objects through Base Object Swapper

- Status: accepted; changed by [ADR 0040](0040-collisions-as-a-worklist.md): a spot can have a choice of its own; [ADR 0046](0046-why-the-game-relies-on-an-object.md): which objects the game relies on; [ADR 0051](0051-paths-over-hidden-objects.md): the paths that run over hidden objects; [ADR 0052](0052-paths-over-hidden-objects-are-cut-out.md): a plugin that cuts paths over hidden objects out of the navigation meshes; [ADR 0053](0053-shareable-patches-in-parts.md): a patch has a name and parts, with an installer; [ADR 0060](0060-objects-moved-a-little-through-a-plugin.md): objects moved a little through a plugin of overrides; [ADR 0067](0067-stacked-doors-are-merged-into-one.md): what hiding a door leaves behind is said, and the changes it needs are planned
- Date: 2026-11-15

## Context

When two mods place the same object twice, or a mod is installed both on
its own and as part of a collection, the fix is to hide one copy. The
usual way is a patch plugin that overrides each object to switch it off.
Writing such a plugin correctly is harder than it looks:

- An override of an object has to sit inside copies of its cell and its
  worldspace. Those copies have to be the versions that win in the load
  order, or the patch quietly undoes other mods. On a real setup 117
  plugins carry their own copy of the main worldspace's record, and the
  winning one sets the water of the whole world.
- Every reference to another record inside those copies has to be
  rewritten for the patch's own list of masters, and which bytes are such
  references differs per field.
- A switched-off object stays switched off in every save made with the
  patch, even after the patch is removed.

Base Object Swapper, a widely used script extender plugin, swaps the base
object of a placed object while the game runs. It reads plain text files
named `*_SWAP.ini`, in which a line names one placed object by its number
and plugin and what to show instead. Many setups have it already.

## Decision

- A patch of Mod Mallet is a **swap file for Base Object Swapper**: one
  line per object to hide, swapping it for the game's invisible marker
  (`XMarker`). No plugin is written and no record is copied.
- Mod Mallet writes the file into a zip archive in its own folder
  (`%LOCALAPPDATA%\mod-mallet\patches`) and asks Vortex, through the link,
  to install it like a downloaded mod. The extension installs archives
  only from that folder. Nothing is written into the game or Vortex's
  data by Mod Mallet itself.
- Only objects that nothing else relies on are offered: not persistent
  ones, which quests and scripts may refer to, and not large references,
  which the worldspace's distant view is built around.
- For each collision the user chooses which side disappears. Of two
  copies, the side of the plugin that places fewer objects is suggested.
- The patch view needs Base Object Swapper and says so when it is missing.

## Consequences

- Removing the patch mod in Vortex brings every object back. Saves are not
  changed by it.
- Objects are hidden, not removed: the invisible marker stays in their
  place. Distant views generated from the load order (for example by
  DynDOLOD) may still show them until they are generated again.
- Every new patch replaces the previous one; Vortex asks whether to
  replace the installed mod.
- A scan reads the swap file that is in the game and reports the objects
  it hides and the mod it came with (`ScanReport.installed_patch`). The
  patch view starts from that choice and says where today's collisions
  differ from it.
- Patches that change records, such as writing a plugin, remain possible
  later and need a decision of their own.

## Alternatives considered

- **A patch plugin that switches objects off.** The standard tool's way,
  with the pitfalls above; it needs the winning cell and worldspace
  records copied with every reference rewritten.
- **A patch through SkyPatcher.** Also text-based, but its support for
  single placed objects could not be confirmed.
