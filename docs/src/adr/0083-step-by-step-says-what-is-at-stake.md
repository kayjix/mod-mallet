# 0083 "Step by step" says what is at stake in every step

- Status: accepted
- Date: 2026-11-25
- Changes: ADR 0050 (a guided review through the homes): its name, its steps
  and what each says; ADR 0080 (the collisions step): how the sections of a
  place and their parts are laid out

## Context

The guided review (ADR 0050) is the first thing Mod Mallet offers. After
the collisions step was laid out by side (ADR 0080), the user went
through the whole review on a real setup and found the collisions step
practical, and the rest thin:

- Its name, "Go through it", does not say what it is.
- The installation step listed findings without saying what the app is
  for or why the step matters.
- The load order had a step of its own, though the order is set in the
  mod manager and Mod Mallet has no data on which plugin wins which
  record. Its few findings did not make a step.
- The assets step was a dry list of duel lines; the way to Asset
  collisions, where they are decided, was the only thing that counted.
- The world map step listed worlds and said nothing about them.
- In the collisions step, the sections of a place were rows of numbers
  and buttons, and their part lists two columns of bare names.
- The report did not say what it all means.

## Decision

- The review is called **Step by step** (German: *Schritt für Schritt*).
  The user chose it from three names.
- It has five steps: installation, assets, world map, collisions,
  report. The load order joins the installation as a group of its own
  ("Plugins and their order"); a remembered step "order" opens the
  installation. The user chose this over a step reshaped around record
  conflicts, which would need data the engine does not read.
- **Installation** opens with who does what, as three tiles: Mod Mallet
  decides, the mod manager carries out, the game shows. Then what the step
  checks and why, the setup in four numbers, and what to fix, mods and
  installers apart from plugins.
- **Assets** says what an asset collision is, shows how many are
  decided, and the biggest open duels as cards with a picture of each
  side's file (the one in the game marked), what they disagree about and
  the way to compare; the way to Asset collisions is the step's main
  button.
- **World map** shows, per world with collisions, its land (the game's
  picture of it, else drawn from its heights) with a point per place,
  coloured by its worst kind, decided ones small and green; places inside
  are buttons below. A point leads to the place in the collisions step.
  The user chose this over folding the map into the collisions step.
- **Collisions**: each section of a place shows itself in the views,
  says in plain words what differs (only one mod; one builds at least
  half again as much; both alike) and the worst kind of collision there,
  and offers one recommended action: keep the bigger version, or what the
  whole place keeps, or keep both where nothing can go. Without a reason
  to prefer one, both versions are offered alike. The other choices and
  pinning come on demand. Its parts are one list for both mods, by mod and
  by name, with what kind of thing each is (from its model's folder:
  building, furniture, small things, light, door, plant, rock, effect) and
  what is known of it (rests on another, tied in, needed, walked on). A
  passage's summary is a note of the kit that opens for the details (ADR
  0077), as the kit's own work on the collisions brings it.
- **Report** opens with four numbers (places decided, asset collisions
  decided, what is left to fix, objects the patch hides), says what the
  patch does pair by pair of mods with the way to each place, what is
  still open with the way there, and what to try in the game, with the
  console command for each decided place to copy. The full report with
  every part's name stays below, on request.

## Consequences

- Every step says what it is for before it shows a list, and each leads
  to its page.
- The steps read the shared decisions (ADR 0081): what the report counts
  as hidden is what every page shows hidden.
- A part's kind is a guess from its folder; a mod that keeps its models
  in a folder of its own name says "other", and the list leaves the kind
  out.
- The world map step draws a world's land once per visit; the big map
  stays the place to look around.

## Alternatives considered

- **Keep the load order step, reshaped as "where your load order decides
  a conflict".** It would need the engine to read which plugin wins each
  record; a step about it would be empty until then.
- **Fold the map into the collisions step.** The collisions step is long
  already, and a map before it says where the work is.
- **Pictures of each section.** Rendering a picture per section costs a
  view per section; showing the section in the views already there, with
  the rest dimmed, shows the same.
