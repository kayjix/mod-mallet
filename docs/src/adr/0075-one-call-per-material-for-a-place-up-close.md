# 0075 A place up close is drawn with one call per material

- Status: accepted
- Date: 2026-11-22
- Changes: ADR 0038 (a place up close), decision 5

## Context

Up close the 3D views draw every object placed around a point with its own
mesh (ADR 0038). Each model was drawn once for all its objects
(instancing), but each of its shapes was a draw call of its own, with its
own vertex array, its own buffers and its texture bound anew. A place of a
real setup has a few thousand objects of several hundred models, most of
them used once, each of several shapes: a few thousand draw calls per
view. The comparison of the guided review (ADR 0050) draws the place three
times, and wrote every object's look anew into every model's buffer for
each of the three views, every frame.

The user's test case, JK's Sky Haven Temple against JK's Skyrim, lagged
badly, most in the comparison. The graphics card was in use (WebView2's
GPU process on the user's card, at about 12 % load). Measured with a made-up
heavy place of the same kind (`?heavy` in the development build, about
3 000 objects of 400 models, three textured shapes each), a frame of the
comparison took 6.5 ms in JavaScript and 12.4 ms on the card: 3 834 draw
calls and 1 290 buffer uploads per frame. The card spent its time on the
calls, not on the triangles.

## Decision

1. **All meshes of a view share one set of buffers.** A model's shapes
   are added to one pool of points and triangles as they arrive, sorted by
   material; shapes of one model with the same texture become one range.
2. **Where an object stands and how it looks are kept in two small
   textures**, one matrix and one look per object, in an order where the
   objects of one model follow each other. A shader reads them by the
   number of the object.
3. **One call draws every model that uses a material**: the browser's
   `WEBGL_multi_draw` hands many draws to the card at once, each with the
   first object of its model. Where the browser lacks it, the same data is
   drawn with one call per range, still without switching buffers.
4. **What the camera does not see is left out**: models whose objects are
   all outside the view, or all less than 1.5 pixels across, are not
   drawn. Hidden objects that the view leaves out are dropped before their
   pixels are coloured.
5. **A view can show the same objects with up to four sets of looks.** The
   comparison's three views keep one set each; a look is sent to the card
   only when it changed. Picking reads the look of the view clicked.
6. Textures are decoded away from the page's own thread
   (`createImageBitmap`), where the browser can.
7. **Finding a comparison's structures does not keep the page busy.**
   Growing a structure rules out parts too far apart by their spheres
   before the box test, with a bound that never changes the answer; each
   part's facts and box are worked out once while the objects stay; what
   of a structure lies in a passage is worked out once per structure, not
   once per spot. While models arrive, the comparison waits at least four
   times as long as its last try took before it tries again.
8. In a development build, `__malletBench()` in the console measures what
   a frame of each open view costs (JavaScript, card, draw calls, uploads);
   a built app does so when `mallet.renderStats` is `1` in its local
   storage. The settings' About page names the graphics card that draws
   the 3D views, and says when there is none and the processor draws.

## Consequences

- With the heavy place, a frame of the comparison takes 1.7 ms in
  JavaScript and 2.3 ms on the card: 483 draw calls and no uploads. The
  single map view went from 1 257 calls to 164.
- On the user's setup, at JK's Sky Haven Temple against JKs Skyrim, the
  comparison drew a frame with 522 calls in 1.9 ms of JavaScript and
  2.9 ms on the card, and the single map view at the forge with 844 calls
  in 2.8 ms and 2.0 ms. Before, the comparison never finished loading
  there: finding its structures ran again with every model that arrived
  and kept the page busy for more than fourteen minutes. Now the place is
  loaded in about nine seconds, a try takes at most half a second, and the
  structures found are the same (205 and 188 parts).
- Pictures stay the same: drawn both ways, the comparison differed in 18
  of 493 500 pixels, all of objects smaller than a pixel and a half.
- The number of calls now grows with the number of materials, not of
  shapes. A place of many objects with textures of their own still needs
  many calls; a texture array would join them, at the cost of giving every
  texture the same size.
- Models stay in the pool as long as the view lives, as they stayed in
  their own buffers before.
