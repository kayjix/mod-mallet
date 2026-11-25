# 0085 Pictures of meshes are drawn in the engine; the relief reads its textures from an index

- Status: accepted
- Date: 2026-11-25

## Context

The lists of the assets page and of the catalogue show textures as small
pictures, but every mesh as a grey "3D". Someone choosing between two
versions of a brazier has to open each one to see what it is. To the user
it looked as if many models did not load at all.

A count over the user's real setup (48,405 meshes, the versions the game
uses) showed what the 3D viewer can and cannot draw:

| | meshes |
| --- | ---: |
| drawn | 45,603 (94 %) |
| skeletons: bones only | 1,711 |
| glow, mist, flames, particles, editor markers with an effect shader | about 830 |
| cameras, empty nodes, markers | about 120 |
| `BSLODTriShape` (trees and bushes in the first Skyrim's format) | 37 |
| other surfaces not read | about 10 |

Of 217,587 surfaces, 863 name no texture, 682 name a texture the setup
does not have, and 35 use one texture of 89 MB, larger than the 64 MB that
are read for showing. Only the `BSLODTriShape` was a gap in our reader.

The 3D relief of the map loaded slowly the first time. Measured around
Whiterun as Map collisions shows it (123 blocks of level of detail, 130
textures): reading and taking apart the blocks takes about 70 ms; the
textures took 2.3 s in the engine and make 76 MB of pictures for the
window. Textures outside the worldspace's own folder, such as of water and
mountains, were looked for in every archive of the setup again, 300 to
550 ms each. The 3D viewer did the same for every texture of a mesh.

## Decision

1. **The engine draws the pictures.** `mesh_thumbnails` reads each mesh
   version with the reader the 3D viewer uses, decodes its textures at no
   more than 128 pixels, and draws it with a small software renderer
   (`raster.rs`): textured triangles from the 3D viewer's starting angle,
   orthographic and fitted to the frame, a depth buffer, the viewer's
   default light, two-by-two supersampling and a transparent background.
2. **Pictures are kept in the cache store**, namespace `mesh-thumbnail`,
   by size, path and version, with the setup's fingerprint and the
   renderer's version as the fingerprint. A changed setup draws them again
   as rows come into view.
3. **A mesh without a picture says why**: missing, unreadable, skeleton,
   only effects, nothing to see, or surfaces Mod Mallet cannot draw yet.
   The list shows a sign for each with the reason as its tooltip.
4. **The window asks lazily**: a slot asks when it comes near the view,
   six pictures per call, two calls at once, the newest rows first; a row
   scrolled away before its turn is not asked for. A shimmer stands in
   while it is drawn.
5. **Every texture of the relief and of the 3D viewer is read through an
   index** of the setup's meshes and textures, made once per setup (the
   one a place up close already uses), not by opening every archive again.
6. **The large shared textures of the relief are kept between runs**:
   those asked for at 1024 pixels or more (the atlas of the objects and
   the textures of mountains, water and buildings that many blocks share;
   13 in all of Tamriel). The textures of single land blocks are quick to
   make and are not kept, which keeps the cache small.
7. The map's "Game detail" switch says how many of the blocks and textures
   the view wants are there while they load.

## Consequences

- Lists show what each mesh looks like. A picture takes 7 to 35 ms the
  first time (a dragon is the slowest seen), under a millisecond from the
  cache; the first call of a session also finds the setup's files, about
  a second, which the catalogue shares.
- The renderer is plain Rust: no graphics context per list, nothing on the
  window's thread, and the CLI could use it too. It draws less than the 3D
  viewer: no face tint, no normal maps, flat light.
- The textures of the relief around Whiterun: 2.3 s before, 1.6 s with the
  index, 0.4 s on the next run with the kept textures (all one after the
  other; the window asks for several at once). Water's texture alone went
  from 520 ms to 25 ms on the next run.
- The cache grows by about 8 MB for each large shared texture, some
  100 MB for all of Tamriel at most.
- `BSLODTriShape` is read as the `NiTriShape` it extends; 37 meshes more
  are drawn, bushes and trees of Cyrodiil mods among them.

## Alternatives considered

- **Drawing the pictures in the window**, with WebGL in a worker or a
  hidden shared canvas. It would reuse the viewer's shaders, but every
  picture would first send the whole mesh and its textures at 512 pixels
  through the window's channel and be decoded there; the engine has both
  at hand. Browsers also limit how many graphics contexts a page may have.
- **Keeping every texture of the relief.** The textures of single land
  blocks would fill the cache with hundreds of megabytes while costing
  10 ms each to make.
- **Keeping the blocks of the relief.** Reading and taking them apart
  takes 70 ms for a whole view; keeping them would save little.
- **Sending smaller textures to the relief.** It would be faster still,
  but would change how the relief looks, which the user is happy with.
