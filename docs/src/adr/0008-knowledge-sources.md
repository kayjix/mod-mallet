# 0008 Where knowledge comes from

- Status: accepted
- Date: 2026-11-15

## Context

To judge a setup Mod Mallet needs to know what a mod contains, what it
requires and what it conflicts with. Much of this already exists in
structured form. Some exists only as prose on a mod's page. Language models
can read prose, but they are costly, need a network connection, and can be
wrong in ways that are hard to notice.

## Decision

Knowledge is taken from these sources, in this order of trust:

1. **The mod's own content.** File paths, record types in its plugins, DLLs,
   the installer description. A mod that ships `textures/landscape/grass`
   changes grass, whatever its page says.
2. **The mod manager's metadata.** Category, version, source, and the
   installer options that were chosen.
3. **Community-maintained structured data**, first of all the LOOT
   masterlist: requirements, incompatibilities, patches.
4. **Mod Mallet's own rules**, kept as data files that can be reviewed and
   contributed to like code.
5. **Language models**, only for free text that none of the above covers.
   Their output is always a finding of confidence *hint*, and no core
   feature depends on them.

A category shown to the user says where it came from when sources disagree.

## Consequences

- The tool is useful offline and without any account.
- Results are reproducible: the same setup gives the same findings.
- Reading content is more work up front than asking a model, but it is
  correct by construction.
- A language model never produces or modifies plugin data.
