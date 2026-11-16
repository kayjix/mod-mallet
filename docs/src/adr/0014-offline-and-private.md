# 0014 Offline and private

- Status: accepted
- Date: 2026-11-15

## Context

A scan reads a person's whole mod list, file paths that contain their user
name, and logs of what they play. None of that is anyone else's business. A
tool that needs a network connection also stops working when a service
changes or disappears.

## Decision

- Mod Mallet works **fully offline**. Every core feature runs without a
  network connection and without an account.
- It collects **no telemetry** and sends no usage data, crash reports or
  mod lists anywhere.
- Network access happens only for an **explicit, visible action** of the
  user, such as updating the LOOT masterlist, and says what it contacts.
- Reports meant for sharing, for example attached to a bug report, are
  created by an explicit export that replaces the user's home directory in
  paths.
- The desktop app ships a Content Security Policy that allows no remote
  content.

## Consequences

- There are no usage numbers to guide development. Feedback has to come from
  people.
- Optional features that need a service (ADR 0008, language models) must be
  switched on by the user and must degrade to "not available" without it.
