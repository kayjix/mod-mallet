# 0007 Findings: kinds, confidence, identity

- Status: accepted
- Date: 2026-11-15

## Context

A finding is the product. Three properties decide whether people can work
with findings over time:

- They have to be readable in the user's language, in the terminal and in
  the app alike.
- People must be able to tell a fact from a guess, or they will stop
  trusting either.
- A decision such as "ignore this" must still apply to the same finding
  after the next scan, even though the load order may have changed.

## Decision

**A finding carries no prose.** It names a *kind* (for example
`extender.plugin-incompatible`) and the arguments for that kind's message.
`mallet-messages` renders the text from Fluent catalogs, one per language,
with English as the source language. Every kind is declared once in
`mallet-core` and must have complete texts in every language; a test
enforces it.

**Every kind has a fixed confidence:**

| Confidence | Meaning | Example |
|---|---|---|
| certain | A fact read from the setup | The game logged that it switched a plugin off |
| likely | A heuristic that is usually right | Two buildings overlap |
| hint | Knowledge that may not apply here | A note that two mods are said to conflict |

**A finding's identity is `kind:subject`**, where the subject is described
in terms that do not depend on the load order, the machine or the language:
a normalized path relative to the game directory, or a plugin's file name
plus the record's local number. A full FormID is never part of an identity,
because its first part changes with the load order.

**Kind identifiers are permanent.** They are never renamed or reused.

## Consequences

- Adding a language is adding a directory of text files.
- The reference documentation of all findings is generated from the same
  declarations and texts.
- Free-form detail has to be passed as an argument, which keeps messages
  uniform but makes one-off wording impossible.
- Texts of the app's own controls (buttons, labels) live in the frontend.
  Only texts about findings are rendered by Rust.
