# 0045 A health score, and tasks by what to do

- Status: accepted; changed by [ADR 0076](0076-task-rows-crash-risks-first.md): task rows, crash risks first
- Date: 2026-11-18
- Changes: ADR 0022 (the report as a task list), ADR 0027 (the home page)

## Context

Since the collisions became a worklist of their own (ADR 0040), the task
page repeated them as one more list of findings, sorted by severity. On a
real setup that list was dominated by collision findings, and the rest
said little about what to do: "fix" and "check" are not instructions. The
mods page showed every mod with its findings in a flat list, without its
collisions or what other mods cover of it. The home page counted findings,
but a count does not tell someone who has never opened a modding tool
whether the game will run.

The user's own words: collisions lower the quality; problems with
navigation meshes may make the game crash.

## Decision

1. **The home page shows a health score and a verdict.** The score runs
   from 0 to 100, the verdict is one of *playable*, *playable, but the
   quality suffers* and *crashes likely*. Below them are the few reasons
   that take off the most points, each with a button to where it is dealt
   with (tasks, collisions, patch).
2. **The score is computed in the window** (`apps/desktop/src/health.ts`),
   from the report's findings, its collision spots and the patch's
   choices. It weighs; it finds nothing. An engine value would not know
   which spots the user decided or which findings they set aside, both of
   which live in the window's settings.
3. **The formula.**
   - Every kind of finding has an *impact* and a weight in points: *crash*
     (can crash or keep the game from starting: a missing master 30,
     deleted navigation meshes 10, doors cut off by navigation meshes 8,
     skinned meshes for another game 12, crash logs 25, navigation meshes
     undone 1.5), *broken* (wrong in the game: script extender plugins
     switched off 12, missing requirements and incompatibilities 8, faces
     and missing textures 6), *quality* (3–4) and *note* (0–0.5). Kinds
     the window does not know count by their severity: error 10 as
     broken, warning 3 as quality, note 0.
   - A finding counts its weight times its confidence: certain 1, likely
     0.8, a guess 0.4.
   - The findings of one kind count √n times their mean, so that dozens
     of notes do not weigh like dozens of crashes.
   - Collision spots count by their kind in the worklist: a duplicate 1,
     an overlap 0.4, a deep touch 0.15, a touch 0.01. An undecided spot
     counts fully; a spot decided to hide one side counts half until the
     patch in the game hides it, then not at all; a spot where both stay
     does not count. Together they make 1.5 × √(sum) raw points.
   - The raw points of each group (the impacts crash, broken, quality and
     note, and the collisions) are *saturated*: a group with *r* raw
     points and a cap *c* takes `c × (1 − e^(−r/c))`. While *r* is small
     that is nearly *r*; it never reaches *c*, and it still moves when *r*
     is large. The caps: crash 60, broken 30, collisions 30, quality 15,
     note 5.
   - The groups take their shares one after the other, of what the
     previous ones left: the score is `100 × ∏ (1 − taken / 100)`,
     rounded. It cannot reach 0 (with every group at its cap it is about
     16), and fixing one of many findings raises it.
   - What is lost in all is shared out to the reasons by their part in it,
     so the points the home page shows add up to 100 minus the score.
   - The verdict is *crashes likely* when a finding with a heavy crash
     weight (marked `severe`) is open and not a guess, or the score is
     below 40; *the quality suffers* when a broken finding is open or the
     score is below 80; *playable* otherwise.
   - Findings set aside count not at all: the user has looked at them and
     decided. Recommendations (`mods.collides-widely`,
     `mods.mostly-overridden`, ADR 0043) are optional and never count.
     Where Mod Mallet could not read everything (`setup.*`), the page says
     the score may be too kind.
4. **The task page groups findings by what the user does**, in this
   order: done with one click (an action for the mod manager exists),
   install what is missing, update, rebuild or clean, decide which mod
   wins (with the duels that are not about placed objects), what Mod
   Mallet could not read, look at it (no fixed recipe), then
   *recommendations* (optional: the mod-level kinds and every finding that
   is only a guess), *good to know* (notes, folded) and *set aside*
   (folded). Collisions are not listed; one line says how many spots are
   still to decide and leads to the worklist. A finding whose impact is
   *crash* carries a tag saying so.
5. **The mods page lists the mods like a mod manager:** name and author,
   version (and a newer one), enabled, the count of open findings and of
   undecided collision spots, the mod's page. It searches by name and
   author, sorts by problems, by collisions or by name, and filters to
   mods with problems or switched off. A mod opens to show its findings,
   the mods it collides with (each with a button that opens the worklist
   at one of their spots), what other mods cover of it or it covers of
   others, and the recommendations about it.

## Consequences

- The weights are a judgement, written down in one place and here. They
  will be wrong for some setups; changing one is a change of this record.
- A new kind of finding counts by its severity until it gets a weight in
  `health.ts` and a step in `arrange.ts`.
- Because the score depends on the user's decisions, it rises as the
  worklist is worked through and the patch is built. That is the intent:
  the score says what is left to do, not how many mods the setup has.
- Collision findings name mods by name; the mods page matches them to
  the mod manager's mods by name and leaves out what it cannot match.

## Alternatives considered

- **Score in the engine.** Rejected for now: the decisions that move the
  score (set aside, spots decided, the patch built) are kept by the
  window. The weights could move to the kind catalogue later, next to
  severity and confidence.
- **A penalty per finding without √n and caps.** One large setup has
  hundreds of collision spots and dozens of navigation mesh notes; the
  score sat at 0 whatever else was fixed.
- **Hard caps, added up** (the first version of this record: each group
  took off its raw points up to its cap, and the score was 100 minus the
  sum). The caps added up to 140, so a real setup with cut-off doors, a
  recent crash, dark faces and 963 undecided spots scored 0, and fixing
  the doors left it at 0 because the crash group stayed at its cap. The
  gentle saturation and the shares one after the other replaced them on
  2026-11-18: the same setup scores about 30, and about 38 once the doors
  are fixed.
- **Set-aside findings counting a little.** It made the score drop for
  things the user had explicitly decided to accept, which reads as
  Mod Mallet not listening.
