# 0086 Preferences between mods, learned from the decisions

- Status: accepted
- Date: 2026-11-25

## Context

The user decides hundreds of conflicts between mods: duels of files in
Asset collisions (ADR 0054), spots, passages and stacked doors in Map
collisions and Go through it (ADR 0067, 0072, 0080). Many of these follow
one taste: a mesh mod they find better, an overhaul of characters they
always keep, one city mod over another. Saying the same thing again at
every conflict is the tedious part, and the user called reading that
pattern the point of the app: notice it, say so in a small prompt, and
pre-select it at the other conflicts, so that they see they already have
preferences and can settle the rest at once.

The app already suggests sides: by size (ADR 0050), by LOOT's rules
(ADR 0030), by a duel's reason (built on, specialist), by the ranking the
user sorts (Asset collisions), and it recommends removing mods that
collide widely (ADR 0061). It must not learn these back as the user's
taste: a batch of suggestions taken at once says little about what the
user likes.

## Decision

The window learns preferences from the decisions, in a pure module
(`preferences.ts`), and nothing leaves the machine.

**Evidence.** Every decision that kept one mod against another is a vote:
a duel's pick (for each of its topics), a collision decided as a whole, a
spot decided on its own, and stacked doors merged into one door (the kept
door's mod against each mod whose doors go). The spots of one passage
count together as one decision. Objects hidden on their own and decisions
to keep both say nothing about a preference. The votes come from every
patch of the user's own, switched on or off (a taste does not depend on
which patch counts now), the patch being edited winning where two decide
the same; patches loaded from others do not count.

**Weights.** A decision that agrees with a suggestion weighs a quarter: the
size suggestion of a spot (the parts marked suggested) or a passage, the
reason Mod Mallet gives for a duel, and the side the ranking makes win. A
decision made by applying a preference weighs nothing. The texts show the
user's own decisions ("you kept it 14 of 15 times") and name those that
followed a suggestion apart.

**Preferences.** Mods are compared by the name the mod manager shows, found
alike for plugins (the load order's provider) and the sides of duels (the
mod's identity). Two kinds are read, by topic (meshes, textures, faces and
characters, other files, buildings and placed objects):

- a *favourite*: a mod kept in a topic in at least 5 weighted decisions,
  at least 80 % of them, against at least two other mods;
- a *pair*: one mod kept over another, over all topics, by the same
  thresholds.

**Rules and conflicts.** The preferences that count are the ones the user
set (always prefer a mod, for a topic or everything, over every other mod
or one) and, while learning is on, the learned ones not turned down. A
conflict they can decide is a duel about files or a spot of two mods'
plugins; stacked doors are merged by the door panel and left out. Where
several preferences speak, a pair goes before a favourite and one the user
set before one learned; where equally strong ones pull both ways, none
decides.

**Offer.** While learned preferences have open conflicts they would
decide, a small mallet waits in a corner of the window, the hammer of the
logo, with how many on a badge: Mal the Rubber Mallet, as the user named
it, the name it goes by where it speaks. About once a minute it makes a short move
(tossed and caught, a bounce, a tap to either side), never while the user
holds a button, types or scrolls, and not for those who asked their
system for less motion; it says nothing until clicked. Clicked, a panel
slides up from it with one preference at a time (a pager where there are
several): the mod, the topic, how often the user kept it, and Apply to
the open conflicts it would decide; what Apply does in full sits behind
an ⓘ and in the settings. Show list lists each with the way to it (the
duel on the assets page, the spot in Map collisions). Apply decides them into the patch being edited
(ADR 0070): each spot with the mod given up, which hides only the parts
that may go, as every spot decision does; a spot where characters walk
on what would go stays open (ADR 0051). A duel is picked as by hand
(ADR 0054), which asks the mod manager to let the files of the kept mod
win; when the mod manager cannot be reached, the duels stop there, as the
ranking's do. One undo takes the batch back in Mod Mallet; what the mod
manager was asked stays there, and the note says so. Closing the panel
puts it back into the mallet. "Don't suggest this again", in a small menu
of the panel, stops a preference, as an offer and as a suggestion. (An
earlier "Not now", which asked again once 5 more of the user's own
decisions spoke for it, is still honoured where it was given.)

**What is kept.** Per game in the settings, as `preferences`
(`PatchPreferences`): learning on or off, the preferences the user set,
the answers to the offers, and which decisions applying a preference made
(by key, with the preference). What is learned is not kept: it is read
again from the decisions whenever they change.

**Visible.** Map collisions, Go through it and Asset collisions mark the
side a preference keeps "suggested: your preference", apart from the size
suggestion and LOOT's word, or "from your preference" where applying it
decided the conflict. The settings have a page for it: learning on or
off, what was learned with how often, each to turn off or use again and
to apply to its open conflicts, and the preferences the user sets.

## Consequences

- A user who keeps choosing the same mod is told, and can settle the rest
  of its conflicts in one go, with a preview and one undo.
- Learning reads every decision of the user's own patches on each change
  of the book: one pass over the decisions and the spots, as the snapshot
  of ADR 0081 makes.
- Decisions made by suggestions taken in bulk before the preferences
  existed cannot be told from the user's own where they agree with no
  suggestion the window can see again (Go through it's "Take suggestions"
  measures sides in 3D); those count fully.
- A preference is about mods by name: a mod renamed in the mod manager
  starts afresh, and a preference the user set for it stays with the old
  name.
- The host of the preferences sits beside the pages and publishes to them
  (one window, one host), so that the shell is not wrapped and other work
  on it does not collide.

## Alternatives considered

- **A card that stays in the corner until answered.** The first version
  did so, with Preview, Apply, Not now and Never; the user found a card
  that waits there all the time annoying. The mallet keeps the count in
  sight and talks only when asked.

- **Learning per plugin instead of per mod.** The spots know plugins, the
  duels mods; a mod with several plugins would split its evidence, and the
  user thinks in mods.
- **Applying a preference as pre-selections only, decided one by one.**
  The pre-selection is shown anyway; asking each again is the tedium the
  user wants gone. Apply decides, with a preview before and one undo after.
- **Counting each spot of a passage.** A passage decided as a whole is one
  act of the user's; counting its spots made one passage look like a
  strong preference.
