# Adding a finding

A new check touches four places. The tests and `cargo xtask ci` tell you if
you forgot one.

## 1. Declare the kind

In `crates/mallet-core/src/kinds.rs`, add a constant and list it in `ALL`:

```rust,ignore
pub const EXTENDER_PLUGIN_INCOMPATIBLE: KindInfo = KindInfo {
    id: "extender.plugin-incompatible",
    severity: Severity::Error,
    confidence: Confidence::Certain,
    args: &["file", "gameVersion"],
};
```

- The **id** is `area.what-is-wrong` in lower-case kebab-case. It is
  permanent: people's "ignore" decisions refer to it. Never rename or reuse
  one.
- The **confidence** is `Certain` only for something read directly from the
  setup. A heuristic is `Likely`. Knowledge that may not apply is `Hint`.
- The **args** are the placeholders of the message.

## 2. State the facts

If the analysis needs something that is not in `Facts` yet, add a plain data
type to `crates/mallet-core/src/facts.rs` and have an adapter fill it. Facts
are observations, not judgements: "the extender logged this status", not
"this plugin is broken".

The code that reads a file is split in two: a pure parser from text or bytes
to facts, and a few lines that find and read the file. The parser gets a
property test showing that it never panics
([ADR 0013](../adr/0013-untrusted-input.md)).

## 3. Write the analysis

In `crates/mallet-analysis`, implement `Analysis` and list it in `all()`:

```rust,ignore
Finding::new(&kinds::EXTENDER_PLUGIN_INCOMPATIBLE, Subject::File { path })
    .with_arg("file", file_name)
    .with_arg("gameVersion", version)
    .with_provider(context.mods.provider_of(&path))
```

A finding's identity is derived from its kind and its subject, so the
subject must not depend on the load order, the machine or the language
([ADR 0007](../adr/0007-findings.md)).

## 4. Write the texts

In `crates/mallet-messages/locales/en/findings.ftl`, and then in every other
language:

```ftl
extender-plugin-incompatible = Script extender plugin does not support this game version
    .summary = { $file } was switched off because it was not built for game version { $gameVersion }.
    .explanation = What this is and why it matters in the game.
    .remedy = What to do about it.
```

Write for someone who installs mods but has never opened a modding tool.
Only `.summary` and `.headline` may use the arguments. The headline is
the finding in a few words, for lists that give each finding one line.

## 5. Regenerate and check

```text
cargo xtask gen
cargo xtask ci
```

`gen` refreshes the [finding catalog](../reference/findings.md). `ci` fails
if a kind lacks texts in any language, if a finding carries arguments its
kind does not declare, or if a generated file is out of date.
