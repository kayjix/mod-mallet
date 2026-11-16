# 0022 The desktop app shows a report as a task list

- Status: accepted
- Date: 2026-11-15

## Context

The first desktop app showed every finding as a card with its title, a
sentence, the remedy and the explanation, one below the other. On a real
setup that was twenty cards of equal weight and several screens of text.
The person in front of it could not tell what had to be done, what could
be done with a click, and what was only worth knowing. The buttons that
start an action were inside the cards, so finding them meant reading
everything.

## Decision

The window has three levels, and each shows only what the level above
cannot.

1. **The overview** says where the setup stands: one sentence ("2 errors
   to fix, 6 things to check") and five numbers: open, to fix, to check,
   fixable from here, good to know. The numbers are also the switches of
   the list below.
2. **The task list** gives every finding one line, the *headline*, and
   puts the button that does something about it on that line. Findings of
   the same kind share a group. Errors come before warnings; notes are a
   view of their own and never mixed with what needs attention.
3. **Behind a line** is everything else: what exactly is wrong, what to
   do, what it means, which mod and file.

To make that possible:

- Every kind of finding has a **headline** next to its title and summary:
  the finding in a few words, with its arguments
  ([Findings](../architecture/findings.md)). Titles name the kind and are
  short enough to head a group.
- **Arranging is the window's business; deciding is not.** Which section a
  finding is in follows from its severity, which button it has from its
  actions, both set by the engine. The frontend only sorts and counts.
- The severity words of the window are about the user, not the tool:
  "to fix", "to check", "good to know".

## Consequences

- A setup with 150 findings opens as a sentence, five numbers and a
  handful of lines.
- What can be done with one click is visible without opening anything,
  and has a view of its own.
- A new kind of finding needs a headline in every language. The test that
  checks the texts of every kind checks it.
- The command line still prints findings one after the other. It can use
  the headline when it grows a summary view.

## Alternatives considered

- **Cards with less text.** Shorter, but still one weight for everything
  and no place for the question "what now?".
- **Grouping by mod.** Natural for someone who thinks in mods, and planned
  as a second view (mod profiles on the [roadmap](../product/roadmap.md)).
  As the first view it would spread one kind of problem over many places.
- **Building the groups in the engine.** Keeps the frontend thinner, but
  the groups are a matter of presentation, and the command line would
  group differently.
