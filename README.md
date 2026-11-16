# Mod Mallet

Mod Mallet diagnoses a modded game setup, explains in plain language what is
wrong, and proposes fixes that can be undone. It works alongside the tools
modders already use instead of replacing them.

The first supported game is Skyrim Special Edition with the Vortex mod
manager. Mod Mallet is written in Rust.

> **Status: pre-alpha.** Mod Mallet reports script extender plugins that the
> game refused to load, characters whose face files do not match the record
> that wins ("dark faces"), patches a mod's installer would offer now but
> that were never installed, replacement meshes that a texture mod no
> longer reaches, and files in a mod's archive that the game takes from
> somewhere else. See the [roadmap](docs/src/product/roadmap.md)
> for what comes next.

## What it looks like today

```text
$ mallet scan
Skyrim Special Edition 1.7.104, script extender 2.3.1
48 script extender plugins checked
385 plugins and 209 archives read

Error (certain): Script extender plugin does not support this game version
  Old.dll was switched off because it was not built for game version 1.7.104.
  File: data/skse/plugins/old.dll
  Mod: Old Mod
  What to do: Look on the mod's page for a version built for your game version and install it. If there is none yet, disable the mod until it has been updated.
  [extender.plugin-incompatible]

Warning (likely): Characters will probably have dark or mismatched faces
  College.esp is loaded after Faces.esp and describes different faces for 16 characters than the face files in use, which come from Face Overhaul. For example: Ancano, Brelyna, Colette.
  What to do: If you want the looks of the face files, load their plugin after the other one, or look for a compatibility patch for the two mods. If you want the other plugin's version of the characters, its mod has to provide matching face files.
  [npcs.face-mismatch]

1 error, 1 warning, 133 notes
Notes are not listed. Add --notes to see them.
More about a finding: mallet explain <kind>
```

A scan only reads. It changes nothing in the game, the mods or the mod
manager. Where the mod manager can fix a finding, Mod Mallet offers to ask
it: so far, to run a mod's installer again in Vortex.

## Building

You need [Rust](https://rustup.rs) and, for the desktop app,
[Node.js](https://nodejs.org) 22 or newer. On Windows you also need the
Visual Studio Build Tools. Details are in
[Development setup](docs/src/contributing/setup.md).

```text
npm install
cargo run -p mallet-cli -- scan     # the command line
npm run desktop -- dev              # the desktop app
cargo xtask ci                      # every check CI runs
```

## Documentation

The documentation is a book in [`docs/`](docs/src/SUMMARY.md). Build and open
it with `cargo xtask docs --open`. Good places to start:

- [Vision and non-goals](docs/src/product/vision.md): what Mod Mallet is for
  and what it will not become.
- [Architecture overview](docs/src/architecture/overview.md): how the code is
  organised.
- [Decisions](docs/src/adr/README.md): why it is organised that way.
- [Finding catalog](docs/src/reference/findings.md): everything Mod Mallet
  can report.

## Contributing

Contributions are welcome. Read [CONTRIBUTING.md](CONTRIBUTING.md) first.

## License

Mod Mallet is free software under the
[GNU General Public License, version 3 or later](LICENSE).

Mod Mallet is not affiliated with Bethesda Softworks, Nexus Mods, the LOOT
project or the Rust Foundation.
