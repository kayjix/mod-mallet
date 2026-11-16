# 0013 Mod files are untrusted input

- Status: accepted
- Date: 2026-11-15

## Context

Mod Mallet reads files that thousands of strangers produced with dozens of
tools over more than a decade: plugins, archives, meshes, installer scripts,
logs. Some are damaged, some deviate from the format, and a file could be
crafted on purpose. A diagnosis tool that crashes on the broken file is
useless exactly when it is needed.

## Decision

1. **Code that reads external data never panics.** It returns an error, or
   it records what it could not read and continues. The workspace lints deny
   `unwrap`, `expect`, `panic!`, unchecked indexing, `todo!` and
   `unimplemented!` outside tests.
2. **Every parser has a property test** that feeds it arbitrary input and
   requires that it returns. Parsers of binary formats additionally get a
   fuzzing target, run on Linux in CI.
3. **`unsafe` is denied** for the whole workspace. The lint level is `deny`
   and not `forbid`, so that one reviewed exception remains possible, for
   example for memory-mapping large archives. An exception needs a comment
   that explains why it is sound and is confined to one small module.
4. **A file that cannot be read becomes a finding** where that helps the
   user, since a damaged file is itself a problem with the setup.
5. **Sizes and counts read from a file are checked** before memory is
   allocated for them.

## Consequences

- Writing parsers takes more care: every slice access is checked.
- Tests may use `unwrap` and `panic!` freely; `clippy.toml` exempts them.
  Helper functions in integration tests need an explicit `allow`, because
  the exemption only covers test functions.
- Archive extraction, when it is added, must refuse paths that leave the
  target directory.
