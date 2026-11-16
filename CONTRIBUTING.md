# Contributing to Mod Mallet

Thank you for considering a contribution. This page is the short version;
the details are in the documentation it links to.

## Before you start

- Read [Vision and non-goals](docs/src/product/vision.md). It says what Mod
  Mallet will not become, which saves you from building something that
  cannot be accepted.
- For anything larger than a fix, open an issue first and describe what you
  plan. That is cheaper than a rejected pull request.

## Setting up

Follow [Development setup](docs/src/contributing/setup.md). When
`cargo xtask ci` passes, you are ready.

You do not need the game or any mod to build and test.

## Making a change

1. Create a branch.
2. Make the change, with tests. A bug fix comes with a test that fails
   without it.
3. If you changed a type that crosses a boundary, a finding or the command
   line, run `cargo xtask gen` and commit the regenerated files.
4. Run `cargo xtask ci`.
5. Commit using [Conventional Commits](docs/src/contributing/conventions.md#commits).
6. Open a pull request and say what changed and why.

Common tasks have their own guide:

- [Adding a finding](docs/src/architecture/findings.md)
- [Adding a dependency](docs/src/contributing/dependencies.md)
- [Recording a decision](docs/src/adr/README.md)

## What a review looks at

- Does the change respect the [crate rules](docs/src/architecture/crates.md)?
- Can code that reads external data fail without panicking?
- Are the texts understandable for someone who has never opened a modding
  tool?
- Is every new public item documented?

## Reporting a wrong finding

A finding that is wrong or misleading is a bug. Please report it with the
output of `mallet scan --format json`. Check the output for anything you do
not want to share before posting it: it contains file names from your game
directory and the names of your mods.

## License

By contributing you agree that your contribution is licensed under
GPL-3.0-or-later, the license of the project.
