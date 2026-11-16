# 0033 Mods are compared on a stage and decided by a ranking

- Status: accepted
- Date: 2026-11-16
- Changes: ADR 0029 (duels and a 3D view of single things)

## Context

With duels (ADR 0029) a real setup still has well over a hundred pairs of
mods to decide. The inline comparison under each duel card showed a few
files without names; a pair of texture mods could not be seen in 3D at
all; and the same preference ("this face overhaul beats everything",
"this patch collection should lose everywhere") had to be clicked again
in every pair. The overview tiles above the task list took more room
than the tasks.

## Decision

1. **A comparison view in three columns.** Left the pairs, filterable by
   topic, name and object; middle the thing they disagree about; right
   every file of the pair with the name of the object that uses it and its
   kind, each column scrolling on its own. A duel lists its files
   (`ModDuel::files`), named from the base objects whose model is the file
   (`BaseObject::name` from `FULL` when the plugin is not localized, else
   the editor ID).
2. **One stage, one camera.** Both sides are drawn under a camera they
   share: turning, zooming and resetting one does the same to the other.
   The stage has controls for brightness, where the light stands (around
   and height), turning and a reset. A few generic scenes (dark room,
   white room, meadow, forest, cave, snow) set the background and the light
   around the thing. They are painted gradients and ambient colours, not
   game assets.
3. **Textures on a ball.** A texture without a mesh is wrapped around a
   ball, so that texture pairs can be compared on the same stage, next to
   the flat picture.
4. **A ranking.** The user orders mods by preference. A marker stands for
   all mods not placed, so that a single mod can be put below everything
   else without ordering the rest. In each pair the side higher up wins.
   One button decides all open pairs the ranking decides, after a
   confirmation; it stops at the first answer that says the mod manager
   cannot be reached. Pairs decided by hand stay as they are. The ranking
   is kept in the settings next to the decisions.
5. **The task list is arranged by kind or by pair**, switchable; the
   overview is one compact line of counts.

## Consequences

- The stage still is no renderer of the game: no normal maps, no shadows,
  no game lighting. An ENB of the setup is not read; translating its
  settings into the stage's light is possible later but not done.
- Object names come only from plugins that are not localized; localized
  plugins show the internal name.
- A ranking that decides many pairs at once hands many rules to the mod
  manager. Every rule can be taken back pair by pair.
