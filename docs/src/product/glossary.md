# Glossary

Modding has its own vocabulary, and so does this project. Terms are listed
alphabetically within each group.

## Modding terms

**Address Library**
: A mod that lets script extender plugins find functions inside the game
  without being rebuilt for every game update.

**Archive (BSA, BA2)**
: A file that packs many assets into one. A plugin of the same name makes the
  game load it.

**Asset**
: A file the game displays or plays: a mesh, a texture, a sound, an
  animation.

**Cell**
: A square piece of a worldspace, or one interior. The game loads and saves
  the world cell by cell.

**Deployment**
: What a mod manager does to make enabled mods visible to the game. Vortex
  links each mod's files into the game directory and records what it linked.

**FOMOD**
: An installer format. The mod author describes options, and the mod manager
  shows them as a dialog during installation. Which options are preselected
  often depends on the mods already installed.

**FormID**
: The number that identifies a record. Its first part depends on the
  plugin's position in the load order, so the same record has different
  FormIDs in different setups.

**Generator**
: A tool whose output is derived from the whole load order, such as LOD or
  animation generators. The output must be regenerated when the mods change.

**Load order**
: The order in which the game loads plugins. When two plugins change the same
  record, the later one wins completely.

**LOD (level of detail)**
: Simplified versions of terrain and objects shown at a distance.

**Loose file**
: An asset that lies in the game directory as an ordinary file. A loose file
  always wins over the same file inside an archive.

**Master**
: A plugin that another plugin builds on and therefore requires.

**Mesh (NIF)**
: The 3D model of an object. It names the textures it uses by path.

**Navmesh**
: The surface non-player characters can walk on, stored per cell.

**Override**
: A record in one plugin that replaces a record of the same FormID from an
  earlier plugin.

**Plugin (ESP, ESM, ESL)**
: A file of records that changes or adds to the game's data.

**Record**
: One entry in a plugin: a weapon, a character, a placed object, a cell.

**Script extender (SKSE, F4SE)**
: A loader that starts the game and lets mods run native code. Its plugins
  are DLL files built for specific game versions.

**Staging folder**
: Where a mod manager keeps each installed mod in its own directory.

**Worldspace**
: A contiguous outdoor world, such as the province of Skyrim or the inside of
  a walled city.

## Mod Mallet's terms

**Action**
: A proposed change that fixes a finding. An action can be previewed and
  undone, and never alters an original mod file.

**Adapter**
: Code that knows one game or one mod manager and translates it into facts.

**Analysis**
: A pure function from facts to findings.

**Confidence**
: How sure Mod Mallet is about a finding: *certain*, *likely* or *hint*.

**Fact**
: Something read from the setup, before any judgement.

**Finding**
: One problem Mod Mallet reports. It has a kind, a subject, a severity and a
  confidence.

**Kind**
: The category of a finding, for example `extender.plugin-incompatible`. All
  kinds are listed in the [finding catalog](../reference/findings.md).

**Scan**
: One run of ingest and analysis. A scan only reads.

**Subject**
: What a finding is about, such as a file.
