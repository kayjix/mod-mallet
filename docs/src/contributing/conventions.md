# Conventions

Most conventions are enforced by tools, so this page is short. It lists what
the tools cannot check.

## Language

Code, comments, commit messages and documentation are in English. Texts
shown to users are translated; see
[Adding a finding](../architecture/findings.md).

## Commits

Commit messages follow [Conventional Commits](https://www.conventionalcommits.org):

```text
feat(skse-log): report plugins that failed to load

Longer explanation if the change needs one.
```

Types: `feat`, `fix`, `docs`, `refactor`, `test`, `perf`, `build`, `ci`,
`chore`, `revert`. A hook checks the format. One commit does one thing.

## Rust

- **No panics outside tests.** Code that reads external data returns an
  error or records the problem and continues
  ([ADR 0013](../adr/0013-untrusted-input.md)). The lints enforce the
  mechanics; the intent is yours to keep.
- **No `unsafe`.** If you believe you need it, open an issue first.
- **Public items are documented.** Say what the thing is for, not how it is
  implemented. A reader should be able to use a crate from its documentation
  alone.
- **Errors are typed in libraries** (`thiserror`) and say what the user can
  do. `anyhow` is for the command line and `xtask`.
- **Parsing is separate from reading.** A parser takes text or bytes and
  has a test that it never panics.
- **Respect the crate rules.** If your change needs a dependency the
  [rules](../architecture/crates.md) forbid, the design needs another look.

## Tests

- A test's name states the behaviour it checks:
  `a_cache_from_another_version_is_emptied`.
- Tests use made-up data only ([ADR 0017](../adr/0017-synthetic-test-data.md)).
- A bug fix comes with a test that fails without it.

## Texts for users

- Write for someone who installs mods but has never opened a modding tool.
- Say what happened, why it matters in the game, and what to do.
- Explain a technical term the first time it appears, or avoid it.
- Do not blame the user or the mod author.

## Decisions

A change that alters the architecture comes with a decision record. See
[Decisions](../adr/README.md).
