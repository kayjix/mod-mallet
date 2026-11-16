# 0010 Games: the Bethesda family first

- Status: accepted
- Date: 2026-11-15

## Context

The problems Mod Mallet addresses are not unique to Skyrim. It is tempting
to design for "any game" from the start. But an abstraction made before the
second case exists is a guess, and guesses in the foundation are what force
rewrites later. The games that share Skyrim's technology (Fallout 4,
Starfield, and likely the next Elder Scrolls) share its plugin and archive
formats almost unchanged.

## Decision

- `mallet-core` and `mallet-analysis` contain **no game-specific code**.
  That is enforced by the dependency rules.
- Game-specific knowledge lives in an adapter crate per game. The first is
  `mallet-game-skyrimse`.
- Formats shared by the Bethesda family will live in one crate that their
  adapters use.
- A concept moves into the game-neutral core **when a second adapter needs
  it**, not before.
- Games outside the Bethesda family are neither excluded nor prepared for.

## Consequences

- Supporting Fallout 4 is expected to be a small adapter plus tests.
- Supporting a game with a wholly different modding model would show where
  the core still assumes Bethesda conventions. That work is accepted as the
  cost of not guessing now.
- Vocabulary in the core is deliberately general ("script extender", not
  "SKSE"), so that it reads correctly for the second game.
