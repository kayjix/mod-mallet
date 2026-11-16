# 0028 Findings show the textures they are about

- Status: accepted
- Date: 2026-11-16

## Context

Findings about faces, textures and meshes said in words that two mods
disagree about how something looks. To decide which to keep, the user
needs to see it. ADR 0011 rules out drawing the game world in 3D, and
leaves open whether single things may be shown.

Most textures of current mods are BC7. A face texture of the game is the
unwrapped skin of one character's head: not a face, but enough to tell a
dark, a pale and a painted version apart.

## Decision

1. **Textures are shown as flat pictures.** A finding names the files
   worth looking at (`Finding::previews`): the face textures of a few of
   the characters it is about, a few of the textures a mod replaces, the
   textures of a replaced mesh and of the mesh that replaces it.
2. **Every version side by side.** For a named file, the window shows
   each version the setup has, from the archives the game loads in their
   order and the loose file, with the mod each comes from, and marks the
   one the game uses. Nothing is rendered, lit or put onto a mesh.
3. **Decoding is ours.** The texture reader decodes BC1, BC3 and BC7 and
   takes the largest mipmap level that fits 256 pixels. Files are read from
   the user's installation only when a finding is opened.
4. **3D stays a decision of its own.** Showing a head mesh with its
   texture, or any object, would be the first 3D view; ADR 0011 still
   excludes it until that is decided.

## Consequences

- The pictures come from the user's own files and are never stored or
  sent anywhere.
- Formats other than BC1, BC3 and BC7 show "no picture".
- A face texture shows colour and markings, not the shape of a face; a
  wrong head shape is not visible this way.
