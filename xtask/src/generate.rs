//! Files that are derived from the code.
//!
//! These files are committed so that people can read them without building
//! anything, and CI runs `cargo xtask gen --check` so they can never drift
//! from the code they describe (ADR 0012).

use std::fmt::Write as _;
use std::path::Path;

use anyhow::{Context, Result, bail};
use mallet_core::{ScanReport, kinds};
use mallet_messages::{
    Locale, Messages, RenderedLink, RenderedOutcome, RenderedPlaces, RenderedReport,
    RenderedToolbox,
};

use crate::tools;

/// The contracts of the desktop app: for each, the schema and the file its
/// TypeScript types are written to.
const DESKTOP_CONTRACTS: [(&str, &str); 14] = [
    (
        "schemas/rendered-report.schema.json",
        "apps/desktop/src/generated/rendered-report.ts",
    ),
    (
        "schemas/rendered-outcome.schema.json",
        "apps/desktop/src/generated/rendered-outcome.ts",
    ),
    (
        "schemas/rendered-link.schema.json",
        "apps/desktop/src/generated/rendered-link.ts",
    ),
    (
        "schemas/rendered-places.schema.json",
        "apps/desktop/src/generated/rendered-places.ts",
    ),
    (
        "schemas/rendered-toolbox.schema.json",
        "apps/desktop/src/generated/rendered-toolbox.ts",
    ),
    (
        "schemas/map-image.schema.json",
        "apps/desktop/src/generated/map-image.ts",
    ),
    (
        "schemas/file-preview.schema.json",
        "apps/desktop/src/generated/file-preview.ts",
    ),
    (
        "schemas/mesh-view.schema.json",
        "apps/desktop/src/generated/mesh-view.ts",
    ),
    (
        "schemas/game-choice.schema.json",
        "apps/desktop/src/generated/game-choice.ts",
    ),
    (
        "schemas/lod-tiles.schema.json",
        "apps/desktop/src/generated/lod-tiles.ts",
    ),
    (
        "schemas/lod-mesh.schema.json",
        "apps/desktop/src/generated/lod-mesh.ts",
    ),
    (
        "schemas/area-objects.schema.json",
        "apps/desktop/src/generated/area-objects.ts",
    ),
    (
        "schemas/area-land.schema.json",
        "apps/desktop/src/generated/area-land.ts",
    ),
    (
        "schemas/object-anchors.schema.json",
        "apps/desktop/src/generated/object-anchors.ts",
    ),
];

/// A file and the content it should have.
struct Generated {
    /// Path relative to the workspace root, with `/` separators.
    path: &'static str,
    content: String,
}

pub(crate) fn run(root: &Path, check: bool) -> Result<()> {
    let mut files = vec![
        Generated {
            path: "schemas/scan-report.schema.json",
            content: schema(&schemars::schema_for!(ScanReport))?,
        },
        Generated {
            path: "schemas/rendered-report.schema.json",
            content: schema(&schemars::schema_for!(RenderedReport))?,
        },
        Generated {
            path: "schemas/rendered-outcome.schema.json",
            content: schema(&schemars::schema_for!(RenderedOutcome))?,
        },
        Generated {
            path: "schemas/rendered-link.schema.json",
            content: schema(&schemars::schema_for!(RenderedLink))?,
        },
        Generated {
            path: "schemas/rendered-places.schema.json",
            content: schema(&schemars::schema_for!(RenderedPlaces))?,
        },
        Generated {
            path: "schemas/rendered-toolbox.schema.json",
            content: schema(&schemars::schema_for!(RenderedToolbox))?,
        },
        Generated {
            path: "schemas/map-image.schema.json",
            content: schema(&schemars::schema_for!(mallet_core::MapImage))?,
        },
        Generated {
            path: "schemas/file-preview.schema.json",
            content: schema(&schemars::schema_for!(mallet_core::FilePreview))?,
        },
        Generated {
            path: "schemas/mesh-view.schema.json",
            content: schema(&schemars::schema_for!(mallet_core::mesh::MeshView))?,
        },
        Generated {
            path: "schemas/game-choice.schema.json",
            content: schema(&schemars::schema_for!(mallet_core::GameChoice))?,
        },
        Generated {
            path: "schemas/lod-tiles.schema.json",
            content: schema(&schemars::schema_for!(mallet_core::mesh::LodTiles))?,
        },
        Generated {
            path: "schemas/lod-mesh.schema.json",
            content: schema(&schemars::schema_for!(mallet_core::mesh::LodMesh))?,
        },
        Generated {
            path: "schemas/area-objects.schema.json",
            content: schema(&schemars::schema_for!(mallet_core::mesh::AreaObjects))?,
        },
        Generated {
            path: "schemas/area-land.schema.json",
            content: schema(&schemars::schema_for!(mallet_core::mesh::AreaLand))?,
        },
        Generated {
            path: "schemas/object-anchors.schema.json",
            content: schema(&schemars::schema_for!(mallet_core::anchors::ObjectAnchors))?,
        },
        Generated {
            path: "docs/src/reference/findings.md",
            content: finding_catalog(),
        },
        Generated {
            path: "docs/src/reference/cli.md",
            content: cli_reference(),
        },
    ];

    // The TypeScript types are derived from a schema, which therefore has to
    // be on disk before the converter runs.
    if !check {
        write_all(root, &files)?;
    }
    if tools::node_modules_installed(root) && root.join("apps/desktop").is_dir() {
        for (schema, types) in DESKTOP_CONTRACTS {
            files.push(Generated {
                path: types,
                content: typescript_types(root, schema)?,
            });
        }
    } else {
        println!("note: Node dependencies are not installed, skipping the TypeScript types");
    }

    if check {
        check_all(root, &files)
    } else {
        write_all(root, &files)?;
        println!("generated {} files", files.len());
        Ok(())
    }
}

