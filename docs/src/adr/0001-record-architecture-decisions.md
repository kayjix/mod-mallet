# 0001 Record architecture decisions

- Status: accepted
- Date: 2026-11-15

## Context

The project is meant to be built once and extended for years, by more people
than started it. Decisions made in a conversation are lost with the
conversation. Later contributors then either repeat the discussion or undo a
decision without knowing what it protected.

## Decision

Every decision that shapes the architecture is written down as a short
record in `docs/src/adr/`, in the form described in the
[index](README.md). Records are numbered, kept in the repository next to the
code, and published with the documentation.

## Consequences

- The reasons for the structure of the code can be read, not guessed.
- Changing a decision means writing a new record that answers the old one.
- Writing a record takes a few minutes. That is the price, and it also
  filters out decisions nobody can justify.
