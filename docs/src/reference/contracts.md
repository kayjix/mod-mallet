# Data contracts

Mod Mallet hands its results to other programs as JSON. The shape of that
JSON is described by JSON Schema files in the repository's `schemas/`
directory. They are generated from the Rust types and checked in CI
([ADR 0012](../adr/0012-typed-contracts.md)).

| Schema | Describes | Produced by |
|---|---|---|
| `schemas/scan-report.schema.json` | The result of a scan: the game, and the findings with their kinds and arguments. No prose. | `mallet scan --format json`, `mallet report --format json` |
| `schemas/rendered-report.schema.json` | The same report with the texts of one language added. | The desktop app's `scan` and `last_report` commands |

## Using the scan report

```text
mallet scan --format json > report.json
```

Every report carries a `schemaVersion`. It is raised when a field is removed
or changes meaning. Fields may be added without raising it, so a consumer
should ignore fields it does not know.

A finding's `id` stays the same from scan to scan as long as the problem is
the same, so it can be used to track or suppress findings.

## TypeScript

The desktop app's types in `apps/desktop/src/generated/` are derived from
the rendered-report schema. Do not edit them. Change the Rust type and run
`cargo xtask gen`.
