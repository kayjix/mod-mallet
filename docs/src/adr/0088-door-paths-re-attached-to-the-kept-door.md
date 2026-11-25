# 0088 The paths of merged doors are re-attached to the kept door

- Status: accepted
- Date: 2026-11-25
- Changes: [ADR 0067](0067-stacked-doors-are-merged-into-one.md) (its rule 9,
  "the door triangles wait for the navigation mesh writer", and the
  window's "(later)"); [ADR 0058](0058-paths-repaired-on-the-ground-below.md)
  (the plugin of paths also carries door triangles)

## Context

Merging stacked doors (ADR 0067) hides the other doors and leads the doors
on the other side to the kept one through the plugin of moves. The paths of
the characters still named the hidden door: a door triangle of a navigation
mesh (`NVNM`) says through which placed door characters step there. The
plan of a merge listed these as `RecordChange::NavmeshDoor`, but nothing
wrote them, and the window said they would be attached "later". Characters
walked to where the hidden door stood, right beside the kept one, and
through a door the game no longer shows.

ADR 0058 has since brought the writer and the repair of navigation meshes:
the versions the game uses are copied, changed and written into the plugin
of paths of each patch (ADR 0070).

What was found on the user's setup:

- In the Arch-Mage's quarters of the College of Winterhold, the version of
  `Skyrim.esm:077D7F` the game uses is JK's patch for Immersive College of
  Winterhold. Its triangle 282 lists two door entries, one for the game's
  door (`Skyrim.esm:109685`) and one for the College mod's door
  (`CollegeOfWinterholdImmersive.esp:2D908E`), which stand 5 units apart.
  A triangle can name several doors.
- At Riften's main gate, neither of the two stacked gates (JK's Skyrim's and
  Cutting Room Floor's) is named by any door triangle: Open Cities' version
  of the mesh leads characters through a hidden door of its own
  (`Skyrim.esm:079260`, "OCS AI Door") about 17 m away. There is nothing to
  re-attach there.

## Decision

1. **The door triangles go into the plugin of paths.** Each place of a
   patch's repairs (`RepairPlace`) carries the door triangles to re-attach
   there (`doors`, each a `DoorRelink`: mesh, triangle, the door it names,
   the door it is to name). They come from the plans of the spots whose
   decision merges doors, as the teleports do (ADR 0067 rule 7): they are
   not choices of their own. The window puts them into the patch when it is
   written (`doorRelinksOf`).
2. **In the same copies as the repairs.** A mesh that is repaired and has
   door triangles to re-attach is one override with both. A mesh with
   nothing to repair is copied for its door triangles alone, unrepaired,
   even where the user turned the repair of the place off. Two places that
   reach the same mesh add up their door triangles.
3. **The change is made where the masters are known.** A copied record
   (`NavmeshRecord`) keeps its fields in the frame of the plugin it was
   copied from and lists its re-attachments (`relinked`); they are applied
   while its form IDs are rewritten for the plugin of paths
   (`fields_for`). The kept door's plugin becomes a master of the plugin of
   paths. A triangle that names both the hidden and the kept door names the
   kept one once.
4. **What cannot be made is said.** A triangle the repair takes out, a mesh
   or entry that is not where the plan says (another plugin changed it
   since the scan), or a mesh that cannot be copied leaves the
   re-attachment out; the result of writing the paths lists it
   (`CreatedPaths::not_relinked`) and the log names it.
5. **The check of the ways (ADR 0059) asks about the kept door.**
   `door_reach` takes the paths as the patch leaves them, repaired over
   what it hides, with every door triangle of the plan re-attached and
   every door of the plan leading where the plan says, and tells whether a
   way leads to the kept door from the rest of the paths and whether
   characters step through it to the other side.
6. **The window.** A door's plan says "N paths of the characters are
   re-attached to the door of X" and that this goes into the plugin
   Mod Mallet - Paths of the patch. The card of the door merged into shows
   the check: characters reach it and walk through it; or reach it but go
   no further on the other side; or reach it not at all. Where no door of
   the group was on the paths before either, it says that characters go
   their own way there, as at Riften's gate. The patch page counts the
   paths it re-attaches.

## Consequences

- At the Arch-Mage's quarters, merging into either door re-attaches one
  door entry of `Skyrim.esm:077D7F` (as JK's patch for the College has
  it); the check finds the kept door reached and walked through. Written
  into a test folder, the plugin of paths holds the one mesh with the
  re-attached triangle and the College mod among its masters.
- The plugin of paths now also exists for patches that repair nothing, as
  soon as a door merge re-attaches paths.
- The navigation info map (`NAVI`) still lists the hidden door among the
  mesh's doors, as ADR 0058 accepted for repairs: a character may plan a
  long way through the door by its old name, and finds the kept one on
  the mesh.
- A mod loaded after the patch that changes the same mesh undoes the
  re-attachment, as it undoes a repair.

## Alternatives considered

- **A plugin of its own for door triangles.** A mesh both repaired and
  re-attached would need two overrides that undo each other (the reason
  ADR 0067 put the teleports into the plugin of moves).
- **Remove the hidden door's entry instead of renaming it.** Characters
  would no longer step through anything there; the kept door is what they
  are to use.
- **Re-attach by rewriting the mesh in the analyses' representation and
  writing it from there.** The representation has no form IDs; the kept
  door of another plugin could not be named without the masters of the
  plugin being written.
