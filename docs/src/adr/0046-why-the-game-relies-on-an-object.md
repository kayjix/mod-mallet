# 0046 Why the game relies on an object, and when it may be hidden all the same

- Status: accepted
- Date: 2026-11-18

## Context

Any object of the 3D view can be hidden through the patch (ADR 0025,
0040). Objects the game relies on were locked by two flags of the placed
object: persistent (the game keeps it loaded because something refers to
it) and visible when distant (it is part of the distant view). The user
sees a lock but not its reason, and many locks are needlessly strict. A
real example: a wall stone of JK's Whiterun Outskirts by the city gate
looks like the base of a gate that a mod took away, so it seems it could
go too. It turns out to be locked only because it is drawn from afar.

Hiding does not remove an object. Base Object Swapper swaps what is placed
for the game's invisible marker while the game runs (ADR 0025): the placed
object stays where it is, keeps its links, its switched-on state, its
quest roles and the scripts placed on it. What goes is its look, its
collision and what its base object does: a door's way through, a chest's
contents, a chair to sit on, a lever to pull. So the question is not
whether something points at an object, but what pointing at it needs from
it.

Who points at an object can be anywhere in the load order: in another
cell, in a quest, in a location, in a character's AI package. The reader
of a place (ADR 0038) reads only the cells around a point, which is not
enough.

## Decision

1. **The game crate reads, once per setup, every pointer at placed
   objects** (`mallet_game_bethesda::anchors`), in the versions the load
   order gives the records:
   - of placed objects (`REFR`, `ACHR` and the other placed kinds): the
     enable parent (`XESP`), linked references with their keyword
     (`XLKR`), activate parents (`XAPR`), the door on the other side
     (`XTEL`), the object's own roles in locations (`XLRT`) and the objects
     named by the properties of its scripts (`VMAD`);
   - of quests: forced references of aliases (`ALFR`) and the objects the
     properties of the quest's scripts, and up to Skyrim its aliases'
     scripts, name;
   - of locations: their special objects with their role (`LCSR`, `ACSR`);
   - of AI packages: places and targets that are a placed object (`PLDT`,
     `PLD2`, `PTDA` of type 0);
   - of worldspaces: the large objects (`RNAM`). Each plugin lists the
     large objects it places or moves, and a version of a worldspace
     without the list takes none away, so the lists of all plugins are
     joined. On a real setup the main file lists 34,034, about 120 other
     versions of Tamriel list none, and a few mods list their own.
   Records are looked at one by one as the plugin is read
   (`plugin::read_request_each`) and only the pointers are kept, so even
   the main file is never held in memory whole. A pointer from a version
   that a later plugin deletes is kept, so that it can be shown as gone.
2. **The other side of each tie is present, off or gone.** Gone is a
   deleted record, or a placed object switched off for good: switched off
   at the start and buried below the ground by a cleaning tool, or shown
   only while the player is not, or with neither an enable parent nor
   persistence, so that nothing can switch it on. Off is switched off at
   the start with something that may switch it on. Ties to what is gone
   are listed but do not count.
3. **A verdict per object from the ties that count**, the most binding
   wins:
   - **locked**: a door with a way through (hiding takes the way away), a
     forced alias of a quest (the quest's steps may need to use it), a
     large object of the worldspace (the distant view is built around it);
   - **careful**, may be hidden with a warning that names the ties: it
     activates or switches others (hiding a lever leaves nothing to pull,
     hiding the parent of a group leaves its parts standing; the parts
     themselves keep following it), others link to it (a guard's post, a
     carriage seat), it has a role in a location, an AI package or a
     script uses it, it is drawn from afar (the distant view shows it
     until it is generated again), or it is persistent without anything
     found that still uses it (scripts outside quests and placed objects
     are not read);
   - **free**: nothing ties it.
4. **The engine keeps the reader per setup** and answers
   `object_anchors(world_or_cell, objects)` with the reasons and the
   verdict of each object (`mallet_core::anchors::ObjectAnchors`). The
   reasons carry kinds and names, no prose; the window words them.

## Consequences

- The object card can say why an object is locked and offer what is only
  careful. The wall stone of the example is careful: it is drawn from
  afar, nothing else ties it.
- The first question for a setup reads every plugin: 1.4 to 4 seconds on
  the user's setup of about 400 plugins, then answers take no time. The
  window can ask with no objects ahead of time.
- The kept index holds the placed objects that point somewhere or are
  persistent or drawn from afar (about 330,000 on that setup), the
  quests, locations and packages that point at objects, and the names of
  base objects.
- Not read: scripts of base objects, dialogue, scenes and other records,
  condition functions naming an object, and the fragments and alias
  scripts of quests after Skyrim. What they need of an object is covered
  only by its persistence, which keeps it careful.

## Alternatives considered

- **Keep the lock on both flags.** Simple, but locks most of a city's
  walls, which are only drawn from afar, and cannot say why.
- **Read the pointers with the place.** The reader of a place sees only
  its cells; enable parents, quests and packages are elsewhere.
- **Keep the index in the cache between runs.** Possible later; reading
  takes seconds once per run, and the index would have to follow every
  change of the load order.
