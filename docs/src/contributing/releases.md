# Releases

Mod Mallet has one version for the whole app (ADR 0057). It lives in
`[workspace.package]` of the workspace's `Cargo.toml`; every crate takes
it from there, and the desktop app shows it under Settings → About.

## Counting

Versions follow [Semantic Versioning](https://semver.org). Before 1.0:

- the **minor** number goes up when a build brings something new a user
  can see or do (`0.1.0` → `0.2.0`);
- the **patch** number goes up when a build only fixes or polishes
  (`0.2.0` → `0.2.1`).

Every build that leaves the developer's machine to be tried on a real
setup gets a version of its own. Builds in between show the commit they
were made from, so two builds of one version can still be told apart.

## Making a version

1. Move what is under `## [Unreleased]` in `CHANGELOG.md` into a new
   section `## [x.y.z] - YYYY-MM-DD`, written for someone who installs
   mods, not for developers. Add the link at the bottom.
2. Change the version in `Cargo.toml` (`[workspace.package]`),
   `package.json` and `apps/desktop/package.json`, then run
   `npm install` so that `package-lock.json` follows.
3. `cargo xtask ci`. Its step "One version" fails if these places
   disagree or the change log has no section for the version.
4. Commit as `chore(release): x.y.z` and tag the commit `vx.y.z`.
5. `cargo xtask app --release --no-run` builds `dist/Mod Mallet.exe`
   with the commit stamped in.

The Vortex extension in `integrations/vortex-extension` keeps a version
of its own: the toolbox compares it with the one Vortex runs to offer an update.