fn write_all(root: &Path, files: &[Generated]) -> Result<()> {
    for file in files {
        let path = root.join(file.path);
        if let Some(directory) = path.parent() {
            std::fs::create_dir_all(directory)
                .with_context(|| format!("could not create `{}`", directory.display()))?;
        }
        std::fs::write(&path, &file.content)
            .with_context(|| format!("could not write `{}`", path.display()))?;
    }
    Ok(())
}

fn check_all(root: &Path, files: &[Generated]) -> Result<()> {
    let stale: Vec<_> = files
        .iter()
        .filter(|file| {
            // A checkout may have converted line endings; only the text counts.
            std::fs::read_to_string(root.join(file.path))
                .map(|on_disk| on_disk.replace("\r\n", "\n"))
                .ok()
                .as_deref()
                != Some(file.content.as_str())
        })
        .map(|file| file.path)
        .collect();

    if stale.is_empty() {
        println!("{} generated files are up to date", files.len());
        Ok(())
    } else {
        bail!(
            "out of date: {}. Run `cargo xtask gen` and commit the result.",
            stale.join(", ")
        );
    }
}

fn schema(schema: &schemars::Schema) -> Result<String> {
    let mut json = serde_json::to_string_pretty(schema).context("could not serialize a schema")?;
    json.push('\n');
    Ok(json)
}

/// Converts a schema on disk to TypeScript types.
fn typescript_types(root: &Path, schema: &str) -> Result<String> {
    let banner = format!("/* Generated by `cargo xtask gen` from {schema}. Do not edit. */");
    let mut command = tools::npx(root, "json2ts");
    command.args([
        "--input",
        schema,
        "--bannerComment",
        &banner,
        "--additionalProperties",
        "false",
    ]);
    let types = tools::capture(command, "json2ts")?;
    Ok(types.replace("\r\n", "\n"))
}

fn finding_catalog() -> String {
    let messages = Messages::new(Locale::En);
    let mut page = String::from(
        "<!-- Generated by `cargo xtask gen` from crates/mallet-core/src/kinds.rs and \
         crates/mallet-messages/locales/en. Do not edit. -->\n\n\
         # Finding catalog\n\n\
         Every kind of finding Mod Mallet can report. `mallet explain <kind>` prints the \
         same texts in the terminal.\n\n\
         | Kind | Severity | Confidence | Title |\n\
         |---|---|---|---|\n",
    );
    for kind in kinds::ALL {
        let _ = writeln!(
            page,
            "| `{}` | {} | {} | {} |",
            kind.id,
            messages.severity(kind.severity),
            messages.confidence(kind.confidence),
            messages.kind(kind).title,
        );
    }
    for kind in kinds::ALL {
        let text = messages.kind(kind);
        let arguments: Vec<_> = kind.args.iter().map(|name| format!("`{name}`")).collect();
        let _ = write!(
            page,
            "\n## {id}\n\n\
             **{title}**\n\n\
             - Severity: {severity}\n\
             - Confidence: {confidence}\n\
             - Message arguments: {arguments}\n\n\
             {explanation}\n\n\
             **What to do:** {remedy}\n",
            id = kind.id,
            title = text.title,
            severity = messages.severity(kind.severity),
            confidence = messages.confidence(kind.confidence),
            arguments = arguments.join(", "),
            explanation = text.explanation,
            remedy = text.remedy,
        );
    }
    page
}

fn cli_reference() -> String {
    let mut command = mallet_cli::command().term_width(100);
    command.build();

    let mut page = String::from(
        "<!-- Generated by `cargo xtask gen` from crates/mallet-cli/src/lib.rs. Do not edit. -->\n\n\
         # Command line\n\n\
         The `mallet` command. This page shows the output of `--help` for the command and \
         each of its subcommands.\n",
    );
    push_help(&mut page, "mallet", &mut command);
    let names: Vec<_> = command
        .get_subcommands()
        .map(|subcommand| subcommand.get_name().to_owned())
        .filter(|name| name != "help")
        .collect();
    for name in names {
        if let Some(subcommand) = command.find_subcommand_mut(&name) {
            push_help(&mut page, &format!("mallet {name}"), subcommand);
        }
    }
    page
}

fn push_help(page: &mut String, title: &str, command: &mut clap::Command) {
    let help = command.render_long_help().to_string();
    let _ = write!(page, "\n## {title}\n\n```text\n");
    for line in help.trim_end().lines() {
        page.push_str(line.trim_end());
        page.push('\n');
    }
    page.push_str("```\n");
}
